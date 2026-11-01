//! A side that is a man down plays worse, as the literature on red cards measures it.
//!
//! Ridder, Cramer and Hopstaken (1994, 340 Dutch league matches): after a red card the side left
//! with eleven scores 88% more often (×1.88) and the side down to ten as often as before.
//! Červený, van Ours and van Tuijl (2018, World Cups 1998–2014, Table 6): the opponent's scoring
//! rate rises about 120% (×2.2) and the sanctioned side's falls 47% (×0.53).

use engine::live_match::{LiveMatchState, MatchCommand};
use engine::types::Zone;
use engine::{
    EventType, MatchConfig, MatchEvent, PlayStyle, PlayerData, PlayerRole, Position, Side,
    TacticsConfig, TeamData,
};
use rand::SeedableRng;
use rand::rngs::StdRng;

const SEEDS: u64 = 2000;
const RED_AT: u8 = 30;

fn player(id: &str, position: Position) -> PlayerData {
    PlayerData {
        id: id.to_string(),
        name: id.to_string(),
        position,
        ovr: 70,
        condition: 100,
        fitness: 100,
        injury_risk: 1.0,
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

/// A 4-4-2 whose outfielders are `{prefix}_1` .. `{prefix}_10`.
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

/// Any home outfielder, a different one from seed to seed.
fn an_outfielder(seed: u64) -> String {
    format!("h_{}", 1 + seed % 10)
}

/// One of the home side's four midfielders, a different one from seed to seed.
fn a_midfielder(seed: u64) -> String {
    format!("h_{}", 5 + seed % 4)
}

/// Plays every seed and hands each event after minute RED_AT to `watch`. With `send_off`, the
/// home player it names for the seed is sent off at RED_AT; without, the same seeds play on with
/// the engine's own cards only. With `reshape_to`, the home side changes to that shape a minute
/// after the red.
fn watch_after_minute_30(
    send_off: Option<fn(u64) -> String>,
    reshape_to: Option<&str>,
    mut watch: impl FnMut(&MatchEvent),
) {
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
        let mut past_the_red = false;
        let mut reshaped = false;
        loop {
            if !past_the_red && state.minute() >= RED_AT {
                past_the_red = true;
                if let Some(player_for) = send_off {
                    state.test_send_off(&player_for(seed));
                }
            }
            if let Some(formation) = reshape_to
                && !reshaped
                && state.minute() > RED_AT
            {
                reshaped = true;
                state
                    .apply_command(MatchCommand::ChangeFormation {
                        side: Side::Home,
                        formation: formation.to_string(),
                    })
                    .expect("a change of shape is always accepted");
            }
            let minute = state.step_minute(&mut rng);
            if past_the_red {
                minute.events.iter().for_each(&mut watch);
            }
            if minute.is_finished {
                break;
            }
        }
    }
}

/// Goals each side scores after minute RED_AT over every seed, home first (see
/// `watch_after_minute_30`).
fn goals_after_minute_30(
    send_off: Option<fn(u64) -> String>,
    reshape_to: Option<&str>,
) -> [u32; 2] {
    let mut goals = [0u32; 2];
    watch_after_minute_30(send_off, reshape_to, |event| {
        if matches!(event.event_type, EventType::Goal | EventType::PenaltyGoal) {
            goals[match event.side {
                Side::Home => 0,
                Side::Away => 1,
            }] += 1;
        }
    });
    goals
}

/// The share of the away side's duels in `zone` that it wins after minute RED_AT: an event in
/// `won` for the away side is a duel won, an event in `lost` for the home side a duel lost.
fn away_duel_share(
    send_off: Option<fn(u64) -> String>,
    zone: Zone,
    won: EventType,
    lost: &[EventType],
) -> f64 {
    let (mut wins, mut losses) = (0u32, 0u32);
    watch_after_minute_30(send_off, None, |event| {
        if event.zone != zone {
            return;
        }
        if event.side == Side::Away && event.event_type == won {
            wins += 1;
        } else if event.side == Side::Home && lost.contains(&event.event_type) {
            losses += 1;
        }
    });
    f64::from(wins) / f64::from(wins + losses)
}

/// The share a duel's attacker wins once the defending side covers its ground at `cover` of its
/// strength, from the share `before` he won against a full side: the duel is `a / (a + d)`, and
/// only the defender's `d` is scaled.
fn share_against_cover(before: f64, cover: f64) -> f64 {
    let odds = before / (1.0 - before);
    odds / (odds + cover)
}

/// The cover a side one player short keeps when it defends: 1 / (1 + 0.7).
const COVER_ONE_SHORT: f64 = 1.0 / 1.7;

/// Given two identical sides, over many seeds,
/// When one home outfielder is sent off at minute 30,
/// Then, against the same seeds played eleven against eleven, the away side (left with eleven)
/// scores about 1.9 to 2.2 times as often after it and the home side (down to ten) no more often
/// than before, as the literature finds. The bands, ×1.6–2.5 and ×0.45–1.05, take in both
/// studies' estimates and the counts' own noise at SEEDS matches (about ±0.05 on each ratio).
#[test]
fn a_side_down_to_ten_plays_worse_as_the_literature_finds() {
    let eleven_a_side = goals_after_minute_30(None, None);
    let after_a_red = goals_after_minute_30(Some(an_outfielder), None);
    let ratio = |side: usize| f64::from(after_a_red[side]) / f64::from(eleven_a_side[side]);
    let (sanctioned, other) = (ratio(0), ratio(1));
    assert!(
        (1.6..=2.5).contains(&other),
        "the side left with eleven scored {other:.3}x as often (goals {after_a_red:?} against {eleven_a_side:?})"
    );
    assert!(
        (0.45..=1.05).contains(&sanctioned),
        "the side down to ten scored {sanctioned:.3}x as often (goals {after_a_red:?} against {eleven_a_side:?})"
    );
}

/// Given the same seeds, with one home midfielder sent off at minute 30,
/// When the home side changes to a 4-3-2 a minute later, so its three midfielders fill every
/// midfield slot again,
/// Then the side left with eleven still scores at least 1.5 times as often as it does eleven
/// against eleven. The change of shape mends the midfield, but the side is still a man down and
/// covers the ground with ten. If the reshape erased being a man down, the ratio would fall back
/// to about 1.0. The floor sits clear of that and of the counts' noise (about ±0.05).
#[test]
fn a_side_down_to_ten_is_still_a_man_down_after_a_change_of_shape() {
    let eleven_a_side = goals_after_minute_30(None, None);
    let reshaped = goals_after_minute_30(Some(a_midfielder), Some("4-3-2"));
    let other = f64::from(reshaped[1]) / f64::from(eleven_a_side[1]);
    assert!(
        other >= 1.5,
        "after a change of shape the side left with eleven scored {other:.3}x as often (goals {reshaped:?} against {eleven_a_side:?})"
    );
}

/// Given the same seeds, with one home outfielder sent off at minute 30,
/// When the away side (left with eleven) takes on the home midfield,
/// Then it wins the share of its midfield duels that a defender at the short side's cover
/// predicts from the share it wins against a full side. The duel closes with the away side's
/// completed pass or the home side's tackle or interception in midfield. The tolerance, 0.02,
/// is several times the share's noise over SEEDS matches (about ±0.004), and well under the
/// effect (about +0.13).
#[test]
fn the_side_left_with_eleven_wins_more_midfield_duels() {
    let measure = |send_off: Option<fn(u64) -> String>| {
        away_duel_share(
            send_off,
            Zone::Midfield,
            EventType::PassCompleted,
            &[EventType::Tackle, EventType::Interception],
        )
    };
    let before = measure(None);
    let after = measure(Some(an_outfielder));
    let predicted = share_against_cover(before, COVER_ONE_SHORT);
    assert!(
        (after - predicted).abs() <= 0.02,
        "the side left with eleven won {after:.3} of its midfield duels, from {before:.3}; the cover predicts {predicted:.3}"
    );
}

/// Given the same seeds, with one home outfielder sent off at minute 30,
/// When the away side (left with eleven) runs at the home defence in its attacking third,
/// Then its dribbles beat the short side as often as a defender at the short side's cover
/// predicts. A failed dribble ends half the time in a tackle and half in a clearance, so the
/// dribbles won over dribbles won and tackled, `q`, is `s / (s + (1 - s) / 2)` of the true share
/// `s`, which this inverts. The tolerance is as for the midfield duels.
#[test]
fn the_side_left_with_eleven_wins_more_attacking_duels() {
    let measure = |send_off: Option<fn(u64) -> String>| {
        let q = away_duel_share(
            send_off,
            Zone::attacking_third(Side::Away),
            EventType::Dribble,
            &[EventType::Tackle],
        );
        q / (2.0 - q)
    };
    let before = measure(None);
    let after = measure(Some(an_outfielder));
    let predicted = share_against_cover(before, COVER_ONE_SHORT);
    assert!(
        (after - predicted).abs() <= 0.02,
        "the side left with eleven won {after:.3} of its attacking duels, from {before:.3}; the cover predicts {predicted:.3}"
    );
}

/// A midfield of four whose passing, vision, decisions and stamina are each `skills[i]`.
fn midfield_of(skills: [u8; 4]) -> TeamData {
    let mut team = side("t");
    for (index, skill) in skills.into_iter().enumerate() {
        let midfielder = &mut team.players[5 + index];
        midfielder.passing = skill;
        midfielder.vision = skill;
        midfielder.decisions = skill;
        midfielder.stamina = skill;
    }
    team
}

fn midfield_skill(p: &PlayerData) -> u8 {
    ((p.passing as u16 + p.vision as u16 + p.decisions as u16 + p.stamina as u16) / 4) as u8
}

/// Given a four-man midfield rated 60, 70, 80 and 90,
/// When the 90 is sent off,
/// Then the line loses exactly that player's 90 over its four slots: an empty slot, not a smaller line.
#[test]
fn a_line_that_loses_a_player_loses_that_players_share_of_its_slots() {
    let team = midfield_of([60, 70, 80, 90]);
    let full = team.line_strength(Position::Midfielder, midfield_skill, &|_| false, 4);
    let short = team.line_strength(Position::Midfielder, midfield_skill, &|id| id == "t_8", 4);
    assert_eq!(full, 75.0);
    assert_eq!(short - full, -90.0 / 4.0);
}

/// Given the same midfield with the 90 sent off,
/// When the side's shape gives the midfield only three slots,
/// Then the line is whole again: a change of shape fills the empty slot.
#[test]
fn a_change_of_shape_fills_the_empty_slot() {
    let team = midfield_of([60, 70, 80, 90]);
    let short = team.line_strength(Position::Midfielder, midfield_skill, &|id| id == "t_8", 4);
    let reshaped = team.line_strength(Position::Midfielder, midfield_skill, &|id| id == "t_8", 3);
    assert_eq!(reshaped, 70.0);
    assert_eq!(reshaped - short, 70.0 - 210.0 / 4.0);
}

/// Given a full four-man midfield rated 60, 70, 80 and 90,
/// When its shape says four midfield slots, or five (a 4-1-4-1 counts five),
/// Then the line is its plain average either way: only a player who leaves the pitch empties a
/// slot, so a match without a sending-off plays as it did.
#[test]
fn a_line_with_nobody_off_is_its_plain_average_whatever_the_shape() {
    let team = midfield_of([60, 70, 80, 90]);
    for slots in [4, 5] {
        assert_eq!(
            team.line_strength(Position::Midfielder, midfield_skill, &|_| false, slots),
            team.midfield_rating()
        );
    }
}
