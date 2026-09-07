use portcellar_core::{
    analyze_game, analyze_local_game, apply_game_runtime_profile, apply_isaac_runtime_profile,
    apply_steam_cef_patch, apply_steam_session_reset, collect_porting_analysis,
    collect_steam_client_analysis, doctor_report, game_launch_plan, game_runtime_profile_plan,
    inspect_game_runtime, inspect_game_runtime_preflight, isaac_launch_plan, isaac_profile,
    isaac_runtime_profile_plan, kill_wine_steam_processes, prepare_game_runtime_stage, run_plan,
    run_plan_detached, run_plan_detached_with_log, state_root, wine_steam_cef_patch_plan,
    wine_steam_cef_patch_plan_for, wine_steam_configure_plans, wine_steam_configure_plans_for,
    wine_steam_login_plan, wine_steam_login_plan_for, wine_steam_session_reset_plan,
    wine_steam_stop_plan, CompatibilityStatus, EngineKind, GameProfile, GenericGameProfile,
    GenericGameProfileCatalog, LaunchMode, Result, SteamClientAnalysisOptions, SteamIntegration,
};
use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
use std::time::{Duration, Instant};

mod game_bottle;
mod game_evidence;
mod game_installer;
mod game_modules;
mod game_observer;
mod game_steam;
mod game_support;

fn main() {
    if let Err(error) = run(env::args_os().skip(1).collect()) {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run(args: Vec<OsString>) -> Result<()> {
    let args = args
        .into_iter()
        .map(|arg| arg.to_string_lossy().to_string())
        .collect::<Vec<_>>();

    match args.as_slice() {
        [] => {
            print_help();
            Ok(())
        }
        [cmd] if cmd == "help" || cmd == "--help" || cmd == "-h" => {
            print_help();
            Ok(())
        }
        [cmd] if cmd == "doctor" => {
            print_doctor();
            Ok(())
        }
        [area, cmd, rest @ ..] if area == "analyze" && cmd == "collect" => analyze_collect(rest),
        [area, cmd, rest @ ..] if area == "analyze" && cmd == "local" => analyze_local(rest),
        [area, cmd, rest @ ..] if area == "analyze" && cmd == "steam-client" => {
            analyze_steam_client(rest)
        }
        [area, cmd] if area == "isaac" && cmd == "doctor" => {
            print_doctor();
            Ok(())
        }
        [area, cmd, rest @ ..] if area == "isaac" && cmd == "configure-wine" => {
            configure_wine(rest)
        }
        [area, cmd, rest @ ..] if area == "isaac" && cmd == "run" => isaac_run(rest),
        [area, cmd, rest @ ..] if area == "isaac" && cmd == "play" => isaac_play(rest),
        [area, cmd, rest @ ..] if area == "isaac" && cmd == "launch" => launch(rest),
        [area, cmd, rest @ ..] if area == "game" && cmd == "profiles" => game_profiles(rest),
        [area, cmd, rest @ ..] if area == "game" && (cmd == "modules" || cmd == "module") => {
            game_modules::inspect(rest, cmd == "module")
        }
        [area, cmd, rest @ ..] if area == "game" && cmd == "evidence" => {
            game_evidence::evidence(rest)
        }
        [area, cmd, rest @ ..] if area == "game" && cmd == "mutation-plan" => {
            game_bottle::mutation_plan(rest)
        }
        [area, cmd, rest @ ..] if area == "game" && cmd == "mutation" => {
            game_bottle::mutation(rest)
        }
        [area, cmd, rest @ ..] if area == "game" && cmd == "rollback-plan" => {
            game_bottle::rollback_plan(rest)
        }
        [area, cmd, rest @ ..] if area == "game" && cmd == "rollback" => {
            game_bottle::rollback(rest)
        }
        [area, cmd, rest @ ..] if area == "game" && cmd == "install-plan" => {
            game_installer::install_plan(rest)
        }
        [area, cmd, rest @ ..] if area == "game" && cmd == "install" => {
            game_installer::install(rest)
        }
        [area, cmd, rest @ ..] if area == "game" && cmd == "steam-install" => {
            game_steam::install(rest)
        }
        [area, cmd, rest @ ..] if area == "game" && cmd == "launch" => game_launch(rest),
        [area, cmd, rest @ ..] if area == "isaac" && cmd == "steam-login" => steam_login(rest),
        [area, cmd, rest @ ..] if area == "isaac" && cmd == "steam-stop" => steam_stop(rest),
        [area, cmd, rest @ ..] if area == "isaac" && cmd == "steam-reset-session" => {
            steam_reset_session(rest)
        }
        [area, cmd, rest @ ..] if area == "isaac" && cmd == "steam-patch-cef" => {
            steam_patch_cef(rest)
        }
        [cmd] if cmd == "engines" => {
            print_engines();
            Ok(())
        }
        _ => {
            print_help();
            Err(portcellar_core::PortCellarError::Message(
                "unknown command".to_string(),
            ))
        }
    }
}

fn print_help() {
    println!("portcellar runtime CLI");
    println!();
    println!("Commands:");
    println!("  portcellar doctor");
    println!("  portcellar engines");
    println!("  portcellar analyze collect --app-id APPID");
    println!(
        "  portcellar analyze local --path DIR [--name NAME] [--app-id APPID] [--exe RELATIVE.EXE]"
    );
    println!(
        "  portcellar analyze steam-client [--probe] [--probe-seconds N] [--stop-after] [--legacy-login]"
    );
    println!("  portcellar isaac doctor");
    println!("  portcellar isaac configure-wine [--dry-run]");
    println!("  portcellar isaac play [run options]");
    println!(
        "  portcellar isaac run [--dry-run] [--legacy-login] [--capture-wine-log] [--login-timeout-seconds N] [--launch-timeout-seconds N] [--stable-seconds N]"
    );
    println!(
        "  portcellar isaac steam-login [--dry-run] [--detach] [--wait] [--legacy-login] [--timeout-seconds N]"
    );
    println!("  portcellar isaac steam-stop [--dry-run]");
    println!("  portcellar isaac steam-reset-session [--dry-run]");
    println!("  portcellar isaac steam-patch-cef [--dry-run]");
    println!(
        "  portcellar isaac launch [--mode steam|native|direct|wine-steam|wine-direct] [--dry-run] [--detach] [--wait] [--wait-login] [--legacy-login] [--capture-wine-log] [--login-timeout-seconds N] [--launch-timeout-seconds N] [--stable-seconds N]"
    );
    println!("  portcellar game profiles [--root PATH]");
    println!("  portcellar game modules [--root PATH]");
    println!("  portcellar game module [--root PATH] --id MODULE");
    println!("  portcellar game evidence [--app-id APPID] [--root PATH]");
    println!(
        "  portcellar game mutation-plan [--from-profile PATH | --from-catalog APPID] --snapshot-id ID [--profile-root PATH]"
    );
    println!(
        "  portcellar game mutation [--from-profile PATH | --from-catalog APPID] --snapshot-id ID --confirm [--profile-root PATH]"
    );
    println!(
        "  portcellar game rollback-plan [--from-profile PATH | --from-catalog APPID] --snapshot-id ID [--profile-root PATH]"
    );
    println!(
        "  portcellar game rollback [--from-profile PATH | --from-catalog APPID] --snapshot-id ID --confirm [--profile-root PATH]"
    );
    println!(
        "  portcellar game install-plan [--from-profile PATH | --from-catalog APPID] --installer PATH [--installer-arg VALUE] [--profile-root PATH]"
    );
    println!(
        "  portcellar game install [--from-profile PATH | --from-catalog APPID] --installer PATH [--installer-arg VALUE] --snapshot-id ID --confirm [--profile-root PATH]"
    );
    println!(
        "  portcellar game steam-install --app-id APPID [--name NAME] [--dry-run] [--confirm]"
    );
    println!(
        "  portcellar game launch [--from-catalog APPID --profile-root PATH | --from-profile PATH | --from-analysis APPID | --app-id APPID --name NAME --exe GAME.EXE] [--launch-arg VALUE] [--install-dir DIR] [--wine-engine PATH] [--no-steam --windows-path C:\\Games\\Example] [--mode wine-steam|wine-direct|steam|native|direct] [--dry-run] [--detach] [--wait] [--wait-login] [--legacy-login] [--capture-wine-log] [--login-timeout-seconds N] [--launch-timeout-seconds N] [--stable-seconds N]"
    );
}

fn game_profiles(args: &[String]) -> Result<()> {
    let mut root = default_profile_catalog_root();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--root" => {
                index += 1;
                root = PathBuf::from(args.get(index).ok_or_else(|| {
                    portcellar_core::PortCellarError::Message("--root requires a path".to_string())
                })?);
            }
            value if value.starts_with("--root=") => {
                root = PathBuf::from(value.trim_start_matches("--root="));
            }
            value => {
                return Err(portcellar_core::PortCellarError::Message(format!(
                    "unknown game profiles option: {value}"
                )));
            }
        }
        index += 1;
    }

    let catalog = GenericGameProfileCatalog::load(&root)?;
    println!("profile catalog: {}", catalog.root().display());
    if catalog.entries().is_empty() {
        println!("no profiles found");
        return Ok(());
    }
    for entry in catalog.entries() {
        println!(
            "{}\t{}\t{}\t{}",
            entry.app_id(),
            entry.name(),
            entry.profile().runtime_profile_version(),
            entry.path().display()
        );
    }
    Ok(())
}

