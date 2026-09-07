use super::*;
use crate::*;
use std::fs;
use std::path::PathBuf;

pub fn write_porting_analysis_artifacts(
    analysis: &GamePortingAnalysis,
) -> Result<PortingAnalysisArtifacts> {
    let root = analysis_output_root(analysis);
    let raw_dir = root.join("raw");
    let experiments_dir = root.join("experiments");
    fs::create_dir_all(&raw_dir)?;
    fs::create_dir_all(&experiments_dir)?;

    let report_path = root.join("report.md");
    let profile_draft_path = root.join("profile-draft.yml");
    let profile_path = root.join("profile.toml");
    let reproduce_path = root.join("reproduce.sh");
    let raw_doctor_path = raw_dir.join("doctor-summary.txt");
    let experiment_path = experiments_dir.join("001-collect-only.yml");

    fs::write(&report_path, render_report(analysis))?;
    fs::write(&profile_draft_path, render_profile_draft(analysis))?;
    let profile_path = match analysis.to_generic_profile_candidate() {
        Ok(candidate) => {
            candidate.profile.write_profile_toml(&profile_path)?;
            Some(profile_path)
        }
        Err(_) => None,
    };
    fs::write(&reproduce_path, render_reproduce_script(analysis))?;
    fs::write(&raw_doctor_path, render_raw_doctor_summary(analysis))?;
    fs::write(&experiment_path, render_collect_experiment(analysis))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&reproduce_path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&reproduce_path, permissions)?;
    }

    Ok(PortingAnalysisArtifacts {
        root,
        report_path,
        profile_draft_path,
        profile_path,
        reproduce_path,
        raw_doctor_path,
        experiment_path,
    })
}

fn analysis_output_root(analysis: &GamePortingAnalysis) -> PathBuf {
    state_root()
        .join("analysis")
        .join(format!("{}-{}", analysis.app_id, analysis.slug))
}

fn render_report(analysis: &GamePortingAnalysis) -> String {
    let mut text = String::new();
    text.push_str("# Porting Analysis Report\n\n");
    text.push_str(
        "This report is private and may contain local filesystem paths. Keep it under `.portcellar/`.\n\n",
    );
    text.push_str("## Identity\n\n");
    text.push_str(&format!("- analysis version: `{}`\n", analysis.version));
    text.push_str(&format!("- app id: `{}`\n", analysis.app_id));
    if let Some(manifest) = &analysis.manifest {
        text.push_str(&format!("- name: `{}`\n", manifest.name));
        text.push_str(&format!("- source: `{}`\n", manifest.source));
        text.push_str(&format!(
            "- build id: `{}`\n",
            manifest.build_id.as_deref().unwrap_or("unknown")
        ));
        text.push_str(&format!(
            "- manifest: `{}`\n",
            manifest.manifest_path.display()
        ));
        text.push_str(&format!(
            "- install dir: `{}`\n",
            manifest.install_dir.display()
        ));
    } else {
        text.push_str("- manifest: `not found`\n");
    }

    text.push_str("\n## Runtime Snapshot\n\n");
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
        "- Windows version: `{}`\n",
        analysis
            .runtime
            .windows_version
            .as_deref()
            .unwrap_or("unknown")
    ));
    text.push_str(&format!(
        "- prefix: `{}`\n",
        analysis.runtime.prefix.display()
    ));
    text.push_str(&format!(
        "- Steam exe: `{}`\n",
        path_or_missing(analysis.runtime.steam_exe.as_ref())
    ));
    text.push_str(&format!(
        "- DXMT root: `{}`\n",
        path_or_missing(analysis.runtime.dxmt_root.as_ref())
    ));
    text.push_str(&format!(
        "- Steam running: `{}`\n",
        analysis.runtime.steam_running
    ));
    text.push_str(&format!(
        "- logged in: `{}`\n",
        analysis
            .runtime
            .logged_in
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    ));
    text.push_str(&format!(
        "- connection: `{}`\n",
        analysis
            .runtime
            .connection_status
            .as_deref()
            .unwrap_or("unknown")
    ));
    text.push_str(&format!(
        "- CEF: `{}`\n",
        analysis.runtime.cef_status.as_deref().unwrap_or("unknown")
    ));

    text.push_str("\n## Executable Candidates\n\n");
    if analysis.exe_candidates.is_empty() {
        text.push_str("- none found\n");
    } else {
        for candidate in &analysis.exe_candidates {
            text.push_str(&format!(
                "- `{}` score={} size={}\n",
                candidate.relative_path, candidate.score, candidate.size
            ));
            if let Some(pe) = &candidate.pe {
                text.push_str(&format!(
                    "  - machine: `{}` subsystem: `{}`\n",
                    pe.machine.as_deref().unwrap_or("unknown"),
                    pe.subsystem.as_deref().unwrap_or("unknown")
                ));
                text.push_str(&format!(
                    "  - layers: `{}`\n",
                    list_or_none(&pe.detected_layers)
                ));
                text.push_str(&format!(
                    "  - imports: `{}`\n",
                    list_or_none(&pe.import_dlls)
                ));
            }
        }
    }

    text.push_str("\n## Bundled DLLs\n\n");
    if analysis.bundled_dlls.is_empty() {
        text.push_str("- none found in scanned depth\n");
    } else {
        for dll in &analysis.bundled_dlls {
            text.push_str(&format!("- `{dll}`\n"));
        }
    }

    text.push_str("\n## Recommendations\n\n");
    for item in &analysis.recommendations {
        text.push_str(&format!("- {item}\n"));
    }

    text.push_str("\n## Blockers\n\n");
    if analysis.blockers.is_empty() {
        text.push_str("- none detected by collect-only analysis\n");
    } else {
        for item in &analysis.blockers {
            text.push_str(&format!("- {item}\n"));
        }
    }

    text
}

