use super::*;
use crate::*;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn inspect_engines(
    profile: &dyn GameProfile,
    game: Option<&GameInstall>,
    steam: &SteamReport,
    wine_steam: &WineSteamRuntime,
) -> Vec<EngineCandidate> {
    let mut engines = Vec::new();

    engines.push(EngineCandidate {
        name: "Steam URI launcher".to_string(),
        kind: EngineKind::NativeSteam,
        path: PathBuf::from(format!("steam://rungameid/{}", profile.app_id())),
        version: None,
        available: steam.running && game.is_some(),
        detail: Some(if steam.running {
            "Steam is running".to_string()
        } else {
            "Steam is not running".to_string()
        }),
        components: Vec::new(),
        backend_candidates: engine_backend_candidates(EngineKind::NativeSteam),
        backend_matrix: engine_backend_matrix(EngineKind::NativeSteam),
    });

    if let Some(game) = game {
        engines.push(EngineCandidate {
            name: "Native macOS app bundle".to_string(),
            kind: EngineKind::NativeApp,
            path: game
                .app_bundle
                .clone()
                .unwrap_or_else(|| game.install_dir.clone()),
            version: None,
            available: game.app_bundle.is_some(),
            detail: game.executable_kind.clone(),
            components: Vec::new(),
            backend_candidates: engine_backend_candidates(EngineKind::NativeApp),
            backend_matrix: engine_backend_matrix(EngineKind::NativeApp),
        });
    }

    let (wine_components, wine_backend_candidates, wine_backend_matrix) = backend_details(
        EngineKind::Wine,
        wine_steam.wine.as_deref(),
        wine_steam.dxmt_root.as_deref(),
    );
    engines.push(EngineCandidate {
        name: "Windows Steam in Wine prefix".to_string(),
        kind: EngineKind::Wine,
        path: wine_steam
            .wine
            .clone()
            .unwrap_or_else(|| PathBuf::from("<not found>")),
        version: wine_steam.wine_version.clone(),
        available: wine_steam.wine.is_some()
            && wine_steam.steam_exe.is_some()
            && wine_steam.game_executable.is_some(),
        detail: Some(format!(
            "prefix: {}; Steam: {}; {}: {}",
            wine_steam.prefix.display(),
            present_missing(wine_steam.steam_exe.is_some()),
            profile.name(),
            present_missing(wine_steam.game_executable.is_some())
        )),
        components: wine_components,
        backend_candidates: wine_backend_candidates,
        backend_matrix: wine_backend_matrix,
    });

    add_path_engine(
        &mut engines,
        "wine64 in PATH",
        EngineKind::Wine,
        find_in_path("wine64"),
    );
    add_path_engine(
        &mut engines,
        "wine in PATH",
        EngineKind::Wine,
        find_in_path("wine"),
    );
    add_path_engine(
        &mut engines,
        "CrossOver cxrun in PATH",
        EngineKind::CrossOver,
        find_in_path("cxrun"),
    );

    for path in crossover_candidates() {
        let (components, backend_candidates, backend_matrix) = backend_details(
            EngineKind::CrossOver,
            Some(&path),
            wine_steam.dxmt_root.as_deref(),
        );
        engines.push(EngineCandidate {
            name: format!(
                "CrossOver {}",
                path.file_name().unwrap_or_default().to_string_lossy()
            ),
            kind: EngineKind::CrossOver,
            available: path.exists(),
            version: engine_version_for_path(&path),
            path,
            detail: crossover_bottle_detail(),
            components,
            backend_candidates,
            backend_matrix,
        });
    }

    for path in wine_app_candidates() {
        let (components, backend_candidates, backend_matrix) = backend_details(
            EngineKind::Wine,
            Some(&path),
            wine_steam.dxmt_root.as_deref(),
        );
        engines.push(EngineCandidate {
            name: format!("Wine app bundle {}", wine_bundle_name(&path)),
            kind: EngineKind::Wine,
            available: path.exists(),
            version: engine_version_for_path(&path),
            path,
            detail: None,
            components,
            backend_candidates,
            backend_matrix,
        });
    }

    for path in [
        "/opt/homebrew/bin/gameportingtoolkit",
        "/opt/local/bin/gameportingtoolkit",
        "/opt/homebrew/bin/gameportingtoolkit-no-hud",
        "/opt/local/bin/gameportingtoolkit-no-hud",
    ] {
        let path = PathBuf::from(path);
        let (components, backend_candidates, backend_matrix) =
            backend_details(EngineKind::GamePortingToolkit, Some(&path), None);
        engines.push(EngineCandidate {
            name: format!(
                "GPTK wrapper {}",
                path.file_name().unwrap_or_default().to_string_lossy()
            ),
            kind: EngineKind::GamePortingToolkit,
            available: path.exists(),
            version: engine_version_for_path(&path),
            path,
            detail: None,
            components,
            backend_candidates,
            backend_matrix,
        });
    }

    engines
}

