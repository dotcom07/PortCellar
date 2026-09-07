use super::*;

static CURRENT_DIR_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

struct CurrentDirGuard {
    old_dir: PathBuf,
}

impl CurrentDirGuard {
    fn enter(path: &Path) -> Self {
        let old_dir = env::current_dir().unwrap();
        env::set_current_dir(path).unwrap();
        Self { old_dir }
    }
}

impl Drop for CurrentDirGuard {
    fn drop(&mut self) {
        env::set_current_dir(&self.old_dir).unwrap();
    }
}

#[test]
fn reads_windows_version_from_system_registry() {
    let prefix = env::temp_dir().join(format!("portcellar-system-reg-test-{}", std::process::id()));
    fs::create_dir_all(&prefix).unwrap();
    fs::write(
        prefix.join("system.reg"),
        "[Software\\\\Microsoft\\\\Windows NT\\\\CurrentVersion] 1783277586\n\
         \"CurrentBuild\"=\"19045\"\n\
         \"CurrentMajorVersionNumber\"=dword:0000000a\n\
         \"CurrentMinorVersionNumber\"=dword:00000000\n",
    )
    .unwrap();

    assert_eq!(wine_windows_version(&prefix), Some("win10".to_string()));

    fs::remove_dir_all(prefix).unwrap();
}