pub(crate) fn default_profile_catalog_root() -> PathBuf {
    env::var_os("PORTCELLAR_PROFILE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| state_root().join("profiles"))
}

fn analyze_collect(args: &[String]) -> Result<()> {
    let mut app_id = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--app-id" => {
                index += 1;
                app_id = args.get(index).cloned();
            }
            value if value.starts_with("--app-id=") => {
                app_id = Some(value.trim_start_matches("--app-id=").to_string());
            }
            value => {
                return Err(portcellar_core::PortCellarError::Message(format!(
                    "unknown analyze collect option: {value}"
                )));
            }
        }
        index += 1;
    }

    let app_id = app_id.ok_or_else(|| {
        portcellar_core::PortCellarError::Message(
            "analyze collect requires --app-id APPID".to_string(),
        )
    })?;
    let artifacts = collect_porting_analysis(&app_id)?;

    println!("Porting analysis collected");
    println!("  root: {}", artifacts.root.display());
    println!("  report: {}", artifacts.report_path.display());
    println!(
        "  profile draft: {}",
        artifacts.profile_draft_path.display()
    );
    if let Some(profile_path) = artifacts.profile_path {
        println!("  profile: {}", profile_path.display());
    }
    println!("  reproduce: {}", artifacts.reproduce_path.display());
    println!("  raw doctor: {}", artifacts.raw_doctor_path.display());
    println!("  experiment: {}", artifacts.experiment_path.display());

    Ok(())
}

fn analyze_local(args: &[String]) -> Result<()> {
    let mut path = None;
    let mut name = None;
    let mut app_id = None;
    let mut executable = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--path" => {
                index += 1;
                path = args.get(index).map(PathBuf::from);
            }
            value if value.starts_with("--path=") => {
                path = Some(PathBuf::from(value.trim_start_matches("--path=")));
            }
            "--name" => {
                index += 1;
                name = args.get(index).cloned();
            }
            value if value.starts_with("--name=") => {
                name = Some(value.trim_start_matches("--name=").to_string());
            }
            "--app-id" => {
                index += 1;
                app_id = args.get(index).cloned();
            }
            value if value.starts_with("--app-id=") => {
                app_id = Some(value.trim_start_matches("--app-id=").to_string());
            }
            "--exe" => {
                index += 1;
                executable = args.get(index).cloned();
            }
            value if value.starts_with("--exe=") => {
                executable = Some(value.trim_start_matches("--exe=").to_string());
            }
            value => {
                return Err(portcellar_core::PortCellarError::Message(format!(
                    "unknown analyze local option: {value}"
                )));
            }
        }
        index += 1;
    }

    let path = path.ok_or_else(|| {
        portcellar_core::PortCellarError::Message("analyze local requires --path DIR".to_string())
    })?;
    let analysis = analyze_local_game(
        &path,
        name.as_deref(),
        app_id.as_deref(),
        executable.as_deref(),
    )?;
    let artifacts = portcellar_core::write_local_game_analysis_artifacts(&analysis)?;

    println!("Local game analysis collected");
    println!("  name: {}", analysis.name);
    println!("  root: {}", analysis.root.display());
    println!("  app id: {}", analysis.app_id);
    println!(
        "  selected executable: {}",
        analysis
            .selected_exe
            .as_ref()
            .map(|candidate| candidate.relative_path.as_str())
            .unwrap_or("none")
    );
    for candidate in &analysis.exe_candidates {
        let selected = analysis
            .selected_exe
            .as_ref()
            .map(|value| value.relative_path == candidate.relative_path)
            .unwrap_or(false);
        println!(
            "  candidate{}: {} score={} layers={}",
            if selected { " (selected)" } else { "" },
            candidate.relative_path,
            candidate.score,
            candidate
                .pe
                .as_ref()
                .map(|pe| pe.detected_layers.join(","))
                .unwrap_or_else(|| "unknown".to_string())
        );
    }
    for item in &analysis.review_items {
        println!("  review: {item}");
    }
    println!("  report: {}", artifacts.report_path.display());
    println!("  profile: {}", artifacts.profile_path.display());
    println!("  reproduce: {}", artifacts.reproduce_path.display());

    Ok(())
}

