use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

mod catalog;
mod generic;
mod isaac;
mod module;

pub use module::{
    GameModule, GameModuleCatalog, GameModuleDescriptor, GameModuleProfileReference,
    GameModuleVariant,
};

pub use catalog::{GenericGameProfileCatalog, GenericGameProfileCatalogEntry};
pub use generic::{
    load_generic_game_profile, GenericGameProfile, GenericGameProfileDocument,
    GENERIC_GAME_PROFILE_FORMAT_VERSION,
};
#[cfg(test)]
pub(crate) use isaac::isaac_steam_cloud_option_value_from;
pub use isaac::{isaac_profile, IsaacProfile};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameRuntimeArtifact {
    pub id: String,
    pub source_path: String,
    pub target_path: String,
    #[serde(default)]
    pub architecture: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GameBinaryPatchKind {
    LargeAddressAware,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameBinaryPatch {
    pub target_path: String,
    pub kind: GameBinaryPatchKind,
}

pub(crate) fn validate_runtime_artifact_metadata(
    artifact: &GameRuntimeArtifact,
) -> std::result::Result<(), String> {
    if artifact.id.trim().is_empty() || artifact.id.contains('\0') {
        return Err("runtime artifact id must be non-empty and must not contain NUL".to_string());
    }
    if artifact.source_path.trim().is_empty() || artifact.source_path.contains('\0') {
        return Err(format!(
            "runtime artifact {} source_path must be non-empty and must not contain NUL",
            artifact.id
        ));
    }
    let normalized_target = artifact.target_path.replace('\\', "/");
    let target = Path::new(&normalized_target);
    if normalized_target.trim().is_empty()
        || target.is_absolute()
        || target.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!(
            "runtime artifact {} target_path must be a relative path without '..'",
            artifact.id
        ));
    }
    if let Some(architecture) = artifact.architecture.as_deref() {
        if architecture.trim().is_empty() || architecture.contains('\0') {
            return Err(format!(
                "runtime artifact {} architecture must not be empty or contain NUL",
                artifact.id
            ));
        }
    }
    Ok(())
}

pub(crate) fn validate_binary_patch_metadata(
    patch: &GameBinaryPatch,
) -> std::result::Result<(), String> {
    let normalized_target = patch.target_path.replace('\\', "/");
    let target = Path::new(&normalized_target);
    if normalized_target.trim().is_empty()
        || target.is_absolute()
        || target.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(
            "binary patch target_path must be a non-empty relative path without '..'".to_string(),
        );
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GraphicsBackend {
    Auto,
    WineD3D,
    Dxmt,
    Dxvk,
    D3dMetal,
    Vkd3d,
    Scgl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SteamIntegration {
    Required,
    Optional,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SteamCefPolicy {
    #[default]
    WineDefault,
    CrossOverCompatible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GameCapability {
    Steamworks,
    Direct3d9,
    Direct3d11,
    Direct3d12,
    OpenGl,
    Vulkan,
    VideoPlayback,
    Audio,
    AntiCheat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeDependency {
    Vcrun,
    Vcrun2022,
    D3dCompiler,
    MediaFoundation,
    OpenAl,
    XAudio,
    TrebuchetMs,
}

impl RuntimeDependency {
    pub fn winetricks_verb(self) -> Option<&'static str> {
        match self {
            Self::Vcrun => Some("vcrun2019"),
            Self::Vcrun2022 => Some("vcrun2022"),
            Self::D3dCompiler => Some("d3dcompiler_47"),
            Self::MediaFoundation => None,
            Self::OpenAl => Some("openal"),
            Self::XAudio => Some("xact"),
            Self::TrebuchetMs => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameRuntimePolicy {
    pub windows_version: String,
    pub wine_engine_path: Option<PathBuf>,
    pub graphics_backend: GraphicsBackend,
    pub dxmt_config: Option<String>,
    pub steam_integration: SteamIntegration,
    pub steam_cef_policy: SteamCefPolicy,
    pub launch_arguments: Vec<String>,
    pub runtime_options_path: Option<String>,
    pub runtime_options: BTreeMap<String, String>,
    pub environment: BTreeMap<String, String>,
    pub runtime_artifacts: Vec<GameRuntimeArtifact>,
    pub binary_patches: Vec<GameBinaryPatch>,
    pub capabilities: BTreeSet<GameCapability>,
    pub dependencies: Vec<RuntimeDependency>,
}

pub trait GameProfile {
    fn app_id(&self) -> &str;
    fn name(&self) -> &str;
    fn bottle_name(&self) -> Option<&str> {
        None
    }
    fn windows_depot_id(&self) -> Option<&str>;
    fn windows_exe(&self) -> &str;
    fn launch_arguments(&self) -> Vec<String> {
        Vec::new()
    }
    fn install_dir_hint(&self) -> &str;
    fn windows_install_path_hint(&self) -> Option<&str> {
        None
    }
    fn runtime_profile_version(&self) -> &str;
    fn wine_engine_path_hint(&self) -> Option<&Path> {
        None
    }
    fn wine_windows_version(&self) -> &str;
    fn dxmt_config(&self) -> Option<&str>;
    fn savedata_path_marker(&self) -> Option<&str>;
    fn default_documents_subdir(&self) -> Option<&str>;
    fn runtime_options_path_hint(&self) -> Option<&str> {
        None
    }
    fn runtime_options(&self) -> BTreeMap<String, String>;

    fn graphics_backend(&self) -> GraphicsBackend {
        self.dxmt_config()
            .map(|_| GraphicsBackend::Dxmt)
            .unwrap_or(GraphicsBackend::Auto)
    }

    fn steam_integration(&self) -> SteamIntegration {
        SteamIntegration::Required
    }

    fn steam_cef_policy(&self) -> SteamCefPolicy {
        SteamCefPolicy::WineDefault
    }

    fn runtime_environment(&self) -> BTreeMap<String, String> {
        BTreeMap::new()
    }

    fn runtime_artifacts(&self) -> Vec<GameRuntimeArtifact> {
        Vec::new()
    }

    fn binary_patches(&self) -> Vec<GameBinaryPatch> {
        Vec::new()
    }

    fn capabilities(&self) -> BTreeSet<GameCapability> {
        BTreeSet::new()
    }

    fn dependencies(&self) -> Vec<RuntimeDependency> {
        Vec::new()
    }

    fn runtime_policy(&self) -> GameRuntimePolicy {
        GameRuntimePolicy {
            windows_version: self.wine_windows_version().to_string(),
            wine_engine_path: self.wine_engine_path_hint().map(Path::to_path_buf),
            graphics_backend: self.graphics_backend(),
            dxmt_config: self.dxmt_config().map(str::to_string),
            steam_integration: self.steam_integration(),
            steam_cef_policy: self.steam_cef_policy(),
            launch_arguments: self.launch_arguments(),
            runtime_options_path: self.runtime_options_path_hint().map(str::to_string),
            runtime_options: self.runtime_options(),
            environment: self.runtime_environment(),
            runtime_artifacts: self.runtime_artifacts(),
            binary_patches: self.binary_patches(),
            capabilities: self.capabilities(),
            dependencies: self.dependencies(),
        }
    }
}
