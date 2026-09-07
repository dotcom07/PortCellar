use super::*;
use std::collections::BTreeMap;
use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

pub fn isaac_launch_plan(mode: LaunchMode) -> Result<CommandPlan> {
    game_launch_plan(&isaac_profile(), mode)
}

pub fn game_installer_plan(
    profile: &dyn GameProfile,
    installer: &Path,
    installer_args: &[OsString],
) -> Result<GameInstallerPlan> {
    let runtime = inspect_wine_steam_runtime_for(profile);
    game_installer_plan_for(profile, &runtime, installer, installer_args)
}

pub(crate) fn game_installer_plan_for(
    profile: &dyn GameProfile,
    runtime: &WineSteamRuntime,
    installer: &Path,
    installer_args: &[OsString],
) -> Result<GameInstallerPlan> {
    if runtime.running || runtime.game_running {
        return Err(PortCellarError::Message(
            "stop Wine Steam and the game before planning an installer run".to_string(),
        ));
    }
    let wine = runtime
        .wine
        .as_ref()
        .ok_or_else(|| PortCellarError::Message("Wine was not found".to_string()))?;
    if !installer.is_file() {
        return Err(PortCellarError::Message(format!(
            "Windows installer was not found: {}",
            installer.display()
        )));
    }
    let extension = installer
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase);
    if !matches!(extension.as_deref(), Some("exe" | "msi" | "bat" | "cmd")) {
        return Err(PortCellarError::Message(
            "Windows installer must use .exe, .msi, .bat, or .cmd".to_string(),
        ));
    }

    let mut args = vec![installer.as_os_str().to_os_string()];
    args.extend(installer_args.iter().cloned());
    Ok(GameInstallerPlan {
        installer: installer.to_path_buf(),
        command: CommandPlan {
            program: wine.clone(),
            args,
            envs: game_wine_installer_env(runtime, profile),
            current_dir: installer.parent().map(Path::to_path_buf),
        },
    })
}

pub fn game_launch_plan(profile: &dyn GameProfile, mode: LaunchMode) -> Result<CommandPlan> {
    let steam = inspect_steam();

    match mode {
        LaunchMode::Steam => {
            if profile.runtime_policy().steam_integration == SteamIntegration::None {
                return Err(PortCellarError::Message(format!(
                    "{} does not declare Steam integration",
                    profile.name()
                )));
            }
            require_native_game(&steam, profile)?;
            Ok(CommandPlan {
                program: PathBuf::from("/usr/bin/open"),
                args: vec![OsString::from(format!(
                    "steam://rungameid/{}",
                    profile.app_id()
                ))],
                envs: BTreeMap::new(),
                current_dir: None,
            })
        }
        LaunchMode::Native => {
            let game = require_native_game(&steam, profile)?;
            let app_bundle = game.app_bundle.as_ref().ok_or_else(|| {
                PortCellarError::Message(format!(
                    "The macOS {} .app bundle was not found",
                    profile.name()
                ))
            })?;
            Ok(CommandPlan {
                program: PathBuf::from("/usr/bin/open"),
                args: vec![app_bundle.as_os_str().to_os_string()],
                envs: BTreeMap::new(),
                current_dir: None,
            })
        }
        LaunchMode::Direct => {
            let game = require_native_game(&steam, profile)?;
            let executable = game.executable.as_ref().ok_or_else(|| {
                PortCellarError::Message(format!(
                    "The macOS {} executable was not found",
                    profile.name()
                ))
            })?;
            let current_dir = executable.parent().map(Path::to_path_buf).ok_or_else(|| {
                PortCellarError::Message(format!("{} executable has no parent", profile.name()))
            })?;
            let mut envs = BTreeMap::new();
            envs.insert("SteamAppId".to_string(), profile.app_id().to_string());
            Ok(CommandPlan {
                program: PathBuf::from("/usr/bin/arch"),
                args: vec![
                    OsString::from("-x86_64"),
                    executable.as_os_str().to_os_string(),
                ],
                envs,
                current_dir: Some(current_dir),
            })
        }
        LaunchMode::WineSteam => {
            let runtime = inspect_wine_steam_runtime_for(profile);
            game_wine_steam_launch_plan(profile, &runtime)
        }
        LaunchMode::WineDirect => {
            let runtime = inspect_wine_steam_runtime_for(profile);
            game_wine_direct_launch_plan(profile, &runtime)
        }
    }
}