fn analyze_steam_client(args: &[String]) -> Result<()> {
    let mut options = SteamClientAnalysisOptions::default();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--probe" => {
                options.probe = true;
            }
            "--stop-after" => {
                options.stop_after = true;
            }
            "--legacy-login" => {
                options.legacy_login = true;
            }
            "--probe-seconds" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    portcellar_core::PortCellarError::Message(
                        "--probe-seconds requires a numeric value".to_string(),
                    )
                })?;
                options.probe_seconds = parse_u64(value, "--probe-seconds")?;
            }
            value if value.starts_with("--probe-seconds=") => {
                options.probe_seconds = parse_u64(
                    value.trim_start_matches("--probe-seconds="),
                    "--probe-seconds",
                )?;
            }
            value => {
                return Err(portcellar_core::PortCellarError::Message(format!(
                    "unknown analyze steam-client option: {value}"
                )));
            }
        }
        index += 1;
    }

    let artifacts = collect_steam_client_analysis(options)?;

    println!("Steam client analysis collected");
    println!("  root: {}", artifacts.root.display());
    println!("  report: {}", artifacts.report_path.display());
    println!(
        "  process before: {}",
        artifacts.raw_process_before_path.display()
    );
    println!(
        "  process after: {}",
        artifacts.raw_process_after_path.display()
    );
    println!("  probe log: {}", artifacts.raw_probe_log_path.display());
    println!("  experiment: {}", artifacts.experiment_path.display());

    Ok(())
}

fn print_doctor() {
    let report = doctor_report();

    println!("Host");
    println!("  native arch: {}", report.host.native_arch);
    println!(
        "  translated: {}",
        report
            .host
            .translated
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    );
    println!(
        "  Rosetta x86_64: {}",
        pass_fail(report.host.rosetta_x86_64)
    );
    println!();

    println!("Steam");
    println!(
        "  root: {}",
        report
            .steam
            .root
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "<not found>".to_string())
    );
    println!("  running: {}", pass_fail(report.steam.running));
    println!("  libraries:");
    for library in &report.steam.libraries {
        println!("    - {}", library.display());
    }
    println!();

    println!("Isaac");
    if let Some(isaac) = &report.game {
        println!("  app id: {}", isaac.app_id);
        println!("  name: {}", isaac.name);
        println!(
            "  build id: {}",
            isaac.build_id.as_deref().unwrap_or("<unknown>")
        );
        println!("  manifest: {}", isaac.manifest_path.display());
        println!("  install dir: {}", isaac.install_dir.display());
        println!(
            "  app bundle: {}",
            isaac
                .app_bundle
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "<not found>".to_string())
        );
        println!(
            "  executable: {}",
            isaac
                .executable
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "<not found>".to_string())
        );
        println!(
            "  executable kind: {}",
            isaac
                .executable_kind
                .as_deref()
                .unwrap_or("<not inspected>")
        );
        println!(
            "  steam_appid.txt: {}",
            isaac.steam_appid_txt.as_deref().unwrap_or("<missing>")
        );
    } else {
        println!("  not installed in detected Steam libraries");
    }
    println!();

    println!("Wine Steam");
    println!("  prefix: {}", report.wine_steam.prefix.display());
    println!(
        "  wine: {}",
        report
            .wine_steam
            .wine
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "<not found>".to_string())
    );
    println!(
        "  wine version: {}",
        report
            .wine_steam
            .wine_version
            .as_deref()
            .unwrap_or("unknown")
    );
    println!(
        "  Windows version: {}",
        report
            .wine_steam
            .windows_version
            .as_deref()
            .unwrap_or("default")
    );
    println!(
        "  DXMT root: {}",
        report
            .wine_steam
            .dxmt_root
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "<not found>".to_string())
    );
    println!(
        "  Steam exe: {}",
        report
            .wine_steam
            .steam_exe
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "<not found>".to_string())
    );
    println!("  running: {}", pass_fail(report.wine_steam.running));
    println!(
        "  Isaac process: {}",
        pass_fail(report.wine_steam.game_running)
    );
    println!(
        "  launch ready after login: {}",
        pass_fail(report.wine_steam.launch_ready_after_login)
    );
    for issue in &report.wine_steam.readiness_issues {
        println!("    - {issue}");
    }
    println!(
        "  logged in: {}",
        report
            .wine_steam
            .logged_in
            .map(|value| if value { "likely" } else { "no" })
            .unwrap_or("unknown")
    );
    println!(
        "  active session: {}",
        report
            .wine_steam
            .active_session
            .map(|value| if value { "yes" } else { "no" })
            .unwrap_or("unknown")
    );
    println!(
        "  cached credentials: {}",
        report
            .wine_steam
            .cached_credentials
            .map(|value| if value { "present" } else { "missing" })
            .unwrap_or("unknown")
    );
    println!(
        "  login status: {}",
        report
            .wine_steam
            .login_status
            .as_deref()
            .unwrap_or("unknown")
    );
    println!(
        "  connection status: {}",
        report
            .wine_steam
            .connection_status
            .as_deref()
            .unwrap_or("unknown")
    );
    println!(
        "  CEF status: {}",
        report.wine_steam.cef_status.as_deref().unwrap_or("unknown")
    );
    println!(
        "  macOS window status: {}",
        report
            .wine_steam
            .mac_window_status
            .as_deref()
            .unwrap_or("unknown")
    );
    println!(
        "  virtual desktop: {}",
        report
            .wine_steam
            .virtual_desktop
            .as_deref()
            .unwrap_or("disabled")
    );
    println!(
        "  macdrv AllowImmovableWindows: {}",
        report
            .wine_steam
            .mac_driver
            .allow_immovable_windows
            .as_deref()
            .unwrap_or("default")
    );
    println!(
        "  macdrv RetinaMode: {}",
        report
            .wine_steam
            .mac_driver
            .retina_mode
            .as_deref()
            .unwrap_or("default")
    );
    println!(
        "  Isaac manifest: {}",
        report
            .wine_steam
            .game_manifest
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "<not found>".to_string())
    );
    println!(
        "  Isaac install dir: {}",
        report
            .wine_steam
            .game_install_dir
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "<not found>".to_string())
    );
    println!(
        "  Isaac executable: {}",
        report
            .wine_steam
            .game_executable
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "<not found>".to_string())
    );
    println!();

    print_engines_from_report(&report);

    println!();
    println!("Latest Isaac crash");
    if let Some(crash) = &report.latest_crash {
        println!("  report: {}", crash.path.display());
        println!(
            "  translated: {}",
            crash
                .translated
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        );
        println!("  cpu: {}", crash.cpu_type.as_deref().unwrap_or("unknown"));
        println!(
            "  exception: {}",
            crash.exception.as_deref().unwrap_or("<not found>")
        );
        println!(
            "  termination: {}",
            crash.termination.as_deref().unwrap_or("<not found>")
        );
        if !crash.suspect_symbols.is_empty() {
            println!("  suspect symbols: {}", crash.suspect_symbols.join(", "));
        }
        if !crash.suspect_images.is_empty() {
            println!("  suspect images: {}", crash.suspect_images.join(", "));
        }
        if let Some(diagnosis) = &crash.diagnosis {
            println!("  diagnosis: {diagnosis}");
        }
    } else {
        println!("  no local Isaac crash reports found");
    }

    match isaac_launch_plan(LaunchMode::WineSteam).or_else(|_| isaac_launch_plan(LaunchMode::Steam))
    {
        Ok(plan) => println!("\nRecommended launch: {}", plan.display()),
        Err(error) => println!("\nRecommended launch unavailable: {error}"),
    }
}

