#![cfg(unix)]

use portcellar_core::{
    wine_steam_cef_patch_plan, wine_steam_cef_patch_plan_for, GenericGameProfile,
};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

struct EnvGuard {
    key: &'static str,
    previous: Option<OsString>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &Path) -> Self {
        let previous = env::var_os(key);
        env::set_var(key, value);
        Self { key, previous }
    }

    fn remove(key: &'static str) -> Self {
        let previous = env::var_os(key);
        env::remove_var(key);
        Self { key, previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        if let Some(value) = &self.previous {
            env::set_var(self.key, value);
        } else {
            env::remove_var(self.key);
        }
    }
}

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
fn profile_cef_plan_stays_in_selected_prefix_and_isaac_wrapper_uses_default() {
    let root = env::temp_dir().join(format!("portcellar-cef-routing-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let _state_root = EnvGuard::set("PORTCELLAR_STATE_ROOT", &root);
    let _wine = EnvGuard::set("PORTCELLAR_WINE", Path::new("/usr/bin/true"));
    let _wine_prefix = EnvGuard::remove("PORTCELLAR_WINEPREFIX");
    let _steam_exe = EnvGuard::remove("PORTCELLAR_STEAM_EXE");

    let isaac_prefix = root.join("prefixes/isaac-steam");
    let other_prefix = root.join("prefixes/other-game");
    let isaac_steam = synthetic_steam_layout(&isaac_prefix, b'I', "isaac.lock");
    let other_steam = synthetic_steam_layout(&other_prefix, b'O', "other.lock");
    let other_profile = GenericGameProfile::new("123456", "Other Game", "other.exe")
        .with_bottle_name("other-game")
        .with_wine_engine_path("/usr/bin/true");

    let other_plan = wine_steam_cef_patch_plan_for(&other_profile).unwrap();
    assert_eq!(other_plan.steam_dir, other_steam);
    assert_eq!(other_plan.cef_targets.len(), 1);
    assert_eq!(
        other_plan.cef_targets[0].target,
        other_steam.join("bin/cef/cef.win64/steamwebhelper.exe")
    );
    assert_eq!(other_plan.htmlcache_locks.len(), 1);
    assert_eq!(
        other_plan.htmlcache_locks[0],
        other_prefix.join("drive_c/users/test/AppData/Local/Steam/htmlcache/other.lock")
    );
    assert!(!other_plan
        .cef_targets
        .iter()
        .any(|target| target.target.starts_with(&isaac_steam)));
    assert!(!other_plan
        .htmlcache_locks
        .iter()
        .any(|path| path.starts_with(&isaac_prefix)));

    let isaac_plan = wine_steam_cef_patch_plan().unwrap();
    assert_eq!(isaac_plan.steam_dir, isaac_steam);
    assert_eq!(isaac_plan.cef_targets.len(), 1);
    assert_eq!(
        isaac_plan.cef_targets[0].target,
        isaac_steam.join("bin/cef/cef.win64/steamwebhelper.exe")
    );
    assert_eq!(isaac_plan.htmlcache_locks.len(), 1);
    assert_eq!(
        isaac_plan.htmlcache_locks[0],
        isaac_prefix.join("drive_c/users/test/AppData/Local/Steam/htmlcache/isaac.lock")
    );
    assert!(!isaac_plan
        .cef_targets
        .iter()
        .any(|target| target.target.starts_with(&other_steam)));
    assert!(!isaac_plan
        .htmlcache_locks
        .iter()
        .any(|path| path.starts_with(&other_prefix)));

    fs::remove_dir_all(root).unwrap();
}
