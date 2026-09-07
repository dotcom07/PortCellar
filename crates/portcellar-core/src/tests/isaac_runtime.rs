use super::*;

#[test]
fn upserts_runtime_options_without_dropping_other_values() {
    let values = BTreeMap::from([
        ("ControllerHotplug".to_string(), "0".to_string()),
        ("SteamCloud".to_string(), "0".to_string()),
    ]);
    assert_eq!(
        upsert_runtime_options(
            "[Options]\nMusicVolume=0.7\nSteamCloud=1\n[Other]\nX=1\n",
            &values
        ),
        "[Options]\nMusicVolume=0.7\nSteamCloud=0\nControllerHotplug=0\n[Other]\nX=1\n"
    );
    assert_eq!(
        upsert_runtime_options("", &values),
        "[Options]\nControllerHotplug=0\nSteamCloud=0\n"
    );
}

#[test]
fn isaac_steam_cloud_defaults_off_and_allows_opt_in() {
    assert_eq!(isaac_steam_cloud_option_value_from(None), "0");
    assert_eq!(isaac_steam_cloud_option_value_from(Some("0")), "0");
    assert_eq!(isaac_steam_cloud_option_value_from(Some("maybe")), "0");
    assert_eq!(isaac_steam_cloud_option_value_from(Some("1")), "1");
    assert_eq!(isaac_steam_cloud_option_value_from(Some("true")), "1");
}

#[test]
fn steam_cef_single_process_flag_is_opt_in() {
    assert!(env_flag_enabled("1"));
    assert!(env_flag_enabled(" true "));
    assert!(env_flag_enabled("YES"));
    assert!(env_flag_enabled("on"));
    assert!(!env_flag_enabled(""));
    assert!(!env_flag_enabled("0"));
    assert!(!env_flag_enabled("false"));
}

#[test]
fn isaac_profile_uses_the_observed_crossover_cef_policy() {
    assert_eq!(
        isaac_profile().steam_cef_policy(),
        SteamCefPolicy::CrossOverCompatible
    );
}

#[test]
fn wine_steam_launch_uses_isaac_profile_env() {
    let runtime = WineSteamRuntime {
        wine: Some(PathBuf::from("/usr/local/bin/wine")),
        wine_version: None,
        windows_version: Some("win10".to_string()),
        dxmt_root: None,
        prefix: PathBuf::from("/tmp/portcellar-prefix"),
        steam_exe: Some(PathBuf::from("/tmp/portcellar-prefix/drive_c/Steam/steam.exe")),
        game_manifest: None,
        game_install_dir: None,
        game_executable: Some(PathBuf::from(
            "/tmp/portcellar-prefix/drive_c/Steam/steamapps/common/The Binding of Isaac Rebirth/isaac-ng.exe",
        )),
        running: false,
        game_running: false,
        launch_ready_after_login: true,
        readiness_issues: Vec::new(),
        logged_in: None,
        active_session: None,
        cached_credentials: None,
        login_status: None,
        connection_status: None,
        cef_status: None,
        mac_window_status: None,
        virtual_desktop: None,
        mac_driver: WineMacDriverConfig {
            allow_immovable_windows: None,
            retina_mode: None,
        },
    };

    let plan = game_wine_steam_launch_plan(&isaac_profile(), &runtime).unwrap();
    let dxmt_config = env::var("DXMT_CONFIG").unwrap_or_else(|_| ISAAC_DXMT_CONFIG.to_string());
    let dxmt_log_level = env::var("DXMT_LOG_LEVEL").unwrap_or_else(|_| "error".to_string());

    assert!(STEAM_WINE_CEF_ARGS
        .iter()
        .all(|arg| plan.args.contains(&OsString::from(*arg))));
    assert!(plan.args.contains(&OsString::from("-applaunch")));
    assert!(plan.args.contains(&OsString::from(ISAAC_APP_ID)));
    assert_eq!(
        plan.envs.get("STEAM_GAME_ID"),
        Some(&ISAAC_APP_ID.to_string())
    );
    assert_eq!(plan.envs.get("SteamAppId"), Some(&ISAAC_APP_ID.to_string()));
    assert_eq!(
        plan.envs.get("SteamGameId"),
        Some(&ISAAC_APP_ID.to_string())
    );
    assert_eq!(
        plan.envs.get("SteamNoOverlayUI").map(String::as_str),
        Some("1")
    );
    assert_eq!(
        plan.envs
            .get("DISABLE_VK_LAYER_VALVE_steam_overlay_1")
            .map(String::as_str),
        Some("1")
    );
    assert_eq!(
        plan.envs.get("WINEDLLOVERRIDES").map(String::as_str),
        Some("gameoverlayrenderer,gameoverlayrenderer64=")
    );
    assert_eq!(plan.envs.get("DXMT_CONFIG"), Some(&dxmt_config));
    assert_eq!(plan.envs.get("DXMT_LOG_LEVEL"), Some(&dxmt_log_level));

    let login_plan = wine_steam_login_plan_for_runtime(&isaac_profile(), &runtime, false).unwrap();
    assert_eq!(
        login_plan.envs.get("SteamNoOverlayUI").map(String::as_str),
        Some("1")
    );
    assert_eq!(
        login_plan.envs.get("WINEDLLOVERRIDES").map(String::as_str),
        Some("gameoverlayrenderer,gameoverlayrenderer64=")
    );
}

