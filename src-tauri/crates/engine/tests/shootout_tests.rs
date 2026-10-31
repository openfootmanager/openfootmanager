//! Who takes the kicks in a penalty shootout.
//!
//! Every match here is forced level at full time and at the end of extra time, so every seed
//! ends in a shootout, and the tests read each side's kickers from the shootout events.

use engine::live_match::LiveMatchState;
use engine::{
    EventType, MatchCommand, MatchConfig, MatchPhase, PlayStyle, PlayerData, PlayerRole, Position,
    Side, TacticsConfig, TeamData,
};
use rand::SeedableRng;
use rand::rngs::StdRng;

const SEEDS: u64 = 300;

/// A player with the two attributes a shootout kick uses.
fn player(id: &str, position: Position, (shooting, composure): (u8, u8)) -> PlayerData {
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
        shooting,
        tackling: 70,
        dribbling: 70,
        defending: 70,
        positioning: 70,
        vision: 70,
        decisions: 70,
        composure,
        // Low aggression keeps the referee's cards out of the way: the tests send players off
        // themselves.
        aggression: 1,
        teamwork: 70,
        leadership: 70,
        handling: 70,
        reflexes: 70,
        aerial: 70,
        traits: vec![],
        role: PlayerRole::Standard,
    }
}

/// (shooting, composure) of outfield players 1 to 10. Ranked by their sum, what the kick uses,
/// they go 4, 6, 2, 1, 5, 3, 8, 10, 9, 7. Ranked by shooting alone, composure alone, their
/// difference or their product, the order is different, so a test that reads the order tells
/// those apart.
const PENALTY_SKILLS: [(u8, u8); 10] = [
    (40, 90),
    (90, 50),
    (60, 60),
    (85, 70),
    (30, 95),
    (70, 75),
    (50, 50),
    (95, 20),
    (45, 60),
    (65, 45),
];

/// A 4-4-2 of outfield players `{prefix}1`..`{prefix}10` with the skills above. The goalkeeper
/// `{prefix}_gk` is the best penalty taker of all, so a keeper who kicks last does so because he
/// is the keeper, not because he is poor at it.
fn side(prefix: &str) -> TeamData {
    let mut players = vec![player(
        &format!("{prefix}_gk"),
        Position::Goalkeeper,
        (99, 99),
    )];
    for (index, skills) in PENALTY_SKILLS.into_iter().enumerate() {
        let number = index + 1;
        let position = match number {
            1..=2 => Position::Forward,
            3..=6 => Position::Midfielder,
            _ => Position::Defender,
        };
        players.push(player(&format!("{prefix}{number}"), position, skills));
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

fn ids(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| name.to_string()).collect()
}

/// Each side's kickers, in the order they kicked.
struct Kickers {
    home: Vec<String>,
    away: Vec<String>,
}

fn kick_off(home_bench: Vec<PlayerData>) -> LiveMatchState {
    LiveMatchState::new(
        side("h"),
        side("a"),
        MatchConfig::default(),
        home_bench,
        vec![],
        true,
    )
}

/// Plays `state` with seed `seed` to the final whistle of extra time, level: the phase is
/// `ExtraTimeEnd` and the shootout has not begun. `None` when the referee sent anyone off, since
/// the tests choose who is off the pitch.
fn to_the_final_whistle(seed: u64, mut state: LiveMatchState) -> Option<(LiveMatchState, StdRng)> {
    let mut rng = StdRng::seed_from_u64(seed);
    while state.phase() != MatchPhase::ExtraTimeEnd {
        if state.phase() == MatchPhase::FullTime {
            state.test_set_score(0, 0);
        }
        state.step_minute(&mut rng);
    }
    if !state.snapshot().sent_off.is_empty() {
        return None;
    }
    state.test_set_score(0, 0);
    Some((state, rng))
}

/// Plays a match that is level at the final whistle of extra time through its shootout.
fn play_out(mut state: LiveMatchState, mut rng: StdRng) -> Kickers {
    let mut kickers = Kickers {
        home: vec![],
        away: vec![],
    };
    loop {
        let minute = state.step_minute(&mut rng);
        for event in &minute.events {
            if matches!(
                event.event_type,
                EventType::ShootoutGoal | EventType::ShootoutMiss
            ) {
                let kicker = event.player_id.clone().unwrap_or_default();
                match event.side {
                    Side::Home => kickers.home.push(kicker),
                    Side::Away => kickers.away.push(kicker),
                }
            }
        }
        if minute.is_finished {
            return kickers;
        }
    }
}

/// Plays seed `seed` to a shootout and through it. `before_kick_off` runs before the first
/// minute, `at_the_final_whistle` once extra time is over and before the shootout starts.
/// `None` when the referee sent anyone off.
fn shootout(
    seed: u64,
    before_kick_off: &dyn Fn(&mut LiveMatchState),
    at_the_final_whistle: &dyn Fn(&mut LiveMatchState),
) -> Option<Kickers> {
    let mut state = kick_off(vec![]);
    before_kick_off(&mut state);
    let (mut state, rng) = to_the_final_whistle(seed, state)?;
    at_the_final_whistle(&mut state);
    Some(play_out(state, rng))
}

fn shootouts(
    before_kick_off: &dyn Fn(&mut LiveMatchState),
    at_the_final_whistle: &dyn Fn(&mut LiveMatchState),
) -> Vec<Kickers> {
    (0..SEEDS)
        .filter_map(|seed| shootout(seed, before_kick_off, at_the_final_whistle))
        .collect()
}

fn nothing(_: &mut LiveMatchState) {}

/// Sends off each named player, as if the referee had during the match.
fn send_off(names: &'static [&'static str]) -> impl Fn(&mut LiveMatchState) {
    move |state| {
        for name in names {
            state.test_send_off(name);
        }
    }
}

