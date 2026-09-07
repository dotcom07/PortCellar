use super::*;
use crate::*;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};

pub(crate) fn inspect_wine_steam_runtime_for(profile: &dyn GameProfile) -> WineSteamRuntime {
    let staged_game_root =
        crate::runtime::game_runtime_stage_for(profile).map(|stage| stage.game_root);
    inspect_wine_steam_runtime_for_root(profile, staged_game_root.as_deref())
}

pub(crate) fn inspect_wine_steam_runtime_for_root(
    profile: &dyn GameProfile,
    staged_game_root: Option<&Path>,
) -> WineSteamRuntime {
    let prefix = wine_prefix_for(profile);
    let wine = find_wine_engine_for(profile);
    let wine_version = wine
        .as_ref()
        .and_then(|path| command_stdout_path(path, &[OsString::from("--version")]));
    let steam_exe = find_windows_steam_exe(&prefix);
    let steam_dir = steam_exe
        .as_ref()
        .and_then(|path| path.parent())
        .map(Path::to_path_buf);
    let game_manifest = steam_dir
        .as_ref()
        .map(|dir| {
            dir.join("steamapps")
                .join(format!("appmanifest_{}.acf", profile.app_id()))
        })
        .filter(|path| path.exists());
    let game_install_dir = if profile.steam_integration() == SteamIntegration::None {
        staged_game_root.map(Path::to_path_buf).or_else(|| {
            profile
                .windows_install_path_hint()
                .and_then(|path| prefix_path_from_windows_path(&prefix, path))
        })
    } else {
        steam_dir
            .as_ref()
            .and_then(|dir| find_windows_game_install_dir(dir, profile, game_manifest.as_ref()))
            .or_else(|| {
                profile
                    .windows_install_path_hint()
                    .and_then(|path| prefix_path_from_windows_path(&prefix, path))
            })
    };
    let game_executable = game_install_dir
        .as_ref()
        .map(|dir| dir.join(profile.windows_exe()))
        .filter(|path| path.exists());
    let process_list =
        command_stdout("/bin/ps", &["-axo".into(), "pid=,command=".into()]).unwrap_or_default();
    let steam_process_running =
        process_command_matches_in_prefix(&process_list, &prefix, &["steam.exe"]);
    let steam_webhelper_running =
        process_command_matches_in_prefix(&process_list, &prefix, &["steamwebhelper.exe"]);
    let game_process_name = profile_process_exe_name(profile.windows_exe());
    let game_running =
        process_command_matches_in_prefix(&process_list, &prefix, &[game_process_name]);
    let active_session =
        steam_webhelper_session_state_from_processes_in_prefix(&process_list, &prefix);
    let running = wine_steam_running_status(
        steam_process_running,
        steam_webhelper_running,
        active_session,
    );
    let raw_login_status = steam_dir.as_ref().and_then(|dir| steam_login_status(dir));
    let raw_connection_status = steam_dir
        .as_ref()
        .and_then(|dir| steam_connection_status(dir));
    let cached_credentials = steam_dir.as_ref().and_then(|dir| steam_login_state(dir));
    let logged_in = steam_logged_in_status(
        active_session,
        cached_credentials,
        raw_login_status.as_deref(),
    );
    let login_status = raw_login_status.map(|status| last_status_when_stopped(status, running));
    let connection_status =
        raw_connection_status.map(|status| last_status_when_stopped(status, running));
    let mac_driver = wine_mac_driver_config(&prefix);
    let windows_version = wine_windows_version(&prefix);
    let dxmt_root = find_dxmt_root(&prefix);
    let cef_status = steam_dir
        .as_ref()
        .and_then(|dir| steam_cef_status(dir, &mac_driver, running));
    let mac_window_status = wine_mac_window_status(
        running.then(wine_mac_window_count).flatten(),
        cef_status.as_deref(),
    );
    let virtual_desktop = steam_virtual_desktop_setting();
    let mut readiness_issues = wine_steam_readiness_issues_for(
        wine.as_ref(),
        steam_exe.as_ref(),
        game_executable.as_ref(),
        cef_status.as_deref(),
        profile.name(),
        profile.steam_integration() != SteamIntegration::None,
    );
    if steam_webhelper_running && !steam_process_running {
        readiness_issues.push(
            "orphaned Steam WebHelper process tree detected; run `portcellar isaac steam-stop`"
                .to_string(),
        );
    }
    let launch_ready_after_login = readiness_issues.is_empty();

    WineSteamRuntime {
        wine,
        wine_version,
        windows_version,
        dxmt_root,
        prefix,
        steam_exe,
        game_manifest,
        game_install_dir,
        game_executable,
        running,
        game_running,
        launch_ready_after_login,
        readiness_issues,
        logged_in,
        active_session,
        cached_credentials,
        login_status,
        connection_status,
        cef_status,
        mac_window_status,
        virtual_desktop,
        mac_driver,
    }
}

