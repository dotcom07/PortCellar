use crate::game_support::{run_checked_plan, ProfileSource};
use portcellar_core::{
    inspect_game_runtime, wine_bottle_mutation_plan, wine_bottle_rollback_plan, GameProfile,
    GenericGameProfile, Result, RuntimeDependencyStatus,
};
#[derive(Debug)]
struct MutationRequest {
    source: ProfileSource,
    snapshot_id: String,
    confirm: bool,
}

pub(crate) fn mutation_plan(args: &[String]) -> Result<()> {
    let request = parse_request(args, "mutation-plan", false)?;
    let profile = request.source.load("mutation")?;
    let runtime = inspect_game_runtime(&profile);
    let plan = wine_bottle_mutation_plan(&profile, &runtime, &request.snapshot_id)?;

    print_plan(&profile, &plan);
    Ok(())
}

pub(crate) fn mutation(args: &[String]) -> Result<()> {
    let request = parse_request(args, "mutation", true)?;
    if !request.confirm {
        return Err(portcellar_core::PortCellarError::Message(
            "game mutation requires --confirm; use mutation-plan to review without executing"
                .to_string(),
        ));
    }

    let profile = request.source.load("mutation")?;
    let runtime = inspect_game_runtime(&profile);
    let plan = wine_bottle_mutation_plan(&profile, &runtime, &request.snapshot_id)?;
    let unavailable = unresolved_dependencies(&plan.dependencies);
    if !unavailable.is_empty() {
        return Err(portcellar_core::PortCellarError::Message(format!(
            "game mutation requires executable dependency plans; unresolved: {}",
            unavailable.join(", ")
        )));
    }

    println!("mutation profile: {}", profile.name());
    println!("prefix: {}", plan.prefix.display());
    run_checked_plan(
        &plan.snapshot.prepare_command,
        "snapshot directory preparation",
    )?;
    if let Err(error) = run_checked_plan(&plan.snapshot.command, "bottle snapshot") {
        return Err(portcellar_core::PortCellarError::Message(format!(
            "{error}; snapshot destination may be incomplete: {}",
            plan.snapshot.destination.display()
        )));
    }

    for dependency in &plan.dependencies {
        let command = dependency.command.as_ref().ok_or_else(|| {
            portcellar_core::PortCellarError::Message(format!(
                "planned dependency {:?} has no command",
                dependency.dependency
            ))
        })?;
        println!(
            "applying dependency {:?}: {}",
            dependency.dependency,
            command.display()
        );
        if let Err(error) =
            run_checked_plan(command, &format!("{:?} dependency", dependency.dependency))
        {
            return Err(portcellar_core::PortCellarError::Message(format!(
                "{error}; snapshot retained at {} for rollback",
                plan.snapshot.destination.display()
            )));
        }
    }

    println!("bottle mutation completed successfully");
    println!(
        "snapshot retained at {}",
        plan.snapshot.destination.display()
    );
    println!("game launch was not started automatically");
    Ok(())
}

pub(crate) fn rollback_plan(args: &[String]) -> Result<()> {
    let request = parse_request(args, "rollback-plan", false)?;
    let profile = request.source.load("rollback")?;
    let runtime = inspect_game_runtime(&profile);
    let plan = wine_bottle_rollback_plan(&runtime, &request.snapshot_id)?;

    print_rollback_plan(&profile, &plan);
    Ok(())
}

pub(crate) fn rollback(args: &[String]) -> Result<()> {
    let request = parse_request(args, "rollback", true)?;
    if !request.confirm {
        return Err(portcellar_core::PortCellarError::Message(
            "game rollback requires --confirm; use rollback-plan to review without executing"
                .to_string(),
        ));
    }

    let profile = request.source.load("rollback")?;
    let runtime = inspect_game_runtime(&profile);
    let plan = wine_bottle_rollback_plan(&runtime, &request.snapshot_id)?;

    println!("rollback profile: {}", profile.name());
    println!("snapshot: {}", plan.snapshot.display());
    println!("current prefix backup: {}", plan.backup.display());
    run_checked_plan(&plan.prepare_command, "rollback backup preparation")?;
    if let Err(error) = run_checked_plan(&plan.stage_command, "rollback snapshot staging") {
        return Err(portcellar_core::PortCellarError::Message(format!(
            "{error}; current prefix was not moved"
        )));
    }
    if let Err(error) = run_checked_plan(&plan.backup_command, "current prefix backup") {
        return Err(portcellar_core::PortCellarError::Message(format!(
            "{error}; staged snapshot remains at {}",
            plan.staging.display()
        )));
    }
    if let Err(error) = run_checked_plan(&plan.activate_command, "snapshot activation") {
        return Err(portcellar_core::PortCellarError::Message(format!(
            "{error}; current prefix backup is at {}, staged snapshot is at {}",
            plan.backup.display(),
            plan.staging.display()
        )));
    }

    println!("bottle rollback completed successfully");
    println!("previous prefix retained at {}", plan.backup.display());
    println!("game launch was not started automatically");
    Ok(())
}