fn render_profile_draft(analysis: &GamePortingAnalysis) -> String {
    let manifest = analysis.manifest.as_ref();
    let selected = analysis.selected_exe.as_ref();
    let selected_pe = selected.and_then(|candidate| candidate.pe.as_ref());
    let name = manifest
        .map(|manifest| manifest.name.as_str())
        .unwrap_or("unknown");
    let install_dir_hint = manifest
        .map(|manifest| manifest.install_dir_name.as_str())
        .unwrap_or("unknown");
    let windows_exe = selected
        .map(|candidate| candidate.relative_path.as_str())
        .unwrap_or("unknown");

    let mut text = String::new();
    text.push_str(
        "# Generated draft. Validate manually before turning this into Rust GameProfile code.\n",
    );
    text.push_str(&format!(
        "analysis_version: {}\n",
        yaml_string(analysis.version)
    ));
    text.push_str("app:\n");
    text.push_str(&format!("  id: {}\n", yaml_string(&analysis.app_id)));
    text.push_str(&format!("  name: {}\n", yaml_string(name)));
    text.push_str(&format!(
        "  install_dir_hint: {}\n",
        yaml_string(install_dir_hint)
    ));
    text.push_str(&format!("  windows_exe: {}\n", yaml_string(windows_exe)));
    text.push_str(&format!(
        "  arch: {}\n",
        yaml_string(
            selected_pe
                .and_then(|pe| pe.machine.as_deref())
                .unwrap_or("unknown")
        )
    ));
    text.push_str("runtime:\n");
    text.push_str("  recommended_mode: \"wine-steam\"\n");
    text.push_str(&format!(
        "  wine_version: {}\n",
        yaml_string(
            analysis
                .runtime
                .wine_version
                .as_deref()
                .unwrap_or("unknown")
        )
    ));
    text.push_str(&format!(
        "  windows_version: {}\n",
        yaml_string(
            analysis
                .runtime
                .windows_version
                .as_deref()
                .unwrap_or("unknown")
        )
    ));
    text.push_str(&format!(
        "  detected_layers: [{}]\n",
        yaml_list(
            selected_pe
                .map(|pe| pe.detected_layers.as_slice())
                .unwrap_or(&[])
        )
    ));
    text.push_str("steam:\n");
    text.push_str("  requires_client: true\n");
    text.push_str("  overlay_recommended_off: true\n");
    text.push_str("evidence:\n");
    text.push_str("  successful_experiment: null\n");
    text.push_str("  collect_experiment: \"experiments/001-collect-only.yml\"\n");
    text.push_str("recommendations:\n");
    for recommendation in &analysis.recommendations {
        text.push_str(&format!("  - {}\n", yaml_string(recommendation)));
    }
    if analysis.blockers.is_empty() {
        text.push_str("blockers: []\n");
    } else {
        text.push_str("blockers:\n");
        for blocker in &analysis.blockers {
            text.push_str(&format!("  - {}\n", yaml_string(blocker)));
        }
    }
    text.push_str("next_steps:\n");
    text.push_str("  - \"Run a timed wine-steam probe and capture Wine/graphics logs.\"\n");
    text.push_str(
        "  - \"Validate save path and Steam Cloud behavior after first successful launch.\"\n",
    );
    text
}

