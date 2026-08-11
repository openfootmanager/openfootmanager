use log::info;
use std::sync::Arc;
use tauri::{Manager as TauriManager, State};

use chrono::Datelike;

use db::{save_index::SaveEntry, save_manager::SaveManager};
use domain::manager::Manager;
use domain::stats::StatsState;
use ofm_core::game::Game;
use ofm_core::state::StateManager;

use crate::commands::util::persist_active_game;
use crate::SaveManagerState;

mod foundation;
mod helpers;
mod startup;
mod validation;
mod world_build;
mod world_load;

// Private globs, so a submodule's items need only `pub(super)` to be reachable
// from here and from the tests below. The names the rest of the crate calls are
// re-exported explicitly, and they are the only promise this module makes.
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

fn has_existing_world_context(game: &Game, stats_state: &StatsState) -> bool {
    !game.competitions.is_empty()
        || game.league.is_some()
        || !game.news.is_empty()
        || !stats_state.player_matches.is_empty()
        || !stats_state.team_matches.is_empty()
}

fn bootstrap_existing_world_takeover(
    game: &mut Game,
    team_id: &str,
    stats_state: StatsState,
) -> Result<StatsState, String> {
    let team = game
        .teams
        .iter()
        .find(|t| t.id == team_id)
        .ok_or("be.error.teamNotFound".to_string())?;
    let team_name = team.name.clone();

    ofm_core::ai_hiring::seed_ai_managers(game);

    let takeover_date = game.clock.current_date.format("%Y-%m-%d").to_string();
    let incumbent_manager_id = game
        .teams
        .iter()
        .find(|candidate| candidate.id == team_id)
        .and_then(|candidate| candidate.manager_id.clone());

    if incumbent_manager_id.as_deref() != Some(game.manager.id.as_str()) {
        let fired = ofm_core::firing::fire_ai_manager_for_team(game, team_id, &takeover_date);
        if !fired {
            if let Some(team) = game
                .teams
                .iter_mut()
                .find(|candidate| candidate.id == team_id)
            {
                team.manager_id = None;
            }
        }
        ofm_core::job_offers::hire_manager(game, team_id, &takeover_date)?;
    }

    let staff_msg = ofm_core::messages::staff_advice_message(&team_name, team_id, &takeover_date);
    game.messages.push(staff_msg);
    ofm_core::player_events::generate_takeover_contract_review_message(game);
    ofm_core::season_context::refresh_game_context(game);

    Ok(stats_state)
}

pub(crate) fn create_new_save(
    save_manager: &mut SaveManager,
    game: &Game,
    stats_state: &StatsState,
    save_name: &str,
) -> Result<String, String> {
    save_manager.create_save_with_stats(game, stats_state, save_name)
}

fn bootstrap_season_start(game: &mut Game, team_id: &str) -> Result<StatsState, String> {
    let team = game
        .teams
        .iter()
        .find(|t| t.id == team_id)
        .ok_or("be.error.teamNotFound".to_string())?;
    let team_name = team.name.clone();

    game.manager.hire(team_id.to_string());
    if let Some(t) = game.teams.iter_mut().find(|t| t.id == team_id) {
        t.manager_id = Some(game.manager.id.clone());
    }
    game.manager_id = game.manager.id.clone();
    ofm_core::ai_hiring::seed_ai_managers(game);

    let season_start = preseason_season_start(&game.clock);
    let team_ids: Vec<String> = game.teams.iter().map(|t| t.id.clone()).collect();
    let mut league = ofm_core::schedule::generate_league(
        DEFAULT_LEAGUE_NAME,
        preseason_league_year(&game.clock),
        &team_ids,
        season_start,
    );
    league.name_key = Some(DEFAULT_LEAGUE_NAME_KEY.to_string());
    let friendlies = ofm_core::schedule::generate_preseason_friendlies(&team_ids, season_start, 4);
    ofm_core::schedule::append_fixtures(&mut league, friendlies);
    game.league = Some(league);
    ofm_core::season_context::refresh_game_context(game);

    let date_str = game.clock.current_date.to_rfc3339();
    let welcome_msg = ofm_core::messages::welcome_message(&team_name, team_id, &date_str);
    game.messages.push(welcome_msg);

    // Both params are resolved frontend-side: the league name is a translation
    // key, and the ISO date is formatted in the player's locale.
    let season_msg = ofm_core::messages::season_schedule_message(
        DEFAULT_LEAGUE_NAME_KEY,
        &season_start.format(ISO_DATE_FORMAT).to_string(),
        &date_str,
    );
    game.messages.push(season_msg);

    let team_names: Vec<String> = game.teams.iter().map(|team| team.name.clone()).collect();
    game.news.push(ofm_core::news::season_preview_article(
        &team_names,
        &date_str,
    ));

    let staff_msg = ofm_core::messages::staff_advice_message(&team_name, team_id, &date_str);
    game.messages.push(staff_msg);

    ofm_core::player_events::generate_takeover_contract_review_message(game);

    Ok(StatsState::default())
}

fn competitive_fixture_count_for_team(game: &Game, team_id: &str) -> usize {
    game.league
        .as_ref()
        .map(|league| {
            league
                .fixtures
                .iter()
                .filter(|fixture| {
                    fixture.counts_for_league_standings()
                        && (fixture.home_team_id == team_id || fixture.away_team_id == team_id)
                })
                .count()
        })
        .unwrap_or_default()
}

