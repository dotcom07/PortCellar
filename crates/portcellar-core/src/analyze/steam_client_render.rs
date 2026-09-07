use super::*;
use crate::*;
use std::fs;
use std::path::PathBuf;

pub fn write_steam_client_analysis_artifacts(
    analysis: &SteamClientAnalysis,
) -> Result<SteamClientAnalysisArtifacts> {
    let root = steam_client_output_root();
    let raw_dir = root.join("raw");
    let experiments_dir = root.join("experiments");
    fs::create_dir_all(&raw_dir)?;
    fs::create_dir_all(&experiments_dir)?;

    let report_path = root.join("report.md");
    let raw_process_before_path = raw_dir.join("process-before.txt");
    let raw_process_after_path = raw_dir.join("process-after.txt");
    let raw_probe_log_path = raw_dir.join("steam-client-probe.log");
    let experiment_path = experiments_dir.join(if analysis.options.probe {
        "001-steam-client-probe.yml"
    } else {
        "001-steam-client-collect.yml"
    });

    fs::write(&report_path, render_steam_client_report(analysis))?;
    if analysis.probe.is_none() || !raw_process_before_path.exists() {
        fs::write(&raw_process_before_path, process_snapshot())?;
    }
    fs::write(&raw_process_after_path, process_snapshot())?;
    fs::write(&experiment_path, render_steam_client_experiment(analysis))?;

    Ok(SteamClientAnalysisArtifacts {
        root,
        report_path,
        raw_process_before_path,
        raw_process_after_path,
        raw_probe_log_path,
        experiment_path,
    })
}

pub(super) fn process_snapshot() -> String {
    command_stdout(
        "/bin/ps",
        &["-axo".into(), "pid,ppid,stat,etime,command".into()],
    )
    .unwrap_or_default()
    .lines()
    .filter(|line| {
        let lower = line.to_ascii_lowercase();
        lower.contains("steam")
            || lower.contains("wine")
            || lower.contains("cef")
            || lower.contains("isaac")
    })
    .collect::<Vec<_>>()
    .join("\n")
}

pub(super) fn steam_client_output_root() -> PathBuf {
    state_root().join("analysis/steam-client")
}

