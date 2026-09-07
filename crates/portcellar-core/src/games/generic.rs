use super::{
    validate_binary_patch_metadata, GameBinaryPatch, GameCapability, GameProfile,
    GameRuntimeArtifact, GraphicsBackend, RuntimeDependency, SteamCefPolicy, SteamIntegration,
};
use crate::{PortCellarError, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const GENERIC_GAME_PROFILE_FORMAT_VERSION: &str = "game-profile-v1";

// The public module boundary is strict; legacy direct profiles remain permissive.
pub(super) fn public_module_profile(text: &str) -> Result<GenericGameProfile> {
    let value: toml::Table = toml::from_str(text)
        .map_err(|error| PortCellarError::Message(format!("invalid module profile: {error}")))?;
    reject_unknown_fields(
        &value,
        &[
            "format_version",
            "app_id",
            "name",
            "bottle_name",
            "windows_exe",
            "launch_arguments",
            "windows_depot_id",
            "install_dir_hint",
            "windows_install_path",
            "runtime_options_path",
            "runtime_profile_version",
            "wine_engine_path",
            "wine_windows_version",
            "graphics_backend",
            "dxmt_config",
            "steam_integration",
            "steam_cef_policy",
            "runtime_options",
            "runtime_environment",
            "runtime_artifacts",
            "binary_patches",
            "capabilities",
            "dependencies",
        ],
    )?;
    for (field, keys) in [
        (
            "runtime_artifacts",
            &["id", "source_path", "target_path", "architecture"][..],
        ),
        ("binary_patches", &["target_path", "kind"][..]),
    ] {
        if let Some(records) = value.get(field).and_then(toml::Value::as_array) {
            for record in records {
                if let Some(table) = record.as_table() {
                    reject_unknown_fields(table, keys)?;
                }
            }
        }
    }
    let profile = GenericGameProfile::from_profile_toml(text)?;
    if value.contains_key("windows_install_path")
        || value.contains_key("wine_engine_path")
        || !profile.runtime_artifacts().is_empty()
    {
        return Err(PortCellarError::Message(
            "public module profiles must not contain host paths or runtime artifacts".to_string(),
        ));
    }
    Ok(profile)
}

fn reject_unknown_fields(table: &toml::Table, fields: &[&str]) -> Result<()> {
    if let Some(key) = table.keys().find(|key| !fields.contains(&key.as_str())) {
        return Err(PortCellarError::Message(format!(
            "unknown module profile field: {key}"
        )));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenericGameProfileDocument {
    pub format_version: String,
    pub app_id: String,
    pub name: String,
    pub bottle_name: String,
    pub windows_exe: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub launch_arguments: Vec<String>,
    #[serde(default)]
    pub windows_depot_id: Option<String>,
    #[serde(default)]
    pub install_dir_hint: Option<String>,
    #[serde(default)]
    pub windows_install_path: Option<String>,
    #[serde(default)]
    pub runtime_options_path: Option<String>,
    pub runtime_profile_version: String,
    #[serde(default)]
    pub wine_engine_path: Option<String>,
    pub wine_windows_version: String,
    pub graphics_backend: GraphicsBackend,
    #[serde(default)]
    pub dxmt_config: Option<String>,
    pub steam_integration: SteamIntegration,
    #[serde(default)]
    pub steam_cef_policy: SteamCefPolicy,
    #[serde(default)]
    pub runtime_options: BTreeMap<String, String>,
    #[serde(default)]
    pub runtime_environment: BTreeMap<String, String>,
    #[serde(default)]
    pub runtime_artifacts: Vec<GameRuntimeArtifact>,
    #[serde(default)]
    pub binary_patches: Vec<GameBinaryPatch>,
    #[serde(default)]
    pub capabilities: BTreeSet<GameCapability>,
    #[serde(default)]
    pub dependencies: Vec<RuntimeDependency>,
}

#[derive(Debug, Clone)]
pub struct GenericGameProfile {
    app_id: String,
    name: String,
    bottle_name: String,
    windows_exe: String,
    launch_arguments: Vec<String>,
    wine_engine_path: Option<PathBuf>,
    windows_depot_id: Option<String>,
    install_dir_hint: String,
    windows_install_path: Option<String>,
    runtime_options_path: Option<String>,
    runtime_profile_version: String,
    wine_windows_version: String,
    dxmt_config: Option<String>,
    graphics_backend: GraphicsBackend,
    steam_integration: SteamIntegration,
    steam_cef_policy: SteamCefPolicy,
    runtime_options: BTreeMap<String, String>,
    runtime_environment: BTreeMap<String, String>,
    runtime_artifacts: Vec<GameRuntimeArtifact>,
    binary_patches: Vec<GameBinaryPatch>,
    capabilities: BTreeSet<GameCapability>,
    dependencies: Vec<RuntimeDependency>,
}

impl GenericGameProfile {
    pub fn new(
        app_id: impl Into<String>,
        name: impl Into<String>,
        windows_exe: impl Into<String>,
    ) -> Self {
        let app_id = app_id.into();
        Self {
            runtime_profile_version: format!("generic-wine-steam-{app_id}"),
            bottle_name: format!("game-{}", safe_bottle_component(&app_id)),
            app_id,
            name: name.into(),
            windows_exe: windows_exe.into(),
            launch_arguments: Vec::new(),
            wine_engine_path: None,
            windows_depot_id: None,
            install_dir_hint: String::new(),
            windows_install_path: None,
            runtime_options_path: None,
            wine_windows_version: "win10".to_string(),
            dxmt_config: None,
            graphics_backend: GraphicsBackend::Auto,
            steam_integration: SteamIntegration::Required,
            steam_cef_policy: SteamCefPolicy::WineDefault,
            runtime_options: BTreeMap::new(),
            runtime_environment: BTreeMap::new(),
            runtime_artifacts: Vec::new(),
            binary_patches: Vec::new(),
            capabilities: BTreeSet::new(),
            dependencies: Vec::new(),
        }
    }

    pub fn with_install_dir_hint(mut self, value: impl Into<String>) -> Self {
        self.install_dir_hint = value.into();
        self
    }

    pub fn with_launch_argument(mut self, value: impl Into<String>) -> Self {
        self.launch_arguments.push(value.into());
        self
    }

    pub fn with_bottle_name(mut self, value: impl Into<String>) -> Self {
        self.bottle_name = safe_bottle_component(&value.into());
        self
    }

    pub fn with_runtime_profile_version(mut self, value: impl Into<String>) -> Self {
        self.runtime_profile_version = value.into();
        self
    }

    pub fn with_wine_engine_path(mut self, value: impl Into<PathBuf>) -> Self {
        self.wine_engine_path = Some(value.into());
        self
    }

    pub fn with_windows_depot_id(mut self, value: impl Into<String>) -> Self {
        self.windows_depot_id = Some(value.into());
        self
    }

    pub fn with_windows_install_path(mut self, value: impl Into<String>) -> Self {
        self.windows_install_path = Some(value.into());
        self
    }

    pub fn with_runtime_options_path(mut self, value: impl Into<String>) -> Self {
        self.runtime_options_path = Some(value.into());
        self
    }

    pub fn with_wine_windows_version(mut self, value: impl Into<String>) -> Self {
        self.wine_windows_version = value.into();
        self
    }

    pub fn with_graphics_backend(mut self, value: GraphicsBackend) -> Self {
        self.graphics_backend = value;
        self
    }

    pub fn with_dxmt_config(mut self, value: impl Into<String>) -> Self {
        self.dxmt_config = Some(value.into());
        self.graphics_backend = GraphicsBackend::Dxmt;
        self
    }

    pub fn with_steam_integration(mut self, value: SteamIntegration) -> Self {
        self.steam_integration = value;
        self
    }

    pub fn with_steam_cef_policy(mut self, value: SteamCefPolicy) -> Self {
        self.steam_cef_policy = value;
        self
    }

    pub fn with_runtime_option(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.runtime_options.insert(key.into(), value.into());
        self
    }

    pub fn with_runtime_environment(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.runtime_environment.insert(key.into(), value.into());
        self
    }

    pub fn with_runtime_artifact(mut self, value: GameRuntimeArtifact) -> Self {
        self.runtime_artifacts.push(value);
        self
    }

    pub fn with_binary_patch(mut self, value: GameBinaryPatch) -> Self {
        self.binary_patches.push(value);
        self
    }

    pub fn with_capability(mut self, capability: GameCapability) -> Self {
        self.capabilities.insert(capability);
        self
    }

    pub fn with_dependency(mut self, dependency: RuntimeDependency) -> Self {
        self.dependencies.push(dependency);
        self
    }

    pub fn to_profile_document(&self) -> GenericGameProfileDocument {
        GenericGameProfileDocument {
            format_version: GENERIC_GAME_PROFILE_FORMAT_VERSION.to_string(),
            app_id: self.app_id.clone(),
            name: self.name.clone(),
            bottle_name: self.bottle_name.clone(),
            windows_exe: self.windows_exe.clone(),
            launch_arguments: self.launch_arguments.clone(),
            windows_depot_id: self.windows_depot_id.clone(),
            install_dir_hint: (!self.install_dir_hint.is_empty())
                .then(|| self.install_dir_hint.clone()),
            windows_install_path: self.windows_install_path.clone(),
            runtime_options_path: self.runtime_options_path.clone(),
            runtime_profile_version: self.runtime_profile_version.clone(),
            wine_engine_path: self
                .wine_engine_path
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
            wine_windows_version: self.wine_windows_version.clone(),
            graphics_backend: self.graphics_backend,
            dxmt_config: self.dxmt_config.clone(),
            steam_integration: self.steam_integration,
            steam_cef_policy: self.steam_cef_policy,
            runtime_options: self.runtime_options.clone(),
            runtime_environment: self.runtime_environment.clone(),
            runtime_artifacts: self.runtime_artifacts.clone(),
            binary_patches: self.binary_patches.clone(),
            capabilities: self.capabilities.clone(),
            dependencies: self.dependencies.clone(),
        }
    }

    pub fn from_profile_document(document: GenericGameProfileDocument) -> Result<Self> {
        if document.format_version != GENERIC_GAME_PROFILE_FORMAT_VERSION {
            return Err(PortCellarError::Message(format!(
                "unsupported game profile format: {}",
                document.format_version
            )));
        }
        if document.app_id.trim().is_empty()
            || document.name.trim().is_empty()
            || document.windows_exe.trim().is_empty()
            || document.runtime_profile_version.trim().is_empty()
            || document.wine_windows_version.trim().is_empty()
        {
            return Err(PortCellarError::Message(
                "game profile identity and runtime fields must not be empty".to_string(),
            ));
        }
        if document.steam_integration != SteamIntegration::None
            && !document.app_id.chars().all(|ch| ch.is_ascii_digit())
        {
            return Err(PortCellarError::Message(
                "Steam game profiles require a numeric app_id".to_string(),
            ));
        }
        validate_windows_executable(&document.windows_exe)?;
        if document
            .launch_arguments
            .iter()
            .any(|argument| argument.contains('\0'))
        {
            return Err(PortCellarError::Message(
                "game profile launch_arguments must not contain NUL".to_string(),
            ));
        }
        if let Some(path) = document.runtime_options_path.as_deref() {
            validate_windows_absolute_path(path, "runtime_options_path")?;
        }
        if !document.runtime_options.is_empty() && document.runtime_options_path.is_none() {
            return Err(PortCellarError::Message(
                "game profile runtime_options require runtime_options_path".to_string(),
            ));
        }
        if safe_bottle_component(&document.bottle_name) != document.bottle_name {
            return Err(PortCellarError::Message(
                "game profile bottle_name must be a single safe path component".to_string(),
            ));
        }
        if document.dxmt_config.is_some() && document.graphics_backend != GraphicsBackend::Dxmt {
            return Err(PortCellarError::Message(
                "dxmt_config requires the Dxmt graphics backend".to_string(),
            ));
        }
        for artifact in &document.runtime_artifacts {
            super::validate_runtime_artifact_metadata(artifact)
                .map_err(PortCellarError::Message)?;
        }
        for patch in &document.binary_patches {
            validate_binary_patch_metadata(patch).map_err(PortCellarError::Message)?;
        }

        let mut profile = Self::new(document.app_id, document.name, document.windows_exe)
            .with_bottle_name(document.bottle_name)
            .with_runtime_profile_version(document.runtime_profile_version)
            .with_wine_windows_version(document.wine_windows_version)
            .with_graphics_backend(document.graphics_backend)
            .with_steam_integration(document.steam_integration)
            .with_steam_cef_policy(document.steam_cef_policy);
        if let Some(value) = document.windows_depot_id {
            profile = profile.with_windows_depot_id(value);
        }
        if let Some(value) = document.install_dir_hint {
            profile = profile.with_install_dir_hint(value);
        }
        if let Some(value) = document.windows_install_path {
            profile = profile.with_windows_install_path(value);
        }
        for argument in document.launch_arguments {
            profile = profile.with_launch_argument(argument);
        }
        if let Some(value) = document.runtime_options_path {
            profile = profile.with_runtime_options_path(value);
        }
        if let Some(value) = document.wine_engine_path {
            profile = profile.with_wine_engine_path(value);
        }
        if let Some(value) = document.dxmt_config {
            profile = profile.with_dxmt_config(value);
        }
        for (key, value) in document.runtime_options {
            profile = profile.with_runtime_option(key, value);
        }
        for (key, value) in document.runtime_environment {
            profile = profile.with_runtime_environment(key, value);
        }
        for artifact in document.runtime_artifacts {
            profile = profile.with_runtime_artifact(artifact);
        }
        for patch in document.binary_patches {
            profile = profile.with_binary_patch(patch);
        }
        for capability in document.capabilities {
            profile = profile.with_capability(capability);
        }
        for dependency in document.dependencies {
            profile = profile.with_dependency(dependency);
        }
        Ok(profile)
    }

    pub fn to_profile_toml(&self) -> Result<String> {
        toml::to_string_pretty(&self.to_profile_document()).map_err(|error| {
            PortCellarError::Message(format!("could not serialize game profile: {error}"))
        })
    }

    pub fn from_profile_toml(value: &str) -> Result<Self> {
        let document = toml::from_str::<GenericGameProfileDocument>(value).map_err(|error| {
            PortCellarError::Message(format!("could not parse game profile: {error}"))
        })?;
        Self::from_profile_document(document)
    }

    pub fn write_profile_toml(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, self.to_profile_toml()?)?;
        Ok(())
    }
}

impl GameProfile for GenericGameProfile {
    fn app_id(&self) -> &str {
        &self.app_id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn bottle_name(&self) -> Option<&str> {
        Some(&self.bottle_name)
    }

    fn windows_depot_id(&self) -> Option<&str> {
        self.windows_depot_id.as_deref()
    }

    fn windows_exe(&self) -> &str {
        &self.windows_exe
    }

    fn launch_arguments(&self) -> Vec<String> {
        self.launch_arguments.clone()
    }

    fn install_dir_hint(&self) -> &str {
        &self.install_dir_hint
    }

    fn windows_install_path_hint(&self) -> Option<&str> {
        self.windows_install_path.as_deref()
    }

    fn runtime_profile_version(&self) -> &str {
        &self.runtime_profile_version
    }

    fn wine_engine_path_hint(&self) -> Option<&Path> {
        self.wine_engine_path.as_deref()
    }

    fn wine_windows_version(&self) -> &str {
        &self.wine_windows_version
    }

    fn dxmt_config(&self) -> Option<&str> {
        self.dxmt_config.as_deref()
    }

    fn savedata_path_marker(&self) -> Option<&str> {
        None
    }

    fn default_documents_subdir(&self) -> Option<&str> {
        None
    }

    fn runtime_options_path_hint(&self) -> Option<&str> {
        self.runtime_options_path.as_deref()
    }

    fn runtime_options(&self) -> BTreeMap<String, String> {
        self.runtime_options.clone()
    }

    fn graphics_backend(&self) -> GraphicsBackend {
        self.graphics_backend
    }

    fn steam_integration(&self) -> SteamIntegration {
        self.steam_integration
    }

    fn steam_cef_policy(&self) -> SteamCefPolicy {
        self.steam_cef_policy
    }

    fn runtime_environment(&self) -> BTreeMap<String, String> {
        self.runtime_environment.clone()
    }

    fn runtime_artifacts(&self) -> Vec<GameRuntimeArtifact> {
        self.runtime_artifacts.clone()
    }

    fn binary_patches(&self) -> Vec<GameBinaryPatch> {
        self.binary_patches.clone()
    }

    fn capabilities(&self) -> BTreeSet<GameCapability> {
        self.capabilities.clone()
    }

    fn dependencies(&self) -> Vec<RuntimeDependency> {
        self.dependencies.clone()
    }
}

fn safe_bottle_component(value: &str) -> String {
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

fn validate_windows_executable(value: &str) -> Result<()> {
    let value = value.trim();
    let drive_qualified = value.as_bytes().get(1) == Some(&b':');
    if value.is_empty()
        || value.contains('\0')
        || value.starts_with(['\\', '/'])
        || drive_qualified
        || value
            .split(['\\', '/'])
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(PortCellarError::Message(
            "game profile windows_exe must be a relative Windows executable path".to_string(),
        ));
    }
    Ok(())
}

fn validate_windows_absolute_path(value: &str, field: &str) -> Result<()> {
    let value = value.trim();
    let drive_absolute = value.len() >= 3
        && value.as_bytes()[1] == b':'
        && matches!(value.as_bytes()[0].to_ascii_lowercase(), b'c' | b'z')
        && matches!(value.as_bytes()[2], b'\\' | b'/');
    if !drive_absolute || value.contains('\0') || value.split(['\\', '/']).any(|part| part == "..")
    {
        return Err(PortCellarError::Message(format!(
            "game profile {field} must be a safe absolute Windows path"
        )));
    }
    Ok(())
}

pub fn load_generic_game_profile(path: &Path) -> Result<GenericGameProfile> {
    let contents = fs::read_to_string(path)?;
    GenericGameProfile::from_profile_toml(&contents)
}
