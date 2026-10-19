use log::info;
use std::sync::Arc;
use tauri::{Manager as TauriManager, State};

use db::save_index::SaveEntry;
use domain::manager::Manager;
use ofm_core::game::Game;
use ofm_core::state::StateManager;

use crate::commands::util::persist_active_game;
use crate::SaveManagerState;

mod bootstrap;
mod helpers;
mod startup;
mod validation;
mod world_build;
mod world_load;

// Private globs, so a submodule's items need only `pub(super)` to be reachable
// from here and from the tests below. The names the rest of the crate calls are
// re-exported explicitly, and they are the only promise this module makes.
// No private glob for `bootstrap`: everything this module still calls is
// something `mcp_server/tools_impl/game.rs` drives a whole career opening
// through, so it needs a real path anyway.
pub(crate) use bootstrap::{delete_save_internal, persist_chosen_club};
#[cfg(feature = "mcp")]
pub(crate) use bootstrap::{
    start_career_for_mcp, McpCareerOptions, McpNewCareer, AUTO_START_NEEDS_CLUB_ERROR,
};
use helpers::*;
pub(crate) use helpers::{default_save_name, first_package_error_message};
pub(crate) use ofm_core::career::{begin_career, CareerScope};
use ofm_core::world::*;
use startup::*;
use world_build::*;
use world_load::*;
// Public, unlike the others: these are `#[tauri::command]`s and the types in
// their signatures. `commands/mod.rs` re-exports them again for the invoke
// handler in `lib.rs`, so the path must stay `crate::commands::<name>`.
pub use validation::*;

/// Step 1: Create manager + generate world. No team assigned yet.
/// Returns the Game object so the frontend can show team selection.
/// world_source: "random" (default) or a file path to a JSON world database.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn start_new_game(
    state: State<'_, Arc<StateManager>>,
    app_handle: tauri::AppHandle,
    first_name: String,
    last_name: String,
    dob: String,
    nationality: String,
    startup_options: Option<RawStartupOptions>,
    world_source: Option<String>,
    competition_definitions_json: Option<String>,
    package_ids: Option<Vec<String>>,
) -> Result<Game, String> {
    // Before any validation can return — see `StartRequestSummary` for why, and for what it omits.
    log::info!(
        "[cmd] start_new_game {}",
        startup::StartRequestSummary {
            first_name: &first_name,
            last_name: &last_name,
            dob: &dob,
            nationality: &nationality,
            startup_options: startup_options.as_ref(),
            world_source: world_source.as_deref(),
            package_ids: package_ids.as_deref(),
        }
        .describe()
    );

    // Validate inputs
    let first_name = first_name.trim().to_string();
    let last_name = last_name.trim().to_string();
    if first_name.is_empty() || last_name.is_empty() {
        return Err("be.error.createManager.nameRequired".to_string());
    }
    if first_name.len() > 30 || last_name.len() > 30 {
        return Err("be.error.createManager.nameMaxLength".to_string());
    }
    let nationality = nationality.trim().to_string();
    if nationality.is_empty() {
        return Err("be.error.createManager.nationalityRequired".to_string());
    }

    // Validate DOB against the selected career start date.
    let birth_date = chrono::NaiveDate::parse_from_str(&dob, "%Y-%m-%d")
        .map_err(|_| "be.error.createManager.invalidDobFormat".to_string())?;

    let startup_options = normalize_startup_options(startup_options)?;
    let (mut world, package_lockfile, authored_reputations) =
        if let Some(ids) = package_ids.as_deref().filter(|ids| !ids.is_empty()) {
            let packages_dir = app_handle
                .path()
                .app_data_dir()
                .map_err(|e| e.to_string())?
                .join("packages");
            // The career start year wins over the package's declared `baseYear`:
            // squads must be aged against the clock the player actually gets.
            let opening_year = u32::try_from(startup_options.start_year).ok();
            let asset_root = crate::commands::world::package_assets_dir(&app_handle).ok();
            load_world_data_from_package_ids(
                &packages_dir,
                ids,
                opening_year,
                asset_root.as_deref(),
                &definition_sources(&app_handle),
            )?
        } else {
            let (world, authored) =
                load_world_data(world_source.as_deref(), &definition_sources(&app_handle))?;
            (world, vec![], authored)
        };

    // Competitions are resolved, here and below, with clubs ranked by the package's authored
    // reputations; a world that did not come from a package has none and ranks by its own.
    let reputation_basis = ofm_core::generator::ReputationBasis::Authored(&authored_reputations);

    // Layer a user-picked standalone definition file onto the world. It is
    // validated strictly; the UI has already shown any details via
    // validate_competition_definitions.
    if let Some(json) = &competition_definitions_json {
        let file = parse_competition_definitions(json)?;
        if !validate_against_world(&file, &world, reputation_basis).is_empty() {
            return Err("be.error.competitionDef.invalidStandalone".to_string());
        }
        world.competition_definitions = Some(file);
    }

    let clock = game_clock_for_world(&startup_options, &world.metadata)?;
    let is_non_random = package_ids.as_deref().is_some_and(|ids| !ids.is_empty())
        || matches!(world_source.as_deref(), Some(source) if source != "random");
    let reference_date = clock.current_date.date_naive();
    if let Some(error) = manager_age_error(birth_date, reference_date) {
        return Err(error.to_string());
    }

    let manager = Manager::new(
        "mgr_user".to_string(),
        first_name,
        last_name,
        dob,
        nationality,
    );
    info!(
        "[cmd] start_new_game: {} {} (nationality={}, start_year={}, start_phase={}, history_depth_years={}, world_source={:?})",
        manager.first_name,
        manager.last_name,
        manager.nationality,
        startup_options.start_year,
        startup_options.start_phase.as_str(),
        startup_options.history_depth_years,
        world_source
    );

    // An imported world (a package or a world file) is made ready for a career as it is built, by
    // the one preparation `ofm_core` owns; a generated one is built as it is.
    let build = if is_non_random {
        build_game_from_imported_world_ranked
    } else {
        build_game_from_world_data_ranked
    };
    let built = build(clock, manager, &startup_options, world, reputation_basis);
    Ok(install_new_game(&state, built, package_lockfile))
}