fn completed_competitive_fixture_count_for_team(game: &Game, team_id: &str) -> usize {
    game.league
        .as_ref()
        .map(|league| {
            league
                .fixtures
                .iter()
                .filter(|fixture| {
                    fixture.counts_for_league_standings()
                        && fixture.status == domain::league::FixtureStatus::Completed
                        && (fixture.home_team_id == team_id || fixture.away_team_id == team_id)
                })
                .count()
        })
        .unwrap_or_default()
}

fn bootstrap_midseason_takeover(game: &mut Game, team_id: &str) -> Result<StatsState, String> {
    let team = game
        .teams
        .iter()
        .find(|t| t.id == team_id)
        .ok_or("be.error.teamNotFound".to_string())?;
    let team_name = team.name.clone();

    ofm_core::ai_hiring::seed_ai_managers(game);

    let season_start = preseason_season_start(&game.clock);
    let team_ids: Vec<String> = game.teams.iter().map(|t| t.id.clone()).collect();
    let mut league = ofm_core::schedule::generate_league(
        DEFAULT_LEAGUE_NAME,
        preseason_league_year(&game.clock),
        &team_ids,
        season_start,
    );
    league.name_key = Some(DEFAULT_LEAGUE_NAME_KEY.to_string());
    game.league = Some(league);
    game.clock.current_date = season_start;
    ofm_core::season_context::refresh_game_context(game);

    let total_fixtures = competitive_fixture_count_for_team(game, team_id);
    let target_completed = (total_fixtures / 2).max(1);
    let mut stats_state = StatsState::default();
    let mut safeguard_days = 0usize;
    while completed_competitive_fixture_count_for_team(game, team_id) < target_completed {
        let mut captures = Vec::new();
        ofm_core::turn::process_day_with_capture(game, &mut |capture| captures.push(capture));
        for capture in captures {
            stats_state.append(capture);
        }
        safeguard_days += 1;
        if safeguard_days > 240 {
            break;
        }
    }

    let takeover_date = game.clock.current_date.format("%Y-%m-%d").to_string();
    let _ = ofm_core::firing::fire_ai_manager_for_team(game, team_id, &takeover_date);
    ofm_core::job_offers::hire_manager(game, team_id, &takeover_date)?;

    let staff_msg = ofm_core::messages::staff_advice_message(&team_name, team_id, &takeover_date);
    game.messages.push(staff_msg);
    ofm_core::player_events::generate_takeover_contract_review_message(game);
    ofm_core::season_context::refresh_game_context(game);

    Ok(stats_state)
}

pub(crate) fn bootstrap_team_selection(
    game: &mut Game,
    team_id: &str,
    start_phase: StartPhase,
    stats_state: StatsState,
) -> Result<StatsState, String> {
    let stats_state = if has_existing_world_context(game, &stats_state) {
        bootstrap_existing_world_takeover(game, team_id, stats_state)?
    } else {
        match start_phase {
            StartPhase::SeasonStart => bootstrap_season_start(game, team_id)?,
            StartPhase::MidSeason => bootstrap_midseason_takeover(game, team_id)?,
        }
    };

    ofm_core::transfers::seed_opening_ai_loan_market(game);
    Ok(stats_state)
}

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

