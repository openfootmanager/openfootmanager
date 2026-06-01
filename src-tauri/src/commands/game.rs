use log::info;
use tauri::State;

use chrono::{Datelike, Duration, TimeZone, Utc};

use db::{save_index::SaveEntry, save_manager::SaveManager};
use domain::manager::Manager;
use domain::stats::StatsState;
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::state::StateManager;

use crate::SaveManagerState;

fn load_world_data_from_path(world_source: &str) -> Result<ofm_core::generator::WorldData, String> {
    let path = world_source.strip_prefix("file:").unwrap_or(world_source);
    let json =
        std::fs::read_to_string(path).map_err(|_| "be.error.worldReadFileFailed".to_string())?;
    ofm_core::generator::load_world_from_json(&json)
}

fn map_save_manager_lock_error<T>(result: std::sync::LockResult<T>) -> Result<T, String> {
    result.map_err(|_| "be.error.saveManagerUnavailable".to_string())
}

fn require_active_stats_state(state: &StateManager) -> Result<StatsState, String> {
    state
        .get_stats_state(|stats| stats.clone())
        .ok_or("be.error.noActiveStatsSession".to_string())
}

fn default_league_name() -> String {
    ["Premier", "Division"].join(" ")
}

const DEFAULT_GENERATED_HISTORY_DEPTH_YEARS: u32 = 12;
const MAX_GENERATED_HISTORY_DEPTH_YEARS: u32 = 24;

fn long_date_format() -> String {
    ['%', 'B', ' ', '%', 'd', ',', ' ', '%', 'Y']
        .into_iter()
        .collect()
}