/// Installs a freshly built game as the active career, and returns it as the screen must hold it
/// (with the career's token).
fn install_new_game(
    state: &StateManager,
    (mut new_game, stats_state): (Game, domain::stats::StatsState),
    package_lockfile: Vec<ofm_core::generator::PackageLock>,
) -> Game {
    new_game.package_lockfile = package_lockfile;
    info!(
        "[cmd] start_new_game: world generated with {} teams, {} players, {} staff",
        new_game.teams.len(),
        new_game.players.len(),
        new_game.staff.len()
    );
    crate::application::career::install_career_for_screen(state, new_game, stats_state, None)
}

/// Step 2: User picks a team. Assigns manager, generates welcome message, saves to DB.
#[tauri::command]
pub async fn select_team(
    state: State<'_, Arc<StateManager>>,
    sm_state: State<'_, Arc<SaveManagerState>>,
    team_id: String,
    active_region_ids: Option<Vec<String>>,
    active_competition_ids: Option<Vec<String>>,
) -> Result<Game, String> {
    info!("[cmd] select_team: team_id={}", team_id);
    let mut sm = map_save_manager_lock_error(sm_state.0.lock())?;
    select_team_internal(
        &state,
        &mut sm,
        &team_id,
        active_region_ids,
        active_competition_ids,
    )
}

/// Takes the club, saves the career and installs it; returns the game as the screen must hold it.
fn select_team_internal(
    state: &StateManager,
    sm: &mut db::save_manager::SaveManager,
    team_id: &str,
    active_region_ids: Option<Vec<String>>,
    active_competition_ids: Option<Vec<String>>,
) -> Result<Game, String> {
    let (mut game, loaded_save_id) = crate::application::career::snapshot_career(state)
        .ok_or("be.error.noActiveGameSession".to_string())?;
    let current_stats_state = state
        .get_stats_state(|stats| stats.clone())
        .unwrap_or_default();
    // `game` is a copy of the active game, so a refusal leaves the session untouched.
    let scope = CareerScope {
        regions: active_region_ids,
        competitions: active_competition_ids,
    };
    let stats_state = begin_career(&mut game, team_id, scope, current_stats_state)?;

    let save_id = persist_chosen_club(loaded_save_id.as_deref(), sm, &game, &stats_state)?;
    Ok(crate::application::career::install_career_for_screen(
        state,
        game,
        stats_state,
        Some(save_id),
    ))
}

