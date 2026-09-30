//! Persisting a new career, and the MCP auto-start that builds one from a file.
//!
//! The game rules for putting a manager in charge of a club live in
//! `ofm_core::career`; this is the part that touches saves and MCP.

use db::save_manager::SaveManager;
use domain::stats::StatsState;
#[cfg(feature = "mcp")]
use ofm_core::career::{bootstrap_team_selection, date_opening_contracts, start_phase_for_game};
use ofm_core::game::Game;

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

pub(crate) fn create_new_save(
    save_manager: &mut SaveManager,
    game: &Game,
    stats_state: &StatsState,
    save_name: &str,
) -> Result<String, String> {
    save_manager.create_save_with_stats(game, stats_state, save_name)
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

        ofm_core::career::date_opening_contracts(&mut game, Some("br-1"));

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