pub(crate) fn wine_steam_running_status(
    steam_process_running: bool,
    steam_webhelper_running: bool,
    active_session: Option<bool>,
) -> bool {
    steam_process_running || (steam_webhelper_running && active_session == Some(true))
}

pub fn inspect_game_runtime(profile: &dyn GameProfile) -> WineSteamRuntime {
    inspect_wine_steam_runtime_for(profile)
}

#[cfg(test)]
pub(crate) fn wine_steam_readiness_issues(
    wine: Option<&PathBuf>,
    steam_exe: Option<&PathBuf>,
    isaac_executable: Option<&PathBuf>,
    cef_status: Option<&str>,
) -> Vec<String> {
    wine_steam_readiness_issues_for(wine, steam_exe, isaac_executable, cef_status, "Isaac", true)
}

pub(crate) fn wine_steam_readiness_issues_for(
    wine: Option<&PathBuf>,
    steam_exe: Option<&PathBuf>,
    game_executable: Option<&PathBuf>,
    cef_status: Option<&str>,
    game_name: &str,
    steam_required: bool,
) -> Vec<String> {
    let mut issues = Vec::new();
    if wine.is_none() {
        issues.push("Wine engine missing".to_string());
    }
    if steam_required && steam_exe.is_none() {
        issues.push("Windows Steam missing".to_string());
    }
    if game_executable.is_none() {
        issues.push(format!("Windows {game_name} executable missing"));
    }
    if let Some(
        status @ ("steamwebhelper fatal: process_metrics_win" | "steamwebhelper restart loop"),
    ) = cef_status
    {
        issues.push(format!("Steam CEF unhealthy: {status}"));
    }

    issues
}

fn prefix_path_from_windows_path(prefix: &Path, windows_path: &str) -> Option<PathBuf> {
    let trimmed = windows_path.trim().trim_matches('"');
    let drive = trimmed.as_bytes().first()?.to_ascii_lowercase();
    let tail = trimmed.get(3..)?;
    let mut path = match drive {
        b'c' => prefix.join("drive_c"),
        b'z' => PathBuf::from("/"),
        _ => return None,
    };
    for part in tail.split(['\\', '/']).filter(|part| !part.is_empty()) {
        if matches!(part, "." | "..") {
            return None;
        }
        path.push(part);
    }
    Some(path)
}

pub(crate) fn wine_prefix() -> PathBuf {
    if let Some(prefix) = env::var_os("PORTCELLAR_WINEPREFIX") {
        return PathBuf::from(prefix);
    }

    if let Some(prefix) = project_wine_prefix() {
        return prefix;
    }

    state_root().join("prefixes/isaac-steam")
}

pub(crate) fn wine_prefix_for(profile: &dyn GameProfile) -> PathBuf {
    if let Some(prefix) = env::var_os("PORTCELLAR_WINEPREFIX") {
        return PathBuf::from(prefix);
    }

    if let Some(path) = profile_bottle_path(profile, &state_root().join("prefixes")) {
        if path.exists() {
            return path;
        }
    }

    if let Some(home) = home_dir() {
        if let Some(path) = profile_bottle_path(
            profile,
            &home.join("Library/Application Support/PortCellar/prefixes"),
        ) {
            return path;
        }
    }

    wine_prefix()
}

pub(crate) fn profile_bottle_path(profile: &dyn GameProfile, prefixes: &Path) -> Option<PathBuf> {
    let name = profile.bottle_name()?;
    let mut components = Path::new(name).components();
    if !matches!(components.next(), Some(Component::Normal(_))) || components.next().is_some() {
        return None;
    }
    Some(prefixes.join(name))
}

pub(crate) fn profile_process_exe_name(executable: &str) -> &str {
    executable.rsplit(['\\', '/']).next().unwrap_or(executable)
}

pub(crate) fn project_wine_prefix() -> Option<PathBuf> {
    let prefixes = state_root().join("prefixes");
    for name in ["isaac-steam-cx-clean", "isaac-steam", "isaac"] {
        let prefix = prefixes.join(name);
        if wine_prefix_has_windows_steam(&prefix) {
            return Some(prefix);
        }
    }

    let mut candidates = fs::read_dir(prefixes)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| wine_prefix_has_windows_steam(path))
        .collect::<Vec<_>>();
    candidates.sort();
    candidates.into_iter().next()
}

