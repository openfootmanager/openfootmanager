//! Every season's calendar is clash-free at the moment it is generated, across rollovers and
//! reloads. Kept apart from the gate: the gate holds the whole run (including the cup rounds
//! drawn while it is played) to the rest rule, and that is a different claim.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use domain::league::{FixtureCompetition, FixtureStatus};
use ofm_core::calendar_booking::{MIN_GAP_DAYS, too_close};
use ofm_core::game::Game;
use season_harness::driver::{RunOptions, run_seasons};
use season_harness::invariants::Invariants;
use season_harness::worlds::WorldSpec;

/// Pairs of a club's matches still to be played that sit closer than the minimum rest.
fn unplayed_clashes(game: &Game) -> Vec<String> {
    let today = game.clock.current_date.date_naive();
    let mut by_club: BTreeMap<&str, Vec<(NaiveDate, &str, &str)>> = BTreeMap::new();
    for competition in &game.competitions {
        for fixture in &competition.fixtures {
            if fixture.competition == FixtureCompetition::InternationalNation
                || fixture.status != FixtureStatus::Scheduled
            {
                continue;
            }
            let Ok(date) = NaiveDate::parse_from_str(&fixture.date, "%Y-%m-%d") else {
                continue;
            };
            if date < today {
                continue;
            }
            for club in [&fixture.home_team_id, &fixture.away_team_id] {
                by_club
                    .entry(club)
                    .or_default()
                    .push((date, &competition.id, &fixture.id));
            }
        }
    }
    let mut clashes = Vec::new();
    for (club, mut rows) in by_club {
        rows.sort();
        for pair in rows.windows(2) {
            if too_close(pair[0].0, pair[1].0) {
                clashes.push(format!(
                    "{club}: {} {} and {} {}",
                    pair[0].1, pair[0].0, pair[1].1, pair[1].0
                ));
            }
        }
    }
    clashes
}

/// Given a generated world played through four seasons, saved and reloaded between each,
/// When every rollover has generated the next season's calendar,
/// Then no club has two matches to play within the minimum rest in that calendar, and neither has
///      the calendar the world was built with.
#[test]
fn four_seasons_generate_clash_free_calendars_with_reloads() {
    assert_eq!(MIN_GAP_DAYS, 3, "the rule this test states");
    let mut game = WorldSpec::gate(1).build().expect("the world builds");
    assert_eq!(
        unplayed_clashes(&game),
        Vec::<String>::new(),
        "the built world"
    );
    let mut clashes = Vec::new();

    for _ in 0..4 {
        let mut invariants = Invariants::new(&game);
        run_seasons(&mut game, RunOptions::seasons(1), &mut invariants)
            .unwrap_or_else(|error| panic!("{error}"));
        // The run ends on the rollover, so this is the calendar it has just generated.
        clashes.extend(unplayed_clashes(&game));
        game = serde_json::from_slice(&serde_json::to_vec(&game).unwrap()).unwrap();
    }

    assert_eq!(clashes, Vec::<String>::new());
}
