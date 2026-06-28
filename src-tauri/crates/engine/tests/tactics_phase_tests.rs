//! Directional behaviour tests for the extended phase dials (tempo, defensive
//! shape, pressing-possession, counter-press, break speed). Each runs two
//! otherwise-identical sides differing in one dial over a band of seeds and
//! asserts the aggregate outcome moves the claimed way: possession share for the
//! "keep/win the ball" dials, shots for the "create/deny a chance" dials.
//!
//! build_up / width / def_line / marking are owned by the original engine config
//! and covered by its own tests; they are intentionally not re-tested here.

use engine::live_match::LiveMatchState;
use engine::{
    BreakSpeed, CounterPressDuration, DefensiveShape, EventType, MatchConfig, PlayStyle,
    PlayerData, PlayerRole, Position, PressingIntensity, Side, TacticsConfig, TeamData, Tempo,
};
use rand::SeedableRng;
use rand::rngs::StdRng;

const SEEDS: u64 = 200;
const SHOTS: [EventType; 4] = [
    EventType::Goal,
    EventType::ShotSaved,
    EventType::ShotOffTarget,
    EventType::ShotBlocked,
];

#[derive(Default, Clone, Copy)]
struct Totals {
    home_possession: usize,
    home_shots: usize,
    away_shots: usize,
}

fn mk(id: &str, pos: Position) -> PlayerData {
    PlayerData {
        id: id.to_string(), name: id.to_string(), position: pos,
        ovr: 70, condition: 90, fitness: 75,
        pace: 70, stamina: 70, strength: 70, agility: 70, passing: 70, shooting: 70,
        tackling: 70, dribbling: 70, defending: 70, positioning: 70, vision: 70,
        decisions: 70, composure: 70, aggression: 70, teamwork: 70, leadership: 70,
        handling: 70, reflexes: 70, aerial: 70, traits: vec![], role: PlayerRole::Standard,
    }
}

fn team(id: &str, tactics: TacticsConfig) -> TeamData {
    TeamData {
        id: id.to_string(), name: id.to_string(), formation: "4-4-2".to_string(),
        play_style: PlayStyle::Balanced, tactics,
        players: vec![
            mk(&format!("{id}_gk"), Position::Goalkeeper),
            mk(&format!("{id}_d1"), Position::Defender), mk(&format!("{id}_d2"), Position::Defender),
            mk(&format!("{id}_d3"), Position::Defender), mk(&format!("{id}_d4"), Position::Defender),
            mk(&format!("{id}_m1"), Position::Midfielder), mk(&format!("{id}_m2"), Position::Midfielder),
            mk(&format!("{id}_m3"), Position::Midfielder), mk(&format!("{id}_m4"), Position::Midfielder),
            mk(&format!("{id}_f1"), Position::Forward), mk(&format!("{id}_f2"), Position::Forward),
        ],
    }
}

fn play(home: TacticsConfig, seed: u64) -> Totals {
    let mut state = LiveMatchState::new(
        team("h", home),
        team("a", TacticsConfig::default()),
        MatchConfig::default(),
        vec![], vec![], false,
    );
    let mut rng = StdRng::seed_from_u64(seed);
    let mut t = Totals::default();
    loop {
        let r = state.step_minute(&mut rng);
        if r.possession == Side::Home {
            t.home_possession += 1;
        }
        for e in &r.events {
            if SHOTS.contains(&e.event_type) {
                match e.side {
                    Side::Home => t.home_shots += 1,
                    Side::Away => t.away_shots += 1,
                }
            }
        }
        if r.is_finished {
            break;
        }
    }
    t
}

fn band(home: TacticsConfig, metric: impl Fn(&Totals) -> usize) -> usize {
    (0..SEEDS).map(|s| metric(&play(home.clone(), s))).sum()
}

fn with(f: impl FnOnce(&mut TacticsConfig)) -> TacticsConfig {
    let mut c = TacticsConfig::default();
    f(&mut c);
    c
}

#[test]
fn neutral_is_reproducible() {
    let n = TacticsConfig::default();
    for seed in 0..15 {
        let a = play(n.clone(), seed);
        let b = play(n.clone(), seed);
        assert_eq!(
            (a.home_possession, a.home_shots, a.away_shots),
            (b.home_possession, b.home_shots, b.away_shots),
            "seed {seed}: neutral not reproducible"
        );
    }
}

// --- keep / win the ball → possession ---

#[test]
fn patient_tempo_keeps_more_possession_than_direct() {
    let patient = band(with(|c| c.tempo = Tempo::Patient), |t| t.home_possession);
    let direct = band(with(|c| c.tempo = Tempo::Direct), |t| t.home_possession);
    assert!(patient > direct, "Patient should hold the ball more than Direct: {patient} vs {direct}");
}

#[test]
fn aggressive_pressing_wins_more_possession_than_passive() {
    let agg = band(with(|c| c.pressing_intensity = PressingIntensity::Aggressive), |t| t.home_possession);
    let pas = band(with(|c| c.pressing_intensity = PressingIntensity::Passive), |t| t.home_possession);
    assert!(agg > pas, "Aggressive pressing should win the ball back more than passive: {agg} vs {pas}");
}

#[test]
fn long_counter_press_keeps_more_possession_than_none() {
    let long = band(with(|c| c.counter_press_duration = CounterPressDuration::Long), |t| t.home_possession);
    let none = band(with(|c| c.counter_press_duration = CounterPressDuration::None), |t| t.home_possession);
    assert!(long > none, "Long counter-press should regain possession more than none: {long} vs {none}");
}

// --- create / deny a chance → shots ---

#[test]
fn direct_tempo_shoots_more_than_patient() {
    let direct = band(with(|c| c.tempo = Tempo::Direct), |t| t.home_shots);
    let patient = band(with(|c| c.tempo = Tempo::Patient), |t| t.home_shots);
    assert!(direct > patient, "Direct tempo should produce more shots than Patient: {direct} vs {patient}");
}

#[test]
fn compact_shape_concedes_fewer_chances_than_stretched() {
    let compact = band(with(|c| c.defensive_shape = DefensiveShape::Compact), |t| t.away_shots);
    let stretched = band(with(|c| c.defensive_shape = DefensiveShape::Stretched), |t| t.away_shots);
    assert!(compact < stretched, "Compact should concede fewer chances than Stretched: {compact} vs {stretched}");
}

#[test]
fn fast_break_creates_more_chances_than_slow() {
    let fast = band(with(|c| c.break_speed = BreakSpeed::Fast), |t| t.home_shots);
    let slow = band(with(|c| c.break_speed = BreakSpeed::Slow), |t| t.home_shots);
    assert!(fast > slow, "Fast breaks should create more chances than slow: {fast} vs {slow}");
}