pub(crate) fn find_wine_engine_for(profile: &dyn GameProfile) -> Option<PathBuf> {
    if let Some(path) = profile.wine_engine_path_hint() {
        return path.exists().then(|| path.to_path_buf());
    }

    if let Some(path) = env::var_os("PORTCELLAR_WINE").map(PathBuf::from) {
        if path.exists() {
            return Some(path);
        }
    }

    let mut candidates = wine_app_candidates();
    candidates.extend(
        crossover_candidates()
            .into_iter()
            .filter(|path| path.file_name().and_then(|name| name.to_str()) != Some("cxrun")),
    );
    candidates.extend(gptk_candidates());
    candidates.extend(
        [find_in_path("wine64"), find_in_path("wine")]
            .into_iter()
            .flatten(),
    );

    select_wine_engine_for(profile, candidates)
}

pub(crate) fn select_wine_engine_for(
    profile: &dyn GameProfile,
    candidates: impl IntoIterator<Item = PathBuf>,
) -> Option<PathBuf> {
    candidates.into_iter().find(|path| {
        if !path.exists() {
            return false;
        }
        let kind = crate::runtime::wine_engine_kind_for_path(path);
        crate::runtime::engine_backend_compatibility(kind, profile.graphics_backend()).status
            != CompatibilityStatus::Unsupported
    })
}

pub(crate) fn find_windows_steam_exe(prefix: &Path) -> Option<PathBuf> {
    if let Some(path) = env::var_os("PORTCELLAR_STEAM_EXE").map(PathBuf::from) {
        if path.exists() {
            return Some(path);
        }
    }

    windows_steam_exe_candidates(prefix)
        .into_iter()
        .find(|path| path.exists())
}

pub(crate) fn wine_prefix_has_windows_steam(prefix: &Path) -> bool {
    windows_steam_exe_candidates(prefix)
        .into_iter()
        .any(|path| path.exists())
}

pub(crate) fn windows_steam_exe_candidates(prefix: &Path) -> Vec<PathBuf> {
    [
        prefix.join("drive_c/Steam/steam.exe"),
        prefix.join("drive_c/Steam/Steam.exe"),
        prefix.join("drive_c/Program Files (x86)/Steam/steam.exe"),
        prefix.join("drive_c/Program Files (x86)/Steam/Steam.exe"),
        prefix.join("drive_c/Program Files/Steam/steam.exe"),
        prefix.join("drive_c/Program Files/Steam/Steam.exe"),
    ]
    .into()
}

pub(crate) fn find_dxmt_root(prefix: &Path) -> Option<PathBuf> {
    env::var_os("PORTCELLAR_DXMT_ROOT")
        .map(PathBuf::from)
        .into_iter()
        .chain(env::var_os("DXMT_ROOT").map(PathBuf::from))
        .chain(state_root_for_prefix(prefix).map(|root| root.join("dxmt")))
        .chain(home_dir().map(|home| home.join("DXMT")))
        .find(|path| is_dxmt_root(path))
}

pub(crate) fn state_root_for_prefix(prefix: &Path) -> Option<PathBuf> {
    let prefixes = prefix.parent()?;
    (prefixes.file_name()? == "prefixes").then(|| prefixes.parent().map(Path::to_path_buf))?
}

pub(crate) fn is_dxmt_root(path: &Path) -> bool {
    path.join("i386-windows").is_dir()
        && path.join("x86_64-windows").is_dir()
        && path.join("x86_64-unix").is_dir()
}

pub(crate) fn find_windows_game_install_dir(
    steam_dir: &Path,
    profile: &dyn GameProfile,
    manifest_path: Option<&PathBuf>,
) -> Option<PathBuf> {
    if let Some(manifest_path) = manifest_path {
        if let Ok(manifest) = fs::read_to_string(manifest_path) {
            let values = parse_key_values(&manifest);
            if let Some(install_dir_name) = values.get("installdir") {
                let install_dir = steam_dir.join("steamapps/common").join(install_dir_name);
                if install_dir.exists() {
                    return Some(install_dir);
                }
            }
        }
    }

    if profile.install_dir_hint().is_empty() {
        return None;
    }
    let install_dir = steam_dir
        .join("steamapps/common")
        .join(profile.install_dir_hint());
    install_dir.exists().then_some(install_dir)
}

