use super::*;
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

#[test]
fn generic_profile_owns_portable_game_metadata() {
    let profile = GenericGameProfile::new("123456", "Example Game", "Example.exe")
        .with_install_dir_hint("Example Game")
        .with_windows_depot_id("123457")
        .with_wine_windows_version("win7")
        .with_graphics_backend(GraphicsBackend::WineD3D)
        .with_launch_argument("-windowed")
        .with_runtime_option("EnableIntro", "0")
        .with_runtime_environment("EXAMPLE_MODE", "compat");

    assert_eq!(profile.app_id(), "123456");
    assert_eq!(profile.name(), "Example Game");
    assert_eq!(profile.bottle_name(), Some("game-123456"));
    assert_eq!(profile.windows_exe(), "Example.exe");
    assert_eq!(profile.launch_arguments(), vec!["-windowed"]);
    assert_eq!(profile.install_dir_hint(), "Example Game");
    assert_eq!(profile.windows_depot_id(), Some("123457"));
    assert_eq!(profile.wine_windows_version(), "win7");
    assert_eq!(profile.graphics_backend(), GraphicsBackend::WineD3D);
    assert_eq!(profile.steam_integration(), SteamIntegration::Required);
    assert_eq!(profile.steam_cef_policy(), SteamCefPolicy::WineDefault);
    assert_eq!(
        profile.runtime_options().get("EnableIntro"),
        Some(&"0".to_string())
    );
    assert_eq!(
        profile.runtime_environment().get("EXAMPLE_MODE"),
        Some(&"compat".to_string())
    );
    let policy = profile.runtime_policy();
    assert_eq!(policy.windows_version, "win7");
    assert_eq!(policy.graphics_backend, GraphicsBackend::WineD3D);
    assert_eq!(
        policy.runtime_options.get("EnableIntro"),
        Some(&"0".to_string())
    );
}

#[test]
fn generic_profile_can_pin_a_wine_engine_path() {
    let path = PathBuf::from("/tmp/wine-staging/bin/wine");
    let profile = GenericGameProfile::new("123456", "Example Game", "Example.exe")
        .with_wine_engine_path(path.clone());

    assert_eq!(profile.wine_engine_path_hint(), Some(path.as_path()));
    assert_eq!(profile.runtime_policy().wine_engine_path, Some(path));
}

