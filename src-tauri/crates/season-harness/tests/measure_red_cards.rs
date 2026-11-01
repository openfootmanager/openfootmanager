//! What a red card does to a club match on the shipped world: how often one is shown, how each
//! side scores after it, and the points the side that receives it takes.
//!
//! `OUTCOME_SEED=<n> cargo test --release --manifest-path src-tauri/Cargo.toml -p season-harness --test measure_red_cards -- --ignored --nocapture`
//!
//! Scoring after a red card is compared with what sides score over the same minutes of matches
//! without one, so a red shown late is not credited with the late goals every match has. The
//! literature it is read against: Ridder, Cramer and Hopstaken (1994, 340 Dutch league matches)
//! found the side left with eleven scores 88% more often after the red card; Červený, van Ours
//! and van Tuijl (2018, World Cup matches) found the sanctioned side's rate falls and the other
//! side's rises.

use domain::league::{CompactMatchReport, CompetitionType, MatchResult};
use ofm_core::game::Game;
use season_harness::driver::{RunOptions, SeasonObserver, run_seasons};
use season_harness::worlds::{Scope, WorldSpec};

const MINUTES: usize = 131;

fn is_goal(event_type: &str) -> bool {
    matches!(event_type, "Goal" | "PenaltyGoal")
}

fn is_dismissal(event_type: &str) -> bool {
    matches!(event_type, "RedCard" | "SecondYellow")
}

fn points(scored: u8, conceded: u8) -> u32 {
    match scored.cmp(&conceded) {
        std::cmp::Ordering::Greater => 3,
        std::cmp::Ordering::Equal => 1,
        std::cmp::Ordering::Less => 0,
    }
}

#[derive(Default)]
struct Watch {
    matches: u64,
    dismissals: u64,
    matches_with_one: u64,
    /// Over matches without a red card: goals scored at each minute by either side, and how many
    /// side-matches reached that minute.
    goals_at: Vec<f64>,
    sides_at: Vec<f64>,
    /// Matches with a red card, kept for the second pass.
    red_matches: Vec<RedMatch>,
    points_sanctioned: u32,
    points_opponent: u32,
    wins_sanctioned: u32,
    wins_opponent: u32,
    /// Wins over all sides of matches without a red card, and how many such sides there were.
    wins_without_a_red: u32,
    sides_without_a_red: u32,
}

struct RedMatch {
    /// Minute of the first red card, and the minute the window ends (a second red, or the end).
    from: usize,
    to: usize,
    sanctioned_goals: u32,
    opponent_goals: u32,
}

impl Watch {
    fn note(&mut self, result: &MatchResult, report: &CompactMatchReport) {
        self.matches += 1;
        let end = usize::from(report.total_minutes).min(MINUTES - 1);
        let reds: Vec<_> = report
            .events
            .iter()
            .filter(|event| is_dismissal(&event.event_type))
            .collect();
        self.dismissals += reds.len() as u64;
        let Some(first) = reds.first() else {
            self.sides_without_a_red += 2;
            self.wins_without_a_red += u32::from(result.home_goals != result.away_goals);
            for minute in 1..=end {
                self.sides_at[minute] += 2.0;
            }
            for event in report.events.iter().filter(|e| is_goal(&e.event_type)) {
                self.goals_at[usize::from(event.minute).min(MINUTES - 1)] += 1.0;
            }
            return;
        };
        self.matches_with_one += 1;
        let from = usize::from(first.minute);
        let to = reds
            .get(1)
            .map(|second| usize::from(second.minute))
            .unwrap_or(end)
            .min(end);
        let sanctioned = first.side.clone();
        let (mut sanctioned_goals, mut opponent_goals) = (0, 0);
        for event in report.events.iter().filter(|e| is_goal(&e.event_type)) {
            let minute = usize::from(event.minute);
            if minute > from && minute <= to {
                if event.side == sanctioned {
                    sanctioned_goals += 1;
                } else {
                    opponent_goals += 1;
                }
            }
        }
        let (home, away) = (result.home_goals, result.away_goals);
        let (sanctioned_points, opponent_points) = if sanctioned == "Home" {
            (points(home, away), points(away, home))
        } else {
            (points(away, home), points(home, away))
        };
        self.points_sanctioned += sanctioned_points;
        self.points_opponent += opponent_points;
        self.wins_sanctioned += u32::from(sanctioned_points == 3);
        self.wins_opponent += u32::from(opponent_points == 3);
        self.red_matches.push(RedMatch {
            from,
            to,
            sanctioned_goals,
            opponent_goals,
        });
    }

