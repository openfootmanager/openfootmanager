//! How many cards a match shows, against what real football shows. CIES Football Observatory
//! Monthly Report n°57, "Global study of penalty cards in professional football" (Poli, Ravenel
//! and Besson, September 2020; 101,491 matches in 87 top divisions, 2015/16 to 2019/20), gives
//! per match, in the Big Five: England 3.39 yellows and 0.12 reds, Germany 3.76 and 0.15, France
//! 3.78 and 0.26, Italy 4.76 and 0.27, Spain 5.27 and 0.23. Cited, not derived.
//!
//! The engine is calibrated so a season of the shipped world shows about the Big Five's mean
//! (4.0 yellows and 0.21 reds a match against 4.2 and 0.21). Between two equal sides of
//! 70-rated, 70-aggression players the rates differ from a world of mixed temperaments, so the
//! bands below are this fixture's calibrated rates with room for the counts' noise.

use engine::live_match::LiveMatchState;
use engine::{
    EventType, MatchConfig, MatchEvent, PlayStyle, PlayerData, PlayerRole, Position, TacticsConfig,
    TeamData, simulate_with_rng,
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

/// Cards over every seed. A second yellow is a booking and a sending-off, as the match report
/// counts it.
#[derive(Default)]
struct Cards {
    yellows: u32,
    straight_reds: u32,
    second_yellows: u32,
}

impl Cards {
    fn count(&mut self, event: &MatchEvent) {
        match event.event_type {
            EventType::YellowCard => self.yellows += 1,
            EventType::RedCard => self.straight_reds += 1,
            EventType::SecondYellow => self.second_yellows += 1,
            _ => {}
        }
    }

    fn yellows_per_match(&self) -> f64 {
        f64::from(self.yellows + self.second_yellows) / SEEDS as f64
    }

    fn reds_per_match(&self) -> f64 {
        f64::from(self.straight_reds + self.second_yellows) / SEEDS as f64
    }
}

/// The cards of SEEDS matches between equal sides, played once and shared by both tests.
fn cards_between_equal_sides() -> &'static Cards {
    static CARDS: std::sync::OnceLock<Cards> = std::sync::OnceLock::new();
    CARDS.get_or_init(play_equal_sides)
}

fn play_equal_sides() -> Cards {
    let mut cards = Cards::default();
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
            minute.events.iter().for_each(|event| cards.count(event));
            if minute.is_finished {
                break;
            }
        }
    }
    cards
}

/// Given two equal sides of 70-rated, 70-aggression players, over many seeds,
/// When their bookings are counted,
/// Then a match shows about as many yellow cards as the Big Five's (3.39 to 5.27, CIES n°57),
/// at this fixture's calibrated rate within the counts' noise.
#[test]
fn a_match_shows_as_many_yellow_cards_as_real_football() {
    let yellows = cards_between_equal_sides().yellows_per_match();
    assert!(
        (YELLOWS_LOW..=YELLOWS_HIGH).contains(&yellows),
        "{yellows:.2} yellow cards a match"
    );
}

/// Given two equal sides of 70-rated, 70-aggression players, over many seeds,
/// When their sendings-off are counted, straight reds and second yellows alike,
/// Then a match shows about as many as the Big Five's (0.12 to 0.27, CIES n°57), at this
/// fixture's calibrated rate within the counts' noise.
#[test]
fn a_match_shows_as_many_red_cards_as_real_football() {
    let reds = cards_between_equal_sides().reds_per_match();
    assert!(
        (REDS_LOW..=REDS_HIGH).contains(&reds),
        "{reds:.3} red cards a match"
    );
}

/// The cards of SEEDS instant-engine matches between the same equal sides, played once and
/// shared. The instant engine (sim-bench, the sim lab) carries its own copy of the card rule.
fn instant_cards_between_equal_sides() -> &'static Cards {
    static CARDS: std::sync::OnceLock<Cards> = std::sync::OnceLock::new();
    CARDS.get_or_init(|| {
        let mut cards = Cards::default();
        for seed in 0..SEEDS {
            let mut rng = StdRng::seed_from_u64(seed);
            let report =
                simulate_with_rng(&side("h"), &side("a"), &MatchConfig::default(), &mut rng);
            report.events.iter().for_each(|event| cards.count(event));
        }
        cards
    })
}

/// Given two equal sides of 70-rated, 70-aggression players, over many seeds,
/// When the instant engine plays them,
/// Then it books and sends off at its own calibrated rate, inside the Big Five's range: its
/// copy of the card rule moves with the live engine's.
#[test]
fn the_instant_engine_shows_as_many_cards_as_real_football() {
    let cards = instant_cards_between_equal_sides();
    let (yellows, reds) = (cards.yellows_per_match(), cards.reds_per_match());
    assert!(
        (INSTANT_YELLOWS_LOW..=INSTANT_YELLOWS_HIGH).contains(&yellows)
            && (INSTANT_REDS_LOW..=INSTANT_REDS_HIGH).contains(&reds),
        "the instant engine shows {yellows:.2} yellow and {reds:.3} red cards a match"
    );
}

/// The instant engine's calibrated rates between these sides, with the same room as the live
/// engine's.
const INSTANT_YELLOWS_LOW: f64 = 4.35;
const INSTANT_YELLOWS_HIGH: f64 = 4.85;
const INSTANT_REDS_LOW: f64 = 0.20;
const INSTANT_REDS_HIGH: f64 = 0.31;

/// The calibrated 4.54 yellows a match between these sides, a quarter of a card either side:
/// about five times the noise of a mean over SEEDS matches.
const YELLOWS_LOW: f64 = 4.30;
const YELLOWS_HIGH: f64 = 4.80;
/// The calibrated 0.252 reds a match between these sides, 0.05 either side: about five times the
/// noise of a mean over SEEDS matches.
const REDS_LOW: f64 = 0.20;
const REDS_HIGH: f64 = 0.30;
