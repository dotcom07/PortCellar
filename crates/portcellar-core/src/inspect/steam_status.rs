use super::*;
use crate::*;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub(crate) fn steam_login_state(steam_dir: &Path) -> Option<bool> {
    let loginusers = steam_dir.join("config/loginusers.vdf");
    let text = fs::read_to_string(loginusers).ok()?;
    let values = parse_key_values(&text);
    Some(
        quoted_bool(&values, "RememberPassword")
            || quoted_bool(&values, "AllowAutoLogin")
            || quoted_bool(&values, "MostRecent"),
    )
}

pub(crate) fn steam_logged_in_status(
    active_session: Option<bool>,
    cached_credentials: Option<bool>,
    login_status: Option<&str>,
) -> Option<bool> {
    if active_session == Some(true) {
        return Some(true);
    }

    match login_status {
        Some("success") => Some(true),
        Some("auth-poll-transport-error" | "logon-failure" | "waiting-for-credentials") => {
            Some(false)
        }
        _ => active_session.or(cached_credentials),
    }
}

pub(crate) fn quoted_bool(values: &BTreeMap<String, String>, key: &str) -> bool {
    values.get(key).map(|value| value == "1").unwrap_or(false)
}

pub(crate) fn steam_login_status(steam_dir: &Path) -> Option<String> {
    let login_log_text = fs::read_to_string(steam_dir.join("logs/steamui_login.txt")).ok()?;
    let login_log = latest_log_segment(&login_log_text, "Client version:");
    let webhelper_js_text =
        fs::read_to_string(steam_dir.join("logs/webhelper_js.txt")).unwrap_or_default();
    let webhelper_js = latest_log_segment(&webhelper_js_text, "Client version:");

    let mut status = None;
    for line in login_log.lines() {
        if line.contains("Received logon success response")
            || line.contains("SetLoginState: Success")
            || line.contains("Resolve login request: OK - Success")
        {
            status = Some("success".to_string());
        } else if line.contains("Received logon failure response") {
            status = Some("logon-failure".to_string());
        } else if line.contains("SetLoginState: WaitingForCredentials") {
            status = Some("waiting-for-credentials".to_string());
        }
    }

    if status.as_deref() != Some("success")
        && webhelper_js.contains("Login: Failed to poll auth session")
    {
        status = Some("auth-poll-transport-error".to_string());
    }

    status
}

pub(crate) fn steam_connection_status(steam_dir: &Path) -> Option<String> {
    let mut status = None;
    for log_name in ["connection_log.txt", "console_log.txt"] {
        let text = fs::read_to_string(steam_dir.join("logs").join(log_name)).unwrap_or_default();
        let text = latest_log_segment(&text, "Client version:");
        for line in text.lines() {
            if line.contains("LogonFailure No Connection")
                || line.contains("SteamServerConnectFailure_t No Connection")
            {
                status = Some("no-connection".to_string());
            } else if line.contains("Try another CM") {
                status = Some("try-another-cm".to_string());
            } else if line.contains("ConnectionDisconnected") {
                status = Some("disconnected".to_string());
            } else if line.contains("ConnectionCompleted()")
                || line.contains("Client thinks it can connect via")
                || line.contains("[Connected,")
            {
                status = Some("cm-transport-ready".to_string());
            } else if line.contains("Connectivity test: result=Connected") {
                status = Some("internet-connected".to_string());
            }
        }
    }

    if matches!(status.as_deref(), None | Some("internet-connected"))
        && steam_webui_transport_ready(steam_dir)
    {
        status = Some("webui-transport-ready".to_string());
    }

    status
}

pub(crate) fn steam_webui_transport_ready(steam_dir: &Path) -> bool {
    [
        "webhelper_js.txt",
        "transport_client.txt",
        "transport_steamui.txt",
    ]
    .into_iter()
    .map(|log_name| fs::read_to_string(steam_dir.join("logs").join(log_name)).unwrap_or_default())
    .map(|text| latest_log_segment_owned(text, "Client version:"))
    .any(|text| {
        text.contains("WebUITransportStore: Connection status: connected")
            || text.contains("CWebSocketConnection (steamUI): connection ready")
            || text.contains("CWebSocketConnection (clientdll): connection ready")
            || text
                .contains("WebUITransport: Websocket connection from: https://steamloopback.host")
    })
}

