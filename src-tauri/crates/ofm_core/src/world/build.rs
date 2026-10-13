//! Turning loaded world data into a playable `Game`: the authored competition definitions
//! resolved, a season in progress caught up, the squad floor restored.
//!
//! The Tauri layer and the season harness both build their games here, so a world a player starts
//! and a world a test plays are assembled by the same code.

use domain::league::League;
use domain::manager::Manager;
use domain::stats::StatsState;

use crate::clock::GameClock;
use crate::game::Game;
use crate::generator::{ReputationBasis, WorldData, WorldDataKind};

/// How much past a freshly generated world is given: seasons of history ending in `start_year`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PastHistory {
    pub start_year: i32,
    pub depth_years: u32,
}

/// Builds the game for `world`, resolving its competition definitions with clubs ranked by
/// `reputation_basis`.
pub fn build_game_from_world_data(
    clock: GameClock,
    manager: Manager,
    history: PastHistory,
    world: WorldData,
    reputation_basis: ReputationBasis<'_>,
) -> (Game, StatsState) {
    // Resolve any authored competition definitions while we still hold the
    // world (validation already passed at load). These replace the auto-built
    // foundation competitions.
    let game_start = clock.start_date;
    // The game's seed, settled first because the catch-up of an in-progress season below
    // draws from it: the world's own when it was generated from one, a fresh draw for a
    // world nothing seeded (an import, a package).
    let seed = world.generation_seed.unwrap_or_else(rand::random);
    let defined_competitions: Vec<League> = world
        .competition_definitions
        .as_ref()
        .map(|file| {
            let mut comps = crate::generator::resolve_definitions_ranked(
                file,
                &world,
                reputation_basis,
                crate::world::preseason_league_year(&clock),
                game_start,
            );
            for comp in &mut comps {
                let (_, is_mid_season) = crate::generator::start_date_at_game_open(
                    game_start,
                    comp.season_start_month,
                    comp.season_start_day,
                );
                if is_mid_season {
                    let mut rng = crate::seed::rng_for_seed(
                        seed,
                        &format!("catchup/definitions/{}", comp.id),
                        &game_start.format("%Y-%m-%d").to_string(),
                    );
                    crate::catchup::simulate_past_fixtures(
                        comp,
                        &world.players,
                        game_start,
                        &mut rng,
                    );
                }
            }
            comps
        })
        .unwrap_or_default();

    let WorldData {
        teams,
        players,
        staff,
        managers,
        retired_managers,
        retired_players,
        competitions,
        national_teams,
        default_active_regions,
        default_active_competitions,
        league,
        news,
        stats,
        world_history,
        metadata,
        extra_translations,
        ..
    } = world;

    let mut game = Game::new(clock, manager, teams, players, staff, vec![]);
    // Stored on the game, which is the one place the day's dice come from.
    game.seed = seed;
    // Before any history is generated: seeding managers asks the archive whether a club has had one.
    game.retired_managers = retired_managers;
    game.retired_players = retired_players;
    if game
        .staff
        .iter()
        .any(|staff_member| staff_member.team_id.is_none())
    {
        game.available_staff_market_last_activity_date =
            Some(game.clock.current_date.format("%Y-%m-%d").to_string());
    }
    crate::generator::repair_opening_youth_academies(&mut game);

    // Authored definitions take precedence over both the snapshot's stored
    // competitions and the auto-built foundations.
    let competitions = if defined_competitions.is_empty() {
        competitions
    } else {
        defined_competitions
    };

    let (mut game, stats) = match metadata.kind {
        WorldDataKind::HistoricalSnapshot => {
            game.managers.extend(
                managers
                    .into_iter()
                    .filter(|existing_manager| existing_manager.id != game.manager.id),
            );
            game.competitions = competitions;
            game.national_teams = national_teams;
            game.active_region_ids = default_active_regions;
            game.active_competition_ids = default_active_competitions;
            game.league = league;
            game.promote_legacy_league();
            game.news = news;
            game.world_history = world_history;
            game.extra_translations = extra_translations;
            crate::world::ensure_multi_competition_foundations(&mut game);
            crate::season_context::refresh_game_context(&mut game);
            (game, stats)
        }
        WorldDataKind::RosterBaseline => {
            // Authored definitions, if any, become the world's competitions;
            // otherwise ensure_multi_competition_foundations auto-builds them.
            game.competitions = competitions;
            game.extra_translations = extra_translations;
            // Build the league/division foundations *before* generating history so
            // each club's past seasons are attributed to its real ~20-team
            // division. Otherwise history runs with no competitions and treats the
            // whole world as one mega-league (≈880-match seasons).
            crate::world::ensure_multi_competition_foundations(&mut game);
            crate::history_generation::generate_past_world_history(
                &mut game,
                history.start_year,
                history.depth_years,
            );
            (game, StatsState::default())
        }
    };
    // Once the competitions are in place, so "does the player's club play
    // today" has an answer. A package can leave a club thin.
    crate::squad_floor::repair_squads_on_load(&mut game);
    (game, stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::{DefinitionSources, WorldGenConfig, generate_world_data_seeded_with};
    use crate::world::start_date_for_year;

    fn manager() -> Manager {
        Manager::new(
            "mgr".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        )
    }

    /// Given a world generated from a seed,
    /// When a game is built from it,
    /// Then the game carries the seed, every club and player, a competition structure, and the
    ///      past seasons it was asked for.
    #[test]
    fn a_seeded_world_becomes_a_game_with_its_seed_competitions_and_history() {
        let sources = DefinitionSources::embedded_only();
        let world = generate_world_data_seeded_with(5, &WorldGenConfig::compact(), &sources);
        let (clubs, players) = (world.teams.len(), world.players.len());
        let clock = GameClock::new(start_date_for_year(2033).unwrap());
        let history = PastHistory {
            start_year: 2033,
            depth_years: 2,
        };

        let (game, _) =
            build_game_from_world_data(clock, manager(), history, world, ReputationBasis::Stored);

        assert_eq!(game.seed, 5);
        assert_eq!(game.teams.len(), clubs);
        assert!(game.players.len() >= players);
        assert!(!game.competitions.is_empty());
        assert!(game.teams.iter().all(|team| team.history.len() == 2));
    }

    /// Given a world that carries archived managers and retired players
    /// When a game is built from it
    /// Then the game holds both archives, so an imported career remembers a club's past managers.
    #[test]
    fn a_world_s_archives_become_the_games_archives() {
        let sources = DefinitionSources::embedded_only();
        let mut world = generate_world_data_seeded_with(5, &WorldGenConfig::compact(), &sources);
        let mut retired_manager = manager();
        retired_manager.id = "retired-boss".to_string();
        world.retired_managers.push(retired_manager);
        let mut veteran = world.players[0].clone();
        veteran.id = "retired-veteran".to_string();
        world
            .retired_players
            .push(domain::retired_player::RetiredPlayer::from_player(
                &veteran,
                "2030-06-30",
                2030,
            ));
        let clock = GameClock::new(start_date_for_year(2033).unwrap());
        let history = PastHistory {
            start_year: 2033,
            depth_years: 0,
        };

        let (game, _) =
            build_game_from_world_data(clock, manager(), history, world, ReputationBasis::Stored);

        assert_eq!(game.retired_managers.len(), 1);
        assert_eq!(game.retired_managers[0].id, "retired-boss");
        assert_eq!(game.retired_players.len(), 1);
        assert_eq!(game.retired_players[0].id, "retired-veteran");
    }
}
