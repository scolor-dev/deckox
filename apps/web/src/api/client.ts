export interface AgentStatus {
  status: string;
  hostname: string;
  operating_system: string;
  architecture: string;
  uptime_seconds: number | null;
}

export interface ServerStatus {
  name: string;
  version: string;
  status: string;
  port: number;
  agent: AgentStatus | null;
  agent_error: string | null;
  webhook_configured: boolean;
}

export interface SystemInfo {
  hostname: string;
  operating_system: string;
  os_version: string | null;
  kernel_version: string;
  architecture: string;
  uptime_seconds: number;
  boot_id: string | null;
  timezone: string | null;
  lan_addresses: string[];
}

export interface SystemMetrics {
  cpu: {
    logical_cores: number;
    usage_percent: number;
    temperature_celsius?: number | null;
  };
  memory: {
    total_bytes: number;
    used_bytes: number;
    available_bytes: number;
    swap_total_bytes: number;
    swap_used_bytes: number;
  };
  load_average: {
    one_minute: number;
    five_minutes: number;
    fifteen_minutes: number;
  };
  network?: {
    received_bytes_per_second: number;
    transmitted_bytes_per_second: number;
  } | null;
  disk_io?: {
    read_bytes_per_second: number;
    written_bytes_per_second: number;
  } | null;
}

export interface SystemCapabilities {
  reboot_allowed: boolean;
  update_allowed: boolean;
}

export interface ServerHealth {
  status: "ok";
  instance_id: string;
}

export interface StorageMount {
  filesystem: string;
  filesystem_type: string;
  mount_point: string;
  total_bytes: number;
  used_bytes: number;
  available_bytes: number;
  usage_percent: number;
  standard: boolean;
}

export interface BackupSummary {
  name: string;
  previous_version: string | null;
  created_at_ms: number | null;
  size_bytes: number;
}

export interface ServiceSummary {
  id: string;
  description: string;
  load_state: string;
  active_state: string;
  sub_state: string;
  unit_file_state: string | null;
  control_allowed: boolean;
  standard_system: boolean;
  deckox_managed: boolean;
  product: string | null;
}

export type ScheduleAction = "start" | "stop" | "restart";

export interface ServiceSchedule {
  id: string;
  service_id: string;
  action: ScheduleAction;
  hour: number;
  minute: number;
  weekdays: number[];
  enabled: boolean;
  created_at_ms: number;
  last_run_at_ms: number | null;
  last_result: string | null;
}

export interface CreateScheduleRequest {
  service_id: string;
  action: ScheduleAction;
  hour: number;
  minute: number;
  weekdays: number[];
}

export type ServiceLogPriority = "all" | "error" | "warning" | "info";

export interface ServiceLogEntry {
  timestamp_ms: number;
  priority: number;
  message: string;
  process: string | null;
  pid: number | null;
}

export interface ServiceLogs {
  service_id: string;
  entries: ServiceLogEntry[];
}

export interface CommandResult {
  command_id: string;
  status: "accepted" | "running" | "completed" | "failed";
  message: string | null;
}

export interface AuthStatus {
  authenticated: boolean;
  totp_required: boolean;
}

export interface TotpStatus {
  enabled: boolean;
  recovery_codes_remaining: number;
}

export interface TotpSetup {
  secret_base32: string;
  otpauth_uri: string;
}

export interface TotpConfirmResult {
  recovery_codes: string[];
}

export interface DiagnosticsResponse {
  generated_at_ms: number;
  server: {
    version: string;
    status: string;
  };
  agent: {
    connected: boolean;
    version: string | null;
    error_code: string | null;
  };
  host: {
    hostname: string;
    operating_system: string;
    os_version: string | null;
    kernel_version: string;
    architecture: string;
    uptime_seconds: number;
    timezone: string | null;
    upgradable_packages: number | null;
  } | null;
  deckox_services: {
    agent: {
      load_state: string;
      active_state: string;
      sub_state: string;
      unit_file_state: string | null;
    };
    server: {
      load_state: string;
      active_state: string;
      sub_state: string;
      unit_file_state: string | null;
    };
  } | null;
  runtime_config: {
    reboot_allowed: boolean;
    update_allowed: boolean;
    allowed_services_count: number;
  } | null;
}