fn render_steam_client_report(analysis: &SteamClientAnalysis) -> String {
    let mut text = String::new();
    text.push_str("# Steam Client Analysis Report\n\n");
    text.push_str("This private report may contain local paths and Steam runtime state. Keep it under `.portcellar/`.\n\n");
    text.push_str("## Summary\n\n");
    text.push_str(&format!("- analysis version: `{}`\n", analysis.version));
    text.push_str(&format!("- probe enabled: `{}`\n", analysis.options.probe));
    text.push_str(&format!(
        "- probe seconds: `{}`\n",
        analysis.options.probe_seconds
    ));
    text.push_str(&format!(
        "- stop after: `{}`\n",
        analysis.options.stop_after
    ));
    text.push_str(&format!(
        "- Steam dir: `{}`\n",
        path_or_missing(analysis.steam_dir.as_ref())
    ));
    text.push_str(&format!(
        "- wine: `{}`\n",
        path_or_missing(analysis.runtime.wine.as_ref())
    ));
    text.push_str(&format!(
        "- wine version: `{}`\n",
        analysis
            .runtime
            .wine_version
            .as_deref()
            .unwrap_or("unknown")
    ));
    text.push_str(&format!(
        "- prefix: `{}`\n",
        analysis.runtime.prefix.display()
    ));
    text.push_str(&format!(
        "- running after analysis: `{}`\n",
        analysis.runtime.steam_running
    ));
    text.push_str(&format!(
        "- logged in after analysis: `{}`\n",
        optional_bool(analysis.runtime.logged_in)
    ));
    text.push_str(&format!(
        "- connection after analysis: `{}`\n",
        analysis
            .runtime
            .connection_status
            .as_deref()
            .unwrap_or("unknown")
    ));
    text.push_str(&format!(
        "- CEF after analysis: `{}`\n",
        analysis.runtime.cef_status.as_deref().unwrap_or("unknown")
    ));

    text.push_str("\n## Findings\n\n");
    render_list(&mut text, &analysis.findings, "none");
    text.push_str("\n## Recommendations\n\n");
    render_list(&mut text, &analysis.recommendations, "none");
    text.push_str("\n## Blockers\n\n");
    render_list(&mut text, &analysis.blockers, "none");

    text.push_str("\n## Steam Binaries\n\n");
    for binary in &analysis.binaries {
        text.push_str(&format!(
            "- `{}` `{}` size={}\n",
            binary.label, binary.relative_path, binary.size
        ));
        if let Some(pe) = &binary.pe {
            text.push_str(&format!(
                "  - machine: `{}` subsystem: `{}` layers: `{}`\n",
                pe.machine.as_deref().unwrap_or("unknown"),
                pe.subsystem.as_deref().unwrap_or("unknown"),
                list_or_none(&pe.detected_layers)
            ));
            text.push_str(&format!(
                "  - imports: `{}`\n",
                list_or_none(&pe.import_dlls)
            ));
        }
    }

    text.push_str("\n## CEF Targets\n\n");
    for target in &analysis.cef_targets {
        text.push_str(&format!(
            "- `{}` arch={} helper={} wrapper={} real={} libcef={} vulkan={} d3dcompiler={}\n",
            target.relative_dir,
            target.arch,
            target.helper_exists,
            target.helper_is_wrapper,
            target.real_helper_exists,
            target.libcef_exists,
            target.vulkan_loader_exists,
            target.d3dcompiler_exists
        ));
        if let Some(pe) = &target.helper_pe {
            text.push_str(&format!(
                "  - helper machine: `{}` imports: `{}`\n",
                pe.machine.as_deref().unwrap_or("unknown"),
                list_or_none(&pe.import_dlls)
            ));
        }
    }

    text.push_str("\n## Recent Logs\n\n");
    for log in analysis.logs.iter().take(30) {
        text.push_str(&format!(
            "- `{}` size={} modified={} signal=`{}`\n",
            log.name,
            log.size,
            log.modified_unix_seconds
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            log.signal.as_deref().unwrap_or("none")
        ));
    }

    text.push_str("\n## Packages\n\n");
    for package in analysis.packages.iter().take(40) {
        text.push_str(&format!("- `{}` size={}\n", package.name, package.size));
    }

    if let Some(probe) = &analysis.probe {
        text.push_str("\n## Active Probe\n\n");
        text.push_str(&format!(
            "- started pid: `{}`\n",
            probe
                .started_pid
                .map(|value| value.to_string())
                .unwrap_or_else(|| "none".to_string())
        ));
        text.push_str(&format!(
            "- start error: `{}`\n",
            probe.start_error.as_deref().unwrap_or("none")
        ));
        text.push_str(&format!("- stopped after: `{}`\n", probe.stopped_after));
        text.push_str(&format!(
            "- stop exit code: `{}`\n",
            probe
                .stop_exit_code
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        ));
        for tick in &probe.ticks {
            text.push_str(&format!(
                "- t={}s running={} logged_in={} active_session={} connection=`{}` cef=`{}` issues=`{}`\n",
                tick.elapsed_seconds,
                tick.running,
                optional_bool(tick.logged_in),
                optional_bool(tick.active_session),
                tick.connection_status.as_deref().unwrap_or("unknown"),
                tick.cef_status.as_deref().unwrap_or("unknown"),
                if tick.readiness_issues.is_empty() {
                    "none".to_string()
                } else {
                    tick.readiness_issues.join("; ")
                }
            ));
        }
    }

    text
}

fn render_steam_client_experiment(analysis: &SteamClientAnalysis) -> String {
    let mut text = String::new();
    text.push_str(&format!(
        "id: {}\n",
        if analysis.options.probe {
            "\"001-steam-client-probe\""
        } else {
            "\"001-steam-client-collect\""
        }
    ));
    text.push_str(&format!("analysis_version: {:?}\n", analysis.version));
    text.push_str(&format!("probe: {}\n", analysis.options.probe));
    text.push_str(&format!(
        "probe_seconds: {}\n",
        analysis.options.probe_seconds
    ));
    text.push_str(&format!("stop_after: {}\n", analysis.options.stop_after));
    text.push_str(&format!(
        "steam_dir_found: {}\n",
        analysis.steam_dir.is_some()
    ));
    text.push_str(&format!("binary_count: {}\n", analysis.binaries.len()));
    text.push_str(&format!(
        "cef_target_count: {}\n",
        analysis.cef_targets.len()
    ));
    text.push_str(&format!("blocker_count: {}\n", analysis.blockers.len()));
    text
}

fn path_or_missing(path: Option<&PathBuf>) -> String {
    path.map(|path| path.display().to_string())
        .unwrap_or_else(|| "<not found>".to_string())
}

fn list_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values.join(", ")
    }
}

fn optional_bool(value: Option<bool>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn render_list(text: &mut String, values: &[String], empty: &str) {
    if values.is_empty() {
        text.push_str(&format!("- {empty}\n"));
    } else {
        for value in values {
            text.push_str(&format!("- {value}\n"));
        }
    }
}
