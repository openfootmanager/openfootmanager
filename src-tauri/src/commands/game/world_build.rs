//! Turning loaded world data into a playable `Game`.
//!
//! Two jobs that only look separate. Building the `Game` itself — teams,
//! players, staff, the manager's club — and deriving the structure the
//! competition layer will need from it: which region a club belongs to, which
//! clubs represent a nation, and how a country's clubs divide into tiers.

use domain::league::League;
use domain::manager::Manager;
use domain::stats::StatsState;
use ofm_core::clock::GameClock;
use ofm_core::game::Game;

use ofm_core::world::preseason_league_year;

use super::{apply_generated_past_history, ensure_multi_competition_foundations, StartupOptions};

pub(super) fn build_game_from_world_data(
    clock: GameClock,
    manager: Manager,
    startup_options: &StartupOptions,
    world: ofm_core::generator::WorldData,
) -> (Game, StatsState) {
    // Resolve any authored competition definitions while we still hold the
    // world (validation already passed at load). These replace the auto-built
    // foundation competitions.
    let game_start = clock.start_date;
    let defined_competitions: Vec<League> = world
        .competition_definitions
        .as_ref()
        .map(|file| {
            let mut comps = ofm_core::generator::resolve_definitions(
                file,
                &world,
                preseason_league_year(&clock),
                game_start,
            );
            for comp in &mut comps {
                let (_, is_mid_season) = ofm_core::generator::start_date_at_game_open(
                    game_start,
                    comp.season_start_month,
                    comp.season_start_day,
                );
                if is_mid_season {
                    ofm_core::catchup::simulate_past_fixtures(comp, &world.players, game_start);
                }
            }
            comps
        })
        .unwrap_or_default();

    let ofm_core::generator::WorldData {
        teams,
        players,
        staff,
        managers,
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
    if game
        .staff
        .iter()
        .any(|staff_member| staff_member.team_id.is_none())
    {
        game.available_staff_market_last_activity_date =
            Some(game.clock.current_date.format("%Y-%m-%d").to_string());
    }
    ofm_core::generator::repair_opening_youth_academies(&mut game);

    // Authored definitions take precedence over both the snapshot's stored
    // competitions and the auto-built foundations.
    let competitions = if defined_competitions.is_empty() {
        competitions
    } else {
        defined_competitions
    };

    match metadata.kind {
        ofm_core::generator::WorldDataKind::HistoricalSnapshot => {
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
            ensure_multi_competition_foundations(&mut game);
            ofm_core::season_context::refresh_game_context(&mut game);
            (game, stats)
        }
        ofm_core::generator::WorldDataKind::RosterBaseline => {
            // Authored definitions, if any, become the world's competitions;
            // otherwise ensure_multi_competition_foundations auto-builds them.
            game.competitions = competitions;
            game.extra_translations = extra_translations;
            // Build the league/division foundations *before* generating history so
            // each club's past seasons are attributed to its real ~20-team
            // division. Otherwise history runs with no competitions and treats the
            // whole world as one mega-league (≈880-match seasons).
            ensure_multi_competition_foundations(&mut game);
            apply_generated_past_history(&mut game, startup_options);
            (game, StatsState::default())
        }
    }
}