fn print_engines() {
    let report = doctor_report();
    print_engines_from_report(&report);
}

fn print_engines_from_report(report: &portcellar_core::DoctorReport) {
    println!("Engines");
    for engine in &report.engines {
        let kind = match engine.kind {
            EngineKind::NativeSteam => "native-steam",
            EngineKind::NativeApp => "native-app",
            EngineKind::CrossOver => "crossover",
            EngineKind::Wine => "wine",
            EngineKind::GamePortingToolkit => "gptk",
        };
        println!(
            "  [{}] {} ({})",
            pass_fail(engine.available),
            engine.name,
            kind
        );
        println!("      path: {}", engine.path.display());
        if let Some(version) = &engine.version {
            println!("      version: {version}");
        }
        if let Some(detail) = &engine.detail {
            println!("      detail: {detail}");
        }
        if !engine.components.is_empty() {
            let components = engine
                .components
                .iter()
                .map(|component| format!("{component:?}"))
                .collect::<Vec<_>>()
                .join(", ");
            println!("      runtime components: {components}");
        }
        if !engine.backend_candidates.is_empty() {
            let backends = engine
                .backend_candidates
                .iter()
                .map(|backend| format!("{backend:?}"))
                .collect::<Vec<_>>()
                .join(", ");
            println!("      backend candidates: {backends}");
        }
        if !engine.backend_matrix.is_empty() {
            let matrix = engine
                .backend_matrix
                .iter()
                .filter(|entry| entry.status != CompatibilityStatus::Unsupported)
                .map(|entry| {
                    format!(
                        "{:?}={}",
                        entry.backend,
                        compatibility_status_label(entry.status)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            println!("      compatibility matrix: {matrix}");
        }
    }
}

fn compatibility_status_label(status: CompatibilityStatus) -> &'static str {
    match status {
        CompatibilityStatus::Verified => "verified",
        CompatibilityStatus::Candidate => "candidate",
        CompatibilityStatus::ExternalDependency => "external-dependency",
        CompatibilityStatus::Unsupported => "unsupported",
    }
}

fn launch(args: &[String]) -> Result<()> {
    let mut mode = LaunchMode::Steam;
    let mut dry_run = false;
    let mut detach = false;
    let mut wait = false;
    let mut wait_login = false;
    let mut legacy_login = false;
    let mut capture_wine_log = false;
    let mut login_timeout = Duration::from_secs(600);
    let mut launch_timeout = Duration::from_secs(90);
    let mut stable_duration = Duration::from_secs(10);
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--dry-run" => dry_run = true,
            "--detach" => detach = true,
            "--wait" => wait = true,
            "--wait-login" => wait_login = true,
            "--legacy-login" => legacy_login = true,
            "--capture-wine-log" => capture_wine_log = true,
            "--login-timeout-seconds" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    portcellar_core::PortCellarError::Message(
                        "--login-timeout-seconds requires a number".to_string(),
                    )
                })?;
                login_timeout = parse_timeout(value)?;
            }
            value if value.starts_with("--login-timeout-seconds=") => {
                login_timeout =
                    parse_timeout(value.trim_start_matches("--login-timeout-seconds="))?;
            }
            "--launch-timeout-seconds" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    portcellar_core::PortCellarError::Message(
                        "--launch-timeout-seconds requires a number".to_string(),
                    )
                })?;
                launch_timeout = parse_timeout(value)?;
            }
            value if value.starts_with("--launch-timeout-seconds=") => {
                launch_timeout =
                    parse_timeout(value.trim_start_matches("--launch-timeout-seconds="))?;
            }
            "--stable-seconds" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    portcellar_core::PortCellarError::Message(
                        "--stable-seconds requires a number".to_string(),
                    )
                })?;
                stable_duration = parse_timeout(value)?;
            }
            value if value.starts_with("--stable-seconds=") => {
                stable_duration = parse_timeout(value.trim_start_matches("--stable-seconds="))?;
            }
            "--mode" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    portcellar_core::PortCellarError::Message(
                        "--mode requires steam, native, direct, wine-steam, or wine-direct"
                            .to_string(),
                    )
                })?;
                mode = parse_mode(value)?;
            }
            value if value.starts_with("--mode=") => {
                mode = parse_mode(value.trim_start_matches("--mode="))?;
            }
            value => {
                return Err(portcellar_core::PortCellarError::Message(format!(
                    "unknown launch option: {value}"
                )));
            }
        }
        index += 1;
    }

    if wait && !matches!(mode, LaunchMode::WineSteam | LaunchMode::WineDirect) {
        return Err(portcellar_core::PortCellarError::Message(
            "--wait is only supported with --mode wine-steam or wine-direct".to_string(),
        ));
    }
    if wait_login && mode != LaunchMode::WineSteam {
        return Err(portcellar_core::PortCellarError::Message(
            "--wait-login is only supported with --mode wine-steam".to_string(),
        ));
    }
    if legacy_login && !wait_login {
        return Err(portcellar_core::PortCellarError::Message(
            "--legacy-login is only used with --wait-login".to_string(),
        ));
    }
    if capture_wine_log && !matches!(mode, LaunchMode::WineSteam | LaunchMode::WineDirect) {
        return Err(portcellar_core::PortCellarError::Message(
            "--capture-wine-log is only supported with --mode wine-steam or wine-direct"
                .to_string(),
        ));
    }

    if wait {
        detach = true;
    }
    if capture_wine_log {
        detach = true;
    }

    if matches!(mode, LaunchMode::WineSteam | LaunchMode::WineDirect) {
        run_and_print_wine_configure(dry_run)?;
    }
    if mode == LaunchMode::WineSteam {
        run_and_print_steam_cef_patch(dry_run, true, None)?;
    }
    if matches!(mode, LaunchMode::WineSteam | LaunchMode::WineDirect) {
        run_and_print_isaac_profile(dry_run)?;
    }

    if mode == LaunchMode::WineSteam && wait_login {
        let login_plan = wine_steam_login_plan(legacy_login)?;
        println!("{}", login_plan.display());
        if !dry_run {
            let pid = run_plan_detached(&login_plan)?;
            println!("detached login pid: {pid}");
            wait_for_wine_login(login_timeout)?;
        }
    }

    let plan = isaac_launch_plan(mode)?;
    println!("{}", plan.display());

    if !dry_run {
        if detach {
            let pid = run_launch_plan_detached(&plan, capture_wine_log)?;
            println!("detached pid: {pid}");
            if wait {
                wait_for_wine_isaac(launch_timeout, stable_duration)?;
            }
        } else {
            let code = run_plan(&plan)?;
            if code != 0 {
                return Err(portcellar_core::PortCellarError::Message(format!(
                    "launch command exited with {code}"
                )));
            }
        }
    }

    Ok(())
}

