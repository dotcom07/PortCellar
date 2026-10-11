use portcellar_core::{
    GameModuleCatalog, InstallationRequest, ManagedInstallation, PortCellarError, Result,
};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    path::{Path, PathBuf},
    time::Duration,
};

pub(crate) fn installation(args: &[String]) -> Result<()> {
    let operation = args
        .first()
        .ok_or_else(|| error("installation requires an operation"))?;
    let allowed: &[&str] = match operation.as_str() {
        "create" => &[
            "--root",
            "--id",
            "--module-root",
            "--module",
            "--variant",
            "--profile",
            "--source-root",
            "--wine",
            "--dxmt-root",
        ],
        "plan" => &["--root", "--installer", "--installer-arg"],
        "prepare" => &["--root"],
        "launch" | "recover" => &["--root", "--timeout-seconds"],
        "install" => &[
            "--root",
            "--installer",
            "--installer-arg",
            "--timeout-seconds",
        ],
        "replace" => &[
            "--root",
            "--target",
            "--artifact",
            "--original-sha256",
            "--replacement-sha256",
        ],
        "restore" => &["--root", "--target"],
        _ => return Err(error("unknown installation operation")),
    };
    let mut options = BTreeMap::new();
    let mut installer_args = Vec::new();
    let mut index = 1;
    while index < args.len() {
        let (key, inline) = args[index]
            .split_once('=')
            .map_or((args[index].as_str(), None), |(key, value)| {
                (key, Some(value))
            });
        if !allowed.contains(&key) {
            return Err(error(format!(
                "unknown installation {operation} option: {key}"
            )));
        }
        let value = if let Some(value) = inline {
            value.to_string()
        } else {
            index += 1;
            args.get(index)
                .cloned()
                .ok_or_else(|| error(format!("{key} requires a value")))?
        };
        if key == "--installer-arg" {
            installer_args.push(OsString::from(value));
        } else if options.insert(key.to_string(), value).is_some() {
            return Err(error(format!("duplicate option: {key}")));
        }
        index += 1;
    }
    let required = |key: &str| -> Result<&str> {
        options
            .get(key)
            .map(String::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| error(format!("{operation} requires {key}")))
    };
    let root = Path::new(required("--root")?);
    if operation == "create" {
        let catalog_root = options
            .get("--module-root")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("modules"));
        let catalog = GameModuleCatalog::load(&catalog_root)?;
        let request = InstallationRequest {
            id: required("--id")?.into(),
            module_id: required("--module")?.into(),
            variant_id: required("--variant")?.into(),
            profile_id: required("--profile")?.into(),
            source_root: required("--source-root")?.into(),
            wine: required("--wine")?.into(),
            dxmt_root: options.get("--dxmt-root").map(PathBuf::from),
        };
        let installation = ManagedInstallation::create(root, &catalog, &request)?;
        println!("installation created: {}", installation.root().display());
        println!("prefix and staged game have not been prepared");
        return Ok(());
    }
    let timeout = options
        .get("--timeout-seconds")
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|_| error("timeout must be an integer number of seconds"))
        })
        .transpose()?
        .unwrap_or(3600);
    if timeout == 0 || timeout > 86400 {
        return Err(error("timeout must be between 1 and 86400 seconds"));
    }
    let timeout = Duration::from_secs(timeout);
    let installation = ManagedInstallation::load(root)?;
    match operation.as_str() {
        "plan" => {
            let plan = if let Some(installer) = options.get("--installer") {
                installation.installer_plan(Path::new(installer), &installer_args)?
            } else {
                if !installer_args.is_empty() {
                    return Err(error("--installer-arg requires --installer"));
                }
                installation.launch_plan()?
            };
            println!("{}", plan.display());
            println!("read-only plan; prefix, stage and recovery data were not changed");
        }
        "prepare" => {
            installation.prepare()?;
            println!("staged game prepared; game process was not launched");
        }
        "launch" => {
            checked(installation.launch(timeout)?, "launch")?;
            println!(
                "foreground launch completed and Wine server stopped; gameplay remains unverified"
            );
        }
        "install" => {
            checked(
                installation.install(
                    Path::new(required("--installer")?),
                    &installer_args,
                    timeout,
                )?,
                "installer",
            )?;
            println!("installer completed and Wine server stopped; installed files and gameplay require inspection");
        }
        "replace" => {
            let preimage = required("--original-sha256")?;
            installation.replace(
                required("--target")?,
                Path::new(required("--artifact")?),
                (preimage != "absent").then_some(preimage),
                required("--replacement-sha256")?,
            )?;
            println!("file change applied; recovery data retained");
        }
        "restore" => {
            installation.restore(required("--target")?)?;
            println!("owned file change restored; recovery data retained");
        }
        "recover" => {
            installation.recover(timeout)?;
            println!("Wine server stopped; interrupted-session marker cleared");
        }
        _ => unreachable!(),
    }
    Ok(())
}

fn error(message: impl Into<String>) -> PortCellarError {
    PortCellarError::Message(message.into())
}
fn checked(code: i32, operation: &str) -> Result<()> {
    if code != 0 {
        return Err(error(format!(
            "{operation} exited with code {code}; installation retained"
        )));
    }
    Ok(())
}