fn render_reproduce_script(analysis: &GamePortingAnalysis) -> String {
    let Some(wine) = analysis.runtime.wine.as_ref() else {
        return "#!/usr/bin/env bash\nset -euo pipefail\n\necho 'Wine engine was not detected during analysis.'\nexit 1\n".to_string();
    };
    let Some(steam_exe) = analysis.runtime.steam_exe.as_ref() else {
        return "#!/usr/bin/env bash\nset -euo pipefail\n\necho 'Windows Steam was not detected during analysis.'\nexit 1\n".to_string();
    };
    let steam_arg = windows_path_in_prefix(&analysis.runtime.prefix, steam_exe)
        .unwrap_or_else(|| steam_exe.display().to_string());

    let mut text = String::new();
    text.push_str("#!/usr/bin/env bash\nset -euo pipefail\n\n");
    text.push_str(&format!(
        "export WINEPREFIX={}\n",
        shell_single_quote(&analysis.runtime.prefix.display().to_string())
    ));
    text.push_str("export WINEDEBUG=${PORTCELLAR_WINEDEBUG:--all}\n");
    text.push_str("export MVK_CONFIG_LOG_LEVEL=0\n");
    if let Some(dxmt_root) = &analysis.runtime.dxmt_root {
        text.push_str(&format!(
            "export WINEDLLPATH_PREPEND={}\n",
            shell_single_quote(&dxmt_root.display().to_string())
        ));
        text.push_str("export DXMT_LOG_LEVEL=${DXMT_LOG_LEVEL:-error}\n");
    }
    text.push_str(&format!(
        "export SteamAppId={}\n",
        shell_single_quote(&analysis.app_id)
    ));
    text.push_str(&format!(
        "export SteamGameId={}\n",
        shell_single_quote(&analysis.app_id)
    ));
    text.push_str(&format!(
        "export STEAM_GAME_ID={}\n",
        shell_single_quote(&analysis.app_id)
    ));
    text.push_str("export SteamNoOverlayUI=1\n");
    text.push_str("export DISABLE_VK_LAYER_VALVE_steam_overlay_1=1\n");
    text.push_str("export WINEDLLOVERRIDES='gameoverlayrenderer,gameoverlayrenderer64='\n\n");
    text.push_str(&format!(
        "exec {} {}",
        shell_single_quote(&wine.display().to_string()),
        shell_single_quote(&steam_arg)
    ));
    for arg in STEAM_WINE_CEF_ARGS {
        text.push_str(&format!(" {}", shell_single_quote(arg)));
    }
    text.push_str(&format!(
        " -silent -applaunch {}\n",
        shell_single_quote(&analysis.app_id)
    ));
    text
}

fn render_raw_doctor_summary(analysis: &GamePortingAnalysis) -> String {
    let mut text = String::new();
    text.push_str("portcellar raw doctor snapshot for porting analysis\n\n");
    text.push_str(&format!("app_id={}\n", analysis.app_id));
    text.push_str(&format!(
        "wine={}\n",
        path_or_missing(analysis.runtime.wine.as_ref())
    ));
    text.push_str(&format!("prefix={}\n", analysis.runtime.prefix.display()));
    text.push_str(&format!(
        "steam_exe={}\n",
        path_or_missing(analysis.runtime.steam_exe.as_ref())
    ));
    text.push_str(&format!(
        "dxmt_root={}\n",
        path_or_missing(analysis.runtime.dxmt_root.as_ref())
    ));
    text.push_str(&format!(
        "readiness_issues={:?}\n",
        analysis.runtime.readiness_issues
    ));
    if let Some(crash) = &analysis.latest_crash {
        text.push_str(&format!("latest_crash={}\n", crash.path.display()));
        text.push_str(&format!("crash_exception={:?}\n", crash.exception));
        text.push_str(&format!("crash_diagnosis={:?}\n", crash.diagnosis));
    }
    text
}

fn render_collect_experiment(analysis: &GamePortingAnalysis) -> String {
    let mut text = String::new();
    text.push_str("id: \"001-collect-only\"\n");
    text.push_str("mode: \"read-only\"\n");
    text.push_str(&format!("app_id: {}\n", yaml_string(&analysis.app_id)));
    text.push_str("result: \"collected\"\n");
    text.push_str("launch_attempted: false\n");
    text.push_str(&format!(
        "manifest_found: {}\n",
        analysis.manifest.is_some()
    ));
    text.push_str(&format!(
        "exe_candidates: {}\n",
        analysis.exe_candidates.len()
    ));
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

fn yaml_list(values: &[String]) -> String {
    values
        .iter()
        .map(|value| yaml_string(value))
        .collect::<Vec<_>>()
        .join(", ")
}

fn yaml_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
