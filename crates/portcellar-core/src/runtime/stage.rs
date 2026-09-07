use super::*;
use crate::runtime::profile::prefix_path_from_windows_path;
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};

const STAGE_MARKER: &str = ".portcellar-stage";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameRuntimeStage {
    pub root: PathBuf,
    pub game_root: PathBuf,
}

pub fn prepare_game_runtime_stage(profile: &dyn GameProfile) -> Result<Option<GameRuntimeStage>> {
    let Some(stage) = stage_for_profile(profile) else {
        return Ok(None);
    };
    let source_root = local_game_root(profile)?;
    validate_artifacts(profile, &source_root)?;

    if stage_is_ready(&stage, profile, &source_root) && !env_flag("PORTCELLAR_STAGE_REFRESH") {
        return Ok(Some(stage));
    }

    if stage.game_root.exists() {
        fs::remove_dir_all(&stage.game_root)?;
    }
    if let Some(parent) = stage.game_root.parent() {
        fs::create_dir_all(parent)?;
    }

    copy_tree(&source_root, &stage.game_root)?;
    install_artifacts(profile, &stage.game_root)?;
    apply_binary_patches(profile, &stage.game_root)?;
    fs::write(
        stage.game_root.join(STAGE_MARKER),
        stage_marker(profile, &source_root)?,
    )?;
    Ok(Some(stage))
}

pub(crate) fn game_runtime_stage_for(profile: &dyn GameProfile) -> Option<GameRuntimeStage> {
    let stage = stage_for_profile(profile)?;
    let source_root = local_game_root(profile).ok()?;
    stage_is_ready(&stage, profile, &source_root).then_some(stage)
}

fn stage_for_profile(profile: &dyn GameProfile) -> Option<GameRuntimeStage> {
    if profile.steam_integration() != SteamIntegration::None
        || profile.runtime_artifacts().is_empty()
    {
        return None;
    }

    let root = env::var_os("PORTCELLAR_RUNTIME_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| state_root().join("runtime"));
    let app_root = root.join(safe_component(profile.app_id()));
    Some(GameRuntimeStage {
        root: app_root.clone(),
        game_root: app_root.join("game"),
    })
}

fn local_game_root(profile: &dyn GameProfile) -> Result<PathBuf> {
    let path = profile.windows_install_path_hint().ok_or_else(|| {
        PortCellarError::Message(format!(
            "{} requires windows_install_path for runtime artifact staging",
            profile.name()
        ))
    })?;
    let root = prefix_path_from_windows_path(Path::new("/"), path).ok_or_else(|| {
        PortCellarError::Message(format!(
            "{} has an invalid host-backed Windows install path: {path}",
            profile.name()
        ))
    })?;
    if !root.is_dir() {
        return Err(PortCellarError::Message(format!(
            "local game directory was not found: {}",
            root.display()
        )));
    }
    Ok(root)
}

fn validate_artifacts(profile: &dyn GameProfile, source_root: &Path) -> Result<()> {
    for artifact in profile.runtime_artifacts() {
        crate::games::validate_runtime_artifact_metadata(&artifact)
            .map_err(PortCellarError::Message)?;
        validate_relative_path(&artifact.target_path, "runtime artifact target_path")?;
        if artifact.id.trim().is_empty() || artifact.id.contains('\0') {
            return Err(PortCellarError::Message(
                "runtime artifact id must be non-empty and must not contain NUL".to_string(),
            ));
        }
        let source = resolve_source_path(&artifact.source_path)?;
        if !source.is_file() {
            return Err(PortCellarError::Message(format!(
                "runtime artifact source was not found: {}",
                source.display()
            )));
        }
        if artifact.architecture.as_deref() == Some("pe32-i386") && !is_pe32_i386(&source)? {
            return Err(PortCellarError::Message(format!(
                "runtime artifact is not a PE32 i386 image: {}",
                source.display()
            )));
        }
        let target = source_root.join(artifact_target_path(&artifact));
        if target == source {
            return Err(PortCellarError::Message(format!(
                "runtime artifact would overwrite its source: {}",
                artifact.source_path
            )));
        }
    }
    for patch in profile.binary_patches() {
        crate::games::validate_binary_patch_metadata(&patch).map_err(PortCellarError::Message)?;
        let target = source_root.join(normalize_relative_path(&patch.target_path));
        if !target.is_file() {
            return Err(PortCellarError::Message(format!(
                "binary patch target was not found: {}",
                target.display()
            )));
        }
        if target.symlink_metadata()?.file_type().is_symlink() {
            return Err(PortCellarError::Message(format!(
                "refusing to patch symlink: {}",
                target.display()
            )));
        }
        match patch.kind {
            crate::games::GameBinaryPatchKind::LargeAddressAware if !is_pe32_i386(&target)? => {
                return Err(PortCellarError::Message(format!(
                    "LAA patch requires a PE32 i386 target: {}",
                    target.display()
                )));
            }
            crate::games::GameBinaryPatchKind::LargeAddressAware => {}
        }
    }
    Ok(())
}

fn install_artifacts(profile: &dyn GameProfile, stage_root: &Path) -> Result<()> {
    for artifact in profile.runtime_artifacts() {
        let source = resolve_source_path(&artifact.source_path)?;
        let target = stage_root.join(artifact_target_path(&artifact));
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source, target)?;
    }
    Ok(())
}