export interface DeckoxServiceDiagnostic {
  id: string;
  state: {
    load_state: string;
    active_state: string;
    sub_state: string;
    unit_file_state: string | null;
  };
}

export const DIAGNOSTICS_REPORT_FILENAME = "deckox-diagnostics.json";
export const AUDIT_REPORT_FILENAME = "deckox-audit.json";

export interface AuditEvent {
  timestamp_ms: number;
  event: string;
  actor: string;
  source_ip: string;
  result: string;
  detail: string | null;
}

export interface AuditPage {
  events: AuditEvent[];
  has_more: boolean;
}

export interface UpdateStatus {
  status: "up_to_date" | "available" | "unavailable";
  current_version: string;
  latest_version?: string | null;
  update_available: boolean;
  release_url?: string | null;
  checked_at_ms?: number | null;
}

export class ApiError extends Error {
  constructor(
    message: string,
    public readonly status: number,
    public readonly code?: string,
  ) {
    super(message);
  }
}

async function request<T>(
  path: string,
  init?: RequestInit,
  notifyUnauthorized = true,
): Promise<T> {
  const headers = new Headers(init?.headers);
  headers.set("Accept", "application/json");
  const response = await fetch(path, {
    ...init,
    headers,
    credentials: "same-origin",
    signal: init?.signal ?? AbortSignal.timeout(15_000),
  });
  const body = await response.json().catch(() => null) as
    | { code?: string; message?: string }
    | null;

  if (!response.ok) {
    if (response.status === 401 && notifyUnauthorized) {
      window.dispatchEvent(new Event("deckox:unauthorized"));
    }
    throw new ApiError(
      body?.message ?? `APIリクエストに失敗しました (${String(response.status)})`,
      response.status,
      body?.code,
    );
  }

  return body as T;
}

async function requestBlob(path: string): Promise<Blob> {
  const response = await fetch(path, {
    headers: { Accept: "application/json" },
    credentials: "same-origin",
    signal: AbortSignal.timeout(15_000),
  });
  if (!response.ok) {
    const body = await response.json().catch(() => null) as
      | { code?: string; message?: string }
      | null;
    if (response.status === 401) {
      window.dispatchEvent(new Event("deckox:unauthorized"));
    }
    throw new ApiError(
      body?.message ?? `APIリクエストに失敗しました (${String(response.status)})`,
      response.status,
      body?.code,
    );
  }
  return response.blob();
}

