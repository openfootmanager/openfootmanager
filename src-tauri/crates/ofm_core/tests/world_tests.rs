//! A generated world, driven through consecutive season rollovers.
//!
//! These live here rather than beside the command layer because the world
//! builder does: `ensure_multi_competition_foundations` is what every new
//! career gets its divisions, cups and continental competition from, and until
//! it moved into `ofm_core` no test outside the Tauri crate could build a
//! pyramid at all.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{TimeZone, Utc};
use domain::league::{CompetitionFormat, League};
use domain::manager::Manager;

use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::world::ladder::{division_sizes, ladder_violations};
use ofm_core::world::{
    ensure_multi_competition_foundations, resolve_simulation_scope, start_date_for_year,
};

/// A club that belongs to `nation`, ranked by `reputation`.
///
/// Integration tests cannot reach the crate's `#[cfg(test)]` fixtures, so this
/// mirrors `ofm_core::world::test_fixtures::nation_team`. It encodes no rule —
/// every value but the nation and the reputation is filler.
fn nation_team(id: &str, nation: &str, reputation: u32) -> domain::team::Team {
    let mut team = domain::team::Team::new(
        id.to_string(),
        id.to_string(),
        id.to_string(),
        nation.to_string(),
        "City".to_string(),
        "Stadium".to_string(),
        10_000,
    );
    team.football_nation = nation.to_string();
    team.reputation = reputation;
    team
}

fn manager_for(team_id: &str) -> Manager {
    let mut manager = Manager::new(
        "mgr".to_string(),
        "A".to_string(),
        "B".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire(team_id.to_string());
    manager
}

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

fn roster(competition: &League) -> BTreeSet<String> {
    competition.participant_ids.iter().cloned().collect()
}

/// The ladder rule lives in `ofm_core::world::ladder`, where the season harness
/// asks it too; this only turns its answer into a failed assertion.
fn assert_one_league_per_club(game: &Game, sizes: &BTreeMap<String, usize>, label: &str) {
    let violations = ladder_violations(game, sizes);
    assert!(violations.is_empty(), "{label}: {violations:#?}");
}

/// A catch-up draws from a generator it is handed; these tests do not care which.
fn game_seeded_rng(n: u64) -> impl rand::Rng {
    ofm_core::seed::rng_for_seed(n, "world-tests", "2100-01-01")
}

#[test]
fn a_generated_world_promotes_and_relegates_for_three_seasons_running() {
    let mut game = production_world(2035, "eng-00");
    let (regions, competitions) =
        resolve_simulation_scope(&game, "eng-00", None, None).expect("a valid scope");
    game.active_region_ids = regions;
    game.active_competition_ids = competitions;
    let sizes = division_sizes(&game);
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
            let mut rng = game_seeded_rng(rollover);
            ofm_core::catchup::simulate_past_fixtures(competition, &players, far_future, &mut rng);
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
            .filter_map(|fixture| chrono::NaiveDate::parse_from_str(&fixture.date, "%Y-%m-%d").ok())
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
            .filter_map(|fixture| chrono::NaiveDate::parse_from_str(&fixture.date, "%Y-%m-%d").ok())
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
                let mut rng = game_seeded_rng(0);
                ofm_core::catchup::simulate_past_fixtures(competition, &players, cutoff, &mut rng);
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