fn apply_binary_patches(profile: &dyn GameProfile, stage_root: &Path) -> Result<()> {
    for patch in profile.runtime_policy().binary_patches {
        let target = stage_root.join(normalize_relative_path(&patch.target_path));
        if !target.is_file() {
            return Err(PortCellarError::Message(format!(
                "binary patch target was not found in staged game: {}",
                target.display()
            )));
        }
        match patch.kind {
            crate::games::GameBinaryPatchKind::LargeAddressAware => {
                set_large_address_aware(&target)?;
            }
        }
    }
    Ok(())
}

fn stage_is_ready(stage: &GameRuntimeStage, profile: &dyn GameProfile, source_root: &Path) -> bool {
    if !stage.game_root.is_dir() || !stage.game_root.join(STAGE_MARKER).is_file() {
        return false;
    }
    let Ok(expected_marker) = stage_marker(profile, source_root) else {
        return false;
    };
    if fs::read_to_string(stage.game_root.join(STAGE_MARKER))
        .ok()
        .as_deref()
        != Some(expected_marker.as_str())
    {
        return false;
    }
    if !stage.game_root.join(profile.windows_exe()).is_file() {
        return false;
    }
    let artifacts_ready = profile.runtime_artifacts().iter().all(|artifact| {
        stage
            .game_root
            .join(artifact_target_path(artifact))
            .is_file()
    });
    let patches_ready = profile.binary_patches().iter().all(|patch| {
        let target = stage
            .game_root
            .join(normalize_relative_path(&patch.target_path));
        target.is_file()
            && match patch.kind {
                crate::games::GameBinaryPatchKind::LargeAddressAware => {
                    large_address_aware_enabled(&target).unwrap_or(false)
                }
            }
    });
    artifacts_ready && patches_ready
}

fn artifact_target_path(artifact: &GameRuntimeArtifact) -> PathBuf {
    normalize_relative_path(&artifact.target_path)
}

fn normalize_relative_path(value: &str) -> PathBuf {
    PathBuf::from(value.replace('\\', "/"))
}

fn stage_marker(profile: &dyn GameProfile, source_root: &Path) -> Result<String> {
    let artifacts = profile
        .runtime_artifacts()
        .iter()
        .map(|artifact| {
            let source = resolve_source_path(&artifact.source_path)?;
            Ok(format!(
                "{}:{}:{}",
                artifact.id, artifact.source_path, artifact.target_path,
            ) + &format!(":{}", file_fingerprint(&source)?))
        })
        .collect::<Result<Vec<_>>>()?
        .join("\n");
    let patches = profile
        .runtime_policy()
        .binary_patches
        .iter()
        .map(|patch| format!("binary-patch:{:?}:{}", patch.kind, patch.target_path))
        .collect::<Vec<_>>()
        .join("\n");
    Ok(format!(
        "app_id={}\nprofile_version={}\nwindows_exe={}\nsource={}\nsource_fingerprint={}\n{}\n{}\n",
        profile.app_id(),
        profile.runtime_profile_version(),
        profile.windows_exe(),
        source_root.display(),
        tree_fingerprint(source_root)?,
        artifacts,
        patches
    ))
}

