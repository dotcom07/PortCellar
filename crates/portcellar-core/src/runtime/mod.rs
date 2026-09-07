use crate::*;
mod compatibility;
mod components;
mod dependencies;
mod launch;
mod observe;
mod preflight;
mod profile;
mod session;
mod smoke;
mod stage;
mod steam_cef;
mod wine;

pub use compatibility::engine_backend_matrix;
pub(crate) use compatibility::{
    engine_backend_candidates, engine_backend_compatibility, engine_backend_matrix_for_path,
    wine_engine_kind_for_path,
};
pub(crate) use components::runtime_components_for_engine;
pub use dependencies::{
    wine_bottle_mutation_plan, wine_bottle_rollback_plan, wine_bottle_snapshot_plan,
    wine_dependency_plans_for,
};
pub use launch::{
    game_installer_plan, game_launch_plan, game_launch_plan_with_stage, isaac_launch_plan,
    wine_steam_install_plan, wine_steam_login_plan, wine_steam_login_plan_for,
    wine_steam_stop_plan,
};
pub use observe::{game_runtime_ready, observe_game_runtime, steam_session_ready};
pub use preflight::{game_runtime_preflight, inspect_game_runtime_preflight};
pub use profile::{
    apply_game_runtime_profile, apply_isaac_runtime_profile, game_runtime_profile_plan,
    isaac_runtime_profile_plan,
};
pub use session::{
    apply_steam_session_reset, kill_wine_steam_processes, wine_steam_session_reset_plan,
};
pub use smoke::{
    game_smoke_evidence, load_game_compatibility_evidence, write_game_compatibility_evidence,
    write_game_smoke_evidence,
};
pub use stage::{prepare_game_runtime_stage, GameRuntimeStage};
pub use steam_cef::{
    apply_steam_cef_patch, wine_steam_cef_patch_plan, wine_steam_cef_patch_plan_for,
};
pub use wine::{wine_steam_configure_plans, wine_steam_configure_plans_for};

pub(crate) use launch::steam_wine_args;
#[cfg(test)]
pub(crate) use launch::*;
#[cfg(test)]
pub(crate) use profile::*;
#[cfg(test)]
pub(crate) use session::*;
pub(crate) use stage::game_runtime_stage_for;
pub(crate) use steam_cef::is_current_steamwebhelper_wrapper;
#[cfg(test)]
pub(crate) use steam_cef::*;
pub(crate) use wine::*;
