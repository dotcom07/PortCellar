use super::*;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn wine_steam_cef_patch_plan() -> Result<SteamCefPatchPlan> {
    wine_steam_cef_patch_plan_for(&isaac_profile())
}

pub fn wine_steam_cef_patch_plan_for(profile: &dyn GameProfile) -> Result<SteamCefPatchPlan> {
    let runtime = inspect_wine_steam_runtime_for(profile);
    let steam_exe = runtime.steam_exe.as_ref().ok_or_else(|| {
        PortCellarError::Message(
            "Windows Steam was not found in the configured Wine prefix".to_string(),
        )
    })?;
    let steam_dir = steam_exe.parent().map(Path::to_path_buf).ok_or_else(|| {
        PortCellarError::Message("Windows Steam executable has no parent".to_string())
    })?;

    Ok(steam_cef_patch_plan_for(&runtime.prefix, &steam_dir))
}

pub fn apply_steam_cef_patch(plan: &SteamCefPatchPlan) -> Result<SteamCefPatchResult> {
    let wrappers = build_steamwebhelper_wrappers()?;
    let mut patched_targets = Vec::new();
    let mut removed_locks = Vec::new();

    for lock in &plan.htmlcache_locks {
        match fs::remove_file(lock) {
            Ok(()) => removed_locks.push(lock.clone()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }

    for target in &plan.cef_targets {
        install_steamwebhelper_wrapper(target, &wrappers)?;
        patched_targets.push(target.clone());
    }

    Ok(SteamCefPatchResult {
        patched_targets,
        removed_locks,
    })
}

pub(crate) fn steam_cef_patch_plan_for(prefix: &Path, steam_dir: &Path) -> SteamCefPatchPlan {
    let cef_root = steam_dir.join("bin/cef");
    let mut cef_targets = Vec::new();
    if let Ok(entries) = fs::read_dir(&cef_root) {
        for entry in entries.flatten() {
            let cef_dir = entry.path();
            let Some(name) = cef_dir.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if !name.starts_with("cef.win") {
                continue;
            }
            let target = cef_dir.join("steamwebhelper.exe");
            if !target.is_file() {
                continue;
            }
            let real = cef_dir.join("steamwebhelper_real.exe");
            cef_targets.push(SteamCefPatchTarget {
                arch: steamwebhelper_arch_for_cef_dir(name),
                already_patched: is_current_steamwebhelper_wrapper(&target),
                cef_dir,
                real,
                target,
            });
        }
    }
    cef_targets.sort_by(|left, right| left.cef_dir.cmp(&right.cef_dir));

    SteamCefPatchPlan {
        cef_targets,
        htmlcache_locks: steam_htmlcache_locks(prefix),
        steam_dir: steam_dir.to_path_buf(),
    }
}

pub(crate) fn steamwebhelper_arch_for_cef_dir(name: &str) -> SteamWebHelperArch {
    if name.contains("x64") || name.contains("win64") {
        SteamWebHelperArch::X86_64
    } else {
        SteamWebHelperArch::X86
    }
}

pub(crate) fn steam_htmlcache_locks(prefix: &Path) -> Vec<PathBuf> {
    let users_dir = prefix.join("drive_c/users");
    let mut locks = Vec::new();
    let Ok(users) = fs::read_dir(users_dir) else {
        return locks;
    };
    for user in users.flatten() {
        let htmlcache = user.path().join("AppData/Local/Steam/htmlcache");
        collect_htmlcache_locks(&htmlcache, 0, &mut locks);
    }
    locks.sort();
    locks
}

pub(crate) fn collect_htmlcache_locks(dir: &Path, depth: usize, locks: &mut Vec<PathBuf>) {
    if depth > 2 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_htmlcache_locks(&path, depth + 1, locks);
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if name.starts_with("Singleton") || name.ends_with(".lock") {
            locks.push(path);
        }
    }
}

struct SteamWebHelperWrappers {
    x86: PathBuf,
    x86_64: PathBuf,
}

fn build_steamwebhelper_wrappers() -> Result<SteamWebHelperWrappers> {
    let build_dir = env::temp_dir().join("portcellar-steamwebhelper-wrapper");
    fs::create_dir_all(&build_dir)?;
    let source = build_dir.join("steamwebhelper-wrapper.c");
    fs::write(&source, STEAM_WEBHELPER_WRAPPER_SOURCE)?;

    let x86_64 = build_dir.join("steamwebhelper-wrapper-x86_64.exe");
    let x86 = build_dir.join("steamwebhelper-wrapper-x86.exe");
    compile_steamwebhelper_wrapper(
        "PORTCELLAR_MINGW64_CC",
        "x86_64-w64-mingw32-gcc",
        &source,
        &x86_64,
    )?;
    compile_steamwebhelper_wrapper(
        "PORTCELLAR_MINGW32_CC",
        "i686-w64-mingw32-gcc",
        &source,
        &x86,
    )?;

    Ok(SteamWebHelperWrappers { x86, x86_64 })
}

pub(crate) fn compile_steamwebhelper_wrapper(
    env_name: &str,
    default_cc: &str,
    source: &Path,
    output: &Path,
) -> Result<()> {
    let cc = env::var_os(env_name)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(default_cc));
    let status = Command::new(&cc)
        .args([
            "-O2",
            "-Wall",
            "-Wextra",
            "-municode",
            "-DUNICODE",
            "-D_UNICODE",
            "-o",
        ])
        .arg(output)
        .arg(source)
        .args(["-static", "-lshell32", "-mwindows"])
        .status()
        .map_err(|error| {
            PortCellarError::Message(format!(
                "failed to run {env_name} compiler {}: {error}",
                cc.display()
            ))
        })?;
    if !status.success() {
        return Err(PortCellarError::Message(format!(
            "steamwebhelper wrapper compiler exited with {status}"
        )));
    }
    Ok(())
}

fn install_steamwebhelper_wrapper(
    target: &SteamCefPatchTarget,
    wrappers: &SteamWebHelperWrappers,
) -> Result<()> {
    let valve_binary_is_present = target.real.is_file() && !is_wrapper_like(&target.real);
    if target.already_patched && !valve_binary_is_present {
        return Err(PortCellarError::Message(format!(
            "Valve steamwebhelper binary is missing for {}",
            target.cef_dir.display()
        )));
    }

    if !target.already_patched && !valve_binary_is_present {
        if is_wrapper_like(&target.target) {
            return Err(PortCellarError::Message(format!(
                "Valve steamwebhelper binary is missing for {}",
                target.cef_dir.display()
            )));
        }
        fs::copy(&target.target, &target.real)?;
    }

    let wrapper = match target.arch {
        SteamWebHelperArch::X86 => &wrappers.x86,
        SteamWebHelperArch::X86_64 => &wrappers.x86_64,
    };
    fs::copy(wrapper, &target.target)?;
    Ok(())
}

pub(crate) fn is_wrapper_like(path: &Path) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.len() < STEAM_WEBHELPER_WRAPPER_SIZE_CEILING)
        .unwrap_or(false)
}

pub(crate) fn is_current_steamwebhelper_wrapper(path: &Path) -> bool {
    fs::read(path)
        .map(|bytes| contains_bytes(&bytes, STEAM_WEBHELPER_WRAPPER_MARKER))
        .unwrap_or(false)
}

pub(crate) fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}
