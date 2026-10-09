//! Persisting a new career, and the MCP auto-start that builds one from a file.
//!
//! The game rules for putting a manager in charge of a club live in
//! `ofm_core::career`; this is the part that touches saves and MCP.

use db::save_manager::SaveManager;
use domain::stats::StatsState;
#[cfg(feature = "mcp")]
use ofm_core::career::{begin_career, date_opening_contracts, CareerScope};
use ofm_core::game::Game;

// Only `start_career_for_mcp` needs these, and it is behind the feature.
#[cfg(feature = "mcp")]
use {
    super::{
        build_game_from_world_data, default_save_name, game_clock_for_world,
        load_world_data_from_path, map_save_manager_lock_error, normalize_startup_options,
        RawStartupOptions, StartupOptions,
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

/// What an MCP client may choose when it creates a career. Everything is optional,
/// and the defaults are the app's: this year, joining at the start of the season.
#[cfg(feature = "mcp")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct McpCareerOptions {
    pub seed: Option<u64>,
    pub start_year: Option<i32>,
    pub start_phase: Option<String>,
}

#[cfg(feature = "mcp")]
pub struct McpNewCareer<'a> {
    /// A world file to load; `None`, an empty string or `"random"` generates one.
    pub world_source: Option<&'a str>,
    pub team_id: Option<&'a str>,
    pub manager_first_name: &'a str,
    pub manager_last_name: &'a str,
    pub manager_nationality: &'a str,
    pub options: McpCareerOptions,
}

/// Creates a manager and a world for an MCP client.
///
/// With a club (named, or already the world's manager's) the career begins and is
/// saved, and the save id comes back. Without one the game is installed clubless,
/// as the app's first step leaves it, and `game_select_team` finishes the job.
#[cfg(feature = "mcp")]
pub fn start_career_for_mcp(
    state_manager: &StateManager,
    save_manager_state: &crate::SaveManagerState,
    request: &McpNewCareer<'_>,
) -> Result<Option<String>, String> {
    let startup_options = normalize_startup_options(Some(RawStartupOptions::new(
        request.options.start_year,
        request.options.start_phase.clone(),
    )))?;
    let mut world = load_world_for_mcp(request.world_source, request.options.seed)?;

    let opening_year = u32::try_from(
        game_clock_for_world(&startup_options, &world.metadata)?
            .start_date
            .year(),
    )
    .unwrap_or_else(|_| ofm_core::generator::default_opening_year());
    if is_world_file(request.world_source) {
        ofm_core::generator::normalize_imported_world_for_career_start(&mut world, opening_year);
    }

    let manager = take_or_create_manager(&mut world, request, &startup_options)?;
    let clock = game_clock_for_world(&startup_options, &world.metadata)?;
    let (mut game, current_stats_state) =
        build_game_from_world_data(clock, manager, &startup_options, world);

    info!(
        "[mcp-bootstrap] Built game: {} teams, {} players, manager.team_id={:?}",
        game.teams.len(),
        game.players.len(),
        game.manager.team_id,
    );

    // A manager the world already placed at a club is resuming a career, so the
    // clock stays where the world put it. Otherwise a club is being chosen, which
    // is `begin_career`'s job, and it dates contracts itself after placing the clock.
    let stats_state = if game.manager.team_id.is_some() {
        date_opening_contracts(&mut game, None);
        ofm_core::ai_hiring::seed_ai_managers(&mut game);
        ofm_core::season_context::refresh_game_context(&mut game);
        ofm_core::transfers::seed_opening_ai_loan_market(&mut game);
        current_stats_state
    } else if let Some(team_id) = request.team_id {
        begin_career(
            &mut game,
            team_id,
            CareerScope::default(),
            current_stats_state,
        )?
    } else {
        crate::application::career::install_career(state_manager, game, current_stats_state, None);
        return Ok(None);
    };

    let manager_name = format!("{} {}", game.manager.first_name, game.manager.last_name);
    let save_name = default_save_name(&manager_name);
    let mut sm = map_save_manager_lock_error(save_manager_state.0.lock())?;
    let save_id = create_new_save(&mut sm, &game, &stats_state, &save_name)?;

    crate::application::career::install_career(
        state_manager,
        game,
        stats_state,
        Some(save_id.clone()),
    );
    info!("[mcp-bootstrap] Game saved with ID: {}", save_id);
    Ok(Some(save_id))
}

