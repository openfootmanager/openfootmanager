use super::*;

fn veteran_with_a_june_contract(end: &str) -> Game {
    let mut game = make_game();
    game.players[0].date_of_birth = "1990-01-01".to_string();
    game.players[0].stage_contract_end(Some(end.to_string()));
    game.players[0].morale = 80;
    game.players[0].morale_core.manager_trust = 70;
    game
}

/// Given a veteran contracted until 30 June 2027 in August 2026
/// When the manager offers one year at an acceptable wage
/// Then the deal is refused without a contract record and asks for at least two years.
#[test]
fn a_one_year_renewal_that_keeps_the_same_end_is_refused() {
    let mut game = veteran_with_a_june_contract("2027-06-30");
    let before = game.players[0].movement_history.clone();
    let wage = game.players[0].wage();
    let outcome = propose_renewal(
        &mut game,
        "player-1",
        RenewalOffer {
            weekly_wage: 30_000,
            contract_years: 1,
        },
    )
    .expect("the negotiation returns a decision");
    assert_eq!(outcome.decision, RenewalDecision::Rejected);
    assert_eq!(outcome.suggested_years, Some(2));
    let feedback = outcome
        .feedback
        .expect("a refusal explains the required length");
    assert_eq!(
        feedback.headline_key,
        "be.error.contracts.renewalDoesNotExtend"
    );
    assert_eq!(
        feedback.detail_key.as_deref(),
        Some("playerProfile.renewalMinimumYears")
    );
    assert_eq!(feedback.params.get("years").map(String::as_str), Some("2"));
    assert_eq!(game.players[0].movement_history, before);
    assert_eq!(game.players[0].wage(), wage);
    assert_eq!(game.players[0].contract_end(), Some("2027-06-30"));
}

/// Given that same June 2027 contract in August 2026
/// When the manager offers two years
/// Then the accepted agreement ends on 30 June 2028.
#[test]
fn a_two_year_renewal_extends_the_june_contract() {
    let mut game = veteran_with_a_june_contract("2027-06-30");
    let before = game.players[0].movement_history.len();
    let outcome = propose_renewal(
        &mut game,
        "player-1",
        RenewalOffer {
            weekly_wage: 30_000,
            contract_years: 2,
        },
    )
    .expect("the offer resolves");
    assert_eq!(outcome.decision, RenewalDecision::Accepted);
    assert_eq!(game.players[0].contract_end(), Some("2028-06-30"));
    assert_eq!(game.players[0].movement_history.len(), before + 1);
}

/// Given a June 2027 contract in August 2026
/// When the shared evaluator assesses a one-year offer
/// Then it refuses the non-extending terms before any caller can apply them.
#[test]
fn the_shared_evaluator_refuses_non_extending_terms() {
    let game = veteran_with_a_june_contract("2027-06-30");
    let outcome = evaluate_renewal_offer(
        &game.players[0],
        &game.teams[0],
        game.clock.current_date.date_naive(),
        &RenewalOffer {
            weekly_wage: 30_000,
            contract_years: 1,
        },
    );
    assert_eq!(outcome.decision, RenewalDecision::Rejected);
    assert_eq!(outcome.suggested_years, Some(2));
}

/// Given a veteran contracted until June 2027 and an assistant allowed two years
/// When the assistant agrees the renewal in August 2026
/// Then the shortest extending agreement is two years, ending in June 2028.
#[test]
fn delegated_renewal_chooses_the_shortest_extending_length() {
    let mut game = veteran_with_a_june_contract("2027-06-30");
    game.staff.push(make_assistant_manager());
    let report = delegate_renewals(
        &mut game,
        DelegatedRenewalOptions {
            player_ids: Some(vec!["player-1".to_string()]),
            max_wage_increase_pct: 200,
            max_contract_years: 2,
        },
    )
    .expect("the assistant reports the renewal");
    assert_eq!(report.success_count, 1);
    assert_eq!(report.cases[0].agreed_years, Some(2));
    assert_eq!(game.players[0].contract_end(), Some("2028-06-30"));
}

/// Given a June 2027 contract and an assistant limited to one year
/// When renewal is delegated in August 2026
/// Then the case stalls with the minimum length and no agreement is recorded.
#[test]
fn delegated_renewal_respects_a_limit_that_cannot_extend() {
    let mut game = veteran_with_a_june_contract("2027-06-30");
    game.staff.push(make_assistant_manager());
    let before = game.players[0].movement_history.clone();
    let report = delegate_renewals(
        &mut game,
        DelegatedRenewalOptions {
            player_ids: Some(vec!["player-1".to_string()]),
            max_wage_increase_pct: 200,
            max_contract_years: 1,
        },
    )
    .expect("the assistant reports the stalled case");
    assert_eq!(report.stalled_count, 1);
    assert_eq!(
        report.cases[0].note_key.as_deref(),
        Some("playerProfile.renewalMinimumYears")
    );
    assert_eq!(
        report.cases[0].note_params.get("years").map(String::as_str),
        Some("2")
    );
    assert_eq!(game.players[0].movement_history, before);
    assert_eq!(game.players[0].contract_end(), Some("2027-06-30"));
}

/// Given a contract already running until June 2031 in August 2026
/// When the manager offers the maximum five years
/// Then the offer is refused without recording another contract.
#[test]
fn a_contract_at_the_maximum_end_cannot_be_renewed_without_extension() {
    let mut game = veteran_with_a_june_contract("2031-06-30");
    let before = game.players[0].movement_history.clone();
    let outcome = propose_renewal(
        &mut game,
        "player-1",
        RenewalOffer {
            weekly_wage: 30_000,
            contract_years: 5,
        },
    )
    .expect("an impossible extension is a negotiation refusal");
    assert_eq!(outcome.decision, RenewalDecision::Rejected);
    assert_eq!(outcome.suggested_years, None);
    assert_eq!(game.players[0].movement_history, before);
}
