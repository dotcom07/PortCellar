use super::*;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

pub fn kill_wine_steam_processes() -> Result<Vec<u32>> {
    let prefix = doctor_report().wine_steam.prefix;
    kill_wine_steam_processes_for_prefix(&prefix)
}

pub(crate) fn kill_wine_steam_processes_for_prefix(prefix: &Path) -> Result<Vec<u32>> {
    let ps_args = ["-axo".into(), "pid=,pgid=,command=".into()];
    let Some(output) = command_stdout("/bin/ps", &ps_args) else {
        return Ok(Vec::new());
    };
    let mut candidates = wine_steam_cleanup_process_groups_from_ps(&output)
        .into_iter()
        .filter(|(pid, _)| process_uses_prefix(*pid, prefix))
        .collect::<Vec<_>>();
    let mut pids = candidates.iter().map(|(pid, _)| *pid).collect::<Vec<_>>();

    signal_process_groups(&candidates, "-TERM");
    signal_processes(&pids, "-TERM");

    if !pids.is_empty() {
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            std::thread::sleep(Duration::from_millis(100));
            let Some(output) = command_stdout("/bin/ps", &ps_args) else {
                break;
            };
            candidates = wine_steam_cleanup_process_groups_from_ps(&output)
                .into_iter()
                .filter(|(pid, _)| process_uses_prefix(*pid, prefix))
                .collect();
            for (pid, _) in &candidates {
                if !pids.contains(pid) {
                    pids.push(*pid);
                }
            }
            if candidates.is_empty() || Instant::now() >= deadline {
                break;
            }
            signal_process_groups(&candidates, "-TERM");
            signal_processes(&pids, "-TERM");
        }

        if !candidates.is_empty() {
            signal_process_groups(&candidates, "-KILL");
            signal_processes(&pids, "-KILL");
        }
    }

    Ok(pids)
}

fn signal_processes(pids: &[u32], signal: &str) {
    for pid in pids {
        let _ = Command::new("/bin/kill")
            .arg(signal)
            .arg(pid.to_string())
            .status();
    }
}

fn signal_process_groups(candidates: &[(u32, u32)], signal: &str) {
    let current_pgid = current_process_group_id();
    let groups = candidates
        .iter()
        .map(|(_, pgid)| *pgid)
        .filter(|pgid| *pgid > 1 && *pgid != std::process::id())
        .filter(|pgid| Some(*pgid) != current_pgid)
        .collect::<BTreeSet<_>>();
    for pgid in groups {
        let _ = Command::new("/bin/kill")
            .arg(signal)
            .arg(format!("-{pgid}"))
            .status();
    }
}

fn current_process_group_id() -> Option<u32> {
    let args = [
        "-p".into(),
        std::process::id().to_string().into(),
        "-o".into(),
        "pgid=".into(),
    ];
    command_stdout("/bin/ps", &args)?.trim().parse().ok()
}

pub fn wine_steam_session_reset_plan() -> Result<SteamSessionResetPlan> {
    let report = doctor_report();
    let runtime = &report.wine_steam;
    let steam_exe = runtime.steam_exe.as_ref().ok_or_else(|| {
        PortCellarError::Message(
            "Windows Steam was not found in the configured Wine prefix".to_string(),
        )
    })?;
    let steam_dir = steam_exe.parent().ok_or_else(|| {
        PortCellarError::Message("Windows Steam executable has no parent".to_string())
    })?;
    let backup_dir = steam_dir.join("portcellar-backups").join(format!(
        "steam-session-{}-{}",
        unix_timestamp(),
        std::process::id()
    ));
    let mut targets = Vec::new();

    for target in steam_session_reset_candidates(&runtime.prefix, steam_dir) {
        if target.exists() {
            targets.push(SteamSessionResetTarget {
                backup: steam_session_backup_path(&backup_dir, &runtime.prefix, steam_dir, &target),
                source: target,
            });
        }
    }

    Ok(SteamSessionResetPlan {
        backup_dir,
        targets,
    })
}

pub fn apply_steam_session_reset(
    plan: &SteamSessionResetPlan,
) -> Result<Vec<SteamSessionResetTarget>> {
    let mut moved = Vec::new();

    for target in &plan.targets {
        if !target.source.exists() {
            continue;
        }
        if let Some(parent) = target.backup.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(&target.source, &target.backup)?;
        moved.push(target.clone());
    }

    Ok(moved)
}

pub(crate) fn steam_session_reset_candidates(prefix: &Path, steam_dir: &Path) -> Vec<PathBuf> {
    let mut targets = vec![
        steam_dir.join("config/loginusers.vdf"),
        steam_dir.join("appcache"),
        steam_dir.join("logs"),
        steam_dir.join("dumps"),
    ];

    let users_dir = prefix.join("drive_c/users");
    if let Ok(entries) = fs::read_dir(users_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            targets.push(path.join("AppData/Local/Steam/htmlcache"));
            targets.push(path.join("AppData/Local/CEF/User Data"));
        }
    }

    targets
}

pub(crate) fn steam_session_backup_path(
    backup_dir: &Path,
    prefix: &Path,
    steam_dir: &Path,
    source: &Path,
) -> PathBuf {
    if let Ok(relative) = source.strip_prefix(steam_dir) {
        return backup_dir.join("Steam").join(relative);
    }

    let drive_c = prefix.join("drive_c");
    if let Ok(relative) = source.strip_prefix(drive_c) {
        return backup_dir.join("drive_c").join(relative);
    }

    backup_dir.join(
        source
            .file_name()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("unknown")),
    )
}

pub(crate) fn unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}
