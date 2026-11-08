//! What becomes of a shot in open play, between two equal sides, against what real football
//! records. A goal rate is shots × the share on target × the share of those scored; the shot
//! volume is in band, so these two shares carry the calibration (#1132).
//!
//! - **The share on target, not counting blocked shots**: the Premier League's own figure is
//!   45.10% over two decades, and 49.96% in 2023/24 ("Season trends: Clinical finishing key to
//!   record goals total", Adrian Clarke, 27 May 2024, https://www.premierleague.com/news/4027257,
//!   which gives both figures to two decimals; its accuracy table is "ex. blocks").
//!   Cited, not derived.
//! - **The share of shots on target scored** has no league-wide figure we can cite from an open
//!   source, so `goal_conversion_base` is calibrated to the derived goal total instead: the
//!   shipped world's goals a game against #1001's band, median 2.71 (openfootball, CC0). The band
//!   here is that calibration's result between two equal sides, with room for the counts' noise.
//!   It was set again after #1139 put more good players into the world (3.1 goals a game, home
//!   clean sheets under the band): lower still, away clean sheets leave their band first.

use engine::live_match::LiveMatchState;
use engine::{EventType, MatchConfig};
use rand::SeedableRng;
use rand::rngs::StdRng;

mod common;
use common::equal_side;

const SEEDS: u64 = 2000;

/// Open-play shots over every seed: scored, saved, off target (blocked shots are left out, as
/// the cited accuracy leaves them out).
#[derive(Default)]
struct Shots {
    scored: u32,
    saved: u32,
    off_target: u32,
}

impl Shots {
    fn on_target(&self) -> u32 {
        self.scored + self.saved
    }

    /// The share of shots that were not blocked that went on target.
    fn accuracy(&self) -> f64 {
        f64::from(self.on_target()) / f64::from(self.on_target() + self.off_target)
    }

    /// The share of shots on target that were scored.
    fn conversion(&self) -> f64 {
        f64::from(self.scored) / f64::from(self.on_target())
    }
}

/// The open-play shots of SEEDS matches between equal sides, played once and shared by both
/// tests.
fn shots_between_equal_sides() -> &'static Shots {
    static SHOTS: std::sync::OnceLock<Shots> = std::sync::OnceLock::new();
    SHOTS.get_or_init(play_equal_sides)
}

fn play_equal_sides() -> Shots {
    let mut shots = Shots::default();
    for seed in 0..SEEDS {
        let mut state = LiveMatchState::new(
            equal_side("h"),
            equal_side("a"),
            MatchConfig::default(),
            vec![],
            vec![],
            false,
        );
        let mut rng = StdRng::seed_from_u64(seed);
        loop {
            let minute = state.step_minute(&mut rng);
            for event in &minute.events {
                match event.event_type {
                    EventType::Goal => shots.scored += 1,
                    EventType::ShotSaved => shots.saved += 1,
                    EventType::ShotOffTarget => shots.off_target += 1,
                    _ => {}
                }
            }
            if minute.is_finished {
                break;
            }
        }
    }
    shots
}

/// Given two equal sides of 70-rated players, over many seeds,
/// When their open-play shots are counted,
/// Then the share of unblocked shots on target is in the Premier League's range, 45.10% over two
/// decades to 49.96% in 2023/24, widened by about 3 points either side for these 70-rated
/// shooters and the counts' noise (about ±0.4 points at SEEDS matches).
#[test]
fn a_shot_finds_the_target_as_often_as_in_real_football() {
    let shots = shots_between_equal_sides();
    let accuracy = shots.accuracy();
    assert!(
        (ACCURACY_LOW..=ACCURACY_HIGH).contains(&accuracy),
        "{:.1}% of unblocked shots went on target ({} on target, {} off)",
        100.0 * accuracy,
        shots.on_target(),
        shots.off_target
    );
}

/// Given two equal sides of 70-rated players, over many seeds,
/// When their open-play shots on target are counted,
/// Then the share scored is the calibrated one (see the module comment): 24.6% to 28.6% of shots
/// on target, where the engine scored 30% before #1139's world and 38% before #1132.
#[test]
fn a_shot_on_target_is_scored_as_often_as_the_goal_total_needs() {
    let shots = shots_between_equal_sides();
    let conversion = shots.conversion();
    assert!(
        (CONVERSION_LOW..=CONVERSION_HIGH).contains(&conversion),
        "{:.1}% of shots on target were scored ({} of {})",
        100.0 * conversion,
        shots.scored,
        shots.on_target()
    );
}

/// The Premier League's 45.10% (two decades) to 49.96% (2023/24), widened by about 3 points
/// either side for these 70-rated shooters and the counts' noise.
const ACCURACY_LOW: f64 = 0.42;
const ACCURACY_HIGH: f64 = 0.53;
/// The calibrated 26.6% between equal 70-rated sides, two points either side: about five times
/// the noise of a share over some 16,000 shots on target.
const CONVERSION_LOW: f64 = 0.246;
const CONVERSION_HIGH: f64 = 0.286;
