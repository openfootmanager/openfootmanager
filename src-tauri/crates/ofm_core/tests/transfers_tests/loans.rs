use super::*;

#[test]
fn opening_loan_market_seeds_only_eligible_ai_players() {
    let mut existing_listing = make_player("existing-listing");
    existing_listing.loan_listed = true;
    existing_listing.date_of_birth = "2001-01-01".to_string();

    let mut youngest_eligible = make_player("youngest-eligible");
    youngest_eligible.date_of_birth = "2006-01-01".to_string();

    let mut older_eligible = make_player("older-eligible");
    older_eligible.date_of_birth = "2002-01-01".to_string();

    let mut starter = make_player("starter");
    starter.date_of_birth = "2007-01-01".to_string();

    let mut transfer_listed = make_player("transfer-listed");
    transfer_listed.date_of_birth = "2008-01-01".to_string();
    transfer_listed.transfer_listed = true;

    let mut short_contract = make_player("short-contract");
    short_contract.date_of_birth = "2009-01-01".to_string();
    short_contract.stage_contract_end(Some("2026-09-01".to_string()));

    let mut user_player = make_user_player("user-player");
    user_player.date_of_birth = "2010-01-01".to_string();

    let mut game = make_game_with_player(
        existing_listing,
        vec!["starter".to_string()],
        5_000_000,
        2_000_000,
    );
    game.players.extend([
        youngest_eligible,
        older_eligible,
        starter,
        transfer_listed,
        short_contract,
        user_player,
    ]);

    let seeded = seed_opening_ai_loan_market(&mut game);

    assert_eq!(seeded, 1);
    assert!(
        game.players
            .iter()
            .find(|player| player.id == "existing-listing")
            .unwrap()
            .loan_listed
    );
    assert!(
        game.players
            .iter()
            .find(|player| player.id == "youngest-eligible")
            .unwrap()
            .loan_listed
    );
    for excluded in [
        "older-eligible",
        "starter",
        "transfer-listed",
        "short-contract",
        "user-player",
    ] {
        assert!(
            !game
                .players
                .iter()
                .find(|player| player.id == excluded)
                .unwrap()
                .loan_listed,
            "{excluded} should not be automatically loan-listed"
        );
    }
}

#[test]
fn opening_loan_market_is_idempotent_after_each_ai_club_reaches_target() {
    let mut first = make_player("first");
    first.date_of_birth = "2005-01-01".to_string();
    let mut second = make_player("second");
    second.date_of_birth = "2004-01-01".to_string();
    let mut third = make_player("third");
    third.date_of_birth = "2003-01-01".to_string();

    let mut game = make_game_with_player(first, vec![], 5_000_000, 2_000_000);
    game.players.extend([second, third]);

    assert_eq!(seed_opening_ai_loan_market(&mut game), 2);
    assert_eq!(seed_opening_ai_loan_market(&mut game), 0);
    assert_eq!(
        game.players
            .iter()
            .filter(|player| player.loan_listed)
            .count(),
        2
    );
}

#[test]
fn accepted_loan_offer_moves_player_until_return_date() {
    let mut player = make_player("player-loan-target");
    player.loan_listed = true;
    player.ovr = 62;
    player.potential = 74;
    player.stats.appearances = 2;
    player.stats.minutes_played = 180;
    player.stage_wage(520_000);

    let mut game = make_game_with_player(
        player,
        vec!["player-loan-target".to_string()],
        5_000_000,
        2_000_000,
    );

    let result = make_loan_offer(&mut game, "player-loan-target", "2027-01-01", 100, None)
        .expect("listed player should accept strong loan terms");

    assert_eq!(
        result.decision,
        ofm_core::transfers::LoanOfferDecision::Accepted
    );
    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-loan-target")
        .unwrap();
    assert_eq!(player.team_id.as_deref(), Some("team-1"));
    assert!(!player.loan_listed);
    assert_eq!(player.loan_offers[0].status, LoanOfferStatus::Accepted);
    let loan = player.active_loan.as_ref().expect("active loan");
    assert_eq!(loan.parent_team_id, "team-2");
    assert_eq!(loan.loan_team_id, "team-1");
    assert_eq!(loan.end_date, "2027-01-01");
    assert_eq!(loan.wage_contribution_pct, 100);
    assert_eq!(loan.loan_start_minutes, 180);
    assert_eq!(loan.loan_start_appearances, 2);
    assert_eq!(loan.development_reported_minutes, 180);
    assert_eq!(loan.development_reported_appearances, 2);
    assert!(
        game.teams
            .iter()
            .find(|team| team.id == "team-2")
            .unwrap()
            .starting_xi_ids
            .is_empty()
    );

    game.clock.current_date = Utc.with_ymd_and_hms(2027, 1, 2, 12, 0, 0).unwrap();
    process_loan_returns(&mut game);

    let returned_player = game
        .players
        .iter()
        .find(|player| player.id == "player-loan-target")
        .unwrap();
    assert_eq!(returned_player.team_id.as_deref(), Some("team-2"));
    assert!(returned_player.active_loan.is_none());
    assert!(returned_player.movement_history.iter().any(|entry| {
        entry.kind == PlayerMovementKind::LoanStart
            && entry.from_team_id.as_deref() == Some("team-2")
            && entry.to_team_id.as_deref() == Some("team-1")
    }));
    assert!(returned_player.movement_history.iter().any(|entry| {
        entry.kind == PlayerMovementKind::LoanReturn
            && entry.from_team_id.as_deref() == Some("team-1")
            && entry.to_team_id.as_deref() == Some("team-2")
    }));
}

