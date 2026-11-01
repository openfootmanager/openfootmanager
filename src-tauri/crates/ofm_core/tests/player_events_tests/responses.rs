//! What a conversation does to the player: his trust in the manager, and the odds of each kind
//! of reaction. Every player here starts from `make_game`: trust 50, no memory of earlier talks,
//! composure, leadership and aggression all 60 (a personality factor of +10).

use super::*;
use domain::player::{PlayerIssue, PlayerIssueCategory, RecentTreatmentMemory};
use ofm_core::player_events::{ResponseBandWeights, build_response_band_weights};

fn trust_after(message_id: &str, option_id: &str) -> u8 {
    let mut game = make_game();
    inject_player_message(&mut game, message_id, "p_fwd0", "respond");
    player_events::apply_player_response(&mut game, message_id, "respond", option_id)
        .expect("the answer is applied");
    game.players
        .iter()
        .find(|p| p.id == "p_fwd0")
        .unwrap()
        .morale_core
        .manager_trust
}

/// Given a player at trust 50 who asked for a talk in a crisis
/// When the manager encourages him
/// Then his trust rises by 4, to 54
#[test]
fn encouraging_a_player_in_crisis_raises_his_trust_by_four() {
    assert_eq!(trust_after("morale_talk_p_fwd0", "encourage"), 54);
}

/// Given a player at trust 50 who asked for a talk in a crisis
/// When the manager tells him to work harder
/// Then his trust falls by 3, to 47
#[test]
fn telling_him_to_work_harder_costs_three_trust() {
    assert_eq!(trust_after("morale_talk_p_fwd0", "work_harder"), 47);
}

/// Given a player at trust 50 worried about his contract
/// When the manager gives a noncommittal answer
/// Then his trust falls by 4, to 46
#[test]
fn a_noncommittal_contract_answer_costs_four_trust() {
    assert_eq!(trust_after("contract_concern_p_fwd0", "noncommittal"), 46);
}

/// Given a player at trust 50 worried about his contract
/// When the manager says there will be no renewal
/// Then his trust falls by 8, to 42
#[test]
fn refusing_a_renewal_costs_eight_trust() {
    assert_eq!(trust_after("contract_concern_p_fwd0", "no_renewal"), 42);
}

/// Given a player at trust 50 who asked for a talk in a crisis
/// When the manager promises him playing time, and then promises it again in a second talk
/// Then his trust rises by 8, to 58, and then by only 4, to 62: a promise repeated is worth half
#[test]
fn promising_playing_time_raises_trust_by_eight_then_four() {
    let mut game = make_game();
    inject_player_message(&mut game, "morale_talk_p_fwd0_first", "p_fwd0", "respond");
    inject_player_message(&mut game, "morale_talk_p_fwd0_second", "p_fwd0", "respond");
    let trust = |game: &Game| {
        game.players
            .iter()
            .find(|p| p.id == "p_fwd0")
            .unwrap()
            .morale_core
            .manager_trust
    };

    player_events::apply_player_response(
        &mut game,
        "morale_talk_p_fwd0_first",
        "respond",
        "promise_time",
    )
    .expect("the first answer is applied");
    let after_first = trust(&game);
    player_events::apply_player_response(
        &mut game,
        "morale_talk_p_fwd0_second",
        "respond",
        "promise_time",
    )
    .expect("the second answer is applied");

    assert_eq!((after_first, trust(&game)), (58, 62));
}

fn weights(
    message_id: &str,
    option_id: &str,
    shape: impl FnOnce(&mut domain::player::Player),
) -> ResponseBandWeights {
    let mut player = make_player("p", "P", "team1", Position::Forward);
    shape(&mut player);
    build_response_band_weights(&player, message_id, option_id)
}

fn bands(sp: u32, mp: u32, n: u32, mn: u32, sn: u32) -> ResponseBandWeights {
    ResponseBandWeights {
        strong_positive: sp,
        mild_positive: mp,
        neutral: n,
        mild_negative: mn,
        strong_negative: sn,
    }
}

/// Given an even-tempered player (personality +10) at trust 50
/// When the manager encourages him
/// Then the odds are 2 strong, 6 mild positive, 2 neutral, 1 mild negative, 0 strong negative
#[test]
fn encourage_weights_for_an_even_tempered_player_at_neutral_trust() {
    assert_eq!(
        weights("morale_talk_p", "encourage", |_| {}),
        bands(2, 6, 2, 1, 0)
    );
}