pub(crate) fn wine_mac_driver_config(prefix: &Path) -> WineMacDriverConfig {
    WineMacDriverConfig {
        allow_immovable_windows: wine_user_reg_value(
            prefix,
            "Software\\\\Wine\\\\Mac Driver",
            "AllowImmovableWindows",
        ),
        retina_mode: wine_user_reg_value(prefix, "Software\\\\Wine\\\\Mac Driver", "RetinaMode"),
    }
}

pub(crate) fn wine_windows_version(prefix: &Path) -> Option<String> {
    wine_user_reg_value(prefix, "Software\\\\Wine", "Version").or_else(|| {
        let major = wine_system_reg_dword(
            prefix,
            "Software\\\\Microsoft\\\\Windows NT\\\\CurrentVersion",
            "CurrentMajorVersionNumber",
        )?;
        let minor = wine_system_reg_dword(
            prefix,
            "Software\\\\Microsoft\\\\Windows NT\\\\CurrentVersion",
            "CurrentMinorVersionNumber",
        )
        .unwrap_or(0);

        Some(match (major, minor) {
            (10, 0) => "win10".to_string(),
            (6, 3) => "win81".to_string(),
            (6, 2) => "win8".to_string(),
            (6, 1) => "win7".to_string(),
            (6, 0) => "vista".to_string(),
            (5, 1) => "winxp".to_string(),
            _ => format!("{major}.{minor}"),
        })
    })
}

pub(crate) fn wine_mac_window_count() -> Option<u32> {
    let script = r#"import CoreGraphics
import Foundation

let windows = (CGWindowListCopyWindowInfo([.optionAll], kCGNullWindowID) as NSArray? as? [[String: Any]]) ?? []
var count = 0
for window in windows {
    let owner = (window[kCGWindowOwnerName as String] as? String ?? "").lowercased()
    let onscreen = window[kCGWindowIsOnscreen as String] as? Bool ?? false
    let alpha = window[kCGWindowAlpha as String] as? Double ?? 0
    guard owner.contains("wine"), onscreen, alpha > 0 else {
        continue
    }
    count += 1
}
print(count)
"#;
    command_stdout_stdin("/usr/bin/swift", &[OsString::from("-")], script)?
        .trim()
        .parse::<u32>()
        .ok()
}

pub(crate) fn wine_mac_window_status(
    window_count: Option<u32>,
    cef_status: Option<&str>,
) -> Option<String> {
    match window_count {
        Some(0)
            if cef_status
                .map(|status| status.starts_with("login-window-ready"))
                .unwrap_or(false) =>
        {
            Some("cef-ready-no-macos-window".to_string())
        }
        Some(0) => Some("no-macos-window".to_string()),
        Some(_) => Some("mac-window-visible".to_string()),
        None => None,
    }
}

pub(crate) fn wine_user_reg_value(prefix: &Path, section: &str, name: &str) -> Option<String> {
    let text = fs::read_to_string(prefix.join("user.reg")).ok()?;
    wine_reg_quoted_value(&text, section, name)
}

pub(crate) fn wine_system_reg_dword(prefix: &Path, section: &str, name: &str) -> Option<u32> {
    let text = fs::read_to_string(prefix.join("system.reg")).ok()?;
    wine_reg_dword_value(&text, section, name)
}

pub(crate) fn wine_reg_quoted_value(text: &str, section: &str, name: &str) -> Option<String> {
    let section_header = format!("[{section}]");
    let mut in_section = false;

    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_section = line == section_header
                || line
                    .strip_prefix(&section_header)
                    .map(|rest| rest.starts_with(' '))
                    .unwrap_or(false);
            continue;
        }

        if in_section {
            let tokens = quoted_tokens(line);
            if tokens.len() >= 2 && tokens[0] == name {
                return Some(tokens[1].clone());
            }
        }
    }

    None
}

pub(crate) fn wine_reg_dword_value(text: &str, section: &str, name: &str) -> Option<u32> {
    let section_header = format!("[{section}]");
    let mut in_section = false;
    let prefix = format!("\"{name}\"=dword:");

    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_section = line == section_header
                || line
                    .strip_prefix(&section_header)
                    .map(|rest| rest.starts_with(' '))
                    .unwrap_or(false);
            continue;
        }

        if in_section && line.starts_with(&prefix) {
            let value = line.strip_prefix(&prefix)?;
            return u32::from_str_radix(value, 16).ok();
        }
    }

    None
}