fn tree_fingerprint(root: &Path) -> Result<String> {
    let mut files = Vec::new();
    collect_files(root, &mut files)?;
    files.sort();

    let mut hash = 0xcbf29ce484222325_u64;
    for path in files {
        let relative = path.strip_prefix(root).map_err(|error| {
            PortCellarError::Message(format!("could not fingerprint staged file: {error}"))
        })?;
        hash_bytes(&mut hash, relative.to_string_lossy().as_bytes());
        hash_bytes(&mut hash, &fs::read(&path)?);
    }
    Ok(format!("{hash:016x}"))
}

fn file_fingerprint(path: &Path) -> Result<String> {
    let mut hash = 0xcbf29ce484222325_u64;
    hash_bytes(&mut hash, &fs::read(path)?);
    Ok(format!("{hash:016x}"))
}

fn collect_files(current: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            return Err(PortCellarError::Message(format!(
                "refusing to fingerprint symlink: {}",
                path.display()
            )));
        }
        if file_type.is_dir() {
            collect_files(&path, files)?;
        } else if file_type.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

fn hash_bytes(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(0x100000001b3);
    }
}

fn resolve_source_path(value: &str) -> Result<PathBuf> {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        return Ok(path);
    }
    let root = current_project_root()
        .or_else(|| env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));
    Ok(root.join(path))
}

fn validate_relative_path(value: &str, field: &str) -> Result<()> {
    let path = Path::new(value);
    if value.trim().is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(PortCellarError::Message(format!(
            "{field} must be a non-empty relative path without '..'"
        )));
    }
    Ok(())
}

fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            return Err(PortCellarError::Message(format!(
                "refusing to stage symlink: {}",
                source_path.display()
            )));
        }
        if file_type.is_dir() {
            copy_tree(&source_path, &destination_path)?;
        } else if file_type.is_file() {
            fs::copy(source_path, destination_path)?;
        }
    }
    Ok(())
}

fn is_pe32_i386(path: &Path) -> Result<bool> {
    let bytes = fs::read(path)?;
    let pe_offset = u32::from_le_bytes(
        bytes
            .get(0x3c..0x40)
            .ok_or_else(|| {
                PortCellarError::Message(format!(
                    "runtime artifact is too small to be a PE image: {}",
                    path.display()
                ))
            })?
            .try_into()
            .expect("slice length checked"),
    ) as usize;
    let signature = bytes.get(pe_offset..pe_offset + 4);
    let machine = bytes.get(pe_offset + 4..pe_offset + 6);
    let optional_magic = bytes.get(pe_offset + 24..pe_offset + 26);
    Ok(signature == Some(b"PE\0\0")
        && machine == Some(&[0x4c, 0x01])
        && optional_magic == Some(&[0x0b, 0x01]))
}

fn set_large_address_aware(path: &Path) -> Result<()> {
    let mut bytes = fs::read(path)?;
    let pe_offset = u32::from_le_bytes(
        bytes
            .get(0x3c..0x40)
            .ok_or_else(|| {
                PortCellarError::Message(format!(
                    "binary is too small for PE header: {}",
                    path.display()
                ))
            })?
            .try_into()
            .expect("slice length checked"),
    ) as usize;
    if bytes.get(pe_offset..pe_offset.saturating_add(4)) != Some(b"PE\0\0") {
        return Err(PortCellarError::Message(format!(
            "binary is not a PE image: {}",
            path.display()
        )));
    }
    let machine = bytes.get(pe_offset + 4..pe_offset + 6).ok_or_else(|| {
        PortCellarError::Message(format!("PE machine field is missing: {}", path.display()))
    })?;
    let optional_magic = bytes.get(pe_offset + 24..pe_offset + 26).ok_or_else(|| {
        PortCellarError::Message(format!("PE optional header is missing: {}", path.display()))
    })?;
    if machine != [0x4c, 0x01] || optional_magic != [0x0b, 0x01] {
        return Err(PortCellarError::Message(format!(
            "LAA patch requires a PE32 i386 image: {}",
            path.display()
        )));
    }
    let characteristics_offset = pe_offset + 4 + 18;
    let characteristics = u16::from_le_bytes(
        bytes
            .get(characteristics_offset..characteristics_offset + 2)
            .ok_or_else(|| {
                PortCellarError::Message(format!(
                    "PE characteristics are missing: {}",
                    path.display()
                ))
            })?
            .try_into()
            .expect("slice length checked"),
    );
    let updated = characteristics | 0x0020;
    bytes[characteristics_offset..characteristics_offset + 2]
        .copy_from_slice(&updated.to_le_bytes());
    if updated != characteristics {
        fs::write(path, bytes)?;
    }
    Ok(())
}

