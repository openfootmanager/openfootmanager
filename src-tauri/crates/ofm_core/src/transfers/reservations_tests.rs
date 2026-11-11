use super::tests::{acceptance_game, incoming_transfer};
use super::*;

fn preview_player(game: &mut Game) -> String {
    let mut player = game.players[0].clone();
    player.id = "next-purchase".into();
    player.transfer_offers.clear();
    let id = player.id.clone();
    game.players.push(player);
    id
}

/// Given a closed window, when a purchase is agreed, then its fee reduces available budget without moving cash.
#[test]
fn agreement_reserves_budget_without_paying_cash() {
    let mut game = acceptance_game("team2", true);
    let next = preview_player(&mut game);
    make_transfer_bid(&mut game, "player-award", 2_000_001).unwrap();
    assert_eq!(game.teams[0].finance, 5_000_000);
    assert_eq!(game.teams[0].transfer_budget, 5_000_000);
    let preview = project_transfer_bid_financial_impact(&game, &next, 1).unwrap();
    assert_eq!(preview.transfer_budget_before, 2_999_999);
}

/// Given an agreed purchase, when a second offer exceeds uncommitted budget, then it is refused before mutating offers.
#[test]
fn reserved_money_cannot_fund_another_offer() {
    let mut game = acceptance_game("team2", true);
    let next = preview_player(&mut game);
    make_transfer_bid(&mut game, "player-award", 2_000_001).unwrap();
    assert_eq!(
        make_transfer_bid(&mut game, &next, 3_000_000).unwrap_err(),
        "be.error.transfers.transferBudgetCommitted"
    );
    assert!(
        game.players
            .iter()
            .find(|p| p.id == next)
            .unwrap()
            .transfer_offers
            .is_empty()
    );
}

/// Given the user's agreed sale, when the AI buyer previews another purchase, then the same reserved-budget rule reduces its available funds.
#[test]
fn ai_agreements_reserve_the_same_budget() {
    let mut game = acceptance_game("team1", true);
    let next = preview_player(&mut game);
    let offer = incoming_transfer(&mut game);
    respond_to_offer(&mut game, "player-award", &offer, true).unwrap();
    game.manager.hire("team2".into());
    assert_eq!(
        project_transfer_bid_financial_impact(&game, &next, 1)
            .unwrap()
            .transfer_budget_before,
        4_400_000
    );
}

/// Given a reserved purchase, when its agreement is withdrawn, then its full budget becomes available again.
#[test]
fn withdrawing_an_agreement_releases_its_reservation() {
    let mut game = acceptance_game("team2", true);
    let next = preview_player(&mut game);
    make_transfer_bid(&mut game, "player-award", 2_000_001).unwrap();
    close_transfer_offer(
        &mut game.players[0].transfer_offers[0],
        TransferOfferStatus::Withdrawn,
        "2026-02-01",
    );
    assert_eq!(
        project_transfer_bid_financial_impact(&game, &next, 1)
            .unwrap()
            .transfer_budget_before,
        5_000_000
    );
    assert_eq!(game.teams[0].finance, 5_000_000);
}

/// Given an agreement consuming the whole budget, when the window opens twice, then its own reservation never blocks it and cash is charged once.
#[test]
fn registration_consumes_its_reservation_and_pays_once() {
    let mut game = acceptance_game("team2", true);
    let next = preview_player(&mut game);
    game.teams[0].transfer_budget = 2_000_001;
    make_transfer_bid(&mut game, "player-award", 2_000_001).unwrap();
    game.clock.current_date = chrono::DateTime::parse_from_rfc3339("2026-07-02T12:00:00Z")
        .unwrap()
        .into();
    game.season_context.transfer_window.status = TransferWindowStatus::Open;
    process_pending_transfer_registrations(&mut game);
    process_pending_transfer_registrations(&mut game);
    assert_eq!(game.players[0].team_id.as_deref(), Some("team1"));
    assert_eq!(game.teams[0].finance, 2_999_999);
    assert_eq!(
        project_transfer_bid_financial_impact(&game, &next, 0)
            .unwrap()
            .transfer_budget_before,
        0
    );
}

/// Given an AI buyer whose budget is already committed, when the user accepts another deferred sale, then it is refused before marking the deal agreed.
#[test]
fn an_ai_buyer_cannot_agree_a_sale_beyond_available_budget() {
    let mut game = acceptance_game("team1", true);
    let next = preview_player(&mut game);
    game.teams[1].transfer_budget = 1_000_000;
    let first = incoming_transfer(&mut game);
    respond_to_offer(&mut game, "player-award", &first, true).unwrap();
    let mut offer = game.players[0].transfer_offers[0].clone();
    offer.id = "second-offer".into();
    offer.status = TransferOfferStatus::Pending;
    game.players
        .iter_mut()
        .find(|p| p.id == next)
        .unwrap()
        .transfer_offers
        .push(offer);
    assert_eq!(
        respond_to_offer(&mut game, &next, "second-offer", true).unwrap_err(),
        "be.error.transfers.transferBudgetCommitted"
    );
    let second = game.players.iter().find(|p| p.id == next).unwrap();
    assert_eq!(
        second.transfer_offers[0].status,
        TransferOfferStatus::Pending
    );
    assert_eq!(game.teams[1].finance, 5_000_000);
    assert_eq!(committed_transfer_fees(&game, "team2"), 600_000);
}