/// Bootstrap a game for MCP auto-start.
/// Creates a manager, loads world, selects team, and saves.
/// Returns the save ID.
#[cfg(feature = "mcp")]
pub fn bootstrap_game_for_mcp(
    state_manager: &StateManager,
    save_manager_state: &crate::SaveManagerState,
    world_path: &str,
    team_id: Option<&str>,
    manager_first_name: &str,
    manager_last_name: &str,
    manager_nationality: &str,
) -> Result<String, String> {
    // Step 1: Load world data
    let mut world = load_world_data_from_path(world_path)?;

    // Normalize imported world for career start (same as start_new_game does for non-random imports)
    let bootstrap_opening_year = normalize_startup_options(None)
        .ok()
        .and_then(|options| game_clock_for_world(&options, &world.metadata).ok())
        .and_then(|clock| u32::try_from(clock.start_date.year()).ok())
        .unwrap_or_else(ofm_core::generator::default_opening_year);
    ofm_core::generator::normalize_imported_world_for_career_start(
        &mut world,
        bootstrap_opening_year,
    );

    // Step 2: Find the existing user manager in the world data.
    // HistoricalSnapshot exports include the user manager (id "mgr_user") already
    // assigned to their team. Reusing it preserves the team assignment, career
    // history, and all manager state — no takeover/hiring logic needed.
    // If not found (e.g. RosterBaseline world), fall back to creating a fresh one.
    let manager = if let Some(idx) = world.managers.iter().position(|m| m.id == "mgr_user") {
        let mut existing = world.managers.remove(idx);
        info!(
            "[mcp-bootstrap] Reusing existing manager {} {} (team_id={:?})",
            existing.first_name, existing.last_name, existing.team_id
        );
        // Apply CLI overrides for name/nationality if provided
        if manager_first_name != "Agent" {
            existing.first_name = manager_first_name.to_string();
        }
        if manager_last_name != "Manager" {
            existing.last_name = manager_last_name.to_string();
        }
        if manager_nationality != "England" {
            existing.nationality = manager_nationality.to_string();
        }
        existing
    } else {
        // No existing user manager — create a fresh one (DOB set to make age ~45)
        let startup_options = normalize_startup_options(None)?;
        let reference_date = game_clock_for_world(&startup_options, &world.metadata)?
            .current_date
            .date_naive();
        let dob = reference_date - chrono::Duration::days(45 * 365);
        let dob_str = dob.format("%Y-%m-%d").to_string();

        let fresh = Manager::new(
            "mgr_user".to_string(),
            manager_first_name.to_string(),
            manager_last_name.to_string(),
            dob_str,
            manager_nationality.to_string(),
        );
        info!(
            "[mcp-bootstrap] Created fresh manager {} {}",
            fresh.first_name, fresh.last_name
        );
        fresh
    };

    // Step 3: Build game from world data
    let startup_options = normalize_startup_options(None)?;
    let clock = game_clock_for_world(&startup_options, &world.metadata)?;
    let (mut game, current_stats_state) =
        build_game_from_world_data(clock, manager, &startup_options, world);

    info!(
        "[mcp-bootstrap] Built game: {} teams, {} players, manager.team_id={:?}",
        game.teams.len(),
        game.players.len(),
        game.manager.team_id,
    );

    // Step 4: If the manager already has a team assigned (reused from world data),
    // we don't need the takeover logic. Just refresh context and proceed.
    // Otherwise, run the normal team selection bootstrap.
    let stats_state = if game.manager.team_id.is_some() {
        ofm_core::ai_hiring::seed_ai_managers(&mut game);
        ofm_core::season_context::refresh_game_context(&mut game);
        ofm_core::transfers::seed_opening_ai_loan_market(&mut game);
        current_stats_state
    } else {
        // Manager has no team — need an explicit team_id to assign one
        let tid = team_id.ok_or(
            "--mcp-auto-start requires a team_id when the world's manager has no team. Format: \"world.json,team_id\""
                .to_string(),
        )?;
        let start_phase = start_phase_for_game(&game);
        bootstrap_team_selection(&mut game, tid, start_phase, current_stats_state)?
    };

    info!(
        "[mcp-bootstrap] Manager assigned to team_id={:?}",
        game.manager.team_id
    );

    // Step 5: Create initial save
    let manager_name = format!("{} {}", game.manager.first_name, game.manager.last_name);
    let save_name = default_save_name(&manager_name);
    let mut sm = map_save_manager_lock_error(save_manager_state.0.lock())?;
    let save_id = create_new_save(&mut sm, &game, &stats_state, &save_name)?;

    // Step 6: Set state
    state_manager.set_game(game);
    state_manager.set_stats_state(stats_state);
    state_manager.set_save_id(save_id.clone());

    info!("[mcp-bootstrap] Game saved with ID: {}", save_id);

    Ok(save_id)
}

/// World-building fixtures shared by the tests of several clusters in this
/// module. Kept apart so each peel of `game.rs` can take its own tests with it
/// without duplicating a fixture or reaching into a sibling.
#[cfg(test)]
mod testkit;

#[cfg(test)]
mod tests {
    use super::testkit::*;
    use super::{
        bootstrap_team_selection, build_game_from_world_data, create_new_save,
        ensure_multi_competition_foundations, game_clock_for_world, resolve_simulation_scope,
        start_date_for_year, StartPhase, StartupOptions, DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
    };
    use chrono::{TimeZone, Utc};
    use db::save_manager::SaveManager;
    use domain::{
        league::{CompetitionFormat, CompetitionScope, CompetitionType, League},
        news::NewsCategory,
    };
    use ofm_core::{clock::GameClock, game::Game};
    use std::collections::{BTreeMap, BTreeSet};

    /// The regression test for #555, at the shape the game actually ships.
    ///
    /// Every earlier promotion test built its divisions by hand, and that is
    /// exactly how the bug survived: hand-built divisions carry no berths, so
    /// none of them ever entered the code path that a real world takes. This
    /// one runs `ensure_multi_competition_foundations` — the same competition
    /// plan `create_game` builds — through three consecutive rollovers and
    /// checks the invariant that matters: every club is registered with exactly
    /// one of its country's league tables, the divisions keep their sizes, and
    /// each top flight actually turns over.
    ///
    /// The nations are chosen to cover the shapes that broke: two-tier pyramids
    /// (ENG, ES, BR), a split-season country whose two halves share one roster
    /// (AR), a single-tier country (PT), Brazil's regional state cups, and a
    /// continental cup every first division berths into — the shared target
    /// that detached every ladder in the world.
    fn production_world(start_year: i32, user_team: &str) -> Game {
        let mut teams = Vec::new();
        for (nation, count) in [("ENG", 40), ("ES", 40), ("BR", 40), ("AR", 20), ("PT", 20)] {
            for index in 0..count {
                teams.push(nation_team(
                    &format!("{}-{index:02}", nation.to_lowercase()),
                    nation,
                    1000 - index as u32,
                ));
            }
        }
        let clock = GameClock::new(start_date_for_year(start_year).expect("valid start year"));
        let mut game = Game::new(clock, manager_for(user_team), teams, vec![], vec![], vec![]);
        ensure_multi_competition_foundations(&mut game);
        game.sync_legacy_league();
        game
    }

