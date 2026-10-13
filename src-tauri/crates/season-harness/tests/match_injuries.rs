//! What match injuries do to a world once the engine's injury events are applied (#835).
//!
//! Every test is `#[ignore]`d (outside the PR gate) and meant for release mode:
//!
//! `cargo test --release -p season-harness --test match_injuries -- --ignored --nocapture`
//!
//! The world is played with every competition active, so every club fixture goes through the
//! engine and the production post-match path, which is where the injuries are applied.

use domain::league::League;
use ofm_core::end_of_season::EndOfSeasonSummary;
use ofm_core::game::Game;
use ofm_core::squad_floor::squad_shortfall;
use season_harness::driver::{RunOptions, SeasonObserver, run_seasons};
use season_harness::worlds::{Scope, WorldSpec};

const SEEDS: &[u64] = &[1, 2, 3];
const SEASONS: u32 = 3;

fn engine_played_fixtures(game: &Game) -> impl Iterator<Item = &domain::league::MatchResult> {
    game.competitions
        .iter()
        .flat_map(|competition: &League| &competition.fixtures)
        .filter_map(|fixture| fixture.result.as_ref())
        .filter(|result| result.report.is_some())
}

/// Counts the engine's injury events in the matches that were played, and notes any club (bar the
/// player's own, which the harness makes no signings for) that is ever short of the squad floor.
#[derive(Default)]
struct InjuryWatch {
    matches: u64,
    injury_events: u64,
    injured_at_season_end: Vec<(String, String, u32)>,
    floor_breaches: Vec<String>,
}

impl InjuryWatch {
    fn note_floor(&mut self, game: &Game, when: &str, season: u32) {
        let user_club = game.manager.team_id.clone();
        for team in &game.teams {
            if user_club.as_deref() == Some(team.id.as_str()) {
                continue;
            }
            let shortfall = squad_shortfall(game, &team.id);
            if !shortfall.is_empty() {
                self.floor_breaches.push(format!(
                    "[season {season}, {when}] {} is short: {shortfall:?}",
                    team.id
                ));
            }
        }
    }
}

impl SeasonObserver for InjuryWatch {
    fn on_day_end(&mut self, game: &Game, season: u32, day: u32) {
        // Weekly is enough to catch a club that cannot field a side, and keeps the run fast.
        if day % 7 == 0 {
            self.note_floor(game, &format!("day {day}"), season);
        }
    }

    fn on_before_rollover(&mut self, game: &Game, season: u32) {
        for result in engine_played_fixtures(game) {
            self.matches += 1;
            self.injury_events += result
                .report
                .as_ref()
                .map(|report| {
                    report
                        .events
                        .iter()
                        .filter(|event| event.event_type == "Injury")
                        .count() as u64
                })
                .unwrap_or(0);
        }
        self.injured_at_season_end
            .extend(game.players.iter().filter_map(|player| {
                player.injury.as_ref().map(|injury| {
                    (
                        player.id.clone(),
                        injury.name.clone(),
                        injury.days_remaining,
                    )
                })
            }));
        self.note_floor(game, "season complete", season);
    }

    fn on_season_end(&mut self, _game: &Game, _summary: &EndOfSeasonSummary, _season: u32) {}
}

fn play(seed: u64) -> InjuryWatch {
    let mut game = WorldSpec::gate(seed)
        .with_scope(Scope::Everything)
        .build()
        .expect("the world builds");
    let mut watch = InjuryWatch::default();
    run_seasons(&mut game, RunOptions::seasons(SEASONS), &mut watch)
        .unwrap_or_else(|error| panic!("seed {seed}: {error}"));
    watch
}

/// Given a gate world with every competition active, played for several seasons over several
///       seeds with match injuries applied,
/// When the engine-played matches are counted,
/// Then the engine's injury events per match are reported, and no AI club is ever short of the
///      squad floor (a club that cannot put a side out).
#[test]
#[ignore = "measurement and nightly check, run by hand in release mode"]
fn match_injuries_never_leave_a_club_unable_to_field_a_side() {
    let mut matches = 0;
    let mut events = 0;
    for &seed in SEEDS {
        let watch = play(seed);
        println!(
            "seed {seed}: {} matches, {} injury events ({:.3} per match), {} players injured at season ends",
            watch.matches,
            watch.injury_events,
            watch.injury_events as f64 / watch.matches.max(1) as f64,
            watch.injured_at_season_end.len()
        );
        assert!(
            watch.floor_breaches.is_empty(),
            "seed {seed}:\n{}",
            watch.floor_breaches.join("\n")
        );
        matches += watch.matches;
        events += watch.injury_events;
    }
    println!(
        "all seeds: {matches} matches, {events} injury events, {:.3} per match",
        events as f64 / matches.max(1) as f64
    );
}

/// Given the same seed,
/// When a world is played twice,
/// Then the same players are injured for the same durations at every season's end.
#[test]
#[ignore = "measurement and nightly check, run by hand in release mode"]
fn the_same_seed_injures_the_same_players_for_the_same_time() {
    let first = play(1);
    let second = play(1);

    assert_eq!(first.injured_at_season_end, second.injured_at_season_end);
}
