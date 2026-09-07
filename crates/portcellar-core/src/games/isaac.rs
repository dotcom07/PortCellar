use super::{GameCapability, GameProfile, SteamCefPolicy};
use crate::{
    env_flag_enabled, ISAAC_APP_ID, ISAAC_APP_NAME, ISAAC_DXMT_CONFIG,
    ISAAC_RUNTIME_PROFILE_VERSION, ISAAC_STEAM_CLOUD_ENV, ISAAC_WINDOWS_DEPOT_ID,
    ISAAC_WINDOWS_EXE, ISAAC_WINE_WINDOWS_VERSION,
};
use std::collections::{BTreeMap, BTreeSet};
use std::env;

#[derive(Debug, Clone, Copy, Default)]
pub struct IsaacProfile;

pub fn isaac_profile() -> IsaacProfile {
    IsaacProfile
}

impl GameProfile for IsaacProfile {
    fn app_id(&self) -> &str {
        ISAAC_APP_ID
    }

    fn name(&self) -> &str {
        ISAAC_APP_NAME
    }

    fn windows_depot_id(&self) -> Option<&str> {
        Some(ISAAC_WINDOWS_DEPOT_ID)
    }

    fn windows_exe(&self) -> &str {
        ISAAC_WINDOWS_EXE
    }

    fn install_dir_hint(&self) -> &str {
        "The Binding of Isaac Rebirth"
    }

    fn runtime_profile_version(&self) -> &str {
        ISAAC_RUNTIME_PROFILE_VERSION
    }

    fn wine_windows_version(&self) -> &str {
        ISAAC_WINE_WINDOWS_VERSION
    }

    fn steam_cef_policy(&self) -> SteamCefPolicy {
        SteamCefPolicy::CrossOverCompatible
    }

    fn dxmt_config(&self) -> Option<&str> {
        Some(ISAAC_DXMT_CONFIG)
    }

    fn savedata_path_marker(&self) -> Option<&str> {
        Some("Save Data Path:")
    }

    fn default_documents_subdir(&self) -> Option<&str> {
        Some("Binding of Isaac Rebirth")
    }

    fn runtime_options(&self) -> BTreeMap<String, String> {
        BTreeMap::from([
            ("ControllerHotplug".to_string(), "0".to_string()),
            ("EnableIntro".to_string(), "0".to_string()),
            ("EnableMods".to_string(), "0".to_string()),
            ("Fullscreen".to_string(), "0".to_string()),
            ("MouseControl".to_string(), "0".to_string()),
            (
                "SteamCloud".to_string(),
                isaac_steam_cloud_option_value_from(
                    env::var(ISAAC_STEAM_CLOUD_ENV).ok().as_deref(),
                ),
            ),
            ("VSync".to_string(), "1".to_string()),
        ])
    }

    fn capabilities(&self) -> BTreeSet<GameCapability> {
        BTreeSet::from([
            GameCapability::Steamworks,
            GameCapability::OpenGl,
            GameCapability::VideoPlayback,
            GameCapability::Audio,
        ])
    }
}

pub(crate) fn isaac_steam_cloud_option_value_from(value: Option<&str>) -> String {
    value
        .filter(|value| env_flag_enabled(value))
        .map(|_| "1")
        .unwrap_or("0")
        .to_string()
}