#[test]
fn detects_state_dxmt_root_for_prefix() {
    let root = env::temp_dir().join(format!("portcellar-dxmt-{}", std::process::id()));
    let prefix = root.join(".portcellar/prefixes/isaac");
    let dxmt = root.join(".portcellar/dxmt");
    fs::create_dir_all(&prefix).unwrap();
    fs::create_dir_all(dxmt.join("i386-windows")).unwrap();
    fs::create_dir_all(dxmt.join("x86_64-windows")).unwrap();
    fs::create_dir_all(dxmt.join("x86_64-unix")).unwrap();

    assert_eq!(find_dxmt_root(&prefix), Some(dxmt));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn dxmt_root_is_prepended_to_wine_dll_path() {
    assert_eq!(
        prepend_env_path(
            Path::new("/tmp/dxmt"),
            Some(std::ffi::OsStr::new("old")),
            ":"
        ),
        "/tmp/dxmt:old"
    );
    assert_eq!(
        prepend_env_path(
            Path::new("/tmp/dxmt"),
            Some(std::ffi::OsStr::new("/tmp/dxmt:old")),
            ":"
        ),
        "/tmp/dxmt:old"
    );
}

#[test]
fn maps_windows_paths_back_into_prefix() {
    assert_eq!(
        prefix_path_from_windows_path(
            Path::new("/tmp/prefix"),
            "C:\\users\\tester\\Documents\\My Games\\Binding of Isaac Rebirth\\"
        ),
        Some(PathBuf::from(
            "/tmp/prefix/drive_c/users/tester/Documents/My Games/Binding of Isaac Rebirth"
        ))
    );
    assert_eq!(
        prefix_path_from_windows_path(Path::new("/tmp"), "Z:\\tmp"),
        Some(PathBuf::from("/tmp"))
    );
}

#[test]
fn maps_host_paths_to_z_drive_arguments() {
    let prefix = PathBuf::from("/tmp/prefix");

    assert_eq!(
        windows_path_for_host_path(
            &prefix,
            Path::new("/tmp/synthetic-user/Downloads/Game/Game.exe")
        ),
        Some("Z:\\tmp\\synthetic-user\\Downloads\\Game\\Game.exe".to_string())
    );
}

#[test]
fn process_matching_uses_the_basename_for_nested_windows_executables() {
    assert_eq!(profile_process_exe_name(r#"bin\\Game.exe"#), "Game.exe");
    assert_eq!(profile_process_exe_name("Game.exe"), "Game.exe");
}

#[test]
fn dll_override_is_prepended_once() {
    assert_eq!(
        prepend_dll_override("gameoverlayrenderer,gameoverlayrenderer64=", None),
        "gameoverlayrenderer,gameoverlayrenderer64="
    );
    assert_eq!(
        prepend_dll_override(
            "gameoverlayrenderer,gameoverlayrenderer64=",
            Some(std::ffi::OsStr::new("dinput8=n,b")),
        ),
        "gameoverlayrenderer,gameoverlayrenderer64=;dinput8=n,b"
    );
    assert_eq!(
        prepend_dll_override(
            "gameoverlayrenderer,gameoverlayrenderer64=",
            Some(std::ffi::OsStr::new(
                "gameoverlayrenderer,gameoverlayrenderer64=;dinput8=n,b"
            )),
        ),
        "gameoverlayrenderer,gameoverlayrenderer64=;dinput8=n,b"
    );
}

#[test]
fn finds_steam_in_short_prefix_layout() {
    let prefix = env::temp_dir().join(format!("portcellar-test-{}", std::process::id()));
    let steam_exe = prefix.join("drive_c/Steam/steam.exe");
    fs::create_dir_all(steam_exe.parent().unwrap()).unwrap();
    fs::write(&steam_exe, "").unwrap();

    assert_eq!(find_windows_steam_exe(&prefix), Some(steam_exe));

    fs::remove_dir_all(prefix).unwrap();
}

#[test]
fn finds_steam_in_program_files_x86_layout() {
    let prefix = env::temp_dir().join(format!(
        "portcellar-program-files-steam-{}",
        std::process::id()
    ));
    let steam_exe = prefix.join("drive_c/Program Files (x86)/Steam/steam.exe");
    fs::create_dir_all(steam_exe.parent().unwrap()).unwrap();
    fs::write(&steam_exe, "").unwrap();

    assert_eq!(find_windows_steam_exe(&prefix), Some(steam_exe));

    fs::remove_dir_all(prefix).unwrap();
}

#[test]
fn finds_project_state_wine_prefix() {
    let project = env::temp_dir().join(format!("portcellar-state-prefix-{}", std::process::id()));
    let prefix = project.join(".portcellar/prefixes/isaac-steam-cx-clean");
    let steam_exe = prefix.join("drive_c/Steam/steam.exe");
    fs::create_dir_all(steam_exe.parent().unwrap()).unwrap();
    fs::write(project.join("Cargo.toml"), "").unwrap();
    fs::create_dir_all(project.join("crates/portcellar-core")).unwrap();
    fs::write(project.join("crates/portcellar-core/Cargo.toml"), "").unwrap();
    fs::write(&steam_exe, "").unwrap();

    {
        let _current_dir_lock = CURRENT_DIR_LOCK.lock().unwrap();
        let _current_dir_guard = CurrentDirGuard::enter(&project);
        assert_eq!(
            project_wine_prefix().and_then(|path| fs::canonicalize(path).ok()),
            fs::canonicalize(&prefix).ok()
        );
    }

    fs::remove_dir_all(project).unwrap();
}

#[test]
fn reads_mac_driver_registry_value() {
    let prefix = env::temp_dir().join(format!("portcellar-reg-test-{}", std::process::id()));
    fs::create_dir_all(&prefix).unwrap();
    fs::write(
        prefix.join("user.reg"),
        "[Software\\\\Wine] 1783277586\n\"Version\"=\"win10\"\n\n\
         [Software\\\\Wine\\\\Mac Driver] 1783277586\n\"AllowImmovableWindows\"=\"N\"\n",
    )
    .unwrap();

    let config = wine_mac_driver_config(&prefix);
    assert_eq!(config.allow_immovable_windows, Some("N".to_string()));
    assert_eq!(config.retina_mode, None);
    assert_eq!(wine_windows_version(&prefix), Some("win10".to_string()));

    fs::remove_dir_all(prefix).unwrap();
}

#[test]
fn parses_virtual_desktop_setting() {
    assert_eq!(
        steam_virtual_desktop_setting_from_value("auto", Some("1728x1117")),
        Some("1728x1117".to_string())
    );
    assert_eq!(
        steam_virtual_desktop_setting_from_value("1600x900", Some("1728x1117")),
        Some("1600x900".to_string())
    );
    assert_eq!(
        steam_virtual_desktop_setting_from_value("off", Some("1728x1117")),
        None
    );
}

#[test]
fn steam_launch_can_use_virtual_desktop() {
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
        virtual_desktop: Some("1600x900".to_string()),
        mac_driver: WineMacDriverConfig {
            allow_immovable_windows: None,
            retina_mode: None,
        },
    };

    let plan = game_wine_steam_launch_plan(&isaac_profile(), &runtime).unwrap();

    assert_eq!(plan.args[0], OsString::from("explorer.exe"));
    assert!(plan.args[1]
        .to_string_lossy()
        .starts_with("/desktop=portcellar-steam,1600x900"));
    assert!(plan.args.contains(&OsString::from("C:\\Steam\\steam.exe")));
}

#[test]
fn finds_wineserver_next_to_wine() {
    let root = env::temp_dir().join(format!("portcellar-wineserver-{}", std::process::id()));
    let bin = root.join("bin");
    fs::create_dir_all(&bin).unwrap();
    let wine = bin.join("wine");
    let wineserver = bin.join("wineserver");
    let winecfg = bin.join("winecfg");
    fs::write(&wine, "").unwrap();
    fs::write(&wineserver, "").unwrap();
    fs::write(&winecfg, "").unwrap();

    assert_eq!(wineserver_for_wine(&wine), Some(wineserver));
    assert_eq!(winecfg_for_wine(&wine), Some(winecfg));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn crossover_configure_plan_keeps_toolchain_and_bottle_together() {
    let root = env::temp_dir().join(format!(
        "portcellar-cross-over-toolchain-{}",
        std::process::id()
    ));
    let crossover = root.join("CrossOver.app/Contents/SharedSupport/CrossOver");
    let bin = crossover.join("bin");
    let x86_64_windows = crossover.join("lib/wine/x86_64-windows");
    let i386_windows = crossover.join("lib/wine/i386-windows");
    let wine_lib = crossover.join("lib/wine");
    let prefix = root.join("Bottles/Example");
    fs::create_dir_all(prefix.join("drive_c/windows")).unwrap();
    fs::create_dir_all(&bin).unwrap();
    fs::create_dir_all(&x86_64_windows).unwrap();
    fs::create_dir_all(&i386_windows).unwrap();
    fs::create_dir_all(&wine_lib).unwrap();

    let wine = bin.join("wine");
    fs::write(&wine, "wine").unwrap();
    fs::write(bin.join("wineserver"), "wineserver").unwrap();
    fs::write(bin.join("wineloader"), "wineloader").unwrap();
    fs::write(x86_64_windows.join("winewrapper.exe"), "wrapper").unwrap();

    let runtime = WineSteamRuntime {
        wine: Some(wine.clone()),
        wine_version: None,
        windows_version: Some("win10".to_string()),
        dxmt_root: None,
        prefix: prefix.clone(),
        steam_exe: None,
        game_manifest: None,
        game_install_dir: None,
        game_executable: None,
        running: false,
        game_running: false,
        launch_ready_after_login: false,
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

    let envs = wine_env(&runtime);
    assert_eq!(envs.get("CX_ROOT"), Some(&crossover.display().to_string()));
    assert_eq!(envs.get("CX_BOTTLE"), Some(&"Example".to_string()));
    assert_eq!(
        envs.get("CX_BOTTLE_PATH"),
        Some(&root.join("Bottles").display().to_string())
    );
    assert_eq!(
        envs.get("WINESERVER"),
        Some(&bin.join("wineserver").display().to_string())
    );

    let plan = wine_windows_version_plan(&wine, &runtime, "win10").unwrap();
    assert_eq!(plan.program, wine);
    assert_eq!(
        plan.args,
        vec![
            OsString::from("--cx-app"),
            OsString::from("winecfg.exe"),
            OsString::from("/v"),
            OsString::from("win10"),
        ]
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn finds_gcenx_style_wine_release_roots() {
    let home = env::temp_dir().join(format!("portcellar-wine-roots-{}", std::process::id()));
    fs::create_dir_all(home.join("wine-11.10")).unwrap();
    fs::create_dir_all(home.join("not-wine-11.10")).unwrap();
    fs::write(home.join("wine-readme"), "").unwrap();

    assert_eq!(wine_release_roots_in(&home), vec![home.join("wine-11.10")]);

    fs::remove_dir_all(home).unwrap();
}

#[test]
fn finds_project_state_engine_roots() {
    let project = env::temp_dir().join(format!("portcellar-state-engines-{}", std::process::id()));
    fs::create_dir_all(project.join(".portcellar/engines/wine-11.10")).unwrap();
    fs::write(project.join("Cargo.toml"), "").unwrap();
    fs::create_dir_all(project.join("crates/portcellar-core")).unwrap();
    fs::write(project.join("crates/portcellar-core/Cargo.toml"), "").unwrap();

    {
        let _current_dir_lock = CURRENT_DIR_LOCK.lock().unwrap();
        let _current_dir_guard = CurrentDirGuard::enter(&project);
        let expected = fs::canonicalize(project.join(".portcellar/engines/wine-11.10")).unwrap();
        assert_eq!(project_state_engine_roots(), vec![expected]);
    }

    fs::remove_dir_all(project).unwrap();
}

#[test]
fn profile_engine_hint_does_not_fall_back_when_missing() {
    let root = env::temp_dir().join(format!("portcellar-pinned-engine-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let pinned = root.join("pinned-wine");
    fs::write(&pinned, "wine").unwrap();
    let pinned_profile = GenericGameProfile::new("123456", "Example Game", "Example.exe")
        .with_wine_engine_path(pinned.clone());
    assert_eq!(find_wine_engine_for(&pinned_profile), Some(pinned));

    let missing = root.join("missing-wine");
    let profile = GenericGameProfile::new("123456", "Example Game", "Example.exe")
        .with_wine_engine_path(missing);

    assert_eq!(find_wine_engine_for(&profile), None);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn selects_an_engine_compatible_with_the_profile_backend() {
    let root = env::temp_dir().join(format!(
        "portcellar-compatible-engine-{}",
        std::process::id()
    ));
    let wine = root.join("Wine Stable.app/Contents/Resources/wine/bin/wine");
    let crossover = root.join("CrossOver.app/Contents/SharedSupport/CrossOver/bin/wine");
    fs::create_dir_all(wine.parent().unwrap()).unwrap();
    fs::create_dir_all(crossover.parent().unwrap()).unwrap();
    fs::write(&wine, "wine").unwrap();
    fs::write(&crossover, "crossover").unwrap();

    let profile = GenericGameProfile::new("123456", "Metal Game", "Game.exe")
        .with_graphics_backend(GraphicsBackend::D3dMetal);

    assert_eq!(
        select_wine_engine_for(&profile, vec![wine, crossover.clone()]),
        Some(crossover)
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn maps_prefix_paths_to_windows_drive_c_paths() {
    let prefix = PathBuf::from("/tmp/portcellar-prefix");
    let steam_exe = prefix.join("drive_c/Steam/steam.exe");

    assert_eq!(
        windows_path_in_prefix(&prefix, &steam_exe),
        Some("C:\\Steam\\steam.exe".to_string())
    );
}

#[test]
fn finds_generic_windows_game_from_manifest() {
    let root = env::temp_dir().join(format!("portcellar-generic-game-{}", std::process::id()));
    let steam_dir = root.join("drive_c/Steam");
    let install_dir = steam_dir.join("steamapps/common/Example Game");
    fs::create_dir_all(&install_dir).unwrap();
    let manifest = steam_dir.join("steamapps/appmanifest_123456.acf");
    fs::write(
        &manifest,
        "\"AppState\"\n{\n\t\"appid\"\t\t\"123456\"\n\t\"installdir\"\t\t\"Example Game\"\n}\n",
    )
    .unwrap();

    let profile = GenericGameProfile::new("123456", "Example Game", "Example.exe");
    assert_eq!(
        find_windows_game_install_dir(&steam_dir, &profile, Some(&manifest)),
        Some(install_dir)
    );

    fs::remove_dir_all(root).unwrap();
}
