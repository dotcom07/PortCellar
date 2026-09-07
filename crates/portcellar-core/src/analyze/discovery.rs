use super::*;
use crate::*;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

pub(super) fn validate_app_id(app_id: &str) -> Result<()> {
    let valid = !app_id.is_empty() && app_id.chars().all(|ch| ch.is_ascii_digit());
    if valid {
        Ok(())
    } else {
        Err(PortCellarError::Message(format!(
            "Steam app id must be numeric, got {app_id:?}"
        )))
    }
}

pub(super) fn find_app_manifest(app_id: &str, report: &DoctorReport) -> Option<SteamAppManifest> {
    wine_manifest(app_id, &report.wine_steam).or_else(|| native_manifest(app_id, &report.steam))
}

pub(crate) fn windows_exe_candidates(install_dir: &Path) -> Result<Vec<WindowsExeCandidate>> {
    let mut paths = Vec::new();
    collect_by_extension(install_dir, "exe", 5, &mut paths)?;
    let mut candidates = paths
        .into_iter()
        .map(|path| exe_candidate(install_dir, path))
        .collect::<Result<Vec<_>>>()?;
    candidates.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.relative_path.cmp(&right.relative_path))
    });
    Ok(candidates)
}

pub(crate) fn select_exe_candidate(
    candidates: &[WindowsExeCandidate],
) -> Option<&WindowsExeCandidate> {
    candidates.first()
}

pub(crate) fn bundled_dlls(install_dir: &Path) -> Result<Vec<String>> {
    let mut paths = Vec::new();
    collect_by_extension(install_dir, "dll", 4, &mut paths)?;
    let mut dlls = paths
        .into_iter()
        .map(|path| relative_display(install_dir, &path))
        .collect::<Vec<_>>();
    dlls.sort();
    Ok(dlls)
}

pub(super) fn recommendations_and_blockers(
    manifest: &Option<SteamAppManifest>,
    candidates: &[WindowsExeCandidate],
    selected: Option<&WindowsExeCandidate>,
    runtime: &RuntimeSnapshot,
) -> (Vec<String>, Vec<String>) {
    let mut recommendations = BTreeSet::new();
    let mut blockers = Vec::new();

    if runtime.wine.is_none() {
        blockers.push("Wine engine missing".to_string());
    }
    if runtime.steam_exe.is_none() {
        blockers.push("Windows Steam missing in the selected prefix".to_string());
    }
    if manifest.is_none() {
        blockers.push("Steam app manifest not found in Wine or native Steam libraries".to_string());
    }
    if candidates.is_empty() {
        blockers.push("No Windows .exe candidates found under the install directory".to_string());
    }
    for issue in &runtime.readiness_issues {
        blockers.push(issue.clone());
    }

    if selected
        .and_then(|candidate| candidate.pe.as_ref())
        .map(|pe| pe.detected_layers.iter().any(|layer| layer == "d3d12"))
        .unwrap_or(false)
    {
        recommendations.insert(
            "Try GPTK/D3DMetal or vkd3d-proton; DXMT alone does not cover D3D12.".to_string(),
        );
    }
    if selected
        .and_then(|candidate| candidate.pe.as_ref())
        .map(|pe| pe.detected_layers.iter().any(|layer| layer == "d3d11/dxgi"))
        .unwrap_or(false)
    {
        recommendations
            .insert("Try DXMT on Apple Silicon and start with a conservative FPS cap.".to_string());
    }
    if selected
        .and_then(|candidate| candidate.pe.as_ref())
        .map(|pe| pe.detected_layers.iter().any(|layer| layer == "d3d9"))
        .unwrap_or(false)
    {
        recommendations.insert("Try DXVK or WineD3D for D3D9-era rendering.".to_string());
    }
    if selected
        .and_then(|candidate| candidate.pe.as_ref())
        .map(|pe| pe.detected_layers.iter().any(|layer| layer == "opengl"))
        .unwrap_or(false)
    {
        recommendations.insert(
            "Check Wine OpenGL/macOS driver behavior first; DXMT may not be in this game's hot path."
                .to_string(),
        );
    }
    if selected
        .and_then(|candidate| candidate.pe.as_ref())
        .map(|pe| {
            pe.detected_layers
                .iter()
                .any(|layer| layer == "media-codec")
        })
        .unwrap_or(false)
    {
        recommendations.insert(
            "Check intro/video playback early; Media Foundation or DirectShow often needs per-game workarounds."
                .to_string(),
        );
    }
    if selected
        .and_then(|candidate| candidate.pe.as_ref())
        .map(|pe| pe.detected_layers.iter().any(|layer| layer == "steamworks"))
        .unwrap_or(false)
    {
        recommendations.insert(
            "Prefer wine-steam first so Steamworks calls see a live Windows Steam client."
                .to_string(),
        );
        recommendations
            .insert("Disable Steam overlay until the baseline launch is stable.".to_string());
    }
    if selected
        .and_then(|candidate| candidate.pe.as_ref())
        .map(|pe| {
            pe.detected_layers
                .iter()
                .any(|layer| layer == "anti-cheat-risk")
        })
        .unwrap_or(false)
    {
        blockers.push("Anti-cheat import detected; Wine compatibility may require unsupported kernel/driver behavior.".to_string());
    }

    if recommendations.is_empty() {
        recommendations.insert(
            "Run a short wine-steam probe next and inspect Wine, crash, and graphics logs."
                .to_string(),
        );
    }

    (recommendations.into_iter().collect(), blockers)
}

