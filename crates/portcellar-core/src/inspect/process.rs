use std::path::Path;
use std::process::{Command, Stdio};

pub(crate) fn process_is_running(name: &str) -> bool {
    Command::new("/usr/bin/pgrep")
        .arg("-x")
        .arg(name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(test)]
pub(crate) fn process_command_matches_in_ps(output: &str, needles: &[&str]) -> bool {
    output
        .lines()
        .any(|line| process_command_line_matches(line, needles))
}

pub(crate) fn process_command_matches_in_prefix(
    output: &str,
    prefix: &Path,
    needles: &[&str],
) -> bool {
    process_ids_matching_in_ps(output, needles)
        .into_iter()
        .any(|pid| process_uses_prefix(pid, prefix))
}

pub(crate) fn process_command_line_matches(line: &str, needles: &[&str]) -> bool {
    let lower = line.to_ascii_lowercase();
    needles
        .iter()
        .all(|needle| lower.contains(&needle.to_ascii_lowercase()))
        && !is_process_observer_command(line)
}

#[cfg(test)]
pub(crate) fn wine_steam_cleanup_process_ids_from_ps(output: &str) -> Vec<u32> {
    output
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let (pid, command) = line.split_once(char::is_whitespace)?;
            let pid = pid.parse::<u32>().ok()?;
            (process_command_line_matches(command, &[])
                && is_wine_steam_cleanup_process_command(command))
            .then_some(pid)
        })
        .collect()
}

pub(crate) fn wine_steam_cleanup_process_groups_from_ps(output: &str) -> Vec<(u32, u32)> {
    output
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let pid = fields.next()?.parse::<u32>().ok()?;
            let pgid = fields.next()?.parse::<u32>().ok()?;
            let command = fields.collect::<Vec<_>>().join(" ");
            (is_wine_steam_cleanup_process_command(&command)).then_some((pid, pgid))
        })
        .collect()
}

pub(crate) fn process_ids_matching_in_ps(output: &str, needles: &[&str]) -> Vec<u32> {
    output
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let (pid, command) = line.split_once(char::is_whitespace)?;
            let pid = pid.parse::<u32>().ok()?;
            process_command_line_matches(command, needles).then_some(pid)
        })
        .collect()
}

pub(crate) fn process_uses_prefix(pid: u32, prefix: &Path) -> bool {
    let args = [
        "-a".into(),
        "-p".into(),
        pid.to_string().into(),
        "-Fn".into(),
    ];
    let Some(output) = super::command_stdout("/usr/sbin/lsof", &args) else {
        return false;
    };
    let prefix = prefix.display().to_string();
    output.lines().any(|line| line.contains(&prefix))
}

pub(crate) fn is_wine_steam_process_command(command: &str) -> bool {
    let lower = command.to_ascii_lowercase();
    lower.starts_with("c:\\steam\\")
        || lower.contains(" c:\\steam\\")
        || lower.starts_with("c:\\program files (x86)\\steam\\")
        || lower.contains(" c:\\program files (x86)\\steam\\")
        || lower.starts_with("c:\\program files\\steam\\")
        || lower.contains(" c:\\program files\\steam\\")
        || lower.starts_with("c:\\windows\\system32\\explorer.exe /desktop")
}

fn is_wine_steam_cleanup_process_command(command: &str) -> bool {
    let lower = command.to_ascii_lowercase();
    is_wine_steam_process_command(command)
        || lower.contains("\\steamservice.exe")
        || lower.contains("\\winedevice.exe")
}

pub(crate) fn is_process_observer_command(line: &str) -> bool {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();

    lower.starts_with("/bin/zsh -lc ")
        || lower.starts_with("/bin/bash -lc ")
        || lower.contains(" rg ")
        || lower.contains("/rg ")
        || lower.starts_with("rg ")
        || lower.contains("cargo run")
        || lower.contains("target/debug/portcellar")
        || lower.contains("ps ax")
        || lower.contains("ps -axo")
}
