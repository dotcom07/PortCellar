use crate::game_support::{run_checked_plan, ProfileSource};
use portcellar_core::{
    game_installer_plan, inspect_game_runtime, run_plan, wine_bottle_snapshot_plan, GameProfile,
    Result,
};
use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Debug)]
struct InstallerRequest {
    source: ProfileSource,
    installer: PathBuf,
    installer_args: Vec<OsString>,
    snapshot_id: Option<String>,
    confirm: bool,
}

pub(crate) fn install_plan(args: &[String]) -> Result<()> {
    let request = parse_request(args, "install-plan", false, false)?;
    let profile = request.source.load("install")?;
    let plan = game_installer_plan(&profile, &request.installer, &request.installer_args)?;
    println!("installer profile: {}", profile.name());
    println!("installer: {}", plan.installer.display());
    println!("command: {}", plan.command.display());
    Ok(())
}

pub(crate) fn install(args: &[String]) -> Result<()> {
    let request = parse_request(args, "install", true, true)?;
    if !request.confirm {
        return Err(portcellar_core::PortCellarError::Message(
            "game install requires --confirm; use install-plan to review without executing"
                .to_string(),
        ));
    }
    let snapshot_id = request.snapshot_id.as_deref().ok_or_else(|| {
        portcellar_core::PortCellarError::Message(
            "game install requires --snapshot-id ID".to_string(),
        )
    })?;
    let profile = request.source.load("install")?;
    let installer_plan =
        game_installer_plan(&profile, &request.installer, &request.installer_args)?;
    let runtime = inspect_game_runtime(&profile);
    let snapshot = wine_bottle_snapshot_plan(&runtime, snapshot_id)?;

    println!("install profile: {}", profile.name());
    println!("snapshot: {}", snapshot.destination.display());
    run_checked_plan(&snapshot.prepare_command, "snapshot directory preparation")?;
    run_checked_plan(&snapshot.command, "bottle snapshot")?;

    let exit_code = run_plan(&installer_plan.command)?;
    if exit_code != 0 {
        return Err(portcellar_core::PortCellarError::Message(format!(
            "installer exited with code {exit_code}; snapshot retained at {} for rollback",
            snapshot.destination.display()
        )));
    }

    println!("installer completed successfully");
    println!("snapshot retained at {}", snapshot.destination.display());
    println!("game launch was not started automatically");
    Ok(())
}

fn parse_request(
    args: &[String],
    command_name: &str,
    allow_snapshot: bool,
    allow_confirm: bool,
) -> Result<InstallerRequest> {
    let mut source = ProfileSource::default();
    let mut installer = None;
    let mut installer_args = Vec::new();
    let mut snapshot_id = None;
    let mut confirm = false;
    let mut index = 0;

    while index < args.len() {
        if source.parse_arg(args, &mut index)? {
            index += 1;
            continue;
        }
        match args[index].as_str() {
            "--installer" => {
                index += 1;
                installer = Some(PathBuf::from(args.get(index).ok_or_else(|| {
                    portcellar_core::PortCellarError::Message(
                        "--installer requires a path".to_string(),
                    )
                })?));
            }
            value if value.starts_with("--installer=") => {
                installer = Some(PathBuf::from(value.trim_start_matches("--installer=")));
            }
            "--installer-arg" => {
                index += 1;
                installer_args.push(OsString::from(args.get(index).ok_or_else(|| {
                    portcellar_core::PortCellarError::Message(
                        "--installer-arg requires a value".to_string(),
                    )
                })?));
            }
            value if value.starts_with("--installer-arg=") => {
                installer_args.push(OsString::from(value.trim_start_matches("--installer-arg=")));
            }
            "--snapshot-id" if allow_snapshot => {
                index += 1;
                snapshot_id = Some(args.get(index).cloned().ok_or_else(|| {
                    portcellar_core::PortCellarError::Message(
                        "--snapshot-id requires an id".to_string(),
                    )
                })?);
            }
            value if allow_snapshot && value.starts_with("--snapshot-id=") => {
                snapshot_id = Some(value.trim_start_matches("--snapshot-id=").to_string());
            }
            "--confirm" if allow_confirm => {
                confirm = true;
            }
            value => {
                return Err(portcellar_core::PortCellarError::Message(format!(
                    "unknown game {command_name} option: {value}"
                )));
            }
        }
        index += 1;
    }

    source.validate(command_name, true)?;
    let installer = installer.ok_or_else(|| {
        portcellar_core::PortCellarError::Message(format!(
            "{command_name} requires --installer PATH"
        ))
    })?;

    Ok(InstallerRequest {
        source,
        installer,
        installer_args,
        snapshot_id,
        confirm,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_preserves_repeated_installer_args() {
        let request = parse_request(
            &[
                "--from-catalog".to_string(),
                "123456".to_string(),
                "--profile-root".to_string(),
                "/tmp/profiles".to_string(),
                "--installer".to_string(),
                "/tmp/setup.exe".to_string(),
                "--installer-arg".to_string(),
                "/quiet".to_string(),
                "--installer-arg=/norestart".to_string(),
                "--snapshot-id=before-install".to_string(),
                "--confirm".to_string(),
            ],
            "install",
            true,
            true,
        )
        .unwrap();

        assert_eq!(request.source.from_catalog.as_deref(), Some("123456"));
        assert_eq!(request.installer, PathBuf::from("/tmp/setup.exe"));
        assert_eq!(request.snapshot_id.as_deref(), Some("before-install"));
        assert!(request.confirm);
        assert_eq!(
            request.installer_args,
            vec![OsString::from("/quiet"), OsString::from("/norestart")]
        );
    }

    #[test]
    fn install_plan_rejects_confirm_flag() {
        let error = parse_request(
            &[
                "--from-profile".to_string(),
                "/tmp/profile.toml".to_string(),
                "--installer".to_string(),
                "/tmp/setup.exe".to_string(),
                "--confirm".to_string(),
            ],
            "install-plan",
            false,
            false,
        )
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("unknown game install-plan option"));
    }

    #[test]
    fn install_requires_confirmation_before_loading_profile() {
        let error = install(&[
            "--from-profile".to_string(),
            "/tmp/nonexistent-profile.toml".to_string(),
            "--installer".to_string(),
            "/tmp/nonexistent-setup.exe".to_string(),
            "--snapshot-id".to_string(),
            "before-install".to_string(),
        ])
        .unwrap_err();

        assert!(error.to_string().contains("requires --confirm"));
    }
}
