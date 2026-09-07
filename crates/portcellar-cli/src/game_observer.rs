use super::*;

pub(crate) fn run_game_launch_plan_detached(
    profile: &dyn GameProfile,
    plan: &portcellar_core::CommandPlan,
    capture_wine_log: bool,
) -> Result<u32> {
    if !capture_wine_log {
        return run_plan_detached(plan);
    }

    let runtime = inspect_game_runtime(profile);
    let log_name = format!(
        "portcellar-{}-launch.log",
        safe_log_component(profile.app_id())
    );
    let log_root = if profile.steam_integration() == SteamIntegration::None {
        runtime.prefix.join("portcellar/logs")
    } else {
        runtime.prefix.join("drive_c/Steam/logs")
    };
    let log_path = log_root.join(log_name);
    println!("capturing Wine launch log: {}", log_path.display());
    run_plan_detached_with_log(plan, &log_path)
}

pub(crate) fn wait_for_game(
    profile: &dyn GameProfile,
    launch_mode: LaunchMode,
    timeout: Duration,
    stable_duration: Duration,
) -> Result<()> {
    let started = Instant::now();
    let mut first_seen = None;
    println!(
        "waiting for {} and required runtime state to stay alive for {} second(s)...",
        profile.name(),
        stable_duration.as_secs()
    );

    while started.elapsed() < timeout {
        let runtime = inspect_game_runtime(profile);
        if portcellar_core::game_runtime_ready(profile, &runtime) {
            let first_seen_at = first_seen.get_or_insert_with(Instant::now);
            if first_seen_at.elapsed() >= stable_duration {
                println!("{} process: stable", profile.name());
                let observation = portcellar_core::observe_game_runtime(profile);
                record_smoke_evidence(
                    profile,
                    launch_mode,
                    observation,
                    stable_duration,
                    true,
                    None,
                );
                return Ok(());
            }
        } else {
            first_seen = None;
        }
        std::thread::sleep(Duration::from_secs(2));
    }

    let runtime = inspect_game_runtime(profile);
    let failure = if profile.steam_integration() == SteamIntegration::None {
        format!(
            "{} process did not stay ready; game running: {}; action: inspect the Wine launch log and private smoke evidence",
            profile.name(),
            pass_fail(runtime.game_running),
        )
    } else {
        format!(
            "{} process did not stay ready; Steam running: {}; active session: {}; login: {}; connection: {}; CEF: {}; action: {}",
            profile.name(),
            pass_fail(runtime.running),
            runtime
                .active_session
                .map(|value| if value { "yes" } else { "no" })
                .unwrap_or("unknown"),
            runtime.login_status.as_deref().unwrap_or("unknown"),
            runtime.connection_status.as_deref().unwrap_or("unknown"),
            runtime.cef_status.as_deref().unwrap_or("unknown"),
            steam_login_action_for_runtime(&runtime)
        )
    };
    let observation = portcellar_core::observe_game_runtime(profile);
    record_smoke_evidence(
        profile,
        launch_mode,
        observation,
        stable_duration,
        false,
        Some(failure.clone()),
    );
    Err(portcellar_core::PortCellarError::Message(failure))
}

pub(crate) fn wait_for_wine_login(profile: &dyn GameProfile, timeout: Duration) -> Result<()> {
    let started = Instant::now();
    println!("waiting for {} Steam session...", profile.name());

    while started.elapsed() < timeout {
        let runtime = inspect_game_runtime(profile);
        if portcellar_core::steam_session_ready(&runtime) {
            println!("Steam session: ok");
            return Ok(());
        }
        std::thread::sleep(Duration::from_secs(2));
    }

    let runtime = inspect_game_runtime(profile);
    let failure = format!(
        "Steam session for {} did not become active; active session: {}; login: {}; connection: {}; CEF: {}; action: {}",
        profile.name(),
        runtime
            .active_session
            .map(|value| if value { "yes" } else { "no" })
            .unwrap_or("unknown"),
        runtime.login_status.as_deref().unwrap_or("unknown"),
        runtime.connection_status.as_deref().unwrap_or("unknown"),
        runtime.cef_status.as_deref().unwrap_or("unknown"),
        steam_login_action_for_runtime(&runtime)
    );
    let observation = portcellar_core::observe_game_runtime(profile);
    record_smoke_evidence(
        profile,
        LaunchMode::WineSteam,
        observation,
        Duration::from_secs(0),
        false,
        Some(failure.clone()),
    );
    Err(portcellar_core::PortCellarError::Message(failure))
}

fn safe_log_component(value: &str) -> String {
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

fn record_smoke_evidence(
    profile: &dyn GameProfile,
    launch_mode: LaunchMode,
    observation: portcellar_core::GameRuntimeObservation,
    stable_duration: Duration,
    passed: bool,
    failure_reason: Option<String>,
) {
    let evidence = portcellar_core::game_smoke_evidence(
        profile,
        launch_mode,
        observation,
        stable_duration.as_secs(),
        passed,
        failure_reason,
    );
    match portcellar_core::write_game_smoke_evidence(&evidence) {
        Ok(path) => println!("smoke evidence: {}", path.display()),
        Err(error) => eprintln!("warning: could not write smoke evidence: {error}"),
    }
}
