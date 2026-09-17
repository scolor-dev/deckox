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
    /// IPv4 addresses with global scope on physical network interfaces
    /// (loopback and virtual interfaces excluded), so the admin UI can show
    /// how to reach this host from elsewhere on the LAN.
    pub lan_addresses: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemCapabilities {
    pub reboot_allowed: bool,
    pub update_allowed: bool,
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
    /// Packages `apt` already knows are upgradable from its existing local
    /// cache. `None` on non-`apt` hosts or when the count could not be
    /// read; never triggers `apt update` itself, so this can go stale until
    /// something else refreshes the cache.
    pub upgradable_packages: Option<u32>,
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
    pub update_allowed: bool,
    pub allowed_services_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateStatus {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub update_available: bool,
    pub release_url: Option<String>,
    pub checked_at_ms: Option<u64>,
    pub status: UpdateCheckStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UpdateCheckStatus {
    UpToDate,
    Available,
    Unavailable,
}

/// Sent from Server to Agent to run a self-update.
///
/// The Server (not the root-privileged Agent) is the one that talks to
/// GitHub, so it resolves `target_version` and fetches `install_script` from
/// the pinned release tag before handing both to the Agent for execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentUpdateRequest {
    pub target_version: String,
    pub install_script: String,
}

/// `true` for well-formed `vMAJOR.MINOR.PATCH` release tags such as `v0.4.2`.
///
/// Both Server and Agent validate a release tag with this before it is
/// embedded in a fetch URL or passed to the self-update installer, so
/// unexpected input never reaches either.
#[must_use]
pub fn is_valid_release_tag(value: &str) -> bool {
    let Some(rest) = value.strip_prefix('v') else {
        return false;
    };
    let mut parts = rest.split('.');
    let (Some(major), Some(minor), Some(patch), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    [major, minor, patch]
        .iter()
        .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}

#[cfg(test)]
mod tag_tests {
    use super::is_valid_release_tag;

    #[test]
    fn accepts_well_formed_tags() {
        assert!(is_valid_release_tag("v1.2.3"));
        assert!(is_valid_release_tag("v0.4.2"));
    }

    #[test]
    fn rejects_malformed_tags() {
        assert!(!is_valid_release_tag("1.2.3"));
        assert!(!is_valid_release_tag("v1.2"));
        assert!(!is_valid_release_tag("v1.2.3-beta"));
        assert!(!is_valid_release_tag("v1.2.3/../etc"));
        assert!(!is_valid_release_tag(""));
    }
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
    /// `true` for well-known system mount points (`/`, `/boot`, `/home`, ...),
    /// mirroring `ServiceSummary::standard_system`'s "ships with the OS"
    /// distinction for services.
    pub standard: bool,
}

/// One pre-update snapshot the installer took under `/var/lib/deckox/backups/`.
///
/// Owned `root:root`, so only the Agent (not the unprivileged Server) can
/// list these; the Server proxies the read the same way it does storage and
/// services.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupSummary {
    pub name: String,
    /// `None` when the installer could not determine the version being
    /// replaced (recorded as `unknown` in the directory name).
    pub previous_version: Option<String>,
    /// `None` when the directory's filesystem metadata could not be read.
    pub created_at_ms: Option<u64>,
    pub size_bytes: u64,
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
    /// The unit file lives under a package/vendor path (e.g.
    /// `/usr/lib/systemd/system`) rather than one written locally, so this
    /// service ships with the OS or a distro package instead of being
    /// custom-installed.
    pub standard_system: bool,
    /// `true` for Deckox's own `deckox-agent.service` / `deckox-server.service`.
    pub deckox_managed: bool,
    /// Display name (`"Docker"`, `"PostgreSQL"`, ...) when the service's base
    /// unit name matches a curated list of well-known software, independent
    /// of `standard_system` — a distro-packaged and a manually installed
    /// Docker are both tagged `Some("Docker")`.
    pub product: Option<String>,
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
    pub standard_system: bool,
    pub deckox_managed: bool,
    pub product: Option<String>,
}

/// State of one admin-managed package.
///
/// There is no fixed software catalog: an admin adds a package by name, the
/// Agent confirms it resolves from the host's already-configured package
/// repositories (never a newly added third-party one), and only entries that
/// passed that check are ever returned here — so every `SoftwarePackage` this
/// type describes is, by construction, one the admin has vetted and allowed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoftwarePackage {
    pub name: String,
    pub installed: bool,
    pub installed_version: Option<String>,
    /// The version the host's package manager would currently install, read
    /// without refreshing its metadata cache (so it can be stale, matching
    /// [`DiagnosticHost::upgradable_packages`]'s read-only convention).
    pub available_version: Option<String>,
    pub upgradable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareAction {
    Install,
    Remove,
    Upgrade,
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

/// The subset of [`ServiceAction`] meaningful to schedule unattended: not
/// `Enable`/`Disable`, which are one-off admin toggles rather than something
/// worth repeating on a timer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleAction {
    Start,
    Stop,
    Restart,
}

/// A recurring `action` the Agent runs against `service_id` on its own,
/// without an admin present. Only allow-listed services (the same ones
/// eligible for manual start/stop/restart) can be scheduled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceSchedule {
    pub id: String,
    pub service_id: String,
    pub action: ScheduleAction,
    /// Local time, 0-23.
    pub hour: u8,
    /// Local time, 0-59.
    pub minute: u8,
    /// ISO weekday numbers (1 = Monday ... 7 = Sunday), never empty.
    pub weekdays: Vec<u8>,
    pub enabled: bool,
    pub created_at_ms: u64,
    pub last_run_at_ms: Option<u64>,
    /// `"success"` or `"failed: <reason>"`; `None` until the schedule has
    /// fired at least once.
    pub last_result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateScheduleRequest {
    pub service_id: String,
    pub action: ScheduleAction,
    pub hour: u8,
    pub minute: u8,
    pub weekdays: Vec<u8>,
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
#[serde(rename_all = "snake_case")]
pub enum CommandStatus {
    Accepted,
    Running,
    Completed,
    Failed,
}

/// A single recorded administrative action. `actor` is a username today
/// (always `"admin"`) so the schema does not need to change if Deckox grows
/// multiple accounts later.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub timestamp_ms: u64,
    pub event: String,
    pub actor: String,
    pub source_ip: String,
    pub result: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditPage {
    pub events: Vec<AuditEvent>,
    pub has_more: bool,
}