    /// (nation, club id) for every club in the world.
    fn team_list(game: &Game) -> Vec<(String, String)> {
        game.teams
            .iter()
            .map(|team| (team.football_nation.clone(), team.id.clone()))
            .collect()
    }

    fn roster(competition: &League) -> BTreeSet<String> {
        competition.participant_ids.iter().cloned().collect()
    }

    /// Every club in exactly one of its country's league tables, every division
    /// still its authored size, and a split-season country's two halves still
    /// running over the same clubs.
    fn assert_one_league_per_club(game: &Game, sizes: &BTreeMap<String, usize>, label: &str) {
        let mut clubs_by_nation: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for team in &team_list(game) {
            clubs_by_nation
                .entry(team.0.clone())
                .or_default()
                .insert(team.1.clone());
        }
        let mut by_country: BTreeMap<String, Vec<&League>> = BTreeMap::new();
        for competition in &game.competitions {
            if competition.rules.format != CompetitionFormat::LeagueTable
                || competition.kind != CompetitionType::League
                || competition.scope != CompetitionScope::Domestic
            {
                continue;
            }
            if let Some(country) = &competition.country_id {
                by_country
                    .entry(country.clone())
                    .or_default()
                    .push(competition);
            }
        }
        // Iterating only the countries that still have competitions means a
        // country whose leagues vanished is never checked — the test passed
        // with both Argentine leagues deleted after every rollover. Compare the
        // key sets first, so a disappearing country is itself a failure.
        let represented: BTreeSet<&String> = by_country.keys().collect();
        let expected: BTreeSet<&String> = clubs_by_nation.keys().collect();
        assert_eq!(
            represented, expected,
            "{label}: every nation must still have at least one league table"
        );

        for (country, leagues) in by_country {
            let mut union: BTreeSet<String> = BTreeSet::new();
            let split_season = ofm_core::nations::is_split_season_country(&country);
            for league in &leagues {
                let clubs = roster(league);
                assert_eq!(
                    clubs.len(),
                    league.participant_ids.len(),
                    "{label}: {} lists a club twice: {:?}",
                    league.id,
                    league.participant_ids
                );
                assert_eq!(
                    league.participant_ids.len(),
                    sizes[&league.id],
                    "{label}: {} changed size",
                    league.id
                );
                if !split_season {
                    assert!(
                        union.is_disjoint(&clubs),
                        "{label}: {} shares clubs with another {country} league",
                        league.id
                    );
                }
                union.extend(clubs);
            }
            if split_season {
                // Both halves are the same division played twice, so they must
                // hold identical rosters rather than disjoint ones.
                let first = roster(leagues[0]);
                for league in &leagues[1..] {
                    assert_eq!(
                        roster(league),
                        first,
                        "{label}: {country} halves drifted apart at {}",
                        league.id
                    );
                }
            }
            assert_eq!(
                union, clubs_by_nation[&country],
                "{label}: every {country} club must be in exactly one league table"
            );
        }
    }

    #[test]
    fn a_generated_world_promotes_and_relegates_for_three_seasons_running() {
        let mut game = production_world(2035, "eng-00");
        let (regions, competitions) =
            resolve_simulation_scope(&game, "eng-00", None, None).expect("a valid scope");
        game.active_region_ids = regions;
        game.active_competition_ids = competitions;
        let sizes: BTreeMap<String, usize> = game
            .competitions
            .iter()
            .map(|competition| (competition.id.clone(), competition.participant_ids.len()))
            .collect();
        let mut previous: BTreeMap<String, BTreeSet<String>> = game
            .competitions
            .iter()
            .filter(|competition| ["eng-d1", "es-d1"].contains(&competition.id.as_str()))
            .map(|competition| (competition.id.clone(), roster(competition)))
            .collect();

        for rollover in 1..=3 {
            let far_future = Utc.with_ymd_and_hms(2100, 1, 1, 0, 0, 0).unwrap();
            let players = game.players.clone();
            for competition in game.competitions.iter_mut() {
                ofm_core::catchup::simulate_past_fixtures(competition, &players, far_future);
            }
            let last_match_day = game
                .competitions
                .iter()
                .find(|competition| {
                    competition.rules.format == CompetitionFormat::LeagueTable
                        && competition.participant_ids.iter().any(|id| id == "eng-00")
                })
                .expect("the user's division")
                .fixtures
                .iter()
                .filter(|fixture| fixture.counts_for_league_standings())
                .filter_map(|fixture| {
                    chrono::NaiveDate::parse_from_str(&fixture.date, "%Y-%m-%d").ok()
                })
                .max()
                .expect("a played season");
            game.clock.current_date = Utc.from_utc_datetime(
                &last_match_day
                    .and_hms_opt(0, 0, 0)
                    .expect("midnight is a valid time"),
            ) + chrono::Duration::days(1);

            ofm_core::end_of_season::process_end_of_season(&mut game);

            let label = format!("rollover {rollover}");
            assert_one_league_per_club(&game, &sizes, &label);

            // Every domestic division that existed at kickoff must still exist.
            // Size and disjointness assertions say nothing about a league that
            // is simply gone.
            for id in sizes.keys() {
                assert!(
                    game.competitions.iter().any(|c| c.id == *id),
                    "{label}: competition {id} disappeared"
                );
            }

            // The user's own division has to stay in simulation scope, or the
            // day loop cannot see their fixtures and runs their match against
            // whichever competition sorts first.
            let user_division = game
                .competitions
                .iter()
                .find(|c| {
                    c.rules.format == CompetitionFormat::LeagueTable
                        && c.participant_ids.iter().any(|id| id == "eng-00")
                })
                .expect("the user is registered somewhere");
            assert!(
                game.active_competition_ids.contains(&user_division.id),
                "{label}: the user's division {} is out of scope",
                user_division.id
            );

            // The point of #555: a top flight that never changes hands is the
            // bug, not a stable league.
            for id in ["eng-d1", "es-d1"] {
                let now = roster(
                    game.competitions
                        .iter()
                        .find(|competition| competition.id == id)
                        .expect(id),
                );
                assert_ne!(
                    previous.get(id),
                    Some(&now),
                    "{label}: {id} promoted and relegated nobody"
                );
                previous.insert(id.to_string(), now);
            }
        }
    }

