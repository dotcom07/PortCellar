use super::*;

#[test]
fn wine_matrix_keeps_builtin_and_external_backends_distinct() {
    let matrix = engine_backend_matrix(EngineKind::Wine);

    let wined3d = matrix
        .iter()
        .find(|entry| entry.backend == GraphicsBackend::WineD3D)
        .unwrap();
    let dxmt = matrix
        .iter()
        .find(|entry| entry.backend == GraphicsBackend::Dxmt)
        .unwrap();
    let d3dmetal = matrix
        .iter()
        .find(|entry| entry.backend == GraphicsBackend::D3dMetal)
        .unwrap();

    assert_eq!(wined3d.status, CompatibilityStatus::Candidate);
    assert_eq!(dxmt.status, CompatibilityStatus::ExternalDependency);
    assert_eq!(d3dmetal.status, CompatibilityStatus::Unsupported);
}

#[test]
fn windows_engine_auto_backend_is_a_candidate_default() {
    let entry = engine_backend_compatibility(EngineKind::Wine, GraphicsBackend::Auto);

    assert_eq!(entry.status, CompatibilityStatus::Candidate);
}

#[test]
fn wine_engine_rejects_d3dmetal_in_preflight_matrix() {
    let entry = engine_backend_compatibility(EngineKind::Wine, GraphicsBackend::D3dMetal);

    assert_eq!(entry.status, CompatibilityStatus::Unsupported);
}

#[test]
fn engine_kind_detection_ignores_project_directory_name() {
    assert_eq!(
        wine_engine_kind_for_path(Path::new(
            "/tmp/synthetic-project/.portcellar/engines/wine-11.10/Wine Staging.app/Contents/Resources/wine/bin/wine",
        )),
        EngineKind::Wine
    );
    assert_eq!(
        wine_engine_kind_for_path(Path::new(
            "/Applications/CrossOver.app/Contents/SharedSupport/CrossOver/bin/wine",
        )),
        EngineKind::CrossOver
    );
}

#[test]
fn native_engine_only_advertises_native_auto_backend() {
    let matrix = engine_backend_matrix(EngineKind::NativeApp);
    let candidates = matrix
        .iter()
        .filter(|entry| entry.status != CompatibilityStatus::Unsupported)
        .map(|entry| entry.backend)
        .collect::<Vec<_>>();

    assert_eq!(candidates, vec![GraphicsBackend::Auto]);
}

#[test]
fn runtime_component_probe_promotes_only_discovered_translation_layers() {
    let root = std::env::temp_dir().join(format!("portcellar-components-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let wine_lib = root.join("lib/wine");
    fs::create_dir_all(wine_lib.join("x86_64-windows")).unwrap();
    fs::create_dir_all(wine_lib.join("x86_64-unix")).unwrap();
    fs::write(wine_lib.join("x86_64-windows/wined3d.dll"), b"wined3d").unwrap();
    fs::write(wine_lib.join("x86_64-windows/libvkd3d-1.dll"), b"vkd3d").unwrap();
    fs::write(
        wine_lib.join("x86_64-windows/winegstreamer.dll"),
        b"gstreamer",
    )
    .unwrap();
    let mono_faudio = root.join("share/wine/mono/wine-mono/lib/x86_64");
    fs::create_dir_all(&mono_faudio).unwrap();
    fs::write(mono_faudio.join("FAudio.dll"), b"faudio").unwrap();
    let dxmt = root.join("dxmt");
    fs::create_dir_all(dxmt.join("i386-windows")).unwrap();
    fs::create_dir_all(dxmt.join("x86_64-windows")).unwrap();
    fs::create_dir_all(dxmt.join("x86_64-unix")).unwrap();
    for path in [
        dxmt.join("i386-windows/d3d11.dll"),
        dxmt.join("x86_64-windows/d3d11.dll"),
        dxmt.join("x86_64-unix/winemetal.so"),
    ] {
        fs::write(path, b"dxmt").unwrap();
    }

    let engine = root.join("bin/wine");
    fs::create_dir_all(engine.parent().unwrap()).unwrap();
    fs::write(&engine, b"wine").unwrap();
    let components = runtime_components_for_engine(&engine, EngineKind::Wine, Some(&dxmt));
    assert!(components.contains(&RuntimeComponent::WineD3d));
    assert!(components.contains(&RuntimeComponent::Vkd3d));
    assert!(components.contains(&RuntimeComponent::WineGstreamer));
    assert!(components.contains(&RuntimeComponent::FAudio));
    assert!(components.contains(&RuntimeComponent::Dxmt));
    assert!(!components.contains(&RuntimeComponent::MoltenVk));

    let vkd3d = engine_backend_matrix_for_path(EngineKind::Wine, Some(&engine), Some(&dxmt))
        .into_iter()
        .find(|entry| entry.backend == GraphicsBackend::Vkd3d)
        .unwrap();
    assert_eq!(vkd3d.status, CompatibilityStatus::Candidate);

    fs::remove_dir_all(root).unwrap();
}