/// Compatibility alias. Both values of the former materialization flag are
/// read-only; callers must explicitly prepare state before execution.
pub fn game_launch_plan_with_stage(
    profile: &dyn GameProfile,
    mode: LaunchMode,
    _materialize_stage: bool,
) -> Result<CommandPlan> {
    game_launch_plan(profile, mode)
}

pub fn wine_steam_login_plan(legacy_login: bool) -> Result<CommandPlan> {
    wine_steam_login_plan_for(&isaac_profile(), legacy_login)
}

pub fn wine_steam_login_plan_for(
    profile: &dyn GameProfile,
    legacy_login: bool,
) -> Result<CommandPlan> {
    let runtime = inspect_wine_steam_runtime_for(profile);
    wine_steam_login_plan_for_runtime(profile, &runtime, legacy_login)
}

pub(crate) fn wine_steam_login_plan_for_runtime(
    profile: &dyn GameProfile,
    runtime: &WineSteamRuntime,
    legacy_login: bool,
) -> Result<CommandPlan> {
    let wine = runtime
        .wine
        .as_ref()
        .ok_or_else(|| PortCellarError::Message("Wine was not found".to_string()))?;
    runtime.steam_exe.as_ref().ok_or_else(|| {
        PortCellarError::Message(
            "Windows Steam was not found in the configured Wine prefix".to_string(),
        )
    })?;
    let extra = if legacy_login {
        &["-noreactlogin"][..]
    } else {
        &[][..]
    };

    Ok(CommandPlan {
        program: wine.clone(),
        args: steam_wine_args(runtime, extra),
        envs: game_wine_launch_env(runtime, profile),
        current_dir: None,
    })
}

pub fn wine_steam_install_plan(profile: &dyn GameProfile) -> Result<CommandPlan> {
    if profile.runtime_policy().steam_integration == SteamIntegration::None {
        return Err(PortCellarError::Message(format!(
            "{} does not declare Steam integration",
            profile.name()
        )));
    }
    let runtime = inspect_wine_steam_runtime_for(profile);
    let wine = runtime
        .wine
        .as_ref()
        .ok_or_else(|| PortCellarError::Message("Wine was not found".to_string()))?;
    runtime.steam_exe.as_ref().ok_or_else(|| {
        PortCellarError::Message(
            "Windows Steam was not found in the configured Wine prefix".to_string(),
        )
    })?;
    let uri = steam_install_uri(profile.app_id())?;
    let args = steam_wine_args(&runtime, &[uri.as_str()]);

    Ok(CommandPlan {
        program: wine.clone(),
        args,
        envs: game_wine_launch_env(&runtime, profile),
        current_dir: None,
    })
}

pub(crate) fn steam_install_uri(app_id: &str) -> Result<String> {
    if app_id.is_empty() || !app_id.chars().all(|character| character.is_ascii_digit()) {
        return Err(PortCellarError::Message(
            "Steam app_id must contain only ASCII digits".to_string(),
        ));
    }
    Ok(format!("steam://install/{app_id}"))
}

pub fn wine_steam_stop_plan() -> Result<CommandPlan> {
    let report = doctor_report();
    let runtime = &report.wine_steam;
    let wine = runtime
        .wine
        .as_ref()
        .ok_or_else(|| PortCellarError::Message("Wine was not found".to_string()))?;
    let wineserver = wineserver_for_wine(wine).ok_or_else(|| {
        PortCellarError::Message(format!(
            "wineserver was not found next to {}",
            wine.display()
        ))
    })?;

    Ok(CommandPlan {
        program: wineserver,
        args: vec![OsString::from("-k")],
        envs: wine_env(runtime),
        current_dir: None,
    })
}

pub(crate) fn require_native_game(
    steam: &SteamReport,
    profile: &dyn GameProfile,
) -> Result<GameInstall> {
    find_game_install(steam, profile).ok_or_else(|| {
        PortCellarError::Message(format!(
            "{} is not installed in the detected Steam libraries",
            profile.name()
        ))
    })
}