fn game_launch(args: &[String]) -> Result<()> {
    let mut app_id = None;
    let mut from_analysis = None;
    let mut profile_source = game_support::ProfileSource::default();
    let mut name = None;
    let mut windows_exe = None;
    let mut launch_arguments = Vec::new();
    let mut install_dir_hint = None;
    let mut windows_install_path = None;
    let mut wine_engine_path = None;
    let mut steam_integration = SteamIntegration::Required;
    let mut mode = LaunchMode::WineSteam;
    let mut dry_run = false;
    let mut detach = false;
    let mut wait = false;
    let mut wait_login = false;
    let mut legacy_login = false;
    let mut capture_wine_log = false;
    let mut login_timeout = Duration::from_secs(600);
    let mut launch_timeout = Duration::from_secs(90);
    let mut stable_duration = Duration::from_secs(10);
    let mut index = 0;

    while index < args.len() {
        if profile_source.parse_arg(args, &mut index)? {
            index += 1;
            continue;
        }
        match args[index].as_str() {
            "--dry-run" => dry_run = true,
            "--detach" => detach = true,
            "--wait" => wait = true,
            "--wait-login" => wait_login = true,
            "--legacy-login" => legacy_login = true,
            "--capture-wine-log" => capture_wine_log = true,
            "--no-steam" => steam_integration = SteamIntegration::None,
            "--login-timeout-seconds" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    portcellar_core::PortCellarError::Message(
                        "--login-timeout-seconds requires a number".to_string(),
                    )
                })?;
                login_timeout = parse_timeout(value)?;
            }
            value if value.starts_with("--login-timeout-seconds=") => {
                login_timeout =
                    parse_timeout(value.trim_start_matches("--login-timeout-seconds="))?;
            }
            "--from-analysis" => {
                index += 1;
                from_analysis = args.get(index).cloned();
            }
            value if value.starts_with("--from-analysis=") => {
                from_analysis = Some(value.trim_start_matches("--from-analysis=").to_string());
            }
            "--launch-timeout-seconds" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    portcellar_core::PortCellarError::Message(
                        "--launch-timeout-seconds requires a number".to_string(),
                    )
                })?;
                launch_timeout = parse_timeout(value)?;
            }
            value if value.starts_with("--launch-timeout-seconds=") => {
                launch_timeout =
                    parse_timeout(value.trim_start_matches("--launch-timeout-seconds="))?;
            }
            "--stable-seconds" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    portcellar_core::PortCellarError::Message(
                        "--stable-seconds requires a number".to_string(),
                    )
                })?;
                stable_duration = parse_timeout(value)?;
            }
            value if value.starts_with("--stable-seconds=") => {
                stable_duration = parse_timeout(value.trim_start_matches("--stable-seconds="))?;
            }
            "--app-id" => {
                index += 1;
                app_id = args.get(index).cloned();
            }
            value if value.starts_with("--app-id=") => {
                app_id = Some(value.trim_start_matches("--app-id=").to_string());
            }
            "--name" => {
                index += 1;
                name = args.get(index).cloned();
            }
            value if value.starts_with("--name=") => {
                name = Some(value.trim_start_matches("--name=").to_string());
            }
            "--exe" => {
                index += 1;
                windows_exe = args.get(index).cloned();
            }
            value if value.starts_with("--exe=") => {
                windows_exe = Some(value.trim_start_matches("--exe=").to_string());
            }
            "--launch-arg" => {
                index += 1;
                launch_arguments.push(args.get(index).cloned().ok_or_else(|| {
                    portcellar_core::PortCellarError::Message(
                        "--launch-arg requires a value".to_string(),
                    )
                })?);
            }
            value if value.starts_with("--launch-arg=") => {
                launch_arguments.push(value.trim_start_matches("--launch-arg=").to_string());
            }
            "--install-dir" => {
                index += 1;
                install_dir_hint = args.get(index).cloned();
            }
            value if value.starts_with("--install-dir=") => {
                install_dir_hint = Some(value.trim_start_matches("--install-dir=").to_string());
            }
            "--windows-path" => {
                index += 1;
                windows_install_path = args.get(index).cloned();
            }
            value if value.starts_with("--windows-path=") => {
                windows_install_path =
                    Some(value.trim_start_matches("--windows-path=").to_string());
            }
            "--wine-engine" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    portcellar_core::PortCellarError::Message(
                        "--wine-engine requires a path".to_string(),
                    )
                })?;
                wine_engine_path = Some(PathBuf::from(value));
            }
            value if value.starts_with("--wine-engine=") => {
                wine_engine_path = Some(PathBuf::from(value.trim_start_matches("--wine-engine=")));
            }
            "--mode" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    portcellar_core::PortCellarError::Message(
                        "--mode requires a launch mode".to_string(),
                    )
                })?;
                mode = parse_mode(value)?;
            }
            value if value.starts_with("--mode=") => {
                mode = parse_mode(value.trim_start_matches("--mode="))?;
            }
            value => {
                return Err(portcellar_core::PortCellarError::Message(format!(
                    "unknown game launch option: {value}"
                )));
            }
        }
        index += 1;
    }

    let source_count = [
        from_analysis.is_some(),
        profile_source.from_profile.is_some(),
        profile_source.from_catalog.is_some(),
    ]
    .into_iter()
    .filter(|value| *value)
    .count();
    if source_count > 1 {
        return Err(portcellar_core::PortCellarError::Message(
            "--from-catalog, --from-profile, and --from-analysis cannot be combined".to_string(),
        ));
    }
    profile_source.validate("game launch", false)?;

    let profile = if let Some(analysis_app_id) = from_analysis {
        if steam_integration == SteamIntegration::None {
            return Err(portcellar_core::PortCellarError::Message(
                "--from-analysis uses a Steam manifest and cannot be combined with --no-steam"
                    .to_string(),
            ));
        }
        if app_id.is_some()
            || name.is_some()
            || windows_exe.is_some()
            || !launch_arguments.is_empty()
            || install_dir_hint.is_some()
            || windows_install_path.is_some()
            || wine_engine_path.is_some()
        {
            return Err(portcellar_core::PortCellarError::Message(
                "--from-analysis cannot be combined with manual profile fields".to_string(),
            ));
        }
        let analysis = analyze_game(&analysis_app_id)?;
        let candidate = analysis.to_generic_profile_candidate()?;
        for item in &candidate.review_items {
            println!("profile review: {item}");
        }
        candidate.profile
    } else if profile_source.from_profile.is_some() {
        if steam_integration == SteamIntegration::None {
            return Err(portcellar_core::PortCellarError::Message(
                "--from-profile cannot be combined with --no-steam; set steam_integration in the profile document"
                    .to_string(),
            ));
        }
        if app_id.is_some()
            || name.is_some()
            || windows_exe.is_some()
            || !launch_arguments.is_empty()
            || install_dir_hint.is_some()
            || windows_install_path.is_some()
            || wine_engine_path.is_some()
        {
            return Err(portcellar_core::PortCellarError::Message(
                "--from-profile cannot be combined with manual profile fields".to_string(),
            ));
        }
        let profile = profile_source.load("game launch")?;
        println!(
            "profile loaded: {} ({})",
            profile.name(),
            profile.runtime_profile_version()
        );
        profile
    } else if profile_source.from_catalog.is_some() {
        if steam_integration == SteamIntegration::None {
            return Err(portcellar_core::PortCellarError::Message(
                "--from-catalog cannot be combined with --no-steam; set steam_integration in the profile document"
                    .to_string(),
            ));
        }
        if app_id.is_some()
            || name.is_some()
            || windows_exe.is_some()
            || !launch_arguments.is_empty()
            || install_dir_hint.is_some()
            || windows_install_path.is_some()
            || wine_engine_path.is_some()
        {
            return Err(portcellar_core::PortCellarError::Message(
                "--from-catalog cannot be combined with manual profile fields".to_string(),
            ));
        }
        let profile = profile_source.load("game launch")?;
        println!(
            "profile loaded: {} ({})",
            profile.name(),
            profile.runtime_profile_version()
        );
        profile
    } else {
        let app_id = app_id.unwrap_or_else(|| "local".to_string());
        if steam_integration != SteamIntegration::None
            && (app_id.is_empty() || !app_id.chars().all(|ch| ch.is_ascii_digit()))
        {
            return Err(portcellar_core::PortCellarError::Message(
                "Steam game launch requires a numeric app id".to_string(),
            ));
        }
        let windows_exe = windows_exe.ok_or_else(|| {
            portcellar_core::PortCellarError::Message(
                "game launch requires --exe GAME.EXE".to_string(),
            )
        })?;
        let name = name.unwrap_or_else(|| {
            if steam_integration == SteamIntegration::None {
                "Standalone Windows app".to_string()
            } else {
                format!("Steam app {app_id}")
            }
        });
        let mut profile = GenericGameProfile::new(&app_id, &name, &windows_exe);
        profile = profile.with_steam_integration(steam_integration);
        if let Some(install_dir_hint) = install_dir_hint {
            profile = profile.with_install_dir_hint(install_dir_hint);
        }
        if let Some(windows_install_path) = windows_install_path {
            profile = profile.with_windows_install_path(windows_install_path);
        }
        if let Some(wine_engine_path) = wine_engine_path {
            profile = profile.with_wine_engine_path(wine_engine_path);
        }
        for argument in launch_arguments {
            profile = profile.with_launch_argument(argument);
        }
        profile
    };
    if profile.steam_integration() == SteamIntegration::None
        && profile.windows_install_path_hint().is_none()
    {
        return Err(portcellar_core::PortCellarError::Message(
            "no-steam requires --windows-path C:\\Games\\Example".to_string(),
        ));
    }
    if profile.steam_integration() == SteamIntegration::None && mode != LaunchMode::WineDirect {
        return Err(portcellar_core::PortCellarError::Message(
            "no-steam currently supports only --mode wine-direct".to_string(),
        ));
    }
    if (wait || capture_wine_log) && !matches!(mode, LaunchMode::WineSteam | LaunchMode::WineDirect)
    {
        return Err(portcellar_core::PortCellarError::Message(
            "--wait and --capture-wine-log require a Wine launch mode".to_string(),
        ));
    }
    if wait_login && profile.steam_integration() != SteamIntegration::Required {
        return Err(portcellar_core::PortCellarError::Message(
            "--wait-login requires a profile with Required Steam integration".to_string(),
        ));
    }
    if wait_login && !matches!(mode, LaunchMode::WineSteam | LaunchMode::WineDirect) {
        return Err(portcellar_core::PortCellarError::Message(
            "--wait-login requires a Wine launch mode".to_string(),
        ));
    }
    let preflight = inspect_game_runtime_preflight(&profile);
    for warning in &preflight.warnings {
        println!("runtime warning: {warning}");
    }
    if !preflight.ready() {
        return Err(portcellar_core::PortCellarError::Message(format!(
            "runtime preflight failed for {}: {}",
            profile.name(),
            preflight.blockers.join("; ")
        )));
    }

    if matches!(mode, LaunchMode::WineSteam | LaunchMode::WineDirect) {
        // Validate the launch before staging, then resolve it again below so
        // execution uses the prepared copy rather than the original files.
        game_launch_plan(&profile, mode)?;
        if !dry_run {
            prepare_game_runtime_stage(&profile)?;
        }
        run_and_print_wine_configure_for(dry_run, &profile)?;
        run_and_print_game_profile(dry_run, &profile)?;
    }
    if mode == LaunchMode::WineSteam {
        run_and_print_steam_cef_patch(dry_run, true, Some(&profile))?;
    }

    let login_plan = if wait_login {
        Some(wine_steam_login_plan_for(&profile, legacy_login)?)
    } else {
        None
    };
    if let Some(login_plan) = &login_plan {
        println!("Steam login plan: {}", login_plan.display());
    }

    let plan = game_launch_plan(&profile, mode)?;
    println!("{}", plan.display());
    if !dry_run {
        if let Some(login_plan) = &login_plan {
            let runtime = inspect_game_runtime(&profile);
            if portcellar_core::steam_session_ready(&runtime) {
                println!("Steam session: ready");
            } else {
                let pid = run_plan_detached(login_plan)?;
                println!("detached Steam login pid: {pid}");
                game_observer::wait_for_wine_login(&profile, login_timeout)?;
            }
        }
        if wait {
            let pid =
                game_observer::run_game_launch_plan_detached(&profile, &plan, capture_wine_log)?;
            println!("detached pid: {pid}");
            game_observer::wait_for_game(&profile, mode, launch_timeout, stable_duration)?;
        } else if detach {
            println!(
                "detached pid: {}",
                game_observer::run_game_launch_plan_detached(&profile, &plan, capture_wine_log,)?
            );
        } else {
            let code = run_plan(&plan)?;
            if code != 0 {
                return Err(portcellar_core::PortCellarError::Message(format!(
                    "launch command exited with {code}"
                )));
            }
        }
    }

    Ok(())
}

