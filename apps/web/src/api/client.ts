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

export interface CommandResult {
  command_id: string;
  status: "accepted" | "running" | "completed" | "failed";
  message: string | null;
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

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, {
    ...init,
    headers: {
      Accept: "application/json",
      ...init?.headers,
    },
  });
  const body = await response.json().catch(() => null) as
    | { code?: string; message?: string }
    | null;

  if (!response.ok) {
    throw new ApiError(
      body?.message ?? `APIリクエストに失敗しました (${response.status})`,
      response.status,
      body?.code,
    );
  }

  return body as T;
}

export const api = {
  serverStatus: () => request<ServerStatus>("/api/v1/status"),
  systemInfo: () => request<SystemInfo>("/api/v1/system"),
  systemMetrics: () => request<SystemMetrics>("/api/v1/system/metrics"),
  storage: () => request<StorageMount[]>("/api/v1/storage"),
  services: () => request<ServiceSummary[]>("/api/v1/services"),
  serviceAction: (serviceId: string, action: "start" | "stop" | "restart") =>
    request<CommandResult>(
      `/api/v1/services/${encodeURIComponent(serviceId)}/${action}`,
      { method: "POST" },
    ),
};

export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  const units = ["B", "KiB", "MiB", "GiB", "TiB"];
  const index = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  return `${(bytes / 1024 ** index).toFixed(index === 0 ? 0 : 1)} ${units[index]}`;
}

export function formatUptime(seconds: number | null | undefined): string {
  if (seconds == null) return "—";
  const days = Math.floor(seconds / 86_400);
  const hours = Math.floor((seconds % 86_400) / 3_600);
  const minutes = Math.floor((seconds % 3_600) / 60);
  if (days > 0) return `${days}日 ${hours}時間`;
  if (hours > 0) return `${hours}時間 ${minutes}分`;
  return `${minutes}分`;
}

