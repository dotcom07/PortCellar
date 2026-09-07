use portcellar_core::{load_game_compatibility_evidence, state_root, Result};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn evidence(args: &[String]) -> Result<()> {
    let (root, app_id) = parse_request(args)?;
    if let Some(app_id) = app_id {
        let path = root.join(&app_id).join("latest.toml");
        let evidence = load_game_compatibility_evidence(&path).map_err(|error| {
            portcellar_core::PortCellarError::Message(format!(
                "could not load compatibility evidence for {app_id} at {}: {error}",
                path.display()
            ))
        })?;
        print_detail(&path, &evidence);
        return Ok(());
    }

    let mut entries = Vec::new();
    if root.is_dir() {
        for entry in fs::read_dir(&root)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let path = entry.path().join("latest.toml");
            if path.is_file() {
                entries.push((path.clone(), load_game_compatibility_evidence(&path)?));
            }
        }
    }
    entries.sort_by(|left, right| left.1.app_id.cmp(&right.1.app_id));

    println!("compatibility evidence: {}", root.display());
    if entries.is_empty() {
        println!("no evidence found");
        return Ok(());
    }
    println!(
        "app_id\tprofile\tprofile_version\tengine\tbottle\tlauncher\tbackend\tstatus\tresult\tobserved"
    );
    for (_, evidence) in entries {
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            evidence.app_id,
            evidence.profile_name,
            evidence.profile_version,
            evidence.engine.as_deref().unwrap_or("unknown"),
            evidence.execution_fingerprint.bottle_layout,
            evidence.execution_fingerprint.launcher_model,
            evidence.graphics_backend,
            evidence
                .compatibility_status
                .as_deref()
                .unwrap_or("unknown"),
            if evidence.passed { "passed" } else { "failed" },
            evidence.observed_at_unix_seconds,
        );
    }
    Ok(())
}

fn parse_request(args: &[String]) -> Result<(PathBuf, Option<String>)> {
    let mut root = env::var_os("PORTCELLAR_COMPATIBILITY_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| state_root().join("compatibility"));
    let mut app_id = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--root" => {
                index += 1;
                root = PathBuf::from(args.get(index).ok_or_else(|| {
                    portcellar_core::PortCellarError::Message("--root requires a path".to_string())
                })?);
            }
            value if value.starts_with("--root=") => {
                root = PathBuf::from(value.trim_start_matches("--root="));
            }
            "--app-id" => {
                index += 1;
                app_id = Some(safe_component(args.get(index).ok_or_else(|| {
                    portcellar_core::PortCellarError::Message("--app-id requires an id".to_string())
                })?)?);
            }
            value if value.starts_with("--app-id=") => {
                app_id = Some(safe_component(value.trim_start_matches("--app-id="))?);
            }
            value => {
                return Err(portcellar_core::PortCellarError::Message(format!(
                    "unknown game evidence option: {value}"
                )));
            }
        }
        index += 1;
    }

    Ok((root, app_id))
}

fn safe_component(value: &str) -> Result<String> {
    if value.is_empty()
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
    {
        return Err(portcellar_core::PortCellarError::Message(
            "evidence app id must be a safe path component".to_string(),
        ));
    }
    Ok(value.to_string())
}

fn print_detail(path: &Path, evidence: &portcellar_core::GameCompatibilityEvidence) {
    println!("evidence: {}", path.display());
    println!("app_id: {}", evidence.app_id);
    println!("profile: {}", evidence.profile_name);
    println!("profile_version: {}", evidence.profile_version);
    println!("launch_mode: {}", evidence.launch_mode);
    println!(
        "engine: {}",
        evidence.engine.as_deref().unwrap_or("unknown")
    );
    println!(
        "engine_version: {}",
        evidence.engine_version.as_deref().unwrap_or("unknown")
    );
    println!(
        "windows_version: {}",
        evidence
            .execution_fingerprint
            .windows_version
            .as_deref()
            .unwrap_or("unknown")
    );
    println!(
        "bottle_layout: {}",
        evidence.execution_fingerprint.bottle_layout
    );
    println!(
        "launcher_model: {}",
        evidence.execution_fingerprint.launcher_model
    );
    println!("graphics_backend: {}", evidence.graphics_backend);
    println!(
        "compatibility_status: {}",
        evidence
            .compatibility_status
            .as_deref()
            .unwrap_or("unknown")
    );
    println!(
        "result: {}",
        if evidence.passed { "passed" } else { "failed" }
    );
    println!("stable_window_seconds: {}", evidence.stable_window_seconds);
    println!("steam_session_ready: {}", evidence.steam_session_ready);
    println!("game_running: {}", evidence.game_running);
    println!(
        "failure_reason: {}",
        evidence.failure_reason.as_deref().unwrap_or("none")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_app_id_cannot_escape_catalog_root() {
        assert!(safe_component("../secret").is_err());
        assert_eq!(safe_component("250900").unwrap(), "250900");
    }
}