#[tauri::command]
pub async fn get_saves(
    sm_state: State<'_, Arc<SaveManagerState>>,
) -> Result<Vec<SaveEntry>, String> {
    log::debug!("[cmd] get_saves");
    let mut sm = map_save_manager_lock_error(sm_state.0.lock())?;
    sm.load_saves()
}

#[tauri::command]
pub async fn delete_save(
    state: State<'_, Arc<StateManager>>,
    sm_state: State<'_, Arc<SaveManagerState>>,
    save_id: String,
) -> Result<bool, String> {
    info!("[cmd] delete_save: save_id={}", save_id);
    let mut sm = map_save_manager_lock_error(sm_state.0.lock())?;
    delete_save_internal(&state, &mut sm, &save_id)
}

#[tauri::command]
pub async fn load_game(
    state: State<'_, Arc<StateManager>>,
    sm_state: State<'_, Arc<SaveManagerState>>,
    save_id: String,
) -> Result<String, String> {
    info!("[cmd] load_game: save_id={}", save_id);
    let mut sm = map_save_manager_lock_error(sm_state.0.lock())?;
    let mut game = sm.load_game(&save_id)?;
    let stats_state = sm.load_stats_state(&save_id)?;
    ofm_core::ai_hiring::seed_ai_managers(&mut game);
    ofm_core::season_context::refresh_game_context(&mut game);

    let mgr_name = format!("{} {}", game.manager.first_name, game.manager.last_name);

    crate::application::career::install_career(&state, game, stats_state, Some(save_id));
    Ok(mgr_name)
}

/// Whether the active career still has to pick its first club, so the frontend only routes on the
/// answer and holds no copy of the rule.
#[tauri::command]
pub async fn get_career_stage(
    state: State<'_, Arc<StateManager>>,
) -> Result<ofm_core::career_stage::CareerStage, String> {
    get_career_stage_internal(&state)
}

/// The career stage of the active game, or the no-session error key when there is none.
pub fn get_career_stage_internal(
    state: &StateManager,
) -> Result<ofm_core::career_stage::CareerStage, String> {
    state
        .get_game(ofm_core::career_stage::career_stage)
        .ok_or("be.error.noActiveGameSession".to_string())
}

#[tauri::command]
pub async fn get_active_game(state: State<'_, Arc<StateManager>>) -> Result<Game, String> {
    log::debug!("[cmd] get_active_game");
    state
        .get_game(|g: &Game| g.clone())
        .ok_or("be.error.noActiveGameSession".to_string())
}

#[tauri::command]
pub async fn get_active_save_id(
    state: State<'_, Arc<StateManager>>,
) -> Result<Option<String>, String> {
    log::debug!("[cmd] get_active_save_id");
    Ok(state.get_save_id())
}

#[tauri::command]
pub async fn save_game(
    state: State<'_, Arc<StateManager>>,
    sm_state: State<'_, Arc<SaveManagerState>>,
) -> Result<(), String> {
    info!("[cmd] save_game");
    let mut sm = map_save_manager_lock_error(sm_state.0.lock())?;
    persist_active_game(&state, &mut sm)
}

/// Save the current game and clear the active session so the player returns to the main menu.
#[tauri::command]
pub async fn exit_to_menu(
    state: State<'_, Arc<StateManager>>,
    sm_state: State<'_, Arc<SaveManagerState>>,
) -> Result<(), String> {
    info!("[cmd] exit_to_menu");
    let mut sm = map_save_manager_lock_error(sm_state.0.lock())?;
    crate::application::saving::exit_to_menu(&state, &mut sm)?;
    Ok(())
}

/// World-building fixtures shared by the tests of several clusters in this
/// module. Kept apart so each peel of `game.rs` can take its own tests with it
/// without duplicating a fixture or reaching into a sibling.
#[cfg(test)]
mod testkit;

#[cfg(test)]
mod tests;
