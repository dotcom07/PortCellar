use super::*;

pub fn steam_session_ready(runtime: &WineSteamRuntime) -> bool {
    runtime.running && runtime.active_session == Some(true)
}

pub fn game_runtime_ready(profile: &dyn GameProfile, runtime: &WineSteamRuntime) -> bool {
    runtime.game_running
        && (profile.steam_integration() != SteamIntegration::Required
            || steam_session_ready(runtime))
}

pub fn observe_game_runtime(profile: &dyn GameProfile) -> GameRuntimeObservation {
    let runtime = inspect_wine_steam_runtime_for(profile);
    game_runtime_observation(profile, &runtime)
}

fn game_runtime_observation(
    profile: &dyn GameProfile,
    runtime: &WineSteamRuntime,
) -> GameRuntimeObservation {
    GameRuntimeObservation {
        app_id: profile.app_id().to_string(),
        profile_name: profile.name().to_string(),
        profile_version: profile.runtime_profile_version().to_string(),
        prefix: runtime.prefix.clone(),
        wine: runtime.wine.clone(),
        wine_version: runtime.wine_version.clone(),
        windows_version: runtime.windows_version.clone(),
        steam_exe: runtime.steam_exe.clone(),
        game_executable: runtime.game_executable.clone(),
        steam_running: runtime.running,
        game_running: runtime.game_running,
        active_session: runtime.active_session,
        steam_session_ready: steam_session_ready(runtime),
        logged_in: runtime.logged_in,
        cached_credentials: runtime.cached_credentials,
        login_status: runtime.login_status.clone(),
        connection_status: runtime.connection_status.clone(),
        cef_status: runtime.cef_status.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn runtime() -> WineSteamRuntime {
        WineSteamRuntime {
            wine: Some(PathBuf::from("/usr/local/bin/wine")),
            wine_version: Some("wine-11.10".to_string()),
            windows_version: Some("win10".to_string()),
            dxmt_root: None,
            prefix: PathBuf::from("/tmp/prefix"),
            steam_exe: Some(PathBuf::from("/tmp/prefix/drive_c/Steam/steam.exe")),
            game_manifest: None,
            game_install_dir: Some(PathBuf::from("/tmp/prefix/drive_c/Game")),
            game_executable: Some(PathBuf::from("/tmp/prefix/drive_c/Game/Game.exe")),
            running: true,
            game_running: true,
            launch_ready_after_login: true,
            readiness_issues: Vec::new(),
            logged_in: Some(true),
            active_session: Some(true),
            cached_credentials: Some(true),
            login_status: Some("success".to_string()),
            connection_status: Some("cm-transport-ready".to_string()),
            cef_status: Some("login-window-ready".to_string()),
            mac_window_status: None,
            virtual_desktop: None,
            mac_driver: WineMacDriverConfig {
                allow_immovable_windows: None,
                retina_mode: None,
            },
        }
    }

    #[test]
    fn steam_session_requires_a_running_client() {
        let mut value = runtime();
        value.running = false;
        assert!(!steam_session_ready(&value));
    }

    #[test]
    fn steam_session_accepts_active_webhelper_session() {
        let mut value = runtime();
        value.login_status = None;
        value.connection_status = None;
        assert!(steam_session_ready(&value));
    }

    #[test]
    fn steam_session_rejects_cached_credentials_without_transport() {
        let mut value = runtime();
        value.active_session = None;
        value.login_status = None;
        value.connection_status = None;
        assert!(!steam_session_ready(&value));
    }

    #[test]
    fn steam_session_rejects_an_explicitly_inactive_webhelper_session() {
        let mut value = runtime();
        value.active_session = Some(false);
        assert!(!steam_session_ready(&value));
    }

    #[test]
    fn steam_session_rejects_historical_success_without_live_session() {
        let mut value = runtime();
        value.active_session = None;
        assert!(!steam_session_ready(&value));
    }

    #[test]
    fn game_readiness_observation_is_profile_aware() {
        let profile = GenericGameProfile::new("123456", "Steam Game", "Game.exe");
        let value = runtime();
        assert!(game_runtime_ready(&profile, &value));

        let mut no_session = value.clone();
        no_session.active_session = None;
        no_session.login_status = None;
        no_session.connection_status = None;
        assert!(!game_runtime_ready(&profile, &no_session));

        let standalone = profile.with_steam_integration(SteamIntegration::None);
        let mut stopped = value;
        stopped.running = false;
        stopped.active_session = None;
        assert!(game_runtime_ready(&standalone, &stopped));
    }

    #[test]
    fn observation_preserves_profile_and_engine_evidence() {
        let profile = GenericGameProfile::new("123456", "Steam Game", "Game.exe")
            .with_runtime_profile_version("analysis-v1-123456");
        let observation = game_runtime_observation(&profile, &runtime());

        assert_eq!(observation.app_id, "123456");
        assert_eq!(observation.profile_version, "analysis-v1-123456");
        assert_eq!(observation.wine_version.as_deref(), Some("wine-11.10"));
        assert!(observation.steam_session_ready);
        assert!(observation.ready(true));
    }
}