#[test]
fn wine_direct_launch_uses_windows_isaac_exe() {
    let isaac_exe = PathBuf::from(
        "/tmp/portcellar-prefix/drive_c/Steam/steamapps/common/The Binding of Isaac Rebirth/isaac-ng.exe",
    );
    let runtime = WineSteamRuntime {
        wine: Some(PathBuf::from("/usr/local/bin/wine")),
        wine_version: None,
        windows_version: Some("win10".to_string()),
        dxmt_root: None,
        prefix: PathBuf::from("/tmp/portcellar-prefix"),
        steam_exe: Some(PathBuf::from(
            "/tmp/portcellar-prefix/drive_c/Steam/steam.exe",
        )),
        game_manifest: None,
        game_install_dir: isaac_exe.parent().map(Path::to_path_buf),
        game_executable: Some(isaac_exe.clone()),
        running: true,
        game_running: false,
        launch_ready_after_login: true,
        readiness_issues: Vec::new(),
        logged_in: Some(true),
        active_session: Some(true),
        cached_credentials: Some(true),
        login_status: Some("authenticated".to_string()),
        connection_status: Some("connected".to_string()),
        cef_status: None,
        mac_window_status: None,
        virtual_desktop: None,
        mac_driver: WineMacDriverConfig {
            allow_immovable_windows: None,
            retina_mode: None,
        },
    };

    let plan = game_wine_direct_launch_plan(&isaac_profile(), &runtime).unwrap();

    assert_eq!(plan.program, PathBuf::from("/usr/local/bin/wine"));
    assert_eq!(plan.args, vec![isaac_exe.as_os_str().to_os_string()]);
    assert_eq!(plan.current_dir, isaac_exe.parent().map(Path::to_path_buf));
    assert_eq!(plan.envs.get("SteamAppId"), Some(&ISAAC_APP_ID.to_string()));
    assert_eq!(
        plan.envs.get("STEAM_GAME_ID"),
        Some(&ISAAC_APP_ID.to_string())
    );
    assert_eq!(
        plan.envs.get("WINEDLLOVERRIDES").map(String::as_str),
        Some("gameoverlayrenderer,gameoverlayrenderer64=")
    );
}

#[test]
fn parses_configured_steam_args() {
    assert_eq!(
        steam_arg_tokens("-tcp -clearbeta"),
        vec![OsString::from("-tcp"), OsString::from("-clearbeta")]
    );
}
