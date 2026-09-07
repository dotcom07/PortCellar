use super::*;
use std::fs;

#[test]
fn dependency_verbs_are_stable_for_profiles() {
    assert_eq!(
        RuntimeDependency::Vcrun.winetricks_verb(),
        Some("vcrun2019")
    );
    assert_eq!(
        RuntimeDependency::Vcrun2022.winetricks_verb(),
        Some("vcrun2022")
    );
    assert_eq!(
        RuntimeDependency::D3dCompiler.winetricks_verb(),
        Some("d3dcompiler_47")
    );
    assert_eq!(RuntimeDependency::MediaFoundation.winetricks_verb(), None);
    assert_eq!(RuntimeDependency::OpenAl.winetricks_verb(), Some("openal"));
    assert_eq!(RuntimeDependency::XAudio.winetricks_verb(), Some("xact"));
    assert_eq!(RuntimeDependency::TrebuchetMs.winetricks_verb(), None);
}

#[test]
fn dependency_plan_is_explicit_and_does_not_execute_the_verb() {
    let root = env::temp_dir().join(format!("portcellar-dependency-plan-{}", std::process::id()));
    let engine = root.join("engine");
    fs::create_dir_all(&engine).unwrap();
    fs::File::create(engine.join("wine")).unwrap();
    fs::File::create(engine.join("winetricks")).unwrap();

    let profile = GenericGameProfile::new("123456", "Example Game", "Example.exe")
        .with_dependency(RuntimeDependency::Vcrun);
    let runtime = WineSteamRuntime {
        wine: Some(engine.join("wine")),
        wine_version: None,
        windows_version: Some("win10".to_string()),
        dxmt_root: None,
        prefix: root.join("prefix"),
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

    let plans = wine_dependency_plans_for(&profile, &runtime);
    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0].status, RuntimeDependencyStatus::Planned);
    let command = plans[0].command.as_ref().unwrap();
    assert_eq!(command.program, engine.join("winetricks"));
    assert_eq!(command.args, vec![OsString::from("vcrun2019")]);
    assert_eq!(
        command.envs.get("WINEPREFIX"),
        Some(&runtime.prefix.display().to_string())
    );
    assert!(plans[0].note.contains("not applied automatically"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn media_foundation_requires_a_game_specific_recipe() {
    let profile = GenericGameProfile::new("123456", "Video Game", "Game.exe")
        .with_dependency(RuntimeDependency::MediaFoundation);
    let runtime = WineSteamRuntime {
        wine: None,
        wine_version: None,
        windows_version: None,
        dxmt_root: None,
        prefix: env::temp_dir().join("portcellar-media-foundation-prefix"),
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

    let plans = wine_dependency_plans_for(&profile, &runtime);
    assert_eq!(plans[0].status, RuntimeDependencyStatus::Manual);
    assert!(plans[0].command.is_none());
}

#[test]
fn bottle_mutation_plan_snapshots_before_dependency_commands() {
    let root = env::temp_dir().join(format!("portcellar-bottle-mutation-{}", std::process::id()));
    let prefix = root.join("prefix");
    fs::create_dir_all(&prefix).unwrap();
    let runtime = WineSteamRuntime {
        wine: None,
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
    let profile = GenericGameProfile::new("123456", "Example Game", "Example.exe")
        .with_dependency(RuntimeDependency::MediaFoundation);

    let plan = wine_bottle_mutation_plan(&profile, &runtime, "before-media-foundation").unwrap();

    assert_eq!(plan.prefix, prefix);
    assert_eq!(plan.dependencies.len(), 1);
    assert_eq!(
        plan.snapshot.destination,
        root.join("snapshots/before-media-foundation")
    );
    assert_eq!(
        plan.snapshot.prepare_command.program,
        PathBuf::from("/bin/mkdir")
    );
    assert_eq!(plan.snapshot.command.program, PathBuf::from("/bin/cp"));
    assert!(!plan.snapshot.destination.exists());

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn bottle_mutation_plan_rejects_live_runtime() {
    let root = env::temp_dir().join(format!("portcellar-live-mutation-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let mut runtime = WineSteamRuntime {
        wine: None,
        wine_version: None,
        windows_version: None,
        dxmt_root: None,
        prefix: root.clone(),
        steam_exe: None,
        game_manifest: None,
        game_install_dir: None,
        game_executable: None,
        running: true,
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
    let profile = GenericGameProfile::new("123456", "Example Game", "Example.exe");

    let error = wine_bottle_mutation_plan(&profile, &runtime, "before-change").unwrap_err();
    assert!(error.to_string().contains("stop Wine Steam"));

    runtime.running = false;
    runtime.game_running = true;
    let error = wine_bottle_mutation_plan(&profile, &runtime, "before-change").unwrap_err();
    assert!(error.to_string().contains("stop Wine Steam"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn bottle_snapshot_plan_rejects_existing_destination() {
    let root = env::temp_dir().join(format!(
        "portcellar-existing-snapshot-{}",
        std::process::id()
    ));
    let prefix = root.join("prefix");
    let destination = root.join("snapshots/before-change");
    fs::create_dir_all(&prefix).unwrap();
    fs::create_dir_all(&destination).unwrap();
    let runtime = WineSteamRuntime {
        wine: None,
        wine_version: None,
        windows_version: Some("win10".to_string()),
        dxmt_root: None,
        prefix,
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

    let error = wine_bottle_snapshot_plan(&runtime, "before-change").unwrap_err();
    assert!(error
        .to_string()
        .contains("snapshot destination already exists"));

    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn bottle_snapshot_plan_rejects_dangling_destination_symlink() {
    let root = env::temp_dir().join(format!(
        "portcellar-dangling-snapshot-{}",
        std::process::id()
    ));
    let prefix = root.join("prefix");
    let destination = root.join("snapshots/before-change");
    fs::create_dir_all(&prefix).unwrap();
    fs::create_dir_all(destination.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(root.join("missing"), &destination).unwrap();
    let runtime = WineSteamRuntime {
        wine: None,
        wine_version: None,
        windows_version: Some("win10".to_string()),
        dxmt_root: None,
        prefix,
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

    let error = wine_bottle_snapshot_plan(&runtime, "before-change").unwrap_err();
    assert!(error
        .to_string()
        .contains("snapshot destination already exists"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn bottle_rollback_plan_stages_snapshot_before_moving_prefix() {
    let root = env::temp_dir().join(format!("portcellar-rollback-plan-{}", std::process::id()));
    let prefix = root.join("prefix");
    let snapshot = root.join("snapshots/before-change");
    fs::create_dir_all(&prefix).unwrap();
    fs::create_dir_all(&snapshot).unwrap();
    let runtime = WineSteamRuntime {
        wine: None,
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

    let plan = wine_bottle_rollback_plan(&runtime, "before-change").unwrap();

    assert_eq!(plan.prefix, prefix);
    assert_eq!(plan.snapshot, snapshot);
    assert_eq!(
        plan.backup,
        root.join("rollback-backups/before-change-current")
    );
    assert_eq!(
        plan.staging,
        root.join("rollback-backups/before-change-restore")
    );
    assert_eq!(plan.prepare_command.program, PathBuf::from("/bin/mkdir"));
    assert_eq!(plan.stage_command.program, PathBuf::from("/bin/cp"));
    assert_eq!(plan.backup_command.program, PathBuf::from("/bin/mv"));
    assert_eq!(plan.activate_command.program, PathBuf::from("/bin/mv"));
    assert!(!plan.backup.exists());
    assert!(!plan.staging.exists());

    fs::remove_dir_all(root).unwrap();
}
