use super::*;

#[test]
fn pe_summary_detects_machine_subsystem_and_layers() {
    let bytes = synthetic_pe_with_imports(&["steam_api64.dll", "d3d11.dll", "mfplat.dll"]);
    let summary = summarize_pe_bytes(&bytes).unwrap();

    assert_eq!(summary.machine.as_deref(), Some("x86_64"));
    assert_eq!(summary.subsystem.as_deref(), Some("windows-gui"));
    assert_eq!(
        summary.import_dlls,
        vec!["d3d11.dll", "mfplat.dll", "steam_api64.dll"]
    );
    assert!(summary.detected_layers.contains(&"d3d11/dxgi".to_string()));
    assert!(summary.detected_layers.contains(&"steamworks".to_string()));
    assert!(summary.detected_layers.contains(&"media-codec".to_string()));
}

#[test]
fn slugify_keeps_analysis_paths_stable() {
    assert_eq!(
        slugify("The Binding of Isaac: Rebirth"),
        "the-binding-of-isaac-rebirth"
    );
    assert_eq!(slugify("  Weird___Game!! 2026  "), "weird-game-2026");
}

#[test]
fn steam_client_log_signal_detects_porting_relevant_states() {
    assert_eq!(
        steam_log_signal_from_text(
            "Restart webhelper process\nRestart webhelper process\nRestart webhelper process"
        ),
        Some("webhelper restart loop".to_string())
    );
    assert_eq!(
        steam_log_signal_from_text("WebUITransportStore: Connection status: connected"),
        Some("transport connected".to_string())
    );
    assert_eq!(
        steam_log_signal_from_text("Login: Failed to poll auth session"),
        Some("login attention needed".to_string())
    );
}

#[test]
fn analysis_builds_a_reviewable_generic_profile_candidate() {
    let analysis = GamePortingAnalysis {
        version: PORTING_ANALYSIS_VERSION,
        app_id: "123456".to_string(),
        slug: "example-game".to_string(),
        manifest: Some(SteamAppManifest {
            source: "wine-prefix".to_string(),
            app_id: "123456".to_string(),
            name: "Example Game".to_string(),
            build_id: Some("42".to_string()),
            install_dir_name: "Example Game".to_string(),
            manifest_path: PathBuf::from("/tmp/manifest.acf"),
            install_dir: PathBuf::from("/tmp/Example Game"),
        }),
        exe_candidates: Vec::new(),
        selected_exe: Some(WindowsExeCandidate {
            path: PathBuf::from("/tmp/Example Game/Game.exe"),
            relative_path: "Game.exe".to_string(),
            file_name: "Game.exe".to_string(),
            size: 123,
            pe: Some(PeSummary {
                machine: Some("x86_64".to_string()),
                subsystem: Some("windows-gui".to_string()),
                import_dlls: vec![
                    "d3d11.dll".to_string(),
                    "mfplat.dll".to_string(),
                    "vcruntime140.dll".to_string(),
                ],
                detected_layers: vec!["d3d11/dxgi".to_string(), "media-codec".to_string()],
            }),
            score: 28,
            reasons: vec!["valid PE".to_string()],
        }),
        bundled_dlls: Vec::new(),
        runtime: RuntimeSnapshot {
            wine: Some(PathBuf::from("/usr/local/bin/wine")),
            wine_version: Some("Wine 11".to_string()),
            windows_version: Some("win10".to_string()),
            prefix: PathBuf::from("/tmp/prefix"),
            steam_exe: None,
            dxmt_root: None,
            steam_running: false,
            logged_in: None,
            connection_status: None,
            cef_status: None,
            readiness_issues: Vec::new(),
        },
        latest_crash: None,
        recommendations: Vec::new(),
        blockers: Vec::new(),
    };

    let candidate = analysis.to_generic_profile_candidate().unwrap();
    assert_eq!(candidate.analysis_version, PORTING_ANALYSIS_VERSION);
    assert_eq!(candidate.profile.app_id(), "123456");
    assert_eq!(candidate.profile.bottle_name(), Some("game-123456"));
    assert_eq!(candidate.profile.windows_exe(), "Game.exe");
    assert!(candidate.profile.wine_engine_path_hint().is_none());
    assert_eq!(
        candidate.profile.steam_cef_policy(),
        SteamCefPolicy::CrossOverCompatible
    );
    assert!(candidate
        .profile
        .capabilities()
        .contains(&GameCapability::Direct3d11));
    assert!(candidate
        .profile
        .dependencies()
        .contains(&RuntimeDependency::MediaFoundation));
    assert!(candidate
        .profile
        .dependencies()
        .contains(&RuntimeDependency::Vcrun2022));
    assert!(candidate
        .review_items
        .iter()
        .any(|item| item.contains("both x86 and x64 redistributable")));
    assert!(candidate
        .review_items
        .iter()
        .any(|item| item.contains("graphics backend remains Auto")));
    assert!(candidate
        .review_items
        .iter()
        .any(|item| item.contains("CrossOver-compatible CEF policy")));
    assert!(candidate
        .review_items
        .iter()
        .any(|item| item.contains("observed Wine engine is evidence only")));
}

