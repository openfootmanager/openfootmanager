//! What a play style does to its own defending: the attacking-third duels the opponent wins
//! against it, and the chances it concedes, against an identical Balanced side.
//!
//! The duel is the one place the Defense entry of the play-style table applies, so it is what
//! these tests count. The opponent's `Dribble` is a duel it won and its `DribbleTackled` one it
//! lost to a tackle; both engines emit those two events from that duel only. A lost duel ends in
//! a tackle or a clearance at random, and that clearance is the same event as the one that
//! clears a won duel's cross, so the tests count won duels against tackled ones: a rate that
//! rises and falls with the duel win rate without equalling it.

use engine::live_match::LiveMatchState;
use engine::{
    EventType, MatchConfig, MatchEvent, PlayStyle, PlayerData, PlayerRole, Position, Side,
    TacticsConfig, TeamData, simulate_with_rng,
};
use rand::SeedableRng;
use rand::rngs::StdRng;

const SEEDS: u64 = 4000;
const SHOTS: [EventType; 4] = [
    EventType::Goal,
    EventType::ShotSaved,
    EventType::ShotOffTarget,
    EventType::ShotBlocked,
];

fn player(id: &str, position: Position) -> PlayerData {
    PlayerData {
        id: id.to_string(),
        name: id.to_string(),
        position,
        ovr: 70,
        condition: 100,
        fitness: 100,
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

fn side(id: &str, play_style: PlayStyle) -> TeamData {
    let mut players = vec![player(&format!("{id}_gk"), Position::Goalkeeper)];
    for (position, count) in [
        (Position::Defender, 4),
        (Position::Midfielder, 4),
        (Position::Forward, 2),
    ] {
        for index in 0..count {
            players.push(player(&format!("{id}_{position:?}{index}"), position));
        }
    }
    TeamData {
        id: id.to_string(),
        name: id.to_string(),
        formation: "4-4-2".to_string(),
        play_style,
        tactics: TacticsConfig::default(),
        players,
    }
}

#[derive(Clone, Copy)]
enum Engine {
    Live,
    Instant,
}

/// What the Balanced away side did against the home side, summed over every seed.
#[derive(Default)]
struct Against {
    duels_won: usize,
    duels_tackled: usize,
    shots: usize,
    goals: usize,
}

impl Against {
    fn count(&mut self, events: &[MatchEvent]) {
        for event in events.iter().filter(|event| event.side == Side::Away) {
            self.duels_won += usize::from(event.event_type == EventType::Dribble);
            self.duels_tackled += usize::from(event.event_type == EventType::DribbleTackled);
            self.shots += usize::from(SHOTS.contains(&event.event_type));
            self.goals += usize::from(event.event_type == EventType::Goal);
        }
    }

    /// The opponent's won attacking-third duels against those it won or lost to a tackle:
    /// won / (won + tackled). Higher when its duels go better, though not their win share.
    fn won_against_tackled(&self) -> f64 {
        self.duels_won as f64 / (self.duels_won + self.duels_tackled) as f64
    }
}

/// The home side plays `style`; the away side is Balanced with the same players.
fn against(style: PlayStyle, engine: Engine) -> Against {
    let mut against = Against::default();
    for seed in 0..SEEDS {
        let home = side("h", style);
        let away = side("a", PlayStyle::Balanced);
        let mut rng = StdRng::seed_from_u64(seed);
        match engine {
            Engine::Live => {
                let mut state =
                    LiveMatchState::new(home, away, MatchConfig::default(), vec![], vec![], false);
                loop {
                    let minute = state.step_minute(&mut rng);
                    against.count(&minute.events);
                    if minute.is_finished {
                        break;
                    }
                }
            }
            Engine::Instant => {
                let report = simulate_with_rng(&home, &away, &MatchConfig::default(), &mut rng);
                against.count(&report.events);
            }
        }
    }
    against
}

/// How much the opponent's won-against-tackled rate moves when the home side plays `style`
/// instead of Balanced (positive: the style defends worse).
fn duel_rate_shift(style: PlayStyle, engine: Engine) -> f64 {
    against(style, engine).won_against_tackled()
        - against(PlayStyle::Balanced, engine).won_against_tackled()
}

// Between identical sides at the default home advantage, the table entries should move the
// opponent's won-against-tackled rate by about -0.024 (Defensive), +0.018 (Attacking) and +0.012
// (HighPress). Before the Defense entry applied, every shift was noise around zero (at most
// 0.002). Defensive's margin is three quarters of its effect, so a ×1.12 that is added to the
// role factor instead of multiplying it (a shift of about -0.014) fails too. The two costs keep
// half, since they are smaller against the same seed-to-seed noise.
const DEFENSIVE_MARGIN: f64 = -0.018;

/// Given two identical sides, one Defensive and one Balanced,
/// When the Balanced side attacks the Defensive one in the live engine,
/// Then it wins clearly fewer attacking-third duels than it does against a Balanced side.
#[test]
fn a_defensive_side_wins_more_attacking_third_duels_in_the_live_engine() {
    let shift = duel_rate_shift(PlayStyle::Defensive, Engine::Live);
    assert!(
        shift < DEFENSIVE_MARGIN,
        "opponent won-against-tackled rate moved {shift:+.4} against Defensive"
    );
}

/// Given the same sides, When the Balanced side attacks the Defensive one in the instant engine
/// (sim-bench and the sim lab), Then it also wins clearly fewer attacking-third duels.
#[test]
fn a_defensive_side_wins_more_attacking_third_duels_in_the_instant_engine() {
    let shift = duel_rate_shift(PlayStyle::Defensive, Engine::Instant);
    assert!(
        shift < DEFENSIVE_MARGIN,
        "opponent won-against-tackled rate moved {shift:+.4} against Defensive"
    );
}

/// Given an Attacking side, whose attacking bonus is paid for with a weaker defence,
/// When the Balanced side attacks it in the live engine,
/// Then it wins more attacking-third duels than it does against a Balanced side.
#[test]
fn an_attacking_side_pays_for_its_attack_in_attacking_third_duels() {
    let shift = duel_rate_shift(PlayStyle::Attacking, Engine::Live);
    assert!(
        shift > 0.008,
        "opponent won-against-tackled rate moved {shift:+.4} against Attacking"
    );
}

/// Given a HighPress side, whose press is paid for with a weaker defence behind it,
/// When the Balanced side attacks it in the live engine,
/// Then it wins more attacking-third duels than it does against a Balanced side.
#[test]
fn a_high_pressing_side_pays_for_its_press_in_attacking_third_duels() {
    let shift = duel_rate_shift(PlayStyle::HighPress, Engine::Live);
    assert!(
        shift > 0.006,
        "opponent won-against-tackled rate moved {shift:+.4} against HighPress"
    );
}

#[test]
#[ignore = "prints the measurement"]
fn print_what_each_style_concedes() {
    for engine in [Engine::Live, Engine::Instant] {
        for style in [
            PlayStyle::Balanced,
            PlayStyle::Attacking,
            PlayStyle::Defensive,
            PlayStyle::Possession,
            PlayStyle::Counter,
            PlayStyle::HighPress,
        ] {
            let against = against(style, engine);
            let engine_name = match engine {
                Engine::Live => "live",
                Engine::Instant => "instant",
            };
            println!(
                "CONCEDED {engine_name} {style:?}: duels won {} tackled {} rate {:.4} shots {} goals {}",
                against.duels_won,
                against.duels_tackled,
                against.won_against_tackled(),
                against.shots,
                against.goals
            );
        }
    }
}