    /// Brazil is the one shipped nation whose divisions run on different
    /// calendars — Série A opens 28 January, Série B on 21 March — so at the
    /// first division's last matchday the second still has eight rounds to
    /// play. The rollover used to fire anyway, the ladder skipped the whole
    /// country for want of a finished lower tier, and Série B was never
    /// regenerated: promotion happened every *other* season, and Série B
    /// skipped a calendar year each time it did.
    ///
    /// A Brazilian career is the whole point of this test. An English one
    /// cannot see the bug, because both English divisions finish together.
    #[test]
    fn a_brazilian_career_promotes_and_relegates_every_season() {
        let mut game = production_world(2035, "br-00");
        let (regions, competitions) =
            resolve_simulation_scope(&game, "br-00", None, None).expect("a valid scope");
        game.active_region_ids = regions;
        game.active_competition_ids = competitions;

        for rollover in 1..=3 {
            let user_last = game
                .competitions
                .iter()
                .find(|competition| {
                    competition.rules.format == CompetitionFormat::LeagueTable
                        && competition.participant_ids.iter().any(|id| id == "br-00")
                })
                .expect("the user's division")
                .fixtures
                .iter()
                .filter(|fixture| fixture.counts_for_league_standings())
                .filter_map(|fixture| {
                    chrono::NaiveDate::parse_from_str(&fixture.date, "%Y-%m-%d").ok()
                })
                .max()
                .expect("a scheduled season");

            // Advance in steps the way a player does, until the game itself
            // says the season is over. If the gate let the rollover through
            // while Série B was mid-season, this would stop at the first step.
            let mut cutoff = Utc.from_utc_datetime(
                &user_last
                    .and_hms_opt(0, 0, 0)
                    .expect("midnight is a valid time"),
            ) + chrono::Duration::days(1);
            for _ in 0..40 {
                let players = game.players.clone();
                for competition in game.competitions.iter_mut() {
                    ofm_core::catchup::simulate_past_fixtures(competition, &players, cutoff);
                }
                game.clock.current_date = cutoff;
                if ofm_core::end_of_season::is_season_complete(&game) {
                    break;
                }
                cutoff += chrono::Duration::days(14);
            }
            assert!(
                ofm_core::end_of_season::is_season_complete(&game),
                "rollover {rollover}: the Brazilian season never completed"
            );

            let roster_before = by_competition_id(&game, "br-d1").participant_ids.clone();
            let second_tier_season_before = by_competition_id(&game, "br-d2").season;

            ofm_core::end_of_season::process_end_of_season(&mut game);

            assert_ne!(
                by_competition_id(&game, "br-d1").participant_ids,
                roster_before,
                "rollover {rollover}: Série A promoted and relegated nobody"
            );
            assert_eq!(
                by_competition_id(&game, "br-d2").season,
                second_tier_season_before + 1,
                "rollover {rollover}: Série B must advance exactly one season, not skip a year"
            );
        }
    }

