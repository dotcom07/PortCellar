use super::discovery;
use crate::*;
use std::fs;
use std::path::{Component, Path, PathBuf};

pub const LOCAL_GAME_ANALYSIS_VERSION: &str = "local-game-analysis-v1";

#[derive(Debug, Clone)]
pub struct LocalGameAnalysis {
    pub version: &'static str,
    pub root: PathBuf,
    pub name: String,
    pub slug: String,
    pub app_id: String,
    pub windows_install_path: String,
    pub exe_candidates: Vec<WindowsExeCandidate>,
    pub selected_exe: Option<WindowsExeCandidate>,
    pub bundled_dlls: Vec<String>,
    pub profile: GenericGameProfile,
    pub review_items: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct LocalGameAnalysisArtifacts {
    pub root: PathBuf,
    pub report_path: PathBuf,
    pub profile_path: PathBuf,
    pub reproduce_path: PathBuf,
}

pub fn analyze_local_game(
    root: &Path,
    name: Option<&str>,
    app_id: Option<&str>,
    executable: Option<&str>,
) -> Result<LocalGameAnalysis> {
    let root = fs::canonicalize(root).map_err(|error| {
        PortCellarError::Message(format!(
            "local game directory could not be resolved: {error}"
        ))
    })?;
    if !root.is_dir() {
        return Err(PortCellarError::Message(format!(
            "local game path is not a directory: {}",
            root.display()
        )));
    }

    let exe_candidates = discovery::windows_exe_candidates(&root)?;
    let selected_exe = select_executable(&exe_candidates, executable)?;
    let name = name
        .map(str::to_string)
        .or_else(|| {
            root.file_name()
                .and_then(|value| value.to_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "Local Windows Game".to_string());
    let slug = discovery::slugify(&name);
    let embedded_app_id = read_steam_app_id(&root);
    let app_id = app_id
        .map(str::to_string)
        .unwrap_or_else(|| format!("local-{}", if slug.is_empty() { "game" } else { &slug }));
    let steam_integration = SteamIntegration::None;
    let windows_install_path = host_path_to_z_path(&root)?;
    let bundled_dlls = discovery::bundled_dlls(&root)?;
    let selected_pe = selected_exe
        .as_ref()
        .and_then(|candidate| candidate.pe.as_ref());
    let mut profile = GenericGameProfile::new(
        app_id.clone(),
        name.clone(),
        selected_exe
            .as_ref()
            .map(|candidate| candidate.relative_path.clone())
            .unwrap_or_else(|| "Game.exe".to_string()),
    )
    .with_bottle_name(format!(
        "local-{}",
        if slug.is_empty() { "game" } else { &slug }
    ))
    .with_install_dir_hint(name.clone())
    .with_windows_install_path(windows_install_path.clone())
    .with_runtime_profile_version(format!("{}-{}", LOCAL_GAME_ANALYSIS_VERSION, app_id))
    .with_steam_integration(steam_integration)
    .with_steam_cef_policy(SteamCefPolicy::WineDefault);

    let mut review_items = Vec::new();
    if let Some(steam_app_id) = embedded_app_id {
        review_items.push(format!(
            "steam_appid.txt detected ({steam_app_id}); recorded as static evidence only and ignored by the local launch policy"
        ));
    } else {
        review_items.push(
            "no steam_appid.txt detected; profile uses direct non-Steam Wine launch".to_string(),
        );
    }
    review_items.push(
        "the local folder is exposed through Wine Z:; keep the host path stable after profiling"
            .to_string(),
    );
    if exe_candidates.len() > 1 {
        review_items.push(format!(
            "{} executable candidates found; review the selected executable before launch",
            exe_candidates.len()
        ));
    }

    if let Some(pe) = selected_pe {
        apply_pe_evidence(&mut profile, pe, &mut review_items);
    } else {
        review_items.push("selected executable has no readable PE summary".to_string());
    }

    let recommendations = local_recommendations(selected_pe);
    review_items.extend(recommendations);

    Ok(LocalGameAnalysis {
        version: LOCAL_GAME_ANALYSIS_VERSION,
        root,
        name,
        slug,
        app_id,
        windows_install_path,
        exe_candidates,
        selected_exe,
        bundled_dlls,
        profile,
        review_items,
    })
}

pub fn write_local_game_analysis_artifacts(
    analysis: &LocalGameAnalysis,
) -> Result<LocalGameAnalysisArtifacts> {
    let artifact_root = state_root()
        .join("analysis")
        .join(format!("{}-{}", analysis.app_id, analysis.slug));
    fs::create_dir_all(&artifact_root)?;

    let report_path = artifact_root.join("report.md");
    let profile_path = artifact_root.join("profile.toml");
    let reproduce_path = artifact_root.join("reproduce.sh");
    fs::write(&report_path, render_report(analysis))?;
    analysis.profile.write_profile_toml(&profile_path)?;
    fs::write(
        &reproduce_path,
        render_reproduce_script(analysis, &artifact_root),
    )?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&reproduce_path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&reproduce_path, permissions)?;
    }

    Ok(LocalGameAnalysisArtifacts {
        root: artifact_root,
        report_path,
        profile_path,
        reproduce_path,
    })
}

fn select_executable(
    candidates: &[WindowsExeCandidate],
    executable: Option<&str>,
) -> Result<Option<WindowsExeCandidate>> {
    let Some(executable) = executable else {
        return Ok(discovery::select_exe_candidate(candidates).cloned());
    };
    let normalized = executable.replace('\\', "/");
    candidates
        .iter()
        .find(|candidate| candidate.relative_path.eq_ignore_ascii_case(&normalized))
        .cloned()
        .map(Some)
        .ok_or_else(|| {
            PortCellarError::Message(format!(
                "local executable was not found below the game directory: {executable}"
            ))
        })
}

fn read_steam_app_id(root: &Path) -> Option<String> {
    let value = fs::read_to_string(root.join("steam_appid.txt")).ok()?;
    let value = value.lines().next()?.trim();
    (!value.is_empty() && value.chars().all(|value| value.is_ascii_digit()))
        .then(|| value.to_string())
}

fn host_path_to_z_path(path: &Path) -> Result<String> {
    let relative = path.strip_prefix(Path::new("/")).map_err(|_| {
        PortCellarError::Message(format!(
            "local game path must be absolute after resolution: {}",
            path.display()
        ))
    })?;
    let mut windows = String::from("Z:");
    for component in relative.components() {
        let Component::Normal(value) = component else {
            continue;
        };
        windows.push('\\');
        windows.push_str(value.to_str().ok_or_else(|| {
            PortCellarError::Message("local game path contains a non-Unicode component".to_string())
        })?);
    }
    Ok(windows)
}

fn apply_pe_evidence(
    profile: &mut GenericGameProfile,
    pe: &PeSummary,
    review_items: &mut Vec<String>,
) {
    for layer in &pe.detected_layers {
        let capability = match layer.as_str() {
            "steamworks" => Some(GameCapability::Steamworks),
            "d3d12" => Some(GameCapability::Direct3d12),
            "d3d11/dxgi" => Some(GameCapability::Direct3d11),
            "d3d9" => Some(GameCapability::Direct3d9),
            "opengl" => Some(GameCapability::OpenGl),
            "vulkan" => Some(GameCapability::Vulkan),
            "media-codec" => Some(GameCapability::VideoPlayback),
            "audio" => Some(GameCapability::Audio),
            "anti-cheat-risk" => Some(GameCapability::AntiCheat),
            _ => None,
        };
        if let Some(capability) = capability {
            *profile = profile.clone().with_capability(capability);
        }
        if layer == "steamworks" {
            review_items.push(
                "Steamworks imports detected; the local runtime will not start Windows Steam, so the game may need a game-specific API workaround"
                    .to_string(),
            );
        }
    }

    let has_import = |needles: &[&str]| {
        pe.import_dlls.iter().any(|dll| {
            needles
                .iter()
                .any(|needle| dll.to_ascii_lowercase().contains(needle))
        })
    };
    if has_import(&["vcruntime", "msvcp"]) {
        *profile = profile
            .clone()
            .with_dependency(RuntimeDependency::Vcrun2022);
        review_items.push(
            "Visual C++ imports detected; verify the matching redistributable before launch"
                .to_string(),
        );
    }
    if has_import(&["d3dcompiler"]) {
        *profile = profile
            .clone()
            .with_dependency(RuntimeDependency::D3dCompiler);
    }
    if has_import(&["openal"]) {
        *profile = profile.clone().with_dependency(RuntimeDependency::OpenAl);
    }
    if has_import(&["xaudio2"]) {
        *profile = profile.clone().with_dependency(RuntimeDependency::XAudio);
    }
    if has_import(&["mfplat", "mfreadwrite", "wmvcore"]) {
        *profile = profile
            .clone()
            .with_dependency(RuntimeDependency::MediaFoundation);
    }
    if has_import(&["quartz", "theora", "bink", "avcodec"]) {
        review_items.push(
            "media imports detected; validate video playback separately from the graphics backend"
                .to_string(),
        );
    }
}

fn local_recommendations(pe: Option<&PeSummary>) -> Vec<String> {
    let mut recommendations = Vec::new();
    let Some(pe) = pe else {
        return recommendations;
    };
    if pe.detected_layers.iter().any(|layer| layer == "d3d11/dxgi") {
        recommendations.push("try DXMT or DXVK after a WineD3D baseline".to_string());
    }
    if pe.detected_layers.iter().any(|layer| layer == "d3d9") {
        recommendations.push("try DXVK or WineD3D for this D3D9-era executable".to_string());
    }
    if pe.detected_layers.iter().any(|layer| layer == "opengl") {
        recommendations.push("check Wine OpenGL/macOS driver behavior first".to_string());
    }
    recommendations
}

fn render_report(analysis: &LocalGameAnalysis) -> String {
    let mut text = String::new();
    text.push_str("# Local Game Analysis\n\n");
    text.push_str("This report is private and may contain local filesystem paths.\n\n");
    text.push_str(&format!("- analysis version: `{}`\n", analysis.version));
    text.push_str(&format!("- name: `{}`\n", analysis.name));
    text.push_str(&format!("- app id: `{}`\n", analysis.app_id));
    text.push_str(&format!("- root: `{}`\n", analysis.root.display()));
    text.push_str(&format!(
        "- Wine path: `{}`\n",
        analysis.windows_install_path
    ));
    text.push_str("\n## Selected executable\n\n");
    if let Some(candidate) = &analysis.selected_exe {
        text.push_str(&format!(
            "- `{}` score={} size={}\n",
            candidate.relative_path, candidate.score, candidate.size
        ));
        if let Some(pe) = &candidate.pe {
            text.push_str(&format!(
                "- machine: `{}`\n",
                pe.machine.as_deref().unwrap_or("unknown")
            ));
            text.push_str(&format!(
                "- subsystem: `{}`\n",
                pe.subsystem.as_deref().unwrap_or("unknown")
            ));
            text.push_str(&format!("- layers: `{}`\n", pe.detected_layers.join(", ")));
            text.push_str(&format!("- imports: `{}`\n", pe.import_dlls.join(", ")));
        }
    } else {
        text.push_str("- none\n");
    }
    text.push_str("\n## Candidates\n\n");
    for candidate in &analysis.exe_candidates {
        text.push_str(&format!(
            "- `{}` score={} reasons={}\n",
            candidate.relative_path,
            candidate.score,
            candidate.reasons.join(", ")
        ));
    }
    text.push_str("\n## Profile review\n\n");
    for item in &analysis.review_items {
        text.push_str(&format!("- {item}\n"));
    }
    text.push_str(&format!(
        "\nBundled DLLs scanned: {}\n",
        analysis.bundled_dlls.len()
    ));
    text
}

fn render_reproduce_script(analysis: &LocalGameAnalysis, artifact_root: &Path) -> String {
    let artifact_name = artifact_root
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("local-game");
    let wait_login = if analysis.profile.steam_integration() == SteamIntegration::Required {
        " --wait-login"
    } else {
        ""
    };
    format!(
        "#!/usr/bin/env bash\nset -euo pipefail\n\nrepo_root=\"$(cd -- \"$(dirname \"$0\")/../../..\" && pwd)\"\ncd \"$repo_root\"\nexec cargo run --quiet -- game launch --from-profile \"$repo_root/.portcellar/analysis/{artifact_name}/profile.toml\" --mode wine-direct{wait_login} --wait\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_analysis_does_not_promote_steam_app_id_to_runtime_integration() {
        let root =
            std::env::temp_dir().join(format!("portcellar-local-analysis-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("steam_appid.txt"), "382900\n").unwrap();
        fs::write(root.join("Game.exe"), b"not a PE").unwrap();

        let analysis = analyze_local_game(&root, Some("Example Game"), None, None).unwrap();

        assert_eq!(analysis.app_id, "local-example-game");
        assert_eq!(analysis.profile.steam_integration(), SteamIntegration::None);
        assert_eq!(
            analysis.profile.steam_cef_policy(),
            SteamCefPolicy::WineDefault
        );
        assert!(analysis
            .review_items
            .iter()
            .any(|item| item.contains("static evidence only")));

        fs::remove_dir_all(root).unwrap();
    }
}