#[cfg(feature = "mcp")]
fn is_world_file(world_source: Option<&str>) -> bool {
    !matches!(world_source, None | Some("") | Some("random"))
}

#[cfg(feature = "mcp")]
fn load_world_for_mcp(
    world_source: Option<&str>,
    seed: Option<u64>,
) -> Result<ofm_core::generator::WorldData, String> {
    if !is_world_file(world_source) {
        let seed = seed.unwrap_or_else(rand::random);
        return Ok(ofm_core::generator::generate_world_data_seeded_with(
            seed,
            &ofm_core::generator::WorldGenConfig::compact(),
            &ofm_core::generator::DefinitionSources::embedded_only(),
        ));
    }
    let mut world = load_world_data_from_path(world_source.unwrap_or_default())?;
    if seed.is_some() {
        world.generation_seed = seed;
    }
    Ok(world)
}

/// A snapshot's own `mgr_user` keeps its club and history; anything else gets a
/// fresh 45-year-old manager.
#[cfg(feature = "mcp")]
fn take_or_create_manager(
    world: &mut ofm_core::generator::WorldData,
    request: &McpNewCareer<'_>,
    startup_options: &StartupOptions,
) -> Result<Manager, String> {
    if let Some(idx) = world.managers.iter().position(|m| m.id == "mgr_user") {
        let mut existing = world.managers.remove(idx);
        // "Agent", "Manager" and "England" are the auto-start defaults, not a choice.
        if request.manager_first_name != "Agent" {
            existing.first_name = request.manager_first_name.to_string();
        }
        if request.manager_last_name != "Manager" {
            existing.last_name = request.manager_last_name.to_string();
        }
        if request.manager_nationality != "England" {
            existing.nationality = request.manager_nationality.to_string();
        }
        return Ok(existing);
    }
    let reference_date = game_clock_for_world(startup_options, &world.metadata)?
        .current_date
        .date_naive();
    let dob = reference_date - chrono::Duration::days(45 * 365);
    Ok(Manager::new(
        "mgr_user".to_string(),
        request.manager_first_name.to_string(),
        request.manager_last_name.to_string(),
        dob.format("%Y-%m-%d").to_string(),
        request.manager_nationality.to_string(),
    ))
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
}

#[cfg(all(test, feature = "mcp"))]
mod mcp_career_tests {
    use super::*;
    use ofm_core::career::{start_phase_for_game, StartPhase};

    struct Sandbox {
        state: StateManager,
        saves: crate::SaveManagerState,
        _dir: tempfile::TempDir,
    }

    fn sandbox() -> Sandbox {
        let dir = tempfile::tempdir().unwrap();
        let saves = crate::SaveManagerState(std::sync::Mutex::new(
            SaveManager::init(dir.path()).unwrap(),
        ));
        Sandbox {
            state: StateManager::new(),
            saves,
            _dir: dir,
        }
    }

    fn start(
        sandbox: &Sandbox,
        team_id: Option<&str>,
        options: McpCareerOptions,
    ) -> Result<Option<String>, String> {
        start_career_for_mcp(
            &sandbox.state,
            &sandbox.saves,
            &McpNewCareer {
                world_source: None,
                team_id,
                manager_first_name: "Ada",
                manager_last_name: "Lovelace",
                manager_nationality: "England",
                options,
            },
        )
    }

    fn seeded(seed: u64) -> McpCareerOptions {
        McpCareerOptions {
            seed: Some(seed),
            ..Default::default()
        }
    }

