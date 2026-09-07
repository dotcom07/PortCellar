use super::*;
use crate::*;

pub fn doctor_report() -> DoctorReport {
    let profile = isaac_profile();
    doctor_report_for(&profile, latest_isaac_crash())
}

pub(crate) fn doctor_report_for(
    profile: &dyn GameProfile,
    latest_crash: Option<CrashSummary>,
) -> DoctorReport {
    let host = inspect_host();
    let steam = inspect_steam();
    let game = find_game_install(&steam, profile);
    let wine_steam = inspect_wine_steam_runtime_for(profile);
    let engines = inspect_engines(profile, game.as_ref(), &steam, &wine_steam);

    DoctorReport {
        host,
        steam,
        game,
        wine_steam,
        engines,
        latest_crash,
    }
}
