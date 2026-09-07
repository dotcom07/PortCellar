use std::fs;
use std::path::Path;

const PE_READ_SIZE_CEILING: u64 = 256 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeSummary {
    pub machine: Option<String>,
    pub subsystem: Option<String>,
    pub import_dlls: Vec<String>,
    pub detected_layers: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
struct Section {
    virtual_address: u32,
    virtual_size: u32,
    raw_ptr: u32,
    raw_size: u32,
}

pub(crate) fn summarize_pe(path: &Path) -> Option<PeSummary> {
    let metadata = fs::metadata(path).ok()?;
    if metadata.len() > PE_READ_SIZE_CEILING {
        return None;
    }

    summarize_pe_bytes(&fs::read(path).ok()?)
}

pub(crate) fn summarize_pe_bytes(bytes: &[u8]) -> Option<PeSummary> {
    if bytes.len() < 0x40 || &bytes[0..2] != b"MZ" {
        return None;
    }

    let pe_offset = read_u32(bytes, 0x3c)? as usize;
    if pe_offset + 24 >= bytes.len() || &bytes[pe_offset..pe_offset + 4] != b"PE\0\0" {
        return None;
    }

    let machine = machine_name(read_u16(bytes, pe_offset + 4)?).map(str::to_string);
    let section_count = read_u16(bytes, pe_offset + 6)? as usize;
    let optional_size = read_u16(bytes, pe_offset + 20)? as usize;
    let optional_offset = pe_offset + 24;
    let section_offset = optional_offset.checked_add(optional_size)?;
    let magic = read_u16(bytes, optional_offset)?;
    let subsystem = subsystem_name(read_u16(bytes, optional_offset + 68)?).map(str::to_string);
    let data_dir_offset = match magic {
        0x10b => optional_offset + 96,
        0x20b => optional_offset + 112,
        _ => return None,
    };
    let import_directory_offset = data_dir_offset + 8;
    let import_rva = read_u32(bytes, import_directory_offset)?;

    let mut sections = Vec::with_capacity(section_count);
    for index in 0..section_count {
        let offset = section_offset + index * 40;
        if offset + 40 > bytes.len() {
            break;
        }
        sections.push(Section {
            virtual_size: read_u32(bytes, offset + 8).unwrap_or_default(),
            virtual_address: read_u32(bytes, offset + 12).unwrap_or_default(),
            raw_size: read_u32(bytes, offset + 16).unwrap_or_default(),
            raw_ptr: read_u32(bytes, offset + 20).unwrap_or_default(),
        });
    }

    let import_dlls = import_rva
        .checked_sub(1)
        .and_then(|_| import_dlls(bytes, import_rva, &sections))
        .unwrap_or_default();
    let detected_layers = detected_layers(&import_dlls);

    Some(PeSummary {
        machine,
        subsystem,
        import_dlls,
        detected_layers,
    })
}

fn import_dlls(bytes: &[u8], import_rva: u32, sections: &[Section]) -> Option<Vec<String>> {
    let mut dlls = Vec::new();
    let mut descriptor_offset = rva_to_offset(import_rva, sections)?;

    for _ in 0..512 {
        if descriptor_offset + 20 > bytes.len() {
            break;
        }
        let original_first_thunk = read_u32(bytes, descriptor_offset)?;
        let time_date_stamp = read_u32(bytes, descriptor_offset + 4)?;
        let forwarder_chain = read_u32(bytes, descriptor_offset + 8)?;
        let name_rva = read_u32(bytes, descriptor_offset + 12)?;
        let first_thunk = read_u32(bytes, descriptor_offset + 16)?;
        if original_first_thunk == 0
            && time_date_stamp == 0
            && forwarder_chain == 0
            && name_rva == 0
            && first_thunk == 0
        {
            break;
        }

        if let Some(name_offset) = rva_to_offset(name_rva, sections) {
            if let Some(name) = read_c_string(bytes, name_offset) {
                dlls.push(name.to_ascii_lowercase());
            }
        }
        descriptor_offset += 20;
    }

    dlls.sort();
    dlls.dedup();
    Some(dlls)
}

fn detected_layers(import_dlls: &[String]) -> Vec<String> {
    let mut layers = Vec::new();
    let has = |needle: &str| import_dlls.iter().any(|dll| dll.contains(needle));

    if has("steam_api") {
        layers.push("steamworks".to_string());
    }
    if has("d3d12") {
        layers.push("d3d12".to_string());
    }
    if has("d3d11") || has("dxgi") {
        layers.push("d3d11/dxgi".to_string());
    }
    if has("d3d9") {
        layers.push("d3d9".to_string());
    }
    if has("opengl32") {
        layers.push("opengl".to_string());
    }
    if has("vulkan-1") {
        layers.push("vulkan".to_string());
    }
    if has("xinput") || has("dinput") {
        layers.push("controller/input".to_string());
    }
    if has("xaudio") || has("dsound") || has("mmdevapi") || has("winmm") || has("openal") {
        layers.push("audio".to_string());
    }
    if has("mfplat")
        || has("mfreadwrite")
        || has("wmvcore")
        || has("quartz")
        || has("bink")
        || has("theora")
        || has("avcodec")
    {
        layers.push("media-codec".to_string());
    }
    if has("vcruntime") || has("msvcp") {
        layers.push("vc-runtime".to_string());
    }
    if has("mscoree") {
        layers.push(".net-runtime".to_string());
    }
    if has("easyanticheat") || has("battleye") {
        layers.push("anti-cheat-risk".to_string());
    }

    layers.sort();
    layers.dedup();
    layers
}

fn rva_to_offset(rva: u32, sections: &[Section]) -> Option<usize> {
    sections.iter().find_map(|section| {
        let size = section.virtual_size.max(section.raw_size);
        let start = section.virtual_address;
        let end = start.checked_add(size)?;
        if (start..end).contains(&rva) {
            Some((section.raw_ptr + (rva - start)) as usize)
        } else {
            None
        }
    })
}

fn read_c_string(bytes: &[u8], offset: usize) -> Option<String> {
    let end = bytes[offset..]
        .iter()
        .position(|byte| *byte == 0)
        .map(|position| offset + position)?;
    std::str::from_utf8(&bytes[offset..end])
        .ok()
        .map(str::to_string)
}

fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    let bytes = bytes.get(offset..offset + 2)?;
    Some(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let bytes = bytes.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn machine_name(machine: u16) -> Option<&'static str> {
    match machine {
        0x014c => Some("x86"),
        0x8664 => Some("x86_64"),
        0xaa64 => Some("arm64"),
        _ => None,
    }
}

fn subsystem_name(subsystem: u16) -> Option<&'static str> {
    match subsystem {
        2 => Some("windows-gui"),
        3 => Some("windows-console"),
        _ => None,
    }
}
