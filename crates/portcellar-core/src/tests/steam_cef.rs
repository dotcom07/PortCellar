use super::*;

#[test]
fn session_reset_candidates_preserve_steamapps() {
    let prefix = env::temp_dir().join(format!("portcellar-reset-test-{}", std::process::id()));
    let steam_dir = prefix.join("drive_c/Steam");
    fs::create_dir_all(prefix.join("drive_c/users/test/AppData/Local/Steam")).unwrap();

    let candidates = steam_session_reset_candidates(&prefix, &steam_dir);

    assert!(candidates.contains(&steam_dir.join("config/loginusers.vdf")));
    assert!(!candidates.contains(&steam_dir.join("config/config.vdf")));
    assert!(candidates.contains(&prefix.join("drive_c/users/test/AppData/Local/Steam/htmlcache")));
    assert!(!candidates.iter().any(|path| path.ends_with("steamapps")));

    fs::remove_dir_all(prefix).unwrap();
}

#[test]
fn session_reset_backup_paths_are_namespaced() {
    let prefix = PathBuf::from("/tmp/prefix");
    let steam_dir = prefix.join("drive_c/Steam");
    let backup_dir = steam_dir.join("portcellar-backups/session");

    assert_eq!(
        steam_session_backup_path(
            &backup_dir,
            &prefix,
            &steam_dir,
            &steam_dir.join("config/loginusers.vdf")
        ),
        backup_dir.join("Steam/config/loginusers.vdf")
    );
    assert_eq!(
        steam_session_backup_path(
            &backup_dir,
            &prefix,
            &steam_dir,
            &prefix.join("drive_c/users/test/AppData/Local/Steam/htmlcache")
        ),
        backup_dir.join("drive_c/users/test/AppData/Local/Steam/htmlcache")
    );
}

#[test]
fn steam_cef_patch_plan_finds_helpers_and_locks() {
    let prefix = env::temp_dir().join(format!("portcellar-cef-test-{}", std::process::id()));
    let steam_dir = prefix.join("drive_c/Steam");
    let cef64 = steam_dir.join("bin/cef/cef.win64");
    let cef32 = steam_dir.join("bin/cef/cef.win7");
    let htmlcache = prefix.join("drive_c/users/test/AppData/Local/Steam/htmlcache/GPUCache");
    fs::create_dir_all(&cef64).unwrap();
    fs::create_dir_all(&cef32).unwrap();
    fs::create_dir_all(&htmlcache).unwrap();
    fs::write(cef64.join("steamwebhelper.exe"), vec![0; 600_000]).unwrap();
    fs::write(cef32.join("steamwebhelper.exe"), vec![0; 600_000]).unwrap();
    fs::write(htmlcache.join("SingletonLock"), "").unwrap();
    fs::write(htmlcache.join("cache.lock"), "").unwrap();

    let plan = steam_cef_patch_plan_for(&prefix, &steam_dir);

    assert_eq!(plan.cef_targets.len(), 2);
    assert!(plan
        .cef_targets
        .iter()
        .any(|target| target.arch == SteamWebHelperArch::X86_64));
    assert!(plan
        .cef_targets
        .iter()
        .any(|target| target.arch == SteamWebHelperArch::X86));
    assert_eq!(plan.htmlcache_locks.len(), 2);

    fs::remove_dir_all(prefix).unwrap();
}

#[test]
fn detects_current_steamwebhelper_wrapper_marker() {
    let dir = env::temp_dir().join(format!("portcellar-wrapper-marker-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let current = dir.join("current.exe");
    let old = dir.join("old.exe");
    fs::write(
        &current,
        [b"prefix".as_slice(), STEAM_WEBHELPER_WRAPPER_MARKER].concat(),
    )
    .unwrap();
    fs::write(&old, b"portcellar-old-wrapper").unwrap();

    assert!(is_current_steamwebhelper_wrapper(&current));
    assert!(!is_current_steamwebhelper_wrapper(&old));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn wrapper_source_has_an_explicit_cross_over_cef_policy() {
    assert!(STEAM_WEBHELPER_WRAPPER_SOURCE.contains(STEAM_CEF_POLICY_ENV));
    assert!(STEAM_WEBHELPER_WRAPPER_SOURCE.contains(STEAM_CEF_CROSSOVER_POLICY_VALUE));
    assert!(STEAM_WEBHELPER_WRAPPER_SOURCE.contains("--in-process-gpu"));
    assert!(STEAM_WEBHELPER_WRAPPER_SOURCE.contains("--use-angle=swiftshader-webgl"));
    assert!(STEAM_WEBHELPER_WRAPPER_SOURCE.contains("--use-gl=angle"));
    assert!(STEAM_WEBHELPER_WRAPPER_SOURCE.contains("--disable-component-update"));
}
