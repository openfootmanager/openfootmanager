use log::info;
use std::sync::Arc;
use tauri::{Manager as TauriManager, State};

use chrono::Datelike;

use db::save_index::SaveEntry;
use domain::manager::Manager;
use ofm_core::game::Game;
use ofm_core::state::StateManager;

use crate::commands::util::persist_active_game;
use crate::SaveManagerState;

mod bootstrap;
mod foundation;
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
#[cfg(feature = "mcp")]
pub(crate) use bootstrap::bootstrap_game_for_mcp;
pub(crate) use bootstrap::{bootstrap_team_selection, create_new_save};
use foundation::*;
use helpers::*;
pub(crate) use helpers::{default_save_name, first_package_error_message};
use startup::*;
pub(crate) use startup::{start_phase_for_game, StartPhase};
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
    let (mut world, package_lockfile) =
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
            (
                load_world_data(world_source.as_deref(), &definition_sources(&app_handle))?,
                vec![],
            )
        };

    // Layer a user-picked standalone definition file onto the world. It is
    // validated strictly; the UI has already shown any details via
    // validate_competition_definitions.
    if let Some(json) = &competition_definitions_json {
        let file = parse_competition_definitions(json)?;
        if !validate_against_world(&file, &world).is_empty() {
            return Err("be.error.competitionDef.invalidStandalone".to_string());
        }
        world.competition_definitions = Some(file);
    }

    let clock = game_clock_for_world(&startup_options, &world.metadata)?;
    let is_non_random = package_ids.as_deref().is_some_and(|ids| !ids.is_empty())
        || matches!(world_source.as_deref(), Some(source) if source != "random");
    if is_non_random {
        // Staff generated to fill an imported world are aged against the year the
        // career actually opens in, so a historical world does not get a backroom
        // team born decades after the season it models.
        let opening_year = u32::try_from(clock.start_date.year())
            .unwrap_or_else(|_| ofm_core::generator::default_opening_year());
        ofm_core::generator::normalize_imported_world_for_career_start(&mut world, opening_year);
    }
    let reference_date = clock.current_date.date_naive();
    let age = age_on_date(birth_date, reference_date);
    if age < 30 {
        return Err("be.error.createManager.minAge".to_string());
    }
    if age > 99 {
        return Err("be.error.createManager.invalidDob".to_string());
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

    let (mut new_game, stats_state) =
        build_game_from_world_data(clock, manager, &startup_options, world);

    new_game.package_lockfile = package_lockfile;

    info!(
        "[cmd] start_new_game: world generated with {} teams, {} players, {} staff",
        new_game.teams.len(),
        new_game.players.len(),
        new_game.staff.len()
    );
    state.set_game(new_game.clone());
    state.set_stats_state(stats_state);
    Ok(new_game)
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
    let mut game = state
        .get_game(|g: &Game| g.clone())
        .ok_or("be.error.noActiveGameSession".to_string())?;
    let current_stats_state = state
        .get_stats_state(|stats| stats.clone())
        .unwrap_or_default();
    ensure_multi_competition_foundations(&mut game);

    // Hemisphere fix: when the player picks SeasonStart for a southern-
    // hemisphere (or other non-August-start) club, align the game clock to
    // that club's actual season-start date and rebuild competitions from that
    // anchor so the player arrives at the beginning of their season, not July.
    if start_phase_for_game(&game) == StartPhase::SeasonStart {
        if let Some(actual_start) = team_season_anchor(&game, &team_id) {
            if actual_start < game.clock.current_date {
                game.clock.current_date = actual_start;
                game.clock.start_date = actual_start;
                rebuild_competitions_for_management_date(&mut game, actual_start);
                game.national_teams.clear();
                ensure_multi_competition_foundations(&mut game);
            }
        }
    }

    let (resolved_region_ids, resolved_competition_ids) =
        resolve_simulation_scope(&game, &team_id, active_region_ids, active_competition_ids)?;
    game.active_region_ids = resolved_region_ids;
    game.active_competition_ids = resolved_competition_ids;

    let start_phase = start_phase_for_game(&game);
    let stats_state =
        bootstrap_team_selection(&mut game, &team_id, start_phase, current_stats_state)?;

    // Upgrade generic (legacy-bucket) positions to granular on new-game creation
    // so the frontend sees the same granular positions immediately, rather than
    // only after the first save/reload cycle (where load_game applies this same
    // upgrade).
    ofm_core::player_identity::upgrade_game_player_identities(&mut game);

    // Save to new per-save DB
    let manager_name = format!("{} {}", game.manager.first_name, game.manager.last_name);
    let save_name = default_save_name(&manager_name);

    let mut sm = map_save_manager_lock_error(sm_state.0.lock())?;
    let save_id = create_new_save(&mut sm, &game, &stats_state, &save_name)?;
    state.set_save_id(save_id);

    state.set_game(game.clone());
    state.set_stats_state(stats_state);
    Ok(game)
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
    sm_state: State<'_, Arc<SaveManagerState>>,
    save_id: String,
) -> Result<bool, String> {
    info!("[cmd] delete_save: save_id={}", save_id);
    let mut sm = map_save_manager_lock_error(sm_state.0.lock())?;
    sm.delete_save(&save_id)
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

    state.set_save_id(save_id);
    state.set_game(game);
    state.set_stats_state(stats_state);
    Ok(mgr_name)
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
    if state.get_save_id().is_some_and(|id| !id.is_empty()) {
        let mut sm = map_save_manager_lock_error(sm_state.0.lock())?;
        persist_active_game(&state, &mut sm)?;
    }

    // Clear the in-memory game state
    state.clear_game();
    state.clear_save_id();

    Ok(())
}

/// World-building fixtures shared by the tests of several clusters in this
/// module. Kept apart so each peel of `game.rs` can take its own tests with it
/// without duplicating a fixture or reaching into a sibling.
#[cfg(test)]
mod testkit;

#[cfg(test)]
mod tests;
