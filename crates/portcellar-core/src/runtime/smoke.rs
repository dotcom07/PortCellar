use super::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn game_smoke_evidence(
    profile: &dyn GameProfile,
    launch_mode: LaunchMode,
    observation: GameRuntimeObservation,
    stable_window_seconds: u64,
    passed: bool,
    failure_reason: Option<String>,
) -> GameSmokeEvidence {
    let engine = observation.wine.as_deref().map(wine_engine_kind_for_path);
    let backend_compatibility = engine.and_then(|kind| {
        engine_backend_matrix(kind)
            .into_iter()
            .find(|entry| entry.backend == profile.graphics_backend())
    });
    let execution_fingerprint = runtime_execution_fingerprint(&observation, engine);

    GameSmokeEvidence {
        observed_at_unix_seconds: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        profile: observation,
        launch_mode,
        engine,
        backend_compatibility,
        execution_fingerprint,
        graphics_backend: profile.graphics_backend(),
        steam_integration: profile.steam_integration(),
        stable_window_seconds,
        passed,
        failure_reason,
    }
}

fn runtime_execution_fingerprint(
    observation: &GameRuntimeObservation,
    engine: Option<EngineKind>,
) -> RuntimeExecutionFingerprint {
    let bottle_layout = if observation
        .prefix
        .parent()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        == Some("Bottles")
    {
        "crossover-bottle"
    } else {
        "wine-prefix"
    };
    let launcher_model = match engine {
        Some(EngineKind::CrossOver) => "crossover-hosted-launcher",
        Some(_) => "wine-launcher",
        None => "unknown",
    };

    RuntimeExecutionFingerprint {
        windows_version: observation.windows_version.clone(),
        bottle_layout: bottle_layout.to_string(),
        launcher_model: launcher_model.to_string(),
    }
}

pub fn write_game_smoke_evidence(evidence: &GameSmokeEvidence) -> Result<PathBuf> {
    let root = state_root().join("smoke");
    let path = root
        .join(safe_component(&evidence.profile.app_id))
        .join("latest.md");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, render_game_smoke_evidence(evidence))?;
    write_game_compatibility_evidence(evidence)?;
    Ok(path)
}

pub fn write_game_compatibility_evidence(evidence: &GameSmokeEvidence) -> Result<PathBuf> {
    let root = state_root().join("compatibility");
    write_compatibility_evidence_at(&root, evidence)
}

fn write_compatibility_evidence_at(root: &Path, evidence: &GameSmokeEvidence) -> Result<PathBuf> {
    let app_root = root.join(safe_component(&evidence.profile.app_id));
    let history_root = app_root.join("history");
    fs::create_dir_all(&history_root)?;

    let document = compatibility_evidence_from_smoke(evidence);
    let contents = toml::to_string_pretty(&document).map_err(|error| {
        PortCellarError::Message(format!(
            "could not serialize compatibility evidence: {error}"
        ))
    })?;
    let latest = app_root.join("latest.toml");
    fs::write(&latest, &contents)?;

    let stem = format!(
        "{}-{}",
        document.observed_at_unix_seconds,
        safe_component(&document.launch_mode)
    );
    let mut history = history_root.join(format!("{stem}.toml"));
    let mut suffix = 1;
    while history.exists() {
        history = history_root.join(format!("{stem}-{suffix}.toml"));
        suffix += 1;
    }
    fs::write(&history, contents)?;
    Ok(latest)
}

pub fn load_game_compatibility_evidence(path: &Path) -> Result<GameCompatibilityEvidence> {
    let contents = fs::read_to_string(path)?;
    let evidence = toml::from_str::<GameCompatibilityEvidence>(&contents).map_err(|error| {
        PortCellarError::Message(format!("invalid compatibility evidence: {error}"))
    })?;
    if !matches!(
        evidence.format_version.as_str(),
        GAME_COMPATIBILITY_EVIDENCE_FORMAT_VERSION_V1 | GAME_COMPATIBILITY_EVIDENCE_FORMAT_VERSION
    ) {
        return Err(PortCellarError::Message(format!(
            "unsupported compatibility evidence format: {}",
            evidence.format_version
        )));
    }
    Ok(evidence)
}

