//! Who takes the kicks in a penalty shootout.
//!
//! Every match here is forced level at full time and at the end of extra time, so every seed
//! ends in a shootout, and the tests read each side's kickers from the shootout events.

use engine::live_match::LiveMatchState;
use engine::{
    EventType, MatchCommand, MatchConfig, MatchPhase, PlayStyle, PlayerData, PlayerRole, Position,
    ShootoutOrderSnapshot, Side, TacticsConfig, TeamData,
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
/// `{prefix}_gk` is the best penalty taker of all, so a keeper who kicks last does so for being
/// the keeper, not for poor penalty skill.
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
    step_until(&mut state, &mut rng, MatchPhase::ExtraTimeEnd);
    if !state.snapshot().sent_off.is_empty() {
        return None;
    }
    state.test_set_score(0, 0);
    Some((state, rng))
}

/// Steps `state` until it reaches `phase`, level at full time so that extra time is played.
fn step_until(state: &mut LiveMatchState, rng: &mut StdRng, phase: MatchPhase) {
    while state.phase() != phase {
        if state.phase() == MatchPhase::FullTime {
            state.test_set_score(0, 0);
        }
        state.step_minute(rng);
    }
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

/// Plays seed `seed` to the start of a shootout: the phase is `PenaltyShootout` and no kick has
/// been taken. `None` when the referee sent anyone off.
fn to_the_shootout(
    seed: u64,
    before_kick_off: &dyn Fn(&mut LiveMatchState),
    at_the_final_whistle: &dyn Fn(&mut LiveMatchState),
) -> Option<(LiveMatchState, StdRng)> {
    let mut state = kick_off(vec![]);
    before_kick_off(&mut state);
    let (mut state, mut rng) = to_the_final_whistle(seed, state)?;
    at_the_final_whistle(&mut state);
    state.step_minute(&mut rng);
    assert_eq!(state.phase(), MatchPhase::PenaltyShootout);
    Some((state, rng))
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
/// Then that player kicks first, the rest follow from the best penalty taker down, and the goalkeeper
/// kicks last, though the best of all. The side with no designated taker starts from its
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
/// Then that player never kicks.
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
            "h4 kicked after a red card: {:?}",
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

/// Given a level match at half-time of extra time, with a substitute on the bench,
/// When a manager brings the substitute on for h7,
/// Then it is accepted, and at the shootout the substitute, the best penalty taker on the
/// pitch, kicks first while h7 never kicks.
#[test]
fn a_substitution_before_the_final_whistle_of_extra_time_still_counts() {
    let substitute = player("h_sub", Position::Forward, (99, 99));
    let order_with_the_substitute = ids(&[
        "h_sub", "h4", "h6", "h2", "h1", "h5", "h3", "h8", "h10", "h9", "h_gk",
    ]);
    for seed in 0..SEEDS {
        let mut state = kick_off(vec![substitute.clone()]);
        let mut rng = StdRng::seed_from_u64(seed);
        step_until(&mut state, &mut rng, MatchPhase::ExtraTimeHalfTime);
        if !state.snapshot().sent_off.is_empty() {
            continue;
        }
        bring_on_h_sub_for_h7(&mut state).expect("a substitution at half-time of extra time");
        step_until(&mut state, &mut rng, MatchPhase::ExtraTimeEnd);
        if !state.snapshot().sent_off.is_empty() {
            continue;
        }
        state.test_set_score(0, 0);
        assert_follows(&play_out(state, rng).home, &order_with_the_substitute);
        return;
    }
    panic!("no seed reached the final whistle of extra time without a red card");
}

// ---------------------------------------------------------------------------
// A manager's own order
// ---------------------------------------------------------------------------

const NOT_NOW: &str = "be.error.liveMatch.shootoutOrderNotNow";
const NOT_ELIGIBLE: &str = "be.error.liveMatch.shootoutOrderNotEligible";

/// Everyone in a full side, in the automatic order.
const FULL_HOME_ORDER: [&str; 11] = [
    "h4", "h6", "h2", "h1", "h5", "h3", "h8", "h10", "h9", "h7", "h_gk",
];

/// A full side's order that is not the automatic one: the keeper first, the best penalty taker
/// last.
const CHOSEN_HOME_ORDER: [&str; 11] = [
    "h_gk", "h7", "h9", "h10", "h8", "h3", "h5", "h1", "h2", "h6", "h4",
];

fn set_order(state: &mut LiveMatchState, side: Side, names: &[&str]) -> Result<(), String> {
    state.apply_command(MatchCommand::SetShootoutOrder {
        side,
        player_ids: ids(names),
    })
}

/// The first seed that reaches a shootout with nobody sent off by the referee.
fn first_shootout(at_the_final_whistle: &dyn Fn(&mut LiveMatchState)) -> (LiveMatchState, StdRng) {
    (0..SEEDS)
        .find_map(|seed| to_the_shootout(seed, &nothing, at_the_final_whistle))
        .expect("some seed reaches a shootout without a red card")
}

/// Given two full sides at the start of a shootout,
/// When the engine is asked for each side's order,
/// Then it lists all eleven in the automatic order, and all eleven kick.
#[test]
fn the_engine_reports_each_sides_order_at_the_start_of_the_shootout() {
    let (state, _) = first_shootout(&nothing);
    assert_eq!(
        state.shootout_order(Side::Home),
        Some(ShootoutOrderSnapshot {
            player_ids: ids(&FULL_HOME_ORDER),
            kickers: 11,
        })
    );
    assert_eq!(
        state.shootout_order(Side::Away).map(|order| order.kickers),
        Some(11)
    );
}

/// Given a full side against a side with seven players left,
/// When the engine is asked for each side's order,
/// Then the full side's lists all eleven, so the manager can choose who sits out, but only
/// seven kick on either side.
#[test]
fn a_full_side_against_seven_lists_all_eleven_but_seven_kick() {
    let (state, _) = first_shootout(&send_off(&["a7", "a8", "a9", "a10"]));
    assert_eq!(
        state.shootout_order(Side::Home),
        Some(ShootoutOrderSnapshot {
            player_ids: ids(&FULL_HOME_ORDER),
            kickers: 7,
        })
    );
    assert_eq!(
        state.shootout_order(Side::Away),
        Some(ShootoutOrderSnapshot {
            player_ids: ids(&["a4", "a6", "a2", "a1", "a5", "a3", "a_gk"]),
            kickers: 7,
        })
    );
}

/// Given a match that has not reached a shootout,
/// When the engine is asked for an order,
/// Then there is none.
#[test]
fn there_is_no_order_before_the_shootout() {
    assert_eq!(kick_off(vec![]).shootout_order(Side::Home), None);
}

/// Given the start of a shootout,
/// When a manager sets their own order before the first kick,
/// Then the engine reports that order, and the side kicks in it, starting again from the top in
/// sudden death.
#[test]
fn a_side_kicks_in_the_order_its_manager_sets() {
    for seed in 0..SEEDS {
        let Some((mut state, rng)) = to_the_shootout(seed, &nothing, &nothing) else {
            continue;
        };
        set_order(&mut state, Side::Home, &CHOSEN_HOME_ORDER)
            .expect("the order is the eligible players");
        assert_eq!(
            state.shootout_order(Side::Home),
            Some(ShootoutOrderSnapshot {
                player_ids: ids(&CHOSEN_HOME_ORDER),
                kickers: 11,
            })
        );
        assert_follows(&play_out(state, rng).home, &ids(&CHOSEN_HOME_ORDER));
    }
}

/// Given a full side against a side with seven players left,
/// When the full side's manager puts the best penalty taker last,
/// Then that player sits out, and the order starts again after the seventh kick.
#[test]
fn a_player_put_past_the_kicker_count_sits_out() {
    let away_seven = send_off(&["a7", "a8", "a9", "a10"]);
    let mut wrapped = 0;
    for seed in 0..SEEDS {
        let Some((mut state, rng)) = to_the_shootout(seed, &nothing, &away_seven) else {
            continue;
        };
        set_order(&mut state, Side::Home, &CHOSEN_HOME_ORDER)
            .expect("the order is the eligible players");
        let kicked = play_out(state, rng).home;
        assert_follows(&kicked, &ids(&CHOSEN_HOME_ORDER[..7]));
        wrapped += usize::from(kicked.len() >= 8);
    }
    assert!(
        wrapped > 2,
        "only {wrapped} shootouts went past seven kicks"
    );
}

/// Given a shootout in which the first kick has been taken,
/// When the other side's manager tries to set an order,
/// Then it is refused and the order stands.
#[test]
fn an_order_is_refused_once_a_kick_has_been_taken() {
    let (mut state, mut rng) = first_shootout(&nothing);
    let before = state.shootout_order(Side::Away);
    state.step_minute(&mut rng);
    let reversed = [
        "a_gk", "a7", "a9", "a10", "a8", "a3", "a5", "a1", "a2", "a6", "a4",
    ];
    assert_eq!(
        set_order(&mut state, Side::Away, &reversed),
        Err(NOT_NOW.to_string())
    );
    assert_eq!(state.shootout_order(Side::Away), before);
}

/// Given a match before kick-off,
/// When a manager tries to set a shootout order,
/// Then it is refused: there is no shootout to order yet.
#[test]
fn an_order_is_refused_before_the_shootout() {
    let mut state = kick_off(vec![]);
    assert_eq!(
        set_order(&mut state, Side::Home, &FULL_HOME_ORDER),
        Err(NOT_NOW.to_string())
    );
}

/// Given the start of a shootout with h1 sent off,
/// When a manager sends an order that is not exactly the side's eligible players (one missing, one
/// twice, the sent-off player, a stranger),
/// Then each is refused and the automatic order stands; the eligible players reordered are
/// accepted.
#[test]
fn an_order_that_is_not_exactly_the_eligible_players_is_refused() {
    let (mut state, _) = first_shootout(&send_off(&["h1"]));
    let automatic = state.shootout_order(Side::Home);
    let eligible = [
        "h_gk", "h6", "h2", "h5", "h3", "h8", "h10", "h9", "h7", "h4",
    ];
    let missing_one = &eligible[1..];
    let twice = [
        "h4", "h4", "h2", "h5", "h3", "h8", "h10", "h9", "h7", "h_gk",
    ];
    let sent_off = [
        "h1", "h6", "h2", "h5", "h3", "h8", "h10", "h9", "h7", "h_gk",
    ];
    let stranger = [
        "a4", "h6", "h2", "h5", "h3", "h8", "h10", "h9", "h7", "h_gk",
    ];
    for order in [missing_one, &twice[..], &sent_off[..], &stranger[..]] {
        assert_eq!(
            set_order(&mut state, Side::Home, order),
            Err(NOT_ELIGIBLE.to_string()),
            "{order:?} was accepted"
        );
        assert_eq!(state.shootout_order(Side::Home), automatic);
    }
    assert_eq!(set_order(&mut state, Side::Home, &eligible), Ok(()));
}

/// Given the start of a shootout,
/// When the engine's own snapshot is taken,
/// Then it carries neither side's order: what a viewer may see of an order is the session's
/// decision, which shows a manager only their own.
#[test]
fn the_engine_snapshot_never_carries_an_order() {
    let (state, _) = first_shootout(&nothing);
    let shootout = state.snapshot().penalty_shootout.expect("a shootout is on");
    assert_eq!(shootout.home_order, None);
    assert_eq!(shootout.away_order, None);
}
