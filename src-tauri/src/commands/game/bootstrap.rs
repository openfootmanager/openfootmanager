//! Putting a manager into a world that already exists.
//!
//! Three ways in, and the difference between them is what the world already
//! contains. A generated world gets a league and an opening-day inbox; a world
//! with competitions, news and stats of its own gets a takeover instead; and a
//! mid-season start simulates forward until the club is half a season in.
//! `bootstrap_team_selection` picks between them.

use db::save_manager::SaveManager;
use domain::stats::StatsState;
use ofm_core::contracts::{club_season_anchors, stamp_opening_contract_starts};
use ofm_core::game::Game;
use ofm_core::world::{
    ensure_multi_competition_foundations, rebuild_competitions_for_management_date,
    team_season_anchor,
};

use super::{
    preseason_league_year, preseason_season_start, start_phase_for_game, StartPhase,
    DEFAULT_LEAGUE_NAME, DEFAULT_LEAGUE_NAME_KEY, ISO_DATE_FORMAT,
};

// Only `bootstrap_game_for_mcp` needs these, and it is behind the feature.
#[cfg(feature = "mcp")]
use {
    super::{
        build_game_from_world_data, default_save_name, game_clock_for_world,
        load_world_data_from_path, map_save_manager_lock_error, normalize_startup_options,
    },
    chrono::Datelike,
    domain::manager::Manager,
    log::info,
    ofm_core::state::StateManager,
};

/// When the player picks `SeasonStart` for a southern-hemisphere (or other
/// non-August-start) club, align the game clock to that club's actual season-start
/// date and rebuild competitions from that anchor, so the player arrives at the
/// beginning of their season, not in July.
fn align_clock_to_club_season(game: &mut Game, team_id: &str) {
    if start_phase_for_game(game) == StartPhase::SeasonStart {
        if let Some(actual_start) = team_season_anchor(game, team_id) {
            if actual_start < game.clock.current_date {
                game.clock.current_date = actual_start;
                game.clock.start_date = actual_start;
                rebuild_competitions_for_management_date(game, actual_start);
                game.national_teams.clear();
                ensure_multi_competition_foundations(game);
            }
        }
    }
}

/// Settle the date a career opens on, then give every contract that has no start the
/// day it began, measured against that date.
///
/// `align_clock_to` is the club whose season the clock is pulled back to, for the
/// paths that do that. The order is the point and lives here so it is written once:
/// every club's season anchor is read *first*, because Brazil's is worked out from
/// the clock's year and reading it after the clock has moved lands a year early;
/// the clock is then moved; and starts are stamped last, against the date the career
/// really opens on, so none can land after it.
pub(crate) fn date_opening_contracts(game: &mut Game, align_clock_to: Option<&str>) {
    let club_anchors = club_season_anchors(game);
    if let Some(team_id) = align_clock_to {
        align_clock_to_club_season(game, team_id);
    }
    stamp_opening_contract_starts(game, &club_anchors);
}

pub(super) fn has_existing_world_context(game: &Game, stats_state: &StatsState) -> bool {
    !game.competitions.is_empty()
        || game.league.is_some()
        || !game.news.is_empty()
        || !stats_state.player_matches.is_empty()
        || !stats_state.team_matches.is_empty()
}

pub(super) fn bootstrap_existing_world_takeover(
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

pub(super) fn bootstrap_season_start(game: &mut Game, team_id: &str) -> Result<StatsState, String> {
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

pub(super) fn competitive_fixture_count_for_team(game: &Game, team_id: &str) -> usize {
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

pub(super) fn completed_competitive_fixture_count_for_team(game: &Game, team_id: &str) -> usize {
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

pub(super) fn bootstrap_midseason_takeover(
    game: &mut Game,
    team_id: &str,
) -> Result<StatsState, String> {
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

    // World generation has already equipped every club for AI management.
    // At career selection the chosen club becomes the player's blank slate;
    // rivals keep the identity the generator gave them.
    if let Some(team) = game.teams.iter_mut().find(|team| team.id == team_id) {
        team.tactics_phase = domain::team::TacticsPhaseSettings::default();
        team.player_roles.clear();
    }

    ofm_core::transfers::seed_opening_ai_loan_market(game);
    Ok(stats_state)
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
    // This path does not align the clock to a club's season, so nothing moves the
    // opening date; contracts are still dated against it.
    date_opening_contracts(&mut game, None);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::game::testkit::{make_bootstrap_test_game, sample_stats_state};

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

    /// A Brazilian career opens on 15 December of the year *before* the clock's, so
    /// aligning the clock to that club moves it back by half a year. Every contract
    /// has to be dated against the date the career actually opens on, and each
    /// club's anchor has to have been read before the move: Brazil's is worked out
    /// from the clock's year, so reading it afterwards lands a year too early.
    #[test]
    fn opening_a_brazilian_career_dates_no_contract_after_the_opening_date() {
        let mut game = make_bootstrap_test_game();
        let mut brazilian = domain::team::Team::new(
            "br-1".to_string(),
            "Santos FC".to_string(),
            "SAN".to_string(),
            "BR".to_string(),
            "Santos".to_string(),
            "Vila".to_string(),
            20_000,
        );
        brazilian.football_nation = "BR".to_string();
        game.teams.push(brazilian);
        for index in 0..3 {
            let mut player = game.players[0].clone();
            player.id = format!("br-player-{index}");
            player.team_id = Some("br-1".to_string());
            game.players.push(player);
        }
        for player in &mut game.players {
            player.contract_end = Some("2033-06-30".to_string());
        }
        ofm_core::world::ensure_multi_competition_foundations(&mut game);

        date_opening_contracts(&mut game, Some("br-1"));

        let opening = game.clock.current_date.date_naive();
        assert_eq!(
            opening.to_string(),
            "2031-12-15",
            "the clock should have been aligned to the Brazilian club's season"
        );
        let brazilian_start = game
            .players
            .iter()
            .find(|player| player.id == "br-player-0")
            .and_then(|player| player.contract_start.clone());
        assert_eq!(
            brazilian_start.as_deref(),
            Some("2031-12-15"),
            "the anchor was read after the clock moved, a year too early"
        );
        // Only people under contract: foundations also mint national-team fillers,
        // who belong to no club and rightly have neither a start nor an end.
        for player in game
            .players
            .iter()
            .filter(|player| player.team_id.is_some() && player.contract_end.is_some())
        {
            let start = player
                .contract_start
                .as_deref()
                .unwrap_or_else(|| panic!("{} was given no contract start", player.id));
            assert!(
                start <= opening.to_string().as_str(),
                "{} starts {start}, after the career opens on {opening}",
                player.id
            );
        }
    }
}