#[test]
fn accepted_closed_window_loan_is_registered_when_the_window_opens() {
    let mut player = make_player("player-scheduled-loan");
    player.loan_listed = true;
    player.ovr = 62;
    player.potential = 74;
    player.stage_wage(520_000);

    let mut game = make_game_with_player(
        player,
        vec!["player-scheduled-loan".to_string()],
        5_000_000,
        2_000_000,
    );
    game.clock.current_date = Utc.with_ymd_and_hms(2026, 12, 20, 12, 0, 0).unwrap();
    game.season_context.transfer_window.status = TransferWindowStatus::Closed;
    game.season_context.transfer_window.opens_on = Some("2027-01-01".to_string());

    let result = make_loan_offer(&mut game, "player-scheduled-loan", "2027-06-30", 100, None)
        .expect("strong terms should be agreed outside the registration window");

    assert_eq!(result.decision, LoanOfferDecision::Accepted);
    let scheduled_player = game
        .players
        .iter()
        .find(|player| player.id == "player-scheduled-loan")
        .unwrap();
    assert_eq!(scheduled_player.team_id.as_deref(), Some("team-2"));
    assert!(scheduled_player.active_loan.is_none());
    assert!(!scheduled_player.loan_listed);
    assert_eq!(
        scheduled_player.loan_offers[0].status,
        LoanOfferStatus::PendingRegistration
    );
    assert_eq!(scheduled_player.loan_offers[0].start_date, "2027-01-01");
    assert!(
        game.news
            .iter()
            .all(|article| !article.id.starts_with("loan_news_")),
        "scheduled agreement should not be reported as a completed loan yet"
    );

    game.clock.current_date = Utc.with_ymd_and_hms(2027, 1, 1, 12, 0, 0).unwrap();
    game.season_context.transfer_window.status = TransferWindowStatus::Open;
    process_pending_loan_registrations(&mut game);

    let registered_player = game
        .players
        .iter()
        .find(|player| player.id == "player-scheduled-loan")
        .unwrap();
    assert_eq!(registered_player.team_id.as_deref(), Some("team-1"));
    assert_eq!(
        registered_player.loan_offers[0].status,
        LoanOfferStatus::Accepted
    );
    // Registering runs through `execute_loan`, which withdraws every live loan offer on the
    // player — including this one. The agreement must not be left wearing that closure stamp.
    assert!(
        registered_player.loan_offers[0].closed_on.is_none(),
        "a registered agreement must not carry a closure date"
    );
    let active_loan = registered_player.active_loan.as_ref().unwrap();
    assert_eq!(active_loan.start_date, "2027-01-01");
    assert_eq!(active_loan.end_date, "2027-06-30");

    let article = game
        .news
        .iter()
        .find(|article| article.id == "loan_news_player-scheduled-loan_team-2_team-1_2027-01-01")
        .expect("loan registration should create a completed loan news article");
    assert_eq!(article.date, "2027-01-01");
    assert_eq!(article.category, NewsCategory::TransferRumour);
    assert_eq!(
        article.headline_key.as_deref(),
        Some("be.news.loanMove.headline")
    );
    assert_eq!(article.body_key.as_deref(), Some("be.news.loanMove.body"));
    assert_eq!(
        article.team_ids,
        vec!["team-2".to_string(), "team-1".to_string()]
    );
    assert_eq!(
        article.player_ids,
        vec!["player-scheduled-loan".to_string()]
    );
    assert_eq!(
        article.i18n_params.get("fromTeam").map(String::as_str),
        Some("Seller FC")
    );
    assert_eq!(
        article.i18n_params.get("toTeam").map(String::as_str),
        Some("User FC")
    );
    assert_eq!(
        article.i18n_params.get("endDate").map(String::as_str),
        Some("2027-06-30")
    );
}

#[test]
fn accepted_closed_window_loan_blocks_permanent_bid_before_registration() {
    let mut player = make_player("player-scheduled-lock");
    player.loan_listed = true;
    player.market_value = 500_000;
    player.stage_wage(20_000);

    let mut game = make_game_with_player(
        player,
        vec!["player-scheduled-lock".to_string()],
        5_000_000,
        2_000_000,
    );
    game.season_context.transfer_window.status = TransferWindowStatus::Closed;
    game.season_context.transfer_window.opens_on = Some("2027-01-01".to_string());

    make_loan_offer(&mut game, "player-scheduled-lock", "2027-06-30", 100, None)
        .expect("closed-window loan should schedule registration");

    game.clock.current_date = Utc.with_ymd_and_hms(2027, 1, 1, 12, 0, 0).unwrap();
    game.season_context.transfer_window.status = TransferWindowStatus::Open;

    let error = make_transfer_bid(&mut game, "player-scheduled-lock", 1_000_000)
        .expect_err("pending loan registration should reserve the player");

    assert_eq!(error, "be.error.transfers.playerAlreadyLoaned");
    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-scheduled-lock")
        .expect("player should exist");
    assert!(player.active_loan.is_none());
    assert_eq!(
        player.loan_offers[0].status,
        LoanOfferStatus::PendingRegistration
    );
    assert_eq!(player.team_id.as_deref(), Some("team-2"));
}

#[test]
fn accepted_post_window_loan_is_scheduled_for_the_next_window() {
    let mut player = make_player("player-next-window-loan");
    player.loan_listed = true;
    player.stage_contract_end(Some("2028-07-31".to_string()));
    player.ovr = 62;
    player.potential = 74;
    player.stage_wage(520_000);

    let mut game = make_game_with_player(
        player,
        vec!["player-next-window-loan".to_string()],
        5_000_000,
        2_000_000,
    );
    game.clock.current_date = Utc.with_ymd_and_hms(2026, 9, 15, 12, 0, 0).unwrap();
    game.season_context.transfer_window.status = TransferWindowStatus::Closed;
    game.season_context.transfer_window.opens_on = Some("2027-07-02".to_string());
    game.season_context.transfer_window.closes_on = Some("2027-08-31".to_string());
    game.season_context.transfer_window.days_until_opens = Some(290);
    game.season_context.transfer_window.days_remaining = None;

    let result = make_loan_offer(
        &mut game,
        "player-next-window-loan",
        "2028-06-30",
        100,
        None,
    )
    .expect("post-window loan should schedule against the next opening date");

    assert_eq!(result.decision, LoanOfferDecision::Accepted);
    let scheduled_player = game
        .players
        .iter()
        .find(|player| player.id == "player-next-window-loan")
        .unwrap();
    assert_eq!(scheduled_player.team_id.as_deref(), Some("team-2"));
    assert!(scheduled_player.active_loan.is_none());
    assert_eq!(
        scheduled_player.loan_offers[0].status,
        LoanOfferStatus::PendingRegistration
    );
    assert_eq!(scheduled_player.loan_offers[0].start_date, "2027-07-02");
    assert_eq!(scheduled_player.loan_offers[0].end_date, "2028-06-30");
}

/// Agrees an AI club's loan for a user player while the window is closed, sets the borrower's
/// wage budget, then opens the window and runs registration.
fn register_scheduled_incoming_loan(borrower_wage_budget: i64) -> Game {
    let mut player = make_user_player("player-ai-borrow");
    player.stage_wage(520_000);
    let mut offer = make_pending_incoming_loan_offer("loan-offer-ai", 75, None);
    // Dated on the day it is answered, so stale-offer expiry leaves it alone.
    offer.date = "2026-12-20".to_string();
    offer.start_date = "2027-01-01".to_string();
    offer.end_date = "2027-06-30".to_string();
    player.loan_offers.push(offer);

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.teams[1].wage_budget = 2_000_000;
    game.clock.current_date = Utc.with_ymd_and_hms(2026, 12, 20, 12, 0, 0).unwrap();
    game.season_context.transfer_window.status = TransferWindowStatus::Closed;
    game.season_context.transfer_window.opens_on = Some("2027-01-01".to_string());

    respond_to_loan_offer(&mut game, "player-ai-borrow", "loan-offer-ai", true)
        .expect("the user should be able to agree the loan outside the window");
    assert_eq!(
        find_player(&game, "player-ai-borrow").loan_offers[0].status,
        LoanOfferStatus::PendingRegistration
    );

    game.teams[1].wage_budget = borrower_wage_budget;
    game.clock.current_date = Utc.with_ymd_and_hms(2027, 1, 1, 12, 0, 0).unwrap();
    game.season_context.transfer_window.status = TransferWindowStatus::Open;
    process_pending_loan_registrations(&mut game);
    game
}

#[test]
fn scheduled_loan_to_an_ai_club_registers_when_the_borrower_can_afford_it() {
    let game = register_scheduled_incoming_loan(2_000_000);

    let player = find_player(&game, "player-ai-borrow");
    assert_eq!(player.team_id.as_deref(), Some("team-2"));
    assert_eq!(player.loan_offers[0].status, LoanOfferStatus::Accepted);
}

