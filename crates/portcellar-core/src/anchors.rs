pub const PORTCELLAR_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const ISAAC_APP_ID: &str = "250900";
pub const ISAAC_APP_NAME: &str = "The Binding of Isaac: Rebirth";
pub const ISAAC_WINDOWS_DEPOT_ID: &str = "250902";
pub const ISAAC_WINDOWS_EXE: &str = "isaac-ng.exe";
pub const ISAAC_DXMT_CONFIG: &str = "d3d11.preferredMaxFrameRate=60;";
pub const ISAAC_STEAM_CLOUD_ENV: &str = "PORTCELLAR_ISAAC_STEAM_CLOUD";
pub const ISAAC_RUNTIME_PROFILE_VERSION: &str = "isaac-wine-steam-v2";
pub const ISAAC_WINE_WINDOWS_VERSION: &str = "win10";
pub const LOCAL_WINE_BASELINE: &str = "Gcenx Wine Staging 11.10";
pub const CROSSOVER_FOSS_BASELINE: &str = "CrossOverFOSS 23.7.1";

pub const STEAM_WINE_CEF_ARGS: &[&str] = &[
    "-allosarches",
    "-cef-force-32bit",
    "-no-cef-sandbox",
    "-noverifyfiles",
];
pub const STEAM_VIRTUAL_DESKTOP_NAME: &str = "portcellar-steam";
pub(crate) const STEAM_WEBHELPER_WRAPPER_SOURCE: &str =
    include_str!("../assets/steamwebhelper-wrapper.c");
pub(crate) const STEAM_WEBHELPER_WRAPPER_SIZE_CEILING: u64 = 500_000;
pub(crate) const STEAM_WEBHELPER_WRAPPER_MARKER: &[u8] = b"portcellar-steamwebhelper-wrapper-v5";
pub(crate) const STEAM_CEF_SINGLE_PROCESS_ARG: &str = "-cef-single-process";
pub(crate) const STEAM_CEF_POLICY_ENV: &str = "PORTCELLAR_STEAM_CEF_POLICY";
pub(crate) const STEAM_CEF_CROSSOVER_POLICY_VALUE: &str = "crossover-compatible";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeAnchors {
    pub package_version: &'static str,
    pub isaac_app_id: &'static str,
    pub isaac_name: &'static str,
    pub isaac_windows_depot_id: &'static str,
    pub isaac_windows_exe: &'static str,
    pub isaac_runtime_profile_version: &'static str,
    pub wine_windows_version: &'static str,
    pub default_dxmt_config: &'static str,
    pub steam_cloud_env: &'static str,
    pub local_wine_baseline: &'static str,
    pub crossover_foss_baseline: &'static str,
}

pub fn runtime_anchors() -> RuntimeAnchors {
    RuntimeAnchors {
        package_version: PORTCELLAR_VERSION,
        isaac_app_id: ISAAC_APP_ID,
        isaac_name: ISAAC_APP_NAME,
        isaac_windows_depot_id: ISAAC_WINDOWS_DEPOT_ID,
        isaac_windows_exe: ISAAC_WINDOWS_EXE,
        isaac_runtime_profile_version: ISAAC_RUNTIME_PROFILE_VERSION,
        wine_windows_version: ISAAC_WINE_WINDOWS_VERSION,
        default_dxmt_config: ISAAC_DXMT_CONFIG,
        steam_cloud_env: ISAAC_STEAM_CLOUD_ENV,
        local_wine_baseline: LOCAL_WINE_BASELINE,
        crossover_foss_baseline: CROSSOVER_FOSS_BASELINE,
    }
}
