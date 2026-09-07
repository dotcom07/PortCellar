use super::*;
use crate::*;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn inspect_steam() -> SteamReport {
    let root = steam_root();
    let libraries = root
        .as_ref()
        .map(|root| steam_libraries(root))
        .unwrap_or_default();

    SteamReport {
        root,
        running: process_is_running("steam_osx"),
        libraries,
    }
}

pub(crate) fn find_game_install(
    steam: &SteamReport,
    profile: &dyn GameProfile,
) -> Option<GameInstall> {
    for library in &steam.libraries {
        let steamapps = library.join("steamapps");
        let manifest_path = steamapps.join(format!("appmanifest_{}.acf", profile.app_id()));
        if !manifest_path.exists() {
            continue;
        }

        let manifest = fs::read_to_string(&manifest_path).ok()?;
        let values = parse_key_values(&manifest);
        let install_dir_name = values.get("installdir")?;
        let install_dir = steamapps.join("common").join(install_dir_name);
        let app_bundle = find_app_bundle(&install_dir);
        let executable = app_bundle
            .as_ref()
            .and_then(|bundle| app_executable(bundle));
        let executable_kind = executable.as_ref().and_then(|path| file_kind(path));
        let steam_appid_txt = executable
            .as_ref()
            .and_then(|path| path.parent())
            .map(|dir| dir.join("steam_appid.txt"))
            .and_then(|path| fs::read_to_string(path).ok())
            .map(|text| text.trim().to_string());

        return Some(GameInstall {
            app_id: values
                .get("appid")
                .cloned()
                .unwrap_or_else(|| profile.app_id().to_string()),
            name: values
                .get("name")
                .cloned()
                .unwrap_or_else(|| profile.name().to_string()),
            build_id: values.get("buildid").cloned(),
            manifest_path,
            install_dir,
            app_bundle,
            executable,
            executable_kind,
            steam_appid_txt,
        });
    }

    None
}

pub(crate) fn steam_root() -> Option<PathBuf> {
    let home = home_dir()?;
    let root = home.join("Library/Application Support/Steam");
    root.exists().then_some(root)
}

pub(crate) fn steam_libraries(root: &Path) -> Vec<PathBuf> {
    let mut libraries = BTreeSet::new();
    libraries.insert(root.to_path_buf());

    let libraryfolders = root.join("steamapps/libraryfolders.vdf");
    if let Ok(text) = fs::read_to_string(libraryfolders) {
        for path in values_for_key(&text, "path") {
            libraries.insert(PathBuf::from(path));
        }
    }

    libraries.into_iter().filter(|path| path.exists()).collect()
}

pub(crate) fn find_app_bundle(install_dir: &Path) -> Option<PathBuf> {
    let expected = install_dir.join("The Binding of Isaac Rebirth.app");
    if expected.exists() {
        return Some(expected);
    }

    fs::read_dir(install_dir).ok()?.flatten().find_map(|entry| {
        let path = entry.path();
        let is_app = path.extension().map(|ext| ext == "app").unwrap_or(false);
        is_app.then_some(path)
    })
}

pub(crate) fn app_executable(app_bundle: &Path) -> Option<PathBuf> {
    let info_plist = app_bundle.join("Contents/Info.plist");
    let executable = command_stdout(
        "/usr/libexec/PlistBuddy",
        &[
            "-c".into(),
            "Print:CFBundleExecutable".into(),
            info_plist.as_os_str().to_os_string(),
        ],
    )?;
    Some(app_bundle.join("Contents/MacOS").join(executable.trim()))
}

pub(crate) fn file_kind(path: &Path) -> Option<String> {
    command_stdout("/usr/bin/file", &[path.as_os_str().to_os_string()])
}