#[test]
fn scheduled_loan_to_an_ai_club_fails_registration_when_the_borrower_cannot_afford_it() {
    // The borrower's 75% share of a 520k wage is 390k: well past a 50k budget's soft cap.
    let game = register_scheduled_incoming_loan(50_000);

    let player = find_player(&game, "player-ai-borrow");
    assert_eq!(player.team_id.as_deref(), Some("team-1"));
    assert!(player.active_loan.is_none());
    assert_eq!(player.loan_offers[0].status, LoanOfferStatus::Withdrawn);
    assert_eq!(
        player.loan_offers[0].closed_on.as_deref(),
        Some("2027-01-01")
    );
}

#[test]
fn loan_offer_rejects_end_date_after_player_contract() {
    let mut player = make_player("player-short-contract-loan");
    player.loan_listed = true;
    player.stage_contract_end(Some("2026-12-01".to_string()));
    player.ovr = 62;
    player.potential = 74;
    player.stats.appearances = 0;
    player.stage_wage(520_000);

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);

    let error = make_loan_offer(
        &mut game,
        "player-short-contract-loan",
        "2027-01-01",
        100,
        None,
    )
    .expect_err("loan should not outlive the player's contract");

    assert_eq!(error, "be.error.transfers.invalidLoanEndDate");
    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-short-contract-loan")
        .unwrap();
    assert!(player.active_loan.is_none());
    assert!(player.loan_offers.is_empty());
}

#[test]
fn loan_offer_rejects_terms_that_exceed_user_wage_budget() {
    let mut player = make_player("player-loan-wage-budget");
    player.loan_listed = true;
    player.stage_wage(120_000);
    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.teams[0].wage_budget = 50_000;

    let error = make_loan_offer(
        &mut game,
        "player-loan-wage-budget",
        "2027-01-01",
        100,
        None,
    )
    .expect_err("loan should be blocked by wage budget");

    assert_eq!(error, "be.error.contracts.boardWagePolicy?budget=50000");
    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-loan-wage-budget")
        .expect("player should exist");
    assert!(player.active_loan.is_none());
    assert!(player.loan_offers.is_empty());
}

#[test]
fn loan_offer_counts_existing_loan_wages_against_borrower_budget() {
    let mut player = make_player("player-loan-existing-wage-budget");
    player.loan_listed = true;
    player.stage_wage(20_000);
    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.teams[0].wage_budget = 100_000;

    let mut existing_loan = make_player("existing-user-loan");
    existing_loan.team_id = Some("team-1".to_string());
    existing_loan.stage_wage(100_000);
    existing_loan.active_loan = Some(ActiveLoan {
        parent_team_id: "team-2".to_string(),
        loan_team_id: "team-1".to_string(),
        start_date: "2026-07-01".to_string(),
        end_date: "2027-06-30".to_string(),
        wage_contribution_pct: 100,
        buy_option_fee: None,
        loan_start_minutes: 0,
        loan_start_appearances: 0,
        development_reported_minutes: 0,
        development_reported_appearances: 0,
    });
    game.players.push(existing_loan);

    assert_eq!(calc_wages(&game, "team-1"), 100_000);

    let error = make_loan_offer(
        &mut game,
        "player-loan-existing-wage-budget",
        "2027-01-01",
        100,
        None,
    )
    .expect_err("existing loan wages should count against borrower affordability");

    assert_eq!(error, "be.error.contracts.boardWagePolicy?budget=100000");
    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-loan-existing-wage-budget")
        .expect("player should exist");
    assert!(player.active_loan.is_none());
    assert!(player.loan_offers.is_empty());
}

#[test]
fn loan_offer_does_not_require_cash_to_cover_wage_share() {
    let mut player = make_player("player-loan-cash");
    player.loan_listed = true;
    player.ovr = 62;
    player.potential = 74;
    player.stats.appearances = 0;
    player.stage_wage(120_000);
    let mut game = make_game_with_player(player, vec![], 50_000, 2_000_000);
    game.teams[0].wage_budget = 500_000;

    let result = make_loan_offer(&mut game, "player-loan-cash", "2027-01-01", 100, None)
        .expect("wage share is a weekly envelope check, not a cash gate");
    assert_eq!(result.decision, LoanOfferDecision::Accepted);
}

#[test]
fn loan_buy_option_can_be_exercised_from_active_user_loan() {
    let mut player = make_player("player-loan-to-buy");
    player.loan_listed = true;
    player.ovr = 62;
    player.potential = 74;
    player.stats.appearances = 0;
    player.stage_wage(520_000);
    // The parent club's contract, signed long before the loan. A loan must leave it
    // alone and buying the player must replace it, so a value that a `None` start
    // could not distinguish from "set by the buy".
    player.stage_contract_start(Some("2019-07-01".to_string()));

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    attach_transfer_log_league(&mut game);

    let result = make_loan_offer(
        &mut game,
        "player-loan-to-buy",
        "2027-01-01",
        40,
        Some(1_250_000),
    )
    .expect("serious loan-to-buy terms should be accepted");

    assert_eq!(
        result.decision,
        ofm_core::transfers::LoanOfferDecision::Accepted
    );
    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-loan-to-buy")
        .unwrap();
    assert_eq!(
        player
            .active_loan
            .as_ref()
            .and_then(|loan| loan.buy_option_fee),
        Some(1_250_000)
    );
    assert!(player.movement_history.iter().any(|entry| {
        entry.kind == PlayerMovementKind::LoanStart
            && entry.from_team_name.as_deref() == Some("Seller FC")
            && entry.to_team_name.as_deref() == Some("User FC")
            && entry.fee.is_none()
            && entry.loan_end_date.as_deref() == Some("2027-01-01")
    }));
    // Out on loan the player is still the parent club's, on the parent's contract.
    assert_eq!(
        player.contract_start(),
        Some("2019-07-01"),
        "a loan must not touch the parent club's contract"
    );

    let buyer_finance_before = game
        .teams
        .iter()
        .find(|team| team.id == "team-1")
        .unwrap()
        .finance;
    let seller_finance_before = game
        .teams
        .iter()
        .find(|team| team.id == "team-2")
        .unwrap()
        .finance;

    exercise_loan_buy_option(&mut game, "player-loan-to-buy")
        .expect("active loan buy option should be exercisable");

    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-loan-to-buy")
        .unwrap();
    assert_eq!(player.team_id.as_deref(), Some("team-1"));
    assert!(player.active_loan.is_none());
    // Buying the player is a new agreement with the buying club, dated the day it is
    // done, not the parent club's 2019 contract carried across.
    assert_eq!(
        player.contract_start().map(str::to_string),
        Some(game.clock.current_date.format("%Y-%m-%d").to_string()),
        "a loan-to-buy starts a new contract on the day of the purchase"
    );
    assert!(player.movement_history.iter().any(|entry| {
        entry.kind == PlayerMovementKind::LoanStart
            && entry.from_team_name.as_deref() == Some("Seller FC")
            && entry.to_team_name.as_deref() == Some("User FC")
    }));
    assert!(player.movement_history.iter().any(|entry| {
        entry.kind == PlayerMovementKind::LoanToBuy
            && entry.from_team_id.as_deref() == Some("team-2")
            && entry.to_team_id.as_deref() == Some("team-1")
            && entry.fee == Some(1_250_000)
    }));
    assert_eq!(
        game.teams
            .iter()
            .find(|team| team.id == "team-1")
            .unwrap()
            .finance,
        buyer_finance_before - 1_250_000
    );
    assert_eq!(
        game.teams
            .iter()
            .find(|team| team.id == "team-2")
            .unwrap()
            .finance,
        seller_finance_before + 1_250_000
    );
    assert_eq!(
        game.league
            .as_ref()
            .and_then(|league| league.transfer_log.last())
            .map(|transfer| transfer.fee),
        Some(1_250_000)
    );
    assert!(game.messages.iter().any(|message| {
        message.id.starts_with("loan_buy_option_player-loan-to-buy")
            && message.context.player_id.as_deref() == Some("player-loan-to-buy")
    }));
}

