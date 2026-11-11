//! The range of each answer's effect on morale, and that the words the manager reads match the
//! way morale moved. Each answer is tried over sixty seeds on a player at morale 50 with the
//! default personality (+10), so every draw of the dice is seen.

use super::*;

/// (morale change, effect key) for each of seeds 0..60, answering a fresh `message_id`.
fn answers(message_id: &str, option_id: &str) -> Vec<(i16, String)> {
    answers_from(message_id, option_id, |_| {})
}

/// As [`answers`], with the player shaped by `shape` first.
fn answers_from(
    message_id: &str,
    option_id: &str,
    shape: impl Fn(&mut Player),
) -> Vec<(i16, String)> {
    (0..60)
        .map(|seed| {
            let mut game = make_game();
            game.seed = seed;
            let player = game.players.iter_mut().find(|p| p.id == "p_fwd0").unwrap();
            player.morale = 50;
            shape(player);
            inject_player_message(&mut game, message_id, "p_fwd0", "respond");
            let effect =
                player_events::apply_player_response(&mut game, message_id, "respond", option_id)
                    .expect("the answer is applied");
            let morale = game
                .players
                .iter()
                .find(|p| p.id == "p_fwd0")
                .unwrap()
                .morale;
            (i16::from(morale) - 50, effect.i18n_key)
        })
        .collect()
}

/// Every change falls in `range`, and the effect reads positive exactly when morale did not fall.
fn assert_ranged_and_worded(answers: &[(i16, String)], range: std::ops::RangeInclusive<i16>) {
    for (delta, key) in answers {
        assert!(range.contains(delta), "{delta} outside {range:?} ({key})");
        assert_eq!(
            key.ends_with(".positive"),
            *delta >= 0,
            "a change of {delta} read as {key}"
        );
    }
}

/// Given a benched player told why he is out
/// When he hears it, over sixty seeds
/// Then his morale moves by −2 to +6 plus a quarter of his personality (0 to 8), and the words
///      match the direction
#[test]
fn explaining_the_bench_moves_morale_within_its_range_and_reads_its_direction() {
    assert_ranged_and_worded(&answers("bench_complaint_p_fwd0", "explain"), 0..=8);
}

/// Given a composed leader on the bench (composure 95, leadership 95, aggression 20: personality
///       +20) told why he is out
/// When he hears it, over sixty seeds
/// Then his morale moves by −2 to +6 plus a quarter of his personality (3 to 11): the quarter is a
///      share of his personality, not what is left over from it
#[test]
fn explaining_the_bench_to_a_composed_leader_lifts_him_by_a_quarter_of_his_personality_more() {
    let leader = |p: &mut Player| {
        p.attributes.composure = 95;
        p.attributes.leadership = 95;
        p.attributes.aggression = 20;
    };

    assert_ranged_and_worded(
        &answers_from("bench_complaint_p_fwd0", "explain", leader),
        3..=11,
    );
}

/// Given a benched player told to prove himself
/// When he hears it, over sixty seeds
/// Then his morale moves by −10 to +6 plus a third of his personality (−7 to 9), and the words
///      match the direction
#[test]
fn telling_him_to_prove_himself_moves_morale_within_its_range_and_reads_its_direction() {
    assert_ranged_and_worded(&answers("bench_complaint_p_fwd0", "prove_yourself"), -7..=9);
}

/// Given a happy player answered professionally
/// When he hears it, over sixty seeds
/// Then his morale moves by −2 to +3 plus a sixth of his personality (−1 to 4), and the words
///      match the direction
#[test]
fn staying_professional_moves_morale_within_its_range_and_reads_its_direction() {
    assert_ranged_and_worded(&answers("happy_player_p_fwd0", "stay_professional"), -1..=4);
}

/// Given a happy player whose expectations are raised
/// When he hears it, over sixty seeds
/// Then his morale moves by −6 to +4 plus a third of his personality (−3 to 7), and the words
///      match the direction
#[test]
fn raising_expectations_moves_morale_within_its_range_and_reads_its_direction() {
    assert_ranged_and_worded(
        &answers("happy_player_p_fwd0", "higher_expectations"),
        -3..=7,
    );
}

/// Given a player worried about his contract, given a noncommittal answer
/// When he hears it, over sixty seeds
/// Then his morale moves by −8 to 0 plus a fifth of his personality (−6 to 2), and the words
///      match the direction
#[test]
fn a_noncommittal_answer_moves_morale_within_its_range_and_reads_its_direction() {
    assert_ranged_and_worded(&answers("contract_concern_p_fwd0", "noncommittal"), -6..=2);
}

/// Given a benched player at trust 50
/// When he is told to prove himself
/// Then his trust falls by 2, to 48
#[test]
fn telling_a_benched_player_to_prove_himself_costs_two_trust() {
    let mut game = make_game();
    inject_player_message(&mut game, "bench_complaint_p_fwd0", "p_fwd0", "respond");

    player_events::apply_player_response(
        &mut game,
        "bench_complaint_p_fwd0",
        "respond",
        "prove_yourself",
    )
    .expect("the answer is applied");

    let trust = game
        .players
        .iter()
        .find(|p| p.id == "p_fwd0")
        .unwrap()
        .morale_core
        .manager_trust;
    assert_eq!(trust, 48);
}

/// Given a happy player at trust 50
/// When his expectations are raised
/// Then his trust falls by 1, to 49
#[test]
fn raising_a_happy_players_expectations_costs_one_trust() {
    let mut game = make_game();
    inject_player_message(&mut game, "happy_player_p_fwd0", "p_fwd0", "respond");

    player_events::apply_player_response(
        &mut game,
        "happy_player_p_fwd0",
        "respond",
        "higher_expectations",
    )
    .expect("the answer is applied");

    let trust = game
        .players
        .iter()
        .find(|p| p.id == "p_fwd0")
        .unwrap()
        .morale_core
        .manager_trust;
    assert_eq!(trust, 49);
}
