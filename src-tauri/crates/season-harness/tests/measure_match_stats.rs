//! What a club match looks like on the shipped world, as the numbers the calibration bands read:
//! goals and the outcome shares, shots and how many are on target and scored, penalties, cards,
//! fouls and corners, per match.
//!
//! `OUTCOME_SEED=<n> cargo test --release --manifest-path src-tauri/Cargo.toml -p season-harness --test measure_match_stats -- --ignored --nocapture`
//!
//! Every played club fixture of one season, every competition, from the report each keeps.
//! Compact reports drop blocked shots, so the share of *unblocked* shots on target (the cited
//! accuracy) is measured in `engine/tests/shot_rate_tests.rs`, not here. The
//! goal level is the subject of #1132; read the shots line first, since a goal rate is shots ×
//! on-target share × on-target conversion, plus penalties.

use domain::league::{CompetitionType, FixtureStatus};
use ofm_core::game::Game;
use season_harness::driver::{RunOptions, SeasonObserver, run_seasons};
use season_harness::worlds::{Scope, WorldSpec};

#[derive(Default)]
struct Totals {
    matches: u64,
    with_report: u64,
    goals: u64,
    home_clean_sheets: u64,
    away_clean_sheets: u64,
    both_scored: u64,
    home_wins: u64,
    shots: u64,
    on_target: u64,
    open_and_set_piece_goals: u64,
    penalty_goals: u64,
    penalty_misses: u64,
    /// How often each event type appears in the kept reports, to show what they record.
    event_types: std::collections::BTreeMap<String, u64>,
    fouls: u64,
    corners: u64,
    yellows: u64,
    reds: u64,
}

impl SeasonObserver for Totals {
    fn on_before_rollover(&mut self, game: &Game, _season: u32) {
        for fixture in game
            .competitions
            .iter()
            .filter(|competition| competition.kind != CompetitionType::InternationalNation)
            .flat_map(|competition| &competition.fixtures)
            .filter(|fixture| fixture.status == FixtureStatus::Completed)
        {
            let Some(result) = &fixture.result else {
                continue;
            };
            let (home, away) = (u64::from(result.home_goals), u64::from(result.away_goals));
            self.matches += 1;
            self.goals += home + away;
            self.home_clean_sheets += u64::from(away == 0);
            self.away_clean_sheets += u64::from(home == 0);
            self.both_scored += u64::from(home > 0 && away > 0);
            self.home_wins += u64::from(home > away);
            let Some(report) = &result.report else {
                continue;
            };
            self.with_report += 1;
            for side in [&report.home_stats, &report.away_stats] {
                self.shots += u64::from(side.shots);
                self.on_target += u64::from(side.shots_on_target);
                self.fouls += u64::from(side.fouls);
                self.corners += u64::from(side.corners);
                self.yellows += u64::from(side.yellow_cards);
                self.reds += u64::from(side.red_cards);
            }
            for event in &report.events {
                *self
                    .event_types
                    .entry(event.event_type.clone())
                    .or_default() += 1;
                match event.event_type.as_str() {
                    "Goal" => self.open_and_set_piece_goals += 1,
                    "PenaltyGoal" => self.penalty_goals += 1,
                    "PenaltyMiss" => self.penalty_misses += 1,
                    _ => {}
                }
            }
        }
    }
}

/// Given the shipped world with every competition played,
/// When one season is played,
/// Then two lines report the outcome shares and the per-match statistics the bands read.
#[test]
#[ignore = "measurement, run by hand in release mode"]
fn match_statistics_of_the_shipped_world() {
    let seeds: Vec<u64> = match std::env::var("OUTCOME_SEED") {
        Ok(seed) => vec![seed.parse().expect("OUTCOME_SEED is a number")],
        Err(_) => vec![1, 2],
    };
    for seed in seeds {
        let spec = WorldSpec::shipped(seed).with_scope(Scope::Everything);
        let mut game = spec.build().expect("the world builds");
        let mut t = Totals::default();
        run_seasons(&mut game, RunOptions::seasons(1), &mut t).expect("the season plays");
        let n = t.matches.max(1) as f64;
        let r = t.with_report.max(1) as f64;
        let pct = |count: u64, of: f64| 100.0 * count as f64 / of;
        // The report counts a penalty kick as a shot, and a scored one as a shot on target, so
        // the open-play and set-piece conversion leaves them out of both.
        let open_play_shots = t.shots.saturating_sub(t.penalty_goals + t.penalty_misses);
        let open_play_on_target = t.on_target.saturating_sub(t.penalty_goals);
        println!(
            "OUTCOMES shipped seed {seed}: matches {} | goals/game {:.3} | clean sheets home {:.2}% away {:.2}% | both scored {:.2}% | home wins {:.2}%",
            t.matches,
            t.goals as f64 / n,
            pct(t.home_clean_sheets, n),
            pct(t.away_clean_sheets, n),
            pct(t.both_scored, n),
            pct(t.home_wins, n),
        );
        println!(
            "MATCH_STATS shipped seed {seed}: reports {} | shots/game {:.2} | on target {:.2} ({:.1}% of shots) | open-play and set-piece goals {:.3}/game, {:.1}% of those shots and {:.1}% of those on target | penalties taken {:.3}/game, scored {:.1}%, {:.1}% of goals | fouls {:.2} | corners {:.2} | yellows {:.2} | reds {:.3}",
            t.with_report,
            t.shots as f64 / r,
            t.on_target as f64 / r,
            pct(t.on_target, t.shots.max(1) as f64),
            t.open_and_set_piece_goals as f64 / r,
            pct(t.open_and_set_piece_goals, open_play_shots.max(1) as f64),
            pct(
                t.open_and_set_piece_goals,
                open_play_on_target.max(1) as f64
            ),
            (t.penalty_goals + t.penalty_misses) as f64 / r,
            pct(
                t.penalty_goals,
                (t.penalty_goals + t.penalty_misses).max(1) as f64
            ),
            pct(
                t.penalty_goals,
                (t.open_and_set_piece_goals + t.penalty_goals).max(1) as f64
            ),
            t.fouls as f64 / r,
            t.corners as f64 / r,
            t.yellows as f64 / r,
            t.reds as f64 / r,
        );
        let kinds: Vec<String> = t
            .event_types
            .iter()
            .map(|(kind, count)| format!("{kind} {count}"))
            .collect();
        println!(
            "MATCH_STATS shipped seed {seed} events kept: {}",
            kinds.join(", ")
        );
    }
}