#[test]
fn loan_development_report_is_generated_for_parent_club() {
    let mut player = make_user_player("player-loan-development");
    player.team_id = Some("team-2".to_string());
    player.ovr = 60;
    player.potential = 75;
    player.stats.minutes_played = 360;
    player.active_loan = Some(ActiveLoan {
        parent_team_id: "team-1".to_string(),
        loan_team_id: "team-2".to_string(),
        start_date: "2026-08-01".to_string(),
        end_date: "2027-01-01".to_string(),
        wage_contribution_pct: 75,
        buy_option_fee: None,
        loan_start_minutes: 0,
        loan_start_appearances: 0,
        development_reported_minutes: 0,
        development_reported_appearances: 0,
    });

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.teams[1].reputation = 700;
    game.clock.current_date = Utc.with_ymd_and_hms(2026, 8, 31, 12, 0, 0).unwrap();

    process_loan_development_reports(&mut game);

    let report = game
        .messages
        .iter()
        .find(|message| message.id == "loan_development_player-loan-development_2026-08-31")
        .expect("loan development report");
    assert_eq!(report.category, MessageCategory::Training);
    assert_eq!(
        report.context.player_id.as_deref(),
        Some("player-loan-development")
    );
    assert_eq!(
        report.i18n_params.get("team").map(String::as_str),
        Some("Seller FC")
    );
    assert_eq!(
        report.i18n_params.get("attributeGains").map(String::as_str),
        Some("3")
    );
}

#[test]
fn loan_development_only_counts_minutes_since_last_report() {
    let mut player = make_user_player("player-loan-development-delta");
    player.team_id = Some("team-2".to_string());
    player.ovr = 60;
    player.potential = 75;
    player.stats.minutes_played = 900;
    player.stats.appearances = 8;
    player.active_loan = Some(ActiveLoan {
        parent_team_id: "team-1".to_string(),
        loan_team_id: "team-2".to_string(),
        start_date: "2026-08-01".to_string(),
        end_date: "2027-01-01".to_string(),
        wage_contribution_pct: 75,
        buy_option_fee: None,
        loan_start_minutes: 900,
        loan_start_appearances: 8,
        development_reported_minutes: 900,
        development_reported_appearances: 8,
    });

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.teams[1].reputation = 700;
    game.clock.current_date = Utc.with_ymd_and_hms(2026, 8, 31, 12, 0, 0).unwrap();

    process_loan_development_reports(&mut game);

    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-loan-development-delta")
        .unwrap();
    assert_eq!(player.attributes.shooting, 60);
    assert_eq!(player.ovr, 60);
    assert_eq!(
        player
            .active_loan
            .as_ref()
            .map(|loan| loan.development_reported_minutes),
        Some(900)
    );
    let first_report = game
        .messages
        .iter()
        .find(|message| message.id == "loan_development_player-loan-development-delta_2026-08-31")
        .expect("first loan development report");
    assert_eq!(
        first_report
            .i18n_params
            .get("attributeGains")
            .map(String::as_str),
        Some("0")
    );

    let player = game
        .players
        .iter_mut()
        .find(|player| player.id == "player-loan-development-delta")
        .unwrap();
    player.stats.minutes_played = 1_080;
    player.stats.appearances = 10;
    game.clock.current_date = Utc.with_ymd_and_hms(2026, 9, 30, 12, 0, 0).unwrap();

    process_loan_development_reports(&mut game);

    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-loan-development-delta")
        .unwrap();
    assert_eq!(player.attributes.shooting, 61);
    assert_eq!(
        player
            .active_loan
            .as_ref()
            .map(|loan| loan.development_reported_minutes),
        Some(1_080)
    );

    game.clock.current_date = Utc.with_ymd_and_hms(2027, 1, 2, 12, 0, 0).unwrap();
    process_loan_returns(&mut game);

    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-loan-development-delta")
        .unwrap();
    assert_eq!(player.attributes.shooting, 61);
    assert!(player.active_loan.is_none());
}

#[test]
fn ai_loan_club_can_exercise_buy_option_when_loan_expires() {
    let mut player = make_user_player("player-ai-loan-option");
    player.team_id = Some("team-2".to_string());
    player.market_value = 1_000_000;
    player.ovr = 64;
    player.potential = 75;
    player.stats.appearances = 12;
    player.stats.minutes_played = 1_080;
    player.active_loan = Some(ActiveLoan {
        parent_team_id: "team-1".to_string(),
        loan_team_id: "team-2".to_string(),
        start_date: "2026-08-01".to_string(),
        end_date: "2027-01-01".to_string(),
        wage_contribution_pct: 60,
        buy_option_fee: Some(1_200_000),
        loan_start_minutes: 0,
        loan_start_appearances: 0,
        development_reported_minutes: 0,
        development_reported_appearances: 0,
    });

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.teams[1].finance = 6_000_000;
    game.teams[1].transfer_budget = 3_000_000;
    attach_transfer_log_league(&mut game);
    game.clock.current_date = Utc.with_ymd_and_hms(2027, 1, 2, 12, 0, 0).unwrap();

    process_loan_returns(&mut game);

    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-ai-loan-option")
        .unwrap();
    assert_eq!(player.team_id.as_deref(), Some("team-2"));
    assert!(player.active_loan.is_none());
    assert!(player.movement_history.iter().any(|entry| {
        entry.kind == PlayerMovementKind::LoanToBuy
            && entry.from_team_id.as_deref() == Some("team-1")
            && entry.to_team_id.as_deref() == Some("team-2")
            && entry.fee == Some(1_200_000)
    }));
    assert_eq!(
        game.teams
            .iter()
            .find(|team| team.id == "team-1")
            .unwrap()
            .finance,
        6_200_000
    );
    assert_eq!(
        game.teams
            .iter()
            .find(|team| team.id == "team-2")
            .unwrap()
            .finance,
        4_800_000
    );
    assert_eq!(
        game.league
            .as_ref()
            .and_then(|league| league.transfer_log.last())
            .map(|transfer| (
                transfer.from_team_id.as_str(),
                transfer.to_team_id.as_str(),
                transfer.fee,
            )),
        Some(("team-1", "team-2", 1_200_000))
    );
    assert!(game.messages.iter().any(|message| {
        message
            .id
            .starts_with("loan_buy_option_player-ai-loan-option")
            && message.context.player_id.as_deref() == Some("player-ai-loan-option")
    }));
}