pub(crate) fn game_wine_steam_launch_plan(
    profile: &dyn GameProfile,
    runtime: &WineSteamRuntime,
) -> Result<CommandPlan> {
    if profile.runtime_policy().steam_integration == SteamIntegration::None {
        return Err(PortCellarError::Message(format!(
            "{} does not declare Steam integration",
            profile.name()
        )));
    }
    ensure_game_runtime_preflight(profile, runtime)?;
    let wine = runtime
        .wine
        .as_ref()
        .ok_or_else(|| PortCellarError::Message("Wine was not found".to_string()))?;
    runtime.steam_exe.as_ref().ok_or_else(|| {
        PortCellarError::Message(
            "Windows Steam was not found in the configured Wine prefix".to_string(),
        )
    })?;
    if runtime.game_executable.is_none() {
        return Err(PortCellarError::Message(format!(
            "Windows {} files were not found in the configured Wine prefix",
            profile.name()
        )));
    }

    let mut args = steam_wine_args(runtime, &["-silent", "-applaunch", profile.app_id()]);
    args.extend(profile.launch_arguments().into_iter().map(OsString::from));
    Ok(CommandPlan {
        program: wine.clone(),
        args,
        envs: game_wine_launch_env(runtime, profile),
        current_dir: None,
    })
}

pub(crate) fn game_wine_direct_launch_plan(
    profile: &dyn GameProfile,
    runtime: &WineSteamRuntime,
) -> Result<CommandPlan> {
    ensure_game_runtime_preflight(profile, runtime)?;
    let wine = runtime
        .wine
        .as_ref()
        .ok_or_else(|| PortCellarError::Message("Wine was not found".to_string()))?;
    let game_executable = runtime.game_executable.as_ref().ok_or_else(|| {
        PortCellarError::Message(format!(
            "Windows {} files were not found in the configured Wine prefix",
            profile.name()
        ))
    })?;
    let current_dir = game_executable
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            PortCellarError::Message(format!(
                "Windows {} executable has no parent",
                profile.name()
            ))
        })?;

    let game_argument = windows_path_for_host_path(&runtime.prefix, game_executable)
        .map(OsString::from)
        .unwrap_or_else(|| game_executable.as_os_str().to_os_string());
    let mut args = vec![game_argument];
    args.extend(profile.launch_arguments().into_iter().map(OsString::from));
    Ok(CommandPlan {
        program: wine.clone(),
        args,
        envs: game_wine_launch_env(runtime, profile),
        current_dir: Some(current_dir),
    })
}

fn ensure_game_runtime_preflight(
    profile: &dyn GameProfile,
    runtime: &WineSteamRuntime,
) -> Result<()> {
    let preflight = game_runtime_preflight(profile, runtime);
    if preflight.ready() {
        return Ok(());
    }
    Err(PortCellarError::Message(format!(
        "runtime preflight failed for {}: {}",
        profile.name(),
        preflight.blockers.join("; ")
    )))
}

pub(crate) fn steam_wine_args(runtime: &WineSteamRuntime, extra: &[&str]) -> Vec<OsString> {
    let Some(steam_exe) = runtime.steam_exe.as_ref() else {
        return Vec::new();
    };
    let configured_args = env::var_os("PORTCELLAR_STEAM_ARGS")
        .map(|value| steam_arg_tokens(&value.to_string_lossy()))
        .unwrap_or_default();
    let steam_exe_arg = windows_path_in_prefix(&runtime.prefix, steam_exe)
        .map(OsString::from)
        .unwrap_or_else(|| steam_exe.as_os_str().to_os_string());
    let desktop = runtime.virtual_desktop.as_deref();
    let desktop_arg_count = if desktop.is_some() { 2 } else { 0 };
    let single_process_arg_count = usize::from(steam_cef_single_process_enabled());
    let mut args = Vec::with_capacity(
        1 + desktop_arg_count
            + STEAM_WINE_CEF_ARGS.len()
            + single_process_arg_count
            + configured_args.len()
            + extra.len(),
    );
    if let Some(desktop) = desktop {
        args.push(OsString::from("explorer.exe"));
        args.push(OsString::from(format!(
            "/desktop={},{}",
            steam_virtual_desktop_name(),
            desktop
        )));
    }
    args.push(steam_exe_arg);
    args.extend(STEAM_WINE_CEF_ARGS.iter().map(OsString::from));
    if steam_cef_single_process_enabled() {
        args.push(OsString::from(STEAM_CEF_SINGLE_PROCESS_ARG));
    }
    args.extend(configured_args);
    args.extend(extra.iter().map(OsString::from));
    args
}

pub(crate) fn steam_cef_single_process_enabled() -> bool {
    env::var("PORTCELLAR_STEAM_CEF_SINGLE_PROCESS")
        .map(|value| env_flag_enabled(&value))
        .unwrap_or(false)
}

pub(crate) fn steam_arg_tokens(value: &str) -> Vec<OsString> {
    value.split_whitespace().map(OsString::from).collect()
}
