//! Where a renewal or a free-agent offer lands: signed, countered, refused or blocked, at the
//! edges of the wage, the length and the day.

use super::*;
use domain::player::RenewalSessionOutcome;

fn offer(weekly_wage: u32, contract_years: u32) -> RenewalOffer {
    RenewalOffer {
        weekly_wage,
        contract_years,
    }
}

/// Given the free agent who wants 4,000 a week for three years
/// When he is offered 4,000 a week for two years
/// Then he counters at 4,000 for three years and stays unattached: the wage alone is not enough
#[test]
fn a_free_agent_offered_his_wage_for_too_few_years_is_countered_not_signed() {
    let mut game = make_free_agent_game();

    let outcome = offer_free_agent_contract(&mut game, "free-agent-1", offer(4_000, 2))
        .expect("the offer is answered");

    assert!(matches!(outcome.decision, RenewalDecision::CounterOffer));
    assert_eq!(
        (outcome.suggested_wage, outcome.suggested_years),
        (Some(4_000), Some(3))
    );
    assert_eq!(game.players[0].team_id, None);
}

/// A 30-year-old free agent valued 4,000,000: he expects 26,000 a week, will not hear below
/// 17,000 (85% of the 20,000 his value implies), and is only insulted below 16,900.
fn valuable_veteran_free_agent_game() -> Game {
    let mut game = make_free_agent_game();
    game.players[0].market_value = 4_000_000;
    game.players[0].date_of_birth = "1996-01-01".to_string();
    game
}

/// Given the valuable veteran free agent
/// When he is offered 16,950 a week, between the insult and his minimum
/// Then he turns it down without a counter, and no talks are opened
#[test]
fn a_free_agent_offered_just_below_his_minimum_turns_it_down_without_a_counter() {
    let mut game = valuable_veteran_free_agent_game();

    let outcome = offer_free_agent_contract(&mut game, "free-agent-1", offer(16_950, 2))
        .expect("the offer is answered");

    assert!(matches!(outcome.decision, RenewalDecision::Rejected));
    assert_eq!(outcome.session_status, RenewalSessionStatus::Stalled);
    assert!(!outcome.is_terminal);
    assert!(game.players[0].morale_core.renewal_state.is_none());
}

/// Given the valuable veteran free agent
/// When he is offered exactly his minimum, 17,000 a week
/// Then he counters at the 26,000 he expects
#[test]
fn a_free_agent_offered_exactly_his_minimum_makes_a_counter_offer() {
    let mut game = valuable_veteran_free_agent_game();

    let outcome = offer_free_agent_contract(&mut game, "free-agent-1", offer(17_000, 2))
        .expect("the offer is answered");

    assert!(matches!(outcome.decision, RenewalDecision::CounterOffer));
    assert_eq!(outcome.suggested_wage, Some(26_000));
}

/// Given player-1 agreed a renewal at 14,000 a week for three years this morning
/// When he is offered 15,000 for three years the same day
/// Then the offer is refused: the deal is done, and his wage and contract stay as agreed
#[test]
fn a_second_offer_on_the_day_a_renewal_was_agreed_is_refused() {
    let mut game = make_game();
    let agreed = propose_renewal(&mut game, "player-1", offer(14_000, 3)).expect("first offer");
    assert!(matches!(agreed.decision, RenewalDecision::Accepted));

    let again = propose_renewal(&mut game, "player-1", offer(15_000, 3)).expect("second offer");

    assert!(matches!(again.decision, RenewalDecision::Rejected));
    assert_eq!(again.session_status, RenewalSessionStatus::Agreed);
    assert!(again.is_terminal);
    assert_eq!(
        (game.players[0].wage(), game.players[0].contract_end()),
        (14_000, Some("2029-08-01"))
    );
}

/// Given player-1, who wants 14,000 a week
/// When he is offered 14,000 for five years, the longest contract allowed
/// Then he accepts, and his contract runs to 2031-08-01
#[test]
fn a_five_year_renewal_at_the_expected_wage_is_accepted() {
    let mut game = make_game();

    let outcome = propose_renewal(&mut game, "player-1", offer(14_000, 5)).expect("an answer");

    assert!(matches!(outcome.decision, RenewalDecision::Accepted));
    assert_eq!(game.players[0].contract_end(), Some("2031-08-01"));
}

/// Given player-1 with no renewal talks on record
/// When he is offered a six-year contract
/// Then it is refused as invalid without opening talks: nothing is recorded on him
#[test]
fn an_over_long_renewal_offer_is_refused_without_opening_talks() {
    let mut game = make_game();

    let outcome = propose_renewal(&mut game, "player-1", offer(15_000, 6)).expect("an answer");

    assert!(matches!(outcome.decision, RenewalDecision::Rejected));
    assert!(game.players[0].morale_core.renewal_state.is_none());
}

/// Given player-1 blocked talks after an insulting offer, and never said he wants to leave
/// When the manager clears an exit intent he never had, then makes a fair offer
/// Then the block stands: the fair offer is refused as blocked until 2026-08-31
#[test]
fn clearing_an_exit_intent_that_was_never_set_keeps_an_insulting_offer_block() {
    let mut game = make_game();
    propose_renewal(&mut game, "player-1", offer(1_000, 1)).expect("the insult is answered");

    clear_contract_exit_intent(&mut game, "player-1").expect("nothing to clear");
    let fair = propose_renewal(&mut game, "player-1", offer(15_000, 3)).expect("an answer");

    assert_eq!(fair.session_status, RenewalSessionStatus::Blocked);
    let state = game.players[0]
        .morale_core
        .renewal_state
        .as_ref()
        .expect("the block is on record");
    assert_eq!(
        (
            state.status.clone(),
            state.manager_blocked_until.as_deref(),
            state.last_outcome.clone()
        ),
        (
            RenewalSessionStatus::Blocked,
            Some("2026-08-31"),
            Some(RenewalSessionOutcome::BlockedByManager)
        )
    );
}
