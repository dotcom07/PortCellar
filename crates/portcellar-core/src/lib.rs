mod analyze;
mod anchors;
mod command;
mod error;
mod games;
mod inspect;
mod runtime;
mod state;
mod types;

pub use analyze::{
    analyze_game, analyze_local_game, analyze_steam_client, collect_porting_analysis,
    collect_steam_client_analysis, write_local_game_analysis_artifacts,
    write_porting_analysis_artifacts, write_steam_client_analysis_artifacts, GamePortingAnalysis,
    GameProfileCandidate, LocalGameAnalysis, LocalGameAnalysisArtifacts, PeSummary,
    PortingAnalysisArtifacts, RuntimeSnapshot, SteamAppManifest, SteamCefTargetAnalysis,
    SteamClientAnalysis, SteamClientAnalysisArtifacts, SteamClientAnalysisOptions,
    SteamClientBinary, SteamClientProbe, SteamClientProbeTick, SteamLogAnalysis,
    SteamPackageAnalysis, WindowsExeCandidate, LOCAL_GAME_ANALYSIS_VERSION,
    PORTING_ANALYSIS_VERSION,
};
pub use anchors::{
    runtime_anchors, RuntimeAnchors, CROSSOVER_FOSS_BASELINE, ISAAC_APP_ID, ISAAC_APP_NAME,
    ISAAC_DXMT_CONFIG, ISAAC_RUNTIME_PROFILE_VERSION, ISAAC_STEAM_CLOUD_ENV,
    ISAAC_WINDOWS_DEPOT_ID, ISAAC_WINDOWS_EXE, ISAAC_WINE_WINDOWS_VERSION, LOCAL_WINE_BASELINE,
    PORTCELLAR_VERSION, STEAM_VIRTUAL_DESKTOP_NAME, STEAM_WINE_CEF_ARGS,
};
pub use command::{run_plan, run_plan_detached, run_plan_detached_with_log};
pub use error::{PortCellarError, Result};
pub use games::{
    isaac_profile, load_generic_game_profile, GameBinaryPatch, GameBinaryPatchKind, GameCapability,
    GameModule, GameModuleCatalog, GameModuleDescriptor, GameModuleProfileReference,
    GameModuleVariant, GameProfile, GameRuntimeArtifact, GameRuntimePolicy, GenericGameProfile,
    GenericGameProfileCatalog, GenericGameProfileCatalogEntry, GenericGameProfileDocument,
    GraphicsBackend, IsaacProfile, RuntimeDependency, SteamCefPolicy, SteamIntegration,
    GENERIC_GAME_PROFILE_FORMAT_VERSION,
};
pub use inspect::{doctor_report, inspect_game_runtime};
pub use runtime::GameRuntimeStage;
pub use runtime::{
    apply_game_runtime_profile, apply_isaac_runtime_profile, apply_steam_cef_patch,
    apply_steam_session_reset, game_installer_plan, game_launch_plan, game_launch_plan_with_stage,
    game_runtime_preflight, game_runtime_profile_plan, game_runtime_ready, game_smoke_evidence,
    inspect_game_runtime_preflight, isaac_launch_plan, isaac_runtime_profile_plan,
    kill_wine_steam_processes, load_game_compatibility_evidence, observe_game_runtime,
    prepare_game_runtime_stage, steam_session_ready, wine_bottle_mutation_plan,
    wine_bottle_rollback_plan, wine_bottle_snapshot_plan, wine_dependency_plans_for,
    wine_steam_cef_patch_plan, wine_steam_configure_plans, wine_steam_configure_plans_for,
    wine_steam_install_plan, wine_steam_login_plan, wine_steam_login_plan_for,
    wine_steam_session_reset_plan, wine_steam_stop_plan, write_game_compatibility_evidence,
    write_game_smoke_evidence,
};
pub use state::state_root;
pub use types::{
    BottleMutationPlan, BottleRollbackPlan, BottleSnapshotPlan, CommandPlan, CompatibilityStatus,
    CrashSummary, DoctorReport, EngineBackendCompatibility, EngineCandidate, EngineKind,
    GameCompatibilityEvidence, GameInstall, GameInstallerPlan, GameRuntimeObservation,
    GameRuntimeProfilePlan, GameSmokeEvidence, HostReport, IsaacInstall, IsaacRuntimeProfilePlan,
    LaunchMode, RuntimeComponent, RuntimeDependencyPlan, RuntimeDependencyStatus,
    RuntimeExecutionFingerprint, RuntimePreflight, SteamCefPatchPlan, SteamCefPatchResult,
    SteamCefPatchTarget, SteamReport, SteamSessionResetPlan, SteamSessionResetTarget,
    SteamWebHelperArch, WineBottle, WineMacDriverConfig, WineSteamRuntime,
    GAME_COMPATIBILITY_EVIDENCE_FORMAT_VERSION, GAME_COMPATIBILITY_EVIDENCE_FORMAT_VERSION_V1,
};

#[cfg(test)]
pub(crate) use analyze::*;
pub(crate) use anchors::{
    STEAM_CEF_CROSSOVER_POLICY_VALUE, STEAM_CEF_POLICY_ENV, STEAM_CEF_SINGLE_PROCESS_ARG,
    STEAM_WEBHELPER_WRAPPER_MARKER, STEAM_WEBHELPER_WRAPPER_SIZE_CEILING,
    STEAM_WEBHELPER_WRAPPER_SOURCE,
};
#[cfg(test)]
pub(crate) use command::shell_quote;
#[cfg(test)]
pub(crate) use games::isaac_steam_cloud_option_value_from;
pub(crate) use inspect::*;
pub(crate) use runtime::*;
pub(crate) use state::*;

#[cfg(test)]
mod tests;
