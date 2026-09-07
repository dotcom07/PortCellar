use super::*;
use std::collections::BTreeMap;
use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

pub fn wine_steam_configure_plans() -> Result<Vec<CommandPlan>> {
    wine_steam_configure_plans_for(&isaac_profile())
}

pub fn wine_steam_configure_plans_for(profile: &dyn GameProfile) -> Result<Vec<CommandPlan>> {
    let runtime = inspect_wine_steam_runtime_for(profile);
    let wine = runtime
        .wine
        .as_ref()
        .ok_or_else(|| PortCellarError::Message("Wine was not found".to_string()))?;
    Ok(vec![
        wine_windows_version_plan(wine, &runtime, &profile.runtime_policy().windows_version)?,
        wine_reg_add_plan(
            wine,
            &runtime,
            "HKCU\\Software\\Wine\\Mac Driver",
            "AllowImmovableWindows",
            "N",
        ),
        wine_reg_add_plan(
            wine,
            &runtime,
            "HKCU\\Software\\Wine\\Mac Driver",
            "RetinaMode",
            "n",
        ),
        wine_reg_add_plan(
            wine,
            &runtime,
            "HKCU\\Control Panel\\Mouse",
            "MouseSpeed",
            "0",
        ),
        wine_reg_add_plan(
            wine,
            &runtime,
            "HKCU\\Control Panel\\Mouse",
            "MouseThreshold1",
            "0",
        ),
        wine_reg_add_plan(
            wine,
            &runtime,
            "HKCU\\Control Panel\\Mouse",
            "MouseThreshold2",
            "0",
        ),
    ])
}

pub(crate) fn env_flag_enabled(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

pub(crate) fn windows_path_in_prefix(prefix: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(prefix.join("drive_c")).ok()?;
    let mut windows = String::from("C:");
    for component in relative.components() {
        windows.push('\\');
        windows.push_str(component.as_os_str().to_str()?);
    }
    Some(windows)
}

pub(crate) fn windows_path_for_host_path(prefix: &Path, path: &Path) -> Option<String> {
    if path.starts_with(prefix) {
        return None;
    }

    let relative = path.strip_prefix(Path::new("/")).ok()?;
    let mut windows = String::from("Z:");
    for component in relative.components() {
        let std::path::Component::Normal(value) = component else {
            continue;
        };
        windows.push('\\');
        windows.push_str(value.to_str()?);
    }
    Some(windows)
}

pub(crate) fn steam_virtual_desktop_name() -> String {
    env::var("PORTCELLAR_VIRTUAL_DESKTOP_NAME")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| STEAM_VIRTUAL_DESKTOP_NAME.to_string())
}

pub(crate) fn steam_virtual_desktop_setting() -> Option<String> {
    env::var("PORTCELLAR_VIRTUAL_DESKTOP")
        .ok()
        .and_then(|value| steam_virtual_desktop_setting_from_value(&value, None))
}

pub(crate) fn steam_virtual_desktop_setting_from_value(
    value: &str,
    detected_size: Option<&str>,
) -> Option<String> {
    let trimmed = value.trim();
    if matches!(
        trimmed.to_ascii_lowercase().as_str(),
        "0" | "false" | "no" | "none" | "off" | "disabled"
    ) {
        return None;
    }
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("auto") {
        return Some(
            detected_size
                .filter(|size| !size.trim().is_empty())
                .unwrap_or("1440x900")
                .to_string(),
        );
    }
    Some(trimmed.to_string())
}

pub(crate) fn wineserver_for_wine(wine: &Path) -> Option<PathBuf> {
    if let Some(path) = env::var_os("PORTCELLAR_WINESERVER").map(PathBuf::from) {
        if path.exists() {
            return Some(path);
        }
    }

    let parent = wine.parent()?;
    let wineserver = parent.join("wineserver");
    wineserver.exists().then_some(wineserver)
}

pub(crate) fn winecfg_for_wine(wine: &Path) -> Option<PathBuf> {
    let parent = wine.parent()?;
    let winecfg = parent.join("winecfg");
    if winecfg.exists() {
        return Some(winecfg);
    }

    find_in_path("winecfg")
}

