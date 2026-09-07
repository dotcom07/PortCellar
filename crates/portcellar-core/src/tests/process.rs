use super::*;

#[test]
fn marks_runtime_log_status_as_last_when_stopped() {
    assert_eq!(
        last_status_when_stopped("no-connection".to_string(), false),
        "last no-connection"
    );
    assert_eq!(
        last_status_when_stopped("no-connection".to_string(), true),
        "no-connection"
    );
    assert_eq!(
        last_status_when_stopped("last no-connection".to_string(), false),
        "last no-connection"
    );
}

#[test]
fn process_matching_ignores_observer_commands() {
    assert!(process_command_line_matches(
        "C:\\Steam\\steamapps\\common\\The Binding of Isaac Rebirth\\isaac-ng.exe",
        &[ISAAC_WINDOWS_EXE]
    ));
    assert!(process_command_matches_in_ps(
        "c:\\steam\\steam.exe -silent\n",
        &["Steam.exe"]
    ));
    assert!(!process_command_line_matches(
        "/bin/zsh -lc ps axww | rg 'C:\\\\Steam|steamwebhelper|isaac-ng.exe'",
        &[ISAAC_WINDOWS_EXE]
    ));
    assert!(!process_command_line_matches(
        "rg C:\\\\Steam|steamwebhelper|isaac-ng.exe",
        &[ISAAC_WINDOWS_EXE]
    ));
}

#[test]
fn pid_aware_process_matching_requires_host_prefix_ownership() {
    let ps = "100 C:\\Steam\\steam.exe -silent\n101 C:\\Steam\\steamwebhelper.exe\n";

    assert_eq!(process_ids_matching_in_ps(ps, &["steam.exe"]), vec![100]);
    assert_eq!(
        process_ids_matching_in_ps(ps, &["steamwebhelper.exe"]),
        vec![101]
    );
}

#[test]
fn cleanup_candidates_include_prefix_owned_helpers() {
    let ps = "\
      100 C:\\Steam\\steam.exe -silent\n\
      104 C:\\Program Files (x86)\\Steam\\steam.exe -silent\n\
      101 C:\\Program Files (x86)\\Common Files\\Steam\\steamservice.exe /RunAsService\n\
      102 C:\\windows\\system32\\winedevice.exe\n\
      103 /Applications/Steam.app/Contents/MacOS/steam_osx\n";

    assert_eq!(
        wine_steam_cleanup_process_ids_from_ps(ps),
        vec![100, 104, 101, 102]
    );
}

#[test]
fn cleanup_process_groups_preserve_pid_and_pgid_columns() {
    let ps = "\
      100 100 C:\\Steam\\steam.exe -silent\n\
      101 100 C:\\Steam\\steamwebhelper.exe\n\
      102 102 C:\\windows\\system32\\winedevice.exe\n\
      103 1 /Applications/Steam.app/Contents/MacOS/steam_osx\n";

    assert_eq!(
        wine_steam_cleanup_process_groups_from_ps(ps),
        vec![(100, 100), (101, 100), (102, 102)]
    );
}

#[test]
fn webhelper_only_is_not_a_healthy_steam_process() {
    assert!(wine_steam_running_status(true, false, None));
    assert!(wine_steam_running_status(false, true, Some(true)));
    assert!(!wine_steam_running_status(false, true, Some(false)));
    assert!(!wine_steam_running_status(false, true, None));
}
