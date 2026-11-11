//! A player's mood (`PlayerData::morale_factor`) in both engines. Each half of the trade is pinned
//! as a delta against the neutral mood between equal sides, with the band set from the statistic's
//! own spread (the tempo test's method): a lifted side out-shoots the neutral one, a lowered side
//! is out-shot, each by more than STANDARD_ERRORS standard errors of the difference. Totals are
//! not pinned.

mod common;

use common::equal_side;
use engine::live_match::LiveMatchState;
use engine::{EventType, MatchConfig, Side, TeamData, simulate_with_rng};
use rand::SeedableRng;
use rand::rngs::StdRng;

/// Matches per arm. The effect of a few percent of every duel is small against a match's own
/// spread, so the arms are long, as the tempo test's are.
const MATCHES: u64 = 4_000;

/// How many standard errors of the difference a lift or a drop must clear.
const STANDARD_ERRORS: f64 = 4.0;

const SHOTS: [EventType; 4] = [
    EventType::Goal,
    EventType::ShotSaved,
    EventType::ShotOffTarget,
    EventType::ShotBlocked,
];

#[derive(Clone, Copy, Debug)]
enum Engine {
    Instant,
    Live,
}

fn home_with_mood(morale_factor: f64) -> TeamData {
    let mut home = equal_side("h");
    for player in &mut home.players {
        player.morale_factor = morale_factor;
    }
    home
}

/// Home's shots minus away's in one match between `home` and an equal neutral side.
fn shot_difference(engine: Engine, home: &TeamData, seed: u64) -> f64 {
    let away = equal_side("a");
    let mut rng = StdRng::seed_from_u64(seed);
    match engine {
        Engine::Instant => {
            let report = simulate_with_rng(home, &away, &MatchConfig::default(), &mut rng);
            f64::from(report.home_stats.shots) - f64::from(report.away_stats.shots)
        }
        Engine::Live => {
            let mut state = LiveMatchState::new(
                home.clone(),
                away,
                MatchConfig::default(),
                vec![],
                vec![],
                false,
            );
            let mut difference = 0.0;
            loop {
                let minute = state.step_minute(&mut rng);
                for event in &minute.events {
                    if SHOTS.contains(&event.event_type) {
                        match event.side {
                            Side::Home => difference += 1.0,
                            Side::Away => difference -= 1.0,
                        }
                    }
                }
                if minute.is_finished {
                    return difference;
                }
            }
        }
    }
}

/// The mean and the standard error of the mean of the shot difference over MATCHES matches.
fn shot_difference_over_matches(engine: Engine, morale_factor: f64) -> (f64, f64) {
    let home = home_with_mood(morale_factor);
    let differences: Vec<f64> = (0..MATCHES)
        .map(|seed| shot_difference(engine, &home, seed))
        .collect();
    let n = differences.len() as f64;
    let mean = differences.iter().sum::<f64>() / n;
    let variance = differences.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / (n - 1.0);
    (mean, (variance / n).sqrt())
}

/// The change in the shot difference from the neutral mood to `morale_factor`, and its standard error.
fn effect_against_neutral(engine: Engine, morale_factor: f64) -> (f64, f64) {
    let (neutral, neutral_se) = shot_difference_over_matches(engine, 1.0);
    let (moved, moved_se) = shot_difference_over_matches(engine, morale_factor);
    (
        moved - neutral,
        (neutral_se.powi(2) + moved_se.powi(2)).sqrt(),
    )
}

fn assert_lift(engine: Engine) {
    let (effect, standard_error) = effect_against_neutral(engine, 1.03);
    eprintln!(
        "MORALE_EFFECT {engine:?} +3%: shot difference {effect:+.4}, standard error {standard_error:.4}"
    );
    assert!(
        effect > STANDARD_ERRORS * standard_error,
        "{engine:?}: a side at +3% should out-shoot the neutral one: the shot difference moved \
         {effect:+.4} with a standard error of {standard_error:.4}"
    );
}

fn assert_drop(engine: Engine) {
    let (effect, standard_error) = effect_against_neutral(engine, 0.97);
    eprintln!(
        "MORALE_EFFECT {engine:?} -3%: shot difference {effect:+.4}, standard error {standard_error:.4}"
    );
    assert!(
        effect < -STANDARD_ERRORS * standard_error,
        "{engine:?}: a side at -3% should be out-shot by the neutral one: the shot difference \
         moved {effect:+.4} with a standard error of {standard_error:.4}"
    );
}

/// Given a home side whose players are all at +3% and a neutral side, equal otherwise,
/// When each engine plays MATCHES matches against the same seeds as a neutral home side,
/// Then the lifted side's shot difference rises by more than STANDARD_ERRORS standard errors.
#[test]
fn a_lifted_mood_helps_in_the_instant_engine() {
    assert_lift(Engine::Instant);
}

#[test]
fn a_lowered_mood_costs_in_the_instant_engine() {
    assert_drop(Engine::Instant);
}

#[test]
fn a_lifted_mood_helps_in_the_live_engine() {
    assert_lift(Engine::Live);
}

#[test]
fn a_lowered_mood_costs_in_the_live_engine() {
    assert_drop(Engine::Live);
}

/// Given a mood beyond what the engine allows (a caller's mistake)
/// When a match is played
/// Then it is the match of the widest mood the engine allows, not of the mistake
#[test]
fn a_mood_beyond_the_range_plays_as_the_edge_of_it() {
    for engine in [Engine::Instant, Engine::Live] {
        let wild = home_with_mood(9.0);
        let edge = home_with_mood(1.1);
        for seed in 0..20 {
            assert_eq!(
                shot_difference(engine, &wild, seed),
                shot_difference(engine, &edge, seed),
                "{engine:?}, seed {seed}"
            );
        }
    }
}

/// Given a mood that is not a number
/// When a match is played
/// Then it is the neutral match: a bad value cannot zero a side's skills
#[test]
fn a_mood_that_is_not_a_number_plays_as_neutral() {
    for engine in [Engine::Instant, Engine::Live] {
        let broken = home_with_mood(f64::NAN);
        let neutral = home_with_mood(1.0);
        for seed in 0..20 {
            assert_eq!(
                shot_difference(engine, &broken, seed),
                shot_difference(engine, &neutral, seed),
                "{engine:?}, seed {seed}"
            );
        }
    }
}

/// Given a team saved before the field existed (no `morale_factor` in its JSON)
/// When it is read back
/// Then every player's mood is neutral, not zero
#[test]
fn a_player_saved_without_a_mood_reads_back_neutral() {
    let mut value = serde_json::to_value(equal_side("h")).expect("serializes");
    for player in value["players"].as_array_mut().expect("players") {
        player
            .as_object_mut()
            .expect("object")
            .remove("morale_factor");
    }

    let team: TeamData = serde_json::from_value(value).expect("reads without the field");

    assert!(team.players.iter().all(|p| p.morale_factor == 1.0));
}
