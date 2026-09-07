use super::*;
use crate::*;
use std::fs;
use std::path::PathBuf;

pub(crate) fn latest_isaac_crash() -> Option<CrashSummary> {
    let reports = home_dir()?.join("Library/Logs/DiagnosticReports");
    let mut latest = None;

    for entry in fs::read_dir(reports).ok()?.flatten() {
        let path = entry.path();
        let name = path.file_name()?.to_string_lossy();
        if !name.starts_with("The Binding of Isaac Rebirth") || !name.ends_with(".ips") {
            continue;
        }

        let modified = entry.metadata().ok()?.modified().ok()?;
        if latest
            .as_ref()
            .map(|(current, _): &(std::time::SystemTime, PathBuf)| modified > *current)
            .unwrap_or(true)
        {
            latest = Some((modified, path));
        }
    }

    let (_, path) = latest?;
    summarize_crash(path)
}

pub(crate) fn summarize_crash(path: PathBuf) -> Option<CrashSummary> {
    let text = fs::read_to_string(&path).ok()?;
    let exception = line_containing(&text, "\"exception\"");
    let termination = line_containing(&text, "\"termination\"");
    let translated = if text.contains("\"translated\" : true") {
        Some(true)
    } else if text.contains("\"translated\" : false") {
        Some(false)
    } else {
        None
    };
    let cpu_type = value_after_colon(&text, "\"cpuType\"");
    let suspect_symbols = ["__ARCLite__load()", "CGlobalInitter::CGlobalInitter()"]
        .into_iter()
        .filter(|symbol| text.contains(symbol))
        .map(str::to_string)
        .collect::<Vec<_>>();
    let suspect_images = ["steamloader.dylib", "gameoverlayrenderer.dylib"]
        .into_iter()
        .filter(|image| text.contains(image))
        .map(str::to_string)
        .collect::<Vec<_>>();
    let diagnosis = crash_diagnosis(&exception, &suspect_symbols, &suspect_images);

    Some(CrashSummary {
        path,
        exception,
        termination,
        translated,
        cpu_type,
        suspect_symbols,
        suspect_images,
        diagnosis,
    })
}

pub(crate) fn crash_diagnosis(
    exception: &Option<String>,
    suspect_symbols: &[String],
    suspect_images: &[String],
) -> Option<String> {
    if suspect_symbols
        .iter()
        .any(|symbol| symbol == "__ARCLite__load()")
    {
        return Some(
            "native macOS Isaac is crashing in the legacy ARC-lite initializer under Rosetta"
                .to_string(),
        );
    }

    if suspect_symbols
        .iter()
        .any(|symbol| symbol == "CGlobalInitter::CGlobalInitter()")
        && suspect_images
            .iter()
            .any(|image| image == "steamloader.dylib" || image == "gameoverlayrenderer.dylib")
    {
        return Some("Steam loader/overlay is present in the crash path".to_string());
    }

    if exception
        .as_deref()
        .map(|line| line.contains("EXC_BAD_INSTRUCTION") || line.contains("SIGILL"))
        .unwrap_or(false)
    {
        return Some("native x86_64 process hit an illegal instruction under Rosetta".to_string());
    }

    None
}

pub(crate) fn line_containing(text: &str, needle: &str) -> Option<String> {
    text.lines()
        .find(|line| line.contains(needle))
        .map(|line| line.trim().to_string())
}

pub(crate) fn value_after_colon(text: &str, key: &str) -> Option<String> {
    let line = line_containing(text, key)?;
    let (_, value) = line.split_once(':')?;
    Some(
        value
            .trim()
            .trim_end_matches(',')
            .trim_matches('"')
            .to_string(),
    )
}
