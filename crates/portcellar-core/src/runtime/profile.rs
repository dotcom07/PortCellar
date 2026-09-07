use super::*;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub fn isaac_runtime_profile_plan() -> Result<IsaacRuntimeProfilePlan> {
    game_runtime_profile_plan(&isaac_profile())
}

pub fn game_runtime_profile_plan(profile: &dyn GameProfile) -> Result<GameRuntimeProfilePlan> {
    let runtime = inspect_wine_steam_runtime_for(profile);
    game_runtime_profile_plan_for(profile, &runtime)
}

pub(crate) fn game_runtime_profile_plan_for(
    profile: &dyn GameProfile,
    runtime: &WineSteamRuntime,
) -> Result<GameRuntimeProfilePlan> {
    let options_path = if let Some(path) = profile.runtime_options_path_hint() {
        prefix_path_from_windows_path(&runtime.prefix, path).ok_or_else(|| {
            PortCellarError::Message(format!(
                "{} has an invalid runtime options path: {path}",
                profile.name()
            ))
        })?
    } else {
        let install_dir = runtime.game_install_dir.as_ref().ok_or_else(|| {
            PortCellarError::Message(format!(
                "Windows {} files were not found in the Wine Steam library",
                profile.name()
            ))
        })?;
        game_save_dir(runtime, install_dir, profile)?.join("options.ini")
    };

    Ok(GameRuntimeProfilePlan {
        options_path,
        values: profile.runtime_policy().runtime_options,
    })
}

pub fn apply_isaac_runtime_profile() -> Result<IsaacRuntimeProfilePlan> {
    let plan = isaac_runtime_profile_plan()?;
    apply_runtime_profile_plan(&plan)?;
    Ok(plan)
}

pub fn apply_game_runtime_profile(profile: &dyn GameProfile) -> Result<GameRuntimeProfilePlan> {
    let plan = game_runtime_profile_plan(profile)?;
    apply_runtime_profile_plan(&plan)?;
    Ok(plan)
}

fn apply_runtime_profile_plan(plan: &GameRuntimeProfilePlan) -> Result<()> {
    if let Some(parent) = plan.options_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let existing = fs::read_to_string(&plan.options_path).unwrap_or_default();
    let updated = upsert_runtime_options(&existing, &plan.values);
    fs::write(&plan.options_path, updated)?;
    Ok(())
}

pub(crate) fn prefix_path_from_windows_path(prefix: &Path, windows_path: &str) -> Option<PathBuf> {
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

pub(crate) fn game_save_dir(
    runtime: &WineSteamRuntime,
    install_dir: &Path,
    profile: &dyn GameProfile,
) -> Result<PathBuf> {
    let Some(default_subdir) = profile.default_documents_subdir() else {
        return Err(PortCellarError::Message(format!(
            "{} has no save directory contract",
            profile.name()
        )));
    };

    let savedatapath = install_dir.join("savedatapath.txt");
    if let Ok(text) = fs::read_to_string(savedatapath) {
        if let Some(marker) = profile.savedata_path_marker() {
            for line in text.lines() {
                if let Some(path) = line.trim().strip_prefix(marker) {
                    if let Some(path) = prefix_path_from_windows_path(&runtime.prefix, path) {
                        return Ok(path);
                    }
                }
            }
        }
    }

    Ok(wine_user_dir(&runtime.prefix)
        .join("Documents/My Games")
        .join(default_subdir))
}

fn wine_user_dir(prefix: &Path) -> PathBuf {
    let users_dir = prefix.join("drive_c/users");
    let preferred = env::var("USER").ok();
    if let Some(preferred) = preferred {
        let path = users_dir.join(preferred);
        if path.is_dir() {
            return path;
        }
    }
    if let Ok(entries) = fs::read_dir(&users_dir) {
        if let Some(path) = entries
            .flatten()
            .map(|entry| entry.path())
            .find(|path| path.is_dir())
        {
            return path;
        }
    }
    users_dir.join("user")
}

pub(crate) fn upsert_runtime_options(existing: &str, values: &BTreeMap<String, String>) -> String {
    let mut lines = existing
        .lines()
        .map(str::to_string)
        .collect::<Vec<String>>();
    let options_start = lines.iter().position(|line| line.trim() == "[Options]");
    let options_start = match options_start {
        Some(index) => index,
        None => {
            if !lines.is_empty() && !lines.last().map(|line| line.is_empty()).unwrap_or(false) {
                lines.push(String::new());
            }
            lines.push("[Options]".to_string());
            lines.len() - 1
        }
    };
    let options_end = lines
        .iter()
        .enumerate()
        .skip(options_start + 1)
        .find(|(_, line)| {
            let trimmed = line.trim();
            trimmed.starts_with('[') && trimmed.ends_with(']')
        })
        .map(|(index, _)| index)
        .unwrap_or(lines.len());

    let mut existing_keys = BTreeSet::new();
    for line in lines.iter_mut().take(options_end).skip(options_start + 1) {
        let Some((key, _)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if let Some(value) = values.get(key) {
            let key = key.to_string();
            *line = format!("{key}={value}");
            existing_keys.insert(key);
        }
    }

    let mut insert_at = options_end;
    for (key, value) in values {
        if existing_keys.contains(key) {
            continue;
        }
        lines.insert(insert_at, format!("{key}={value}"));
        insert_at += 1;
    }

    let mut output = lines.join("\n");
    output.push('\n');
    output
}