fn isaac_run(args: &[String]) -> Result<()> {
    let mut dry_run = false;
    let mut legacy_login = false;
    let mut capture_wine_log = false;
    let mut login_timeout = Duration::from_secs(600);
    let mut launch_timeout = Duration::from_secs(90);
    let mut stable_duration = Duration::from_secs(10);
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--dry-run" => dry_run = true,
            "--legacy-login" => legacy_login = true,
            "--capture-wine-log" => capture_wine_log = true,
            "--login-timeout-seconds" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    portcellar_core::PortCellarError::Message(
                        "--login-timeout-seconds requires a number".to_string(),
                    )
                })?;
                login_timeout = parse_timeout(value)?;
            }
            value if value.starts_with("--login-timeout-seconds=") => {
                login_timeout =
                    parse_timeout(value.trim_start_matches("--login-timeout-seconds="))?;
            }
            "--launch-timeout-seconds" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    portcellar_core::PortCellarError::Message(
                        "--launch-timeout-seconds requires a number".to_string(),
                    )
                })?;
                launch_timeout = parse_timeout(value)?;
            }
            value if value.starts_with("--launch-timeout-seconds=") => {
                launch_timeout =
                    parse_timeout(value.trim_start_matches("--launch-timeout-seconds="))?;
            }
            "--stable-seconds" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    portcellar_core::PortCellarError::Message(
                        "--stable-seconds requires a number".to_string(),
                    )
                })?;
                stable_duration = parse_timeout(value)?;
            }
            value if value.starts_with("--stable-seconds=") => {
                stable_duration = parse_timeout(value.trim_start_matches("--stable-seconds="))?;
            }
            value => {
                return Err(portcellar_core::PortCellarError::Message(format!(
                    "unknown run option: {value}"
                )));
            }
        };
        index += 1;
    }

    run_and_print_wine_configure(dry_run)?;
    run_and_print_steam_cef_patch(dry_run, true, None)?;
    run_and_print_isaac_profile(dry_run)?;

    let login_plan = wine_steam_login_plan(legacy_login)?;
    let launch_plan = isaac_launch_plan(LaunchMode::WineSteam)?;

    if dry_run {
        println!("{}", login_plan.display());
        println!("{}", launch_plan.display());
        return Ok(());
    }

    let report = doctor_report();
    if portcellar_core::steam_session_ready(&report.wine_steam) {
        println!("Steam session: ready");
    } else {
        println!("{}", login_plan.display());
        let pid = run_plan_detached(&login_plan)?;
        println!("detached login pid: {pid}");
        wait_for_wine_login(login_timeout)?;
    }

    println!("{}", launch_plan.display());
    let pid = run_launch_plan_detached(&launch_plan, capture_wine_log)?;
    println!("detached launch pid: {pid}");
    wait_for_wine_isaac(launch_timeout, stable_duration)
}

