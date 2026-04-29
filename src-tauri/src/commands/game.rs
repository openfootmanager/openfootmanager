use log::info;
use tauri::State;

use db::save_index::SaveEntry;
use domain::manager::Manager;
use domain::stats::StatsState;
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::state::StateManager;

use crate::SaveManagerState;

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
    world_source: Option<String>,
) -> Result<Game, String> {
    info!(
        "[cmd] start_new_game: {} {} (nationality={}, world_source={:?})",
        first_name, last_name, nationality, world_source
    );
    // Validate inputs
    let first_name = first_name.trim().to_string();
    let last_name = last_name.trim().to_string();
    if first_name.is_empty() || last_name.is_empty() {
        return Err("First name and last name are required.".to_string());
    }
    if first_name.len() > 30 || last_name.len() > 30 {
        return Err("First name and last name must not exceed 30 characters.".to_string());
    }
    let nationality = nationality.trim().to_string();
    if nationality.is_empty() {
        return Err("Nationality is required.".to_string());
    }

    // Validate DOB: must be a valid date and manager must be at least 30 years old
    let birth_date = chrono::NaiveDate::parse_from_str(&dob, "%Y-%m-%d")
        .map_err(|_| "Invalid date of birth. Use YYYY-MM-DD format.".to_string())?;
    let today = chrono::Utc::now().date_naive();
    let age = today.signed_duration_since(birth_date).num_days() / 365;
    if age < 30 {
        return Err("Manager must be at least 30 years old.".to_string());
    }
    if age > 99 {
        return Err("Invalid date of birth.".to_string());
    }

    let manager = Manager::new(
        "mgr_user".to_string(),
        first_name,
        last_name,
        dob,
        nationality,
    );

    use chrono::TimeZone;
    let start_date = chrono::Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap();
    let clock = GameClock::new(start_date);

    // Load world based on source
    let world_source = world_source.unwrap_or_else(|| "random".to_string());
    let (teams, players, staff) = if world_source == "random" {
        let world = ofm_core::generator::generate_world_data(None);
        (world.teams, world.players, world.staff)
    } else {
        // Try to load from file path (strip "file:" prefix if present)
        let path = world_source.strip_prefix("file:").unwrap_or(&world_source);
        let json = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read world database: {}", e))?;
        let mut world = ofm_core::generator::load_world_from_json(&json)?;
        ofm_core::generator::ensure_minimum_world_rosters(&mut world, None);
        (world.teams, world.players, world.staff)
    };

    let new_game = Game::new(clock, manager, teams, players, staff, vec![]);

    info!(
        "[cmd] start_new_game: world generated with {} teams, {} players, {} staff",
        new_game.teams.len(),
        new_game.players.len(),
        new_game.staff.len()
    );
    state.set_game(new_game.clone());
    state.set_stats_state(StatsState::default());
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
        .ok_or("No active game session".to_string())?;

    // Validate team exists
    let team = game
        .teams
        .iter()
        .find(|t| t.id == team_id)
        .ok_or("Team not found".to_string())?;
    let team_name = team.name.clone();
    let team_country = team.country.clone();
    let team_league_name = if team.domestic_league.trim().is_empty() {
        format!("{team_country} Premier Division")
    } else {
        team.domestic_league.clone()
    };

    // Assign manager to team
    game.manager.hire(team_id.clone());
    if let Some(t) = game.teams.iter_mut().find(|t| t.id == team_id) {
        t.manager_id = Some(game.manager.id.clone());
    }

    // enerate league schedule from the selected team's country with the season supposed to starts 1 month after game start
    // i did not review this code after some other changes, so the naming is outdated
    use chrono::Duration;
    let season_start = game.clock.current_date + Duration::days(30);
    let same_league_team_ids: Vec<String> = game
        .teams
        .iter()
        .filter(|candidate| {
            candidate.country == team_country
                && {
                    let league_name = if candidate.domestic_league.trim().is_empty() {
                        format!("{} Premier Division", candidate.country)
                    } else {
                        candidate.domestic_league.clone()
                    };
                    league_name == team_league_name
                }
        })
        .map(|candidate| candidate.id.clone())
        .collect();
    let league_name = team_league_name.clone();
    // Keep competitions strictly separated by domestic league.
    // Never fallback to "all teams", otherwise leagues get mixed.
    let league_team_ids = same_league_team_ids;
    let mut league =
        ofm_core::schedule::generate_league(&league_name, 2026, &league_team_ids, season_start);
    let opponents: Vec<String> = league_team_ids
        .iter()
        .filter(|candidate_team_id| candidate_team_id.as_str() != team_id)
        .cloned()
        .collect();
    let friendlies =
        ofm_core::schedule::generate_preseason_friendlies(&team_id, &opponents, season_start, 3);
    ofm_core::schedule::append_fixtures(&mut league, friendlies);
    let cup_round_date = season_start + Duration::days(10);
    let cup_fixtures = ofm_core::schedule::generate_domestic_cup_round(&league_team_ids, cup_round_date, 100);
    ofm_core::schedule::append_fixtures(&mut league, cup_fixtures);
    game.league = Some(league.clone());
    let mut all_leagues = vec![league];
    use std::collections::BTreeMap;
    let mut leagues_by_key: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    for candidate in &game.teams {
        let league_label = if candidate.domestic_league.trim().is_empty() {
            format!("{} Premier Division", candidate.country)
        } else {
            candidate.domestic_league.clone()
        };
        leagues_by_key
            .entry((candidate.country.clone(), league_label))
            .or_default()
            .push(candidate.id.clone());
    }
    for ((country_name, league_label), ids) in leagues_by_key {
        if country_name == team_country && league_label == team_league_name {
            continue;
        }
        if ids.len() < 2 {
            continue;
        }
        let mut parallel = ofm_core::schedule::generate_league(
            &league_label,
            2026,
            &ids,
            season_start,
        );
        let cup_fixtures =
            ofm_core::schedule::generate_domestic_cup_round(&ids, season_start + Duration::days(10), 100);
        ofm_core::schedule::append_fixtures(&mut parallel, cup_fixtures);
        all_leagues.push(parallel);
    }
    game.leagues = all_leagues;
    ofm_core::season_context::refresh_game_context(&mut game);

    // Rich templated messages
    let date_str = game.clock.current_date.to_rfc3339();
    let welcome_msg = ofm_core::messages::welcome_message(&team_name, &team_id, &date_str);
    game.messages.push(welcome_msg);

    let season_msg = ofm_core::messages::season_schedule_message(
        &league_name,
        &season_start.format("%B %d, %Y").to_string(),
        &date_str,
    );
    game.messages.push(season_msg);

    let team_names: Vec<String> = game.teams.iter().map(|team| team.name.clone()).collect();
    game.news.push(ofm_core::news::season_preview_article(
        &team_names,
        &date_str,
    ));

    let staff_msg = ofm_core::messages::staff_advice_message(&team_name, &team_id, &date_str);
    game.messages.push(staff_msg);

    ofm_core::player_events::generate_contract_concern_messages(&mut game, false);

    // Save to new per-save DB
    let manager_name = format!("{} {}", game.manager.first_name, game.manager.last_name);
    let save_name = format!("{}'s Career", manager_name);

    let mut sm = sm_state
        .0
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;
    let save_id = sm.create_save(&game, &save_name)?;
    state.set_save_id(save_id);

    state.set_game(game.clone());
    state.set_stats_state(StatsState::default());
    Ok(game)
}

