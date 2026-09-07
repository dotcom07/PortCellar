use super::*;
use std::path::Path;

pub fn engine_backend_matrix(kind: EngineKind) -> Vec<EngineBackendCompatibility> {
    [
        GraphicsBackend::Auto,
        GraphicsBackend::WineD3D,
        GraphicsBackend::Dxmt,
        GraphicsBackend::Dxvk,
        GraphicsBackend::D3dMetal,
        GraphicsBackend::Vkd3d,
        GraphicsBackend::Scgl,
    ]
    .into_iter()
    .map(|backend| engine_backend_compatibility(kind, backend))
    .collect()
}

pub(crate) fn engine_backend_matrix_for_path(
    kind: EngineKind,
    path: Option<&Path>,
    dxmt_root: Option<&Path>,
) -> Vec<EngineBackendCompatibility> {
    let components = path
        .map(|path| super::runtime_components_for_engine(path, kind, dxmt_root))
        .unwrap_or_default();
    engine_backend_matrix(kind)
        .into_iter()
        .map(|entry| {
            let discovered = match entry.backend {
                GraphicsBackend::Dxmt => components.contains(&RuntimeComponent::Dxmt),
                GraphicsBackend::Dxvk => components.contains(&RuntimeComponent::Dxvk),
                GraphicsBackend::D3dMetal => components.contains(&RuntimeComponent::D3dMetal),
                GraphicsBackend::Vkd3d => components.contains(&RuntimeComponent::Vkd3d),
                _ => false,
            };
            if discovered {
                EngineBackendCompatibility {
                    status: CompatibilityStatus::Candidate,
                    rationale: "backend component discovered in the selected engine/runtime paths",
                    ..entry
                }
            } else {
                entry
            }
        })
        .collect()
}

pub(crate) fn engine_backend_compatibility(
    kind: EngineKind,
    backend: GraphicsBackend,
) -> EngineBackendCompatibility {
    let (status, rationale) = match (kind, backend) {
        (EngineKind::NativeSteam | EngineKind::NativeApp, GraphicsBackend::Auto) => {
            (CompatibilityStatus::Verified, "native macOS launch path")
        }
        (
            EngineKind::Wine | EngineKind::CrossOver | EngineKind::GamePortingToolkit,
            GraphicsBackend::Auto,
        ) => (
            CompatibilityStatus::Candidate,
            "engine default backend; verify per game profile",
        ),
        (EngineKind::CrossOver, GraphicsBackend::WineD3D) => (
            CompatibilityStatus::Candidate,
            "CrossOver-provided WineD3D path; verify per bottle",
        ),
        (EngineKind::CrossOver, GraphicsBackend::D3dMetal) => (
            CompatibilityStatus::Candidate,
            "CrossOver D3DMetal path; verify engine build and game profile",
        ),
        (EngineKind::CrossOver, GraphicsBackend::Dxvk | GraphicsBackend::Vkd3d) => (
            CompatibilityStatus::ExternalDependency,
            "requires a packaged or bottle-local translation layer",
        ),
        (EngineKind::CrossOver, GraphicsBackend::Dxmt) => (
            CompatibilityStatus::ExternalDependency,
            "requires an external DXMT build and DLL search path",
        ),
        (EngineKind::Wine, GraphicsBackend::WineD3D) => (
            CompatibilityStatus::Candidate,
            "Wine-provided fallback path; verify driver behavior",
        ),
        (EngineKind::Wine, GraphicsBackend::Dxmt) => (
            CompatibilityStatus::ExternalDependency,
            "requires an external DXMT build and DLL search path",
        ),
        (EngineKind::Wine, GraphicsBackend::Dxvk | GraphicsBackend::Vkd3d) => (
            CompatibilityStatus::ExternalDependency,
            "requires a bottle-local translation layer",
        ),
        (EngineKind::Wine, GraphicsBackend::Scgl) => (
            CompatibilityStatus::ExternalDependency,
            "requires a profile-pinned 32-bit SCGL artifact staged beside the game",
        ),
        (EngineKind::CrossOver, GraphicsBackend::Scgl) => (
            CompatibilityStatus::ExternalDependency,
            "requires a profile-pinned 32-bit SCGL artifact staged beside the game",
        ),
        (EngineKind::GamePortingToolkit, GraphicsBackend::D3dMetal) => (
            CompatibilityStatus::Candidate,
            "GPTK D3DMetal path; verify toolkit version",
        ),
        (EngineKind::GamePortingToolkit, GraphicsBackend::Vkd3d) => (
            CompatibilityStatus::Candidate,
            "GPTK VKD3D path; verify toolkit version",
        ),
        _ => (
            CompatibilityStatus::Unsupported,
            "not part of the engine's supported baseline",
        ),
    };

    EngineBackendCompatibility {
        engine: kind,
        backend,
        status,
        rationale,
    }
}

pub(crate) fn wine_engine_kind_for_path(path: &Path) -> EngineKind {
    let components = path
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>();
    if components
        .iter()
        .any(|component| matches!(component.as_str(), "crossover.app" | "cxrun"))
    {
        EngineKind::CrossOver
    } else if components.iter().any(|component| {
        matches!(
            component.as_str(),
            "gameportingtoolkit" | "gameportingtoolkit-no-hud" | "game porting toolkit.app"
        )
    }) {
        EngineKind::GamePortingToolkit
    } else {
        EngineKind::Wine
    }
}

pub(crate) fn engine_backend_candidates(kind: EngineKind) -> Vec<GraphicsBackend> {
    engine_backend_matrix(kind)
        .into_iter()
        .filter(|entry| entry.status != CompatibilityStatus::Unsupported)
        .map(|entry| entry.backend)
        .collect()
}
