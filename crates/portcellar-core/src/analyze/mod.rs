mod discovery;
mod local;
mod pe;
mod render;
mod steam_client;
mod steam_client_render;

#[cfg(test)]
pub(crate) use discovery::slugify;
pub use local::{
    analyze_local_game, write_local_game_analysis_artifacts, LocalGameAnalysis,
    LocalGameAnalysisArtifacts, LOCAL_GAME_ANALYSIS_VERSION,
};
#[cfg(test)]
pub(crate) use pe::summarize_pe_bytes;
pub use pe::PeSummary;
pub use render::write_porting_analysis_artifacts;
#[cfg(test)]
pub(crate) use steam_client::steam_log_signal_from_text;
pub use steam_client::{
    analyze_steam_client, collect_steam_client_analysis, SteamCefTargetAnalysis,
    SteamClientAnalysis, SteamClientAnalysisArtifacts, SteamClientAnalysisOptions,
    SteamClientBinary, SteamClientProbe, SteamClientProbeTick, SteamLogAnalysis,
    SteamPackageAnalysis,
};
pub use steam_client_render::write_steam_client_analysis_artifacts;

use crate::*;
use std::path::PathBuf;

pub const PORTING_ANALYSIS_VERSION: &str = "porting-analysis-v1";

#[derive(Debug, Clone)]
pub struct PortingAnalysisArtifacts {
    pub root: PathBuf,
    pub report_path: PathBuf,
    pub profile_draft_path: PathBuf,
    pub profile_path: Option<PathBuf>,
    pub reproduce_path: PathBuf,
    pub raw_doctor_path: PathBuf,
    pub experiment_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct GamePortingAnalysis {
    pub version: &'static str,
    pub app_id: String,
    pub slug: String,
    pub manifest: Option<SteamAppManifest>,
    pub exe_candidates: Vec<WindowsExeCandidate>,
    pub selected_exe: Option<WindowsExeCandidate>,
    pub bundled_dlls: Vec<String>,
    pub runtime: RuntimeSnapshot,
    pub latest_crash: Option<CrashSummary>,
    pub recommendations: Vec<String>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct GameProfileCandidate {
    pub analysis_version: &'static str,
    pub profile: GenericGameProfile,
    pub review_items: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SteamAppManifest {
    pub source: String,
    pub app_id: String,
    pub name: String,
    pub build_id: Option<String>,
    pub install_dir_name: String,
    pub manifest_path: PathBuf,
    pub install_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct WindowsExeCandidate {
    pub path: PathBuf,
    pub relative_path: String,
    pub file_name: String,
    pub size: u64,
    pub pe: Option<PeSummary>,
    pub score: i32,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RuntimeSnapshot {
    pub wine: Option<PathBuf>,
    pub wine_version: Option<String>,
    pub windows_version: Option<String>,
    pub prefix: PathBuf,
    pub steam_exe: Option<PathBuf>,
    pub dxmt_root: Option<PathBuf>,
    pub steam_running: bool,
    pub logged_in: Option<bool>,
    pub connection_status: Option<String>,
    pub cef_status: Option<String>,
    pub readiness_issues: Vec<String>,
}

pub fn collect_porting_analysis(app_id: &str) -> Result<PortingAnalysisArtifacts> {
    let analysis = analyze_game(app_id)?;
    write_porting_analysis_artifacts(&analysis)
}

pub fn analyze_game(app_id: &str) -> Result<GamePortingAnalysis> {
    discovery::validate_app_id(app_id)?;
    let analysis_profile = GenericGameProfile::new(
        app_id.to_string(),
        format!("Steam app {app_id}"),
        "__portcellar_analysis_placeholder__.exe",
    )
    .with_steam_cef_policy(SteamCefPolicy::CrossOverCompatible);
    let report = if app_id == ISAAC_APP_ID {
        doctor_report()
    } else {
        doctor_report_for(&analysis_profile, None)
    };
    let manifest = discovery::find_app_manifest(app_id, &report);
    let exe_candidates = manifest
        .as_ref()
        .map(|manifest| discovery::windows_exe_candidates(&manifest.install_dir))
        .transpose()?
        .unwrap_or_default();
    let selected_exe = discovery::select_exe_candidate(&exe_candidates).cloned();
    let bundled_dlls = manifest
        .as_ref()
        .map(|manifest| discovery::bundled_dlls(&manifest.install_dir))
        .transpose()?
        .unwrap_or_default();
    let latest_crash = report.latest_crash.clone();
    let runtime_report = match (manifest.as_ref(), selected_exe.as_ref()) {
        (Some(manifest), Some(selected)) => {
            let resolved_profile = GenericGameProfile::new(
                app_id.to_string(),
                manifest.name.clone(),
                selected.relative_path.clone(),
            )
            .with_install_dir_hint(manifest.install_dir_name.clone())
            .with_bottle_name(format!("game-{app_id}"))
            .with_steam_cef_policy(SteamCefPolicy::CrossOverCompatible);
            doctor_report_for(&resolved_profile, latest_crash.clone())
        }
        _ => report.clone(),
    };
    let runtime = RuntimeSnapshot::from(&runtime_report.wine_steam);
    let slug = manifest
        .as_ref()
        .map(|manifest| discovery::slugify(&manifest.name))
        .filter(|slug| !slug.is_empty())
        .unwrap_or_else(|| "unknown-game".to_string());
    let (recommendations, blockers) = discovery::recommendations_and_blockers(
        &manifest,
        &exe_candidates,
        selected_exe.as_ref(),
        &runtime,
    );

    Ok(GamePortingAnalysis {
        version: PORTING_ANALYSIS_VERSION,
        app_id: app_id.to_string(),
        slug,
        manifest,
        exe_candidates,
        selected_exe,
        bundled_dlls,
        runtime,
        latest_crash,
        recommendations,
        blockers,
    })
}

impl GamePortingAnalysis {
    pub fn to_generic_profile_candidate(&self) -> Result<GameProfileCandidate> {
        let manifest = self.manifest.as_ref().ok_or_else(|| {
            PortCellarError::Message(
                "cannot build a profile without a Steam app manifest".to_string(),
            )
        })?;
        let selected = self.selected_exe.as_ref().ok_or_else(|| {
            PortCellarError::Message(
                "cannot build a profile without a selected Windows executable".to_string(),
            )
        })?;

        let mut profile = GenericGameProfile::new(
            self.app_id.clone(),
            manifest.name.clone(),
            selected.relative_path.clone(),
        )
        .with_install_dir_hint(manifest.install_dir_name.clone())
        .with_bottle_name(format!("game-{}", self.app_id))
        .with_runtime_profile_version(format!("{}-{}", self.version, self.app_id))
        .with_steam_cef_policy(SteamCefPolicy::CrossOverCompatible);

        if let Some(windows_version) = &self.runtime.windows_version {
            profile = profile.with_wine_windows_version(windows_version.clone());
        }
        let mut review_items = self
            .blockers
            .iter()
            .map(|blocker| format!("analysis blocker: {blocker}"))
            .collect::<Vec<_>>();
        if self.runtime.wine.is_some() {
            review_items.push(
                "observed Wine engine is evidence only; pin wine_engine_path manually when reproducibility requires it"
                    .to_string(),
            );
        }
        review_items.push(
            "graphics backend remains Auto until a backend matrix entry is validated by smoke testing"
                .to_string(),
        );
        review_items.push(
            "Steam UI uses the observed CrossOver-compatible CEF policy; game graphics remain independent"
                .to_string(),
        );

        let Some(pe) = selected.pe.as_ref() else {
            review_items.push("selected executable has no PE summary".to_string());
            return Ok(GameProfileCandidate {
                analysis_version: self.version,
                profile,
                review_items,
            });
        };

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
                profile = profile.with_capability(capability);
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
            profile = profile.with_dependency(RuntimeDependency::Vcrun2022);
            review_items.push(
                "Visual C++ imports detected; verify both x86 and x64 redistributable components before applying the vcrun2022 recipe".to_string(),
            );
        }
        if has_import(&["d3dcompiler"]) {
            profile = profile.with_dependency(RuntimeDependency::D3dCompiler);
        }
        if has_import(&["openal"]) {
            profile = profile.with_dependency(RuntimeDependency::OpenAl);
        }
        if has_import(&["xaudio2"]) {
            profile = profile.with_dependency(RuntimeDependency::XAudio);
        }
        if has_import(&["mfplat", "mfreadwrite", "wmvcore"]) {
            profile = profile.with_dependency(RuntimeDependency::MediaFoundation);
        }
        if has_import(&["quartz", "theora", "bink", "avcodec"]) {
            review_items.push(
                "Non-MediaFoundation media imports detected; verify bundled codec/DirectShow behavior during video smoke instead of inferring a Media Foundation dependency".to_string(),
            );
        }

        if !pe.detected_layers.iter().any(|layer| layer == "steamworks") {
            review_items.push(
                "Steam integration remains Required because the analysis came from a Steam manifest; verify the runtime path"
                    .to_string(),
            );
        }

        Ok(GameProfileCandidate {
            analysis_version: self.version,
            profile,
            review_items,
        })
    }
}

impl From<&WineSteamRuntime> for RuntimeSnapshot {
    fn from(runtime: &WineSteamRuntime) -> Self {
        Self {
            wine: runtime.wine.clone(),
            wine_version: runtime.wine_version.clone(),
            windows_version: runtime.windows_version.clone(),
            prefix: runtime.prefix.clone(),
            steam_exe: runtime.steam_exe.clone(),
            dxmt_root: runtime.dxmt_root.clone(),
            steam_running: runtime.running,
            logged_in: runtime.logged_in,
            connection_status: runtime.connection_status.clone(),
            cef_status: runtime.cef_status.clone(),
            readiness_issues: runtime.readiness_issues.clone(),
        }
    }
}
