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

use engine::live_match::LiveMatchState;
use engine::{
    EventType, MatchConfig, PlayStyle, PlayerData, PlayerRole, Position, TacticsConfig, TeamData,
};
use rand::SeedableRng;
use rand::rngs::StdRng;

const SEEDS: u64 = 2000;

fn player(id: &str, position: Position) -> PlayerData {
    PlayerData {
        id: id.to_string(),
        name: id.to_string(),
        position,
        ovr: 70,
        condition: 100,
        fitness: 100,
        injury_risk: 1.0,
        line_familiarity: [1.0; 4],
        pace: 70,
        stamina: 70,
        strength: 70,
        agility: 70,
        passing: 70,
        shooting: 70,
        tackling: 70,
        dribbling: 70,
        defending: 70,
        positioning: 70,
        vision: 70,
        decisions: 70,
        composure: 70,
        aggression: 70,
        teamwork: 70,
        leadership: 70,
        handling: 70,
        reflexes: 70,
        aerial: 70,
        traits: vec![],
        role: PlayerRole::Standard,
    }
}

/// A 4-4-2 of equal players.
fn side(prefix: &str) -> TeamData {
    let mut players = vec![player(&format!("{prefix}_gk"), Position::Goalkeeper)];
    for number in 1..=10 {
        let position = match number {
            1..=4 => Position::Defender,
            5..=8 => Position::Midfielder,
            _ => Position::Forward,
        };
        players.push(player(&format!("{prefix}_{number}"), position));
    }
    TeamData {
        id: prefix.to_string(),
        name: prefix.to_string(),
        formation: "4-4-2".to_string(),
        play_style: PlayStyle::Balanced,
        tactics: TacticsConfig::default(),
        players,
    }
}

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
            side("h"),
            side("a"),
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
/// Then the share scored is the calibrated one (see the module comment): 28% to 32% of shots on
/// target, where the engine used to score 38%.
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
/// The calibrated 30% between equal 70-rated sides, two points either side: about five times
/// the noise of a share over some 16,000 shots on target.
const CONVERSION_LOW: f64 = 0.28;
const CONVERSION_HIGH: f64 = 0.32;