    fn identities(sandbox: &Sandbox) -> (Vec<String>, Vec<String>, Vec<String>) {
        sandbox
            .state
            .get_game(|game| {
                (
                    game.teams.iter().map(|t| t.id.clone()).collect(),
                    game.players.iter().map(|p| p.id.clone()).collect(),
                    game.competitions.iter().map(|c| c.id.clone()).collect(),
                )
            })
            .unwrap()
    }

    /// Given a sandbox with no game and no world file
    /// When a career is created from a seed with no club
    /// Then a generated world is installed, clubless, and nothing is saved.
    #[test]
    fn a_generated_world_needs_no_world_source() {
        let sandbox = sandbox();

        let save_id = start(&sandbox, None, seeded(7)).unwrap();

        assert_eq!(save_id, None);
        let (club, teams) = sandbox
            .state
            .get_game(|game| (game.manager.team_id.clone(), game.teams.len()))
            .unwrap();
        assert_eq!(club, None);
        assert!(teams > 0);
        assert_eq!(sandbox.state.get_save_id(), None);
    }

    /// Given two sandboxes
    /// When both generate from one seed, and a third from another
    /// Then team, player and competition ids agree for the first two and differ for the third.
    #[test]
    fn the_same_seed_gives_the_same_core() {
        let (one, two, other) = (sandbox(), sandbox(), sandbox());
        start(&one, None, seeded(7)).unwrap();
        start(&two, None, seeded(7)).unwrap();
        start(&other, None, seeded(8)).unwrap();

        assert_eq!(identities(&one), identities(&two));
        assert_ne!(identities(&one).0, identities(&other).0);
        assert_eq!(one.state.get_game(|g| g.seed), Some(7));
    }

    /// Given a generated world and a club chosen from it
    /// When the career starts at seasonStart and at midSeason in 2030
    /// Then each opens in 2030 in its own phase, and is saved.
    #[test]
    fn the_start_options_reach_the_career() {
        let probe = sandbox();
        start(&probe, None, seeded(7)).unwrap();
        let club = probe.state.get_game(|g| g.teams[0].id.clone()).unwrap();

        for (phase, expected) in [
            ("seasonStart", StartPhase::SeasonStart),
            ("midSeason", StartPhase::MidSeason),
        ] {
            let sandbox = sandbox();
            let options = McpCareerOptions {
                seed: Some(7),
                start_year: Some(2030),
                start_phase: Some(phase.to_string()),
            };

            let save_id = start(&sandbox, Some(&club), options).unwrap();

            assert!(save_id.is_some());
            let (year, started) = sandbox
                .state
                .get_game(|g| {
                    (
                        chrono::Datelike::year(&g.clock.current_date),
                        start_phase_for_game(g),
                    )
                })
                .unwrap();
            assert_eq!((year, started), (2030, expected));
        }
    }

    /// Given no career
    /// When creation is asked for an invalid phase
    /// Then it is refused, no game is installed, and a valid creation then succeeds.
    #[test]
    fn an_invalid_phase_is_refused() {
        let sandbox = sandbox();
        let bad = McpCareerOptions {
            start_phase: Some("nextWeek".to_string()),
            ..seeded(7)
        };

        let refused = start(&sandbox, None, bad);

        assert_eq!(
            refused.unwrap_err(),
            "be.error.createManager.invalidStartPhase"
        );
        assert!(sandbox.state.get_game(|_| ()).is_none());
        assert!(start(&sandbox, None, seeded(7)).is_ok());
    }

    /// Given a club that is not in the generated world
    /// When the career is created for it
    /// Then it is refused and nothing is installed or saved.
    #[test]
    fn an_unknown_club_installs_nothing() {
        let sandbox = sandbox();

        let refused = start(&sandbox, Some("no_such_team"), seeded(7));

        assert_eq!(refused.unwrap_err(), "be.error.teamNotFound");
        assert!(sandbox.state.get_game(|_| ()).is_none());
        assert_eq!(sandbox.state.get_save_id(), None);
    }
}
