use super::*;

/// Given an accepted purchase
/// When the player registers with the buyer through the public entry point
/// Then the new employment contract ends on 30 June of the agreed year.
#[test]
fn a_permanent_transfer_creates_a_new_contract_with_the_buyers_terms() {
    let mut player = make_player("player-buyer-terms");
    // The seller's deal: a wage that is not a round thousand and an end date a
    // standard contract would never produce, so carrying either across shows.
    player.stage_wage(7_777);
    player.stage_contract_start(Some("2019-07-01".to_string()));
    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);

    make_transfer_bid(&mut game, "player-buyer-terms", 2_000_000)
        .expect("an accepted bid executes the transfer");

    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-buyer-terms")
        .unwrap();
    assert_eq!(player.team_id.as_deref(), Some("team-1"));
    let entry = entry_of_kind(player, PlayerMovementKind::PermanentTransfer);
    assert_eq!(entry.fee, Some(2_000_000));
    let record = entry
        .contract
        .as_ref()
        .expect("a transfer makes a contract with the buying club");
    assert_eq!(record.source, ContractSource::Transfer);
    assert_eq!(record.start.as_deref(), Some("2026-08-01"));
    assert_eq!(
        record.end.as_deref(),
        Some("2029-06-30"),
        "a 26-year-old's standard contract ends on 30 June in the third year after the move"
    );
    assert!(
        record.weekly_wage > 7_777 && record.weekly_wage.is_multiple_of(1_000),
        "the wage is the buyer's standard wage, not the seller's 7,777: {}",
        record.weekly_wage
    );
    assert_eq!(player.contract_end(), Some("2029-06-30"));
    assert_eq!(player.wage(), record.weekly_wage);
}

#[test]
fn a_transfer_never_leaves_the_player_unpaid() {
    // Every bid today carries a wage_offered of 0; that must not become his wage.
    let mut player = make_player("player-zero-offer");
    player.stage_wage(0);
    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);

    make_transfer_bid(&mut game, "player-zero-offer", 2_000_000).expect("the bid executes");

    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-zero-offer")
        .unwrap();
    assert!(
        player.wage() > 0,
        "a zero offered wage means 'use the standard'"
    );
}

/// Given an accepted purchase
/// When the player registers with the buyer through the public entry point
/// Then the new employment contract ends on 30 June of the agreed year.
#[test]
fn a_transfer_registered_after_the_window_dates_its_contract_from_registration() {
    let player = make_player("player-late-contract");
    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.clock.current_date = Utc.with_ymd_and_hms(2026, 12, 20, 12, 0, 0).unwrap();
    game.season_context.transfer_window.status = TransferWindowStatus::Closed;
    game.season_context.transfer_window.opens_on = Some("2027-01-01".to_string());
    make_transfer_bid(&mut game, "player-late-contract", 2_000_000)
        .expect("an accepted closed-window bid schedules registration");

    game.clock.current_date = Utc.with_ymd_and_hms(2027, 1, 1, 12, 0, 0).unwrap();
    game.season_context.transfer_window.status = TransferWindowStatus::Open;
    process_pending_transfer_registrations(&mut game);

    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-late-contract")
        .unwrap();
    let entry = entry_of_kind(player, PlayerMovementKind::PermanentTransfer);
    let record = entry
        .contract
        .as_ref()
        .expect("the registered move made a contract");
    assert_eq!(entry.date, "2027-01-01");
    assert_eq!(record.start.as_deref(), Some("2027-01-01"));
    assert_eq!(record.end.as_deref(), Some("2030-06-30"));
}

#[test]
fn a_refused_bid_appends_nothing() {
    let player = make_player("player-refused-bid");
    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);

    let outcome = make_transfer_bid(&mut game, "player-refused-bid", 1)
        .expect("a lowball bid still gets a decision");

    assert_ne!(
        outcome.decision,
        ofm_core::transfers::TransferNegotiationDecision::Accepted,
        "a lowball bid must not be accepted"
    );
    assert_eq!(game.players[0].team_id.as_deref(), Some("team-2"));
    assert!(game.players[0].movement_history.is_empty());
}

// ---------------------------------------------------------------------------
// The buyer's standard wage goes through the one wage rule.
// ---------------------------------------------------------------------------