fn compatibility_evidence_from_smoke(evidence: &GameSmokeEvidence) -> GameCompatibilityEvidence {
    GameCompatibilityEvidence {
        format_version: GAME_COMPATIBILITY_EVIDENCE_FORMAT_VERSION.to_string(),
        observed_at_unix_seconds: evidence.observed_at_unix_seconds,
        app_id: evidence.profile.app_id.clone(),
        profile_name: evidence.profile.profile_name.clone(),
        profile_version: evidence.profile.profile_version.clone(),
        launch_mode: format!("{:?}", evidence.launch_mode),
        engine: evidence.engine.map(|value| format!("{value:?}")),
        engine_version: evidence.profile.wine_version.clone(),
        execution_fingerprint: evidence.execution_fingerprint.clone(),
        graphics_backend: format!("{:?}", evidence.graphics_backend),
        compatibility_status: evidence
            .backend_compatibility
            .map(|value| format!("{:?}", value.status)),
        passed: evidence.passed,
        stable_window_seconds: evidence.stable_window_seconds,
        steam_session_ready: evidence.profile.steam_session_ready,
        game_running: evidence.profile.game_running,
        failure_reason: evidence.failure_reason.clone(),
    }
}

fn render_game_smoke_evidence(evidence: &GameSmokeEvidence) -> String {
    let result = if evidence.passed { "passed" } else { "failed" };
    let failure = evidence.failure_reason.as_deref().unwrap_or("none");

    format!(
        "# Game Smoke Evidence\n\n\
- observed_at_unix_seconds: {}\n\
- result: {}\n\
- app_id: {}\n\
- profile: {}\n\
- profile_version: {}\n\
- launch_mode: {:?}\n\
- engine: {:?}\n\
- backend_compatibility: {}\n\
- graphics_backend: {:?}\n\
- steam_integration: {:?}\n\
- stable_window_seconds: {}\n\
- failure_reason: {}\n\n\
## Runtime Observation\n\n\
- prefix: {}\n\
- wine: {}\n\
- wine_version: {}\n\
- windows_version: {}\n\
- bottle_layout: {}\n\
- launcher_model: {}\n\
- steam_exe: {}\n\
- game_executable: {}\n\
- steam_running: {}\n\
- game_running: {}\n\
- active_session: {}\n\
- steam_session_ready: {}\n\
- logged_in: {}\n\
- cached_credentials: {}\n\
- login_status: {}\n\
- connection_status: {}\n\
- cef_status: {}\n",
        evidence.observed_at_unix_seconds,
        result,
        evidence.profile.app_id,
        evidence.profile.profile_name,
        evidence.profile.profile_version,
        evidence.launch_mode,
        evidence.engine,
        display_backend_compatibility(evidence.backend_compatibility),
        evidence.graphics_backend,
        evidence.steam_integration,
        evidence.stable_window_seconds,
        failure,
        evidence.profile.prefix.display(),
        display_option_path(evidence.profile.wine.as_deref()),
        display_option(evidence.profile.wine_version.as_deref()),
        display_option(evidence.execution_fingerprint.windows_version.as_deref()),
        evidence.execution_fingerprint.bottle_layout,
        evidence.execution_fingerprint.launcher_model,
        display_option_path(evidence.profile.steam_exe.as_deref()),
        display_option_path(evidence.profile.game_executable.as_deref()),
        evidence.profile.steam_running,
        evidence.profile.game_running,
        display_option_bool(evidence.profile.active_session),
        evidence.profile.steam_session_ready,
        display_option_bool(evidence.profile.logged_in),
        display_option_bool(evidence.profile.cached_credentials),
        display_option(evidence.profile.login_status.as_deref()),
        display_option(evidence.profile.connection_status.as_deref()),
        display_option(evidence.profile.cef_status.as_deref()),
    )
}

fn display_option_path(value: Option<&Path>) -> String {
    value
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn display_option(value: Option<&str>) -> String {
    value.unwrap_or("unknown").to_string()
}

fn display_option_bool(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "unknown",
    }
}

fn display_backend_compatibility(value: Option<EngineBackendCompatibility>) -> String {
    value
        .map(|entry| format!("{:?}: {}", entry.status, entry.rationale))
        .unwrap_or_else(|| "unknown".to_string())
}

