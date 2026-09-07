use super::steam_client_render::{
    process_snapshot, steam_client_output_root, write_steam_client_analysis_artifacts,
};
use super::*;
use crate::*;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const STEAM_CLIENT_ANALYSIS_VERSION: &str = "steam-client-analysis-v1";

#[derive(Debug, Clone)]
pub struct SteamClientAnalysisOptions {
    pub probe: bool,
    pub probe_seconds: u64,
    pub stop_after: bool,
    pub legacy_login: bool,
}

impl Default for SteamClientAnalysisOptions {
    fn default() -> Self {
        Self {
            probe: false,
            probe_seconds: 20,
            stop_after: false,
            legacy_login: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SteamClientAnalysisArtifacts {
    pub root: PathBuf,
    pub report_path: PathBuf,
    pub raw_process_before_path: PathBuf,
    pub raw_process_after_path: PathBuf,
    pub raw_probe_log_path: PathBuf,
    pub experiment_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct SteamClientAnalysis {
    pub version: &'static str,
    pub options: SteamClientAnalysisOptions,
    pub steam_dir: Option<PathBuf>,
    pub runtime: RuntimeSnapshot,
    pub binaries: Vec<SteamClientBinary>,
    pub cef_targets: Vec<SteamCefTargetAnalysis>,
    pub packages: Vec<SteamPackageAnalysis>,
    pub logs: Vec<SteamLogAnalysis>,
    pub probe: Option<SteamClientProbe>,
    pub findings: Vec<String>,
    pub recommendations: Vec<String>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SteamClientBinary {
    pub label: String,
    pub path: PathBuf,
    pub relative_path: String,
    pub size: u64,
    pub pe: Option<PeSummary>,
}

#[derive(Debug, Clone)]
pub struct SteamCefTargetAnalysis {
    pub relative_dir: String,
    pub arch: String,
    pub helper_exists: bool,
    pub helper_is_wrapper: bool,
    pub helper_size: Option<u64>,
    pub real_helper_exists: bool,
    pub libcef_exists: bool,
    pub libcef_size: Option<u64>,
    pub vulkan_loader_exists: bool,
    pub d3dcompiler_exists: bool,
    pub helper_pe: Option<PeSummary>,
}

#[derive(Debug, Clone)]
pub struct SteamPackageAnalysis {
    pub name: String,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct SteamLogAnalysis {
    pub name: String,
    pub size: u64,
    pub modified_unix_seconds: Option<u64>,
    pub signal: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SteamClientProbe {
    pub started_pid: Option<u32>,
    pub start_error: Option<String>,
    pub ticks: Vec<SteamClientProbeTick>,
    pub stopped_after: bool,
    pub stop_exit_code: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct SteamClientProbeTick {
    pub elapsed_seconds: u64,
    pub running: bool,
    pub logged_in: Option<bool>,
    pub connection_status: Option<String>,
    pub cef_status: Option<String>,
    pub active_session: Option<bool>,
    pub readiness_issues: Vec<String>,
}

pub fn collect_steam_client_analysis(
    options: SteamClientAnalysisOptions,
) -> Result<SteamClientAnalysisArtifacts> {
    let mut analysis = analyze_steam_client(&options)?;
    if options.probe {
        let root = steam_client_output_root();
        let raw_dir = root.join("raw");
        fs::create_dir_all(&raw_dir)?;
        analysis.probe = Some(run_steam_client_probe(&options, &raw_dir)?);
        refresh_steam_client_status(&mut analysis);
        analysis.findings = steam_client_findings(&analysis);
        analysis.recommendations = steam_client_recommendations(&analysis);
        analysis.blockers = steam_client_blockers(&analysis);
    }
    write_steam_client_analysis_artifacts(&analysis)
}

pub fn analyze_steam_client(options: &SteamClientAnalysisOptions) -> Result<SteamClientAnalysis> {
    let report = doctor_report();
    let runtime = RuntimeSnapshot::from(&report.wine_steam);
    let steam_dir = report
        .wine_steam
        .steam_exe
        .as_ref()
        .and_then(|path| path.parent())
        .map(Path::to_path_buf);
    let binaries = steam_dir
        .as_ref()
        .map(|steam_dir| steam_client_binaries(steam_dir))
        .transpose()?
        .unwrap_or_default();
    let cef_targets = steam_dir
        .as_ref()
        .map(|steam_dir| steam_cef_targets(steam_dir))
        .transpose()?
        .unwrap_or_default();
    let packages = steam_dir
        .as_ref()
        .map(|steam_dir| steam_packages(steam_dir))
        .transpose()?
        .unwrap_or_default();
    let logs = steam_dir
        .as_ref()
        .map(|steam_dir| steam_logs(steam_dir))
        .transpose()?
        .unwrap_or_default();

    let mut analysis = SteamClientAnalysis {
        version: STEAM_CLIENT_ANALYSIS_VERSION,
        options: options.clone(),
        steam_dir,
        runtime,
        binaries,
        cef_targets,
        packages,
        logs,
        probe: None,
        findings: Vec::new(),
        recommendations: Vec::new(),
        blockers: Vec::new(),
    };
    analysis.findings = steam_client_findings(&analysis);
    analysis.recommendations = steam_client_recommendations(&analysis);
    analysis.blockers = steam_client_blockers(&analysis);
    Ok(analysis)
}

fn run_steam_client_probe(
    options: &SteamClientAnalysisOptions,
    raw_dir: &Path,
) -> Result<SteamClientProbe> {
    let before = raw_dir.join("process-before.txt");
    fs::write(&before, process_snapshot())?;

    let report = doctor_report();
    let runtime = &report.wine_steam;
    let plan = steam_client_start_plan(runtime, options.legacy_login)?;
    let log_path = raw_dir.join("steam-client-probe.log");

    let (started_pid, start_error) = match run_plan_detached_with_log(&plan, &log_path) {
        Ok(pid) => (Some(pid), None),
        Err(error) => (None, Some(error.to_string())),
    };

    let mut ticks = Vec::new();
    if start_error.is_none() {
        let deadline = options.probe_seconds.max(1);
        for elapsed in 0..=deadline {
            ticks.push(steam_client_probe_tick(elapsed));
            if elapsed < deadline {
                thread::sleep(Duration::from_secs(1));
            }
        }
    }

    let (stopped_after, stop_exit_code) = if options.stop_after {
        match wine_steam_stop_plan().and_then(|plan| run_plan(&plan)) {
            Ok(code) => (true, Some(code)),
            Err(_) => (true, None),
        }
    } else {
        (false, None)
    };

    if options.stop_after {
        thread::sleep(Duration::from_secs(1));
    }
    let after = raw_dir.join("process-after.txt");
    fs::write(&after, process_snapshot())?;

    Ok(SteamClientProbe {
        started_pid,
        start_error,
        ticks,
        stopped_after,
        stop_exit_code,
    })
}

fn steam_client_start_plan(runtime: &WineSteamRuntime, legacy_login: bool) -> Result<CommandPlan> {
    let wine = runtime
        .wine
        .as_ref()
        .ok_or_else(|| PortCellarError::Message("Wine was not found".to_string()))?;
    runtime.steam_exe.as_ref().ok_or_else(|| {
        PortCellarError::Message(
            "Windows Steam was not found in the configured Wine prefix".to_string(),
        )
    })?;

    let extra = if legacy_login {
        &["-noreactlogin"][..]
    } else {
        &[][..]
    };
    let mut envs = wine_env(runtime);
    envs.insert("SteamNoOverlayUI".to_string(), "1".to_string());
    envs.insert(
        "DISABLE_VK_LAYER_VALVE_steam_overlay_1".to_string(),
        "1".to_string(),
    );
    envs.insert(
        "WINEDLLOVERRIDES".to_string(),
        prepend_dll_override(
            "gameoverlayrenderer,gameoverlayrenderer64=",
            std::env::var_os("WINEDLLOVERRIDES").as_deref(),
        ),
    );

    Ok(CommandPlan {
        program: wine.clone(),
        args: steam_wine_args(runtime, extra),
        envs,
        current_dir: None,
    })
}

fn steam_client_probe_tick(elapsed_seconds: u64) -> SteamClientProbeTick {
    let report = doctor_report();
    SteamClientProbeTick {
        elapsed_seconds,
        running: report.wine_steam.running,
        logged_in: report.wine_steam.logged_in,
        connection_status: report.wine_steam.connection_status,
        cef_status: report.wine_steam.cef_status,
        active_session: report.wine_steam.active_session,
        readiness_issues: report.wine_steam.readiness_issues,
    }
}

fn refresh_steam_client_status(analysis: &mut SteamClientAnalysis) {
    let report = doctor_report();
    analysis.runtime = RuntimeSnapshot::from(&report.wine_steam);
    analysis.steam_dir = report
        .wine_steam
        .steam_exe
        .as_ref()
        .and_then(|path| path.parent())
        .map(Path::to_path_buf);
    if let Some(steam_dir) = analysis.steam_dir.as_ref() {
        if let Ok(logs) = steam_logs(steam_dir) {
            analysis.logs = logs;
        }
    }
}

fn steam_client_binaries(steam_dir: &Path) -> Result<Vec<SteamClientBinary>> {
    let candidates = [
        ("steam", steam_dir.join("steam.exe")),
        ("steam service", steam_dir.join("bin/SteamService.exe")),
        ("steam monitor", steam_dir.join("bin/steam_monitor.exe")),
        ("x86 launcher", steam_dir.join("bin/x86launcher.exe")),
        ("x64 launcher", steam_dir.join("bin/x64launcher.exe")),
        ("game overlay ui", steam_dir.join("bin/gameoverlayui.dll")),
        (
            "overlay renderer x86",
            steam_dir.join("GameOverlayRenderer.dll"),
        ),
        (
            "overlay renderer x64",
            steam_dir.join("GameOverlayRenderer64.dll"),
        ),
        (
            "vulkan driver query x86",
            steam_dir.join("bin/vulkandriverquery.exe"),
        ),
        (
            "vulkan driver query x64",
            steam_dir.join("bin/vulkandriverquery64.exe"),
        ),
        (
            "gl driver query x86",
            steam_dir.join("bin/gldriverquery.exe"),
        ),
        (
            "gl driver query x64",
            steam_dir.join("bin/gldriverquery64.exe"),
        ),
    ];
    let mut binaries = Vec::new();
    for (label, path) in candidates {
        if path.exists() {
            let size = fs::metadata(&path)?.len();
            binaries.push(SteamClientBinary {
                label: label.to_string(),
                relative_path: relative_display(steam_dir, &path),
                pe: pe::summarize_pe(&path),
                path,
                size,
            });
        }
    }
    Ok(binaries)
}

fn steam_cef_targets(steam_dir: &Path) -> Result<Vec<SteamCefTargetAnalysis>> {
    let cef_root = steam_dir.join("bin/cef");
    let mut targets = Vec::new();
    if !cef_root.exists() {
        return Ok(targets);
    }

    for entry in fs::read_dir(cef_root)? {
        let entry = entry?;
        let cef_dir = entry.path();
        if !cef_dir.is_dir() {
            continue;
        }
        let Some(name) = cef_dir.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.starts_with("cef.win") {
            continue;
        }

        let helper = cef_dir.join("steamwebhelper.exe");
        let real_helper = cef_dir.join("steamwebhelper_real.exe");
        let libcef = cef_dir.join("libcef.dll");
        let helper_size = helper.metadata().ok().map(|metadata| metadata.len());
        let libcef_size = libcef.metadata().ok().map(|metadata| metadata.len());
        targets.push(SteamCefTargetAnalysis {
            relative_dir: relative_display(steam_dir, &cef_dir),
            arch: if name.contains("64") { "x86_64" } else { "x86" }.to_string(),
            helper_exists: helper.exists(),
            helper_is_wrapper: helper.exists() && is_current_steamwebhelper_wrapper(&helper),
            helper_size,
            real_helper_exists: real_helper.exists(),
            libcef_exists: libcef.exists(),
            libcef_size,
            vulkan_loader_exists: cef_dir.join("vulkan-1.dll").exists(),
            d3dcompiler_exists: cef_dir.join("d3dcompiler_47.dll").exists(),
            helper_pe: pe::summarize_pe(&helper),
        });
    }
    targets.sort_by(|left, right| left.relative_dir.cmp(&right.relative_dir));
    Ok(targets)
}

fn steam_packages(steam_dir: &Path) -> Result<Vec<SteamPackageAnalysis>> {
    let package_dir = steam_dir.join("package");
    let mut packages = Vec::new();
    if !package_dir.exists() {
        return Ok(packages);
    }

    for entry in fs::read_dir(package_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if name.ends_with(".manifest") || name.ends_with(".installed") || name.contains(".zip") {
            packages.push(SteamPackageAnalysis {
                name: name.to_string(),
                size: entry.metadata()?.len(),
            });
        }
    }
    packages.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(packages)
}

fn steam_logs(steam_dir: &Path) -> Result<Vec<SteamLogAnalysis>> {
    let logs_dir = steam_dir.join("logs");
    let mut logs = Vec::new();
    if !logs_dir.exists() {
        return Ok(logs);
    }

    for entry in fs::read_dir(logs_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let metadata = entry.metadata()?;
        let signal = steam_log_signal(&path);
        logs.push(SteamLogAnalysis {
            name: name.to_string(),
            size: metadata.len(),
            modified_unix_seconds: metadata.modified().ok().and_then(unix_seconds),
            signal,
        });
    }
    logs.sort_by(|left, right| {
        right
            .modified_unix_seconds
            .cmp(&left.modified_unix_seconds)
            .then_with(|| left.name.cmp(&right.name))
    });
    Ok(logs)
}

fn steam_log_signal(path: &Path) -> Option<String> {
    let text = tail_text(path, 128 * 1024)?;
    steam_log_signal_from_text(&text)
}

pub(crate) fn steam_log_signal_from_text(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    if lower.contains("fatal:process_metrics_win") {
        Some("cef fatal process_metrics_win".to_string())
    } else if lower.matches("restart webhelper process").count() >= 3 {
        Some("webhelper restart loop".to_string())
    } else if lower.contains("eglcreatecontext") {
        Some("cef graphics context warning".to_string())
    } else if lower.contains("connection status: connected")
        || lower.contains("connectioncompleted()")
        || lower.contains("connection ready")
    {
        Some("transport connected".to_string())
    } else if lower.contains("received logon success response")
        || lower.contains("setloginstate: success")
    {
        Some("login success".to_string())
    } else if lower.contains("failed to poll auth session")
        || lower.contains("waitingforcredentials")
    {
        Some("login attention needed".to_string())
    } else {
        None
    }
}

fn tail_text(path: &Path, max_bytes: u64) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    let start = bytes.len().saturating_sub(max_bytes as usize);
    Some(String::from_utf8_lossy(&bytes[start..]).to_string())
}

fn steam_client_findings(analysis: &SteamClientAnalysis) -> Vec<String> {
    let mut findings = BTreeSet::new();
    if analysis.steam_dir.is_some() {
        findings.insert("Windows Steam install detected in the selected Wine prefix.".to_string());
    }
    if analysis
        .cef_targets
        .iter()
        .any(|target| target.helper_is_wrapper)
    {
        findings.insert(
            "Current steamwebhelper wrapper is installed for at least one CEF target.".to_string(),
        );
    }
    if analysis
        .cef_targets
        .iter()
        .any(|target| target.libcef_exists && target.vulkan_loader_exists)
    {
        findings.insert("Steam CEF ships its own libcef and Vulkan loader stack.".to_string());
    }
    if analysis
        .logs
        .iter()
        .any(|log| log.signal.as_deref() == Some("transport connected"))
    {
        findings
            .insert("Steam logs show a previously connected WebUI/client transport.".to_string());
    }
    if analysis
        .logs
        .iter()
        .any(|log| log.signal.as_deref() == Some("webhelper restart loop"))
    {
        findings.insert(
            "Recent Steam UI logs still contain steamwebhelper restart-loop evidence.".to_string(),
        );
    }
    if let Some(probe) = &analysis.probe {
        if probe.ticks.iter().any(|tick| tick.running) {
            findings.insert("Active probe observed Wine Steam processes running.".to_string());
        }
        if probe.ticks.iter().any(|tick| {
            tick.cef_status
                .as_deref()
                .unwrap_or("")
                .contains("login-window-ready")
        }) {
            findings
                .insert("Active probe reached a Steam CEF login/window-ready state.".to_string());
        }
        if probe.ticks.iter().any(|tick| tick.logged_in == Some(true))
            && !probe
                .ticks
                .iter()
                .any(|tick| tick.active_session == Some(true))
        {
            findings.insert(
                "Active probe did not observe a nonzero SteamID webhelper session; login readiness is inferred from cache/logs."
                    .to_string(),
            );
        }
    }
    findings.into_iter().collect()
}

fn steam_client_recommendations(analysis: &SteamClientAnalysis) -> Vec<String> {
    let mut recommendations = BTreeSet::new();
    recommendations.insert(
        "Keep Steam client compatibility separate from per-game profiles; CEF and overlay failures can mask game issues."
            .to_string(),
    );
    recommendations.insert(
        "Continue disabling Steam overlay for baseline game probes until Steam CEF is stable."
            .to_string(),
    );
    recommendations.insert(
        "Gate game launch on live Steam transport/session checks, not cached credentials alone."
            .to_string(),
    );
    if analysis
        .cef_targets
        .iter()
        .any(|target| target.helper_exists && !target.helper_is_wrapper)
    {
        recommendations.insert(
            "Patch or verify steamwebhelper wrapper before active Wine Steam login probes."
                .to_string(),
        );
    }
    if analysis
        .logs
        .iter()
        .any(|log| log.signal.as_deref() == Some("cef graphics context warning"))
    {
        recommendations.insert(
            "Prioritize CEF graphics-context workarounds before changing game render backends."
                .to_string(),
        );
    }
    if analysis
        .logs
        .iter()
        .any(|log| log.signal.as_deref() == Some("login attention needed"))
    {
        recommendations.insert(
            "Treat login/auth transport as a Steam-client layer issue, not a game profile issue."
                .to_string(),
        );
    }
    recommendations.into_iter().collect()
}

fn steam_client_blockers(analysis: &SteamClientAnalysis) -> Vec<String> {
    let mut blockers = Vec::new();
    if analysis.runtime.wine.is_none() {
        blockers.push("Wine engine missing".to_string());
    }
    if analysis.runtime.steam_exe.is_none() {
        blockers.push("Windows Steam executable missing in selected prefix".to_string());
    }
    if analysis.steam_dir.is_none() {
        blockers.push("Steam directory could not be derived from steam.exe".to_string());
    }
    if analysis.options.probe {
        if let Some(probe) = &analysis.probe {
            if let Some(error) = &probe.start_error {
                blockers.push(format!("Active probe could not start Steam: {error}"));
            }
            if !probe.ticks.iter().any(|tick| tick.running) {
                blockers
                    .push("Active probe did not observe running Wine Steam processes".to_string());
            }
        }
    }
    blockers
}

fn unix_seconds(time: SystemTime) -> Option<u64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|value| value.as_secs())
}

fn relative_display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("/")
}
