//! A fouled player's injury chance is the match's injury probability times the player's own
//! injury risk, in both engines.

use engine::live_match::LiveMatchState;
use engine::{
    EventType, MatchConfig, MatchEvent, PlayStyle, PlayerData, PlayerRole, Position, Side,
    TacticsConfig, TeamData, simulate_with_rng,
};
use rand::SeedableRng;
use rand::rngs::StdRng;

const SEEDS: u64 = 400;

fn player(id: &str, position: Position, injury_risk: f64) -> PlayerData {
    PlayerData {
        id: id.to_string(),
        name: id.to_string(),
        position,
        ovr: 70,
        condition: 100,
        fitness: 100,
        injury_risk,
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

/// A 4-4-2 whose every player carries `injury_risk`.
fn side(id: &str, injury_risk: f64) -> TeamData {
    let mut players = vec![player(
        &format!("{id}_gk"),
        Position::Goalkeeper,
        injury_risk,
    )];
    for (position, count) in [
        (Position::Defender, 4),
        (Position::Midfielder, 4),
        (Position::Forward, 2),
    ] {
        for index in 0..count {
            players.push(player(
                &format!("{id}_{position:?}{index}"),
                position,
                injury_risk,
            ));
        }
    }
    TeamData {
        id: id.to_string(),
        name: id.to_string(),
        formation: "4-4-2".to_string(),
        play_style: PlayStyle::Balanced,
        tactics: TacticsConfig::default(),
        players,
    }
}

/// A higher injury probability than the default, so the counts carry signal at few seeds.
fn config() -> MatchConfig {
    MatchConfig {
        injury_probability: 0.1,
        ..MatchConfig::default()
    }
}

#[derive(Clone, Copy)]
enum Engine {
    Live,
    Instant,
}

/// Injuries per foul suffered by the home side (risk 3.0) over the away side (risk 1.0).
fn injury_rate_ratio(engine: Engine) -> f64 {
    let (mut fouls, mut injuries) = ([0u32; 2], [0u32; 2]);
    let index = |side: Side| match side {
        Side::Home => 0,
        Side::Away => 1,
    };
    let mut count = |events: &[MatchEvent]| {
        for event in events {
            match event.event_type {
                // A foul is recorded against the side that commits it.
                EventType::Foul => fouls[1 - index(event.side)] += 1,
                // An injury is recorded for the side whose player is hurt.
                EventType::Injury => injuries[index(event.side)] += 1,
                _ => {}
            }
        }
    };
    for seed in 0..SEEDS {
        let (home, away) = (side("h", 3.0), side("a", 1.0));
        let mut rng = StdRng::seed_from_u64(seed);
        match engine {
            Engine::Live => {
                let mut state = LiveMatchState::new(home, away, config(), vec![], vec![], false);
                loop {
                    let minute = state.step_minute(&mut rng);
                    count(&minute.events);
                    if minute.is_finished {
                        break;
                    }
                }
            }
            Engine::Instant => count(&simulate_with_rng(&home, &away, &config(), &mut rng).events),
        }
    }
    let rate = |side: usize| f64::from(injuries[side]) / f64::from(fouls[side]);
    rate(0) / rate(1)
}

/// Given a side whose players carry three times the injury risk of an otherwise identical side,
/// When they play in the live engine,
/// Then each foul on them injures about three times as often.
#[test]
fn a_player_with_three_times_the_risk_is_injured_about_three_times_as_often_live() {
    let ratio = injury_rate_ratio(Engine::Live);
    assert!((2.3..=3.9).contains(&ratio), "ratio {ratio:.2}");
}

/// Given the same sides in the instant engine (sim-bench and the sim lab),
/// When they play,
/// Then each foul on the riskier side injures about three times as often there too.
#[test]
fn a_player_with_three_times_the_risk_is_injured_about_three_times_as_often_instant() {
    let ratio = injury_rate_ratio(Engine::Instant);
    assert!((2.3..=3.9).contains(&ratio), "ratio {ratio:.2}");
}

/// Given a player saved before injury risk existed, with no `injury_risk` field,
/// When the player is read back,
/// Then the player carries the match's own injury probability: a risk of 1.0.
#[test]
fn a_player_without_an_injury_risk_reads_as_the_matchs_own_probability() {
    let mut json = serde_json::to_value(player("p", Position::Forward, 2.5)).expect("serialises");
    json.as_object_mut()
        .expect("a player is an object")
        .remove("injury_risk");
    let read: PlayerData = serde_json::from_value(json).expect("an older player still reads");
    assert_eq!(read.injury_risk, 1.0);
}
