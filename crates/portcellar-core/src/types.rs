use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;

use crate::games::{GameCapability, GraphicsBackend, RuntimeDependency, SteamIntegration};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct DoctorReport {
    pub host: HostReport,
    pub steam: SteamReport,
    pub game: Option<GameInstall>,
    pub wine_steam: WineSteamRuntime,
    pub engines: Vec<EngineCandidate>,
    pub latest_crash: Option<CrashSummary>,
}

#[derive(Debug, Clone)]
pub struct HostReport {
    pub native_arch: String,
    pub translated: Option<bool>,
    pub rosetta_x86_64: bool,
}

#[derive(Debug, Clone)]
pub struct SteamReport {
    pub root: Option<PathBuf>,
    pub running: bool,
    pub libraries: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct GameInstall {
    pub app_id: String,
    pub name: String,
    pub build_id: Option<String>,
    pub manifest_path: PathBuf,
    pub install_dir: PathBuf,
    pub app_bundle: Option<PathBuf>,
    pub executable: Option<PathBuf>,
    pub executable_kind: Option<String>,
    pub steam_appid_txt: Option<String>,
}

pub type IsaacInstall = GameInstall;

#[derive(Debug, Clone)]
pub struct WineSteamRuntime {
    pub wine: Option<PathBuf>,
    pub wine_version: Option<String>,
    pub windows_version: Option<String>,
    pub dxmt_root: Option<PathBuf>,
    pub prefix: PathBuf,
    pub steam_exe: Option<PathBuf>,
    pub game_manifest: Option<PathBuf>,
    pub game_install_dir: Option<PathBuf>,
    pub game_executable: Option<PathBuf>,
    pub running: bool,
    pub game_running: bool,
    pub launch_ready_after_login: bool,
    pub readiness_issues: Vec<String>,
    pub logged_in: Option<bool>,
    pub active_session: Option<bool>,
    pub cached_credentials: Option<bool>,
    pub login_status: Option<String>,
    pub connection_status: Option<String>,
    pub cef_status: Option<String>,
    pub mac_window_status: Option<String>,
    pub virtual_desktop: Option<String>,
    pub mac_driver: WineMacDriverConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameRuntimeObservation {
    pub app_id: String,
    pub profile_name: String,
    pub profile_version: String,
    pub prefix: PathBuf,
    pub wine: Option<PathBuf>,
    pub wine_version: Option<String>,
    pub windows_version: Option<String>,
    pub steam_exe: Option<PathBuf>,
    pub game_executable: Option<PathBuf>,
    pub steam_running: bool,
    pub game_running: bool,
    pub active_session: Option<bool>,
    pub steam_session_ready: bool,
    pub logged_in: Option<bool>,
    pub cached_credentials: Option<bool>,
    pub login_status: Option<String>,
    pub connection_status: Option<String>,
    pub cef_status: Option<String>,
}

impl GameRuntimeObservation {
    pub fn ready(&self, steam_required: bool) -> bool {
        self.game_running && (!steam_required || self.steam_session_ready)
    }
}

#[derive(Debug, Clone)]
pub struct GameSmokeEvidence {
    pub observed_at_unix_seconds: u64,
    pub profile: GameRuntimeObservation,
    pub launch_mode: LaunchMode,
    pub engine: Option<EngineKind>,
    pub backend_compatibility: Option<EngineBackendCompatibility>,
    pub execution_fingerprint: RuntimeExecutionFingerprint,
    pub graphics_backend: GraphicsBackend,
    pub steam_integration: SteamIntegration,
    pub stable_window_seconds: u64,
    pub passed: bool,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeExecutionFingerprint {
    pub windows_version: Option<String>,
    pub bottle_layout: String,
    pub launcher_model: String,
}

impl Default for RuntimeExecutionFingerprint {
    fn default() -> Self {
        Self {
            windows_version: None,
            bottle_layout: "unknown".to_string(),
            launcher_model: "unknown".to_string(),
        }
    }
}

pub const GAME_COMPATIBILITY_EVIDENCE_FORMAT_VERSION_V1: &str = "game-compatibility-evidence-v1";
pub const GAME_COMPATIBILITY_EVIDENCE_FORMAT_VERSION: &str = "game-compatibility-evidence-v2";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameCompatibilityEvidence {
    pub format_version: String,
    pub observed_at_unix_seconds: u64,
    pub app_id: String,
    pub profile_name: String,
    pub profile_version: String,
    pub launch_mode: String,
    pub engine: Option<String>,
    pub engine_version: Option<String>,
    #[serde(default)]
    pub execution_fingerprint: RuntimeExecutionFingerprint,
    pub graphics_backend: String,
    pub compatibility_status: Option<String>,
    pub passed: bool,
    pub stable_window_seconds: u64,
    pub steam_session_ready: bool,
    pub game_running: bool,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WineBottle {
    pub prefix: PathBuf,
    pub windows_version: Option<String>,
    pub isolated: bool,
}

#[derive(Debug, Clone)]
pub struct RuntimePreflight {
    pub game: String,
    pub engine: Option<EngineKind>,
    pub backend: GraphicsBackend,
    pub backend_compatibility: Option<EngineBackendCompatibility>,
    pub capabilities: Vec<GameCapability>,
    pub dependencies: Vec<RuntimeDependency>,
    pub dependency_plans: Vec<RuntimeDependencyPlan>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
}

impl RuntimePreflight {
    pub fn ready(&self) -> bool {
        self.blockers.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeDependencyStatus {
    Planned,
    Manual,
    ToolMissing,
}

#[derive(Debug, Clone)]
pub struct RuntimeDependencyPlan {
    pub dependency: RuntimeDependency,
    pub status: RuntimeDependencyStatus,
    pub command: Option<CommandPlan>,
    pub note: String,
}

#[derive(Debug, Clone)]
pub struct BottleSnapshotPlan {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub prepare_command: CommandPlan,
    pub command: CommandPlan,
}

#[derive(Debug, Clone)]
pub struct BottleMutationPlan {
    pub prefix: PathBuf,
    pub snapshot: BottleSnapshotPlan,
    pub dependencies: Vec<RuntimeDependencyPlan>,
}

#[derive(Debug, Clone)]
pub struct BottleRollbackPlan {
    pub prefix: PathBuf,
    pub snapshot: PathBuf,
    pub backup: PathBuf,
    pub staging: PathBuf,
    pub prepare_command: CommandPlan,
    pub stage_command: CommandPlan,
    pub backup_command: CommandPlan,
    pub activate_command: CommandPlan,
}

impl WineSteamRuntime {
    pub fn bottle(&self) -> WineBottle {
        WineBottle {
            prefix: self.prefix.clone(),
            windows_version: self.windows_version.clone(),
            isolated: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WineMacDriverConfig {
    pub allow_immovable_windows: Option<String>,
    pub retina_mode: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EngineCandidate {
    pub name: String,
    pub kind: EngineKind,
    pub path: PathBuf,
    pub version: Option<String>,
    pub available: bool,
    pub detail: Option<String>,
    pub components: Vec<RuntimeComponent>,
    pub backend_candidates: Vec<GraphicsBackend>,
    pub backend_matrix: Vec<EngineBackendCompatibility>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeComponent {
    WineD3d,
    WineVulkan,
    WineGstreamer,
    WineCoreAudio,
    FAudio,
    Vkd3d,
    MoltenVk,
    GstreamerHost,
    D3dMetal,
    Dxmt,
    Dxvk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityStatus {
    Verified,
    Candidate,
    ExternalDependency,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EngineBackendCompatibility {
    pub engine: EngineKind,
    pub backend: GraphicsBackend,
    pub status: CompatibilityStatus,
    pub rationale: &'static str,
}

#[derive(Debug, Clone)]
pub struct CrashSummary {
    pub path: PathBuf,
    pub exception: Option<String>,
    pub termination: Option<String>,
    pub translated: Option<bool>,
    pub cpu_type: Option<String>,
    pub suspect_symbols: Vec<String>,
    pub suspect_images: Vec<String>,
    pub diagnosis: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineKind {
    NativeSteam,
    NativeApp,
    CrossOver,
    Wine,
    GamePortingToolkit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchMode {
    Steam,
    Native,
    Direct,
    WineSteam,
    WineDirect,
}

#[derive(Debug, Clone)]
pub struct CommandPlan {
    pub program: PathBuf,
    pub args: Vec<OsString>,
    pub envs: BTreeMap<String, String>,
    pub current_dir: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct GameInstallerPlan {
    pub installer: PathBuf,
    pub command: CommandPlan,
}

#[derive(Debug, Clone)]
pub struct SteamSessionResetPlan {
    pub backup_dir: PathBuf,
    pub targets: Vec<SteamSessionResetTarget>,
}

#[derive(Debug, Clone)]
pub struct SteamSessionResetTarget {
    pub source: PathBuf,
    pub backup: PathBuf,
}

#[derive(Debug, Clone)]
pub struct SteamCefPatchPlan {
    pub steam_dir: PathBuf,
    pub cef_targets: Vec<SteamCefPatchTarget>,
    pub htmlcache_locks: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SteamWebHelperArch {
    X86,
    X86_64,
}

#[derive(Debug, Clone)]
pub struct SteamCefPatchTarget {
    pub cef_dir: PathBuf,
    pub target: PathBuf,
    pub real: PathBuf,
    pub arch: SteamWebHelperArch,
    pub already_patched: bool,
}

#[derive(Debug, Clone)]
pub struct SteamCefPatchResult {
    pub patched_targets: Vec<SteamCefPatchTarget>,
    pub removed_locks: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct GameRuntimeProfilePlan {
    pub options_path: PathBuf,
    pub values: BTreeMap<String, String>,
}

pub type IsaacRuntimeProfilePlan = GameRuntimeProfilePlan;