pub(crate) fn latest_log_segment_owned(text: String, marker: &str) -> String {
    let index = text
        .rmatch_indices(marker)
        .next()
        .map(|(index, _)| index)
        .unwrap_or(0);
    text[index..].to_string()
}

#[cfg(test)]
pub(crate) fn steam_webhelper_session_state_from_processes(output: &str) -> Option<bool> {
    let mut saw_logged_out_helper = false;

    for line in output.lines() {
        if !line.to_ascii_lowercase().contains("steamwebhelper.exe") {
            continue;
        }

        let Some(value) = token_value(line, "-steamid=") else {
            continue;
        };
        if value != "0" {
            return Some(true);
        }
        saw_logged_out_helper = true;
    }

    saw_logged_out_helper.then_some(false)
}

pub(crate) fn steam_webhelper_session_state_from_processes_in_prefix(
    output: &str,
    prefix: &Path,
) -> Option<bool> {
    let mut saw_logged_out_helper = false;

    for line in output.lines() {
        if !line.to_ascii_lowercase().contains("steamwebhelper.exe") {
            continue;
        }
        let Some((pid, _)) = line.trim().split_once(char::is_whitespace) else {
            continue;
        };
        let Ok(pid) = pid.parse::<u32>() else {
            continue;
        };
        if !process_uses_prefix(pid, prefix) {
            continue;
        }

        let Some(value) = token_value(line, "-steamid=") else {
            continue;
        };
        if value != "0" {
            return Some(true);
        }
        saw_logged_out_helper = true;
    }

    saw_logged_out_helper.then_some(false)
}

pub(crate) fn token_value<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    let start = line.find(prefix)? + prefix.len();
    line[start..].split_whitespace().next()
}

pub(crate) fn steam_cef_status(
    steam_dir: &Path,
    mac_driver: &WineMacDriverConfig,
    steam_running: bool,
) -> Option<String> {
    let cef_log_text = fs::read_to_string(steam_dir.join("logs/cef_log.txt")).unwrap_or_default();
    let html_log_text =
        fs::read_to_string(steam_dir.join("logs/steamui_html.txt")).unwrap_or_default();
    let webhelper_log_text =
        fs::read_to_string(steam_dir.join("logs/webhelper.txt")).unwrap_or_default();
    let cef_log = latest_log_segment(&cef_log_text, "Client version:");
    let html_log = latest_log_segment(&html_log_text, "Client version:");
    let webhelper_log = latest_log_segment(&webhelper_log_text, "Client version:");

    if cef_log.contains("FATAL:process_metrics_win") {
        return Some(if steam_running {
            "steamwebhelper fatal: process_metrics_win".to_string()
        } else {
            "last steamwebhelper fatal: process_metrics_win".to_string()
        });
    }
    if html_log.matches("Restart webhelper process").count() >= 3 {
        return Some(if steam_running {
            "steamwebhelper restart loop".to_string()
        } else {
            "last steamwebhelper restart loop".to_string()
        });
    }
    if webhelper_log.contains("SP DesktopLoginWindow")
        && webhelper_log.contains("805240832,805240832")
    {
        if mac_driver
            .allow_immovable_windows
            .as_deref()
            .map(|value| value.eq_ignore_ascii_case("N"))
            .unwrap_or(false)
        {
            return Some("login-window-ready-macdrv-remapped".to_string());
        }
        return Some("login-window-ready-offscreen".to_string());
    }
    if html_log.contains("BrowserReady") || webhelper_log.contains("SP DesktopLoginWindow") {
        return Some("login-window-ready".to_string());
    }
    if webhelper_log.contains("Startup - webhelper launched") {
        return Some("steamwebhelper running".to_string());
    }
    if cef_log.contains("eglCreateContext") {
        return Some("steamwebhelper graphics context warnings".to_string());
    }
    None
}

pub(crate) fn latest_log_segment<'a>(text: &'a str, marker: &str) -> &'a str {
    text.rmatch_indices(marker)
        .next()
        .map(|(index, _)| &text[index..])
        .unwrap_or(text)
}

pub(crate) fn last_status_when_stopped(status: String, running: bool) -> String {
    if running || status.starts_with("last ") {
        status
    } else {
        format!("last {status}")
    }
}
