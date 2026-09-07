use super::*;

#[test]
fn reads_latest_steam_login_status() {
    let steam_dir = env::temp_dir().join(format!("portcellar-login-test-{}", std::process::id()));
    fs::create_dir_all(steam_dir.join("logs")).unwrap();
    fs::write(
        steam_dir.join("logs/steamui_login.txt"),
        "[ WaitingForCredentials ] Received logon failure response\n\
         [ None ] SetLoginState: WaitingForCredentials - OK\n",
    )
    .unwrap();

    assert_eq!(
        steam_login_status(&steam_dir),
        Some("waiting-for-credentials".to_string())
    );

    fs::remove_dir_all(steam_dir).unwrap();
}

#[test]
fn reads_successful_steam_login_status() {
    let steam_dir = env::temp_dir().join(format!(
        "portcellar-login-success-test-{}",
        std::process::id()
    ));
    fs::create_dir_all(steam_dir.join("logs")).unwrap();
    fs::write(
        steam_dir.join("logs/steamui_login.txt"),
        "[2026-07-06] Client version: 1782866176\n\
         [ None ] SetLoginState: WaitingForCredentials - OK\n\
         [ WaitingForServerResponse ] Received logon success response\n\
         [ WaitingForLibraryReady ] SetLoginState: Success - OK\n",
    )
    .unwrap();

    assert_eq!(steam_login_status(&steam_dir), Some("success".to_string()));
    assert_eq!(
        steam_logged_in_status(None, Some(true), Some("success")),
        Some(true)
    );

    fs::remove_dir_all(steam_dir).unwrap();
}

#[test]
fn reads_steam_connection_status() {
    let steam_dir =
        env::temp_dir().join(format!("portcellar-connection-test-{}", std::process::id()));
    fs::create_dir_all(steam_dir.join("logs")).unwrap();
    fs::write(
        steam_dir.join("logs/connection_log.txt"),
        "Client version: old\n\
         [Logged Off, 0, 0] Sending SteamServerConnectFailure_t No Connection Reconnect\n\
         Client version: new\n\
         Connectivity test: result=Connected (since 0.0s ago), prev=Unknown, in progress=0\n\
         ConnectionCompleted() (146.66.152.52:27018, WebSocket)\n",
    )
    .unwrap();

    assert_eq!(
        steam_connection_status(&steam_dir),
        Some("cm-transport-ready".to_string())
    );

    fs::remove_dir_all(steam_dir).unwrap();
}

#[test]
fn reads_webui_transport_when_cm_status_is_absent() {
    let steam_dir = env::temp_dir().join(format!(
        "portcellar-webui-transport-test-{}",
        std::process::id()
    ));
    fs::create_dir_all(steam_dir.join("logs")).unwrap();
    fs::write(
        steam_dir.join("logs/webhelper_js.txt"),
        "Client version: new\n\
         CWebSocketConnection (steamUI): connection ready\n\
         WebUITransportStore: Connection status: connected\n",
    )
    .unwrap();

    assert_eq!(
        steam_connection_status(&steam_dir),
        Some("webui-transport-ready".to_string())
    );

    fs::remove_dir_all(steam_dir).unwrap();
}

#[test]
fn detects_latest_auth_poll_transport_error() {
    let steam_dir =
        env::temp_dir().join(format!("portcellar-auth-poll-test-{}", std::process::id()));
    fs::create_dir_all(steam_dir.join("logs")).unwrap();
    fs::write(
        steam_dir.join("logs/steamui_login.txt"),
        "SetLoginState: WaitingForCredentials\n",
    )
    .unwrap();
    fs::write(
        steam_dir.join("logs/webhelper_js.txt"),
        "Client version: old\nLogin: Failed to poll auth session\n\
         Client version: new\nLogin: Failed to poll auth session\n",
    )
    .unwrap();

    assert_eq!(
        steam_login_status(&steam_dir),
        Some("auth-poll-transport-error".to_string())
    );
    assert_eq!(
        steam_logged_in_status(None, Some(true), Some("auth-poll-transport-error")),
        Some(false)
    );

    fs::remove_dir_all(steam_dir).unwrap();
}

#[test]
fn readiness_waits_only_on_login_when_runtime_bits_exist() {
    let wine = PathBuf::from("/usr/local/bin/wine");
    let steam = PathBuf::from("/tmp/prefix/drive_c/Steam/steam.exe");
    let isaac = PathBuf::from("/tmp/prefix/drive_c/Steam/steamapps/common/Isaac/isaac-ng.exe");

    assert!(wine_steam_readiness_issues(
        Some(&wine),
        Some(&steam),
        Some(&isaac),
        Some("login-window-ready-macdrv-remapped")
    )
    .is_empty());
    assert_eq!(
        wine_steam_readiness_issues(
            Some(&wine),
            Some(&steam),
            Some(&isaac),
            Some("steamwebhelper restart loop")
        ),
        vec!["Steam CEF unhealthy: steamwebhelper restart loop".to_string()]
    );
}