fn default_save_name(manager_name: &str) -> String {
    let mut save_name = manager_name.to_string();
    save_name.push('\'');
    save_name.push('s');
    save_name.push(' ');
    save_name.push_str("Career");
    save_name
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawStartupOptions {
    #[serde(default)]
    start_year: Option<i32>,
    #[serde(default)]
    start_phase: Option<String>,
    #[serde(default)]
    history_depth_years: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StartPhase {
    SeasonStart,
    MidSeason,
}

impl StartPhase {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "seasonStart" => Some(Self::SeasonStart),
            "midSeason" => Some(Self::MidSeason),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::SeasonStart => "seasonStart",
            Self::MidSeason => "midSeason",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StartupOptions {
    start_year: i32,
    start_phase: StartPhase,
    history_depth_years: u32,
}

fn default_start_year() -> i32 {
    chrono::Utc::now().year().max(2020)
}

fn default_history_depth_years() -> u32 {
    DEFAULT_GENERATED_HISTORY_DEPTH_YEARS
}

fn start_date_for_year(start_year: i32) -> Result<chrono::DateTime<Utc>, String> {
    Utc.with_ymd_and_hms(start_year, 7, 1, 0, 0, 0)
        .single()
        .ok_or_else(|| "be.error.createManager.invalidStartYear".to_string())
}

fn current_date_for_phase(
    start_year: i32,
    start_phase: StartPhase,
) -> Result<chrono::DateTime<Utc>, String> {
    let start_date = start_date_for_year(start_year)?;
    Ok(match start_phase {
        StartPhase::SeasonStart => start_date,
        StartPhase::MidSeason => start_date + Duration::days(120),
    })
}

fn age_on_date(birth_date: chrono::NaiveDate, reference_date: chrono::NaiveDate) -> i64 {
    let mut age = i64::from(reference_date.year() - birth_date.year());
    let has_had_birthday =
        (reference_date.month(), reference_date.day()) >= (birth_date.month(), birth_date.day());
    if !has_had_birthday {
        age -= 1;
    }
    age
}

fn start_phase_for_game(game: &Game) -> StartPhase {
    if game.clock.current_date > game.clock.start_date {
        StartPhase::MidSeason
    } else {
        StartPhase::SeasonStart
    }
}

fn preseason_season_start(clock: &GameClock) -> chrono::DateTime<Utc> {
    clock.start_date + Duration::days(30)
}

fn preseason_league_year(clock: &GameClock) -> u32 {
    u32::try_from(clock.start_date.year()).unwrap_or(2020)
}

fn normalize_startup_options(raw: Option<RawStartupOptions>) -> Result<StartupOptions, String> {
    let raw = raw.unwrap_or_default();
    let start_year = raw.start_year.unwrap_or_else(default_start_year);
    if start_year < 2020 {
        return Err("be.error.createManager.startYearMin".to_string());
    }

    let start_phase = match raw.start_phase.as_deref() {
        None | Some("") => StartPhase::SeasonStart,
        Some(value) => StartPhase::parse(value)
            .ok_or_else(|| "be.error.createManager.invalidStartPhase".to_string())?,
    };
    let history_depth_years = raw
        .history_depth_years
        .unwrap_or_else(default_history_depth_years);
    if history_depth_years > MAX_GENERATED_HISTORY_DEPTH_YEARS {
        return Err("be.error.createManager.historyDepthMax".to_string());
    }

    Ok(StartupOptions {
        start_year,
        start_phase,
        history_depth_years,
    })
}

fn apply_generated_past_history(game: &mut Game, startup_options: &StartupOptions) {
    ofm_core::history_generation::generate_past_world_history(
        game,
        startup_options.start_year,
        startup_options.history_depth_years,
    );
}

fn load_world_data(world_source: Option<&str>) -> Result<ofm_core::generator::WorldData, String> {
    match world_source {
        None | Some("random") => Ok(ofm_core::generator::generate_world_data(None)),
        Some(source) => load_world_data_from_path(source),
    }
}

fn world_start_year(
    startup_options: &StartupOptions,
    metadata: &ofm_core::generator::WorldDataMetadata,
) -> i32 {
    match metadata.kind {
        ofm_core::generator::WorldDataKind::HistoricalSnapshot => {
            metadata.base_year.unwrap_or(startup_options.start_year)
        }
        ofm_core::generator::WorldDataKind::RosterBaseline => startup_options.start_year,
    }
}

fn game_clock_for_world(
    startup_options: &StartupOptions,
    metadata: &ofm_core::generator::WorldDataMetadata,
) -> Result<GameClock, String> {
    let start_year = world_start_year(startup_options, metadata);
    let mut clock = GameClock::new(start_date_for_year(start_year)?);
    clock.current_date = match metadata.kind {
        ofm_core::generator::WorldDataKind::HistoricalSnapshot => metadata
            .snapshot_date
            .as_deref()
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
            .map(|value| value.with_timezone(&Utc))
            .unwrap_or(current_date_for_phase(
                start_year,
                startup_options.start_phase,
            )?),
        ofm_core::generator::WorldDataKind::RosterBaseline => {
            current_date_for_phase(startup_options.start_year, startup_options.start_phase)?
        }
    };
    Ok(clock)
}

fn build_game_from_world_data(
    clock: GameClock,
    manager: Manager,
    startup_options: &StartupOptions,
    world: ofm_core::generator::WorldData,
) -> (Game, StatsState) {
    let ofm_core::generator::WorldData {
        teams,
        players,
        staff,
        managers,
        league,
        news,
        stats,
        world_history,
        metadata,
        ..
    } = world;

    let mut game = Game::new(clock, manager, teams, players, staff, vec![]);

    match metadata.kind {
        ofm_core::generator::WorldDataKind::HistoricalSnapshot => {
            game.managers.extend(
                managers
                    .into_iter()
                    .filter(|existing_manager| existing_manager.id != game.manager.id),
            );
            game.league = league;
            game.news = news;
            game.world_history = world_history;
            ofm_core::season_context::refresh_game_context(&mut game);
            (game, stats)
        }
        ofm_core::generator::WorldDataKind::RosterBaseline => {
            apply_generated_past_history(&mut game, startup_options);
            (game, StatsState::default())
        }
    }
}

fn has_existing_world_context(game: &Game, stats_state: &StatsState) -> bool {
    game.league.is_some()
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

fn create_new_save(
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
    let league_name = default_league_name();
    let mut league = ofm_core::schedule::generate_league(
        &league_name,
        preseason_league_year(&game.clock),
        &team_ids,
        season_start,
    );
    let friendlies = ofm_core::schedule::generate_preseason_friendlies(&team_ids, season_start, 4);
    ofm_core::schedule::append_fixtures(&mut league, friendlies);
    game.league = Some(league);
    ofm_core::season_context::refresh_game_context(game);

    let date_str = game.clock.current_date.to_rfc3339();
    let welcome_msg = ofm_core::messages::welcome_message(&team_name, team_id, &date_str);
    game.messages.push(welcome_msg);

    let season_msg = ofm_core::messages::season_schedule_message(
        &league_name,
        &season_start.format(&long_date_format()).to_string(),
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
    let league_name = default_league_name();
    let team_ids: Vec<String> = game.teams.iter().map(|t| t.id.clone()).collect();
    game.league = Some(ofm_core::schedule::generate_league(
        &league_name,
        preseason_league_year(&game.clock),
        &team_ids,
        season_start,
    ));
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

fn bootstrap_team_selection(
    game: &mut Game,
    team_id: &str,
    start_phase: StartPhase,
    stats_state: StatsState,
) -> Result<StatsState, String> {
    if has_existing_world_context(game, &stats_state) {
        return bootstrap_existing_world_takeover(game, team_id, stats_state);
    }

    match start_phase {
        StartPhase::SeasonStart => bootstrap_season_start(game, team_id),
        StartPhase::MidSeason => bootstrap_midseason_takeover(game, team_id),
    }
}

/// Step 1: Create manager + generate world. No team assigned yet.
/// Returns the Game object so the frontend can show team selection.
/// world_source: "random" (default) or a file path to a JSON world database.
#[tauri::command]
pub async fn start_new_game(
    state: State<'_, StateManager>,
    first_name: String,
    last_name: String,
    dob: String,
    nationality: String,
    startup_options: Option<RawStartupOptions>,
    world_source: Option<String>,
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
    let world = load_world_data(world_source.as_deref())?;
    let clock = game_clock_for_world(&startup_options, &world.metadata)?;
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

    let (new_game, stats_state) =
        build_game_from_world_data(clock, manager, &startup_options, world);

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
    state: State<'_, StateManager>,
    sm_state: State<'_, SaveManagerState>,
    team_id: String,
) -> Result<Game, String> {
    info!("[cmd] select_team: team_id={}", team_id);
    let mut game = state
        .get_game(|g: &Game| g.clone())
        .ok_or("be.error.noActiveGameSession".to_string())?;
    let current_stats_state = state
        .get_stats_state(|stats| stats.clone())
        .unwrap_or_default();

    let start_phase = start_phase_for_game(&game);
    let stats_state =
        bootstrap_team_selection(&mut game, &team_id, start_phase, current_stats_state)?;

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
pub async fn get_saves(sm_state: State<'_, SaveManagerState>) -> Result<Vec<SaveEntry>, String> {
    log::debug!("[cmd] get_saves");
    let mut sm = map_save_manager_lock_error(sm_state.0.lock())?;
    sm.load_saves()
}

#[tauri::command]
pub async fn delete_save(
    sm_state: State<'_, SaveManagerState>,
    save_id: String,
) -> Result<bool, String> {
    info!("[cmd] delete_save: save_id={}", save_id);
    let mut sm = map_save_manager_lock_error(sm_state.0.lock())?;
    sm.delete_save(&save_id)
}

#[tauri::command]
pub async fn load_game(
    state: State<'_, StateManager>,
    sm_state: State<'_, SaveManagerState>,
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
pub async fn get_active_game(state: State<'_, StateManager>) -> Result<Game, String> {
    log::debug!("[cmd] get_active_game");
    state
        .get_game(|g: &Game| g.clone())
        .ok_or("be.error.noActiveGameSession".to_string())
}

#[tauri::command]
pub async fn save_game(
    state: State<'_, StateManager>,
    sm_state: State<'_, SaveManagerState>,
) -> Result<(), String> {
    info!("[cmd] save_game");
    let game = state
        .get_game(|g: &Game| g.clone())
        .ok_or("be.error.noActiveGameSession".to_string())?;

    let save_id = state
        .get_save_id()
        .ok_or("be.error.noActiveSaveSession".to_string())?;

    let mut sm = map_save_manager_lock_error(sm_state.0.lock())?;
    let stats_state = require_active_stats_state(&state)?;
    sm.save_game_with_stats(&game, &stats_state, &save_id)
}

/// Save the current game and clear the active session so the player returns to the main menu.
#[tauri::command]
pub async fn exit_to_menu(
    state: State<'_, StateManager>,
    sm_state: State<'_, SaveManagerState>,
) -> Result<(), String> {
    info!("[cmd] exit_to_menu");
    let game = state
        .get_game(|g: &Game| g.clone())
        .ok_or("be.error.noActiveGameSession")?;

    // Auto-save
    if let Some(save_id) = state.get_save_id() {
        let mut sm = map_save_manager_lock_error(sm_state.0.lock())?;
        let stats_state = require_active_stats_state(&state)?;
        sm.save_game_with_stats(&game, &stats_state, &save_id)?;
    }

    // Clear the in-memory game state
    state.clear_game();
    state.clear_save_id();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        age_on_date, apply_generated_past_history, bootstrap_team_selection,
        build_game_from_world_data, create_new_save, current_date_for_phase, game_clock_for_world,
        load_world_data_from_path, map_save_manager_lock_error, normalize_startup_options,
        preseason_league_year, preseason_season_start, require_active_stats_state,
        start_date_for_year, RawStartupOptions, StartPhase, StartupOptions,
        DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
        MAX_GENERATED_HISTORY_DEPTH_YEARS,
    };
    use db::save_manager::SaveManager;
    use domain::{
        league::{FixtureCompetition, League},
        news::{NewsArticle, NewsCategory},
        stats::{PlayerMatchStatsRecord, TeamMatchStatsRecord},
        world_history::{HistoricalSeasonAwardsRecord, WorldHistoryArchive},
    };
    use ofm_core::{
        clock::GameClock,
        game::Game,
        generator::{WorldData, WorldDataKind, WorldDataMetadata},
        season_context::refresh_game_context,
        state::StateManager,
    };
    use std::sync::Mutex;

    fn default_player_attributes() -> domain::player::PlayerAttributes {
        domain::player::PlayerAttributes {
            pace: 60,
            stamina: 60,
            strength: 60,
            agility: 60,
            passing: 60,
            shooting: 60,
            tackling: 60,
            dribbling: 60,
            defending: 60,
            positioning: 60,
            vision: 60,
            decisions: 60,
            composure: 60,
            aggression: 50,
            teamwork: 60,
            leadership: 50,
            handling: 20,
            reflexes: 20,
            aerial: 60,
        }
    }

    fn make_bootstrap_test_game() -> Game {
        let clock = GameClock::new(start_date_for_year(2032).unwrap());
        let manager = domain::manager::Manager::new(
            "mgr-user".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let teams = vec![
            domain::team::Team::new(
                "team1".to_string(),
                "Alpha FC".to_string(),
                "AFC".to_string(),
                "England".to_string(),
                "London".to_string(),
                "Alpha Park".to_string(),
                20_000,
            ),
            domain::team::Team::new(
                "team2".to_string(),
                "Beta FC".to_string(),
                "BFC".to_string(),
                "England".to_string(),
                "Manchester".to_string(),
                "Beta Park".to_string(),
                22_000,
            ),
        ];
        let staff = vec![
            {
                let mut staff = domain::staff::Staff::new(
                    "staff1".to_string(),
                    "Pat".to_string(),
                    "Coach".to_string(),
                    "1978-01-01".to_string(),
                    domain::staff::StaffRole::AssistantManager,
                    domain::staff::StaffAttributes {
                        coaching: 70,
                        judging_ability: 65,
                        judging_potential: 64,
                        physiotherapy: 40,
                    },
                );
                staff.nationality = "England".to_string();
                staff.team_id = Some("team1".to_string());
                staff
            },
            {
                let mut staff = domain::staff::Staff::new(
                    "staff2".to_string(),
                    "Lee".to_string(),
                    "Coach".to_string(),
                    "1979-01-01".to_string(),
                    domain::staff::StaffRole::AssistantManager,
                    domain::staff::StaffAttributes {
                        coaching: 72,
                        judging_ability: 66,
                        judging_potential: 65,
                        physiotherapy: 39,
                    },
                );
                staff.nationality = "England".to_string();
                staff.team_id = Some("team2".to_string());
                staff
            },
        ];

        let mut players = Vec::new();
        for team_id in ["team1", "team2"] {
            for index in 0..11 {
                let position = if index == 0 {
                    domain::player::Position::Goalkeeper
                } else if index < 5 {
                    domain::player::Position::Defender
                } else if index < 8 {
                    domain::player::Position::Midfielder
                } else {
                    domain::player::Position::Forward
                };
                let mut player = domain::player::Player::new(
                    format!("{}-player-{}", team_id, index),
                    format!("{} P{}", team_id, index),
                    format!("{} Player {}", team_id, index),
                    format!("199{}-01-01", index),
                    "England".to_string(),
                    position,
                    default_player_attributes(),
                );
                player.team_id = Some(team_id.to_string());
                player.ovr = 62 + index as u8;
                player.potential = 68 + index as u8;
                players.push(player);
            }
        }

        Game::new(clock, manager, teams, players, staff, vec![])
    }

    #[test]
    fn load_world_data_from_path_returns_read_file_key_when_missing() {
        let result =
            load_world_data_from_path("file:Z:/definitely-missing/openfootmanager-world.json");

        assert_eq!(result.unwrap_err(), "be.error.worldReadFileFailed");
    }

    fn sample_stats_state() -> domain::stats::StatsState {
        domain::stats::StatsState {
            player_matches: vec![PlayerMatchStatsRecord {
                fixture_id: "fixture-1".to_string(),
                season: 2031,
                matchday: 12,
                date: "2031-11-20".to_string(),
                competition: FixtureCompetition::League,
                player_id: "team1-player-0".to_string(),
                team_id: "team1".to_string(),
                opponent_team_id: "team2".to_string(),
                home_team_id: "team1".to_string(),
                away_team_id: "team2".to_string(),
                home_goals: 2,
                away_goals: 1,
                minutes_played: 90,
                goals: 1,
                assists: 0,
                shots: 4,
                shots_on_target: 2,
                passes_completed: 30,
                passes_attempted: 35,
                tackles_won: 1,
                interceptions: 1,
                fouls_committed: 0,
                yellow_cards: 0,
                red_cards: 0,
                rating: 7.5,
            }],
            team_matches: vec![TeamMatchStatsRecord {
                fixture_id: "fixture-1".to_string(),
                season: 2031,
                matchday: 12,
                date: "2031-11-20".to_string(),
                competition: FixtureCompetition::League,
                team_id: "team1".to_string(),
                opponent_team_id: "team2".to_string(),
                home_team_id: "team1".to_string(),
                away_team_id: "team2".to_string(),
                goals_for: 2,
                goals_against: 1,
                possession_pct: 53,
                shots: 11,
                shots_on_target: 6,
                passes_completed: 310,
                passes_attempted: 360,
                tackles_won: 15,
                interceptions: 9,
                fouls_committed: 7,
                yellow_cards: 1,
                red_cards: 0,
            }],
        }
    }

    fn make_historical_snapshot_world() -> WorldData {
        let base_game = make_bootstrap_test_game();
        let mut league = League::new(
            "league-1".to_string(),
            "Premier Division".to_string(),
            2031,
            &["team1".to_string(), "team2".to_string()],
        );
        league.standings = vec![
            domain::league::StandingEntry {
                team_id: "team1".to_string(),
                played: 12,
                won: 7,
                drawn: 3,
                lost: 2,
                goals_for: 18,
                goals_against: 10,
                points: 24,
            },
            domain::league::StandingEntry {
                team_id: "team2".to_string(),
                played: 12,
                won: 5,
                drawn: 2,
                lost: 5,
                goals_for: 14,
                goals_against: 15,
                points: 17,
            },
        ];

        let mut incumbent = domain::manager::Manager::new(
            "mgr-incumbent".to_string(),
            "Jordan".to_string(),
            "Incumbent".to_string(),
            "1974-01-01".to_string(),
            "England".to_string(),
        );
        incumbent.hire("team1".to_string());

        let mut teams = base_game.teams.clone();
        teams[0].manager_id = Some(incumbent.id.clone());

        let mut archive = WorldHistoryArchive::default();
        archive.record_season_awards(HistoricalSeasonAwardsRecord {
            season: 2030,
            golden_boot: None,
            assist_king: None,
            player_of_year: None,
            clean_sheet_king: None,
            most_appearances: None,
            young_player: None,
            manager_of_season: None,
        });

        WorldData {
            name: "Historical Snapshot".to_string(),
            description: "Season already underway".to_string(),
            teams,
            players: base_game.players,
            staff: base_game.staff,
            managers: vec![incumbent],
            league: Some(league),
            news: vec![NewsArticle::new(
                "news-1".to_string(),
                "Season underway".to_string(),
                "The campaign has begun.".to_string(),
                "World Feed".to_string(),
                "2031-11-20".to_string(),
                NewsCategory::StandingsUpdate,
            )],
            stats: sample_stats_state(),
            world_history: archive,
            metadata: WorldDataMetadata {
                kind: WorldDataKind::HistoricalSnapshot,
                base_year: Some(2031),
                snapshot_date: Some("2031-11-20T00:00:00Z".to_string()),
            },
        }
    }

    #[test]
    fn map_save_manager_lock_error_returns_backend_key_for_poisoned_mutex() {
        let mutex = Mutex::new(());
        let _ = std::panic::catch_unwind(|| {
            let _guard = mutex.lock().unwrap();
            panic!("poison save manager mutex for test");
        });

        let result = map_save_manager_lock_error(mutex.lock());

        assert_eq!(result.unwrap_err(), "be.error.saveManagerUnavailable");
    }

    #[test]
    fn normalize_startup_options_defaults_to_current_year_and_season_start() {
        let options = normalize_startup_options(None).unwrap();

        assert!(options.start_year >= 2020);
        assert_eq!(options.start_phase, StartPhase::SeasonStart);
        assert_eq!(
            options.history_depth_years,
            DEFAULT_GENERATED_HISTORY_DEPTH_YEARS
        );
    }

    #[test]
    fn normalize_startup_options_rejects_years_before_2020() {
        let result = normalize_startup_options(Some(RawStartupOptions {
            start_year: Some(2019),
            start_phase: Some("seasonStart".to_string()),
            history_depth_years: None,
        }));

        assert_eq!(result.unwrap_err(), "be.error.createManager.startYearMin");
    }

    #[test]
    fn normalize_startup_options_rejects_unknown_start_phase() {
        let result = normalize_startup_options(Some(RawStartupOptions {
            start_year: Some(2026),
            start_phase: Some("playoffs".to_string()),
            history_depth_years: None,
        }));

        assert_eq!(
            result.unwrap_err(),
            "be.error.createManager.invalidStartPhase"
        );
    }

    #[test]
    fn normalize_startup_options_rejects_history_depths_above_maximum() {
        let result = normalize_startup_options(Some(RawStartupOptions {
            start_year: Some(2026),
            start_phase: Some("seasonStart".to_string()),
            history_depth_years: Some(MAX_GENERATED_HISTORY_DEPTH_YEARS + 1),
        }));

        assert_eq!(
            result.unwrap_err(),
            "be.error.createManager.historyDepthMax"
        );
    }

    #[test]
    fn normalize_startup_options_accepts_custom_history_depth() {
        let options = normalize_startup_options(Some(RawStartupOptions {
            start_year: Some(2026),
            start_phase: Some("seasonStart".to_string()),
            history_depth_years: Some(24),
        }))
        .unwrap();

        assert_eq!(options.history_depth_years, 24);
    }

    #[test]
    fn start_date_for_year_uses_selected_july_first() {
        let start_date = start_date_for_year(2032).unwrap();

        assert_eq!(start_date.to_rfc3339(), "2032-07-01T00:00:00+00:00");
    }

    #[test]
    fn start_date_for_year_rejects_out_of_range_years() {
        let result = start_date_for_year(i32::MAX);

        assert_eq!(
            result.unwrap_err(),
            "be.error.createManager.invalidStartYear"
        );
    }

    #[test]
    fn current_date_for_midseason_phase_is_after_start_date() {
        let current_date = current_date_for_phase(2032, StartPhase::MidSeason).unwrap();

        assert_eq!(current_date.to_rfc3339(), "2032-10-29T00:00:00+00:00");
    }

    #[test]
    fn age_on_date_uses_selected_start_year() {
        let birth_date = chrono::NaiveDate::from_ymd_opt(2008, 1, 1).unwrap();
        let reference_date = current_date_for_phase(2038, StartPhase::SeasonStart)
            .unwrap()
            .date_naive();

        assert_eq!(age_on_date(birth_date, reference_date), 30);
    }

    #[test]
    fn age_on_date_changes_between_season_start_and_midseason() {
        let birth_date = chrono::NaiveDate::from_ymd_opt(2008, 8, 1).unwrap();
        let season_start = current_date_for_phase(2038, StartPhase::SeasonStart)
            .unwrap()
            .date_naive();
        let midseason = current_date_for_phase(2038, StartPhase::MidSeason)
            .unwrap()
            .date_naive();

        assert_eq!(age_on_date(birth_date, season_start), 29);
        assert_eq!(age_on_date(birth_date, midseason), 30);
    }

    #[test]
    fn age_on_date_uses_world_snapshot_date_over_startup_phase() {
        let startup_options = StartupOptions {
            start_year: 2032,
            start_phase: StartPhase::MidSeason,
            history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
        };
        let world = make_historical_snapshot_world();
        let reference_date = game_clock_for_world(&startup_options, &world.metadata)
            .unwrap()
            .current_date
            .date_naive();
        let birth_date = chrono::NaiveDate::from_ymd_opt(2001, 12, 15).unwrap();

        assert_eq!(reference_date.to_string(), "2031-11-20");
        assert_eq!(age_on_date(birth_date, reference_date), 29);
    }

    #[test]
    fn preseason_league_setup_uses_selected_start_year_for_context() {
        let clock = GameClock::new(start_date_for_year(2032).unwrap());
        let manager = domain::manager::Manager::new(
            "mgr1".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let teams = vec![
            domain::team::Team::new(
                "team1".to_string(),
                "Alpha FC".to_string(),
                "AFC".to_string(),
                "England".to_string(),
                "London".to_string(),
                "Alpha Park".to_string(),
                20_000,
            ),
            domain::team::Team::new(
                "team2".to_string(),
                "Beta FC".to_string(),
                "BFC".to_string(),
                "England".to_string(),
                "Manchester".to_string(),
                "Beta Park".to_string(),
                22_000,
            ),
        ];
        let mut game = Game::new(clock, manager, teams, vec![], vec![], vec![]);

        let season_start = preseason_season_start(&game.clock);
        let team_ids = game
            .teams
            .iter()
            .map(|team| team.id.clone())
            .collect::<Vec<_>>();
        game.league = Some(ofm_core::schedule::generate_league(
            "Premier Division",
            preseason_league_year(&game.clock),
            &team_ids,
            season_start,
        ));
        refresh_game_context(&mut game);

        assert_eq!(
            game.clock.start_date.to_rfc3339(),
            "2032-07-01T00:00:00+00:00"
        );
        assert_eq!(game.league.as_ref().map(|league| league.season), Some(2032));
        assert_eq!(
            game.season_context.season_start.as_deref(),
            Some("2032-07-31")
        );
        assert_eq!(game.season_context.days_until_season_start, Some(30));
    }

    #[test]
    fn apply_generated_past_history_populates_default_twelve_prior_seasons() {
        let clock = GameClock::new(start_date_for_year(2032).unwrap());
        let manager = domain::manager::Manager::new(
            "mgr-user".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let teams = vec![
            domain::team::Team::new(
                "team1".to_string(),
                "Alpha FC".to_string(),
                "AFC".to_string(),
                "England".to_string(),
                "London".to_string(),
                "Alpha Park".to_string(),
                20_000,
            ),
            domain::team::Team::new(
                "team2".to_string(),
                "Beta FC".to_string(),
                "BFC".to_string(),
                "England".to_string(),
                "Manchester".to_string(),
                "Beta Park".to_string(),
                22_000,
            ),
        ];
        let staff = vec![
            {
                let mut staff = domain::staff::Staff::new(
                    "staff1".to_string(),
                    "Pat".to_string(),
                    "Coach".to_string(),
                    "1978-01-01".to_string(),
                    domain::staff::StaffRole::AssistantManager,
                    domain::staff::StaffAttributes {
                        coaching: 70,
                        judging_ability: 65,
                        judging_potential: 64,
                        physiotherapy: 40,
                    },
                );
                staff.nationality = "England".to_string();
                staff.team_id = Some("team1".to_string());
                staff
            },
            {
                let mut staff = domain::staff::Staff::new(
                    "staff2".to_string(),
                    "Lee".to_string(),
                    "Coach".to_string(),
                    "1979-01-01".to_string(),
                    domain::staff::StaffRole::AssistantManager,
                    domain::staff::StaffAttributes {
                        coaching: 72,
                        judging_ability: 66,
                        judging_potential: 65,
                        physiotherapy: 39,
                    },
                );
                staff.nationality = "England".to_string();
                staff.team_id = Some("team2".to_string());
                staff
            },
        ];
        let players = vec![
            {
                let mut player = domain::player::Player::new(
                    "player1".to_string(),
                    "A. Keeper".to_string(),
                    "Alex Keeper".to_string(),
                    "1994-01-01".to_string(),
                    "England".to_string(),
                    domain::player::Position::Goalkeeper,
                    domain::player::PlayerAttributes {
                        pace: 48,
                        stamina: 62,
                        strength: 64,
                        agility: 66,
                        passing: 50,
                        shooting: 20,
                        tackling: 18,
                        dribbling: 32,
                        defending: 24,
                        positioning: 68,
                        vision: 48,
                        decisions: 63,
                        composure: 61,
                        aggression: 38,
                        teamwork: 64,
                        leadership: 58,
                        handling: 76,
                        reflexes: 77,
                        aerial: 72,
                    },
                );
                player.team_id = Some("team1".to_string());
                player.ovr = 68;
                player.potential = 73;
                player
            },
            {
                let mut player = domain::player::Player::new(
                    "player2".to_string(),
                    "A. Striker".to_string(),
                    "Alex Striker".to_string(),
                    "1996-01-01".to_string(),
                    "England".to_string(),
                    domain::player::Position::Striker,
                    domain::player::PlayerAttributes {
                        pace: 72,
                        stamina: 68,
                        strength: 70,
                        agility: 71,
                        passing: 60,
                        shooting: 79,
                        tackling: 34,
                        dribbling: 73,
                        defending: 28,
                        positioning: 74,
                        vision: 62,
                        decisions: 68,
                        composure: 69,
                        aggression: 52,
                        teamwork: 64,
                        leadership: 47,
                        handling: 18,
                        reflexes: 18,
                        aerial: 61,
                    },
                );
                player.team_id = Some("team1".to_string());
                player.ovr = 74;
                player.potential = 80;
                player
            },
            {
                let mut player = domain::player::Player::new(
                    "player3".to_string(),
                    "B. Keeper".to_string(),
                    "Ben Keeper".to_string(),
                    "1993-01-01".to_string(),
                    "England".to_string(),
                    domain::player::Position::Goalkeeper,
                    domain::player::PlayerAttributes {
                        pace: 47,
                        stamina: 61,
                        strength: 63,
                        agility: 65,
                        passing: 49,
                        shooting: 19,
                        tackling: 18,
                        dribbling: 30,
                        defending: 23,
                        positioning: 67,
                        vision: 47,
                        decisions: 62,
                        composure: 60,
                        aggression: 39,
                        teamwork: 63,
                        leadership: 57,
                        handling: 75,
                        reflexes: 76,
                        aerial: 71,
                    },
                );
                player.team_id = Some("team2".to_string());
                player.ovr = 67;
                player.potential = 72;
                player
            },
            {
                let mut player = domain::player::Player::new(
                    "player4".to_string(),
                    "B. Striker".to_string(),
                    "Ben Striker".to_string(),
                    "1995-01-01".to_string(),
                    "England".to_string(),
                    domain::player::Position::Striker,
                    domain::player::PlayerAttributes {
                        pace: 71,
                        stamina: 67,
                        strength: 69,
                        agility: 70,
                        passing: 59,
                        shooting: 78,
                        tackling: 33,
                        dribbling: 72,
                        defending: 27,
                        positioning: 73,
                        vision: 61,
                        decisions: 67,
                        composure: 68,
                        aggression: 51,
                        teamwork: 63,
                        leadership: 46,
                        handling: 18,
                        reflexes: 18,
                        aerial: 60,
                    },
                );
                player.team_id = Some("team2".to_string());
                player.ovr = 73;
                player.potential = 79;
                player
            },
        ];
        let mut game = Game::new(clock, manager, teams, players, staff, vec![]);

        apply_generated_past_history(
            &mut game,
            &StartupOptions {
                start_year: 2032,
                start_phase: StartPhase::SeasonStart,
                history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
            },
        );

        assert!(game.teams.iter().all(|team| team.history.len() == 12));
        assert_eq!(game.world_history.season_awards.len(), 12);
        assert!(game.players.iter().any(|player| player.career.len() == 12));
        assert!(game
            .managers
            .iter()
            .any(|manager| !manager.career_history.is_empty()));
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
    fn game_clock_for_world_rejects_out_of_range_snapshot_base_year() {
        let startup_options = StartupOptions {
            start_year: 2032,
            start_phase: StartPhase::MidSeason,
            history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
        };
        let mut world = make_historical_snapshot_world();
        world.metadata.base_year = Some(i32::MAX);

        let result = game_clock_for_world(&startup_options, &world.metadata);

        assert_eq!(
            result.unwrap_err(),
            "be.error.createManager.invalidStartYear"
        );
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
    fn require_active_stats_state_returns_backend_key_when_missing() {
        let state = StateManager::new();

        let result = require_active_stats_state(&state);

        assert_eq!(result.unwrap_err(), "be.error.noActiveStatsSession");
    }

    #[test]
    fn require_active_stats_state_clones_active_stats() {
        let state = StateManager::new();
        let stats = sample_stats_state();
        state.set_stats_state(stats.clone());

        let result = require_active_stats_state(&state).unwrap();

        assert_eq!(result.team_matches.len(), stats.team_matches.len());
        assert_eq!(result.player_matches.len(), stats.player_matches.len());
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
}