#[test]
fn bundled_media_imports_do_not_become_media_foundation_dependencies() {
    let analysis = GamePortingAnalysis {
        version: PORTING_ANALYSIS_VERSION,
        app_id: "123456".to_string(),
        slug: "example-game".to_string(),
        manifest: Some(SteamAppManifest {
            source: "wine-prefix".to_string(),
            app_id: "123456".to_string(),
            name: "Example Game".to_string(),
            build_id: Some("42".to_string()),
            install_dir_name: "Example Game".to_string(),
            manifest_path: PathBuf::from("/tmp/manifest.acf"),
            install_dir: PathBuf::from("/tmp/Example Game"),
        }),
        exe_candidates: Vec::new(),
        selected_exe: Some(WindowsExeCandidate {
            path: PathBuf::from("/tmp/Example Game/Game.exe"),
            relative_path: "Game.exe".to_string(),
            file_name: "Game.exe".to_string(),
            size: 123,
            pe: Some(PeSummary {
                machine: Some("x86_64".to_string()),
                subsystem: Some("windows-gui".to_string()),
                import_dlls: vec!["theora.dll".to_string(), "openal32.dll".to_string()],
                detected_layers: vec!["audio".to_string(), "media-codec".to_string()],
            }),
            score: 28,
            reasons: vec!["valid PE".to_string()],
        }),
        bundled_dlls: Vec::new(),
        runtime: RuntimeSnapshot {
            wine: Some(PathBuf::from("/usr/local/bin/wine")),
            wine_version: Some("Wine 11".to_string()),
            windows_version: Some("win10".to_string()),
            prefix: PathBuf::from("/tmp/prefix"),
            steam_exe: None,
            dxmt_root: None,
            steam_running: false,
            logged_in: None,
            connection_status: None,
            cef_status: None,
            readiness_issues: Vec::new(),
        },
        latest_crash: None,
        recommendations: Vec::new(),
        blockers: Vec::new(),
    };

    let candidate = analysis.to_generic_profile_candidate().unwrap();
    assert!(!candidate
        .profile
        .dependencies()
        .contains(&RuntimeDependency::MediaFoundation));
    assert!(candidate
        .profile
        .capabilities()
        .contains(&GameCapability::VideoPlayback));
    assert!(candidate
        .review_items
        .iter()
        .any(|item| item.contains("Non-MediaFoundation media imports")));
}

fn synthetic_pe_with_imports(imports: &[&str]) -> Vec<u8> {
    let mut bytes = vec![0u8; 0x500];
    let pe_offset = 0x80usize;
    let optional_offset = pe_offset + 24;
    let section_offset = optional_offset + 0xf0;
    let section_raw = 0x200usize;
    let section_rva = 0x1000u32;
    let import_descriptor_rva = section_rva;
    let mut name_rva = section_rva + 0x80;

    bytes[0..2].copy_from_slice(b"MZ");
    write_u32(&mut bytes, 0x3c, pe_offset as u32);
    bytes[pe_offset..pe_offset + 4].copy_from_slice(b"PE\0\0");
    write_u16(&mut bytes, pe_offset + 4, 0x8664);
    write_u16(&mut bytes, pe_offset + 6, 1);
    write_u16(&mut bytes, pe_offset + 20, 0xf0);

    write_u16(&mut bytes, optional_offset, 0x20b);
    write_u16(&mut bytes, optional_offset + 68, 2);
    write_u32(&mut bytes, optional_offset + 112 + 8, import_descriptor_rva);
    write_u32(&mut bytes, optional_offset + 112 + 12, 0x100);

    bytes[section_offset..section_offset + 8].copy_from_slice(b".rdata\0\0");
    write_u32(&mut bytes, section_offset + 8, 0x300);
    write_u32(&mut bytes, section_offset + 12, section_rva);
    write_u32(&mut bytes, section_offset + 16, 0x300);
    write_u32(&mut bytes, section_offset + 20, section_raw as u32);

    for (index, import) in imports.iter().enumerate() {
        let descriptor_offset = section_raw + index * 20;
        write_u32(&mut bytes, descriptor_offset + 12, name_rva);

        let name_offset = section_raw + (name_rva - section_rva) as usize;
        bytes[name_offset..name_offset + import.len()].copy_from_slice(import.as_bytes());
        name_rva += import.len() as u32 + 1;
    }

    bytes
}

fn write_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
