use crate::{EngineKind, RuntimeComponent};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub(crate) fn runtime_components_for_engine(
    engine: &Path,
    kind: EngineKind,
    dxmt_root: Option<&Path>,
) -> Vec<RuntimeComponent> {
    let mut components = BTreeSet::new();

    for root in engine_roots(engine) {
        let wine_lib = root.join("lib/wine");
        let windows64 = wine_lib.join("x86_64-windows");
        let unix64 = wine_lib.join("x86_64-unix");

        if windows64.join("wined3d.dll").exists() {
            components.insert(RuntimeComponent::WineD3d);
        }
        if windows64.join("winevulkan.dll").exists() || unix64.join("winevulkan.so").exists() {
            components.insert(RuntimeComponent::WineVulkan);
        }
        if windows64.join("winegstreamer.dll").exists() || unix64.join("winegstreamer.so").exists()
        {
            components.insert(RuntimeComponent::WineGstreamer);
        }
        if windows64.join("winecoreaudio.drv").exists() || unix64.join("winecoreaudio.so").exists()
        {
            components.insert(RuntimeComponent::WineCoreAudio);
        }
        if has_faudio(&root, &windows64) {
            components.insert(RuntimeComponent::FAudio);
        }
        if has_vkd3d(&windows64) {
            components.insert(RuntimeComponent::Vkd3d);
        }
        if has_dxvk(&wine_lib) {
            components.insert(RuntimeComponent::Dxvk);
        }

        let lib64 = root.join("lib64");
        if lib64.join("libMoltenVK.dylib").exists() {
            components.insert(RuntimeComponent::MoltenVk);
        }
        if lib64.join("gstreamer-1.0").is_dir() || lib64.join("libgstreamer-1.0.0.dylib").exists() {
            components.insert(RuntimeComponent::GstreamerHost);
        }
        if root
            .join("lib64/apple_gptk/external/D3DMetal.framework/Versions/A/D3DMetal")
            .exists()
        {
            components.insert(RuntimeComponent::D3dMetal);
        }
    }

    if Path::new("/Library/Frameworks/GStreamer.framework/Libraries")
        .join("libgstreamer-1.0.0.dylib")
        .exists()
    {
        components.insert(RuntimeComponent::GstreamerHost);
    }
    if dxmt_root.is_some_and(is_dxmt_root) {
        components.insert(RuntimeComponent::Dxmt);
    }

    if kind == EngineKind::GamePortingToolkit {
        components.insert(RuntimeComponent::D3dMetal);
    }

    for root in external_component_roots(dxmt_root) {
        if has_external_faudio(&root) {
            components.insert(RuntimeComponent::FAudio);
        }
        if has_external_dxvk(&root) {
            components.insert(RuntimeComponent::Dxvk);
        }
        if has_external_vkd3d(&root) {
            components.insert(RuntimeComponent::Vkd3d);
        }
        if has_external_moltenvk(&root) {
            components.insert(RuntimeComponent::MoltenVk);
        }
        if has_external_gstreamer(&root) {
            components.insert(RuntimeComponent::GstreamerHost);
        }
    }

    components.into_iter().collect()
}

fn engine_roots(engine: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(bin) = engine.parent() {
        if let Some(root) = bin.parent() {
            roots.push(root.to_path_buf());
        }
    }
    roots.push(engine.to_path_buf());
    roots.sort();
    roots.dedup();
    roots
}

fn has_vkd3d(windows64: &Path) -> bool {
    [
        "libvkd3d-1.dll",
        "libvkd3d-shader-1.dll",
        "libvkd3d-utils-1.dll",
    ]
    .iter()
    .any(|name| windows64.join(name).exists())
}

fn has_faudio(root: &Path, windows64: &Path) -> bool {
    windows64.join("FAudio.dll").exists()
        || find_named_file(&root.join("share/wine/mono"), "FAudio.dll")
}

fn find_named_file(root: &Path, name: &str) -> bool {
    let mut pending = vec![root.to_path_buf()];
    while let Some(path) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(path) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.file_name().and_then(|value| value.to_str()) == Some(name) {
                return true;
            }
            if path.is_dir() {
                pending.push(path);
            }
        }
    }
    false
}

fn has_dxvk(wine_lib: &Path) -> bool {
    ["x86_64-windows", "i386-windows"].iter().any(|arch| {
        std::fs::read_dir(wine_lib.join(arch))
            .ok()
            .into_iter()
            .flatten()
            .flatten()
            .any(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .to_ascii_lowercase()
                    .starts_with("dxvk")
            })
    })
}

fn has_external_faudio(root: &Path) -> bool {
    root.join("x64/bin/FAudio.dll").exists() || root.join("x86/bin/FAudio.dll").exists()
}

fn has_external_dxvk(root: &Path) -> bool {
    [
        "d3d8.dll",
        "d3d9.dll",
        "d3d10core.dll",
        "d3d11.dll",
        "dxgi.dll",
    ]
    .iter()
    .any(|name| root.join(format!("x64/bin/{name}")).exists())
        && [
            "d3d8.dll",
            "d3d9.dll",
            "d3d10core.dll",
            "d3d11.dll",
            "dxgi.dll",
        ]
        .iter()
        .any(|name| root.join(format!("x86/bin/{name}")).exists())
}

fn has_external_vkd3d(root: &Path) -> bool {
    root.join("x64/bin/libvkd3d-1.dll").exists() || root.join("x86/bin/libvkd3d-1.dll").exists()
}

fn has_external_moltenvk(root: &Path) -> bool {
    root.join("libMoltenVK.dylib").exists() || root.join("lib/libMoltenVK.dylib").exists()
}

fn has_external_gstreamer(root: &Path) -> bool {
    root.join("lib/libgstreamer-1.0.0.dylib").exists() && root.join("lib/gstreamer-1.0").is_dir()
}

fn external_component_roots(dxmt_root: Option<&Path>) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for (key, name) in [
        ("PORTCELLAR_FAUDIO_ROOT", "faudio"),
        ("PORTCELLAR_DXVK_ROOT", "dxvk"),
        ("PORTCELLAR_VKD3D_ROOT", "vkd3d"),
        ("PORTCELLAR_MOLTENVK_ROOT", "moltenvk"),
        ("PORTCELLAR_GSTREAMER_ROOT", "gstreamer"),
    ] {
        if let Some(path) = std::env::var_os(key).map(PathBuf::from) {
            roots.push(path);
        }
        if let Some(root) = dxmt_root.and_then(Path::parent) {
            roots.push(root.join("runtime").join(name));
        }
    }
    roots.push(PathBuf::from("/opt/homebrew/opt/gstreamer"));
    roots.push(PathBuf::from("/usr/local/opt/gstreamer"));
    roots.sort();
    roots.dedup();
    roots
}

fn is_dxmt_root(path: &Path) -> bool {
    path.join("i386-windows/d3d11.dll").exists()
        && path.join("x86_64-windows/d3d11.dll").exists()
        && path.join("x86_64-unix/winemetal.so").exists()
}