#[test]
fn ai_loan_club_cannot_exercise_buy_option_when_window_is_closed() {
    let mut player = make_user_player("player-ai-loan-option-closed");
    player.team_id = Some("team-2".to_string());
    player.market_value = 1_000_000;
    player.ovr = 64;
    player.potential = 75;
    player.stats.appearances = 12;
    player.stats.minutes_played = 1_080;
    player.active_loan = Some(ActiveLoan {
        parent_team_id: "team-1".to_string(),
        loan_team_id: "team-2".to_string(),
        start_date: "2026-08-01".to_string(),
        end_date: "2027-01-01".to_string(),
        wage_contribution_pct: 60,
        buy_option_fee: Some(1_200_000),
        loan_start_minutes: 0,
        loan_start_appearances: 0,
        development_reported_minutes: 0,
        development_reported_appearances: 0,
    });

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.season_context.transfer_window.status = TransferWindowStatus::Closed;
    game.teams[1].finance = 6_000_000;
    game.teams[1].transfer_budget = 3_000_000;
    attach_transfer_log_league(&mut game);
    game.clock.current_date = Utc.with_ymd_and_hms(2027, 1, 2, 12, 0, 0).unwrap();

    process_loan_returns(&mut game);

    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-ai-loan-option-closed")
        .unwrap();
    assert_eq!(player.team_id.as_deref(), Some("team-1"));
    assert!(player.active_loan.is_none());
    assert!(
        !player
            .movement_history
            .iter()
            .any(|entry| entry.kind == PlayerMovementKind::LoanToBuy)
    );
    assert_eq!(
        game.teams
            .iter()
            .find(|team| team.id == "team-1")
            .unwrap()
            .finance,
        5_000_000
    );
    assert_eq!(
        game.teams
            .iter()
            .find(|team| team.id == "team-2")
            .unwrap()
            .finance,
        6_000_000
    );
    assert!(
        game.league
            .as_ref()
            .map(|league| league.transfer_log.is_empty())
            .unwrap_or(true)
    );
}

#[test]
fn ai_loan_club_does_not_exercise_buy_option_from_pre_loan_minutes() {
    let mut player = make_user_player("player-ai-pre-loan-option");
    player.team_id = Some("team-2".to_string());
    player.market_value = 1_000_000;
    player.ovr = 64;
    player.potential = 75;
    player.stats.appearances = 12;
    player.stats.minutes_played = 1_080;
    player.active_loan = Some(ActiveLoan {
        parent_team_id: "team-1".to_string(),
        loan_team_id: "team-2".to_string(),
        start_date: "2026-08-01".to_string(),
        end_date: "2027-01-01".to_string(),
        wage_contribution_pct: 60,
        buy_option_fee: Some(1_200_000),
        loan_start_minutes: 1_080,
        loan_start_appearances: 12,
        development_reported_minutes: 1_080,
        development_reported_appearances: 12,
    });

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.teams[1].finance = 6_000_000;
    game.teams[1].transfer_budget = 3_000_000;
    attach_transfer_log_league(&mut game);
    game.clock.current_date = Utc.with_ymd_and_hms(2027, 1, 2, 12, 0, 0).unwrap();

    process_loan_returns(&mut game);

    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-ai-pre-loan-option")
        .unwrap();
    assert_eq!(player.team_id.as_deref(), Some("team-1"));
    assert!(player.active_loan.is_none());
    assert!(
        !player
            .movement_history
            .iter()
            .any(|entry| entry.kind == PlayerMovementKind::LoanToBuy)
    );
    assert_eq!(
        game.teams
            .iter()
            .find(|team| team.id == "team-1")
            .unwrap()
            .finance,
        5_000_000
    );
    assert_eq!(
        game.teams
            .iter()
            .find(|team| team.id == "team-2")
            .unwrap()
            .finance,
        6_000_000
    );
    assert!(
        game.league
            .as_ref()
            .and_then(|league| league.transfer_log.last())
            .is_none()
    );
}

#[test]
fn incoming_loan_offer_is_generated_for_loan_listed_user_player() {
    let mut player = make_user_player("player-user-loan");
    player.loan_listed = true;
    player.ovr = 68;
    player.potential = 80;
    player.stats.appearances = 0;
    player.stage_wage(260_000);

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);

    generate_incoming_transfer_offers(&mut game);

    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-user-loan")
        .unwrap();
    assert_eq!(player.loan_offers.len(), 1);
    assert_eq!(player.loan_offers[0].status, LoanOfferStatus::Pending);
    assert_eq!(player.loan_offers[0].from_team_id, "team-2");
    assert!(game.messages.iter().any(|message| {
        message.id.starts_with("loan_offer_")
            && message.context.player_id.as_deref() == Some("player-user-loan")
    }));
}

#[test]
fn incoming_loan_offers_are_capped_per_user_player_per_day() {
    let mut player = make_user_player("player-user-loan-flood");
    player.loan_listed = true;
    player.ovr = 68;
    player.potential = 80;
    player.stats.appearances = 0;
    player.stage_wage(260_000);

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.teams
        .push(make_ai_team("team-3", "Buyer C", 10_000_000, 5_000_000));
    game.teams
        .push(make_ai_team("team-4", "Buyer D", 10_000_000, 5_000_000));

    generate_incoming_transfer_offers(&mut game);

    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-user-loan-flood")
        .unwrap();
    let pending_offers = player
        .loan_offers
        .iter()
        .filter(|offer| offer.status == LoanOfferStatus::Pending)
        .count();
    let loan_messages = game
        .messages
        .iter()
        .filter(|message| {
            message.id.starts_with("loan_offer_")
                && message.context.player_id.as_deref() == Some("player-user-loan-flood")
        })
        .count();

    assert_eq!(pending_offers, 1);
    assert_eq!(loan_messages, 1);
}

#[test]
fn incoming_loan_offer_does_not_block_permanent_transfer_interest() {
    let mut loan_player = make_user_player("player-user-loan-mixed-market");
    loan_player.loan_listed = true;
    loan_player.ovr = 68;
    loan_player.potential = 80;
    loan_player.stats.appearances = 0;
    loan_player.stage_wage(260_000);

    let mut contract_risk_player = make_user_player("player-contract-risk-mixed-market");
    contract_risk_player.stage_contract_end(Some("2026-09-01".to_string()));
    contract_risk_player.market_value = 1_200_000;

    let mut game = make_game_with_player(loan_player, vec![], 5_000_000, 2_000_000);
    game.players.push(contract_risk_player);
    game.teams[1].finance = 6_000_000;
    game.teams[1].transfer_budget = 3_000_000;

    generate_incoming_transfer_offers(&mut game);

    let loan_player = game
        .players
        .iter()
        .find(|player| player.id == "player-user-loan-mixed-market")
        .unwrap();
    assert_eq!(loan_player.loan_offers.len(), 1);
    assert_eq!(loan_player.loan_offers[0].from_team_id, "team-2");

    let contract_risk_player = game
        .players
        .iter()
        .find(|player| player.id == "player-contract-risk-mixed-market")
        .unwrap();
    assert_eq!(contract_risk_player.transfer_offers.len(), 1);
    assert_eq!(
        contract_risk_player.transfer_offers[0].status,
        TransferOfferStatus::Pending
    );
    assert_eq!(
        contract_risk_player.transfer_offers[0].from_team_id,
        "team-2"
    );
}