pub(crate) fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;
    for ch in value.chars() {
        let next = if ch.is_ascii_alphanumeric() {
            previous_dash = false;
            Some(ch.to_ascii_lowercase())
        } else if !previous_dash {
            previous_dash = true;
            Some('-')
        } else {
            None
        };
        if let Some(next) = next {
            slug.push(next);
        }
        if slug.len() >= 48 {
            break;
        }
    }
    slug.trim_matches('-').to_string()
}

fn wine_manifest(app_id: &str, runtime: &WineSteamRuntime) -> Option<SteamAppManifest> {
    let steam_dir = runtime.steam_exe.as_ref()?.parent()?;
    manifest_from_steamapps(app_id, "wine-prefix", &steam_dir.join("steamapps"))
}

fn native_manifest(app_id: &str, steam: &SteamReport) -> Option<SteamAppManifest> {
    steam.libraries.iter().find_map(|library| {
        manifest_from_steamapps(app_id, "native-steam", &library.join("steamapps"))
    })
}

fn manifest_from_steamapps(
    app_id: &str,
    source: &str,
    steamapps: &Path,
) -> Option<SteamAppManifest> {
    let manifest_path = steamapps.join(format!("appmanifest_{app_id}.acf"));
    let text = fs::read_to_string(&manifest_path).ok()?;
    let values = parse_key_values(&text);
    let install_dir_name = values.get("installdir")?.to_string();
    let install_dir = steamapps.join("common").join(&install_dir_name);

    Some(SteamAppManifest {
        source: source.to_string(),
        app_id: values
            .get("appid")
            .cloned()
            .unwrap_or_else(|| app_id.to_string()),
        name: values
            .get("name")
            .cloned()
            .unwrap_or_else(|| format!("Steam app {app_id}")),
        build_id: values.get("buildid").cloned(),
        install_dir_name,
        manifest_path,
        install_dir,
    })
}

fn exe_candidate(install_dir: &Path, path: PathBuf) -> Result<WindowsExeCandidate> {
    let size = fs::metadata(&path)?.len();
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_string();
    let relative_path = relative_display(install_dir, &path);
    let pe = pe::summarize_pe(&path);
    let (score, reasons) = score_exe_candidate(&file_name, pe.as_ref());

    Ok(WindowsExeCandidate {
        path,
        relative_path,
        file_name,
        size,
        pe,
        score,
        reasons,
    })
}

fn score_exe_candidate(file_name: &str, pe: Option<&PeSummary>) -> (i32, Vec<String>) {
    let lower = file_name.to_ascii_lowercase();
    let mut score = 0;
    let mut reasons = Vec::new();

    if pe.is_some() {
        score += 20;
        reasons.push("valid PE".to_string());
    }
    if lower.contains("launcher") {
        score -= 10;
        reasons.push("launcher-like name".to_string());
    }
    if ["crash", "redist", "setup", "unins", "vcredist", "dotnet"]
        .iter()
        .any(|needle| lower.contains(needle))
    {
        score -= 30;
        reasons.push("helper/installer-like name".to_string());
    }
    if let Some(pe) = pe {
        if pe.detected_layers.iter().any(|layer| layer == "steamworks") {
            score += 10;
            reasons.push("imports Steamworks".to_string());
        }
        if pe.detected_layers.iter().any(|layer| {
            matches!(
                layer.as_str(),
                "d3d11/dxgi" | "d3d12" | "d3d9" | "opengl" | "vulkan"
            )
        }) {
            score += 8;
            reasons.push("imports graphics layer".to_string());
        }
    }

    (score, reasons)
}

fn collect_by_extension(
    root: &Path,
    extension: &str,
    max_depth: usize,
    output: &mut Vec<PathBuf>,
) -> Result<()> {
    if max_depth == 0 || !root.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            collect_by_extension(&path, extension, max_depth - 1, output)?;
        } else if path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.eq_ignore_ascii_case(extension))
            .unwrap_or(false)
        {
            output.push(path);
        }
    }

    Ok(())
}

fn relative_display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("/")
}
