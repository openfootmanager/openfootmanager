//! What the renewal panel reads back to the manager: the tension bar and the headline.

use super::*;
use chrono::NaiveDate;

fn offer(weekly_wage: u32, contract_years: u32) -> RenewalOffer {
    RenewalOffer {
        weekly_wage,
        contract_years,
    }
}

fn feedback_on(player: &Player, offer: RenewalOffer) -> domain::negotiation::NegotiationFeedback {
    evaluate_renewal_offer(
        player,
        &make_team(),
        NaiveDate::from_ymd_opt(2026, 8, 1).unwrap(),
        &offer,
    )
    .feedback
    .expect("every answer carries feedback")
}

/// Given player-1 with 75 days left (24), morale 50 (12), manager trust 35 (12) and a value of
///       1,000,000 (6)
/// When an offer is weighed
/// Then the tension is 76: the base 22 plus each pressure once
#[test]
fn renewal_tension_adds_contract_urgency_morale_trust_and_value_pressures() {
    let mut player = make_player();
    player.morale = 50;
    player.morale_core.manager_trust = 35;
    player.market_value = 1_000_000;

    assert_eq!(feedback_on(&player, offer(15_000, 3)).tension, 76);
}

/// Given a content player-1 the manager has a fair relationship with, at 150 and then 300 days
///       from the end of his contract
/// When an offer is weighed
/// Then the tension is 38 and then 30: the urgency eases from 16 to 8 as the end moves away
#[test]
fn renewal_tension_eases_as_the_contract_end_moves_further_away() {
    let tension_with_end = |contract_end: &str| {
        let mut player = make_player();
        player.morale_core.manager_trust = 50;
        player.stage_contract_end(Some(contract_end.to_string()));
        feedback_on(&player, offer(15_000, 3)).tension
    };

    assert_eq!(
        (
            tension_with_end("2026-12-29"),
            tension_with_end("2027-05-28")
        ),
        (38, 30)
    );
}

fn headline(outcome: &ofm_core::contracts::RenewalOutcome) -> &str {
    &outcome
        .feedback
        .as_ref()
        .expect("every answer carries feedback")
        .headline_key
}

/// Given player-1, who wants 14,000 a week for three years
/// When he accepts that offer in the first round
/// Then the headline is the plain acceptance, not the late one
#[test]
fn a_first_round_renewal_acceptance_has_the_plain_accepted_headline() {
    let mut game = make_game();

    let outcome = propose_renewal(&mut game, "player-1", offer(14_000, 3)).expect("an answer");

    assert!(matches!(outcome.decision, RenewalDecision::Accepted));
    assert_eq!(
        headline(&outcome),
        "playerProfile.renewalFeedbackAcceptedHeadline"
    );
}

/// Given player-1 countered 13,000 a week for three years this morning
/// When he is offered his 14,000 later the same day
/// Then he accepts in the second round, under the late-acceptance headline
#[test]
fn a_renewal_agreed_in_the_second_round_of_the_day_has_the_late_acceptance_headline() {
    let mut game = make_game();
    let first = propose_renewal(&mut game, "player-1", offer(13_000, 3)).expect("first offer");
    assert!(matches!(first.decision, RenewalDecision::CounterOffer));

    let second = propose_renewal(&mut game, "player-1", offer(14_000, 3)).expect("second offer");

    assert!(matches!(second.decision, RenewalDecision::Accepted));
    assert_eq!(
        headline(&second),
        "playerProfile.renewalFeedbackAcceptedLateHeadline"
    );
}

/// Given player-1, on 12,000 and wanting 14,000, with tension 46
/// When he counters a 13,000 offer in the first round
/// Then the headline is firm, not tense and not an acceptance
#[test]
fn a_first_round_counter_offer_below_tension_seventy_is_firm() {
    let mut game = make_game();

    let outcome = propose_renewal(&mut game, "player-1", offer(13_000, 3)).expect("an answer");

    assert!(matches!(outcome.decision, RenewalDecision::CounterOffer));
    assert_eq!(
        headline(&outcome),
        "playerProfile.renewalFeedbackFirmHeadline"
    );
}

/// Given player-1 trusts the manager little (28), so tension is only 58
/// When he turns down a fair 14,000 offer over the relationship
/// Then the headline is tense: a refusal over the relationship reads as tense at any tension
#[test]
fn a_renewal_refused_over_the_relationship_reads_as_tense() {
    let mut game = make_game();
    game.players[0].morale_core.manager_trust = 28;

    let outcome = propose_renewal(&mut game, "player-1", offer(14_000, 3)).expect("an answer");

    assert!(matches!(outcome.decision, RenewalDecision::Rejected));
    assert_eq!(
        headline(&outcome),
        "playerProfile.renewalFeedbackTenseHeadline"
    );
}
