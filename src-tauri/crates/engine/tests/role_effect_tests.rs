//! What a player role changes in a match, role by role: each test pins a role's effect as a delta
//! against the same side on the Standard role, between two equal sides, against the delta's own
//! standard error (`drafts/tactics-match/wire-unread-controls.md`, the wiring slices).

use engine::live_match::LiveMatchState;
use engine::{
    EventType, MatchConfig, MatchEvent, PlayerRole, Position, Side, TeamData, Zone,
    simulate_with_rng,
};
use rand::SeedableRng;
use rand::rngs::StdRng;

mod common;
use common::equal_side;

const SEEDS: u64 = 4000;
/// How many standard errors a role's effect must clear.
const STANDARD_ERRORS: f64 = 3.0;

#[derive(Clone, Copy)]
enum Engine {
    Live,
    Instant,
}

/// Every event of the match between `home` and an equal away side, from `seed`.
fn events(engine: Engine, home: &TeamData, seed: u64) -> Vec<MatchEvent> {
    let mut rng = StdRng::seed_from_u64(seed);
    match engine {
        Engine::Live => {
            let mut state = LiveMatchState::new(
                home.clone(),
                equal_side("a"),
                MatchConfig::default(),
                vec![],
                vec![],
                false,
            );
            let mut events = Vec::new();
            loop {
                let minute = state.step_minute(&mut rng);
                events.extend(minute.events);
                if minute.is_finished {
                    return events;
                }
            }
        }
        Engine::Instant => {
            simulate_with_rng(home, &equal_side("a"), &MatchConfig::default(), &mut rng).events
        }
    }
}

/// The share of the away side's build-ups that the home side intercepts, per match: the press at
/// work.
fn away_build_ups_intercepted(events: &[MatchEvent]) -> Option<f64> {
    let away = |event: &&MatchEvent| event.side == Side::Away;
    let intercepted = events
        .iter()
        .filter(away)
        .filter(|event| event.event_type == EventType::PassIntercepted)
        .count();
    let completed = events
        .iter()
        .filter(away)
        .filter(|event| {
            event.event_type == EventType::PassCompleted
                && event.zone == Zone::defensive_third(Side::Away)
        })
        .count();
    let attempts = intercepted + completed;
    (attempts > 0).then(|| intercepted as f64 / attempts as f64)
}

/// The mean of a per-match measure over SEEDS matches, and its standard error.
fn mean_and_error(
    engine: Engine,
    home: &TeamData,
    measure: fn(&[MatchEvent]) -> Option<f64>,
) -> (f64, f64) {
    let values: Vec<f64> = (0..SEEDS)
        .filter_map(|seed| measure(&events(engine, home, seed)))
        .collect();
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0);
    (mean, (variance / n).sqrt())
}

/// The home side with both its forwards on `role`.
fn forwards_as(role: PlayerRole) -> TeamData {
    let mut side = equal_side("h");
    for player in side
        .players
        .iter_mut()
        .filter(|p| p.position == Position::Forward)
    {
        player.role = role;
    }
    side
}

/// The press's effect: how much more of the away side's build-up the home side intercepts with
/// its forwards on `role` than on Standard, in standard errors.
fn press_effect(engine: Engine, role: PlayerRole) -> (f64, f64) {
    let (with, with_error) = mean_and_error(engine, &forwards_as(role), away_build_ups_intercepted);
    let (without, without_error) = mean_and_error(
        engine,
        &forwards_as(PlayerRole::Standard),
        away_build_ups_intercepted,
    );
    let delta = with - without;
    (
        delta,
        delta / (with_error.powi(2) + without_error.powi(2)).sqrt(),
    )
}

/// Given two equal sides, the home side's two forwards Pressing Forwards,
/// When the live engine plays them,
/// Then the home side intercepts more of the away side's build-up than with its forwards on
/// Standard: the front line presses too.
#[test]
fn pressing_forwards_win_the_ball_higher_in_a_live_match() {
    let (delta, errors) = press_effect(Engine::Live, PlayerRole::PressingForward);
    assert!(
        errors > STANDARD_ERRORS,
        "the share of build-ups intercepted moved {delta:+.4} ({errors:.1} standard errors)"
    );
}

/// Given the same sides,
/// When the instant engine (sim-bench, the sim lab) plays them,
/// Then the pressing forwards win the ball higher there too.
#[test]
fn pressing_forwards_win_the_ball_higher_in_an_instant_match() {
    let (delta, errors) = press_effect(Engine::Instant, PlayerRole::PressingForward);
    assert!(
        errors > STANDARD_ERRORS,
        "the share of build-ups intercepted moved {delta:+.4} ({errors:.1} standard errors)"
    );
}

/// Given a home side with one forward a Pressing Forward and the other, alike in every
/// attribute, on Standard,
/// When a live match is played to the end,
/// Then the pressing forward has less left: he runs more.
#[test]
fn a_pressing_forward_tires_faster_than_a_standard_one() {
    let mut home = equal_side("h");
    let forwards: Vec<String> = home
        .players
        .iter()
        .filter(|p| p.position == Position::Forward)
        .map(|p| p.id.clone())
        .collect();
    home.players
        .iter_mut()
        .find(|p| p.id == forwards[0])
        .expect("a forward")
        .role = PlayerRole::PressingForward;
    let mut state = LiveMatchState::new(
        home,
        equal_side("a"),
        MatchConfig::default(),
        vec![],
        vec![],
        false,
    );
    let mut rng = StdRng::seed_from_u64(1);
    while !state.step_minute(&mut rng).is_finished {}

    let snapshot = state.snapshot();
    let condition = |id: &str| {
        snapshot
            .home_team
            .players
            .iter()
            .find(|p| p.id == id)
            .expect("he played")
            .condition
    };
    assert!(
        condition(&forwards[0]) < condition(&forwards[1]),
        "the pressing forward ended at {} and the standard one at {}",
        condition(&forwards[0]),
        condition(&forwards[1])
    );
}