/// Given an affordable AI borrower, when its incoming loan is accepted, then the player moves and both clubs retain the agreed wage split.
#[test]
fn accepting_incoming_loan_offer_moves_user_player_to_borrowing_club() {
    let mut player = make_user_player("player-incoming-loan");
    player.loan_listed = true;
    player.stage_wage(520_000);
    player.loan_offers.push(LoanOffer {
        id: "loan-offer-1".to_string(),
        from_team_id: "team-2".to_string(),
        parent_team_id: "team-1".to_string(),
        start_date: "2026-08-01".to_string(),
        end_date: "2027-01-01".to_string(),
        wage_contribution_pct: 75,
        buy_option_fee: Some(1_100_000),
        last_manager_wage_contribution_pct: None,
        last_manager_end_date: None,
        last_manager_buy_option_fee: None,
        negotiation_round: 1,
        suggested_wage_contribution_pct: None,
        suggested_end_date: None,
        suggested_buy_option_fee: None,
        status: LoanOfferStatus::Pending,
        date: "2026-08-01".to_string(),
        closed_on: None,
    });

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.teams[1].wage_budget = 500_000;
    game.teams[0].starting_xi_ids = vec!["player-incoming-loan".to_string()];

    respond_to_loan_offer(&mut game, "player-incoming-loan", "loan-offer-1", true)
        .expect("incoming loan offer should be acceptable");

    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-incoming-loan")
        .unwrap();
    assert_eq!(player.team_id.as_deref(), Some("team-2"));
    assert_eq!(player.loan_offers[0].status, LoanOfferStatus::Accepted);
    assert_eq!(
        player
            .active_loan
            .as_ref()
            .map(|loan| loan.wage_contribution_pct),
        Some(75)
    );
    assert_eq!(
        player
            .active_loan
            .as_ref()
            .and_then(|loan| loan.buy_option_fee),
        Some(1_100_000)
    );
    assert!(game.teams[0].starting_xi_ids.is_empty());
    assert_eq!(calc_wages(&game, "team-1"), 130_000);
    assert_eq!(calc_wages(&game, "team-2"), 390_000);
}

/// Given an affordable AI borrower and an acceptable counter, when terms settle, then the exact loan terms register immediately.
#[test]
fn countering_incoming_loan_offer_can_execute_accepted_terms() {
    let mut player = make_user_player("player-counter-loan-accepted");
    player.loan_listed = true;
    player.stage_wage(520_000);
    player.ovr = 68;
    player.potential = 78;
    player
        .loan_offers
        .push(make_pending_incoming_loan_offer("loan-counter-1", 65, None));

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.teams[1].wage_budget = 500_000;
    game.teams[1].finance = 6_000_000;

    let outcome = counter_loan_offer(
        &mut game,
        "player-counter-loan-accepted",
        "loan-counter-1",
        "2027-01-01",
        85,
        Some(1_200_000),
    )
    .expect("counter should be accepted");

    assert_eq!(outcome.decision, LoanOfferDecision::Accepted);
    assert!(outcome.is_terminal);
    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-counter-loan-accepted")
        .unwrap();
    assert_eq!(player.team_id.as_deref(), Some("team-2"));
    assert_eq!(player.loan_offers[0].status, LoanOfferStatus::Accepted);
    assert_eq!(
        player.loan_offers[0].last_manager_wage_contribution_pct,
        Some(85)
    );
    assert_eq!(
        player
            .active_loan
            .as_ref()
            .map(|loan| (loan.wage_contribution_pct, loan.buy_option_fee)),
        Some((85, Some(1_200_000)))
    );
}

#[test]
fn countering_incoming_loan_offer_can_keep_talks_live_with_suggested_terms() {
    let mut player = make_user_player("player-counter-loan-live");
    player.loan_listed = true;
    player.stage_wage(520_000);
    player.ovr = 60;
    player.potential = 62;
    player.loan_offers.push(make_pending_incoming_loan_offer(
        "loan-counter-live",
        40,
        None,
    ));

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);

    let outcome = counter_loan_offer(
        &mut game,
        "player-counter-loan-live",
        "loan-counter-live",
        "2027-01-01",
        70,
        None,
    )
    .expect("counter should keep negotiation live");

    assert_eq!(outcome.decision, LoanOfferDecision::CounterOffer);
    assert!(!outcome.is_terminal);
    assert_eq!(outcome.suggested_wage_contribution_pct, Some(60));
    assert_eq!(outcome.suggested_end_date.as_deref(), Some("2027-01-01"));
    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-counter-loan-live")
        .unwrap();
    assert_eq!(player.team_id.as_deref(), Some("team-1"));
    assert_eq!(player.loan_offers[0].status, LoanOfferStatus::Pending);
    assert_eq!(player.loan_offers[0].wage_contribution_pct, 60);
    assert_eq!(
        player.loan_offers[0].suggested_wage_contribution_pct,
        Some(60)
    );
    assert_eq!(
        player.loan_offers[0].last_manager_wage_contribution_pct,
        Some(70)
    );
}

#[test]
fn countering_incoming_loan_offer_rejects_terms_that_do_not_improve() {
    let mut player = make_user_player("player-counter-loan-no-improve");
    player.loan_listed = true;
    player.loan_offers.push(make_pending_incoming_loan_offer(
        "loan-counter-no-improve",
        75,
        Some(1_000_000),
    ));

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);

    let error = counter_loan_offer(
        &mut game,
        "player-counter-loan-no-improve",
        "loan-counter-no-improve",
        "2027-01-01",
        70,
        Some(1_000_000),
    )
    .expect_err("counter should improve the incoming offer");

    assert_eq!(error, "be.error.transfers.loanCounterMustImproveTerms");
    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-counter-loan-no-improve")
        .unwrap();
    assert_eq!(player.loan_offers[0].status, LoanOfferStatus::Pending);
    assert_eq!(player.loan_offers[0].wage_contribution_pct, 75);
}

#[test]
fn incoming_loan_offer_rejects_end_date_after_player_contract() {
    let mut player = make_user_player("player-incoming-short-contract-loan");
    player.loan_listed = true;
    player.stage_contract_end(Some("2026-12-01".to_string()));
    player.stage_wage(520_000);
    player.loan_offers.push(LoanOffer {
        id: "loan-offer-short-contract".to_string(),
        from_team_id: "team-2".to_string(),
        parent_team_id: "team-1".to_string(),
        start_date: "2026-08-01".to_string(),
        end_date: "2027-01-01".to_string(),
        wage_contribution_pct: 75,
        buy_option_fee: None,
        last_manager_wage_contribution_pct: None,
        last_manager_end_date: None,
        last_manager_buy_option_fee: None,
        negotiation_round: 1,
        suggested_wage_contribution_pct: None,
        suggested_end_date: None,
        suggested_buy_option_fee: None,
        status: LoanOfferStatus::Pending,
        date: "2026-08-01".to_string(),
        closed_on: None,
    });

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);

    let error = respond_to_loan_offer(
        &mut game,
        "player-incoming-short-contract-loan",
        "loan-offer-short-contract",
        true,
    )
    .expect_err("incoming loan should not outlive the player's contract");

    assert_eq!(error, "be.error.transfers.invalidLoanEndDate");
    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-incoming-short-contract-loan")
        .unwrap();
    assert!(player.active_loan.is_none());
    assert_eq!(player.team_id.as_deref(), Some("team-1"));
    assert_eq!(player.loan_offers[0].status, LoanOfferStatus::Pending);
}