/// Given an even-tempered player at trust 20
/// When the manager encourages him
/// Then a mild positive reaction is one less likely and a mild negative one more: {2,5,2,2,0}
#[test]
fn encourage_weights_for_a_distrustful_player() {
    assert_eq!(
        weights("morale_talk_p", "encourage", |p| p
            .morale_core
            .manager_trust = 20),
        bands(2, 5, 2, 2, 0)
    );
}

/// Given a composed leader (composure 95, leadership 95, aggression 20: personality +20) at
///       trust 50
/// When the manager encourages him
/// Then he is one likelier to take it well and never mildly badly: {2,7,2,0,0}
#[test]
fn encourage_weights_for_a_composed_leader() {
    assert_eq!(
        weights("morale_talk_p", "encourage", |p| {
            p.attributes.composure = 95;
            p.attributes.leadership = 95;
            p.attributes.aggression = 20;
        }),
        bands(2, 7, 2, 0, 0)
    );
}

/// Given a player at trust 80
/// When the manager promises him playing time
/// Then a strong positive reaction is one likelier and a neutral one gone: {6,4,0,0,0}
#[test]
fn promise_time_weights_at_high_trust() {
    assert_eq!(
        weights("morale_talk_p", "promise_time", |p| p
            .morale_core
            .manager_trust = 80),
        bands(6, 4, 0, 0, 0)
    );
}

/// Given a player at trust 20
/// When the manager promises him playing time
/// Then a strong positive reaction is one less likely and a neutral one likelier: {4,4,2,0,0}
#[test]
fn promise_time_weights_at_low_trust() {
    assert_eq!(
        weights("morale_talk_p", "promise_time", |p| p
            .morale_core
            .manager_trust = 20),
        bands(4, 4, 2, 0, 0)
    );
}

/// Given an even-tempered player at trust 80
/// When the manager tells him to work harder
/// Then he takes it better than the base odds: {3,4,2,1,0}
#[test]
fn work_harder_weights_at_high_trust() {
    assert_eq!(
        weights("morale_talk_p", "work_harder", |p| p
            .morale_core
            .manager_trust = 80),
        bands(3, 4, 2, 1, 0)
    );
}

/// Given an even-tempered player at trust 20
/// When the manager tells him to work harder
/// Then he takes it worse than the base odds: {1,2,2,3,2}
#[test]
fn work_harder_weights_at_low_trust() {
    assert_eq!(
        weights("morale_talk_p", "work_harder", |p| p
            .morale_core
            .manager_trust = 20),
        bands(1, 2, 2, 3, 2)
    );
}

fn with_issue(severity: u8) -> impl FnOnce(&mut domain::player::Player) {
    move |p: &mut domain::player::Player| {
        p.morale_core.unresolved_issue = Some(PlayerIssue {
            category: PlayerIssueCategory::PlayingTime,
            severity,
        });
    }
}

/// Given a player at trust 50 with an unresolved issue of severity 80, and another of severity 60
/// When the manager promises each of them playing time
/// Then the severe issue costs more good odds and adds bad ones: {3,1,2,1,1} against {4,3,2,0,0}
#[test]
fn a_severe_unresolved_issue_shifts_promise_time_weights_negative() {
    assert_eq!(
        (
            weights("morale_talk_p", "promise_time", with_issue(80)),
            weights("morale_talk_p", "promise_time", with_issue(60))
        ),
        (bands(3, 1, 2, 1, 1), bands(4, 3, 2, 0, 0))
    );
}

/// Given an even-tempered player at trust 50 who was encouraged once recently
/// When the manager encourages him again
/// Then each positive reaction is two less likely and a neutral one likelier: {0,4,3,1,0}
#[test]
fn one_recent_identical_talk_costs_two_positive_weight() {
    assert_eq!(
        weights("morale_talk_p", "encourage", |p| {
            p.morale_core.recent_treatment = Some(RecentTreatmentMemory {
                action_key: "morale_talk:encourage".to_string(),
                times_recently_used: 1,
            });
        }),
        bands(0, 4, 3, 1, 0)
    );
}
