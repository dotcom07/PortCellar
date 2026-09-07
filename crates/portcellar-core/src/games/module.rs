use super::{generic::public_module_profile, GameProfile, GenericGameProfile, SteamIntegration};
use crate::{PortCellarError, Result};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameModuleDescriptor {
    pub format_version: String,
    pub id: String,
    pub name: String,
    pub revision: u64,
    pub variants: Vec<GameModuleVariant>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameModuleVariant {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub stores: BTreeMap<String, String>,
    pub profiles: Vec<GameModuleProfileReference>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameModuleProfileReference {
    pub id: String,
    pub path: String,
}

#[derive(Debug)]
pub struct GameModule {
    pub descriptor: GameModuleDescriptor,
    profiles: BTreeMap<(String, String), GenericGameProfile>,
}

#[derive(Debug)]
pub struct GameModuleCatalog {
    pub modules: Vec<GameModule>,
}

impl GameModuleCatalog {
    /// Read and validate every module without preparing or executing any state.
    pub fn load(root: &Path) -> Result<Self> {
        let root = root.canonicalize()?;
        let mut modules = BTreeMap::new();
        for entry in fs::read_dir(&root)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_symlink() {
                return Err(invalid(format!(
                    "symlink in module catalog: {}",
                    path.display()
                )));
            }
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let module = load_module(&path)
                .map_err(|error| invalid(format!("{}: {error}", path.display())))?;
            let id = module.descriptor.id.clone();
            if modules.insert(id.clone(), module).is_some() {
                return Err(invalid(format!("duplicate module id: {id}")));
            }
        }
        Ok(Self {
            modules: modules.into_values().collect(),
        })
    }

    pub fn profile(
        &self,
        module: &str,
        variant: &str,
        profile: &str,
    ) -> Option<&GenericGameProfile> {
        self.modules
            .iter()
            .find(|entry| entry.descriptor.id == module)?
            .profiles
            .get(&(variant.to_string(), profile.to_string()))
    }
}

fn load_module(root: &Path) -> Result<GameModule> {
    let descriptor_path = confined_file(root, "module.toml")?;
    let descriptor: GameModuleDescriptor = toml::from_str(&fs::read_to_string(descriptor_path)?)
        .map_err(|error| invalid(format!("invalid module descriptor: {error}")))?;
    if descriptor.format_version != "portcellar-module-v1" {
        return Err(invalid("unsupported module format"));
    }
    slug(&descriptor.id)?;
    if root.file_name().and_then(|name| name.to_str()) != Some(&descriptor.id) {
        return Err(invalid("module id must match its directory"));
    }
    if descriptor.name.trim().is_empty()
        || descriptor.revision == 0
        || descriptor.variants.is_empty()
    {
        return Err(invalid(
            "module requires a name, positive revision and variants",
        ));
    }
    let mut variants = BTreeSet::new();
    let mut profiles = BTreeMap::new();
    for variant in &descriptor.variants {
        slug(&variant.id)?;
        if !variants.insert(&variant.id)
            || variant.name.trim().is_empty()
            || variant.profiles.is_empty()
        {
            return Err(invalid("variant requires a unique id, name and profiles"));
        }
        for (store, id) in &variant.stores {
            slug(store)?;
            if id.trim().is_empty()
                || (store == "steam" && !id.bytes().all(|byte| byte.is_ascii_digit()))
            {
                return Err(invalid(format!("invalid {store} store id")));
            }
        }
        for reference in &variant.profiles {
            slug(&reference.id)?;
            let path = confined_file(root, &reference.path)?;
            let profile = public_module_profile(&fs::read_to_string(path)?)?;
            if profile.steam_integration() == SteamIntegration::Required
                && variant.stores.get("steam").map(String::as_str) != Some(profile.app_id())
            {
                return Err(invalid(
                    "Steam-required profile must match variant stores.steam",
                ));
            }
            if profiles
                .insert((variant.id.clone(), reference.id.clone()), profile)
                .is_some()
            {
                return Err(invalid("duplicate profile id in variant"));
            }
        }
    }
    Ok(GameModule {
        descriptor,
        profiles,
    })
}

fn slug(value: &str) -> Result<()> {
    if value.split('-').any(|part| {
        part.is_empty()
            || !part
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
    }) {
        return Err(invalid(format!("invalid module identifier: {value:?}")));
    }
    Ok(())
}

fn confined_file(root: &Path, value: &str) -> Result<PathBuf> {
    if !value.ends_with(".toml")
        || value.contains(['\\', ':', '\0'])
        || value.split('/').any(|part| matches!(part, "" | "." | ".."))
    {
        return Err(invalid(format!("invalid module file path: {value:?}")));
    }
    let mut path = root.to_path_buf();
    for part in value.split('/') {
        path.push(part);
        if fs::symlink_metadata(&path)?.file_type().is_symlink() {
            return Err(invalid(format!(
                "symlink in module path: {}",
                path.display()
            )));
        }
    }
    let canonical = path.canonicalize()?;
    if !canonical.starts_with(root) || !canonical.is_file() {
        return Err(invalid(
            "module path must be a regular file within its module",
        ));
    }
    Ok(canonical)
}

fn invalid(message: impl Into<String>) -> PortCellarError {
    PortCellarError::Message(message.into())
}