fn large_address_aware_enabled(path: &Path) -> Result<bool> {
    let bytes = fs::read(path)?;
    let pe_offset = u32::from_le_bytes(
        bytes
            .get(0x3c..0x40)
            .ok_or_else(|| {
                PortCellarError::Message(format!(
                    "binary is too small for PE header: {}",
                    path.display()
                ))
            })?
            .try_into()
            .expect("slice length checked"),
    ) as usize;
    let characteristics_offset = pe_offset + 4 + 18;
    let characteristics = u16::from_le_bytes(
        bytes
            .get(characteristics_offset..characteristics_offset + 2)
            .ok_or_else(|| {
                PortCellarError::Message(format!(
                    "PE characteristics are missing: {}",
                    path.display()
                ))
            })?
            .try_into()
            .expect("slice length checked"),
    );
    Ok(characteristics & 0x0020 != 0)
}

fn env_flag(name: &str) -> bool {
    matches!(
        env::var(name).ok().as_deref(),
        Some("1" | "true" | "yes" | "on")
    )
}

fn safe_component(value: &str) -> String {
    let value = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();
    if value.is_empty() {
        "game".to_string()
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pe32_header_is_checked() {
        let path = std::env::temp_dir().join(format!("portcellar-pe-{}", std::process::id()));
        let mut image = vec![0; 0x80];
        image[0..2].copy_from_slice(b"MZ");
        image[0x3c..0x40].copy_from_slice(&(0x40_u32).to_le_bytes());
        image[0x40..0x44].copy_from_slice(b"PE\0\0");
        image[0x44..0x46].copy_from_slice(&[0x4c, 0x01]);
        image[0x58..0x5a].copy_from_slice(&[0x0b, 0x01]);
        fs::write(&path, image).unwrap();
        assert!(is_pe32_i386(&path).unwrap());
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn unsafe_target_path_is_rejected() {
        assert!(validate_relative_path("../SCGL.dll", "target").is_err());
        assert!(validate_relative_path("Apps/SimGLRef.dll", "target").is_ok());
    }

    #[test]
    fn large_address_aware_patch_sets_pe_characteristic() {
        let path = std::env::temp_dir().join(format!("portcellar-laa-{}", std::process::id()));
        let pe_offset = 0x40usize;
        let mut image = vec![0; 0x100];
        image[0..2].copy_from_slice(b"MZ");
        image[0x3c..0x40].copy_from_slice(&(pe_offset as u32).to_le_bytes());
        image[pe_offset..pe_offset + 4].copy_from_slice(b"PE\0\0");
        image[pe_offset + 4..pe_offset + 6].copy_from_slice(&[0x4c, 0x01]);
        image[pe_offset + 24..pe_offset + 26].copy_from_slice(&[0x0b, 0x01]);
        fs::write(&path, image).unwrap();

        set_large_address_aware(&path).unwrap();
        let patched = fs::read(&path).unwrap();
        let characteristics =
            u16::from_le_bytes([patched[pe_offset + 4 + 18], patched[pe_offset + 4 + 19]]);
        assert_ne!(characteristics & 0x0020, 0);
        fs::remove_file(path).unwrap();
    }
}
