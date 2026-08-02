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
  agent: AgentStatus | null;
  agent_error: string | null;
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
}

export interface SystemMetrics {
  cpu: {
    logical_cores: number;
    usage_percent: number;
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
}

export interface SystemCapabilities {
  reboot_allowed: boolean;
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
}

export interface ServiceSummary {
  id: string;
  description: string;
  load_state: string;
  active_state: string;
  sub_state: string;
  unit_file_state: string | null;
  control_allowed: boolean;
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
}

export interface SshKeySummary {
  id: string;
  key_type: string;
  fingerprint: string;
  comment: string | null;
}

export interface SshKeyList {
  enabled: boolean;
  managed_user: string | null;
  keys: SshKeySummary[];
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
  sshKeys: () => request<SshKeyList>("/api/v1/settings/ssh/keys"),
  addSshKey: (publicKey: string) =>
    request<SshKeySummary>("/api/v1/settings/ssh/keys", {
      method: "POST",
      body: JSON.stringify({ public_key: publicKey }),
      headers: { "Content-Type": "application/json" },
    }),
  removeSshKey: (keyId: string) =>
    request<SshKeySummary>(`/api/v1/settings/ssh/keys/${encodeURIComponent(keyId)}`, {
      method: "DELETE",
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
  storage: () => request<StorageMount[]>("/api/v1/storage"),
  services: () => request<ServiceSummary[]>("/api/v1/services"),
  serviceLogs: (serviceId: string, lines: number, priority: ServiceLogPriority) => {
    const query = new URLSearchParams({ lines: String(lines), priority });
    return request<ServiceLogs>(
      `/api/v1/services/${encodeURIComponent(serviceId)}/logs?${query.toString()}`,
    );
  },
  serviceAction: (
    serviceId: string,
    action: "start" | "stop" | "restart" | "enable" | "disable",
  ) =>
    request<CommandResult>(
      `/api/v1/services/${encodeURIComponent(serviceId)}/${action}`,
      { method: "POST" },
    ),
};

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