fn parse_request(
    args: &[String],
    command_name: &str,
    allow_confirm: bool,
) -> Result<MutationRequest> {
    let mut source = ProfileSource::default();
    let mut snapshot_id = None;
    let mut confirm = false;
    let mut index = 0;

    while index < args.len() {
        if source.parse_arg(args, &mut index)? {
            index += 1;
            continue;
        }
        match args[index].as_str() {
            "--snapshot-id" => {
                index += 1;
                snapshot_id = Some(args.get(index).cloned().ok_or_else(|| {
                    portcellar_core::PortCellarError::Message(
                        "--snapshot-id requires an id".to_string(),
                    )
                })?);
            }
            value if value.starts_with("--snapshot-id=") => {
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
    let snapshot_id = snapshot_id.ok_or_else(|| {
        portcellar_core::PortCellarError::Message(format!(
            "{command_name} requires --snapshot-id ID"
        ))
    })?;

    Ok(MutationRequest {
        source,
        snapshot_id,
        confirm,
    })
}

fn print_plan(profile: &GenericGameProfile, plan: &portcellar_core::BottleMutationPlan) {
    println!("mutation profile: {}", profile.name());
    println!("prefix: {}", plan.prefix.display());
    println!(
        "snapshot prepare: {}",
        plan.snapshot.prepare_command.display()
    );
    println!("snapshot copy: {}", plan.snapshot.command.display());
    for dependency in &plan.dependencies {
        println!(
            "dependency {:?}: {:?}",
            dependency.dependency, dependency.status
        );
        if let Some(command) = &dependency.command {
            println!("  command: {}", command.display());
        }
        println!("  note: {}", dependency.note);
    }
}

fn print_rollback_plan(profile: &GenericGameProfile, plan: &portcellar_core::BottleRollbackPlan) {
    println!("rollback profile: {}", profile.name());
    println!("snapshot: {}", plan.snapshot.display());
    println!("prefix: {}", plan.prefix.display());
    println!("staging: {}", plan.staging.display());
    println!("prepare: {}", plan.prepare_command.display());
    println!("stage: {}", plan.stage_command.display());
    println!("backup: {}", plan.backup_command.display());
    println!("activate: {}", plan.activate_command.display());
}

fn unresolved_dependencies(dependencies: &[portcellar_core::RuntimeDependencyPlan]) -> Vec<String> {
    dependencies
        .iter()
        .filter_map(|dependency| {
            if dependency.status != RuntimeDependencyStatus::Planned {
                Some(format!(
                    "{:?}: {:?}",
                    dependency.dependency, dependency.status
                ))
            } else if dependency.command.is_none() {
                Some(format!(
                    "{:?}: PlannedWithoutCommand",
                    dependency.dependency
                ))
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutation_requires_confirmation_before_loading_profile() {
        let error = mutation(&[
            "--from-profile".to_string(),
            "/tmp/nonexistent-profile.toml".to_string(),
            "--snapshot-id".to_string(),
            "before-dependency".to_string(),
        ])
        .unwrap_err();

        assert!(error.to_string().contains("requires --confirm"));
    }

    #[test]
    fn mutation_plan_rejects_confirm_flag() {
        let error = parse_request(
            &[
                "--from-profile".to_string(),
                "/tmp/profile.toml".to_string(),
                "--snapshot-id".to_string(),
                "before-dependency".to_string(),
                "--confirm".to_string(),
            ],
            "mutation-plan",
            false,
        )
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("unknown game mutation-plan option"));
    }

    #[test]
    fn mutation_blocks_non_executable_dependency_plans_before_snapshot() {
        let dependencies = vec![
            portcellar_core::RuntimeDependencyPlan {
                dependency: portcellar_core::RuntimeDependency::Vcrun,
                status: RuntimeDependencyStatus::ToolMissing,
                command: None,
                note: "missing".to_string(),
            },
            portcellar_core::RuntimeDependencyPlan {
                dependency: portcellar_core::RuntimeDependency::OpenAl,
                status: RuntimeDependencyStatus::Planned,
                command: None,
                note: "provider bug".to_string(),
            },
        ];

        assert_eq!(
            unresolved_dependencies(&dependencies),
            vec![
                "Vcrun: ToolMissing".to_string(),
                "OpenAl: PlannedWithoutCommand".to_string()
            ]
        );
    }

    #[test]
    fn rollback_requires_confirmation_before_loading_profile() {
        let error = rollback(&[
            "--from-profile".to_string(),
            "/tmp/nonexistent-profile.toml".to_string(),
            "--snapshot-id".to_string(),
            "before-rollback".to_string(),
        ])
        .unwrap_err();

        assert!(error.to_string().contains("requires --confirm"));
    }
}
