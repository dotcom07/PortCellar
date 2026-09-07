use super::{GameProfile, GenericGameProfile};
use crate::{PortCellarError, Result};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct GenericGameProfileCatalogEntry {
    profile: GenericGameProfile,
    path: PathBuf,
}

impl GenericGameProfileCatalogEntry {
    pub fn profile(&self) -> &GenericGameProfile {
        &self.profile
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn app_id(&self) -> &str {
        self.profile.app_id()
    }

    pub fn name(&self) -> &str {
        self.profile.name()
    }
}

#[derive(Debug, Clone)]
pub struct GenericGameProfileCatalog {
    root: PathBuf,
    entries: Vec<GenericGameProfileCatalogEntry>,
}

impl GenericGameProfileCatalog {
    pub fn load(root: &Path) -> Result<Self> {
        if !root.exists() {
            return Ok(Self {
                root: root.to_path_buf(),
                entries: Vec::new(),
            });
        }
        if !root.is_dir() {
            return Err(PortCellarError::Message(format!(
                "profile catalog root is not a directory: {}",
                root.display()
            )));
        }

        let mut profiles = BTreeMap::new();
        for entry in fs::read_dir(root)? {
            let path = entry?.path();
            if !path.is_file() || path.extension().and_then(|value| value.to_str()) != Some("toml")
            {
                continue;
            }

            let profile = super::load_generic_game_profile(&path).map_err(|error| {
                PortCellarError::Message(format!(
                    "could not load profile {}: {error}",
                    path.display()
                ))
            })?;
            let app_id = profile.app_id().to_string();
            if profiles.contains_key(&app_id) {
                return Err(PortCellarError::Message(format!(
                    "duplicate profile app_id {app_id} in catalog {}",
                    root.display()
                )));
            }
            profiles.insert(app_id, GenericGameProfileCatalogEntry { profile, path });
        }

        Ok(Self {
            root: root.to_path_buf(),
            entries: profiles.into_values().collect(),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn entries(&self) -> &[GenericGameProfileCatalogEntry] {
        &self.entries
    }

    pub fn profile_for_app_id(&self, app_id: &str) -> Option<&GenericGameProfile> {
        self.entries
            .iter()
            .find(|entry| entry.app_id() == app_id)
            .map(GenericGameProfileCatalogEntry::profile)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "portcellar-profile-catalog-{name}-{}",
            std::process::id()
        ))
    }

    #[test]
    fn catalog_loads_profiles_in_app_id_order() {
        let root = test_root("order");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        GenericGameProfile::new("2", "Second", "Second.exe")
            .write_profile_toml(&root.join("second.toml"))
            .unwrap();
        GenericGameProfile::new("1", "First", "First.exe")
            .write_profile_toml(&root.join("first.toml"))
            .unwrap();

        let catalog = GenericGameProfileCatalog::load(&root).unwrap();

        assert_eq!(catalog.entries().len(), 2);
        assert_eq!(catalog.entries()[0].app_id(), "1");
        assert_eq!(catalog.entries()[1].app_id(), "2");
        assert_eq!(catalog.profile_for_app_id("2").unwrap().name(), "Second");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn catalog_rejects_duplicate_app_ids() {
        let root = test_root("duplicate");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        for name in ["first.toml", "second.toml"] {
            GenericGameProfile::new("123", "Example", "Example.exe")
                .write_profile_toml(&root.join(name))
                .unwrap();
        }

        let error = GenericGameProfileCatalog::load(&root).unwrap_err();

        assert!(error.to_string().contains("duplicate profile app_id 123"));
        let _ = fs::remove_dir_all(root);
    }
}
