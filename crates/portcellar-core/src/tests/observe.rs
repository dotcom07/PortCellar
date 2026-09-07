use super::*;

#[test]
fn core_observation_readiness_matches_profile_steam_requirement() {
    let profile = GenericGameProfile::new("local", "Standalone", "Game.exe")
        .with_steam_integration(SteamIntegration::None);
    let observation = GameRuntimeObservation {
        app_id: "local".to_string(),
        profile_name: "Standalone".to_string(),
        profile_version: "generic-v1".to_string(),
        prefix: PathBuf::from("/tmp/prefix"),
        wine: None,
        wine_version: None,
        windows_version: None,
        steam_exe: None,
        game_executable: Some(PathBuf::from("/tmp/prefix/drive_c/Game.exe")),
        steam_running: false,
        game_running: true,
        active_session: None,
        steam_session_ready: false,
        logged_in: None,
        cached_credentials: None,
        login_status: None,
        connection_status: None,
        cef_status: None,
    };

    assert!(observation.ready(profile.steam_integration() == SteamIntegration::Required));
}
