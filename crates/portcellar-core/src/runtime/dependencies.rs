use super::*;
use std::collections::BTreeMap;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

pub fn wine_bottle_mutation_plan(
    profile: &dyn GameProfile,
    runtime: &WineSteamRuntime,
    snapshot_id: &str,
) -> Result<BottleMutationPlan> {
    if runtime.running || runtime.game_running {
        return Err(PortCellarError::Message(
            "stop Wine Steam and the game before creating a bottle mutation plan".to_string(),
        ));
    }

    Ok(BottleMutationPlan {
        prefix: runtime.prefix.clone(),
        snapshot: wine_bottle_snapshot_plan(runtime, snapshot_id)?,
        dependencies: wine_dependency_plans_for(profile, runtime),
    })
}

pub fn wine_bottle_snapshot_plan(
    runtime: &WineSteamRuntime,
    snapshot_id: &str,
) -> Result<BottleSnapshotPlan> {
    if !runtime.prefix.is_dir() {
        return Err(PortCellarError::Message(format!(
            "Wine prefix does not exist: {}",
            runtime.prefix.display()
        )));
    }
    let snapshot_id = safe_snapshot_component(snapshot_id)?;
    let parent = runtime.prefix.parent().ok_or_else(|| {
        PortCellarError::Message(format!(
            "Wine prefix has no snapshot parent: {}",
            runtime.prefix.display()
        ))
    })?;
    let destination = parent.join("snapshots").join(snapshot_id);
    let snapshot_root = parent.join("snapshots");
    match fs::symlink_metadata(&destination) {
        Ok(_) => {
            return Err(PortCellarError::Message(format!(
                "snapshot destination already exists: {}",
                destination.display()
            )));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }

    Ok(BottleSnapshotPlan {
        source: runtime.prefix.clone(),
        destination: destination.clone(),
        prepare_command: CommandPlan {
            program: PathBuf::from("/bin/mkdir"),
            args: vec![
                OsString::from("-p"),
                snapshot_root.as_os_str().to_os_string(),
            ],
            envs: BTreeMap::new(),
            current_dir: None,
        },
        command: CommandPlan {
            program: PathBuf::from("/bin/cp"),
            args: vec![
                OsString::from("-a"),
                OsString::from("--"),
                runtime.prefix.as_os_str().to_os_string(),
                destination.as_os_str().to_os_string(),
            ],
            envs: BTreeMap::new(),
            current_dir: None,
        },
    })
}

pub fn wine_bottle_rollback_plan(
    runtime: &WineSteamRuntime,
    snapshot_id: &str,
) -> Result<BottleRollbackPlan> {
    if runtime.running || runtime.game_running {
        return Err(PortCellarError::Message(
            "stop Wine Steam and the game before creating a bottle rollback plan".to_string(),
        ));
    }
    if !runtime.prefix.is_dir() {
        return Err(PortCellarError::Message(format!(
            "Wine prefix does not exist: {}",
            runtime.prefix.display()
        )));
    }

    let snapshot_id = safe_snapshot_component(snapshot_id)?;
    let parent = runtime.prefix.parent().ok_or_else(|| {
        PortCellarError::Message(format!(
            "Wine prefix has no rollback parent: {}",
            runtime.prefix.display()
        ))
    })?;
    let snapshot = parent.join("snapshots").join(&snapshot_id);
    let rollback_root = parent.join("rollback-backups");
    let backup = rollback_root.join(format!("{snapshot_id}-current"));
    let staging = rollback_root.join(format!("{snapshot_id}-restore"));

    ensure_real_directory(&snapshot, "rollback snapshot")?;
    ensure_absent(&backup, "rollback backup")?;
    ensure_absent(&staging, "rollback staging")?;

    Ok(BottleRollbackPlan {
        prefix: runtime.prefix.clone(),
        snapshot: snapshot.clone(),
        backup: backup.clone(),
        staging: staging.clone(),
        prepare_command: CommandPlan {
            program: PathBuf::from("/bin/mkdir"),
            args: vec![
                OsString::from("-p"),
                rollback_root.as_os_str().to_os_string(),
            ],
            envs: BTreeMap::new(),
            current_dir: None,
        },
        stage_command: filesystem_command("/bin/cp", ["-a", "--"], [&snapshot, &staging]),
        backup_command: filesystem_command("/bin/mv", ["--"], [&runtime.prefix, &backup]),
        activate_command: filesystem_command("/bin/mv", ["--"], [&staging, &runtime.prefix]),
    })
}

pub fn wine_dependency_plans_for(
    profile: &dyn GameProfile,
    runtime: &WineSteamRuntime,
) -> Vec<RuntimeDependencyPlan> {
    let dependencies = profile.runtime_policy().dependencies;
    let winetricks = find_winetricks(runtime);

    dependencies
        .into_iter()
        .map(|dependency| {
            let Some(verb) = dependency.winetricks_verb() else {
                return RuntimeDependencyPlan {
                    dependency,
                    status: RuntimeDependencyStatus::Manual,
                    command: None,
                    note: "no safe generic verb mapping; use the game's installer or a verified bottle recipe".to_string(),
                };
            };

            let Some(program) = winetricks.clone() else {
                return RuntimeDependencyPlan {
                    dependency,
                    status: RuntimeDependencyStatus::ToolMissing,
                    command: None,
                    note: format!("winetricks verb '{verb}' is known, but winetricks was not found"),
                };
            };

            RuntimeDependencyPlan {
                dependency,
                status: RuntimeDependencyStatus::Planned,
                command: Some(CommandPlan {
                    program,
                    args: vec![OsString::from(verb)],
                    envs: wine_env(runtime),
                    current_dir: None,
                }),
                note: format!(
                    "dry-run plan only; not applied automatically; execute '{verb}' explicitly after validating the bottle"
                ),
            }
        })
        .collect()
}

fn find_winetricks(runtime: &WineSteamRuntime) -> Option<PathBuf> {
    if let Some(path) = env::var_os("PORTCELLAR_WINETRICKS").map(PathBuf::from) {
        if path.exists() {
            return Some(path);
        }
    }

    if let Some(wine) = runtime.wine.as_ref() {
        if let Some(parent) = wine.parent() {
            let sibling = parent.join("winetricks");
            if sibling.exists() {
                return Some(sibling);
            }
        }
    }

    find_in_path("winetricks")
}

fn safe_snapshot_component(value: &str) -> Result<String> {
    if value.is_empty()
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
    {
        return Err(PortCellarError::Message(
            "snapshot id must be a non-empty safe path component".to_string(),
        ));
    }
    Ok(value.to_string())
}

fn ensure_real_directory(path: &PathBuf, label: &str) -> Result<()> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            PortCellarError::Message(format!("{label} does not exist: {}", path.display()))
        } else {
            PortCellarError::Io(error)
        }
    })?;
    if !metadata.file_type().is_dir() {
        return Err(PortCellarError::Message(format!(
            "{label} must be a real directory: {}",
            path.display()
        )));
    }
    Ok(())
}

fn ensure_absent(path: &PathBuf, label: &str) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(_) => Err(PortCellarError::Message(format!(
            "{label} already exists: {}",
            path.display()
        ))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn filesystem_command<const N: usize, const M: usize>(
    program: &str,
    flags: [&str; N],
    paths: [&PathBuf; M],
) -> CommandPlan {
    let mut args = flags.into_iter().map(OsString::from).collect::<Vec<_>>();
    args.extend(
        paths
            .into_iter()
            .map(|path| path.as_os_str().to_os_string()),
    );
    CommandPlan {
        program: PathBuf::from(program),
        args,
        envs: BTreeMap::new(),
        current_dir: None,
    }
}