#[test]
fn a_bid_the_board_would_not_let_its_buyer_pay_for_is_refused_before_anything_is_agreed() {
    let player = make_player("player-over-policy");
    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    give_depth(&mut game, "team-1");
    game.teams[0].wage_budget = 1;

    let refused = make_transfer_bid(&mut game, "player-over-policy", 2_000_000)
        .expect_err("the board refuses the wage the buyer would have to pay");

    assert!(
        refused.starts_with("be.error.contracts.boardWagePolicy"),
        "{refused}"
    );
    let target = game
        .players
        .iter()
        .find(|p| p.id == "player-over-policy")
        .unwrap();
    assert_eq!(target.team_id.as_deref(), Some("team-2"), "he did not move");
    assert!(target.transfer_offers.is_empty(), "nothing was left agreed");
    assert!(target.movement_history.is_empty());
}

#[test]
fn a_club_below_the_squad_floor_may_be_given_the_contract_the_policy_would_refuse() {
    // The same tight budget, but the buyer has nobody: a club that cannot put a side out
    // has no wage bill worth protecting (the rule renewals and signings follow too).
    let player = make_player("player-floor-waiver");
    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.players
        .retain(|p| p.team_id.as_deref() != Some("team-1"));
    game.teams[0].wage_budget = 1;

    make_transfer_bid(&mut game, "player-floor-waiver", 2_000_000)
        .expect("the floor waives the policy for a club with no squad");

    let target = game
        .players
        .iter()
        .find(|p| p.id == "player-floor-waiver")
        .unwrap();
    assert_eq!(target.team_id.as_deref(), Some("team-1"));
}

#[test]
fn a_scheduled_transfer_whose_buyer_can_no_longer_pay_his_wage_does_not_register() {
    let player = make_player("player-late-refusal");
    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.clock.current_date = Utc.with_ymd_and_hms(2026, 12, 20, 12, 0, 0).unwrap();
    game.season_context.transfer_window.status = TransferWindowStatus::Closed;
    game.season_context.transfer_window.opens_on = Some("2027-01-01".to_string());
    make_transfer_bid(&mut game, "player-late-refusal", 2_000_000)
        .expect("agreed while the buyer could pay");
    // Before the window opens the board tightens the budget, with a full squad.
    give_depth(&mut game, "team-1");
    game.teams[0].wage_budget = 1;

    game.clock.current_date = Utc.with_ymd_and_hms(2027, 1, 1, 12, 0, 0).unwrap();
    game.season_context.transfer_window.status = TransferWindowStatus::Open;
    process_pending_transfer_registrations(&mut game);

    let target = game
        .players
        .iter()
        .find(|p| p.id == "player-late-refusal")
        .unwrap();
    assert_eq!(target.team_id.as_deref(), Some("team-2"), "he did not move");
    assert!(
        !target
            .movement_history
            .iter()
            .any(|e| e.kind == PlayerMovementKind::PermanentTransfer),
        "no transfer was recorded"
    );
}

/// The preview and the deal are the same rule: what the modal says the incoming player
/// will cost is the wage the transfer then gives him, not the wage he was on.
#[test]
fn a_bid_preview_reports_the_wage_the_buyer_would_actually_pay() {
    let mut player = make_player("player-preview");
    player.stage_wage(7_777);
    let game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);

    let projection = ofm_core::transfers::project_transfer_bid_financial_impact(
        &game,
        "player-preview",
        2_000_000,
    )
    .expect("the preview is informational");

    let mut played_out = game.clone();
    make_transfer_bid(&mut played_out, "player-preview", 2_000_000).expect("the bid goes through");
    let paid = i64::from(
        played_out
            .players
            .iter()
            .find(|p| p.id == "player-preview")
            .unwrap()
            .wage(),
    );
    assert_ne!(paid, 7_777, "he is not paid the seller's wage");
    assert_eq!(projection.incoming_player_weekly_wage, paid);
    assert_eq!(
        projection.annual_wage_bill_after,
        projection.annual_wage_bill_before + paid
    );
}

/// Given an AI club that wants a player and can pay the fee,
/// When its board would not let it pay the contract it would have to give him,
/// Then it does not chase him: no offer reaches his club.
///
/// The same setup as `generates_pending_incoming_offer_for_contract_risk_player`, which
/// is the control: with a board that allows the wage, the offer arrives.
#[test]
fn an_ai_club_whose_board_would_refuse_the_wage_does_not_chase_the_player() {
    let mut player = make_user_player("player-unaffordable");
    player.stage_contract_end(Some("2026-09-01".to_string()));
    player.market_value = 1_200_000;
    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.teams[1].finance = 6_000_000;
    game.teams[1].transfer_budget = 3_000_000;
    give_depth(&mut game, "team-2");
    game.teams[1].wage_budget = 1;

    generate_incoming_transfer_offers(&mut game);

    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-unaffordable")
        .unwrap();
    assert!(
        player.transfer_offers.is_empty(),
        "an offer it could not complete reached the player's club"
    );
}
