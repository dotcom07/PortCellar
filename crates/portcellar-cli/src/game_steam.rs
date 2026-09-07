use portcellar_core::{
    run_plan, wine_steam_install_plan, GenericGameProfile, Result, SteamCefPolicy,
};

struct SteamInstallRequest {
    app_id: String,
    name: String,
    dry_run: bool,
    confirm: bool,
}

pub(crate) fn install(args: &[String]) -> Result<()> {
    let request = parse_request(args)?;
    let profile =
        GenericGameProfile::new(request.app_id.clone(), request.name.clone(), "steam.exe")
            .with_steam_cef_policy(SteamCefPolicy::CrossOverCompatible);
    let plan = wine_steam_install_plan(&profile)?;

    println!("Steam install app: {}", request.app_id);
    println!("Steam URI: steam://install/{}", request.app_id);
    println!("command: {}", plan.display());
    if request.dry_run {
        return Ok(());
    }
    if !request.confirm {
        return Err(portcellar_core::PortCellarError::Message(
            "game steam-install requires --confirm; use --dry-run to review without executing"
                .to_string(),
        ));
    }

    let exit_code = run_plan(&plan)?;
    if exit_code != 0 {
        return Err(portcellar_core::PortCellarError::Message(format!(
            "Steam install request exited with code {exit_code}"
        )));
    }
    println!("Steam install request sent");
    Ok(())
}

fn parse_request(args: &[String]) -> Result<SteamInstallRequest> {
    let mut app_id = None;
    let mut name = "Steam app".to_string();
    let mut dry_run = false;
    let mut confirm = false;
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
            "--name" => {
                index += 1;
                name = args.get(index).cloned().ok_or_else(|| {
                    portcellar_core::PortCellarError::Message("--name requires a value".to_string())
                })?;
            }
            value if value.starts_with("--name=") => {
                name = value.trim_start_matches("--name=").to_string();
            }
            "--dry-run" => dry_run = true,
            "--confirm" => confirm = true,
            value => {
                return Err(portcellar_core::PortCellarError::Message(format!(
                    "unknown game steam-install option: {value}"
                )));
            }
        }
        index += 1;
    }

    let app_id = app_id.ok_or_else(|| {
        portcellar_core::PortCellarError::Message(
            "game steam-install requires --app-id APPID".to_string(),
        )
    })?;
    if app_id.is_empty() || !app_id.chars().all(|character| character.is_ascii_digit()) {
        return Err(portcellar_core::PortCellarError::Message(
            "Steam app_id must contain only ASCII digits".to_string(),
        ));
    }

    Ok(SteamInstallRequest {
        app_id,
        name,
        dry_run,
        confirm,
    })
}

#[cfg(test)]
mod tests {
    use super::parse_request;

    #[test]
    fn parse_install_request_requires_numeric_app_id() {
        let request = parse_request(&[
            "--app-id=123456".to_string(),
            "--name".to_string(),
            "Example Game".to_string(),
            "--dry-run".to_string(),
        ])
        .unwrap();

        assert_eq!(request.app_id, "123456");
        assert_eq!(request.name, "Example Game");
        assert!(request.dry_run);
        assert!(!request.confirm);
        assert!(parse_request(&["--app-id=not-a-number".to_string()]).is_err());
    }
}
