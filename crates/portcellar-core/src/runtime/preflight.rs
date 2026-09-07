use super::*;

pub fn game_runtime_preflight(
    profile: &dyn GameProfile,
    runtime: &WineSteamRuntime,
) -> RuntimePreflight {
    let policy = profile.runtime_policy();
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();
    let dependency_plans = wine_dependency_plans_for(profile, runtime);
    let engine = runtime.wine.as_deref().map(wine_engine_kind_for_path);
    let backend_compatibility = engine.and_then(|kind| {
        engine_backend_matrix_for_path(kind, runtime.wine.as_deref(), runtime.dxmt_root.as_deref())
            .into_iter()
            .find(|entry| entry.backend == policy.graphics_backend)
    });

    if runtime.wine.is_none() {
        blockers.push("Wine engine missing".to_string());
    }
    if let Some(compatibility) = backend_compatibility {
        match compatibility.status {
            CompatibilityStatus::Verified => {}
            CompatibilityStatus::Candidate => warnings.push(format!(
                "{:?} backend is a candidate for {:?}: {}",
                compatibility.backend, compatibility.engine, compatibility.rationale
            )),
            CompatibilityStatus::ExternalDependency => warnings.push(format!(
                "{:?} backend for {:?} requires external dependencies: {}",
                compatibility.backend, compatibility.engine, compatibility.rationale
            )),
            CompatibilityStatus::Unsupported => blockers.push(format!(
                "{:?} backend is unsupported by {:?}: {}",
                compatibility.backend, compatibility.engine, compatibility.rationale
            )),
        }
    }
    if runtime.game_executable.is_none() {
        blockers.push(format!("Windows {} executable missing", profile.name()));
    }
    if policy.steam_integration == SteamIntegration::Required && runtime.steam_exe.is_none() {
        blockers.push("Windows Steam missing".to_string());
    }
    if runtime
        .readiness_issues
        .iter()
        .any(|issue| issue.starts_with("orphaned Steam WebHelper process tree"))
    {
        blockers.push(
            "Wine Steam has an orphaned WebHelper process tree; run `portcellar isaac steam-stop` before launching"
                .to_string(),
        );
    }
    if policy.graphics_backend == GraphicsBackend::Dxmt && runtime.dxmt_root.is_none() {
        warnings.push(
            "DXMT backend selected without a discovered DXMT root; verify WINEDLLPATH or engine packaging"
                .to_string(),
        );
    }
    if policy.graphics_backend == GraphicsBackend::Scgl {
        if policy.steam_integration != SteamIntegration::None {
            blockers.push(
                "SCGL artifact staging currently requires a local non-Steam game profile"
                    .to_string(),
            );
        }
        if policy.runtime_artifacts.is_empty() {
            blockers.push("SCGL backend requires at least one runtime artifact".to_string());
        }
        if profile
            .launch_arguments()
            .iter()
            .any(|argument| argument.eq_ignore_ascii_case("-d:software"))
        {
            blockers.push("SCGL backend cannot be combined with -d:software".to_string());
        }
    }
    if policy.capabilities.contains(&GameCapability::AntiCheat) {
        blockers.push(
            "anti-cheat capability requires a compatibility path that this runtime does not provide"
                .to_string(),
        );
    }
    if policy.capabilities.contains(&GameCapability::Direct3d12)
        && !matches!(
            policy.graphics_backend,
            GraphicsBackend::D3dMetal | GraphicsBackend::Vkd3d
        )
    {
        blockers.push("Direct3D 12 requires an explicit D3DMetal or VKD3D backend".to_string());
    }
    if policy.capabilities.contains(&GameCapability::OpenGl)
        && policy.graphics_backend == GraphicsBackend::Dxmt
    {
        warnings.push(
            "OpenGL capability is declared while DXMT is selected; verify the actual render path"
                .to_string(),
        );
    }
    for plan in &dependency_plans {
        let dependency = format!("{:?}", plan.dependency);
        match plan.status {
            RuntimeDependencyStatus::Planned => warnings.push(format!(
                "{dependency} has an explicit winetricks plan; it is not applied automatically"
            )),
            RuntimeDependencyStatus::Manual => warnings.push(format!(
                "{dependency} requires a game-specific installer or verified bottle recipe"
            )),
            RuntimeDependencyStatus::ToolMissing => warnings.push(format!(
                "{dependency} has a known winetricks verb, but winetricks is unavailable"
            )),
        }
    }

    RuntimePreflight {
        game: profile.name().to_string(),
        engine,
        backend: policy.graphics_backend,
        backend_compatibility,
        capabilities: policy.capabilities.into_iter().collect(),
        dependencies: policy.dependencies,
        dependency_plans,
        blockers,
        warnings,
    }
}

pub fn inspect_game_runtime_preflight(profile: &dyn GameProfile) -> RuntimePreflight {
    let runtime = inspect_wine_steam_runtime_for(profile);
    game_runtime_preflight(profile, &runtime)
}
