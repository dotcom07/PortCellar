#![cfg(unix)]
use portcellar_core::{
    game_launch_plan, game_launch_plan_with_stage, prepare_game_runtime_stage, GameRuntimeArtifact,
    GenericGameProfile, LaunchMode, SteamIntegration,
};
use std::{fs, os::unix::fs::symlink};

// One test owns this process's environment. No installed games or Wine are used.
#[test]
fn plans_do_not_stage_and_preparation_preserves_existing_saves() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("portcellar-stage-preserve-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let source = root.join("source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("Game.exe"), b"synthetic executable").unwrap();
    fs::write(root.join("driver.dll"), b"synthetic driver").unwrap();
    std::env::set_var("PORTCELLAR_STATE_ROOT", root.join("state"));
    std::env::set_var("PORTCELLAR_RUNTIME_ROOT", root.join("runtime"));
    std::env::set_var("PORTCELLAR_WINEPREFIX", root.join("prefix"));
    std::env::set_var("PORTCELLAR_WINE", "/usr/bin/false");
    std::env::remove_var("PORTCELLAR_STAGE_REFRESH");
    let profile = GenericGameProfile::new("local-stage", "Example", "Game.exe")
        .with_steam_integration(SteamIntegration::None)
        .with_windows_install_path(format!("Z:{}", source.display()))
        .with_runtime_artifact(GameRuntimeArtifact {
            id: "driver".into(),
            source_path: root.join("driver.dll").display().to_string(),
            target_path: "driver.dll".into(),
            architecture: None,
        });
    let source_before = fs::read(source.join("Game.exe")).unwrap();
    for mode in [LaunchMode::WineDirect, LaunchMode::WineSteam] {
        let _ = game_launch_plan(&profile, mode);
        let _ = game_launch_plan_with_stage(&profile, mode, true);
        let _ = game_launch_plan_with_stage(&profile, mode, false);
        assert!(
            !root.join("runtime").exists(),
            "planning materialized a stage"
        );
        assert!(!root.join("state").exists());
        assert!(!root.join("prefix").exists());
    }
    let stage = prepare_game_runtime_stage(&profile).unwrap().unwrap();
    let plan = game_launch_plan(&profile, LaunchMode::WineDirect).unwrap();
    assert_eq!(plan.current_dir.as_ref(), Some(&stage.game_root));
    assert_eq!(
        fs::read(stage.game_root.join("driver.dll")).unwrap(),
        b"synthetic driver"
    );
    fs::write(stage.game_root.join("save.dat"), b"irreplaceable save").unwrap();
    assert!(prepare_game_runtime_stage(&profile).is_ok());
    std::env::set_var("PORTCELLAR_STAGE_REFRESH", "1");
    assert!(prepare_game_runtime_stage(&profile).is_err());
    std::env::remove_var("PORTCELLAR_STAGE_REFRESH");
    fs::write(root.join("driver.dll"), b"changed driver").unwrap();
    assert!(prepare_game_runtime_stage(&profile).is_err());
    let _ = game_launch_plan(&profile, LaunchMode::WineDirect);
    assert_eq!(
        fs::read(stage.game_root.join("save.dat")).unwrap(),
        b"irreplaceable save"
    );
    assert_eq!(
        fs::read(stage.game_root.join("driver.dll")).unwrap(),
        b"synthetic driver"
    );
    assert_eq!(fs::read(source.join("Game.exe")).unwrap(), source_before);

    std::env::set_var("PORTCELLAR_RUNTIME_ROOT", source.join("nested"));
    assert!(prepare_game_runtime_stage(&profile).is_err());
    assert!(!source.join("nested").exists());
    let case_alias = root.join("SOURCE");
    if case_alias.is_dir() {
        std::env::set_var("PORTCELLAR_RUNTIME_ROOT", case_alias.join("nested"));
        assert!(prepare_game_runtime_stage(&profile).is_err());
        assert!(!source.join("nested").exists());
    }
    symlink(root.join("runtime"), root.join("alias")).unwrap();
    std::env::set_var("PORTCELLAR_RUNTIME_ROOT", root.join("alias"));
    assert!(prepare_game_runtime_stage(&profile).is_err());

    std::env::set_var("PORTCELLAR_RUNTIME_ROOT", root.join("failed"));
    let missing_executable = profile.clone().with_runtime_artifact(GameRuntimeArtifact {
        id: "conflict".into(),
        source_path: root.join("driver.dll").display().to_string(),
        target_path: "Game.exe/nested.dll".into(),
        architecture: None,
    });
    let error = prepare_game_runtime_stage(&missing_executable).unwrap_err();
    assert!(error.to_string().contains("retained"), "{error}");
    assert!(!root.join("failed/local-stage/game").exists());
    assert!(fs::read_dir(root.join("failed/local-stage"))
        .unwrap()
        .next()
        .is_some());
    assert_eq!(fs::read(source.join("Game.exe")).unwrap(), source_before);
    std::env::set_var("PORTCELLAR_RUNTIME_ROOT", root.join("patch-failed"));
    let bad_patch = profile
        .clone()
        .with_binary_patch(portcellar_core::GameBinaryPatch {
            target_path: "Game.exe".into(),
            kind: portcellar_core::GameBinaryPatchKind::LargeAddressAware,
        });
    assert!(prepare_game_runtime_stage(&bad_patch).is_err());
    assert!(!root.join("patch-failed").exists());
    assert_eq!(fs::read(source.join("Game.exe")).unwrap(), source_before);
    fs::remove_dir_all(root).unwrap();
}