pub(crate) fn wine_windows_version_plan(
    wine: &Path,
    runtime: &WineSteamRuntime,
    version: &str,
) -> Result<CommandPlan> {
    if wine_engine_kind_for_path(wine) == EngineKind::CrossOver {
        return Ok(CommandPlan {
            program: wine.to_path_buf(),
            args: vec![
                OsString::from("--cx-app"),
                OsString::from("winecfg.exe"),
                OsString::from("/v"),
                OsString::from(version),
            ],
            envs: wine_env(runtime),
            current_dir: None,
        });
    }

    let winecfg = winecfg_for_wine(wine).ok_or_else(|| {
        PortCellarError::Message(format!("winecfg was not found next to {}", wine.display()))
    })?;
    Ok(CommandPlan {
        program: winecfg,
        args: vec![OsString::from("/v"), OsString::from(version)],
        envs: wine_env(runtime),
        current_dir: None,
    })
}

pub(crate) fn wine_reg_add_plan(
    wine: &Path,
    runtime: &WineSteamRuntime,
    key: &str,
    name: &str,
    value: &str,
) -> CommandPlan {
    CommandPlan {
        program: wine.to_path_buf(),
        args: vec![
            OsString::from("reg"),
            OsString::from("add"),
            OsString::from(key),
            OsString::from("/v"),
            OsString::from(name),
            OsString::from("/t"),
            OsString::from("REG_SZ"),
            OsString::from("/d"),
            OsString::from(value),
            OsString::from("/f"),
        ],
        envs: wine_env(runtime),
        current_dir: None,
    }
}

pub(crate) fn wine_env(runtime: &WineSteamRuntime) -> BTreeMap<String, String> {
    let mut envs = BTreeMap::new();
    envs.insert(
        "WINEPREFIX".to_string(),
        runtime.prefix.display().to_string(),
    );
    envs.insert(
        "WINEDEBUG".to_string(),
        env::var("PORTCELLAR_WINEDEBUG").unwrap_or_else(|_| "-all".to_string()),
    );
    envs.insert("MVK_CONFIG_LOG_LEVEL".to_string(), "0".to_string());
    add_crossover_toolchain_env(&mut envs, runtime);
    envs
}

fn add_crossover_toolchain_env(envs: &mut BTreeMap<String, String>, runtime: &WineSteamRuntime) {
    let Some(wine) = runtime.wine.as_deref() else {
        return;
    };
    if wine_engine_kind_for_path(wine) != EngineKind::CrossOver {
        return;
    }

    let Some(root) = wine.parent().and_then(Path::parent) else {
        return;
    };
    if root.file_name().and_then(|name| name.to_str()) != Some("CrossOver") {
        return;
    }

    envs.insert("CX_ROOT".to_string(), root.display().to_string());
    let bin = root.join("bin");
    insert_existing_env_path(envs, "WINESERVER", bin.join("wineserver"));
    insert_existing_env_path(envs, "WINELOADER", bin.join("wineloader"));
    insert_existing_env_path(
        envs,
        "WINEWRAPPER",
        root.join("lib/wine/x86_64-windows/winewrapper.exe"),
    );

    let dll_paths = [
        root.join("lib/wine/x86_64-windows"),
        root.join("lib/wine/i386-windows"),
        root.join("lib/wine"),
    ];
    if dll_paths.iter().all(|path| path.is_dir()) {
        envs.insert(
            "WINEDLLPATH".to_string(),
            dll_paths
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(":"),
        );
    }

    let Some(bottles) = runtime.prefix.parent() else {
        return;
    };
    if bottles.file_name().and_then(|name| name.to_str()) != Some("Bottles") {
        return;
    }
    if let Some(bottle) = runtime.prefix.file_name().and_then(|name| name.to_str()) {
        envs.insert("CX_BOTTLE".to_string(), bottle.to_string());
        envs.insert("CX_BOTTLE_PATH".to_string(), bottles.display().to_string());
    }
}

fn insert_existing_env_path(envs: &mut BTreeMap<String, String>, key: &str, path: PathBuf) {
    if path.exists() {
        envs.insert(key.to_string(), path.display().to_string());
    }
}

