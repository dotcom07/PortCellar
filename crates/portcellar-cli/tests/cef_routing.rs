#![cfg(unix)]

use portcellar_core::GenericGameProfile;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn synthetic_steam_layout(prefix: &Path, helper_byte: u8, lock_name: &str) -> PathBuf {
    let steam_dir = prefix.join("drive_c/Steam");
    let cef_dir = steam_dir.join("bin/cef/cef.win64");
    let htmlcache = prefix.join("drive_c/users/test/AppData/Local/Steam/htmlcache");
    fs::create_dir_all(&cef_dir).unwrap();
    fs::create_dir_all(&htmlcache).unwrap();
    fs::write(steam_dir.join("steam.exe"), b"synthetic-steam").unwrap();
    fs::write(
        cef_dir.join("steamwebhelper.exe"),
        vec![helper_byte; 600_000],
    )
    .unwrap();
    fs::write(htmlcache.join(lock_name), b"synthetic-lock").unwrap();
    steam_dir
}

#[test]
fn cli_profile_cef_plan_uses_selected_prefix() {
    let root = std::env::temp_dir().join(format!("portcellar-cli-cef-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();

    let state_root = root.join("state");
    let isaac_prefix = state_root.join("prefixes/isaac-steam");
    let other_prefix = state_root.join("prefixes/other-game");
    let isaac_steam = synthetic_steam_layout(&isaac_prefix, b'I', "isaac.lock");
    let other_steam = synthetic_steam_layout(&other_prefix, b'O', "other.lock");
    let other_game = other_steam.join("steamapps/common/Other Game");
    fs::create_dir_all(&other_game).unwrap();
    fs::write(other_game.join("other.exe"), b"synthetic-game").unwrap();
    fs::write(
        other_steam.join("steamapps/appmanifest_123456.acf"),
        "\"AppState\"\n{\n\t\"appid\"\t\t\"123456\"\n\t\"installdir\"\t\t\"Other Game\"\n}\n",
    )
    .unwrap();

    let profile = GenericGameProfile::new("123456", "Other Game", "other.exe")
        .with_bottle_name("other-game")
        .with_install_dir_hint("Other Game")
        .with_wine_engine_path("/usr/bin/true");
    let profile_path = root.join("other-profile.toml");
    fs::write(&profile_path, profile.to_profile_toml().unwrap()).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_portcellar"))
        .args([
            "game",
            "launch",
            "--from-profile",
            profile_path.to_str().unwrap(),
            "--mode",
            "wine-steam",
            "--dry-run",
        ])
        .env("PORTCELLAR_STATE_ROOT", &state_root)
        .env_remove("PORTCELLAR_WINEPREFIX")
        .env_remove("PORTCELLAR_STEAM_EXE")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let other_target = other_steam.join("bin/cef/cef.win64/steamwebhelper.exe");
    let other_lock =
        other_prefix.join("drive_c/users/test/AppData/Local/Steam/htmlcache/other.lock");
    assert!(stdout.contains(&format!("Steam dir: {}", other_steam.display())));
    assert!(stdout.contains(&format!("patch {}", other_target.display())));
    assert!(stdout.contains(&format!("remove htmlcache lock {}", other_lock.display())));
    assert!(!stdout.contains(&isaac_steam.display().to_string()));
    assert!(!stdout.contains(&isaac_prefix.display().to_string()));

    fs::remove_dir_all(root).unwrap();
}