/// Asserts `kicked` follows `order` from the top, starting again from the top once everyone in
/// it has kicked.
fn assert_follows(kicked: &[String], order: &[String]) {
    for (kick, kicker) in kicked.iter().enumerate() {
        assert_eq!(
            kicker,
            &order[kick % order.len()],
            "kick {} of {kicked:?} breaks the order {order:?}",
            kick + 1
        );
    }
}

/// Given two full sides level after extra time,
/// When a side takes its five regulation kicks,
/// Then five different players take them.
#[test]
fn five_different_players_take_a_full_sides_regulation_kicks() {
    let shootouts = shootouts(&nothing, &nothing);
    let mut five_kicks = 0;
    for kickers in &shootouts {
        for kicked in [&kickers.home, &kickers.away] {
            let regulation: Vec<&String> = kicked.iter().take(5).collect();
            let mut different = regulation.clone();
            different.sort();
            different.dedup();
            assert_eq!(
                different.len(),
                regulation.len(),
                "a player kicked twice in {regulation:?}"
            );
            five_kicks += usize::from(regulation.len() == 5);
        }
    }
    assert!(five_kicks > 100, "only {five_kicks} sides took five kicks");
}

/// Given a side whose designated penalty taker is its weakest penalty taker,
/// When the shootout comes,
/// Then he kicks first, the rest follow from the best penalty taker down, and the goalkeeper
/// kicks last, though he is the best of all. The side with no designated taker starts from its
/// best.
#[test]
fn the_automatic_order_is_the_designated_taker_then_penalty_skill_then_the_keeper() {
    let designate_h7 = |state: &mut LiveMatchState| {
        state
            .apply_command(MatchCommand::SetPenaltyTaker {
                side: Side::Home,
                player_id: "h7".to_string(),
            })
            .expect("h7 is on the pitch");
    };
    let home_order = ids(&[
        "h7", "h4", "h6", "h2", "h1", "h5", "h3", "h8", "h10", "h9", "h_gk",
    ]);
    let away_order = ids(&[
        "a4", "a6", "a2", "a1", "a5", "a3", "a8", "a10", "a9", "a7", "a_gk",
    ]);
    for kickers in shootouts(&designate_h7, &nothing) {
        assert_follows(&kickers.home, &home_order);
        assert_follows(&kickers.away, &away_order);
    }
}

/// Given two sides with seven players left on the pitch, the goalkeeper the best penalty taker,
/// When the shootout goes to a seventh kick,
/// Then the goalkeeper takes it, and never one before.
#[test]
fn the_goalkeeper_kicks_last() {
    let seven_each = send_off(&["h7", "h8", "h9", "h10", "a7", "a8", "a9", "a10"]);
    let mut seventh_kicks = 0;
    for kickers in shootouts(&nothing, &seven_each) {
        for (kicked, keeper) in [(&kickers.home, "h_gk"), (&kickers.away, "a_gk")] {
            if let Some(first) = kicked.iter().position(|kicker| kicker == keeper) {
                assert_eq!(first, 6, "the keeper kicked at {} in {kicked:?}", first + 1);
            }
            seventh_kicks += usize::from(kicked.len() >= 7);
        }
    }
    assert!(
        seventh_kicks > 5,
        "only {seventh_kicks} sides kicked seven times"
    );
}

/// Given two sides with seven players left on the pitch,
/// When sudden death goes past the seventh kick,
/// Then the order starts again from the top: the eighth kick is the first kicker's.
#[test]
fn in_sudden_death_the_order_starts_again_from_the_top() {
    let seven_each = send_off(&["h7", "h8", "h9", "h10", "a7", "a8", "a9", "a10"]);
    let home_order = ids(&["h4", "h6", "h2", "h1", "h5", "h3", "h_gk"]);
    let away_order = ids(&["a4", "a6", "a2", "a1", "a5", "a3", "a_gk"]);
    let mut wrapped = 0;
    for kickers in shootouts(&nothing, &seven_each) {
        assert_follows(&kickers.home, &home_order);
        assert_follows(&kickers.away, &away_order);
        wrapped += usize::from(kickers.home.len() >= 8);
    }
    assert!(
        wrapped > 2,
        "only {wrapped} shootouts went past seven kicks"
    );
}