export const api = {
  health: () => request<ServerHealth>("/healthz", undefined, false),
  authSession: () => request<AuthStatus>("/api/v1/auth/session", undefined, false),
  login: (password: string) =>
    request<AuthStatus>(
      "/api/v1/auth/login",
      {
        method: "POST",
        body: JSON.stringify({ password }),
        headers: { "Content-Type": "application/json" },
      },
      false,
    ),
  loginTotp: (code: string) =>
    request<AuthStatus>(
      "/api/v1/auth/login/totp",
      {
        method: "POST",
        body: JSON.stringify({ code }),
        headers: { "Content-Type": "application/json" },
      },
      false,
    ),
  logout: () => request<AuthStatus>("/api/v1/auth/logout", { method: "POST" }),
  changePassword: (currentPassword: string, newPassword: string) =>
    request<AuthStatus>(
      "/api/v1/settings/password",
      {
        method: "POST",
        body: JSON.stringify({
          current_password: currentPassword,
          new_password: newPassword,
        }),
        headers: { "Content-Type": "application/json" },
      },
      false,
    ),
  totpStatus: () => request<TotpStatus>("/api/v1/settings/totp/status"),
  totpSetup: () => request<TotpSetup>("/api/v1/settings/totp/setup", { method: "POST" }),
  totpConfirm: (code: string) =>
    request<TotpConfirmResult>("/api/v1/settings/totp/confirm", {
      method: "POST",
      body: JSON.stringify({ code }),
      headers: { "Content-Type": "application/json" },
    }),
  totpDisable: (currentPassword: string, code: string) =>
    request<TotpStatus>("/api/v1/settings/totp/disable", {
      method: "POST",
      body: JSON.stringify({ current_password: currentPassword, code }),
      headers: { "Content-Type": "application/json" },
    }),
  serverStatus: () => request<ServerStatus>("/api/v1/status"),
  systemInfo: () => request<SystemInfo>("/api/v1/system"),
  systemMetrics: () => request<SystemMetrics>("/api/v1/system/metrics"),
  systemCapabilities: () => request<SystemCapabilities>("/api/v1/system/capabilities"),
  rebootSystem: (currentPassword: string) =>
    request<CommandResult>("/api/v1/system/reboot", {
      method: "POST",
      body: JSON.stringify({ current_password: currentPassword }),
      headers: { "Content-Type": "application/json" },
    }),
  triggerUpdate: (currentPassword: string) =>
    request<CommandResult>("/api/v1/system/update", {
      method: "POST",
      body: JSON.stringify({ current_password: currentPassword }),
      headers: { "Content-Type": "application/json" },
    }),
  storage: () => request<StorageMount[]>("/api/v1/storage"),
  backups: () => request<BackupSummary[]>("/api/v1/backups"),
  diagnostics: () => request<DiagnosticsResponse>("/api/v1/diagnostics"),
  diagnosticsReport: () => requestBlob("/api/v1/diagnostics/report"),
  auditEvents: (beforeMs?: number) =>
    request<AuditPage>(
      beforeMs == null ? "/api/v1/audit" : `/api/v1/audit?before_ms=${String(beforeMs)}`,
    ),
  auditReport: () => requestBlob("/api/v1/audit/report"),
  updateStatus: () => request<UpdateStatus>("/api/v1/update"),
  services: () => request<ServiceSummary[]>("/api/v1/services"),
  serviceLogs: (serviceId: string, lines: number, priority: ServiceLogPriority) => {
    const query = new URLSearchParams({ lines: String(lines), priority });
    return request<ServiceLogs>(
      `/api/v1/services/${encodeURIComponent(serviceId)}/logs?${query.toString()}`,
    );
  },
  serviceLogsReport: (serviceId: string, lines: number, priority: ServiceLogPriority) => {
    const query = new URLSearchParams({ lines: String(lines), priority });
    return requestBlob(
      `/api/v1/services/${encodeURIComponent(serviceId)}/logs/report?${query.toString()}`,
    );
  },
  serviceAction: (
    serviceId: string,
    action: "start" | "stop" | "restart" | "enable" | "disable" | "allow" | "disallow",
  ) =>
    request<CommandResult>(
      `/api/v1/services/${encodeURIComponent(serviceId)}/${action}`,
      { method: "POST" },
    ),
  schedules: () => request<ServiceSchedule[]>("/api/v1/schedules"),
  createSchedule: (payload: CreateScheduleRequest) =>
    request<ServiceSchedule>("/api/v1/schedules", {
      method: "POST",
      body: JSON.stringify(payload),
      headers: { "Content-Type": "application/json" },
    }),
  deleteSchedule: (scheduleId: string) =>
    request<unknown>(`/api/v1/schedules/${encodeURIComponent(scheduleId)}`, {
      method: "DELETE",
    }),
  setScheduleEnabled: (scheduleId: string, enabled: boolean) =>
    request<ServiceSchedule>(
      `/api/v1/schedules/${encodeURIComponent(scheduleId)}/${enabled ? "enable" : "disable"}`,
      { method: "POST" },
    ),
  testWebhook: () => request<unknown>("/api/v1/settings/webhook/test", { method: "POST" }),
};

