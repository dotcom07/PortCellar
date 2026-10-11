#![cfg(target_os = "macos")]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Command, Output},
};

fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_portcellar"))
        .args(args)
        .output()
        .unwrap()
}
fn succeeds(args: &[&str]) -> String {
    let output = cli(args);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn cli_owns_a_standalone_installation_without_executing_during_planning() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("portcellar-managed-cli-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    fs::create_dir_all(root.join("modules/example/profiles")).unwrap();
    fs::write(root.join("modules/example/module.toml"), "format_version = \"portcellar-module-v1\"\nid = \"example\"\nname = \"Example\"\nrevision = 1\n[[variants]]\nid = \"windows\"\nname = \"Windows\"\n[[variants.profiles]]\nid = \"default\"\npath = \"profiles/game.toml\"\n").unwrap();
    let profile = portcellar_core::GenericGameProfile::new("local-example", "Example", "Game.exe")
        .with_steam_integration(portcellar_core::SteamIntegration::None);
    profile
        .write_profile_toml(&root.join("modules/example/profiles/game.toml"))
        .unwrap();
    fs::create_dir(root.join("source")).unwrap();
    fs::write(root.join("source/Game.exe"), b"fixture").unwrap();
    fs::write(root.join("source/helper.dll"), b"original").unwrap();
    fs::create_dir_all(root.join("engine/bin")).unwrap();
    let wine = root.join("engine/bin/wine");
    script(&wine, "if [ \"$1\" = wineboot.exe ]; then mkdir -p \"$WINEPREFIX/drive_c\"; touch \"$WINEPREFIX/system.reg\"; fi\nexit 0\n");
    script(&root.join("engine/bin/wineserver"), "exit 0\n");
    let installation = root.join("install-a");
    let path = installation.to_str().unwrap();
    succeeds(&[
        "game",
        "installation",
        "create",
        "--root",
        path,
        "--id",
        "install-a",
        "--module-root",
        root.join("modules").to_str().unwrap(),
        "--module",
        "example",
        "--variant",
        "windows",
        "--profile",
        "default",
        "--source-root",
        root.join("source").to_str().unwrap(),
        "--wine",
        wine.to_str().unwrap(),
    ]);
    let plan = succeeds(&["game", "installation", "plan", "--root", path]);
    assert!(plan.contains("G:"));
    assert!(!installation.join("prefix").exists());
    assert!(!installation.join("stage").exists());
    assert!(!installation.join(".portcellar-locks").exists());
    assert!(
        !cli(&["game", "installation", "launch", "--root", path, "--detach"])
            .status
            .success()
    );
    succeeds(&["game", "installation", "prepare", "--root", path]);
    let artifact = root.join("helper-patched.dll");
    fs::write(&artifact, b"replacement").unwrap();
    succeeds(&[
        "game",
        "installation",
        "replace",
        "--root",
        path,
        "--target",
        "helper.dll",
        "--artifact",
        artifact.to_str().unwrap(),
        "--original-sha256",
        "0682c5f2076f099c34cfdd15a9e063849ed437a49677e6fcc5b4198c76575be5",
        "--replacement-sha256",
        "95713e9cbdd1dfcb2d4080c2537f418d43ca0da25f0d7d6631f4f7c97b89dc47",
    ]);
    assert_eq!(
        fs::read(installation.join("stage/game/helper.dll")).unwrap(),
        b"replacement"
    );
    succeeds(&[
        "game",
        "installation",
        "restore",
        "--root",
        path,
        "--target",
        "helper.dll",
    ]);
    assert_eq!(
        fs::read(installation.join("stage/game/helper.dll")).unwrap(),
        b"original"
    );
    succeeds(&[
        "game",
        "installation",
        "launch",
        "--root",
        path,
        "--timeout-seconds",
        "5",
    ]);
    assert!(installation.join("logs/wine.log").is_file());
    let installer = root.join("setup.exe");
    fs::write(&installer, b"installer").unwrap();
    succeeds(&[
        "game",
        "installation",
        "install",
        "--root",
        path,
        "--installer",
        installer.to_str().unwrap(),
        "--timeout-seconds",
        "5",
    ]);
    succeeds(&[
        "game",
        "installation",
        "recover",
        "--root",
        path,
        "--timeout-seconds",
        "5",
    ]);
    assert_eq!(
        fs::read(root.join("source/helper.dll")).unwrap(),
        b"original"
    );
    fs::remove_dir_all(root).unwrap();
}

fn script(path: &Path, text: &str) {
    fs::write(path, format!("#!/bin/sh\nset -eu\n{text}")).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}