#[tauri::command]
pub async fn get_saves(sm_state: State<'_, SaveManagerState>) -> Result<Vec<SaveEntry>, String> {
    log::debug!("[cmd] get_saves");
    let sm = sm_state
        .0
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;
    Ok(sm.list_saves().to_vec())
}

#[tauri::command]
pub async fn delete_save(
    sm_state: State<'_, SaveManagerState>,
    save_id: String,
) -> Result<bool, String> {
    info!("[cmd] delete_save: save_id={}", save_id);
    let mut sm = sm_state
        .0
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;
    sm.delete_save(&save_id)
}

#[tauri::command]
pub async fn load_game(
    state: State<'_, StateManager>,
    sm_state: State<'_, SaveManagerState>,
    save_id: String,
) -> Result<String, String> {
    info!("[cmd] load_game: save_id={}", save_id);
    let mut sm = sm_state
        .0
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;
    let mut game = sm.load_game(&save_id)?;
    let stats_state = sm.load_stats_state(&save_id)?;
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
        .ok_or("No active game session".to_string())
}

#[tauri::command]
pub async fn save_game(
    state: State<'_, StateManager>,
    sm_state: State<'_, SaveManagerState>,
) -> Result<(), String> {
    info!("[cmd] save_game");
    let game = state
        .get_game(|g: &Game| g.clone())
        .ok_or("No active game session".to_string())?;

    let save_id = state
        .get_save_id()
        .ok_or("No active save session".to_string())?;

    let mut sm = sm_state
        .0
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;
    sm.save_game(&game, &save_id)?;
    let stats_state = state
        .get_stats_state(|stats| stats.clone())
        .unwrap_or_default();
    sm.save_stats_state(&stats_state, &save_id)
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
        .ok_or("No active game session")?;

    // Auto-save
    if let Some(save_id) = state.get_save_id() {
        let mut sm = sm_state
            .0
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        sm.save_game(&game, &save_id)?;
        let stats_state = state
            .get_stats_state(|stats| stats.clone())
            .unwrap_or_default();
        sm.save_stats_state(&stats_state, &save_id)?;
    }

    // Clear the in-memory game state
    state.clear_game();
    state.clear_save_id();

    Ok(())
}