#[test]
fn permanent_bid_is_rejected_for_active_loan_player() {
    let mut player = make_player("player-active-loan");
    player.team_id = Some("team-2".to_string());
    player.active_loan = Some(ActiveLoan {
        parent_team_id: "team-3".to_string(),
        loan_team_id: "team-2".to_string(),
        start_date: "2026-08-01".to_string(),
        end_date: "2027-01-01".to_string(),
        wage_contribution_pct: 75,
        buy_option_fee: None,
        loan_start_minutes: 0,
        loan_start_appearances: 0,
        development_reported_minutes: 0,
        development_reported_appearances: 0,
    });

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);

    let error = make_transfer_bid(&mut game, "player-active-loan", 1_000_000)
        .expect_err("active loan player should not be purchasable from loan club");

    assert_eq!(error, "be.error.transfers.playerAlreadyLoaned");
}

/// The cap counts both deal types together, so a club cannot switch from a loan approach to a
/// permanent bid to claim a second slot on the same player.
#[test]
fn a_club_holding_a_pending_loan_offer_does_not_also_open_a_transfer_bid() {
    // The player has to be worth a permanent bid, or the club would never reach the shortlist
    // and the test would pass without the rule it is supposed to be checking. He is no longer
    // loan-listed, so the loan path cannot fire and mask the result by consuming the club's
    // action for the day — the standing loan offer is a leftover from when he was listed.
    let mut player = make_user_player("player-user-loan-crosstype");
    player.loan_listed = false;
    player.transfer_listed = true;
    player.stage_contract_end(Some("2026-09-01".to_string()));
    player.market_value = 1_200_000;
    player
        .loan_offers
        .push(make_pending_incoming_loan_offer("existing-loan", 75, None));

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.teams[1].finance = 8_000_000;
    game.teams[1].transfer_budget = 5_000_000;

    // team-2 already holds the loan approach, so it must not open a second, permanent one.
    let holder = game.teams[1].id.clone();
    assert_eq!(
        find_player(&game, "player-user-loan-crosstype").loan_offers[0].from_team_id,
        holder
    );

    generate_incoming_transfer_offers(&mut game);

    let player = find_player(&game, "player-user-loan-crosstype");
    assert!(
        player
            .transfer_offers
            .iter()
            .all(|offer| offer.from_team_id != holder),
        "{holder} holds a pending loan offer and must not also open a transfer bid"
    );
}

/// The other half of the same rule: the cap counts both deal types, so live loan talks consume
/// the budget a permanent bid would otherwise use.
#[test]
fn pending_loan_offers_count_towards_the_same_cap_as_transfer_bids() {
    let mut player = make_user_player("player-user-crosstype-cap");
    player.loan_listed = true;
    player.transfer_listed = true;
    player.stage_contract_end(Some("2026-09-01".to_string()));
    player.market_value = 1_200_000;
    for index in 0..3 {
        let mut offer =
            make_pending_incoming_loan_offer(&format!("existing-loan-{index}"), 75, None);
        offer.from_team_id = format!("team-holder-{index}");
        player.loan_offers.push(offer);
    }

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    for index in 0..3 {
        game.teams.push(make_ai_team(
            &format!("team-holder-{index}"),
            &format!("Holder {index}"),
            10_000_000,
            5_000_000,
        ));
    }
    game.teams[1].finance = 8_000_000;
    game.teams[1].transfer_budget = 5_000_000;

    generate_incoming_transfer_offers(&mut game);

    let player = find_player(&game, "player-user-crosstype-cap");
    let pending = player
        .transfer_offers
        .iter()
        .filter(|offer| offer.status == TransferOfferStatus::Pending)
        .count();
    assert_eq!(
        pending, 0,
        "three live loan approaches already fill the queue, so no transfer bid should arrive"
    );
}

/// `date` is the arrival date and is rewritten whenever a club re-opens talks, so it cannot
/// answer "when did this close". Expiry has to record that separately.
#[test]
fn an_expired_loan_offer_records_the_date_it_closed() {
    let mut game = make_loan_pileup_game("player-user-loan-closed-on", 2);

    generate_incoming_transfer_offers(&mut game);
    let arrival = game.clock.current_date.format("%Y-%m-%d").to_string();

    game.clock.advance_days(20);
    generate_incoming_transfer_offers(&mut game);
    let expiry_day = game.clock.current_date.format("%Y-%m-%d").to_string();

    let expired = find_player(&game, "player-user-loan-closed-on")
        .loan_offers
        .iter()
        .find(|offer| offer.status == LoanOfferStatus::Withdrawn)
        .expect("the first offer should have expired");

    assert_eq!(expired.date, arrival, "arrival date must be preserved");
    assert_eq!(
        expired.closed_on.as_deref(),
        Some(expiry_day.as_str()),
        "expiry should record when talks cooled"
    );
}

/// Closing an offer must not move `date`. Expiry never did, but the manager-driven paths used to
/// stamp `date = today` on the way out, which quietly destroyed the arrival date on exactly the
/// offers whose history the UI wants to show ("received 27 Dec, talks cooled 10 Jan").
#[test]
fn rejecting_a_loan_offer_preserves_the_arrival_date() {
    let mut player = make_user_player("player-reject-keeps-arrival");
    player.loan_listed = true;
    player
        .loan_offers
        .push(make_pending_incoming_loan_offer("loan-offer-1", 75, None));

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.clock.advance_days(6);
    let rejected_on = game.clock.current_date.format("%Y-%m-%d").to_string();

    respond_to_loan_offer(
        &mut game,
        "player-reject-keeps-arrival",
        "loan-offer-1",
        false,
    )
    .expect("rejecting an incoming loan offer should succeed");

    let offer = &find_player(&game, "player-reject-keeps-arrival").loan_offers[0];
    assert_eq!(offer.status, LoanOfferStatus::Rejected);
    assert_eq!(offer.date, "2026-08-01", "arrival date must be preserved");
    assert_eq!(offer.closed_on.as_deref(), Some(rejected_on.as_str()));
}