fn isaac_play(args: &[String]) -> Result<()> {
    let mut defaults = vec![
        "--capture-wine-log".to_string(),
        "--launch-timeout-seconds".to_string(),
        "300".to_string(),
        "--stable-seconds".to_string(),
        "45".to_string(),
    ];
    defaults.extend(args.iter().cloned());
    isaac_run(&defaults)
}

fn run_launch_plan_detached(
    plan: &portcellar_core::CommandPlan,
    capture_wine_log: bool,
) -> Result<u32> {
    if !capture_wine_log {
        return run_plan_detached(plan);
    }

    let log_path = isaac_wine_log_path();
    println!("capturing Wine launch log: {}", log_path.display());
    run_plan_detached_with_log(plan, &log_path)
}

fn run_and_print_isaac_profile(dry_run: bool) -> Result<()> {
    let plan = isaac_runtime_profile_plan()?;
    println!("Isaac runtime profile: {}", plan.options_path.display());
    for (key, value) in &plan.values {
        println!("  {key}={value}");
    }
    if !dry_run {
        apply_isaac_runtime_profile()?;
    }
    Ok(())
}

fn run_and_print_game_profile(dry_run: bool, profile: &dyn GameProfile) -> Result<()> {
    if profile.runtime_policy().runtime_options.is_empty() {
        return Ok(());
    }

    let plan = game_runtime_profile_plan(profile)?;
    println!(
        "{} runtime profile: {}",
        profile.name(),
        plan.options_path.display()
    );
    for (key, value) in &plan.values {
        println!("  {key}={value}");
    }
    if !dry_run {
        apply_game_runtime_profile(profile)?;
    }
    Ok(())
}

fn isaac_wine_log_path() -> PathBuf {
    doctor_report()
        .wine_steam
        .prefix
        .join("drive_c/Steam/logs/portcellar-isaac-launch.log")
}

fn wait_for_wine_isaac(timeout: Duration, stable_duration: Duration) -> Result<()> {
    game_observer::wait_for_game(
        &isaac_profile(),
        LaunchMode::WineSteam,
        timeout,
        stable_duration,
    )
}

fn configure_wine(args: &[String]) -> Result<()> {
    let mut dry_run = false;

    for value in args {
        match value.as_str() {
            "--dry-run" => dry_run = true,
            value => {
                return Err(portcellar_core::PortCellarError::Message(format!(
                    "unknown configure-wine option: {value}"
                )));
            }
        }
    }

    run_and_print_wine_configure(dry_run)
}

fn parse_mode(value: &str) -> Result<LaunchMode> {
    match value {
        "steam" => Ok(LaunchMode::Steam),
        "native" => Ok(LaunchMode::Native),
        "direct" | "native-direct" => Ok(LaunchMode::Direct),
        "wine-steam" | "wine" => Ok(LaunchMode::WineSteam),
        "wine-direct" => Ok(LaunchMode::WineDirect),
        _ => Err(portcellar_core::PortCellarError::Message(format!(
            "unsupported launch mode: {value}"
        ))),
    }
}

fn steam_login(args: &[String]) -> Result<()> {
    let mut dry_run = false;
    let mut detach = false;
    let mut wait = false;
    let mut legacy_login = false;
    let mut timeout = Duration::from_secs(600);
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--dry-run" => dry_run = true,
            "--detach" => detach = true,
            "--wait" => wait = true,
            "--legacy-login" => legacy_login = true,
            "--timeout-seconds" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    portcellar_core::PortCellarError::Message(
                        "--timeout-seconds requires a number".to_string(),
                    )
                })?;
                timeout = parse_timeout(value)?;
            }
            value if value.starts_with("--timeout-seconds=") => {
                timeout = parse_timeout(value.trim_start_matches("--timeout-seconds="))?;
            }
            value => {
                return Err(portcellar_core::PortCellarError::Message(format!(
                    "unknown steam-login option: {value}"
                )));
            }
        };
        index += 1;
    }

    if wait {
        detach = true;
    }

    run_and_print_wine_configure(dry_run)?;
    run_and_print_steam_cef_patch(dry_run, true, None)?;

    let plan = wine_steam_login_plan(legacy_login)?;
    println!("{}", plan.display());

    if !dry_run {
        if detach {
            let pid = run_plan_detached(&plan)?;
            println!("detached pid: {pid}");
            if wait {
                wait_for_wine_login(timeout)?;
            }
        } else {
            let code = run_plan(&plan)?;
            if code != 0 {
                return Err(portcellar_core::PortCellarError::Message(format!(
                    "steam-login command exited with {code}"
                )));
            }
        }
    }

    Ok(())
}