/// Given a reserved purchase whose player becomes unavailable, when registration fails, then its fee is released without payment.
#[test]
fn failed_registration_releases_the_reserved_fee() {
    let mut game = acceptance_game("team2", true);
    let next = preview_player(&mut game);
    make_transfer_bid(&mut game, "player-award", 2_000_001).unwrap();
    game.players[0].team_id = None;
    game.clock.current_date = chrono::DateTime::parse_from_rfc3339("2026-07-02T12:00:00Z")
        .unwrap()
        .into();
    game.season_context.transfer_window.status = TransferWindowStatus::Open;
    process_pending_transfer_registrations(&mut game);
    assert_eq!(
        game.players[0].transfer_offers[0].status,
        TransferOfferStatus::Withdrawn
    );
    assert_eq!(game.teams[0].finance, 5_000_000);
    assert_eq!(
        project_transfer_bid_financial_impact(&game, &next, 1)
            .unwrap()
            .transfer_budget_before,
        5_000_000
    );
}

/// Given a missed agreed registration and the window now closed, when registrations are processed, then the deal lapses and its commitment is released.
#[test]
fn a_missed_window_releases_the_agreed_fee() {
    let mut game = acceptance_game("team2", true);
    let next = preview_player(&mut game);
    make_transfer_bid(&mut game, "player-award", 2_000_001).unwrap();
    game.clock.current_date = chrono::DateTime::parse_from_rfc3339("2026-09-02T12:00:00Z")
        .unwrap()
        .into();
    process_pending_transfer_registrations(&mut game);
    assert_eq!(
        game.players[0].transfer_offers[0].status,
        TransferOfferStatus::Withdrawn
    );
    assert_eq!(game.teams[0].finance, 5_000_000);
    assert_eq!(
        project_transfer_bid_financial_impact(&game, &next, 1)
            .unwrap()
            .transfer_budget_before,
        5_000_000
    );
}

/// Given an AI buyer with 600k committed from a 1m budget, when a second sale is countered at 650001, then the counter cannot become an agreement.
#[test]
fn an_ai_counter_cannot_spend_an_existing_commitment() {
    let mut game = acceptance_game("team1", true);
    let next = preview_player(&mut game);
    game.teams[1].transfer_budget = 1_000_000;
    let offer_id = incoming_transfer(&mut game);
    respond_to_offer(&mut game, "player-award", &offer_id, true).unwrap();
    let mut offer = game.players[0].transfer_offers[0].clone();
    offer.id = "countered-offer".into();
    offer.status = TransferOfferStatus::Pending;
    game.players
        .iter_mut()
        .find(|p| p.id == next)
        .unwrap()
        .transfer_offers
        .push(offer);
    let outcome = counter_offer(&mut game, &next, "countered-offer", 650_001).unwrap();
    assert_ne!(outcome.decision, TransferNegotiationDecision::Accepted);
    assert_eq!(committed_transfer_fees(&game, "team2"), 600_000);
    assert_eq!(game.teams[1].finance, 5_000_000);
}

/// Given a user purchase reserved for a future window and an active loan, when its buy option exceeds remaining budget, then the option is refused without disturbing either deal.
#[test]
fn a_loan_buy_option_cannot_spend_reserved_money() {
    let mut game = acceptance_game("team2", true);
    let loanee = preview_player(&mut game);
    make_transfer_bid(&mut game, "player-award", 2_000_001).unwrap();
    execute_loan(
        &mut game,
        &loanee,
        "team2",
        "team1",
        "2026-01-12",
        "2026-12-31",
        50,
        Some(3_000_000),
    )
    .unwrap();
    game.season_context.transfer_window.status = TransferWindowStatus::Open;
    assert_eq!(
        exercise_loan_buy_option(&mut game, &loanee).unwrap_err(),
        "be.error.transfers.transferBudgetCommitted"
    );
    assert_eq!(game.teams[0].finance, 5_000_000);
    assert_eq!(committed_transfer_fees(&game, "team1"), 2_000_001);
    assert!(
        game.players
            .iter()
            .find(|p| p.id == loanee)
            .unwrap()
            .active_loan
            .is_some()
    );
}

/// Given an AI buyer with all funds committed, when the market sweeps a desirable player, then no additional fee offer is issued; a control without the reservation still bids.
#[test]
fn the_ai_market_does_not_offer_reserved_money() {
    let mut control = super::tests::make_game();
    evaluate_transfer_market(&mut control);
    assert!(!control.players[0].transfer_offers.is_empty());
    let mut game = super::tests::make_game();
    let mut reserved = game.players[0].clone();
    reserved.id = "reserved-player".into();
    reserved.transfer_offers.push(serde_json::from_value(serde_json::json!({
        "id":"reserved-offer", "from_team_id":"team2", "fee":5_000_000,
        "wage_offered":0, "status":"PendingRegistration", "date":"2026-01-12", "registration_date":"2026-07-02"
    })).unwrap());
    game.players.push(reserved);
    evaluate_transfer_market(&mut game);
    assert!(game.players[0].transfer_offers.is_empty());
    assert_eq!(game.teams[1].finance, 5_000_000);
}