    fn by_competition_id<'a>(game: &'a Game, id: &str) -> &'a League {
        game.competitions
            .iter()
            .find(|competition| competition.id == id)
            .unwrap_or_else(|| panic!("{id} should exist"))
    }

    #[test]
    #[ignore = "perf harness; run: cargo test -p openfootmanager perf_baseline -- --ignored --nocapture"]
    fn perf_baseline() {
        use std::time::Instant;

        let t = Instant::now();
        let world = ofm_core::generator::generate_world_data(
            &ofm_core::generator::DefinitionSources::embedded_only(),
        );
        let gen = t.elapsed();
        let teams = world.teams.len();
        let players = world.players.len();

        let manager = domain::manager::Manager::new(
            "mgr-user".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let startup_options = StartupOptions {
            start_year: 2026,
            start_phase: StartPhase::SeasonStart,
            history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
        };
        let clock = game_clock_for_world(&startup_options, &world.metadata).unwrap();

        let t = Instant::now();
        let (mut game, _stats) =
            build_game_from_world_data(clock, manager, &startup_options, world);
        let build = t.elapsed();

        let competitions = game.competitions.len();
        let active = game.active_competition_ids.len();

        const DAYS: u32 = 30;
        let t = Instant::now();
        for _ in 0..DAYS {
            ofm_core::turn::process_day(&mut game);
        }
        let days = t.elapsed();

        eprintln!(
            "PERF teams={teams} players={players} competitions={competitions} active_competition_ids={active}"
        );
        eprintln!("PERF world-gen         = {gen:?}");
        eprintln!("PERF build-game        = {build:?}  (foundations + history)");
        eprintln!(
            "PERF {DAYS}x process_day   = {days:?}  ({:?}/day)",
            days / DAYS
        );
    }

    #[test]
    fn historical_snapshot_startup_preserves_league_news_history_and_stats() {
        let manager = domain::manager::Manager::new(
            "mgr-user".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let startup_options = StartupOptions {
            start_year: 2032,
            start_phase: StartPhase::MidSeason,
            history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
        };
        let world = make_historical_snapshot_world();
        let clock = game_clock_for_world(&startup_options, &world.metadata).unwrap();

        let (game, stats_state) =
            build_game_from_world_data(clock, manager, &startup_options, world);

        assert_eq!(
            game.clock.start_date.to_rfc3339(),
            "2031-07-01T00:00:00+00:00"
        );
        assert_eq!(
            game.clock.current_date.to_rfc3339(),
            "2031-11-20T00:00:00+00:00"
        );
        assert_eq!(game.league.as_ref().map(|league| league.season), Some(2031));
        assert_eq!(game.news.len(), 1);
        assert_eq!(game.world_history.season_awards.len(), 1);
        assert_eq!(stats_state.team_matches.len(), 1);
        assert_eq!(stats_state.player_matches.len(), 1);
        assert!(game
            .managers
            .iter()
            .any(|manager| manager.id == "mgr-incumbent"));
    }

    #[test]
    fn imported_roster_baseline_bootstrap_backfills_staff_market_and_opening_youth() {
        let manager = domain::manager::Manager::new(
            "mgr-user".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let startup_options = StartupOptions {
            start_year: 2032,
            start_phase: StartPhase::SeasonStart,
            history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
        };
        let mut world = make_imported_baseline_world_without_staff();
        ofm_core::generator::normalize_imported_world_for_career_start(
            &mut world,
            startup_options.start_year as u32,
        );
        let clock = game_clock_for_world(&startup_options, &world.metadata).unwrap();

        let (game, stats_state) =
            build_game_from_world_data(clock, manager, &startup_options, world);

        assert!(stats_state.team_matches.is_empty());
        assert_eq!(
            game.staff
                .iter()
                .filter(|staff_member| staff_member.team_id.is_none())
                .count(),
            12
        );
        for team_id in ["team1", "team2"] {
            for role in [
                domain::staff::StaffRole::AssistantManager,
                domain::staff::StaffRole::Coach,
                domain::staff::StaffRole::Scout,
                domain::staff::StaffRole::Physio,
            ] {
                let count = game
                    .staff
                    .iter()
                    .filter(|staff_member| {
                        staff_member.team_id.as_deref() == Some(team_id)
                            && staff_member.role == role
                    })
                    .count();
                assert_eq!(count, 1);
            }
            let youth_count = game
                .players
                .iter()
                .filter(|player| {
                    player.team_id.as_deref() == Some(team_id)
                        && player.squad_role == domain::player::SquadRole::Youth
                })
                .count();
            assert_eq!(youth_count, 3);
        }
        assert_eq!(
            game.available_staff_market_last_activity_date.as_deref(),
            Some("2032-07-01")
        );
    }

    #[test]
    fn imported_roster_baseline_bootstrap_allows_ai_manager_seeding_without_imported_staff() {
        let manager = domain::manager::Manager::new(
            "mgr-user".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let startup_options = StartupOptions {
            start_year: 2032,
            start_phase: StartPhase::SeasonStart,
            history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
        };
        let mut world = make_imported_baseline_world_without_staff();
        ofm_core::generator::normalize_imported_world_for_career_start(
            &mut world,
            startup_options.start_year as u32,
        );
        let clock = game_clock_for_world(&startup_options, &world.metadata).unwrap();
        let (mut game, stats_state) =
            build_game_from_world_data(clock, manager, &startup_options, world);

        bootstrap_team_selection(&mut game, "team1", StartPhase::SeasonStart, stats_state).unwrap();

        assert_eq!(
            game.teams
                .iter()
                .find(|team| team.id == "team1")
                .and_then(|team| team.manager_id.as_deref()),
            Some("mgr-user")
        );
        assert!(game
            .teams
            .iter()
            .filter(|team| team.id != "team1")
            .all(|team| team.manager_id.is_some()));
    }

    #[test]
    fn bootstrap_team_selection_seeds_ai_loan_market() {
        let mut game = make_bootstrap_test_game();
        game.teams
            .iter_mut()
            .find(|team| team.id == "team2")
            .unwrap()
            .starting_xi_ids = (0..11)
            .map(|index| format!("team2-player-{index}"))
            .collect();

        for (id, date_of_birth) in [
            ("team2-loan-1", "2007-01-01"),
            ("team2-loan-2", "2006-01-01"),
            ("team2-loan-3", "2005-01-01"),
        ] {
            let mut player = domain::player::Player::new(
                id.to_string(),
                id.to_string(),
                id.to_string(),
                date_of_birth.to_string(),
                "England".to_string(),
                domain::player::Position::Midfielder,
                default_player_attributes(),
            );
            player.team_id = Some("team2".to_string());
            player.contract_end = Some("2035-06-30".to_string());
            game.players.push(player);
        }

        bootstrap_team_selection(
            &mut game,
            "team1",
            StartPhase::SeasonStart,
            domain::stats::StatsState::default(),
        )
        .unwrap();

        assert_eq!(
            game.players
                .iter()
                .filter(|player| {
                    player.team_id.as_deref() == Some("team2") && player.loan_listed
                })
                .count(),
            2
        );
        assert!(game
            .players
            .iter()
            .filter(|player| player.team_id.as_deref() == Some("team1"))
            .all(|player| !player.loan_listed));
    }

    #[test]
    fn imported_historical_snapshot_preserves_state_while_backfilling_staff() {
        let manager = domain::manager::Manager::new(
            "mgr-user".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let startup_options = StartupOptions {
            start_year: 2032,
            start_phase: StartPhase::MidSeason,
            history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
        };
        let mut world = make_historical_snapshot_world();
        world.staff.clear();
        let original_news_len = world.news.len();
        let original_season = world.league.as_ref().map(|league| league.season);
        let original_awards = world.world_history.season_awards.len();
        ofm_core::generator::normalize_imported_world_for_career_start(
            &mut world,
            startup_options.start_year as u32,
        );
        let clock = game_clock_for_world(&startup_options, &world.metadata).unwrap();

        let (game, stats_state) =
            build_game_from_world_data(clock, manager, &startup_options, world);

        assert_eq!(
            game.league.as_ref().map(|league| league.season),
            original_season
        );
        assert_eq!(game.news.len(), original_news_len);
        assert_eq!(game.world_history.season_awards.len(), original_awards);
        assert_eq!(stats_state.team_matches.len(), 1);
        assert_eq!(
            game.staff
                .iter()
                .filter(|staff_member| staff_member.team_id.is_none())
                .count(),
            12
        );
        for team_id in ["team1", "team2"] {
            let has_assistant = game.staff.iter().any(|staff_member| {
                staff_member.team_id.as_deref() == Some(team_id)
                    && staff_member.role == domain::staff::StaffRole::AssistantManager
            });
            assert!(has_assistant);
        }
    }

    #[test]
    fn embedded_competition_definitions_replace_the_auto_built_competitions() {
        use ofm_core::generator::{
            CompetitionDefinition, CompetitionDefinitionFile, FormatDef, ParticipantSpec,
        };

        let manager = domain::manager::Manager::new(
            "mgr-user".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let startup_options = StartupOptions {
            start_year: 2032,
            start_phase: StartPhase::MidSeason,
            history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
        };
        let mut world = make_historical_snapshot_world();
        let team_ids: Vec<String> = world.teams.iter().map(|t| t.id.clone()).collect();
        assert!(team_ids.len() >= 2);
        world.competition_definitions = Some(CompetitionDefinitionFile {
            format_version: 1,
            competitions: vec![CompetitionDefinition {
                id: "custom-league".to_string(),
                name: "Custom League".to_string(),
                r#type: domain::league::CompetitionType::League,
                scope: domain::league::CompetitionScope::Domestic,
                region_id: None,
                country_id: None,
                required_region_ids: vec![],
                priority: 0,
                format: FormatDef {
                    kind: domain::league::CompetitionFormat::LeagueTable,
                    legs: None,
                    group_size: None,
                    qualifiers_per_group: None,
                    best_third_qualifiers: None,
                },
                participants: ParticipantSpec {
                    explicit: Some(team_ids.clone()),
                    selector: None,
                },
                berths: Vec::new(),
                season_start_month: None,
                season_start_day: None,
                name_key: None,
                logo: None,
            }],
        });
        let clock = game_clock_for_world(&startup_options, &world.metadata).unwrap();

        let (game, _stats) = build_game_from_world_data(clock, manager, &startup_options, world);

        let custom = game
            .competitions
            .iter()
            .find(|c| c.id == "custom-league")
            .expect("authored competition replaces the auto-built ones");
        assert_eq!(custom.participant_ids, team_ids);
        assert!(
            game.competitions.iter().all(|c| c.id == "custom-league"
                || c.kind == domain::league::CompetitionType::InternationalNation),
            "no auto-generated club competitions when definitions are supplied"
        );
    }

    #[test]
    fn bootstrap_team_selection_preserves_existing_snapshot_state() {
        let manager = domain::manager::Manager::new(
            "mgr-user".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let startup_options = StartupOptions {
            start_year: 2032,
            start_phase: StartPhase::MidSeason,
            history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
        };
        let world = make_historical_snapshot_world();
        let clock = game_clock_for_world(&startup_options, &world.metadata).unwrap();
        let (mut game, stats_state) =
            build_game_from_world_data(clock, manager, &startup_options, world);

        let updated_stats =
            bootstrap_team_selection(&mut game, "team1", StartPhase::MidSeason, stats_state)
                .unwrap();

        assert_eq!(game.league.as_ref().map(|league| league.season), Some(2031));
        assert_eq!(updated_stats.team_matches.len(), 1);
        assert_eq!(updated_stats.player_matches.len(), 1);
        assert_eq!(
            game.teams
                .iter()
                .find(|team| team.id == "team1")
                .and_then(|team| team.manager_id.as_deref()),
            Some("mgr-user")
        );
        assert!(game
            .news
            .iter()
            .any(|article| article.category == NewsCategory::ManagerialChange));
    }

    #[test]
    fn create_new_save_persists_stats_state_on_first_save() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let saves_dir = std::env::temp_dir().join(format!("ofm-game-command-tests-{}", unique));
        std::fs::create_dir_all(&saves_dir).unwrap();
        let mut save_manager = SaveManager::init(&saves_dir).unwrap();
        let game = make_bootstrap_test_game();
        let stats_state = sample_stats_state();

        let save_id =
            create_new_save(&mut save_manager, &game, &stats_state, "Stats Career").unwrap();
        let loaded_stats = save_manager.load_stats_state(&save_id).unwrap();

        assert_eq!(loaded_stats.team_matches.len(), 1);
        assert_eq!(loaded_stats.player_matches.len(), 1);
        assert_eq!(loaded_stats.team_matches[0].team_id, "team1");

        std::fs::remove_dir_all(&saves_dir).unwrap();
    }

    #[test]
    fn bootstrap_team_selection_midseason_populates_half_season_state() {
        let mut game = make_bootstrap_test_game();

        let stats_state = bootstrap_team_selection(
            &mut game,
            "team1",
            StartPhase::MidSeason,
            domain::stats::StatsState::default(),
        )
        .unwrap();

        let league = game.league.as_ref().unwrap();
        let completed = league
            .fixtures
            .iter()
            .filter(|fixture| {
                fixture.counts_for_league_standings()
                    && fixture.status == domain::league::FixtureStatus::Completed
                    && (fixture.home_team_id == "team1" || fixture.away_team_id == "team1")
            })
            .count();
        let scheduled = league
            .fixtures
            .iter()
            .filter(|fixture| {
                fixture.counts_for_league_standings()
                    && (fixture.home_team_id == "team1" || fixture.away_team_id == "team1")
            })
            .count();
        let team_standing = league
            .standings
            .iter()
            .find(|entry| entry.team_id == "team1")
            .unwrap();

        assert_eq!(completed, scheduled / 2);
        assert!(!stats_state.team_matches.is_empty());
        assert!(!stats_state.player_matches.is_empty());
        assert_eq!(team_standing.played as usize, completed);
        assert!(game
            .news
            .iter()
            .any(|article| article.category == domain::news::NewsCategory::ManagerialChange));
        assert!(game.news.iter().any(|article| {
            matches!(
                article.category,
                domain::news::NewsCategory::MatchReport
                    | domain::news::NewsCategory::LeagueRoundup
                    | domain::news::NewsCategory::StandingsUpdate
            )
        }));
    }

    /// Regression test for issue #225: verifies that bootstrap_team_selection followed by
    /// upgrade_game_player_identities converts generic bucket positions
    /// (Defender/Midfielder/Forward) to granular positions (LeftBack/CentralMidfielder/etc.).
    /// select_team calls both in sequence; it cannot be called directly here because it
    /// requires Tauri App state, so this test exercises the same in-memory operations.
    #[test]
    fn bootstrap_and_upgrade_sets_granular_positions() {
        let startup_options = StartupOptions {
            start_year: 2032,
            start_phase: StartPhase::SeasonStart,
            history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
        };
        let mut world = make_imported_baseline_world_without_staff();
        ofm_core::generator::normalize_imported_world_for_career_start(
            &mut world,
            startup_options.start_year as u32,
        );
        let clock = game_clock_for_world(&startup_options, &world.metadata).unwrap();
        let manager = domain::manager::Manager::new(
            "mgr-user".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let (mut game, stats_state) =
            build_game_from_world_data(clock, manager, &startup_options, world);

        // All generated players start with generic (legacy-bucket) positions
        let outfield_before: Vec<_> = game
            .players
            .iter()
            .filter(|p| p.position != domain::player::Position::Goalkeeper)
            .collect();
        assert!(
            outfield_before
                .iter()
                .all(|p| p.natural_position.is_legacy_bucket()),
            "generated players should all start with generic (legacy-bucket) natural_position"
        );

        bootstrap_team_selection(&mut game, "team1", StartPhase::SeasonStart, stats_state).unwrap();
        ofm_core::player_identity::upgrade_game_player_identities(&mut game);

        // After upgrade, outfield players on team1 should have granular natural_position
        let outfield_after: Vec<_> = game
            .players
            .iter()
            .filter(|p| {
                p.team_id.as_deref() == Some("team1")
                    && p.position != domain::player::Position::Goalkeeper
            })
            .collect();
        assert!(
            !outfield_after.is_empty(),
            "team1 should have outfield players"
        );
        assert!(
            outfield_after
                .iter()
                .all(|p| !p.natural_position.is_legacy_bucket()),
            "outfield players on the selected team should have granular natural_position after upgrade"
        );
    }
}
