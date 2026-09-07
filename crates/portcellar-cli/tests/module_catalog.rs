use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn snapshot(root: &Path, path: &Path, entries: &mut BTreeMap<PathBuf, Vec<u8>>) {
    for entry in fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        entries.insert(
            path.strip_prefix(root).unwrap().into(),
            if path.is_dir() {
                Vec::new()
            } else {
                fs::read(&path).unwrap()
            },
        );
        if path.is_dir() {
            snapshot(root, &path, entries);
        }
    }
}

#[test]
fn module_commands_are_read_only_and_reject_unknown_selections() {
    let root = std::env::temp_dir().join(format!("portcellar-module-cli-{}", std::process::id()));
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../modules/simcity-4");
    let module = root.join("modules/simcity-4");
    fs::create_dir_all(module.join("profiles")).unwrap();
    fs::copy(source.join("module.toml"), module.join("module.toml")).unwrap();
    fs::copy(
        source.join("profiles/windows-1.1.610.toml"),
        module.join("profiles/windows-1.1.610.toml"),
    )
    .unwrap();
    let mut before = BTreeMap::new();
    snapshot(&root, &root, &mut before);
    for (args, success) in [
        (vec!["game", "modules", "--root", "modules"], true),
        (
            vec!["game", "module", "--root", "modules", "--id", "simcity-4"],
            true,
        ),
        (vec!["game", "module", "--id", "unknown"], false),
        (vec!["game", "module"], false),
        (vec!["game", "modules", "--execute"], false),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_portcellar"))
            .args(&args)
            .current_dir(&root)
            .env("PORTCELLAR_STATE_ROOT", root.join("private-state"))
            .output()
            .unwrap();
        assert_eq!(
            output.status.success(),
            success,
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        if success {
            assert!(
                String::from_utf8_lossy(&output.stdout).contains("simcity-4/windows-1-1-610/scgl")
            );
        }
    }
    let mut after = BTreeMap::new();
    snapshot(&root, &root, &mut after);
    assert_eq!(before, after);
    fs::remove_dir_all(root).unwrap();
}
