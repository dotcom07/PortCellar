use super::*;

#[test]
fn runtime_anchors_expose_current_isaac_contract() {
    let anchors = runtime_anchors();

    assert_eq!(anchors.package_version, PORTCELLAR_VERSION);
    assert_eq!(anchors.isaac_app_id, ISAAC_APP_ID);
    assert_eq!(anchors.isaac_name, ISAAC_APP_NAME);
    assert_eq!(anchors.isaac_windows_depot_id, ISAAC_WINDOWS_DEPOT_ID);
    assert_eq!(anchors.isaac_windows_exe, ISAAC_WINDOWS_EXE);
    assert_eq!(
        anchors.isaac_runtime_profile_version,
        ISAAC_RUNTIME_PROFILE_VERSION
    );
    assert_eq!(anchors.wine_windows_version, ISAAC_WINE_WINDOWS_VERSION);
    assert_eq!(anchors.default_dxmt_config, ISAAC_DXMT_CONFIG);
    assert_eq!(anchors.steam_cloud_env, ISAAC_STEAM_CLOUD_ENV);
    assert_eq!(anchors.local_wine_baseline, LOCAL_WINE_BASELINE);
    assert_eq!(anchors.crossover_foss_baseline, CROSSOVER_FOSS_BASELINE);
}

#[test]
fn isaac_profile_exposes_current_contract() {
    let profile = isaac_profile();

    assert_eq!(profile.app_id(), ISAAC_APP_ID);
    assert_eq!(profile.name(), ISAAC_APP_NAME);
    assert_eq!(profile.windows_depot_id(), Some(ISAAC_WINDOWS_DEPOT_ID));
    assert_eq!(profile.windows_exe(), ISAAC_WINDOWS_EXE);
    assert_eq!(profile.install_dir_hint(), "The Binding of Isaac Rebirth");
    assert_eq!(
        profile.runtime_profile_version(),
        ISAAC_RUNTIME_PROFILE_VERSION
    );
    assert_eq!(profile.wine_windows_version(), ISAAC_WINE_WINDOWS_VERSION);
    assert_eq!(profile.dxmt_config(), Some(ISAAC_DXMT_CONFIG));
    assert_eq!(profile.savedata_path_marker(), Some("Save Data Path:"));
    assert_eq!(
        profile.default_documents_subdir(),
        Some("Binding of Isaac Rebirth")
    );
}