pub(crate) fn game_wine_launch_env(
    runtime: &WineSteamRuntime,
    profile: &dyn GameProfile,
) -> BTreeMap<String, String> {
    let policy = profile.runtime_policy();
    let mut envs = policy.environment;
    envs.extend(wine_env(runtime));
    if policy.steam_integration != SteamIntegration::None {
        envs.insert("SteamAppId".to_string(), profile.app_id().to_string());
        envs.insert("SteamGameId".to_string(), profile.app_id().to_string());
        envs.insert("STEAM_GAME_ID".to_string(), profile.app_id().to_string());
        envs.insert("SteamNoOverlayUI".to_string(), "1".to_string());
        envs.insert(
            "DISABLE_VK_LAYER_VALVE_steam_overlay_1".to_string(),
            "1".to_string(),
        );
        envs.insert(
            "WINEDLLOVERRIDES".to_string(),
            prepend_dll_override(
                "gameoverlayrenderer,gameoverlayrenderer64=",
                env::var_os("WINEDLLOVERRIDES").as_deref(),
            ),
        );
        if policy.steam_cef_policy == SteamCefPolicy::CrossOverCompatible {
            envs.insert(
                STEAM_CEF_POLICY_ENV.to_string(),
                STEAM_CEF_CROSSOVER_POLICY_VALUE.to_string(),
            );
        }
    }
    if policy.graphics_backend == GraphicsBackend::Dxmt {
        let dxmt_config = env::var("DXMT_CONFIG").ok().or(policy.dxmt_config);
        if let Some(dxmt_config) = dxmt_config {
            envs.insert("DXMT_CONFIG".to_string(), dxmt_config);
            envs.insert(
                "DXMT_LOG_LEVEL".to_string(),
                env::var("DXMT_LOG_LEVEL").unwrap_or_else(|_| "error".to_string()),
            );
        }
    }
    if policy.graphics_backend == GraphicsBackend::Dxmt {
        if let Some(dxmt_root) = &runtime.dxmt_root {
            envs.insert(
                "WINEDLLPATH_PREPEND".to_string(),
                prepend_env_path(
                    dxmt_root,
                    env::var_os("WINEDLLPATH_PREPEND").as_deref(),
                    ":",
                ),
            );
        }
    }
    apply_graphics_runtime_env(&mut envs, runtime, policy.graphics_backend);
    if policy.dependencies.contains(&RuntimeDependency::XAudio) {
        if let Some(root) = find_runtime_root(
            runtime,
            "PORTCELLAR_FAUDIO_ROOT",
            "faudio",
            has_faudio_runtime,
        ) {
            prepend_runtime_dll_paths(&mut envs, &root);
        }
    }
    if policy
        .dependencies
        .contains(&RuntimeDependency::MediaFoundation)
    {
        if let Some(root) = find_runtime_root(
            runtime,
            "PORTCELLAR_GSTREAMER_ROOT",
            "gstreamer",
            has_gstreamer_runtime,
        ) {
            insert_host_library_path(&mut envs, &root.join("lib"));
            if root.join("lib/gstreamer-1.0").is_dir() {
                envs.insert(
                    "GST_PLUGIN_PATH_1_0".to_string(),
                    root.join("lib/gstreamer-1.0").display().to_string(),
                );
                envs.insert(
                    "GST_PLUGIN_SYSTEM_PATH_1_0".to_string(),
                    root.join("lib/gstreamer-1.0").display().to_string(),
                );
            }
        }
    }
    envs
}

fn apply_graphics_runtime_env(
    envs: &mut BTreeMap<String, String>,
    runtime: &WineSteamRuntime,
    backend: GraphicsBackend,
) {
    match backend {
        GraphicsBackend::Dxvk => {
            if let Some(root) =
                find_runtime_root(runtime, "PORTCELLAR_DXVK_ROOT", "dxvk", has_dxvk_runtime)
            {
                prepend_runtime_dll_paths(envs, &root);
                prepend_override(envs, "d3d8,d3d9,d3d10core,d3d11,dxgi=n,b");
            }
            add_moltenvk_runtime_env(envs, runtime);
        }
        GraphicsBackend::Vkd3d => {
            if let Some(root) =
                find_runtime_root(runtime, "PORTCELLAR_VKD3D_ROOT", "vkd3d", has_vkd3d_runtime)
            {
                prepend_runtime_dll_paths(envs, &root);
                prepend_override(envs, "d3d12,d3d12core,dxgi=n,b");
            }
            add_moltenvk_runtime_env(envs, runtime);
        }
        _ => {}
    }
}

fn prepend_override(envs: &mut BTreeMap<String, String>, value: &str) {
    let existing = envs
        .get("WINEDLLOVERRIDES")
        .map(std::ffi::OsString::from)
        .or_else(|| env::var_os("WINEDLLOVERRIDES"));
    envs.insert(
        "WINEDLLOVERRIDES".to_string(),
        prepend_dll_override(value, existing.as_deref()),
    );
}

fn prepend_runtime_dll_paths(envs: &mut BTreeMap<String, String>, root: &Path) {
    let paths = [root.join("x64/bin"), root.join("x86/bin")]
        .into_iter()
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    if paths.is_empty() {
        return;
    }
    let prepend = prepend_env_paths(
        &paths,
        envs.get("WINEDLLPATH_PREPEND").map(std::ffi::OsStr::new),
    );
    envs.insert("WINEDLLPATH_PREPEND".to_string(), prepend);

    let inherited_dllpath = env::var_os("WINEDLLPATH");
    let dllpath = prepend_env_paths(
        &paths,
        envs.get("WINEDLLPATH")
            .map(std::ffi::OsStr::new)
            .or_else(|| inherited_dllpath.as_deref()),
    );
    envs.insert("WINEDLLPATH".to_string(), dllpath);
}