fn safe_component(value: &str) -> String {
    let component = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();
    if component.is_empty() {
        "game".to_string()
    } else {
        component
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_records_profile_policy_and_observation() {
        let profile = GenericGameProfile::new("123456", "Example", "Game.exe");
        let observation = GameRuntimeObservation {
            app_id: "123456".to_string(),
            profile_name: "Example".to_string(),
            profile_version: "generic-v1".to_string(),
            prefix: PathBuf::from("/tmp/prefix"),
            wine: Some(PathBuf::from(
                "/tmp/synthetic-project/.portcellar/engines/wine-11.10/Wine Staging.app/Contents/Resources/wine/bin/wine",
            )),
            wine_version: Some("wine-11.10".to_string()),
            windows_version: Some("win10".to_string()),
            steam_exe: None,
            game_executable: None,
            steam_running: true,
            game_running: true,
            active_session: Some(true),
            steam_session_ready: true,
            logged_in: Some(true),
            cached_credentials: Some(true),
            login_status: Some("success".to_string()),
            connection_status: Some("cm-transport-ready".to_string()),
            cef_status: None,
        };

        let evidence =
            game_smoke_evidence(&profile, LaunchMode::WineSteam, observation, 30, true, None);
        assert!(evidence.passed);
        assert_eq!(evidence.stable_window_seconds, 30);
        assert_eq!(evidence.engine, Some(EngineKind::Wine));
        assert_eq!(
            evidence.backend_compatibility.unwrap().status,
            CompatibilityStatus::Candidate
        );
        assert_eq!(evidence.profile.profile_version, "generic-v1");
        assert_eq!(
            evidence.execution_fingerprint.windows_version.as_deref(),
            Some("win10")
        );
        assert_eq!(evidence.execution_fingerprint.bottle_layout, "wine-prefix");
        assert_eq!(
            evidence.execution_fingerprint.launcher_model,
            "wine-launcher"
        );
    }

    #[test]
    fn safe_component_never_creates_nested_paths() {
        assert_eq!(safe_component("game/name"), "game-name");
        assert_eq!(safe_component(""), "game");
    }

    #[test]
    fn execution_fingerprint_identifies_a_crossover_bottle() {
        let profile = GenericGameProfile::new("123456", "Example", "Game.exe");
        let observation = GameRuntimeObservation {
            app_id: "123456".to_string(),
            profile_name: "Example".to_string(),
            profile_version: "generic-v1".to_string(),
            prefix: PathBuf::from("/tmp/synthetic-crossover/Bottles/Example"),
            wine: Some(PathBuf::from(
                "/Applications/CrossOver.app/Contents/SharedSupport/CrossOver/bin/wine",
            )),
            wine_version: Some("CrossOver 26.2".to_string()),
            windows_version: Some("win10".to_string()),
            steam_exe: None,
            game_executable: None,
            steam_running: false,
            game_running: true,
            active_session: None,
            steam_session_ready: false,
            logged_in: None,
            cached_credentials: None,
            login_status: None,
            connection_status: None,
            cef_status: None,
        };

        let evidence =
            game_smoke_evidence(&profile, LaunchMode::WineDirect, observation, 5, true, None);

        assert_eq!(
            evidence.execution_fingerprint.windows_version.as_deref(),
            Some("win10")
        );
        assert_eq!(
            evidence.execution_fingerprint.bottle_layout,
            "crossover-bottle"
        );
        assert_eq!(
            evidence.execution_fingerprint.launcher_model,
            "crossover-hosted-launcher"
        );
    }

    #[test]
    fn compatibility_evidence_is_structured_without_runtime_paths() {
        let profile = GenericGameProfile::new("123456", "Example", "Game.exe");
        let observation = GameRuntimeObservation {
            app_id: "123456".to_string(),
            profile_name: "Example".to_string(),
            profile_version: "generic-v1".to_string(),
            prefix: PathBuf::from("/tmp/synthetic-prefix"),
            wine: Some(PathBuf::from("/tmp/synthetic-wine")),
            wine_version: Some("wine-11.10".to_string()),
            windows_version: Some("win10".to_string()),
            steam_exe: None,
            game_executable: None,
            steam_running: false,
            game_running: false,
            active_session: Some(false),
            steam_session_ready: false,
            logged_in: Some(true),
            cached_credentials: Some(true),
            login_status: Some("success".to_string()),
            connection_status: Some("cm-transport-ready".to_string()),
            cef_status: None,
        };
        let evidence = game_smoke_evidence(
            &profile,
            LaunchMode::WineSteam,
            observation,
            30,
            false,
            Some("active Steam session missing".to_string()),
        );
        let document = compatibility_evidence_from_smoke(&evidence);
        let serialized = toml::to_string(&document).unwrap();

        assert_eq!(
            document.format_version,
            GAME_COMPATIBILITY_EVIDENCE_FORMAT_VERSION
        );
        assert!(!serialized.contains("/tmp/synthetic-prefix"));
        assert!(!serialized.contains("/tmp/synthetic-wine"));
        assert!(serialized.contains("active Steam session missing"));
        assert!(serialized.contains("bottle_layout = \"wine-prefix\""));
        assert!(!serialized.contains("runtime_execution_fingerprint"));
        assert_eq!(load_document(&serialized).unwrap(), document);

        let root = std::env::temp_dir().join(format!(
            "portcellar-compatibility-evidence-{}",
            std::process::id()
        ));
        let latest = write_compatibility_evidence_at(&root, &evidence).unwrap();
        let second = write_compatibility_evidence_at(&root, &evidence).unwrap();
        assert_eq!(latest, root.join("123456/latest.toml"));
        assert_eq!(second, latest);
        let history = std::fs::read_dir(root.join("123456/history"))
            .unwrap()
            .count();
        assert_eq!(history, 2);
        assert_eq!(load_game_compatibility_evidence(&latest).unwrap(), document);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn compatibility_evidence_rejects_unknown_format() {
        let path = std::env::temp_dir().join(format!(
            "portcellar-invalid-compatibility-evidence-{}.toml",
            std::process::id()
        ));
        let document = GameCompatibilityEvidence {
            format_version: "game-compatibility-evidence-v0".to_string(),
            observed_at_unix_seconds: 1,
            app_id: "123456".to_string(),
            profile_name: "Example".to_string(),
            profile_version: "v1".to_string(),
            launch_mode: "WineSteam".to_string(),
            engine: Some("Wine".to_string()),
            engine_version: Some("wine-11".to_string()),
            execution_fingerprint: RuntimeExecutionFingerprint::default(),
            graphics_backend: "Auto".to_string(),
            compatibility_status: Some("Candidate".to_string()),
            passed: false,
            stable_window_seconds: 30,
            steam_session_ready: false,
            game_running: false,
            failure_reason: Some("fixture".to_string()),
        };
        std::fs::write(&path, toml::to_string(&document).unwrap()).unwrap();

        let error = load_game_compatibility_evidence(&path).unwrap_err();
        assert!(error
            .to_string()
            .contains("unsupported compatibility evidence format"));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn compatibility_evidence_loads_v1_without_a_fingerprint() {
        let path = std::env::temp_dir().join(format!(
            "portcellar-v1-compatibility-evidence-{}.toml",
            std::process::id()
        ));
        let contents = r#"
format_version = "game-compatibility-evidence-v1"
observed_at_unix_seconds = 1
app_id = "123456"
profile_name = "Example"
profile_version = "v1"
launch_mode = "WineSteam"
engine = "Wine"
engine_version = "wine-11"
graphics_backend = "Auto"
compatibility_status = "Candidate"
passed = false
stable_window_seconds = 0
steam_session_ready = false
game_running = false
failure_reason = "legacy"
"#;
        std::fs::write(&path, contents).unwrap();

        let document = load_game_compatibility_evidence(&path).unwrap();
        assert_eq!(
            document.execution_fingerprint,
            RuntimeExecutionFingerprint::default()
        );
        std::fs::remove_file(path).unwrap();
    }

    fn load_document(value: &str) -> Result<GameCompatibilityEvidence> {
        toml::from_str(value).map_err(|error| {
            PortCellarError::Message(format!("invalid compatibility evidence: {error}"))
        })
    }
}