export function buildUpdateCommand(version: string | null | undefined): string | null {
  if (!version) return null;
  const normalized = version.startsWith("v") ? version : `v${version}`;
  if (!/^v\d+\.\d+\.\d+$/.test(normalized)) return null;
  return "curl -fsSL https://raw.githubusercontent.com/scolor-dev/deckox/main/packaging/scripts/install.sh"
    + ` | sudo DECKOX_VERSION=${normalized} sh`;
}

export function safeReleaseUrl(value: string | null | undefined): string | null {
  if (!value) return null;
  try {
    const url = new URL(value);
    const prefix = "/scolor-dev/deckox/releases/tag/v";
    const version = url.pathname.slice(prefix.length);
    return url.protocol === "https:"
      && url.hostname === "github.com"
      && url.pathname.startsWith(prefix)
      && /^\d+\.\d+\.\d+$/.test(version)
      && !url.search
      && !url.hash
      ? url.toString()
      : null;
  } catch {
    return null;
  }
}

export async function writeClipboardText(
  value: string,
  clipboard: Pick<Clipboard, "writeText"> = navigator.clipboard,
): Promise<boolean> {
  try {
    await clipboard.writeText(value);
    return true;
  } catch {
    // Either navigator.clipboard was undefined (insecure context — accessing
    // .writeText threw) or the browser refused the request; either way, fall
    // back to the legacy copy path below rather than giving up.
    return legacyCopyToClipboard(value);
  }
}

/**
 * The Clipboard API used above is restricted to secure contexts (HTTPS or
 * localhost), so it is unavailable — `navigator.clipboard` is `undefined` —
 * when Deckox is reached over a plain-HTTP LAN address, which is how the
 * README and install docs say to use it. `document.execCommand("copy")` is
 * deprecated but still broadly supported and works over plain HTTP, so it is
 * the fallback for exactly that case.
 */
function legacyCopyToClipboard(value: string): boolean {
  // eslint-disable-next-line @typescript-eslint/no-deprecated -- no successor: this is the fallback for exactly the insecure (plain HTTP) contexts that lack the Clipboard API
  if (typeof document === "undefined" || typeof document.execCommand !== "function") return false;

  const textarea = document.createElement("textarea");
  textarea.value = value;
  textarea.setAttribute("readonly", "");
  textarea.style.position = "fixed";
  textarea.style.opacity = "0";
  document.body.append(textarea);
  textarea.select();
  textarea.setSelectionRange(0, value.length);
  try {
    // eslint-disable-next-line @typescript-eslint/no-deprecated -- see above
    return document.execCommand("copy");
  } catch {
    return false;
  } finally {
    textarea.remove();
  }
}

export function formatBytes(bytes: number, locale = "en"): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  const units = ["B", "KiB", "MiB", "GiB", "TiB"];
  const index = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / 1024 ** index;
  return `${new Intl.NumberFormat(locale, {
    maximumFractionDigits: index === 0 ? 0 : 1,
    minimumFractionDigits: index === 0 ? 0 : 1,
  }).format(value)} ${units[index]}`;
}

export function usagePercentage(used: number, total: number): number | null {
  if (!Number.isFinite(used) || !Number.isFinite(total) || used < 0 || total <= 0) return null;
  return used / total * 100;
}

export function appendMetricHistory(
  history: number[],
  value: number,
  limit = 120,
): number[] {
  if (!Number.isFinite(value) || limit <= 0) return history;
  return [...history.slice(-(limit - 1)), value];
}

export function formatUptime(
  seconds: number | null | undefined,
  locale = "en",
): string {
  if (seconds == null) return "—";
  const days = Math.floor(seconds / 86_400);
  const hours = Math.floor((seconds % 86_400) / 3_600);
  const minutes = Math.floor((seconds % 3_600) / 60);
  const japanese = locale.toLowerCase().startsWith("ja");
  if (days > 0) return japanese ? `${String(days)}日 ${String(hours)}時間` : `${String(days)}d ${String(hours)}h`;
  if (hours > 0) return japanese ? `${String(hours)}時間 ${String(minutes)}分` : `${String(hours)}h ${String(minutes)}m`;
  return japanese ? `${String(minutes)}分` : `${String(minutes)}m`;
}