pub(crate) fn crossover_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for app_root in app_roots() {
        paths.extend([
            app_root.join("CrossOver.app/Contents/SharedSupport/CrossOver/bin/wine64"),
            app_root.join("CrossOver.app/Contents/SharedSupport/CrossOver/bin/wine"),
            app_root.join("CrossOver.app/Contents/SharedSupport/CrossOver/bin/cxrun"),
        ]);
    }
    paths
}

pub(crate) fn wine_app_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for app_root in wine_release_roots().into_iter().chain(app_roots()) {
        paths.extend([
            app_root.join("Wine-Crossover-23.7.1-1/Contents/Resources/wine/bin/wine64"),
            app_root.join("Wine Crossover.app/Contents/Resources/wine/bin/wine64"),
            app_root.join("Wine Stable.app/Contents/Resources/wine/bin/wine"),
            app_root.join("Wine Staging.app/Contents/Resources/wine/bin/wine"),
            app_root.join("Wine Devel.app/Contents/Resources/wine/bin/wine"),
        ]);
    }

    paths
}

pub(crate) fn wine_release_roots() -> Vec<PathBuf> {
    let mut roots = home_dir()
        .map(|home| wine_release_roots_in(&home))
        .unwrap_or_default();
    roots.extend(project_state_engine_roots());
    roots.sort();
    roots.dedup();
    roots
}

pub(crate) fn wine_release_roots_in(home: &Path) -> Vec<PathBuf> {
    let mut roots = fs::read_dir(home)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_dir()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name.starts_with("wine-"))
                    .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    roots.sort();
    roots
}

pub(crate) fn project_state_engine_roots() -> Vec<PathBuf> {
    wine_release_roots_in(&state_root().join("engines"))
}

pub(crate) fn gptk_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for app_root in app_roots() {
        paths.push(app_root.join("Game Porting Toolkit.app/Contents/Resources/wine/bin/wine64"));
    }

    paths
}

pub(crate) fn wine_bundle_name(path: &Path) -> String {
    path.components()
        .filter_map(|component| component.as_os_str().to_str())
        .find(|component| component.ends_with(".app") || component.starts_with("Wine-Crossover"))
        .unwrap_or("wine")
        .to_string()
}

pub(crate) fn app_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(home) = home_dir() {
        roots.push(home.join("Applications"));
    }
    roots.push(PathBuf::from("/Applications"));
    roots
}

pub(crate) fn add_path_engine(
    engines: &mut Vec<EngineCandidate>,
    name: &str,
    kind: EngineKind,
    path: Option<PathBuf>,
) {
    let (components, backend_candidates, backend_matrix) =
        backend_details(kind, path.as_deref(), None);
    let version = path.as_deref().and_then(engine_version_for_path);
    engines.push(EngineCandidate {
        name: name.to_string(),
        kind,
        available: path.is_some(),
        path: path.unwrap_or_else(|| PathBuf::from("<not found>")),
        version,
        detail: None,
        components,
        backend_candidates,
        backend_matrix,
    });
}

fn backend_details(
    kind: EngineKind,
    path: Option<&Path>,
    dxmt_root: Option<&Path>,
) -> (
    Vec<RuntimeComponent>,
    Vec<GraphicsBackend>,
    Vec<EngineBackendCompatibility>,
) {
    let components = path
        .map(|path| runtime_components_for_engine(path, kind, dxmt_root))
        .unwrap_or_default();
    let backend_matrix = engine_backend_matrix_for_path(kind, path, dxmt_root);
    let backend_candidates = backend_matrix
        .iter()
        .filter(|entry| entry.status != CompatibilityStatus::Unsupported)
        .map(|entry| entry.backend)
        .collect();
    (components, backend_candidates, backend_matrix)
}

pub(crate) fn engine_version_for_path(path: &Path) -> Option<String> {
    if !path.exists() {
        return None;
    }
    command_stdout_path(path, &[OsString::from("--version")])
        .filter(|version| !version.trim().is_empty())
}

pub(crate) fn crossover_bottle_detail() -> Option<String> {
    let bottles = home_dir()?.join("Library/Application Support/CrossOver/Bottles");
    if bottles.exists() {
        Some(format!("bottle directory exists at {}", bottles.display()))
    } else {
        None
    }
}