/// Given a side whose designated penalty taker and best penalty taker was sent off,
/// When the shootout comes,
/// Then he never kicks.
#[test]
fn a_sent_off_player_never_kicks() {
    let designate_h4 = |state: &mut LiveMatchState| {
        state
            .apply_command(MatchCommand::SetPenaltyTaker {
                side: Side::Home,
                player_id: "h4".to_string(),
            })
            .expect("h4 is on the pitch");
    };
    let h4_off = send_off(&["h4"]);
    for kickers in shootouts(&designate_h4, &h4_off) {
        assert!(
            !kickers.home.iter().any(|kicker| kicker == "h4"),
            "h4 kicked after his red card: {:?}",
            kickers.home
        );
    }
}

/// Given a full side against a side with seven players left on the pitch,
/// When the shootout comes,
/// Then the full side uses only seven kickers too: its best seven by the automatic order, and
/// in sudden death it starts again from its first.
#[test]
fn both_sides_use_only_as_many_kickers_as_the_side_with_fewer() {
    let away_seven = send_off(&["a7", "a8", "a9", "a10"]);
    let home_order = ids(&["h4", "h6", "h2", "h1", "h5", "h3", "h8"]);
    let mut wrapped = 0;
    for kickers in shootouts(&nothing, &away_seven) {
        assert_follows(&kickers.home, &home_order);
        wrapped += usize::from(kickers.home.len() >= 8);
    }
    assert!(
        wrapped > 2,
        "only {wrapped} shootouts went past seven kicks"
    );
}

const NO_SUBSTITUTIONS: &str = "be.error.liveMatch.noSubstitutionsForShootout";

/// The first seed that reaches the final whistle of extra time with nobody sent off, with
/// `h_sub` on the home bench.
fn final_whistle_with_a_substitute() -> (LiveMatchState, StdRng) {
    let substitute = player("h_sub", Position::Forward, (99, 99));
    (0..SEEDS)
        .find_map(|seed| to_the_final_whistle(seed, kick_off(vec![substitute.clone()])))
        .expect("some seed reaches the final whistle without a red card")
}

fn bring_on_h_sub_for_h7(state: &mut LiveMatchState) -> Result<(), String> {
    state.apply_command(MatchCommand::Substitute {
        side: Side::Home,
        player_off_id: "h7".to_string(),
        player_on_id: "h_sub".to_string(),
    })
}

/// Given a level match at the final whistle of extra time, with a substitute on the bench,
/// When a manager tries to bring the substitute on before the shootout,
/// Then it is refused: the shootout is taken by the players on the pitch at the final whistle,
/// and the substitute never kicks.
#[test]
fn a_substitution_after_the_final_whistle_of_extra_time_is_refused() {
    let (mut state, rng) = final_whistle_with_a_substitute();
    assert_eq!(
        bring_on_h_sub_for_h7(&mut state),
        Err(NO_SUBSTITUTIONS.to_string())
    );
    let kicked = play_out(state, rng).home;
    assert!(
        !kicked.iter().any(|kicker| kicker == "h_sub"),
        "the substitute kicked: {kicked:?}"
    );
}

/// Given a shootout under way, with a substitute on the bench,
/// When a manager tries to bring the substitute on,
/// Then it is refused, and the side keeps kicking in its order.
#[test]
fn a_substitution_during_the_shootout_is_refused() {
    let (mut state, mut rng) = final_whistle_with_a_substitute();
    state.step_minute(&mut rng);
    assert_eq!(state.phase(), MatchPhase::PenaltyShootout);
    assert_eq!(
        bring_on_h_sub_for_h7(&mut state),
        Err(NO_SUBSTITUTIONS.to_string())
    );
    let home_order = ids(&[
        "h4", "h6", "h2", "h1", "h5", "h3", "h8", "h10", "h9", "h7", "h_gk",
    ]);
    assert_follows(&play_out(state, rng).home, &home_order);
}

/// Given a side with nobody left on the pitch at the final whistle,
/// When the shootout comes,
/// Then the other side still kicks in its order: a side with nobody cannot hold the other to
/// nobody.
#[test]
fn a_side_still_kicks_in_order_when_the_other_has_nobody_left() {
    let whole_away_side_off = send_off(&[
        "a_gk", "a1", "a2", "a3", "a4", "a5", "a6", "a7", "a8", "a9", "a10",
    ]);
    let home_order = ids(&[
        "h4", "h6", "h2", "h1", "h5", "h3", "h8", "h10", "h9", "h7", "h_gk",
    ]);
    let shootouts = shootouts(&nothing, &whole_away_side_off);
    assert!(!shootouts.is_empty(), "some seed reaches a shootout");
    for kickers in shootouts {
        assert_follows(&kickers.home, &home_order);
    }
}
