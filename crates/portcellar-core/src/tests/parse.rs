use super::*;

#[test]
fn parses_quoted_key_value_lines() {
    let text = "\"appid\"\t\t\"250900\"\n\"installdir\"\t\t\"The Binding of Isaac Rebirth\"";
    let values = parse_key_values(text);
    assert_eq!(values.get("appid"), Some(&"250900".to_string()));
    assert_eq!(
        values.get("installdir"),
        Some(&"The Binding of Isaac Rebirth".to_string())
    );
}

#[test]
fn shell_quote_keeps_simple_tokens_plain() {
    assert_eq!(
        shell_quote("steam://rungameid/250900"),
        "steam://rungameid/250900"
    );
    assert_eq!(
        shell_quote("/tmp/The Binding.app"),
        "'/tmp/The Binding.app'"
    );
}

#[test]
fn steam_install_uri_requires_a_numeric_app_id() {
    assert_eq!(
        steam_install_uri("250900").unwrap(),
        "steam://install/250900"
    );
    assert!(steam_install_uri("250900/extra").is_err());
}