#[test]
fn reports_cef_ready_without_macos_window() {
    assert_eq!(
        wine_mac_window_status(Some(0), Some("login-window-ready-macdrv-remapped")),
        Some("cef-ready-no-macos-window".to_string())
    );
    assert_eq!(
        wine_mac_window_status(Some(1), Some("login-window-ready-macdrv-remapped")),
        Some("mac-window-visible".to_string())
    );
}

#[test]
fn marks_cef_failures_as_last_status_when_steam_is_not_running() {
    let steam_dir =
        env::temp_dir().join(format!("portcellar-stale-cef-test-{}", std::process::id()));
    fs::create_dir_all(steam_dir.join("logs")).unwrap();
    fs::write(
        steam_dir.join("logs/steamui_html.txt"),
        "Restart webhelper process\nRestart webhelper process\nRestart webhelper process\n",
    )
    .unwrap();
    let mac_driver = WineMacDriverConfig {
        allow_immovable_windows: None,
        retina_mode: None,
    };

    assert_eq!(
        steam_cef_status(&steam_dir, &mac_driver, false),
        Some("last steamwebhelper restart loop".to_string())
    );
    assert_eq!(
        steam_cef_status(&steam_dir, &mac_driver, true),
        Some("steamwebhelper restart loop".to_string())
    );
    assert!(wine_steam_readiness_issues(
        Some(&PathBuf::from("/usr/local/bin/wine")),
        Some(&PathBuf::from("/tmp/prefix/drive_c/Steam/steam.exe")),
        Some(&PathBuf::from(
            "/tmp/prefix/drive_c/Steam/steamapps/common/Isaac/isaac-ng.exe"
        )),
        Some("last steamwebhelper restart loop")
    )
    .is_empty());

    fs::remove_dir_all(steam_dir).unwrap();
}

#[test]
fn cef_status_uses_latest_steamui_session() {
    let steam_dir =
        env::temp_dir().join(format!("portcellar-latest-cef-test-{}", std::process::id()));
    fs::create_dir_all(steam_dir.join("logs")).unwrap();
    fs::write(
        steam_dir.join("logs/steamui_html.txt"),
        "Restart webhelper process\nRestart webhelper process\nRestart webhelper process\n\
         [2026-07-06] Client version: 1782866176\nBrowserReady: handle:65536\n",
    )
    .unwrap();
    let mac_driver = WineMacDriverConfig {
        allow_immovable_windows: None,
        retina_mode: None,
    };

    assert_eq!(
        steam_cef_status(&steam_dir, &mac_driver, true),
        Some("login-window-ready".to_string())
    );
    assert_eq!(
        latest_log_segment(
            "old\nClient version: first\nold\nClient version: second\nnew",
            "Client version:"
        ),
        "Client version: second\nnew"
    );

    fs::remove_dir_all(steam_dir).unwrap();
}

#[test]
fn login_failure_overrides_cached_credential_marker() {
    assert_eq!(
        steam_logged_in_status(None, Some(true), Some("logon-failure")),
        Some(false)
    );
    assert_eq!(
        steam_logged_in_status(None, None, Some("waiting-for-credentials")),
        Some(false)
    );
    assert_eq!(
        steam_logged_in_status(Some(true), None, Some("logon-failure")),
        Some(true)
    );
    assert_eq!(
        steam_logged_in_status(None, Some(false), Some("success")),
        Some(true)
    );
}

#[test]
fn cached_credential_marker_requires_enabled_login_values() {
    let values = parse_key_values(
        "\"RememberPassword\"\t\t\"0\"\n\
         \"AllowAutoLogin\"\t\t\"0\"\n\
         \"MostRecent\"\t\t\"1\"",
    );

    assert!(!quoted_bool(&values, "RememberPassword"));
    assert!(quoted_bool(&values, "MostRecent"));
}

#[test]
fn reads_webhelper_session_state_without_exposing_steamid() {
    assert_eq!(
        steam_webhelper_session_state_from_processes(
            "C:\\Steam\\steamwebhelper.exe -steamid=0 --type=renderer",
        ),
        Some(false)
    );
    assert_eq!(
        steam_webhelper_session_state_from_processes(
            "C:\\Steam\\steamwebhelper.exe -steamid=123456 --type=renderer",
        ),
        Some(true)
    );
}