#[test]
fn generic_profile_toml_round_trip_preserves_runtime_policy() {
    let profile = GenericGameProfile::new("123456", "Example Game", "bin/Example.exe")
        .with_bottle_name("example-game")
        .with_windows_depot_id("123457")
        .with_install_dir_hint("Example Game")
        .with_windows_install_path(r#"C:\Games\Example"#)
        .with_runtime_options_path(r#"C:\Games\Example\options.ini"#)
        .with_runtime_profile_version("analysis-v1-123456")
        .with_wine_engine_path("/tmp/wine/bin/wine")
        .with_wine_windows_version("win7")
        .with_graphics_backend(GraphicsBackend::WineD3D)
        .with_steam_integration(SteamIntegration::Optional)
        .with_steam_cef_policy(SteamCefPolicy::CrossOverCompatible)
        .with_launch_argument("-safe-mode")
        .with_runtime_option("EnableIntro", "0")
        .with_runtime_environment("EXAMPLE_MODE", "compat")
        .with_runtime_artifact(GameRuntimeArtifact {
            id: "example-driver".to_string(),
            source_path: ".portcellar/build/example/driver.dll".to_string(),
            target_path: "Apps/driver.dll".to_string(),
            architecture: Some("pe32-i386".to_string()),
        })
        .with_binary_patch(GameBinaryPatch {
            target_path: "Example.exe".to_string(),
            kind: GameBinaryPatchKind::LargeAddressAware,
        })
        .with_capability(GameCapability::Audio)
        .with_dependency(RuntimeDependency::Vcrun);

    let serialized = profile.to_profile_toml().unwrap();
    let restored = GenericGameProfile::from_profile_toml(&serialized).unwrap();

    assert_eq!(restored.runtime_policy(), profile.runtime_policy());
    assert_eq!(
        restored.runtime_options_path_hint(),
        Some(r#"C:\Games\Example\options.ini"#)
    );
    assert_eq!(restored.launch_arguments(), vec!["-safe-mode"]);
    assert_eq!(
        restored.to_profile_document().format_version,
        "game-profile-v1"
    );
}

#[test]
fn generic_profile_rejects_unsafe_runtime_artifact_target() {
    let error = GenericGameProfile::from_profile_toml(
        r#"
format_version = "game-profile-v1"
app_id = "local"
name = "Example"
bottle_name = "local"
windows_exe = "Game.exe"
runtime_profile_version = "profile-v1"
wine_windows_version = "win10"
graphics_backend = "scgl"
steam_integration = "none"
windows_install_path = "Z:\\Games\\Example"

[[runtime_artifacts]]
id = "driver"
source_path = "driver.dll"
target_path = "../driver.dll"
"#,
    )
    .unwrap_err();
    assert!(error.to_string().contains("target_path"));
}

#[test]
fn generic_profile_toml_defaults_cef_policy_for_older_profiles() {
    let profile = GenericGameProfile::from_profile_toml(
        r#"
format_version = "game-profile-v1"
app_id = "123456"
name = "Example Game"
bottle_name = "game-123456"
windows_exe = "Game.exe"
runtime_profile_version = "profile-v1"
wine_windows_version = "win10"
graphics_backend = "auto"
steam_integration = "required"
"#,
    )
    .unwrap();

    assert_eq!(
        profile.runtime_policy().steam_cef_policy,
        SteamCefPolicy::WineDefault
    );
}

#[test]
fn generic_profile_toml_rejects_unknown_format_version() {
    let mut document =
        GenericGameProfile::new("123456", "Example Game", "Example.exe").to_profile_document();
    document.format_version = "game-profile-v0".to_string();

    let error = GenericGameProfile::from_profile_document(document).unwrap_err();
    assert!(error
        .to_string()
        .contains("unsupported game profile format"));
}

#[test]
fn generic_profile_toml_rejects_non_numeric_steam_app_id() {
    let mut document =
        GenericGameProfile::new("123456", "Example Game", "Example.exe").to_profile_document();
    document.app_id = "not-a-steam-id".to_string();

    let error = GenericGameProfile::from_profile_document(document).unwrap_err();
    assert!(error.to_string().contains("numeric app_id"));
}

#[test]
fn generic_profile_toml_rejects_windows_path_escape() {
    let mut document =
        GenericGameProfile::new("123456", "Example Game", "Example.exe").to_profile_document();
    document.windows_exe = r#"..\..\Windows\system32\cmd.exe"#.to_string();

    let error = GenericGameProfile::from_profile_document(document).unwrap_err();
    assert!(error
        .to_string()
        .contains("relative Windows executable path"));
}

#[test]
fn generic_profile_toml_rejects_runtime_options_path_escape() {
    let mut document =
        GenericGameProfile::new("123456", "Example Game", "Example.exe").to_profile_document();
    document.runtime_options_path = Some(r#"C:\Games\..\unsafe.ini"#.to_string());

    let error = GenericGameProfile::from_profile_document(document).unwrap_err();
    assert!(error.to_string().contains("runtime_options_path"));
}

#[test]
fn generic_profile_toml_rejects_runtime_options_without_path() {
    let mut document =
        GenericGameProfile::new("123456", "Example Game", "Example.exe").to_profile_document();
    document
        .runtime_options
        .insert("EnableIntro".to_string(), "0".to_string());

    let error = GenericGameProfile::from_profile_document(document).unwrap_err();
    assert!(error
        .to_string()
        .contains("runtime_options require runtime_options_path"));
}

#[test]
fn generic_profile_toml_rejects_nul_launch_argument() {
    let mut document =
        GenericGameProfile::new("123456", "Example Game", "Example.exe").to_profile_document();
    document.launch_arguments.push("bad\0argument".to_string());

    let error = GenericGameProfile::from_profile_document(document).unwrap_err();
    assert!(error.to_string().contains("launch_arguments"));
}

#[test]
fn generic_profile_uses_a_safe_per_game_bottle_name() {
    let profile = GenericGameProfile::new("my/game", "Example Game", "Example.exe")
        .with_bottle_name("Example Game / unsafe");

    assert_eq!(profile.bottle_name(), Some("Example-Game---unsafe"));
    assert_eq!(
        profile_bottle_path(&profile, Path::new("/tmp/prefixes")),
        Some(PathBuf::from("/tmp/prefixes/Example-Game---unsafe"))
    );
}

#[test]
fn generic_dxmt_profile_exposes_dxmt_contract() {
    let profile = GenericGameProfile::new("123456", "Example Game", "Example.exe")
        .with_dxmt_config("d3d11.preferredMaxFrameRate=60;");

    assert_eq!(profile.graphics_backend(), GraphicsBackend::Dxmt);
    assert_eq!(
        profile.dxmt_config(),
        Some("d3d11.preferredMaxFrameRate=60;")
    );
}

#[test]
fn non_steam_profile_does_not_require_a_steam_process() {
    let profile = GenericGameProfile::new("local", "Standalone Game", "Game.exe")
        .with_steam_integration(SteamIntegration::None)
        .with_windows_install_path(r#"C:GamesStandalone"#);

    assert_eq!(profile.steam_integration(), SteamIntegration::None);
    assert_eq!(
        profile.windows_install_path_hint(),
        Some(r#"C:GamesStandalone"#)
    );
    assert!(wine_steam_readiness_issues_for(
        Some(&PathBuf::from("/usr/local/bin/wine")),
        None,
        Some(&PathBuf::from(
            "/tmp/prefix/drive_c/Games/Standalone/Game.exe"
        )),
        None,
        profile.name(),
        false,
    )
    .is_empty());
}

#[test]
fn non_steam_profile_accepts_a_host_z_drive_path() {
    let profile = GenericGameProfile::new("local", "Standalone Game", "Game.exe")
        .with_steam_integration(SteamIntegration::None)
        .with_windows_install_path(r#"Z:\Users\tester\Downloads\Game"#);

    let serialized = profile.to_profile_toml().unwrap();
    let restored = GenericGameProfile::from_profile_toml(&serialized).unwrap();

    assert_eq!(
        restored.windows_install_path_hint(),
        Some(r#"Z:\Users\tester\Downloads\Game"#)
    );
}

#[test]
fn generic_profile_reuses_wine_direct_launch_path() {
    let profile = GenericGameProfile::new("local", "Standalone Game", "Game.exe")
        .with_steam_integration(SteamIntegration::None)
        .with_launch_argument("-safe-mode")
        .with_windows_install_path(r#"C:\Games\Standalone"#);
    let game_executable = PathBuf::from("/tmp/prefix/drive_c/Games/Standalone/Game.exe");
    let runtime = test_runtime(
        Some(PathBuf::from("/usr/local/bin/wine")),
        None,
        Some(game_executable.clone()),
    );

    let plan = game_wine_direct_launch_plan(&profile, &runtime).unwrap();
    assert_eq!(plan.program, PathBuf::from("/usr/local/bin/wine"));
    assert_eq!(
        plan.args,
        vec![
            game_executable.as_os_str().to_os_string(),
            OsString::from("-safe-mode")
        ]
    );
    assert_eq!(
        plan.envs.get("WINEPREFIX"),
        Some(&"/tmp/prefix".to_string())
    );
    assert!(!plan.envs.contains_key("SteamAppId"));
}

#[test]
fn auto_backend_does_not_inject_dxmt_dlls() {
    let profile = GenericGameProfile::new("123456", "Example Game", "Game.exe")
        .with_graphics_backend(GraphicsBackend::Auto);
    let mut runtime = test_runtime(
        Some(PathBuf::from("/usr/local/bin/wine")),
        Some(PathBuf::from("/tmp/prefix/drive_c/Steam/steam.exe")),
        Some(PathBuf::from("/tmp/prefix/drive_c/Game.exe")),
    );
    runtime.dxmt_root = Some(PathBuf::from("/tmp/dxmt"));

    let envs = game_wine_launch_env(&runtime, &profile);
    assert!(!envs.contains_key("WINEDLLPATH_PREPEND"));

    let dxmt_profile = profile.with_dxmt_config("d3d11.preferredMaxFrameRate=60;");
    let dxmt_envs = game_wine_launch_env(&runtime, &dxmt_profile);
    assert_eq!(
        dxmt_envs.get("WINEDLLPATH_PREPEND"),
        Some(&"/tmp/dxmt".to_string())
    );
}

#[test]
fn selected_open_source_runtime_layers_are_injected_from_state_runtime() {
    let root =
        std::env::temp_dir().join(format!("portcellar-runtime-layers-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let runtime_root = root.join(".portcellar/runtime");
    for path in [
        runtime_root.join("dxvk/x64/bin"),
        runtime_root.join("dxvk/x86/bin"),
        runtime_root.join("faudio/x64/bin"),
        runtime_root.join("gstreamer/lib/gstreamer-1.0"),
        runtime_root.join("moltenvk/lib"),
        root.join(".portcellar/prefixes/example"),
    ] {
        fs::create_dir_all(path).unwrap();
    }
    for path in [
        runtime_root.join("dxvk/x64/bin/d3d11.dll"),
        runtime_root.join("dxvk/x86/bin/d3d11.dll"),
        runtime_root.join("faudio/x64/bin/FAudio.dll"),
        runtime_root.join("gstreamer/lib/libgstreamer-1.0.0.dylib"),
        runtime_root.join("moltenvk/lib/libMoltenVK.dylib"),
    ] {
        fs::write(path, b"runtime").unwrap();
    }

    let profile = GenericGameProfile::new("123456", "Example Game", "Game.exe")
        .with_graphics_backend(GraphicsBackend::Dxvk)
        .with_dependency(RuntimeDependency::XAudio)
        .with_dependency(RuntimeDependency::MediaFoundation);
    let mut runtime = test_runtime(
        Some(PathBuf::from("/usr/local/bin/wine")),
        Some(PathBuf::from("/tmp/prefix/drive_c/Steam/steam.exe")),
        Some(PathBuf::from("/tmp/prefix/drive_c/Game.exe")),
    );
    runtime.prefix = root.join(".portcellar/prefixes/example");

    let envs = game_wine_launch_env(&runtime, &profile);

    assert_eq!(
        envs.get("WINEDLLPATH_PREPEND"),
        Some(&format!(
            "{}:{}:{}",
            runtime_root.join("faudio/x64/bin").display(),
            runtime_root.join("dxvk/x64/bin").display(),
            runtime_root.join("dxvk/x86/bin").display()
        ))
    );
    assert_eq!(
        envs.get("WINEDLLPATH"),
        Some(&format!(
            "{}:{}:{}",
            runtime_root.join("faudio/x64/bin").display(),
            runtime_root.join("dxvk/x64/bin").display(),
            runtime_root.join("dxvk/x86/bin").display()
        ))
    );
    assert!(envs
        .get("WINEDLLOVERRIDES")
        .unwrap()
        .starts_with("d3d8,d3d9,d3d10core,d3d11,dxgi=n,b;"));
    assert!(envs
        .get("DYLD_FALLBACK_LIBRARY_PATH")
        .unwrap()
        .starts_with(&format!(
            "{}:{}",
            runtime_root.join("gstreamer/lib").display(),
            runtime_root.join("moltenvk/lib").display()
        )));
    assert_eq!(
        envs.get("GST_PLUGIN_PATH_1_0"),
        Some(
            &runtime_root
                .join("gstreamer/lib/gstreamer-1.0")
                .display()
                .to_string()
        )
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn profile_environment_cannot_override_runtime_prefix() {
    let profile = GenericGameProfile::new("123456", "Example Game", "Game.exe")
        .with_runtime_environment("WINEPREFIX", "/unsafe/prefix")
        .with_runtime_environment("GAME_MODE", "compat");
    let runtime = test_runtime(
        Some(PathBuf::from("/usr/local/bin/wine")),
        Some(PathBuf::from("/tmp/prefix/drive_c/Steam/steam.exe")),
        Some(PathBuf::from("/tmp/prefix/drive_c/Game.exe")),
    );

    let envs = game_wine_launch_env(&runtime, &profile);

    assert_eq!(envs.get("WINEPREFIX"), Some(&"/tmp/prefix".to_string()));
    assert_eq!(envs.get("GAME_MODE"), Some(&"compat".to_string()));
}

#[test]
fn generic_profile_runtime_options_use_declared_windows_path() {
    let profile = GenericGameProfile::new("123456", "Example Game", "Game.exe")
        .with_runtime_options_path(r#"C:\Games\Example\options.ini"#)
        .with_runtime_option("EnableIntro", "0");
    let runtime = test_runtime(
        Some(PathBuf::from("/usr/local/bin/wine")),
        Some(PathBuf::from("/tmp/prefix/drive_c/Steam/steam.exe")),
        Some(PathBuf::from("/tmp/prefix/drive_c/Game.exe")),
    );

    let plan = game_runtime_profile_plan_for(&profile, &runtime).unwrap();

    assert_eq!(
        plan.options_path,
        PathBuf::from("/tmp/prefix/drive_c/Games/Example/options.ini")
    );
    assert_eq!(plan.values.get("EnableIntro"), Some(&"0".to_string()));
}

#[test]
fn generic_profile_builds_a_host_installer_plan_without_steam_env() {
    let installer =
        std::env::temp_dir().join(format!("portcellar-installer-{}.exe", std::process::id()));
    fs::write(&installer, "installer").unwrap();
    let profile = GenericGameProfile::new("local", "Standalone Game", "Game.exe")
        .with_steam_integration(SteamIntegration::None)
        .with_runtime_environment("INSTALLER_MODE", "compat");
    let runtime = test_runtime(Some(PathBuf::from("/usr/local/bin/wine")), None, None);

    let plan = game_installer_plan_for(&profile, &runtime, &installer, &[OsString::from("/quiet")])
        .unwrap();

    assert_eq!(plan.installer, installer);
    assert_eq!(plan.command.program, PathBuf::from("/usr/local/bin/wine"));
    assert_eq!(plan.command.args[1], OsString::from("/quiet"));
    assert_eq!(
        plan.command.envs.get("WINEPREFIX"),
        Some(&"/tmp/prefix".to_string())
    );
    assert_eq!(
        plan.command.envs.get("INSTALLER_MODE"),
        Some(&"compat".to_string())
    );
    assert!(!plan.command.envs.contains_key("SteamAppId"));

    fs::remove_file(installer).unwrap();
}

#[test]
fn generic_profile_rejects_non_windows_installer_extension() {
    let installer =
        std::env::temp_dir().join(format!("portcellar-installer-{}.txt", std::process::id()));
    fs::write(&installer, "not an installer").unwrap();
    let profile = GenericGameProfile::new("local", "Standalone Game", "Game.exe")
        .with_steam_integration(SteamIntegration::None);
    let runtime = test_runtime(Some(PathBuf::from("/usr/local/bin/wine")), None, None);

    let error = game_installer_plan_for(&profile, &runtime, &installer, &[]).unwrap_err();

    assert!(error.to_string().contains(".exe, .msi, .bat, or .cmd"));
    fs::remove_file(installer).unwrap();
}

#[test]
fn generic_profile_builds_a_profile_specific_steam_login_plan() {
    let profile = GenericGameProfile::new("123456", "Example Game", "Game.exe");
    let runtime = test_runtime(
        Some(PathBuf::from("/usr/local/bin/wine")),
        Some(PathBuf::from("/tmp/prefix/drive_c/Steam/steam.exe")),
        Some(PathBuf::from("/tmp/prefix/drive_c/Game.exe")),
    );

    let plan = wine_steam_login_plan_for_runtime(&profile, &runtime, false).unwrap();
    assert_eq!(plan.program, PathBuf::from("/usr/local/bin/wine"));
    assert_eq!(plan.envs.get("SteamAppId"), Some(&"123456".to_string()));
    assert!(!plan.envs.contains_key(STEAM_CEF_POLICY_ENV));
    assert!(!plan.args.iter().any(|arg| arg == "-noreactlogin"));
}

#[test]
fn generic_profile_cross_over_cef_policy_is_explicit_in_launch_env() {
    let profile = GenericGameProfile::new("123456", "Example Game", "Game.exe")
        .with_steam_cef_policy(SteamCefPolicy::CrossOverCompatible)
        .with_runtime_environment("WINEMSYNC", "1");
    let runtime = test_runtime(
        Some(PathBuf::from("/usr/local/bin/wine")),
        Some(PathBuf::from("/tmp/prefix/drive_c/Steam/steam.exe")),
        Some(PathBuf::from("/tmp/prefix/drive_c/Game.exe")),
    );

    let plan = wine_steam_login_plan_for_runtime(&profile, &runtime, false).unwrap();

    assert_eq!(
        plan.envs.get(STEAM_CEF_POLICY_ENV),
        Some(&STEAM_CEF_CROSSOVER_POLICY_VALUE.to_string())
    );
    assert_eq!(plan.envs.get("WINEMSYNC"), Some(&"1".to_string()));
}

#[test]
fn generic_profile_appends_launch_arguments_to_wine_steam_plan() {
    let profile = GenericGameProfile::new("123456", "Example Game", "Game.exe")
        .with_launch_argument("-windowed")
        .with_launch_argument("-dx11");
    let runtime = test_runtime(
        Some(PathBuf::from("/usr/local/bin/wine")),
        Some(PathBuf::from("/tmp/prefix/drive_c/Steam/steam.exe")),
        Some(PathBuf::from("/tmp/prefix/drive_c/Game.exe")),
    );

    let plan = game_wine_steam_launch_plan(&profile, &runtime).unwrap();

    assert!(plan.args.ends_with(&[
        OsString::from("-silent"),
        OsString::from("-applaunch"),
        OsString::from("123456"),
        OsString::from("-windowed"),
        OsString::from("-dx11"),
    ]));
}

#[test]
fn preflight_blocks_declared_anti_cheat_without_claiming_support() {
    let profile = GenericGameProfile::new("123456", "Protected Game", "Game.exe")
        .with_capability(GameCapability::AntiCheat);
    let runtime = test_runtime(
        Some(PathBuf::from("/usr/local/bin/wine")),
        Some(PathBuf::from("/tmp/prefix/drive_c/Steam/steam.exe")),
        Some(PathBuf::from(
            "/tmp/prefix/drive_c/Steam/steamapps/common/Protected/Game.exe",
        )),
    );

    let preflight = game_runtime_preflight(&profile, &runtime);

    assert!(!preflight.ready());
    assert!(preflight
        .blockers
        .iter()
        .any(|blocker| blocker.contains("anti-cheat")));
}

#[test]
fn preflight_blocks_backend_unsupported_by_selected_wine_engine() {
    let profile = GenericGameProfile::new("123456", "Metal Game", "Game.exe")
        .with_graphics_backend(GraphicsBackend::D3dMetal);
    let runtime = test_runtime(
        Some(PathBuf::from("/usr/local/bin/wine")),
        Some(PathBuf::from("/tmp/prefix/drive_c/Steam/steam.exe")),
        Some(PathBuf::from("/tmp/prefix/drive_c/Game.exe")),
    );

    let preflight = game_runtime_preflight(&profile, &runtime);

    assert!(!preflight.ready());
    assert_eq!(preflight.engine, Some(EngineKind::Wine));
    assert_eq!(
        preflight.backend_compatibility.unwrap().status,
        CompatibilityStatus::Unsupported
    );
}

#[test]
fn preflight_blocks_an_orphaned_steam_webhelper_tree() {
    let profile = GenericGameProfile::new("123456", "Example Game", "Game.exe");
    let mut runtime = test_runtime(
        Some(PathBuf::from("/usr/local/bin/wine")),
        Some(PathBuf::from("/tmp/prefix/drive_c/Steam/steam.exe")),
        Some(PathBuf::from("/tmp/prefix/drive_c/Game.exe")),
    );
    runtime
        .readiness_issues
        .push("orphaned Steam WebHelper process tree detected".to_string());

    let preflight = game_runtime_preflight(&profile, &runtime);

    assert!(!preflight.ready());
    assert!(preflight
        .blockers
        .iter()
        .any(|blocker| blocker.contains("orphaned WebHelper")));
}

fn test_runtime(
    wine: Option<PathBuf>,
    steam_exe: Option<PathBuf>,
    game_executable: Option<PathBuf>,
) -> WineSteamRuntime {
    WineSteamRuntime {
        wine,
        wine_version: None,
        windows_version: Some("win10".to_string()),
        dxmt_root: None,
        prefix: PathBuf::from("/tmp/prefix"),
        steam_exe,
        game_manifest: None,
        game_install_dir: game_executable
            .as_ref()
            .and_then(|path| path.parent().map(PathBuf::from)),
        game_executable,
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
    }
}