fn prepend_env_paths(paths: &[PathBuf], existing: Option<&std::ffi::OsStr>) -> String {
    let mut value = existing
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default();
    for path in paths.iter().rev() {
        value = prepend_env_path(
            path,
            (!value.is_empty()).then_some(std::ffi::OsStr::new(&value)),
            ":",
        );
    }
    value
}

fn add_moltenvk_runtime_env(envs: &mut BTreeMap<String, String>, runtime: &WineSteamRuntime) {
    let Some(root) = find_runtime_root(
        runtime,
        "PORTCELLAR_MOLTENVK_ROOT",
        "moltenvk",
        has_moltenvk_runtime,
    ) else {
        return;
    };
    let library_root = if root.join("lib").is_dir() {
        root.join("lib")
    } else {
        root
    };
    insert_host_library_path(envs, &library_root);
}

fn insert_host_library_path(envs: &mut BTreeMap<String, String>, path: &Path) {
    if !path.is_dir() {
        return;
    }
    let existing = envs
        .get("DYLD_FALLBACK_LIBRARY_PATH")
        .map(std::ffi::OsString::from)
        .or_else(|| env::var_os("DYLD_FALLBACK_LIBRARY_PATH"));
    envs.insert(
        "DYLD_FALLBACK_LIBRARY_PATH".to_string(),
        prepend_env_path(path, existing.as_deref(), ":"),
    );
}

fn find_runtime_root(
    runtime: &WineSteamRuntime,
    env_key: &str,
    name: &str,
    is_usable: fn(&Path) -> bool,
) -> Option<PathBuf> {
    let mut candidates = env::var_os(env_key)
        .map(PathBuf::from)
        .into_iter()
        .collect::<Vec<_>>();
    if let Some(root) = state_root_for_prefix(&runtime.prefix) {
        candidates.push(root.join("runtime").join(name));
    }
    candidates.push(state_root().join("runtime").join(name));
    if name == "gstreamer" {
        candidates.push(PathBuf::from("/opt/homebrew/opt/gstreamer"));
        candidates.push(PathBuf::from("/usr/local/opt/gstreamer"));
    }
    candidates.into_iter().find(|path| is_usable(path))
}

fn has_faudio_runtime(root: &Path) -> bool {
    root.join("x64/bin/FAudio.dll").exists() || root.join("x86/bin/FAudio.dll").exists()
}

fn has_dxvk_runtime(root: &Path) -> bool {
    root.join("x64/bin/d3d11.dll").exists() && root.join("x86/bin/d3d11.dll").exists()
}

fn has_vkd3d_runtime(root: &Path) -> bool {
    root.join("x64/bin/libvkd3d-1.dll").exists() && root.join("x86/bin/libvkd3d-1.dll").exists()
}

fn has_moltenvk_runtime(root: &Path) -> bool {
    root.join("libMoltenVK.dylib").exists() || root.join("lib/libMoltenVK.dylib").exists()
}

fn has_gstreamer_runtime(root: &Path) -> bool {
    root.join("lib/libgstreamer-1.0.0.dylib").exists() && root.join("lib/gstreamer-1.0").is_dir()
}

pub(crate) fn game_wine_installer_env(
    runtime: &WineSteamRuntime,
    profile: &dyn GameProfile,
) -> BTreeMap<String, String> {
    let policy = profile.runtime_policy();
    let mut envs = policy.environment;
    envs.extend(wine_env(runtime));
    envs
}

pub(crate) fn prepend_env_path(
    path: &Path,
    existing: Option<&std::ffi::OsStr>,
    separator: &str,
) -> String {
    let path = path.display().to_string();
    let Some(existing) = existing else {
        return path;
    };
    let existing = existing.to_string_lossy();
    if existing.is_empty() || existing.split(separator).any(|part| part == path) {
        existing.to_string()
    } else {
        format!("{path}{separator}{existing}")
    }
}

pub(crate) fn prepend_dll_override(
    override_value: &str,
    existing: Option<&std::ffi::OsStr>,
) -> String {
    let Some(existing) = existing else {
        return override_value.to_string();
    };
    let existing = existing.to_string_lossy();
    if existing.is_empty() || existing.split(';').any(|part| part == override_value) {
        existing.to_string()
    } else {
        format!("{override_value};{existing}")
    }
}
