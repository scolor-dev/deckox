use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    pub status: String,
    pub hostname: String,
    pub operating_system: String,
    pub architecture: String,
    pub uptime_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub hostname: String,
    pub operating_system: String,
    pub os_version: Option<String>,
    pub kernel_version: String,
    pub architecture: String,
    pub uptime_seconds: u64,
    pub boot_id: Option<String>,
    pub timezone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemCapabilities {
    pub reboot_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDiagnostics {
    pub version: String,
    pub host: DiagnosticHost,
    pub deckox_services: DeckoxServiceDiagnostics,
    pub runtime_config: RuntimeConfigSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsReport {
    pub generated_at_ms: u64,
    pub server: DiagnosticServer,
    pub agent: DiagnosticAgent,
    pub host: Option<DiagnosticHost>,
    pub deckox_services: Option<DeckoxServiceDiagnostics>,
    pub runtime_config: Option<RuntimeConfigSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticServer {
    pub version: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticAgent {
    pub connected: bool,
    pub version: Option<String>,
    pub error_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticHost {
    pub hostname: String,
    pub operating_system: String,
    pub os_version: Option<String>,
    pub kernel_version: String,
    pub architecture: String,
    pub uptime_seconds: u64,
    pub timezone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckoxServiceDiagnostics {
    pub agent: DiagnosticUnitState,
    pub server: DiagnosticUnitState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticUnitState {
    pub load_state: String,
    pub active_state: String,
    pub sub_state: String,
    pub unit_file_state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfigSummary {
    pub reboot_allowed: bool,
    pub allowed_services_count: usize,
    pub ssh_management_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu: CpuMetrics,
    pub memory: MemoryMetrics,
    pub load_average: LoadAverage,
    #[serde(default)]
    pub network: Option<NetworkMetrics>,
    #[serde(default)]
    pub disk_io: Option<DiskIoMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeMetricsEvent {
    pub sequence: u64,
    pub timestamp_ms: u64,
    pub agent_online: bool,
    pub metrics: Option<SystemMetrics>,
    pub error_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuMetrics {
    pub logical_cores: usize,
    pub usage_percent: f64,
    #[serde(default)]
    pub temperature_celsius: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetrics {
    pub received_bytes_per_second: u64,
    pub transmitted_bytes_per_second: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskIoMetrics {
    pub read_bytes_per_second: u64,
    pub written_bytes_per_second: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetrics {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadAverage {
    pub one_minute: f64,
    pub five_minutes: f64,
    pub fifteen_minutes: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageMount {
    pub filesystem: String,
    pub filesystem_type: String,
    pub mount_point: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub usage_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceSummary {
    pub id: String,
    pub description: String,
    pub load_state: String,
    pub active_state: String,
    pub sub_state: String,
    pub unit_file_state: Option<String>,
    pub control_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDetails {
    pub id: String,
    pub description: String,
    pub load_state: String,
    pub active_state: String,
    pub sub_state: String,
    pub unit_file_state: Option<String>,
    pub main_pid: Option<u32>,
    pub control_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum AgentCommand {
    GetSystemStatus,
    ListServices,
    StartService { service_id: String },
    StopService { service_id: String },
    RestartService { service_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceAction {
    Start,
    Stop,
    Restart,
    Enable,
    Disable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceLogs {
    pub service_id: String,
    pub entries: Vec<ServiceLogEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceLogEntry {
    pub timestamp_ms: u64,
    pub priority: u8,
    pub message: String,
    pub process: Option<String>,
    pub pid: Option<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceLogPriority {
    All,
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub command_id: String,
    pub status: CommandStatus,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshKeyList {
    pub enabled: bool,
    pub managed_user: Option<String>,
    pub keys: Vec<SshKeySummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshKeySummary {
    pub id: String,
    pub key_type: String,
    pub fingerprint: String,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddSshKeyRequest {
    pub public_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandStatus {
    Accepted,
    Running,
    Completed,
    Failed,
}
