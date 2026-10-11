#![cfg(target_os = "macos")]

use portcellar_core::{GameModuleCatalog, InstallationRequest, ManagedInstallation};
use std::{
    fs,
    os::unix::fs::{symlink, PermissionsExt},
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

const ORIGINAL: &str = "0682c5f2076f099c34cfdd15a9e063849ed437a49677e6fcc5b4198c76575be5";
const REPLACEMENT: &str = "95713e9cbdd1dfcb2d4080c2537f418d43ca0da25f0d7d6631f4f7c97b89dc47";

struct Fixture {
    root: PathBuf,
    catalog: GameModuleCatalog,
    request: InstallationRequest,
}
impl Fixture {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("portcellar-managed-{name}-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        fs::create_dir_all(root.join("modules/example/profiles")).unwrap();
        fs::write(root.join("modules/example/module.toml"), "format_version = \"portcellar-module-v1\"\nid = \"example\"\nname = \"Example\"\nrevision = 1\n[[variants]]\nid = \"windows\"\nname = \"Windows\"\n[[variants.profiles]]\nid = \"default\"\npath = \"profiles/game.toml\"\n").unwrap();
        fs::write(root.join("modules/example/profiles/game.toml"), "format_version = \"game-profile-v1\"\napp_id = \"local-example\"\nname = \"Example\"\nbottle_name = \"example\"\nwindows_exe = \"Game.exe\"\nruntime_profile_version = \"example-1\"\nwine_windows_version = \"win10\"\ngraphics_backend = \"auto\"\nsteam_integration = \"none\"\n[runtime_environment]\nWINEPREFIX = \"/invalid\"\nHOME = \"/invalid\"\nEXAMPLE_FLAG = \"enabled\"\n").unwrap();
        let source = root.join("Source café");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("Game.exe"), b"synthetic game").unwrap();
        fs::write(source.join("helper.dll"), b"original").unwrap();
        let bin = root.join("engine/bin");
        fs::create_dir_all(&bin).unwrap();
        write_script(&bin.join("wine"), "if [ \"$1\" = wineboot.exe ]; then mkdir -p \"$WINEPREFIX/drive_c\"; touch \"$WINEPREFIX/system.reg\"; exit 0; fi\nif [ \"$1\" = reg.exe ]; then exit 0; fi\nprintf '%s\\n' \"$WINEPREFIX\" \"$HOME\" \"$EXAMPLE_FLAG\" > \"$WINEPREFIX/observed\"\n");
        write_script(&bin.join("wineserver"), "exit 0\n");
        let catalog = GameModuleCatalog::load(&root.join("modules")).unwrap();
        let request = InstallationRequest {
            id: "install-a".into(),
            module_id: "example".into(),
            variant_id: "windows".into(),
            profile_id: "default".into(),
            source_root: source,
            wine: bin.join("wine"),
            dxmt_root: None,
        };
        Self {
            root,
            catalog,
            request,
        }
    }
    fn create(&self) -> ManagedInstallation {
        ManagedInstallation::create(&self.root.join("install-a"), &self.catalog, &self.request)
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}
fn write_script(path: &Path, text: &str) {
    fs::write(path, format!("#!/bin/sh\nset -eu\n{text}")).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn owned_plans_are_pure_and_execution_preserves_sources_and_saves() {
    let f = Fixture::new("paths");
    let i = f.create();
    assert!(ManagedInstallation::create(&i.root(), &f.catalog, &f.request).is_err());
    let plan = i.launch_plan().unwrap();
    assert_eq!(plan.current_dir, Some(i.root().join("stage/game")));
    assert!(!i.root().join("prefix").exists());
    assert!(!i.root().join("stage").exists());
    assert!(!i.root().join(".portcellar-locks").exists());
    assert_eq!(
        plan.envs["WINEPREFIX"],
        i.root().join("prefix").to_str().unwrap()
    );
    assert_eq!(plan.envs["HOME"], i.root().join("home").to_str().unwrap());
    assert_eq!(plan.envs["EXAMPLE_FLAG"], "enabled");
    assert_eq!(i.launch(Duration::from_secs(5)).unwrap(), 0);
    assert!(i.root().join("prefix/observed").exists());
    assert!(
        portcellar_core::run_plan(&plan).is_err(),
        "legacy execution bypassed managed ownership"
    );
    fs::write(i.root().join("stage/game/save.dat"), b"save").unwrap();
    i.prepare().unwrap();
    assert_eq!(
        fs::read(i.root().join("stage/game/save.dat")).unwrap(),
        b"save"
    );
    assert_eq!(
        fs::read(f.request.source_root.join("helper.dll")).unwrap(),
        b"original"
    );
    fs::write(f.request.source_root.join("Game.exe"), b"updated source").unwrap();
    assert!(i.prepare().is_err());
    assert_eq!(
        fs::read(i.root().join("stage/game/save.dat")).unwrap(),
        b"save"
    );
    assert_eq!(
        fs::read(i.root().join("stage/game/Game.exe")).unwrap(),
        b"synthetic game"
    );
}

#[test]
fn replacements_require_exact_content_and_restore_only_owned_changes() {
    let f = Fixture::new("replace");
    let i = f.create();
    i.prepare().unwrap();
    let artifact = f.root.join("replacement.dll");
    fs::write(&artifact, b"replacement").unwrap();
    assert!(i
        .replace("helper.dll", &artifact, Some(ORIGINAL), ORIGINAL)
        .is_err());
    assert_eq!(
        fs::read(i.root().join("stage/game/helper.dll")).unwrap(),
        b"original"
    );
    i.replace("helper.dll", &artifact, Some(ORIGINAL), REPLACEMENT)
        .unwrap();
    i.replace("helper.dll", &artifact, Some(ORIGINAL), REPLACEMENT)
        .unwrap();
    assert_eq!(
        fs::read(i.root().join("stage/game/helper.dll")).unwrap(),
        b"replacement"
    );
    assert_eq!(
        fs::read(f.request.source_root.join("helper.dll")).unwrap(),
        b"original"
    );
    fs::write(
        i.root().join("stage/game/helper.dll"),
        b"updated by publisher",
    )
    .unwrap();
    assert!(i.restore("helper.dll").is_err());
    assert_eq!(
        fs::read(i.root().join("stage/game/helper.dll")).unwrap(),
        b"updated by publisher"
    );
    fs::write(i.root().join("stage/game/helper.dll"), b"replacement").unwrap();
    i.restore("helper.dll").unwrap();
    i.restore("helper.dll").unwrap();
    assert_eq!(
        fs::read(i.root().join("stage/game/helper.dll")).unwrap(),
        b"original"
    );
    for target in [
        "../helper.dll",
        "C:/helper.dll",
        "helper.dll/../bad",
        "/helper.dll",
        "a\\b",
    ] {
        assert!(i
            .replace(target, &artifact, Some(ORIGINAL), REPLACEMENT)
            .is_err());
    }
    symlink(&f.request.source_root, i.root().join("stage/game/link")).unwrap();
    assert!(i
        .replace("link/helper.dll", &artifact, Some(ORIGINAL), REPLACEMENT)
        .is_err());
}

#[test]
fn runtime_updates_and_independent_installations_do_not_share_mutations() {
    let f = Fixture::new("identity");
    let i = f.create();
    let mut request = f.request.clone();
    request.id = "install-b".into();
    let b = ManagedInstallation::create(&f.root.join("install-b"), &f.catalog, &request).unwrap();
    i.prepare().unwrap();
    b.prepare().unwrap();
    fs::write(i.root().join("stage/game/save.dat"), b"first").unwrap();
    assert!(!b.root().join("stage/game/save.dat").exists());
    fs::write(&f.request.wine, b"changed runtime").unwrap();
    assert!(i.launch_plan().is_err());
    assert!(i.prepare().is_err());
}

#[test]
fn installer_plans_are_pure_and_copy_the_exact_selected_exe() {
    let f = Fixture::new("installer");
    let i = f.create();
    let installer = f.root.join("Setup café.exe");
    fs::write(&installer, b"installer fixture").unwrap();
    let plan = i.installer_plan(&installer, &["/quiet".into()]).unwrap();
    assert!(!i.root().join("prefix").exists());
    assert_eq!(plan.args[1], "/quiet");
    assert_eq!(
        i.install(&installer, &["/quiet".into()], Duration::from_secs(5))
            .unwrap(),
        0
    );
    let copied = fs::read_dir(i.root().join("prefix/drive_c/portcellar/installers"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert_eq!(fs::read(copied).unwrap(), b"installer fixture");
    assert_eq!(fs::read(&installer).unwrap(), b"installer fixture");
    assert!(!i.root().join("stage").exists());
}

#[test]
fn a_launcher_exit_keeps_the_lock_until_the_wine_server_finishes() {
    let f = Fixture::new("server");
    write_script(&f.request.wine, "if [ \"$1\" = wineboot.exe ]; then mkdir -p \"$WINEPREFIX/drive_c\"; touch \"$WINEPREFIX/system.reg\"; exit 0; fi\nif [ \"$1\" = reg.exe ]; then exit 0; fi\ntouch \"$WINEPREFIX/game-exited\"\n");
    write_script(&f.request.wine.parent().unwrap().join("wineserver"), "if [ -f \"$WINEPREFIX/game-exited\" ]; then touch \"$WINEPREFIX/server-waiting\"; sleep 1; fi\n");
    let i = f.create();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "lock_child", "--nocapture"])
        .env("PORTCELLAR_TEST_MANAGED_CHILD", &i.root())
        .spawn()
        .unwrap();
    let start = std::time::Instant::now();
    while !i.root().join("prefix/server-waiting").exists() {
        assert!(start.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(i.prepare().unwrap_err().to_string().contains("busy"));
    assert!(child.wait().unwrap().success());
    assert!(!i.root().join(".portcellar-session.toml").exists());
}

#[test]
fn changed_bindings_and_unknown_nested_profile_fields_are_rejected() {
    let f = Fixture::new("binding");
    let i = f.create();
    let path = i.root().join(".portcellar-installation.toml");
    let text = fs::read_to_string(&path).unwrap();
    fs::write(
        &path,
        text.replace("[profile]", "[profile]\nunknown = true"),
    )
    .unwrap();
    assert!(ManagedInstallation::load(&i.root()).is_err());
    assert!(i.prepare().is_err());
}

#[test]
fn dxmt_selects_game_architecture_and_pins_every_payload_file() {
    for executable in ["Game.exe", "bin/Game.exe"] {
        let mut f = Fixture::new("dxmt");
        let profile_path = f.root.join("modules/example/profiles/game.toml");
        let text = fs::read_to_string(&profile_path)
            .unwrap()
            .replace("graphics_backend = \"auto\"", "graphics_backend = \"dxmt\"")
            .replace(
                "windows_exe = \"Game.exe\"",
                &format!("windows_exe = '{executable}'"),
            );
        fs::write(&profile_path, text).unwrap();
        f.catalog = GameModuleCatalog::load(&f.root.join("modules")).unwrap();
        let payload = f.root.join("DXMT");
        for arch in ["x32", "x64"] {
            fs::create_dir_all(payload.join(arch)).unwrap();
            for name in ["d3d10core.dll", "d3d11.dll", "dxgi.dll", "winemetal.dll"] {
                fs::write(payload.join(arch).join(name), arch.as_bytes()).unwrap();
            }
        }
        let mut pe = vec![0u8; 96];
        pe[..2].copy_from_slice(b"MZ");
        pe[60..64].copy_from_slice(&64u32.to_le_bytes());
        pe[64..68].copy_from_slice(b"PE\0\0");
        pe[68..70].copy_from_slice(&0x8664u16.to_le_bytes());
        let executable = f.request.source_root.join(executable);
        fs::create_dir_all(executable.parent().unwrap()).unwrap();
        fs::write(&executable, pe).unwrap();
        let dll = executable
            .parent()
            .unwrap()
            .strip_prefix(&f.request.source_root)
            .unwrap()
            .join("d3d11.dll");
        let dll = dll.to_str().unwrap();
        f.request.dxmt_root = Some(payload.clone());
        let i = f.create();
        let plan = i.launch_plan().unwrap();
        assert!(plan.envs["WINEDLLOVERRIDES"].contains("d3d11,dxgi=n,b"));
        assert_eq!(i.launch(Duration::from_secs(5)).unwrap(), 0);
        assert_eq!(
            fs::read(i.root().join("stage/game").join(dll)).unwrap(),
            b"x64"
        );
        assert!(!f.request.source_root.join(dll).exists());
        i.restore(dll).unwrap();
        assert!(!i.root().join("stage/game").join(dll).exists());
        fs::write(payload.join("x32/d3d11.dll"), b"updated").unwrap();
        assert!(i.prepare().is_err());
    }
}

#[test]
fn module_registry_settings_reach_wine_as_typed_arguments() {
    let mut f = Fixture::new("registry");
    let path = f.root.join("modules/example/profiles/game.toml");
    let mut text = fs::read_to_string(&path).unwrap();
    text.push_str("\n[[registry_entries]]\nkey = 'HKCU\\Software\\Example'\nname = 'DisplayName'\nvalue = { type = 'string', data = 'café' }\n[[registry_entries]]\nkey = 'HKCU\\Software\\Example'\nname = 'Count'\nvalue = { type = 'dword', data = 42 }\n");
    fs::write(&path, &text).unwrap();
    f.catalog = GameModuleCatalog::load(&f.root.join("modules")).unwrap();
    write_script(&f.request.wine, "if [ \"$1\" = wineboot.exe ]; then mkdir -p \"$WINEPREFIX/drive_c\"; touch \"$WINEPREFIX/system.reg\"; fi\nif [ \"$1\" = reg.exe ]; then printf '%s\\n' \"$@\" >> \"$WINEPREFIX/registry-arguments\"; fi\n");
    let i = f.create();
    i.launch(Duration::from_secs(5)).unwrap();
    let arguments = fs::read_to_string(i.root().join("prefix/registry-arguments")).unwrap();
    assert!(arguments.contains("DisplayName\n/t\nREG_SZ\n/d\ncafé\n"));
    assert!(arguments.contains("Count\n/t\nREG_DWORD\n/d\n42\n"));
    for invalid in [
        text.replace("data = 42", "data = -1"),
        text.replace("name = 'Count'", "name = 'Count'\nunknown = true"),
        text.replace("HKCU\\Software\\Example", "INVALID\\Example"),
    ] {
        fs::write(&path, invalid).unwrap();
        assert!(GameModuleCatalog::load(&f.root.join("modules")).is_err());
    }
}

#[test]
fn invalid_execution_bounds_do_not_prepare_game_or_prefix() {
    let f = Fixture::new("bounds");
    let i = f.create();
    assert!(i.launch(Duration::ZERO).is_err());
    assert!(!i.root().join("stage").exists());
    assert!(!i.root().join("prefix").exists());
}

#[test]
fn process_failures_preserve_codes_and_unconfirmed_sessions() {
    for (body, code) in [
        ("exit 7", Some(7)),
        ("kill -TERM $$", None),
        ("touch \"$WINEPREFIX/active\"; exec sleep 5", None),
    ] {
        let f = Fixture::new("process-failure");
        write_script(&f.request.wine, &format!("if [ \"$1\" = wineboot.exe ]; then mkdir -p \"$WINEPREFIX/drive_c\"; touch \"$WINEPREFIX/system.reg\"; exit 0; fi\nif [ \"$1\" = reg.exe ]; then exit 0; fi\n{body}\n"));
        write_script(
            &f.root.join("engine/bin/wineserver"),
            "test ! -f \"$WINEPREFIX/active\"\n",
        );
        let i = f.create();
        let result = i.launch(Duration::from_secs(if body.contains("sleep") {
            1
        } else {
            5
        }));
        match code {
            Some(code) => assert_eq!(result.unwrap(), code),
            None => assert!(result.is_err()),
        }
        if i.root().join("prefix/active").exists() {
            assert!(i.root().join(".portcellar-session.toml").exists());
            assert!(i.prepare().is_err());
            assert!(i.recover(Duration::from_secs(1)).is_err());
            fs::remove_file(i.root().join("prefix/active")).unwrap();
            i.recover(Duration::from_secs(1)).unwrap();
        }
        assert!(!i.root().join(".portcellar-session.toml").exists());
        assert_eq!(
            fs::read(f.request.source_root.join("Game.exe")).unwrap(),
            b"synthetic game"
        );
    }
}

#[test]
fn unsupported_managed_profiles_are_refused_before_creation() {
    let mut f = Fixture::new("unsupported");
    let path = f.root.join("modules/example/profiles/game.toml");
    let original = fs::read_to_string(&path).unwrap();
    for text in [
        original.replace("graphics_backend = \"auto\"", "graphics_backend = \"dxvk\""),
        original.replace(
            "[runtime_environment]",
            "dependencies = ['x-audio']\n[runtime_environment]",
        ),
        original.replace(
            "[runtime_environment]",
            "runtime_options_path = 'C:\\options.ini'\n[runtime_environment]",
        ),
    ] {
        fs::write(&path, text).unwrap();
        f.catalog = GameModuleCatalog::load(&f.root.join("modules")).unwrap();
        assert!(
            ManagedInstallation::create(&f.root.join("install-a"), &f.catalog, &f.request).is_err()
        );
        assert!(!f.root.join("install-a").exists());
    }
}

#[test]
fn a_replaced_installation_root_is_refused_before_lock_creation() {
    let f = Fixture::new("root-replaced");
    let i = f.create();
    let retained = f.root.join("retained");
    fs::rename(i.root(), &retained).unwrap();
    symlink(&retained, i.root()).unwrap();
    assert!(i.prepare().is_err());
    assert!(!retained.join(".portcellar-locks").exists());
    assert!(!retained.join("stage").exists());
}

#[test]
fn legacy_reset_cannot_mutate_a_managed_installation() {
    let f = Fixture::new("legacy-reset");
    let i = f.create();
    i.prepare().unwrap();
    let source = i.root().join("stage/game/helper.dll");
    let plan = portcellar_core::SteamSessionResetPlan {
        backup_dir: i.root().join("legacy-backup"),
        targets: vec![portcellar_core::SteamSessionResetTarget {
            source: source.clone(),
            backup: i.root().join("legacy-backup/helper.dll"),
        }],
    };
    assert!(portcellar_core::apply_steam_session_reset(&plan).is_err());
    assert_eq!(fs::read(source).unwrap(), b"original");
    assert!(!plan.backup_dir.exists());
    fs::remove_file(i.root().join(".portcellar-installation.toml")).unwrap();
    symlink(
        "missing-binding",
        i.root().join(".portcellar-installation.toml"),
    )
    .unwrap();
    assert!(portcellar_core::apply_steam_session_reset(&plan).is_err());
    assert!(!plan.backup_dir.exists());
}

#[test]
fn a_managed_root_cannot_contain_an_independently_locked_installation() {
    let f = Fixture::new("nested");
    let i = f.create();
    let mut request = f.request.clone();
    request.id = "install-b".into();
    assert!(
        ManagedInstallation::create(&i.root().join("install-b"), &f.catalog, &request).is_err()
    );
    assert!(!i.root().join("install-b").exists());
}

#[test]
fn nested_windows_executable_paths_are_staged_and_addressed_consistently() {
    let mut f = Fixture::new("nested-exe");
    fs::create_dir(f.request.source_root.join("bin")).unwrap();
    fs::rename(
        f.request.source_root.join("Game.exe"),
        f.request.source_root.join("bin/Game.exe"),
    )
    .unwrap();
    let path = f.root.join("modules/example/profiles/game.toml");
    let text = fs::read_to_string(&path).unwrap().replace(
        "windows_exe = \"Game.exe\"",
        "windows_exe = 'bin\\Game.exe'",
    );
    fs::write(&path, text).unwrap();
    f.catalog = GameModuleCatalog::load(&f.root.join("modules")).unwrap();
    let i = f.create();
    i.prepare().unwrap();
    assert_eq!(i.launch_plan().unwrap().args[0], "G:\\bin\\Game.exe");
    assert!(i.root().join("stage/game/bin/Game.exe").is_file());
}

#[test]
fn lock_child() {
    if let Some(root) = std::env::var_os("PORTCELLAR_TEST_MANAGED_CHILD") {
        let i = ManagedInstallation::load(Path::new(&root)).unwrap();
        i.launch(Duration::from_secs(10)).unwrap();
    }
}

#[test]
fn another_process_cannot_mutate_an_active_or_interrupted_installation() {
    let f = Fixture::new("locking");
    write_script(&f.request.wine, "if [ \"$1\" = wineboot.exe ]; then mkdir -p \"$WINEPREFIX/drive_c\"; touch \"$WINEPREFIX/system.reg\"; exit 0; fi\nif [ \"$1\" = reg.exe ]; then exit 0; fi\ntouch \"$WINEPREFIX/entered\"\nsleep 1\n");
    let i = f.create();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "lock_child", "--nocapture"])
        .env("PORTCELLAR_TEST_MANAGED_CHILD", &i.root())
        .spawn()
        .unwrap();
    let started = std::time::Instant::now();
    while !i.root().join("prefix/entered").exists() {
        assert!(started.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(i.prepare().unwrap_err().to_string().contains("busy"));
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(i.prepare().unwrap_err().to_string().contains("interrupted"));
    std::thread::sleep(Duration::from_millis(1100));
    i.recover(Duration::from_secs(5)).unwrap();
    i.prepare().unwrap();
}