fn wait_for_wine_login(timeout: Duration) -> Result<()> {
    let started = Instant::now();
    println!("waiting for Steam session...");

    while started.elapsed() < timeout {
        let report = doctor_report();
        if portcellar_core::steam_session_ready(&report.wine_steam) {
            println!("Steam session: ok");
            return Ok(());
        }
        std::thread::sleep(Duration::from_secs(2));
    }

    let report = doctor_report();
    Err(portcellar_core::PortCellarError::Message(format!(
        "Steam session did not become active; active session: {}; login: {}; connection: {}; CEF: {}; action: {}",
        report
            .wine_steam
            .active_session
            .map(|value| if value { "yes" } else { "no" })
            .unwrap_or("unknown"),
        report
            .wine_steam
            .login_status
            .as_deref()
            .unwrap_or("unknown"),
        report
            .wine_steam
            .connection_status
            .as_deref()
            .unwrap_or("unknown"),
        report.wine_steam.cef_status.as_deref().unwrap_or("unknown"),
        steam_login_action(&report)
    )))
}

fn steam_login_action(report: &portcellar_core::DoctorReport) -> &'static str {
    steam_login_action_for_runtime(&report.wine_steam)
}

fn steam_login_action_for_runtime(runtime: &portcellar_core::WineSteamRuntime) -> &'static str {
    let login = runtime.login_status.as_deref().unwrap_or("unknown");
    let connection = runtime.connection_status.as_deref().unwrap_or("unknown");
    let cef = runtime.cef_status.as_deref().unwrap_or("unknown");
    let login_window_ready = cef.starts_with("login-window-ready");
    let transport_ready = matches!(
        connection,
        "cm-transport-ready" | "webui-transport-ready" | "internet-connected"
    );

    if runtime.active_session == Some(false) && login_window_ready {
        return "complete Steam sign-in in the Wine window; cached login status is not an active session";
    }
    if login == "waiting-for-credentials" && transport_ready && login_window_ready {
        "enter Steam credentials or approve QR login in the Wine window"
    } else {
        "run portcellar isaac doctor and inspect login, connection, and CEF status"
    }
}

fn parse_timeout(value: &str) -> Result<Duration> {
    let seconds = value.parse::<u64>().map_err(|_| {
        portcellar_core::PortCellarError::Message(format!("invalid timeout seconds: {value}"))
    })?;
    Ok(Duration::from_secs(seconds))
}

fn parse_u64(value: &str, name: &str) -> Result<u64> {
    value
        .parse::<u64>()
        .map_err(|_| portcellar_core::PortCellarError::Message(format!("invalid {name}: {value}")))
}

fn steam_stop(args: &[String]) -> Result<()> {
    let mut dry_run = false;

    for value in args {
        match value.as_str() {
            "--dry-run" => dry_run = true,
            value => {
                return Err(portcellar_core::PortCellarError::Message(format!(
                    "unknown steam-stop option: {value}"
                )));
            }
        }
    }

    let plan = wine_steam_stop_plan()?;
    println!("{}", plan.display());
    if !dry_run {
        let code = run_plan(&plan)?;
        if code == 1 {
            println!("no active Wine server for this prefix");
        } else if code != 0 {
            return Err(portcellar_core::PortCellarError::Message(format!(
                "steam-stop command exited with {code}"
            )));
        }
        let killed = kill_wine_steam_processes()?;
        if !killed.is_empty() {
            println!("terminated {} orphan Wine Steam process(es)", killed.len());
        }
    }

    Ok(())
}

fn steam_reset_session(args: &[String]) -> Result<()> {
    let mut dry_run = false;

    for value in args {
        match value.as_str() {
            "--dry-run" => dry_run = true,
            value => {
                return Err(portcellar_core::PortCellarError::Message(format!(
                    "unknown steam-reset-session option: {value}"
                )));
            }
        }
    }

    let plan = wine_steam_session_reset_plan()?;
    println!("backup dir: {}", plan.backup_dir.display());
    if plan.targets.is_empty() {
        println!("nothing to reset");
        return Ok(());
    }

    for target in &plan.targets {
        println!(
            "move {} -> {}",
            target.source.display(),
            target.backup.display()
        );
    }

    if !dry_run {
        let moved = apply_steam_session_reset(&plan)?;
        println!("moved {} item(s)", moved.len());
    }

    Ok(())
}

fn steam_patch_cef(args: &[String]) -> Result<()> {
    let mut dry_run = false;

    for value in args {
        match value.as_str() {
            "--dry-run" => dry_run = true,
            value => {
                return Err(portcellar_core::PortCellarError::Message(format!(
                    "unknown steam-patch-cef option: {value}"
                )));
            }
        }
    }

    run_and_print_steam_cef_patch(dry_run, false, None)
}

fn run_and_print_steam_cef_patch(
    dry_run: bool,
    only_if_needed: bool,
    profile: Option<&dyn GameProfile>,
) -> Result<()> {
    let plan = steam_cef_plan(profile)?;
    let needs_patch = plan
        .cef_targets
        .iter()
        .any(|target| !target.already_patched)
        || !plan.htmlcache_locks.is_empty();

    if only_if_needed && !needs_patch {
        println!("Steam CEF patch: up to date");
        return Ok(());
    }

    println!("Steam dir: {}", plan.steam_dir.display());
    if plan.cef_targets.is_empty() {
        println!("no Steam CEF helper targets found");
    } else {
        for target in &plan.cef_targets {
            println!(
                "patch {} ({:?}, already patched: {})",
                target.target.display(),
                target.arch,
                target.already_patched
            );
            println!("  preserve real helper at {}", target.real.display());
        }
    }

    for lock in &plan.htmlcache_locks {
        println!("remove htmlcache lock {}", lock.display());
    }

    if !dry_run {
        let result = apply_steam_cef_patch(&plan)?;
        println!("patched {} helper(s)", result.patched_targets.len());
        println!("removed {} htmlcache lock(s)", result.removed_locks.len());
    }

    Ok(())
}

fn steam_cef_plan(profile: Option<&dyn GameProfile>) -> Result<portcellar_core::SteamCefPatchPlan> {
    match profile {
        Some(profile) => wine_steam_cef_patch_plan_for(profile),
        None => wine_steam_cef_patch_plan(),
    }
}

fn run_and_print_wine_configure(dry_run: bool) -> Result<()> {
    let plans = wine_steam_configure_plans()?;
    run_wine_configure_plans(dry_run, plans)
}

fn run_and_print_wine_configure_for(
    dry_run: bool,
    profile: &dyn portcellar_core::GameProfile,
) -> Result<()> {
    let plans = wine_steam_configure_plans_for(profile)?;
    run_wine_configure_plans(dry_run, plans)
}

fn run_wine_configure_plans(dry_run: bool, plans: Vec<portcellar_core::CommandPlan>) -> Result<()> {
    for plan in plans {
        println!("{}", plan.display());
        if dry_run {
            continue;
        }
        let code = run_plan(&plan)?;
        if code != 0 {
            return Err(portcellar_core::PortCellarError::Message(format!(
                "configure-wine command exited with {code}"
            )));
        }
    }

    Ok(())
}

fn pass_fail(value: bool) -> &'static str {
    if value {
        "ok"
    } else {
        "missing"
    }
}