/// Selling through an exercised buy option has to leave the parent club as well off as selling the
/// same player permanently. The permanent path credits the current-season envelope as well as the
/// balance; the buy-option path credited only the balance.
#[test]
fn a_buy_option_sale_credits_the_parent_club_the_same_as_a_permanent_sale() {
    let mut player = make_user_player("player-option-proceeds");
    player.team_id = Some("team-2".to_string());
    player.market_value = 1_000_000;
    player.ovr = 64;
    player.potential = 75;
    player.stats.appearances = 12;
    player.stats.minutes_played = 1_080;
    player.active_loan = Some(ActiveLoan {
        parent_team_id: "team-1".to_string(),
        loan_team_id: "team-2".to_string(),
        start_date: "2026-08-01".to_string(),
        end_date: "2027-01-01".to_string(),
        wage_contribution_pct: 60,
        buy_option_fee: Some(1_200_000),
        loan_start_minutes: 0,
        loan_start_appearances: 0,
        development_reported_minutes: 0,
        development_reported_appearances: 0,
    });

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.teams[1].finance = 6_000_000;
    game.teams[1].transfer_budget = 3_000_000;
    attach_transfer_log_league(&mut game);
    game.clock.current_date = Utc.with_ymd_and_hms(2027, 1, 2, 12, 0, 0).unwrap();

    let parent_before = game.teams.iter().find(|team| team.id == "team-1").unwrap();
    let finance_before = parent_before.finance;
    let budget_before = parent_before.transfer_budget;

    process_loan_returns(&mut game);

    let parent = game.teams.iter().find(|team| team.id == "team-1").unwrap();
    assert_eq!(
        parent.finance,
        finance_before + 1_200_000,
        "the fee should reach the parent club's balance"
    );
    assert_eq!(
        parent.transfer_budget,
        budget_before + 1_200_000,
        "and its spending room, as a permanent sale would"
    );
}

#[test]
fn squad_floor_a_loan_that_would_leave_the_parent_club_short_is_refused() {
    let mut player = make_player("player-floor-loan");
    player.loan_listed = true;
    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    leave_club_at_the_forward_floor(&mut game, "team-2");

    let result = make_loan_offer(&mut game, "player-floor-loan", "2027-01-01", 100, None);

    assert_eq!(result.err().as_deref(), Some(WOULD_LEAVE_SHORT_OF_FORWARDS));
    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-floor-loan")
        .unwrap();
    assert_eq!(player.team_id.as_deref(), Some("team-2"));
    assert!(player.active_loan.is_none());
    assert!(player.loan_offers.iter().all(|offer| {
        offer.status != LoanOfferStatus::Accepted
            && offer.status != LoanOfferStatus::PendingRegistration
    }));
}

/// Given the player's club at its forward minimum and an offer id nobody made,
/// when the manager accepts it, the answer is that the offer is not pending —
/// the floor is only asked about a departure that could actually happen.
#[test]
fn squad_floor_a_stale_loan_offer_is_refused_as_stale_even_at_the_floor() {
    let mut player = make_user_player("player-stale-loan");
    player.loan_listed = true;
    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    leave_club_at_the_forward_floor(&mut game, "team-1");

    let result = respond_to_loan_offer(&mut game, "player-stale-loan", "no-such-offer", true);

    assert_eq!(
        result.err().as_deref(),
        Some("be.error.transfers.offerNotPending")
    );
}

#[test]
fn a_loan_out_and_back_writes_movements_but_no_contract() {
    let mut player = make_player("player-loan-no-contract");
    player.loan_listed = true;
    player.ovr = 62;
    player.potential = 74;
    player.stage_wage(520_000);
    player.stage_contract_start(Some("2019-07-01".to_string()));
    let mut game = make_game_with_player(
        player,
        vec!["player-loan-no-contract".to_string()],
        5_000_000,
        2_000_000,
    );

    make_loan_offer(
        &mut game,
        "player-loan-no-contract",
        "2027-01-01",
        100,
        None,
    )
    .expect("a strong loan offer is agreed");
    game.clock.current_date = Utc.with_ymd_and_hms(2027, 1, 2, 12, 0, 0).unwrap();
    process_loan_returns(&mut game);

    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-loan-no-contract")
        .unwrap();
    let start = entry_of_kind(player, PlayerMovementKind::LoanStart);
    let back = entry_of_kind(player, PlayerMovementKind::LoanReturn);
    assert!(start.contract.is_none(), "a loan is not a contract");
    assert!(back.contract.is_none(), "neither is the return");
    assert_eq!(player.wage(), 520_000, "the parent's terms are untouched");
    assert_eq!(player.contract_start(), Some("2019-07-01"));
}

#[test]
fn a_loan_to_buy_creates_a_new_contract_with_the_buyers_terms() {
    let mut player = make_player("player-buy-terms");
    player.loan_listed = true;
    player.ovr = 62;
    player.potential = 74;
    player.stats.appearances = 0;
    player.stage_wage(520_000);
    player.stage_contract_start(Some("2019-07-01".to_string()));
    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    attach_transfer_log_league(&mut game);
    make_loan_offer(
        &mut game,
        "player-buy-terms",
        "2027-01-01",
        40,
        Some(1_250_000),
    )
    .expect("serious loan-to-buy terms should be accepted");

    exercise_loan_buy_option(&mut game, "player-buy-terms").expect("the buy option is exercisable");

    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-buy-terms")
        .unwrap();
    let entry = entry_of_kind(player, PlayerMovementKind::LoanToBuy);
    let record = entry
        .contract
        .as_ref()
        .expect("buying the player makes a contract with the buyer");
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let three_years_on = game
        .clock
        .current_date
        .date_naive()
        .checked_add_months(chrono::Months::new(36))
        .expect("three years on exists")
        .format("%Y-%m-%d")
        .to_string();
    assert_eq!(record.source, ContractSource::Transfer);
    assert_eq!(record.start.as_deref(), Some(today.as_str()));
    assert_eq!(
        record.end.as_deref(),
        Some(three_years_on.as_str()),
        "three years from the purchase, not the seller's 2028-06-30"
    );
    assert!(
        record.weekly_wage > 520_000,
        "the buyer's standard wage, not the parent's 520,000 carried across: {}",
        record.weekly_wage
    );
    assert_eq!(player.contract_end(), Some(three_years_on.as_str()));
}

#[test]
fn a_loan_to_buy_is_judged_at_the_whole_wage_not_the_loan_share() {
    let mut player = make_player("player-buy-over-policy");
    player.loan_listed = true;
    player.ovr = 62;
    player.potential = 74;
    player.stats.appearances = 0;
    player.stage_wage(520_000);
    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    attach_transfer_log_league(&mut game);
    give_depth(&mut game, "team-1");
    make_loan_offer(
        &mut game,
        "player-buy-over-policy",
        "2027-01-01",
        40,
        Some(1_250_000),
    )
    .expect("the loan fits the budget it was agreed under");
    // On loan he costs the borrower 40% of his wage (about 208,000 a week). A budget of
    // 300,000 lets the board pay a share of the wage he would be bought on (40% of about
    // 570,000) but not the whole of it, so only a verdict that counts the whole wage
    // refuses.
    assert!(calc_wages(&game, "team-1") < 300_000);
    game.teams[0].wage_budget = 300_000;

    let refused = exercise_loan_buy_option(&mut game, "player-buy-over-policy")
        .expect_err("the whole wage is over the board's policy");

    assert!(
        refused.starts_with("be.error.contracts.boardWagePolicy"),
        "{refused}"
    );
    let target = game
        .players
        .iter()
        .find(|p| p.id == "player-buy-over-policy")
        .unwrap();
    assert!(target.active_loan.is_some(), "he is still on loan");
    assert_eq!(game.teams[0].finance, 5_000_000, "no fee was paid");
}
