use portcellar_core::{
    load_generic_game_profile, run_plan, GenericGameProfile, GenericGameProfileCatalog, Result,
};
use std::path::PathBuf;

#[derive(Debug, Default)]
pub(crate) struct ProfileSource {
    pub(crate) from_catalog: Option<String>,
    pub(crate) from_profile: Option<PathBuf>,
    pub(crate) profile_root: Option<PathBuf>,
}

impl ProfileSource {
    pub(crate) fn parse_arg(&mut self, args: &[String], index: &mut usize) -> Result<bool> {
        match args[*index].as_str() {
            "--from-catalog" => {
                *index += 1;
                self.from_catalog = Some(args.get(*index).cloned().ok_or_else(|| {
                    portcellar_core::PortCellarError::Message(
                        "--from-catalog requires an app id".to_string(),
                    )
                })?);
                Ok(true)
            }
            value if value.starts_with("--from-catalog=") => {
                self.from_catalog = Some(value.trim_start_matches("--from-catalog=").to_string());
                Ok(true)
            }
            "--from-profile" => {
                *index += 1;
                self.from_profile = Some(PathBuf::from(args.get(*index).ok_or_else(|| {
                    portcellar_core::PortCellarError::Message(
                        "--from-profile requires a TOML path".to_string(),
                    )
                })?));
                Ok(true)
            }
            value if value.starts_with("--from-profile=") => {
                self.from_profile =
                    Some(PathBuf::from(value.trim_start_matches("--from-profile=")));
                Ok(true)
            }
            "--profile-root" => {
                *index += 1;
                self.profile_root = Some(PathBuf::from(args.get(*index).ok_or_else(|| {
                    portcellar_core::PortCellarError::Message(
                        "--profile-root requires a path".to_string(),
                    )
                })?));
                Ok(true)
            }
            value if value.starts_with("--profile-root=") => {
                self.profile_root =
                    Some(PathBuf::from(value.trim_start_matches("--profile-root=")));
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    pub(crate) fn validate(&self, command_name: &str, require_source: bool) -> Result<()> {
        if require_source && self.from_catalog.is_some() == self.from_profile.is_some() {
            return Err(portcellar_core::PortCellarError::Message(format!(
                "{command_name} requires exactly one profile source"
            )));
        }
        if self.profile_root.is_some() && self.from_catalog.is_none() {
            return Err(portcellar_core::PortCellarError::Message(
                "--profile-root requires --from-catalog".to_string(),
            ));
        }
        Ok(())
    }

    pub(crate) fn load(&self, command_name: &str) -> Result<GenericGameProfile> {
        if let Some(profile_path) = &self.from_profile {
            return load_generic_game_profile(profile_path);
        }

        let catalog_app_id = self.from_catalog.as_deref().ok_or_else(|| {
            portcellar_core::PortCellarError::Message(format!(
                "{command_name} profile source is missing"
            ))
        })?;
        let root = self
            .profile_root
            .clone()
            .unwrap_or_else(crate::default_profile_catalog_root);
        let catalog = GenericGameProfileCatalog::load(&root)?;
        catalog
            .profile_for_app_id(catalog_app_id)
            .cloned()
            .ok_or_else(|| {
                portcellar_core::PortCellarError::Message(format!(
                    "profile app_id {catalog_app_id} was not found in {}",
                    root.display()
                ))
            })
    }
}

pub(crate) fn run_checked_plan(plan: &portcellar_core::CommandPlan, phase: &str) -> Result<()> {
    let exit_code = run_plan(plan)?;
    if exit_code == 0 {
        return Ok(());
    }
    Err(portcellar_core::PortCellarError::Message(format!(
        "{phase} failed with exit code {exit_code}"
    )))
}