    /// Goals one side is expected to score over minutes `from + 1 ..= to` of a match without a
    /// red card.
    fn expected(&self, from: usize, to: usize) -> f64 {
        (from + 1..=to)
            .map(|minute| {
                if self.sides_at[minute] > 0.0 {
                    self.goals_at[minute] / self.sides_at[minute]
                } else {
                    0.0
                }
            })
            .sum()
    }
}

impl SeasonObserver for Watch {
    fn on_before_rollover(&mut self, game: &Game, _season: u32) {
        for fixture in game
            .competitions
            .iter()
            .filter(|competition| competition.kind != CompetitionType::InternationalNation)
            .flat_map(|competition| &competition.fixtures)
        {
            let Some(result) = fixture.result.as_ref() else {
                continue;
            };
            let Some(report) = result.report.as_ref() else {
                continue;
            };
            self.note(result, report);
        }
    }
}

/// Given the shipped world with every competition played,
/// When one season is played,
/// Then one line reports red cards per match and, after a red card, each side's scoring against
/// the same minutes of matches without one, and the points each side took.
#[test]
#[ignore = "measurement, run by hand in release mode"]
fn red_cards_in_the_shipped_world() {
    let seeds: Vec<u64> = match std::env::var("OUTCOME_SEED") {
        Ok(seed) => vec![seed.parse().expect("OUTCOME_SEED is a number")],
        Err(_) => vec![1, 2],
    };
    for seed in seeds {
        let spec = WorldSpec::shipped(seed).with_scope(Scope::Everything);
        let mut game = spec.build().expect("the world builds");
        let mut watch = Watch {
            goals_at: vec![0.0; MINUTES],
            sides_at: vec![0.0; MINUTES],
            ..Watch::default()
        };
        run_seasons(&mut game, RunOptions::seasons(1), &mut watch).expect("the season plays");
        let (mut sanctioned, mut opponent, mut expected) = (0u32, 0u32, 0.0f64);
        for red in &watch.red_matches {
            sanctioned += red.sanctioned_goals;
            opponent += red.opponent_goals;
            expected += watch.expected(red.from, red.to);
        }
        let with_one = watch.matches_with_one.max(1) as f64;
        println!(
            "RED_CARDS seed {seed}: matches {} | dismissals {} ({:.4} per match) | matches with one {} | after it, sanctioned side scored {sanctioned} against {expected:.1} expected ({:.3}x) and the other side {opponent} ({:.3}x) | points per match: sanctioned {:.3}, other side {:.3} | wins: sanctioned {:.1}%, other side {:.1}%, a side in a match without a red {:.1}%",
            watch.matches,
            watch.dismissals,
            watch.dismissals as f64 / watch.matches.max(1) as f64,
            watch.matches_with_one,
            f64::from(sanctioned) / expected.max(f64::MIN_POSITIVE),
            f64::from(opponent) / expected.max(f64::MIN_POSITIVE),
            f64::from(watch.points_sanctioned) / with_one,
            f64::from(watch.points_opponent) / with_one,
            100.0 * f64::from(watch.wins_sanctioned) / with_one,
            100.0 * f64::from(watch.wins_opponent) / with_one,
            100.0 * f64::from(watch.wins_without_a_red)
                / f64::from(watch.sides_without_a_red.max(1)),
        );
    }
}
