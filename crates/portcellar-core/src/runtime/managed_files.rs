use crate::{PortCellarError, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
};

pub(crate) fn invalid(message: impl Into<String>) -> PortCellarError {
    PortCellarError::Message(message.into())
}

pub(crate) fn digest(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid(
            "SHA-256 must contain 64 lowercase hexadecimal characters",
        ));
    }
    Ok(())
}

pub(crate) fn sha256(path: &Path) -> Result<String> {
    if !fs::symlink_metadata(path)?.file_type().is_file() {
        return Err(invalid(format!(
            "expected a regular file: {}",
            path.display()
        )));
    }
    let output = Command::new("/usr/bin/shasum")
        .args(["-a", "256", "--"])
        .arg(path)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .output()?;
    parse_digest(output.status.success(), &output.stdout)
}

pub(crate) fn sha256_bytes(bytes: &[u8]) -> Result<String> {
    let mut child = Command::new("/usr/bin/shasum")
        .args(["-a", "256"])
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or_else(|| invalid("hash input was unavailable"))?
        .write_all(bytes)?;
    let output = child.wait_with_output()?;
    parse_digest(output.status.success(), &output.stdout)
}

fn parse_digest(success: bool, output: &[u8]) -> Result<String> {
    if !success {
        return Err(invalid("SHA-256 computation failed"));
    }
    let hash = std::str::from_utf8(output)
        .map_err(|_| invalid("invalid SHA-256 output"))?
        .split_whitespace()
        .next()
        .ok_or_else(|| invalid("missing SHA-256 output"))?;
    digest(hash)?;
    Ok(hash.into())
}

pub(crate) fn read_record<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() || metadata.len() > 262_144 {
        return Err(invalid(
            "managed record must be a regular file at most 256 KiB",
        ));
    }
    let mut text = String::new();
    File::open(path)?.take(262_145).read_to_string(&mut text)?;
    if text.len() > 262_144 {
        return Err(invalid("managed record grew during inspection"));
    }
    toml::from_str(&text).map_err(|error| invalid(format!("invalid managed record: {error}")))
}

pub(crate) fn write_record<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let text = toml::to_string_pretty(value).map_err(|error| invalid(error.to_string()))?;
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(text.as_bytes())?;
    file.sync_all()?;
    File::open(
        path.parent()
            .ok_or_else(|| invalid("record has no parent"))?,
    )?
    .sync_all()?;
    Ok(())
}

pub(crate) fn confined(root: &Path, relative: &str) -> Result<PathBuf> {
    if relative.is_empty()
        || relative.contains(['\\', ':', '\0'])
        || relative
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        || Path::new(relative)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(invalid(
            "managed path must be a confined relative path using '/'",
        ));
    }
    let mut path = root.to_path_buf();
    for part in relative.split('/') {
        path.push(part);
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(invalid(format!("refusing symlink: {}", path.display())))
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(path)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileChange {
    format_version: String,
    target: String,
    original_sha256: Option<String>,
    replacement_sha256: String,
}

fn change_paths(root: &Path, scope: &str, relative: &str) -> Result<(PathBuf, PathBuf)> {
    let key = sha256_bytes(format!("{scope}/{relative}").as_bytes())?;
    let directory = confined(root, &format!("recovery/{key}"))?;
    Ok((directory.join("change.toml"), directory))
}

fn retain_copy(source: &Path, destination: &Path, expected: &str) -> Result<()> {
    if sha256(source)? != expected {
        return Err(invalid("recovery input SHA-256 mismatch"));
    }
    match fs::symlink_metadata(destination) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let mut input = File::open(source)?;
            let mut output = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(destination)?;
            std::io::copy(&mut input, &mut output)?;
            output.sync_all()?;
        }
        Err(error) => return Err(error.into()),
    }
    if sha256(destination)? != expected {
        return Err(invalid(
            "retained file digest mismatch; recovery files preserved",
        ));
    }
    Ok(())
}

fn activate(source: &Path, target: &Path, expected: &str) -> Result<()> {
    let parent = target
        .parent()
        .ok_or_else(|| invalid("file has no parent"))?;
    let name = target
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| invalid("target must support UTF-8"))?;
    let temporary = parent.join(format!(".portcellar-{name}-{expected}.pending"));
    retain_copy(source, &temporary, expected)?;
    fs::rename(&temporary, target)?;
    File::open(parent)?.sync_all()?;
    Ok(())
}

pub(crate) fn replace(
    root: &Path,
    scope: &str,
    base: &Path,
    relative: &str,
    artifact: &Path,
    original: Option<&str>,
    replacement: &str,
) -> Result<()> {
    if relative.split('/').any(|p| p.starts_with(".portcellar")) {
        return Err(invalid("reserved managed target path"));
    }
    let target = confined(base, relative)?;
    digest(replacement)?;
    if let Some(hash) = original {
        digest(hash)?;
    }
    if sha256(artifact)? != replacement {
        return Err(invalid("replacement SHA-256 mismatch"));
    }
    let current = if target.try_exists()? {
        Some(sha256(&target)?)
    } else {
        None
    };
    let (record, recovery) = change_paths(root, scope, relative)?;
    if record.try_exists()? {
        let previous: FileChange = read_record(&record)?;
        if previous.format_version != "portcellar-file-change-v1"
            || previous.target != relative
            || previous.original_sha256.as_deref() != original
            || previous.replacement_sha256 != replacement
        {
            return Err(invalid(
                "existing recovery record belongs to a different change",
            ));
        }
        if current.as_deref() == Some(replacement) {
            return Ok(());
        }
    }
    if current.as_deref() != original {
        return Err(invalid("original SHA-256 mismatch; current file preserved"));
    }
    if !target.parent().is_some_and(Path::is_dir) {
        return Err(invalid("target parent must already exist"));
    }
    fs::create_dir_all(&recovery)?;
    if let Some(hash) = original {
        retain_copy(&target, &recovery.join("original"), hash)?;
    }
    retain_copy(artifact, &recovery.join("replacement"), replacement)?;
    if !record.try_exists()? {
        write_record(
            &record,
            &FileChange {
                format_version: "portcellar-file-change-v1".into(),
                target: relative.into(),
                original_sha256: original.map(str::to_string),
                replacement_sha256: replacement.into(),
            },
        )?;
    }
    activate(&recovery.join("replacement"), &target, replacement)
}

pub(crate) fn restore(root: &Path, scope: &str, base: &Path, relative: &str) -> Result<()> {
    let target = confined(base, relative)?;
    let (record, recovery) = change_paths(root, scope, relative)?;
    let change: FileChange = read_record(&record)?;
    if change.format_version != "portcellar-file-change-v1" || change.target != relative {
        return Err(invalid("invalid recovery identity"));
    }
    digest(&change.replacement_sha256)?;
    if let Some(hash) = &change.original_sha256 {
        digest(hash)?;
    }
    let current = if target.try_exists()? {
        Some(sha256(&target)?)
    } else {
        None
    };
    if current == change.original_sha256 {
        return Ok(());
    }
    if current.as_deref() != Some(&change.replacement_sha256) {
        return Err(invalid("current file changed; refusing stale restoration"));
    }
    match change.original_sha256 {
        Some(hash) => activate(&recovery.join("original"), &target, &hash),
        None => {
            fs::remove_file(&target)?;
            File::open(target.parent().unwrap())?.sync_all()?;
            Ok(())
        }
    }
}
