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
    simulate_with_rng,
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
        id: id.to_string(),
        name: id.to_string(),
        position: pos,
        ovr: 70,
        condition: 90,
        fitness: 75,
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

fn team(id: &str, tactics: TacticsConfig) -> TeamData {
    TeamData {
        id: id.to_string(),
        name: id.to_string(),
        formation: "4-4-2".to_string(),
        play_style: PlayStyle::Balanced,
        tactics,
        players: vec![
            mk(&format!("{id}_gk"), Position::Goalkeeper),
            mk(&format!("{id}_d1"), Position::Defender),
            mk(&format!("{id}_d2"), Position::Defender),
            mk(&format!("{id}_d3"), Position::Defender),
            mk(&format!("{id}_d4"), Position::Defender),
            mk(&format!("{id}_m1"), Position::Midfielder),
            mk(&format!("{id}_m2"), Position::Midfielder),
            mk(&format!("{id}_m3"), Position::Midfielder),
            mk(&format!("{id}_m4"), Position::Midfielder),
            mk(&format!("{id}_f1"), Position::Forward),
            mk(&format!("{id}_f2"), Position::Forward),
        ],
    }
}

fn play(home: TacticsConfig, seed: u64) -> Totals {
    let mut state = LiveMatchState::new(
        team("h", home),
        team("a", TacticsConfig::default()),
        MatchConfig::default(),
        vec![],
        vec![],
        false,
    );
    let mut rng = StdRng::seed_from_u64(seed);
    let mut t = Totals::default();
    loop {
        let r = state.step_minute(&mut rng);
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
    // Use the engine's own possession stat (accumulated each minute) rather than
    // the post-turnover `MinuteResult.possession`, so the metric matches what the
    // dial logic actually feeds.
    t.home_possession = state.snapshot().home_possession_pct.round() as usize;
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

/// Tempo moves possession by a third of a point, against a spread of about 5.4 points from match
/// to match, so a fixed lead over a few hundred matches is noise: at 1,000 matches the lead was
/// 0.7 standard errors before #1132's card rates and 1.6 after, and which arm led depended on the
/// seeds. Over TEMPO_SEEDS matches an arm the lead is measured against its own standard error.
const TEMPO_SEEDS: u64 = 10_000;

/// How many standard errors Patient's lead must clear: about 95% one-sided, and well inside the
/// leads measured (3.2 standard errors at this size before #1132's card rates, 4.9 after).
const TEMPO_STANDARD_ERRORS: f64 = 2.0;

/// The mean and the standard error of the mean of `home`'s possession share per match.
fn possession_over_tempo_seeds(home: &TacticsConfig) -> (f64, f64) {
    let shares: Vec<f64> = (0..TEMPO_SEEDS)
        .map(|seed| play(home.clone(), seed).home_possession as f64)
        .collect();
    let n = shares.len() as f64;
    let mean = shares.iter().sum::<f64>() / n;
    let variance = shares
        .iter()
        .map(|share| (share - mean).powi(2))
        .sum::<f64>()
        / (n - 1.0);
    (mean, (variance / n).sqrt())
}

/// Given a home side on Patient tempo and one on Direct, each against the same neutral side,
/// When each plays TEMPO_SEEDS matches,
/// Then Patient keeps more of the ball by more than TEMPO_STANDARD_ERRORS standard errors of the
/// difference: the dial's effect is small, but real.
#[test]
fn patient_tempo_keeps_more_possession_than_direct() {
    let (patient, patient_se) = possession_over_tempo_seeds(&with(|c| c.tempo = Tempo::Patient));
    let (direct, direct_se) = possession_over_tempo_seeds(&with(|c| c.tempo = Tempo::Direct));
    let lead = patient - direct;
    let standard_error = (patient_se.powi(2) + direct_se.powi(2)).sqrt();
    assert!(
        lead > TEMPO_STANDARD_ERRORS * standard_error,
        "Patient should hold the ball more than Direct: {patient:.3}% against {direct:.3}%, a lead \
         of {lead:.3} with a standard error of {standard_error:.3}"
    );
}

#[test]
fn aggressive_pressing_wins_more_possession_than_passive() {
    let agg = band(
        with(|c| c.pressing_intensity = PressingIntensity::Aggressive),
        |t| t.home_possession,
    );
    let pas = band(
        with(|c| c.pressing_intensity = PressingIntensity::Passive),
        |t| t.home_possession,
    );
    assert!(
        agg > pas,
        "Aggressive pressing should win the ball back more than passive: {agg} vs {pas}"
    );
}

#[test]
fn long_counter_press_keeps_more_possession_than_none() {
    let long = band(
        with(|c| c.counter_press_duration = CounterPressDuration::Long),
        |t| t.home_possession,
    );
    let none = band(
        with(|c| c.counter_press_duration = CounterPressDuration::None),
        |t| t.home_possession,
    );
    assert!(
        long > none,
        "Long counter-press should regain possession more than none: {long} vs {none}"
    );
}

// --- create / deny a chance → shots ---

#[test]
fn direct_tempo_shoots_more_than_patient() {
    let direct = band(with(|c| c.tempo = Tempo::Direct), |t| t.home_shots);
    let patient = band(with(|c| c.tempo = Tempo::Patient), |t| t.home_shots);
    assert!(
        direct > patient,
        "Direct tempo should produce more shots than Patient: {direct} vs {patient}"
    );
}

#[test]
fn compact_shape_concedes_fewer_chances_than_stretched() {
    let compact = band(with(|c| c.defensive_shape = DefensiveShape::Compact), |t| {
        t.away_shots
    });
    let stretched = band(
        with(|c| c.defensive_shape = DefensiveShape::Stretched),
        |t| t.away_shots,
    );
    assert!(
        compact < stretched,
        "Compact should concede fewer chances than Stretched: {compact} vs {stretched}"
    );
}

#[test]
fn fast_break_creates_more_chances_than_slow() {
    let fast = band(with(|c| c.break_speed = BreakSpeed::Fast), |t| t.home_shots);
    let slow = band(with(|c| c.break_speed = BreakSpeed::Slow), |t| t.home_shots);
    assert!(
        fast > slow,
        "Fast breaks should create more chances than slow: {fast} vs {slow}"
    );
}

// ---------------------------------------------------------------------------
// Batch (instant) engine — the same dials must move the same way through
// simulate_with_rng, which drives AI matchdays. The hook sites are separate
// from the live engine's, so they get their own coverage.
// ---------------------------------------------------------------------------

fn play_batch(home: TacticsConfig, seed: u64) -> Totals {
    let h = team("h", home);
    let a = team("a", TacticsConfig::default());
    let mut rng = StdRng::seed_from_u64(seed);
    let r = simulate_with_rng(&h, &a, &MatchConfig::default(), &mut rng);
    Totals {
        home_possession: r.home_stats.possession_ticks as usize,
        home_shots: r.home_stats.shots as usize,
        away_shots: r.away_stats.shots as usize,
    }
}

fn band_batch(home: TacticsConfig, metric: impl Fn(&Totals) -> usize) -> usize {
    (0..SEEDS)
        .map(|s| metric(&play_batch(home.clone(), s)))
        .sum()
}

#[test]
fn batch_neutral_is_reproducible() {
    let n = TacticsConfig::default();
    for seed in 0..15 {
        let a = play_batch(n.clone(), seed);
        let b = play_batch(n.clone(), seed);
        assert_eq!(
            (a.home_possession, a.home_shots, a.away_shots),
            (b.home_possession, b.home_shots, b.away_shots),
            "batch seed {seed}: neutral not reproducible"
        );
    }
}

#[test]
fn batch_aggressive_pressing_wins_more_possession_than_passive() {
    let agg = band_batch(
        with(|c| c.pressing_intensity = PressingIntensity::Aggressive),
        |t| t.home_possession,
    );
    let pas = band_batch(
        with(|c| c.pressing_intensity = PressingIntensity::Passive),
        |t| t.home_possession,
    );
    assert!(
        agg > pas,
        "batch: aggressive pressing should win more possession: {agg} vs {pas}"
    );
}

#[test]
fn batch_long_counter_press_keeps_more_possession_than_none() {
    let long = band_batch(
        with(|c| c.counter_press_duration = CounterPressDuration::Long),
        |t| t.home_possession,
    );
    let none = band_batch(
        with(|c| c.counter_press_duration = CounterPressDuration::None),
        |t| t.home_possession,
    );
    assert!(
        long > none,
        "batch: long counter-press should regain more possession: {long} vs {none}"
    );
}

#[test]
fn batch_direct_tempo_shoots_more_than_patient() {
    let direct = band_batch(with(|c| c.tempo = Tempo::Direct), |t| t.home_shots);
    let patient = band_batch(with(|c| c.tempo = Tempo::Patient), |t| t.home_shots);
    assert!(
        direct > patient,
        "batch: Direct should shoot more than Patient: {direct} vs {patient}"
    );
}

#[test]
fn batch_compact_shape_concedes_fewer_chances_than_stretched() {
    let compact = band_batch(with(|c| c.defensive_shape = DefensiveShape::Compact), |t| {
        t.away_shots
    });
    let stretched = band_batch(
        with(|c| c.defensive_shape = DefensiveShape::Stretched),
        |t| t.away_shots,
    );
    assert!(
        compact < stretched,
        "batch: Compact should concede fewer chances: {compact} vs {stretched}"
    );
}

#[test]
fn batch_fast_break_creates_more_chances_than_slow() {
    let fast = band_batch(with(|c| c.break_speed = BreakSpeed::Fast), |t| t.home_shots);
    let slow = band_batch(with(|c| c.break_speed = BreakSpeed::Slow), |t| t.home_shots);
    assert!(
        fast > slow,
        "batch: Fast breaks should create more chances: {fast} vs {slow}"
    );
}

// ---------------------------------------------------------------------------
// Pressing stamina cost (live engine only — it tracks in-match condition).
// ---------------------------------------------------------------------------

/// Total end-of-match condition for (home, away) given each side's tactics.
fn final_conditions(home: TacticsConfig, away: TacticsConfig, seed: u64) -> (i64, i64) {
    let mut s = LiveMatchState::new(
        team("h", home),
        team("a", away),
        MatchConfig::default(),
        vec![],
        vec![],
        false,
    );
    let mut rng = StdRng::seed_from_u64(seed);
    loop {
        if s.step_minute(&mut rng).is_finished {
            break;
        }
    }
    let snap = s.snapshot();
    let sum = |players: &[PlayerData]| players.iter().map(|p| p.condition as i64).sum::<i64>();
    (sum(&snap.home_team.players), sum(&snap.away_team.players))
}

#[test]
fn aggressive_pressing_tires_the_team_more_than_passive() {
    // Accumulate by tactic, not by side: each seed is played in both
    // orientations so the home-advantage bias cancels and only pressing remains.
    let aggressive = with(|c| c.pressing_intensity = PressingIntensity::Aggressive);
    let passive = with(|c| c.pressing_intensity = PressingIntensity::Passive);
    let (mut aggressive_cond, mut passive_cond) = (0i64, 0i64);
    for seed in 0..40u64 {
        let (h, a) = final_conditions(aggressive.clone(), passive.clone(), seed);
        aggressive_cond += h;
        passive_cond += a;
        let (h, a) = final_conditions(passive.clone(), aggressive.clone(), seed);
        passive_cond += h;
        aggressive_cond += a;
    }
    assert!(
        aggressive_cond < passive_cond,
        "aggressive pressing should leave the team more tired: aggressive {aggressive_cond} vs passive {passive_cond}"
    );
}

/// Given a plan with every dial off its default,
/// When its dials are listed and each is turned on a default plan,
/// Then the default plan becomes the first: `dials` names all nine, each as the plan has it.
#[test]
fn a_plans_dials_rebuild_the_plan() {
    let plan = engine::TacticsConfig {
        pressing_intensity: engine::PressingIntensity::Aggressive,
        defensive_line: engine::DefensiveLine::High,
        width: engine::TacticsPitchWidth::Wide,
        build_up_style: engine::TacticsBuildUpStyle::Long,
        marking_style: engine::MarkingStyle::ManToMan,
        tempo: engine::Tempo::Patient,
        defensive_shape: engine::DefensiveShape::Compact,
        counter_press_duration: engine::CounterPressDuration::Long,
        break_speed: engine::BreakSpeed::Fast,
    };
    let mut rebuilt = engine::TacticsConfig::default();

    for dial in plan.dials() {
        dial.set_on(&mut rebuilt);
    }

    assert_eq!(rebuilt, plan);
}
