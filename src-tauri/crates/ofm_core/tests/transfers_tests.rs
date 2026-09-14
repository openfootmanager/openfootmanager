use chrono::{TimeZone, Utc};
use domain::league::League;
use domain::manager::Manager;
use domain::message::MessageCategory;
use domain::news::{NewsArticle, NewsCategory};
use domain::player::{
    ActiveLoan, LoanOffer, LoanOfferStatus, Player, PlayerAttributes, PlayerIssueCategory,
    PlayerMovementKind, Position, TransferOffer, TransferOfferStatus,
};
use domain::season::TransferWindowStatus;
use domain::team::Team;
use ofm_core::clock::GameClock;
use ofm_core::finances::calc_annual_wages;
use ofm_core::game::Game;
use ofm_core::transfers::{
    LoanOfferDecision, TransferNegotiationDecision, counter_loan_offer, counter_offer,
    evaluate_transfer_market, exercise_loan_buy_option, generate_incoming_transfer_offers,
    make_loan_offer, make_transfer_bid, process_loan_development_reports, process_loan_returns,
    process_pending_loan_registrations, process_pending_transfer_registrations,
    respond_to_loan_offer, respond_to_offer, seed_opening_ai_loan_market,
};

fn default_attrs() -> PlayerAttributes {
    PlayerAttributes {
        pace: 60,
        stamina: 60,
        strength: 60,
        agility: 60,
        passing: 60,
        shooting: 60,
        tackling: 60,
        dribbling: 60,
        defending: 60,
        positioning: 60,
        vision: 60,
        decisions: 60,
        composure: 60,
        aggression: 60,
        teamwork: 60,
        leadership: 60,
        handling: 30,
        reflexes: 30,
        aerial: 60,
    }
}

fn make_player(id: &str) -> Player {
    let mut player = Player::new(
        id.to_string(),
        format!("{}. Test", id),
        format!("{} Test", id),
        "2000-01-01".to_string(),
        "England".to_string(),
        Position::Forward,
        default_attrs(),
    );
    player.team_id = Some("team-2".to_string());
    player.contract_end = Some("2028-06-30".to_string());
    player.market_value = 1_000_000;
    player.morale = 70;
    player
}

fn make_user_player(id: &str) -> Player {
    let mut player = make_player(id);
    player.team_id = Some("team-1".to_string());
    player
}

fn make_pending_incoming_offer(id: &str, fee: u64) -> TransferOffer {
    TransferOffer {
        id: id.to_string(),
        from_team_id: "team-2".to_string(),
        fee,
        wage_offered: 0,
        last_manager_fee: None,
        negotiation_round: 1,
        suggested_counter_fee: None,
        status: TransferOfferStatus::Pending,
        date: "2026-08-01".to_string(),
        registration_date: None,
        closed_on: None,
    }
}

fn make_pending_incoming_loan_offer(
    id: &str,
    wage_contribution_pct: u8,
    buy_option_fee: Option<u64>,
) -> LoanOffer {
    LoanOffer {
        id: id.to_string(),
        from_team_id: "team-2".to_string(),
        parent_team_id: "team-1".to_string(),
        start_date: "2026-08-01".to_string(),
        end_date: "2027-01-01".to_string(),
        wage_contribution_pct,
        buy_option_fee,
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
    }
}

fn make_user_team(finance: i64, transfer_budget: i64) -> Team {
    let mut team = Team::new(
        "team-1".to_string(),
        "User FC".to_string(),
        "USR".to_string(),
        "England".to_string(),
        "London".to_string(),
        "User Ground".to_string(),
        25_000,
    );
    team.finance = finance;
    team.transfer_budget = transfer_budget;
    team.wage_budget = 2_000_000;
    team.manager_id = Some("manager-1".to_string());
    team
}

fn make_seller_team(starting_xi_ids: Vec<String>) -> Team {
    let mut team = Team::new(
        "team-2".to_string(),
        "Seller FC".to_string(),
        "SEL".to_string(),
        "England".to_string(),
        "Liverpool".to_string(),
        "Seller Ground".to_string(),
        28_000,
    );
    team.starting_xi_ids = starting_xi_ids;
    team
}

fn make_ai_team(id: &str, name: &str, finance: i64, transfer_budget: i64) -> Team {
    let mut team = Team::new(
        id.to_string(),
        name.to_string(),
        name.chars().take(3).collect(),
        "England".to_string(),
        "Manchester".to_string(),
        format!("{} Ground", name),
        30_000,
    );
    team.finance = finance;
    team.transfer_budget = transfer_budget;
    team
}

fn make_game_with_player(
    player: Player,
    seller_starting_xi_ids: Vec<String>,
    user_finance: i64,
    user_transfer_budget: i64,
) -> Game {
    let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap());

    let mut manager = Manager::new(
        "manager-1".to_string(),
        "Jane".to_string(),
        "Doe".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("team-1".to_string());

    let mut game = Game::new(
        clock,
        manager,
        vec![
            make_user_team(user_finance, user_transfer_budget),
            make_seller_team(seller_starting_xi_ids),
        ],
        vec![player],
        vec![],
        vec![],
    );
    game.season_context.transfer_window.status = TransferWindowStatus::Open;
    game
}

fn attach_transfer_log_league(game: &mut Game) {
    let team_ids: Vec<String> = game.teams.iter().map(|team| team.id.clone()).collect();
    game.league = Some(League::new(
        "league-1".to_string(),
        "Premier Division".to_string(),
        2026,
        &team_ids,
    ));
}

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
    short_contract.contract_end = Some("2026-09-01".to_string());

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
fn incoming_transfer_offers_do_not_arrive_when_window_is_closed() {
    let mut player = make_user_player("player-window-closed");
    player.contract_end = Some("2026-09-01".to_string());
    player.market_value = 1_200_000;

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.season_context.transfer_window.status = TransferWindowStatus::Closed;
    game.teams[1].finance = 6_000_000;
    game.teams[1].transfer_budget = 3_000_000;

    generate_incoming_transfer_offers(&mut game);

    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-window-closed")
        .unwrap();
    assert!(player.transfer_offers.is_empty());
    assert!(game.messages.is_empty());
}

#[test]
fn accepted_closed_window_transfer_bid_is_registered_when_the_window_opens() {
    let player = make_player("player-bid-closed");
    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.clock.current_date = Utc.with_ymd_and_hms(2026, 12, 20, 12, 0, 0).unwrap();
    game.season_context.transfer_window.status = TransferWindowStatus::Closed;
    game.season_context.transfer_window.opens_on = Some("2027-01-01".to_string());

    let result = make_transfer_bid(&mut game, "player-bid-closed", 2_000_000)
        .expect("accepted closed-window bid should schedule registration");

    assert_eq!(result.decision, TransferNegotiationDecision::Accepted);
    assert_eq!(result.registration_date.as_deref(), Some("2027-01-01"));
    let scheduled_player = game
        .players
        .iter()
        .find(|player| player.id == "player-bid-closed")
        .unwrap();
    assert_eq!(scheduled_player.team_id.as_deref(), Some("team-2"));
    assert!(!scheduled_player.transfer_listed);
    assert_eq!(
        scheduled_player.transfer_offers[0].status,
        TransferOfferStatus::PendingRegistration
    );
    assert_eq!(
        scheduled_player.transfer_offers[0]
            .registration_date
            .as_deref(),
        Some("2027-01-01")
    );

    game.clock.current_date = Utc.with_ymd_and_hms(2027, 1, 1, 12, 0, 0).unwrap();
    game.season_context.transfer_window.status = TransferWindowStatus::Open;
    process_pending_transfer_registrations(&mut game);

    let registered_player = game
        .players
        .iter()
        .find(|player| player.id == "player-bid-closed")
        .unwrap();
    assert_eq!(registered_player.team_id.as_deref(), Some("team-1"));
    assert_eq!(
        registered_player.transfer_offers[0].status,
        TransferOfferStatus::Accepted
    );
    assert!(registered_player.movement_history.iter().any(|entry| {
        entry.kind == PlayerMovementKind::PermanentTransfer
            && entry.from_team_id.as_deref() == Some("team-2")
            && entry.to_team_id.as_deref() == Some("team-1")
            && entry.fee == Some(2_000_000)
    }));
}

#[test]
fn accepted_closed_window_incoming_transfer_is_registered_when_the_window_opens() {
    let mut player = make_user_player("player-incoming-scheduled-transfer");
    player.transfer_offers.push(make_pending_incoming_offer(
        "offer-scheduled-transfer",
        1_400_000,
    ));
    player.transfer_offers[0].date = "2026-12-20".to_string();
    let mut game = make_game_with_player(
        player,
        vec!["player-incoming-scheduled-transfer".to_string()],
        5_000_000,
        2_000_000,
    );
    game.clock.current_date = Utc.with_ymd_and_hms(2026, 12, 20, 12, 0, 0).unwrap();
    game.season_context.transfer_window.status = TransferWindowStatus::Closed;
    game.season_context.transfer_window.opens_on = Some("2027-01-01".to_string());
    game.teams[1].finance = 3_000_000;
    game.teams[1].transfer_budget = 3_000_000;

    respond_to_offer(
        &mut game,
        "player-incoming-scheduled-transfer",
        "offer-scheduled-transfer",
        true,
    )
    .expect("accepted incoming transfer should schedule registration");

    let scheduled_player = game
        .players
        .iter()
        .find(|player| player.id == "player-incoming-scheduled-transfer")
        .unwrap();
    assert_eq!(scheduled_player.team_id.as_deref(), Some("team-1"));
    assert_eq!(
        scheduled_player.transfer_offers[0].status,
        TransferOfferStatus::PendingRegistration
    );
    assert_eq!(
        scheduled_player.transfer_offers[0]
            .registration_date
            .as_deref(),
        Some("2027-01-01")
    );

    game.clock.current_date = Utc.with_ymd_and_hms(2027, 1, 1, 12, 0, 0).unwrap();
    game.season_context.transfer_window.status = TransferWindowStatus::Open;
    process_pending_transfer_registrations(&mut game);

    let registered_player = game
        .players
        .iter()
        .find(|player| player.id == "player-incoming-scheduled-transfer")
        .unwrap();
    assert_eq!(registered_player.team_id.as_deref(), Some("team-2"));
    assert_eq!(
        registered_player.transfer_offers[0].status,
        TransferOfferStatus::Accepted
    );
    assert!(registered_player.movement_history.iter().any(|entry| {
        entry.kind == PlayerMovementKind::PermanentTransfer
            && entry.from_team_id.as_deref() == Some("team-1")
            && entry.to_team_id.as_deref() == Some("team-2")
            && entry.fee == Some(1_400_000)
    }));

    let seller = game.teams.iter().find(|team| team.id == "team-1").unwrap();
    assert_eq!(seller.finance, 6_400_000);
    assert_eq!(seller.transfer_budget, 3_400_000);
    let buyer = game.teams.iter().find(|team| team.id == "team-2").unwrap();
    assert_eq!(buyer.finance, 1_600_000);
    assert_eq!(buyer.transfer_budget, 1_600_000);
}

#[test]
fn accepted_incoming_transfer_credits_selling_team_transfer_budget() {
    let fee = 1_568_520;
    let starting_finance = 5_000_000;
    let starting_budget = 116_000;
    let buyer_starting_finance = 8_000_000;
    let buyer_starting_budget = 3_000_000;

    let mut player = make_user_player("player-sale-budget-credit");
    player
        .transfer_offers
        .push(make_pending_incoming_offer("offer-sale-budget-credit", fee));
    let mut game = make_game_with_player(player, vec![], starting_finance, starting_budget);
    game.teams[1].finance = buyer_starting_finance;
    game.teams[1].transfer_budget = buyer_starting_budget;

    respond_to_offer(
        &mut game,
        "player-sale-budget-credit",
        "offer-sale-budget-credit",
        true,
    )
    .expect("accepted incoming transfer should execute immediately while the window is open");

    let seller = game.teams.iter().find(|team| team.id == "team-1").unwrap();
    assert_eq!(seller.finance, starting_finance + fee as i64);
    assert_eq!(seller.transfer_budget, starting_budget + fee as i64);

    let buyer = game.teams.iter().find(|team| team.id == "team-2").unwrap();
    assert_eq!(buyer.finance, buyer_starting_finance - fee as i64);
    assert_eq!(buyer.transfer_budget, buyer_starting_budget - fee as i64);

    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-sale-budget-credit")
        .unwrap();
    assert_eq!(player.team_id.as_deref(), Some("team-2"));
    assert_eq!(
        player.transfer_offers[0].status,
        TransferOfferStatus::Accepted
    );
}

#[test]
fn scheduled_transfer_preserves_market_state_when_registration_fails() {
    let mut player = make_player("player-scheduled-transfer-fails");
    player.transfer_listed = true;
    player.loan_listed = true;
    let mut competing_offer = make_pending_incoming_offer("competing-transfer", 1_200_000);
    competing_offer.from_team_id = "team-3".to_string();
    competing_offer.date = "2026-12-20".to_string();
    player.transfer_offers.push(competing_offer);
    let mut competing_loan = make_pending_incoming_loan_offer("competing-loan", 75, None);
    competing_loan.from_team_id = "team-3".to_string();
    competing_loan.parent_team_id = "team-2".to_string();
    player.loan_offers.push(competing_loan);

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.clock.current_date = Utc.with_ymd_and_hms(2026, 12, 20, 12, 0, 0).unwrap();
    game.season_context.transfer_window.status = TransferWindowStatus::Closed;
    game.season_context.transfer_window.opens_on = Some("2027-01-01".to_string());

    make_transfer_bid(&mut game, "player-scheduled-transfer-fails", 2_000_000)
        .expect("accepted closed-window bid should schedule registration");

    let scheduled_player = game
        .players
        .iter()
        .find(|player| player.id == "player-scheduled-transfer-fails")
        .unwrap();
    assert!(scheduled_player.transfer_listed);
    assert!(scheduled_player.loan_listed);
    assert_eq!(
        scheduled_player
            .transfer_offers
            .iter()
            .find(|offer| offer.id == "competing-transfer")
            .unwrap()
            .status,
        TransferOfferStatus::Pending
    );
    assert_eq!(
        scheduled_player
            .loan_offers
            .iter()
            .find(|offer| offer.id == "competing-loan")
            .unwrap()
            .status,
        LoanOfferStatus::Pending
    );

    game.clock.current_date = Utc.with_ymd_and_hms(2027, 1, 1, 12, 0, 0).unwrap();
    game.season_context.transfer_window.status = TransferWindowStatus::Open;
    game.teams[0].finance = 0;
    game.teams[0].transfer_budget = 0;
    process_pending_transfer_registrations(&mut game);

    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-scheduled-transfer-fails")
        .unwrap();
    assert_eq!(player.team_id.as_deref(), Some("team-2"));
    assert!(player.transfer_listed);
    assert!(player.loan_listed);
    assert_eq!(
        player
            .transfer_offers
            .iter()
            .find(|offer| offer.from_team_id == "team-1")
            .unwrap()
            .status,
        TransferOfferStatus::Withdrawn
    );
    assert_eq!(
        player
            .transfer_offers
            .iter()
            .find(|offer| offer.id == "competing-transfer")
            .unwrap()
            .status,
        TransferOfferStatus::Pending
    );
    assert_eq!(
        player
            .loan_offers
            .iter()
            .find(|offer| offer.id == "competing-loan")
            .unwrap()
            .status,
        LoanOfferStatus::Pending
    );
}

#[test]
fn scheduled_transfer_withdraws_competing_offers_after_registration_succeeds() {
    let mut player = make_player("player-scheduled-transfer-succeeds");
    player.transfer_listed = true;
    player.loan_listed = true;
    let mut competing_offer = make_pending_incoming_offer("competing-transfer", 1_200_000);
    competing_offer.from_team_id = "team-3".to_string();
    competing_offer.date = "2026-12-20".to_string();
    player.transfer_offers.push(competing_offer);
    let mut competing_loan = make_pending_incoming_loan_offer("competing-loan", 75, None);
    competing_loan.from_team_id = "team-3".to_string();
    competing_loan.parent_team_id = "team-2".to_string();
    player.loan_offers.push(competing_loan);

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.clock.current_date = Utc.with_ymd_and_hms(2026, 12, 20, 12, 0, 0).unwrap();
    game.season_context.transfer_window.status = TransferWindowStatus::Closed;
    game.season_context.transfer_window.opens_on = Some("2027-01-01".to_string());

    make_transfer_bid(&mut game, "player-scheduled-transfer-succeeds", 2_000_000)
        .expect("accepted closed-window bid should schedule registration");

    game.clock.current_date = Utc.with_ymd_and_hms(2027, 1, 1, 12, 0, 0).unwrap();
    game.season_context.transfer_window.status = TransferWindowStatus::Open;
    process_pending_transfer_registrations(&mut game);

    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-scheduled-transfer-succeeds")
        .unwrap();
    assert_eq!(player.team_id.as_deref(), Some("team-1"));
    assert!(!player.transfer_listed);
    assert!(!player.loan_listed);
    assert_eq!(
        player
            .transfer_offers
            .iter()
            .find(|offer| offer.from_team_id == "team-1")
            .unwrap()
            .status,
        TransferOfferStatus::Accepted
    );
    assert_eq!(
        player
            .transfer_offers
            .iter()
            .find(|offer| offer.id == "competing-transfer")
            .unwrap()
            .status,
        TransferOfferStatus::Withdrawn
    );
    assert_eq!(
        player
            .loan_offers
            .iter()
            .find(|offer| offer.id == "competing-loan")
            .unwrap()
            .status,
        LoanOfferStatus::Withdrawn
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
    player.wage = 520_000;

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
    player.wage = 520_000;

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
    player.wage = 20_000;

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
    player.contract_end = Some("2028-07-31".to_string());
    player.ovr = 62;
    player.potential = 74;
    player.wage = 520_000;

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

#[test]
fn loan_offer_rejects_end_date_after_player_contract() {
    let mut player = make_player("player-short-contract-loan");
    player.loan_listed = true;
    player.contract_end = Some("2026-12-01".to_string());
    player.ovr = 62;
    player.potential = 74;
    player.stats.appearances = 0;
    player.wage = 520_000;

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
    player.wage = 120_000;
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
    player.wage = 20_000;
    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.teams[0].wage_budget = 100_000;

    let mut existing_loan = make_player("existing-user-loan");
    existing_loan.team_id = Some("team-1".to_string());
    existing_loan.wage = 100_000;
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

    assert_eq!(calc_annual_wages(&game, "team-1"), 100_000);

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
fn loan_offer_rejects_terms_when_user_cannot_cover_loan_wage_share() {
    let mut player = make_player("player-loan-cash");
    player.loan_listed = true;
    player.wage = 120_000;
    let mut game = make_game_with_player(player, vec![], 50_000, 2_000_000);
    game.teams[0].wage_budget = 500_000;

    let error = make_loan_offer(&mut game, "player-loan-cash", "2027-01-01", 100, None)
        .expect_err("loan should be blocked by available finance");

    assert_eq!(error, "be.error.transfers.insufficientFunds");
    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-loan-cash")
        .expect("player should exist");
    assert!(player.active_loan.is_none());
    assert!(player.loan_offers.is_empty());
}

#[test]
fn loan_buy_option_can_be_exercised_from_active_user_loan() {
    let mut player = make_player("player-loan-to-buy");
    player.loan_listed = true;
    player.ovr = 62;
    player.potential = 74;
    player.stats.appearances = 0;
    player.wage = 520_000;

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
    player.wage = 260_000;

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
    player.wage = 260_000;

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
    loan_player.wage = 260_000;

    let mut contract_risk_player = make_user_player("player-contract-risk-mixed-market");
    contract_risk_player.contract_end = Some("2026-09-01".to_string());
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

#[test]
fn accepting_incoming_loan_offer_moves_user_player_to_borrowing_club() {
    let mut player = make_user_player("player-incoming-loan");
    player.loan_listed = true;
    player.wage = 520_000;
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
    assert_eq!(calc_annual_wages(&game, "team-1"), 130_000);
    assert_eq!(calc_annual_wages(&game, "team-2"), 390_000);
}

#[test]
fn countering_incoming_loan_offer_can_execute_accepted_terms() {
    let mut player = make_user_player("player-counter-loan-accepted");
    player.loan_listed = true;
    player.wage = 520_000;
    player.ovr = 68;
    player.potential = 78;
    player
        .loan_offers
        .push(make_pending_incoming_loan_offer("loan-counter-1", 65, None));

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
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
    player.wage = 520_000;
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
    player.contract_end = Some("2026-12-01".to_string());
    player.wage = 520_000;
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

#[test]
fn expiring_contract_lowers_resistance_to_sale() {
    let mut player = make_player("player-expiring");
    player.contract_end = Some("2026-08-31".to_string());

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);

    let result = make_transfer_bid(&mut game, "player-expiring", 1_000_000)
        .expect("bid should be evaluated");

    assert_eq!(result.decision, TransferNegotiationDecision::Accepted);
    assert_eq!(
        game.players
            .iter()
            .find(|player| player.id == "player-expiring")
            .and_then(|player| player.team_id.as_deref()),
        Some("team-1")
    );
    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-expiring")
        .unwrap();
    assert!(player.movement_history.iter().any(|entry| {
        entry.kind == PlayerMovementKind::PermanentTransfer
            && entry.from_team_id.as_deref() == Some("team-2")
            && entry.to_team_id.as_deref() == Some("team-1")
            && entry.fee == Some(1_000_000)
    }));
}

#[test]
fn key_player_is_harder_to_buy_than_fringe_player() {
    let mut star = make_player("player-star");
    star.attributes.shooting = 88;
    star.attributes.dribbling = 86;
    star.attributes.pace = 84;

    let mut star_game =
        make_game_with_player(star, vec!["player-star".to_string()], 5_000_000, 2_000_000);
    let star_result =
        make_transfer_bid(&mut star_game, "player-star", 1_250_000).expect("star bid");

    let fringe = make_player("player-fringe");
    let mut fringe_game = make_game_with_player(fringe, vec![], 5_000_000, 2_000_000);
    let fringe_result =
        make_transfer_bid(&mut fringe_game, "player-fringe", 1_250_000).expect("fringe bid");

    assert_eq!(
        star_result.decision,
        TransferNegotiationDecision::CounterOffer
    );
    assert!(star_result.suggested_fee.is_some());
    assert_eq!(
        fringe_result.decision,
        TransferNegotiationDecision::Accepted
    );
}

#[test]
fn repeated_bid_advances_transfer_negotiation_round() {
    let mut player = make_player("player-repeat-bid");
    player.morale = 35;
    player.stats.appearances = 1;
    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.teams[0].reputation = 700;
    game.teams[1].reputation = 350;

    let first_result =
        make_transfer_bid(&mut game, "player-repeat-bid", 900_000).expect("first bid");

    assert_eq!(
        first_result.decision,
        TransferNegotiationDecision::CounterOffer
    );
    assert_eq!(first_result.feedback.round, 1);
    assert_eq!(first_result.suggested_fee, Some(950_000));

    let second_result =
        make_transfer_bid(&mut game, "player-repeat-bid", 950_000).expect("second bid");

    assert_eq!(
        second_result.decision,
        TransferNegotiationDecision::Accepted
    );
    assert_eq!(second_result.feedback.round, 2);
    assert_eq!(
        game.players
            .iter()
            .find(|player| player.id == "player-repeat-bid")
            .and_then(|player| player.team_id.as_deref()),
        Some("team-1")
    );
}

#[test]
fn stale_outgoing_transfer_negotiation_is_withdrawn_before_new_bid() {
    let mut player = make_player("player-stale-bid");
    player.morale = 35;
    player.stats.appearances = 1;
    player.transfer_offers.push(TransferOffer {
        id: "offer-stale".to_string(),
        from_team_id: "team-1".to_string(),
        fee: 900_000,
        wage_offered: 0,
        last_manager_fee: Some(900_000),
        negotiation_round: 2,
        suggested_counter_fee: Some(1_150_000),
        status: TransferOfferStatus::Pending,
        date: "2026-07-15".to_string(),
        registration_date: None,
        closed_on: None,
    });

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.teams[0].reputation = 700;
    game.teams[1].reputation = 350;

    let result = make_transfer_bid(&mut game, "player-stale-bid", 900_000).expect("new bid");

    assert_eq!(result.decision, TransferNegotiationDecision::CounterOffer);
    assert_eq!(result.feedback.round, 1);

    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-stale-bid")
        .expect("player present");
    assert!(player.transfer_offers.iter().any(|offer| {
        offer.id == "offer-stale" && offer.status == TransferOfferStatus::Withdrawn
    }));
    assert!(player.transfer_offers.iter().any(|offer| {
        offer.id != "offer-stale"
            && offer.from_team_id == "team-1"
            && offer.status == TransferOfferStatus::Pending
            && offer.negotiation_round == 1
    }));
}

#[test]
fn low_transfer_budget_cannot_behave_unrealistically() {
    let mut player = make_player("player-budget");
    player.transfer_listed = true;

    let mut game = make_game_with_player(player, vec![], 5_000_000, 400_000);

    let error = make_transfer_bid(&mut game, "player-budget", 900_000)
        .expect_err("bid should be blocked by transfer budget");

    assert_eq!(error, "be.error.transfers.transferBudgetTooLow");
}

#[test]
fn generates_pending_incoming_offer_for_contract_risk_player() {
    let mut player = make_user_player("player-contract-risk");
    player.contract_end = Some("2026-09-01".to_string());
    player.market_value = 1_200_000;

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.teams[1].finance = 6_000_000;
    game.teams[1].transfer_budget = 3_000_000;

    generate_incoming_transfer_offers(&mut game);

    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-contract-risk")
        .unwrap();

    assert_eq!(player.transfer_offers.len(), 1);
    assert_eq!(
        player.transfer_offers[0].status,
        TransferOfferStatus::Pending
    );
    assert_eq!(player.transfer_offers[0].from_team_id, "team-2");
    assert_eq!(player.team_id.as_deref(), Some("team-1"));
    assert!(game.messages.iter().any(|message| {
        message.category == MessageCategory::Transfer
            && message.context.player_id.as_deref() == Some("player-contract-risk")
    }));
}

#[test]
fn ai_clubs_complete_transfer_between_themselves_without_inbox_message() {
    let mut player = make_player("player-ai-market");
    player.team_id = Some("team-3".to_string());
    player.contract_end = Some("2026-09-01".to_string());
    player.market_value = 1_200_000;
    player.transfer_listed = true;

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.teams
        .push(make_ai_team("team-3", "Seller FC", 3_000_000, 1_000_000));
    game.teams[1].finance = 6_000_000;
    game.teams[1].transfer_budget = 3_000_000;
    attach_transfer_log_league(&mut game);

    evaluate_transfer_market(&mut game);

    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-ai-market")
        .unwrap();
    assert_eq!(player.team_id.as_deref(), Some("team-2"));
    assert!(game.messages.is_empty());

    let buyer = game.teams.iter().find(|team| team.id == "team-2").unwrap();
    let seller = game.teams.iter().find(|team| team.id == "team-3").unwrap();
    assert_eq!(buyer.finance, 5_100_000);
    assert_eq!(seller.finance, 3_900_000);

    let transfer_log = &game.league.as_ref().unwrap().transfer_log;
    assert_eq!(transfer_log.len(), 1);
    assert_eq!(transfer_log[0].player_id, "player-ai-market");
    assert_eq!(transfer_log[0].from_team_id, "team-3");
    assert_eq!(transfer_log[0].to_team_id, "team-2");
    assert_eq!(transfer_log[0].fee, 900_000);
}

#[test]
fn ai_market_limits_completed_ai_transfers_per_day() {
    let mut first = make_player("player-ai-limit-1");
    first.team_id = Some("team-3".to_string());
    first.contract_end = Some("2026-09-01".to_string());
    first.market_value = 1_200_000;
    first.transfer_listed = true;

    let mut second = make_player("player-ai-limit-2");
    second.team_id = Some("team-3".to_string());
    second.contract_end = Some("2026-09-01".to_string());
    second.market_value = 1_100_000;
    second.transfer_listed = true;

    let mut third = make_player("player-ai-limit-3");
    third.team_id = Some("team-3".to_string());
    third.contract_end = Some("2026-09-01".to_string());
    third.market_value = 1_000_000;
    third.transfer_listed = true;

    let mut game = make_game_with_player(first, vec![], 5_000_000, 2_000_000);
    game.players.push(second);
    game.players.push(third);
    game.teams
        .push(make_ai_team("team-3", "Seller FC", 3_000_000, 1_000_000));
    game.teams
        .push(make_ai_team("team-4", "Buyer B", 6_000_000, 3_000_000));
    game.teams
        .push(make_ai_team("team-5", "Buyer C", 6_000_000, 3_000_000));
    game.teams[1].finance = 6_000_000;
    game.teams[1].transfer_budget = 3_000_000;
    attach_transfer_log_league(&mut game);

    evaluate_transfer_market(&mut game);

    let moved_players = game
        .players
        .iter()
        .filter(|player| player.team_id.as_deref() != Some("team-3"))
        .filter(|player| player.id.starts_with("player-ai-limit"))
        .count();

    assert_eq!(moved_players, 2);
    assert_eq!(game.league.as_ref().unwrap().transfer_log.len(), 2);
}

#[test]
fn does_not_duplicate_pending_incoming_offer_from_same_club() {
    let mut player = make_user_player("player-duplicate");
    player.contract_end = Some("2026-09-01".to_string());
    player.transfer_offers.push(TransferOffer {
        id: "offer-existing".to_string(),
        from_team_id: "team-2".to_string(),
        fee: 900_000,
        wage_offered: 0,
        last_manager_fee: None,
        negotiation_round: 1,
        suggested_counter_fee: None,
        status: TransferOfferStatus::Pending,
        date: "2026-08-01".to_string(),
        registration_date: None,
        closed_on: None,
    });

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.teams[1].finance = 6_000_000;
    game.teams[1].transfer_budget = 3_000_000;

    generate_incoming_transfer_offers(&mut game);

    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-duplicate")
        .unwrap();

    assert_eq!(player.transfer_offers.len(), 1);
    assert_eq!(player.transfer_offers[0].id, "offer-existing");
    assert!(game.messages.is_empty());
}

#[test]
fn incoming_offer_messages_from_multiple_clubs_get_unique_ids() {
    // Each user player may attract at most one new club per day, so two unique
    // messages require two different targets.
    let mut first = make_user_player("player-message-ids-1");
    first.contract_end = Some("2026-09-01".to_string());
    first.market_value = 1_200_000;
    first.natural_position = Position::Forward;

    let mut second = make_user_player("player-message-ids-2");
    second.contract_end = Some("2026-09-01".to_string());
    second.market_value = 1_200_000;
    second.natural_position = Position::Defender;

    let mut game = make_game_with_player(first, vec![], 5_000_000, 2_000_000);
    game.players.push(second);
    game.teams[1].finance = 6_000_000;
    game.teams[1].transfer_budget = 3_000_000;

    let mut extra_buyer = Team::new(
        "team-3".to_string(),
        "Buyer FC".to_string(),
        "BUY".to_string(),
        "England".to_string(),
        "Manchester".to_string(),
        "Buyer Ground".to_string(),
        30_000,
    );
    extra_buyer.finance = 6_000_000;
    extra_buyer.transfer_budget = 3_000_000;
    game.teams.push(extra_buyer);

    generate_incoming_transfer_offers(&mut game);

    let message_ids: Vec<&str> = game
        .messages
        .iter()
        .map(|message| message.id.as_str())
        .collect();
    let unique_message_ids: std::collections::HashSet<&str> = message_ids.iter().copied().collect();

    assert_eq!(message_ids.len(), 2);
    assert_eq!(unique_message_ids.len(), 2);
}

#[test]
fn at_most_one_new_club_bids_on_a_user_player_per_day() {
    let mut player = make_user_player("player-flood");
    player.contract_end = Some("2026-09-01".to_string());
    player.market_value = 1_200_000;

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    // Many wealthy suitors that could all afford the player.
    for index in 2..10 {
        game.teams.push(make_ai_team(
            &format!("team-{index}"),
            &format!("Buyer {index}"),
            10_000_000,
            5_000_000,
        ));
    }

    generate_incoming_transfer_offers(&mut game);

    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-flood")
        .unwrap();
    let pending_offers = player
        .transfer_offers
        .iter()
        .filter(|offer| offer.status == TransferOfferStatus::Pending)
        .count();

    assert_eq!(pending_offers, 1);
    assert_eq!(game.messages.len(), 1);
}

#[test]
fn repeat_interest_in_a_user_player_collapses_into_one_digest_message() {
    let mut player = make_user_player("player-digest");
    player.contract_end = Some("2026-09-01".to_string());
    player.market_value = 1_200_000;

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.teams
        .push(make_ai_team("team-3", "Buyer C", 10_000_000, 5_000_000));
    game.teams
        .push(make_ai_team("team-4", "Buyer D", 10_000_000, 5_000_000));

    // Day one: one club opens talks. Day two: a different club enquires.
    generate_incoming_transfer_offers(&mut game);
    game.clock.current_date += chrono::Duration::days(1);
    generate_incoming_transfer_offers(&mut game);

    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-digest")
        .unwrap();
    let pending = player
        .transfer_offers
        .iter()
        .filter(|offer| offer.status == TransferOfferStatus::Pending)
        .count();
    assert_eq!(pending, 2, "two distinct clubs should hold pending bids");

    let digests: Vec<_> = game
        .messages
        .iter()
        .filter(|message| message.id == "transfer_interest_player-digest")
        .collect();
    assert_eq!(
        digests.len(),
        1,
        "repeat interest must collapse into a single digest thread"
    );
    assert_eq!(game.messages.len(), 1);
    assert_eq!(
        digests[0].i18n_params.get("n").map(String::as_str),
        Some("2")
    );
    assert!(!digests[0].read, "an updated digest re-surfaces as unread");
}

#[test]
fn squad_wide_incoming_offers_are_capped_per_day() {
    let positions = [
        Position::Goalkeeper,
        Position::Defender,
        Position::Midfielder,
        Position::Forward,
        Position::Striker,
    ];
    let mut game = make_game_with_player(
        {
            let mut player = make_user_player("player-squad-0");
            player.contract_end = Some("2026-09-01".to_string());
            player.market_value = 1_200_000;
            player.natural_position = positions[0].clone();
            player
        },
        vec![],
        5_000_000,
        2_000_000,
    );
    for (index, position) in positions.iter().enumerate().skip(1) {
        let mut player = make_user_player(&format!("player-squad-{index}"));
        player.contract_end = Some("2026-09-01".to_string());
        player.market_value = 1_200_000;
        player.natural_position = position.clone();
        game.players.push(player);
    }
    for index in 2..12 {
        game.teams.push(make_ai_team(
            &format!("team-{index}"),
            &format!("Buyer {index}"),
            10_000_000,
            5_000_000,
        ));
    }

    generate_incoming_transfer_offers(&mut game);

    let new_pending_offers: usize = game
        .players
        .iter()
        .filter(|player| player.id.starts_with("player-squad"))
        .map(|player| {
            player
                .transfer_offers
                .iter()
                .filter(|offer| offer.status == TransferOfferStatus::Pending)
                .count()
        })
        .sum();

    assert_eq!(new_pending_offers, 3);
    assert_eq!(game.messages.len(), 3);
}

#[test]
fn contract_risk_player_draws_interest_before_similar_stable_player() {
    let mut risky = make_user_player("player-risky");
    risky.contract_end = Some("2026-09-01".to_string());
    risky.market_value = 1_100_000;

    let mut stable = make_user_player("player-stable");
    stable.contract_end = Some("2028-06-30".to_string());
    stable.market_value = 1_100_000;

    let mut game = make_game_with_player(risky, vec![], 5_000_000, 2_000_000);
    game.players.push(stable);
    game.teams[1].finance = 6_000_000;
    game.teams[1].transfer_budget = 3_000_000;

    generate_incoming_transfer_offers(&mut game);

    let risky = game
        .players
        .iter()
        .find(|player| player.id == "player-risky")
        .unwrap();
    let stable = game
        .players
        .iter()
        .find(|player| player.id == "player-stable")
        .unwrap();

    assert_eq!(risky.transfer_offers.len(), 1);
    assert!(stable.transfer_offers.is_empty());
}

#[test]
fn rejecting_pending_offer_closes_the_negotiation_cleanly() {
    let mut player = make_user_player("player-reject");
    player
        .transfer_offers
        .push(make_pending_incoming_offer("offer-reject", 900_000));

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.teams[1].finance = 6_000_000;
    game.teams[1].transfer_budget = 3_000_000;

    respond_to_offer(&mut game, "player-reject", "offer-reject", false)
        .expect("rejecting a pending offer should succeed");

    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-reject")
        .unwrap();
    assert_eq!(player.team_id.as_deref(), Some("team-1"));
    assert_eq!(player.transfer_offers.len(), 1);
    assert_eq!(
        player.transfer_offers[0].status,
        TransferOfferStatus::Rejected
    );
}

#[test]
fn rejecting_pending_offer_succeeds_for_pending_loan_player() {
    let mut player = make_user_player("player-reject-pending-loan");
    player.transfer_offers.push(make_pending_incoming_offer(
        "offer-reject-pending-loan",
        900_000,
    ));
    player.loan_offers.push(LoanOffer {
        status: LoanOfferStatus::PendingRegistration,
        ..make_pending_incoming_loan_offer("loan-pending-registration", 75, None)
    });

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.teams[1].finance = 6_000_000;
    game.teams[1].transfer_budget = 3_000_000;

    respond_to_offer(
        &mut game,
        "player-reject-pending-loan",
        "offer-reject-pending-loan",
        false,
    )
    .expect("rejecting a pending offer should still work for loan-reserved players");

    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-reject-pending-loan")
        .unwrap();
    assert_eq!(player.team_id.as_deref(), Some("team-1"));
    assert_eq!(
        player.transfer_offers[0].status,
        TransferOfferStatus::Rejected
    );
    assert_eq!(
        player.loan_offers[0].status,
        LoanOfferStatus::PendingRegistration
    );
}

#[test]
fn accepting_pending_offer_for_active_loan_player_does_not_mutate_offer() {
    let mut player = make_user_player("player-active-loan-transfer-offer");
    player
        .transfer_offers
        .push(make_pending_incoming_offer("offer-active-loan", 900_000));
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
    game.teams[1].finance = 6_000_000;
    game.teams[1].transfer_budget = 3_000_000;

    let error = respond_to_offer(
        &mut game,
        "player-active-loan-transfer-offer",
        "offer-active-loan",
        true,
    )
    .expect_err("active loan player should not be sold by accepting a transfer offer");

    assert_eq!(error, "be.error.transfers.playerAlreadyLoaned");
    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-active-loan-transfer-offer")
        .unwrap();
    assert_eq!(player.team_id.as_deref(), Some("team-1"));
    assert_eq!(
        player.transfer_offers[0].status,
        TransferOfferStatus::Pending
    );
}

#[test]
fn reasonable_counter_offer_is_accepted_and_executes_transfer() {
    let mut player = make_user_player("player-counter-accept");
    player.market_value = 1_000_000;
    player
        .transfer_offers
        .push(make_pending_incoming_offer("offer-counter-accept", 900_000));

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.teams[1].finance = 6_000_000;
    game.teams[1].transfer_budget = 3_000_000;

    let result = counter_offer(
        &mut game,
        "player-counter-accept",
        "offer-counter-accept",
        1_050_000,
    )
    .expect("counter offer should be evaluated");

    assert_eq!(result.decision, TransferNegotiationDecision::Accepted);
    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-counter-accept")
        .unwrap();
    assert_eq!(player.team_id.as_deref(), Some("team-2"));
    assert_eq!(
        player.transfer_offers[0].status,
        TransferOfferStatus::Accepted
    );
    assert_eq!(
        game.teams
            .iter()
            .find(|team| team.id == "team-1")
            .unwrap()
            .finance,
        6_050_000
    );
    assert_eq!(
        game.teams
            .iter()
            .find(|team| team.id == "team-2")
            .unwrap()
            .finance,
        4_950_000
    );
}

#[test]
fn excessive_counter_offer_is_rejected_and_closes_the_negotiation() {
    let mut player = make_user_player("player-counter-reject");
    player.market_value = 1_000_000;
    player
        .transfer_offers
        .push(make_pending_incoming_offer("offer-counter-reject", 900_000));

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.teams[1].finance = 6_000_000;
    game.teams[1].transfer_budget = 3_000_000;

    let result = counter_offer(
        &mut game,
        "player-counter-reject",
        "offer-counter-reject",
        1_400_000,
    )
    .expect("counter offer should be evaluated");

    assert_eq!(result.decision, TransferNegotiationDecision::Rejected);
    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-counter-reject")
        .unwrap();
    assert_eq!(player.team_id.as_deref(), Some("team-1"));
    assert_eq!(
        player.transfer_offers[0].status,
        TransferOfferStatus::Rejected
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
}

#[test]
fn unhappy_player_with_bigger_ambition_gap_is_easier_to_buy() {
    let mut open_player = make_player("player-open");
    open_player.contract_end = Some("2028-06-30".to_string());
    open_player.morale = 35;
    open_player.stats.appearances = 1;

    let mut open_game = make_game_with_player(open_player, vec![], 5_000_000, 2_000_000);
    open_game.teams[0].reputation = 700;
    open_game.teams[1].reputation = 350;
    let open_result =
        make_transfer_bid(&mut open_game, "player-open", 1_050_000).expect("open-player bid");

    let mut content_player = make_player("player-content");
    content_player.contract_end = Some("2028-06-30".to_string());
    content_player.morale = 80;
    content_player.stats.appearances = 12;

    let mut content_game = make_game_with_player(content_player, vec![], 5_000_000, 2_000_000);
    content_game.teams[0].reputation = 700;
    content_game.teams[1].reputation = 350;
    let content_result = make_transfer_bid(&mut content_game, "player-content", 1_050_000)
        .expect("content-player bid");

    assert_eq!(open_result.decision, TransferNegotiationDecision::Accepted);
    assert_eq!(
        content_result.decision,
        TransferNegotiationDecision::Rejected
    );
}

#[test]
fn blocking_open_player_move_reduces_morale_and_creates_contract_issue() {
    let mut player = make_user_player("player-blocked");
    player.contract_end = Some("2028-06-30".to_string());
    player.morale = 42;
    player.stats.appearances = 0;
    player
        .transfer_offers
        .push(make_pending_incoming_offer("offer-blocked", 950_000));

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.teams[0].reputation = 350;
    game.teams[1].reputation = 700;
    game.teams[1].finance = 6_000_000;
    game.teams[1].transfer_budget = 3_000_000;

    respond_to_offer(&mut game, "player-blocked", "offer-blocked", false)
        .expect("rejecting a pending offer should succeed");

    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-blocked")
        .unwrap();
    assert!(player.morale < 42);
    assert_eq!(
        player
            .morale_core
            .unresolved_issue
            .as_ref()
            .map(|issue| issue.category.clone()),
        Some(PlayerIssueCategory::Contract)
    );
}

#[test]
fn selling_key_player_can_reduce_remaining_starters_morale() {
    let mut key_player = make_user_player("player-key-sale");
    key_player
        .transfer_offers
        .push(make_pending_incoming_offer("offer-key-sale", 1_000_000));

    let mut teammate = make_user_player("player-teammate");
    teammate.morale = 75;

    let mut game = make_game_with_player(key_player, vec![], 5_000_000, 2_000_000);
    game.players.push(teammate);
    game.teams[0].starting_xi_ids =
        vec!["player-key-sale".to_string(), "player-teammate".to_string()];
    game.teams[1].finance = 6_000_000;
    game.teams[1].transfer_budget = 3_000_000;

    respond_to_offer(&mut game, "player-key-sale", "offer-key-sale", true)
        .expect("accepting the pending offer should succeed");

    let teammate = game
        .players
        .iter()
        .find(|player| player.id == "player-teammate")
        .unwrap();
    assert!(teammate.morale < 75);
}

#[test]
fn accepted_major_transfer_generates_news_article() {
    let mut player = make_player("player-news-major");
    player.market_value = 1_400_000;

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);

    let result = make_transfer_bid(&mut game, "player-news-major", 1_700_000)
        .expect("major transfer bid should succeed");

    assert_eq!(result.decision, TransferNegotiationDecision::Accepted);
    let article = game
        .news
        .iter()
        .find(|article| article.id == "transfer_news_player-news-major_team-2_team-1_2026-08-01")
        .expect("major transfer should create a news article");
    assert_eq!(article.category, NewsCategory::TransferRumour);
    assert_eq!(
        article.headline_key.as_deref(),
        Some("be.news.majorTransfer.headline")
    );
    assert_eq!(
        article.body_key.as_deref(),
        Some("be.news.majorTransfer.body")
    );
    assert_eq!(
        article.team_ids,
        vec!["team-2".to_string(), "team-1".to_string()]
    );
    assert_eq!(article.player_ids, vec!["player-news-major".to_string()]);
}

#[test]
fn smaller_completed_transfer_does_not_generate_news_article() {
    let mut player = make_player("player-news-small");
    player.market_value = 350_000;
    player.transfer_listed = true;

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);

    let result = make_transfer_bid(&mut game, "player-news-small", 300_000)
        .expect("small transfer bid should succeed");

    assert_eq!(result.decision, TransferNegotiationDecision::Accepted);
    assert!(game.news.is_empty());
}

#[test]
fn completed_transfer_news_is_not_duplicated_when_article_already_exists() {
    let mut player = make_player("player-news-dup");
    player.market_value = 1_400_000;

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.news.push(
        NewsArticle::new(
            "transfer_news_player-news-dup_team-2_team-1_2026-08-01".to_string(),
            "Existing transfer story".to_string(),
            "Existing body".to_string(),
            "League Chronicle".to_string(),
            "2026-08-01".to_string(),
            NewsCategory::TransferRumour,
        )
        .with_teams(vec!["team-2".to_string(), "team-1".to_string()])
        .with_players(vec!["player-news-dup".to_string()]),
    );

    let result = make_transfer_bid(&mut game, "player-news-dup", 1_700_000)
        .expect("major transfer bid should succeed");

    assert_eq!(result.decision, TransferNegotiationDecision::Accepted);
    assert_eq!(
        game.news
            .iter()
            .filter(|article| article.id == "transfer_news_player-news-dup_team-2_team-1_2026-08-01")
            .count(),
        1
    );
}

// A transfer must succeed end-to-end even when the incoming player's jersey
// number is already worn by someone at the buying club. Both players must
// remain in the game and end up with distinct jerseys at the new club.
#[test]
fn transfer_succeeds_when_incoming_player_jersey_collides_with_buyer_squad() {
    // Player being transferred (currently at the selling club, wears #6 there).
    let mut incoming = make_player("incoming-six");
    incoming.jersey_number = Some(6);
    incoming.contract_end = Some("2028-06-30".to_string());
    incoming.market_value = 1_000_000;

    let mut game = make_game_with_player(incoming, vec![], 5_000_000, 2_000_000);

    // An existing squad member at the buying club, also wearing #6.
    let mut existing = make_user_player("existing-six");
    existing.jersey_number = Some(6);
    game.players.push(existing);

    let result = make_transfer_bid(&mut game, "incoming-six", 1_500_000)
        .expect("bid for an affordable player should be accepted");
    assert_eq!(result.decision, TransferNegotiationDecision::Accepted);

    // If the bid is registered immediately, the transfer is already complete;
    // otherwise process the scheduled registration to finalize the move.
    process_pending_transfer_registrations(&mut game);

    let existing_after = game
        .players
        .iter()
        .find(|player| player.id == "existing-six")
        .expect("existing #6 must not be silently dropped by the transfer");
    let incoming_after = game
        .players
        .iter()
        .find(|player| player.id == "incoming-six")
        .expect("incoming player must be present after transfer");

    assert_eq!(
        incoming_after.team_id.as_deref(),
        Some("team-1"),
        "incoming player must end up at the buying club"
    );
    assert_eq!(
        existing_after.team_id.as_deref(),
        Some("team-1"),
        "existing player must stay at the buying club"
    );
    assert_eq!(
        existing_after.jersey_number,
        Some(6),
        "existing player must keep their #6 — the resolver must not churn settled assignments"
    );
    assert_eq!(
        incoming_after.jersey_number,
        Some(1),
        "incoming player whose #6 is taken must get the lowest free number (#1)"
    );
}

#[test]
fn executed_transfer_debits_the_buying_team_transfer_budget() {
    let mut player = make_player("player-budget-debit");
    player.morale = 35;
    player.stats.appearances = 1;
    let starting_finance = 5_000_000;
    let starting_budget = 2_000_000;
    let mut game = make_game_with_player(player, vec![], starting_finance, starting_budget);
    game.teams[0].reputation = 700;
    game.teams[1].reputation = 350;

    // First bid returns a counter — engine picks a suggestion around 950k.
    make_transfer_bid(&mut game, "player-budget-debit", 900_000)
        .expect("first bid should return a counter");
    // Accepting the suggestion executes the transfer.
    let result = make_transfer_bid(&mut game, "player-budget-debit", 950_000)
        .expect("second bid should be accepted");
    assert_eq!(result.decision, TransferNegotiationDecision::Accepted);

    // Both `finance` and `transfer_budget` must drop by the executed fee.
    // Regression guard for the pre-fix bug where `transfer_budget` gated
    // only the first bid and stayed constant thereafter, letting a team
    // spend €100M in €15M chunks against a €15M budget.
    let buyer = game.teams.iter().find(|t| t.id == "team-1").unwrap();
    assert_eq!(buyer.finance, starting_finance - 950_000);
    assert_eq!(buyer.transfer_budget, starting_budget - 950_000);
}

/// Builds a loan-listed user player plus `ai_teams` extra clubs, all able to afford him.
fn make_loan_pileup_game(player_id: &str, ai_teams: usize) -> Game {
    let mut player = make_user_player(player_id);
    player.loan_listed = true;
    player.ovr = 68;
    player.potential = 80;
    player.stats.appearances = 0;
    player.wage = 260_000;

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    for index in 0..ai_teams {
        game.teams.push(make_ai_team(
            &format!("team-pileup-{index}"),
            &format!("Buyer {index}"),
            10_000_000,
            5_000_000,
        ));
    }
    game
}

fn find_player<'a>(game: &'a Game, player_id: &str) -> &'a Player {
    game.players
        .iter()
        .find(|player| player.id == player_id)
        .expect("player should exist")
}

fn pending_incoming_approaches(game: &Game, player_id: &str) -> usize {
    let player = find_player(game, player_id);
    player
        .transfer_offers
        .iter()
        .filter(|offer| offer.status == TransferOfferStatus::Pending)
        .count()
        + player
            .loan_offers
            .iter()
            .filter(|offer| offer.status == LoanOfferStatus::Pending)
            .count()
}

/// The daily caps only bound *arrivals*. With a fourteen-day expiry and one new club a day,
/// thirteen offers survive alongside each new one, so a loan-listed player accumulates a
/// fourteen-deep stack of live proposals. Every existing guard asserts on a single call and
/// stays green throughout, which is why this reached a shipped save.
#[test]
fn pending_incoming_offers_never_exceed_the_cap_over_a_full_window() {
    let mut game = make_loan_pileup_game("player-user-loan-pileup", 12);

    let mut worst_seen = 0_usize;
    for _ in 0..60 {
        generate_incoming_transfer_offers(&mut game);
        worst_seen = worst_seen.max(pending_incoming_approaches(
            &game,
            "player-user-loan-pileup",
        ));
        game.clock.advance_days(1);
    }

    assert!(
        worst_seen <= 3,
        "a user player should never face more than \
         MAX_PENDING_INCOMING_OFFERS_PER_USER_PLAYER live approaches, saw {worst_seen}"
    );
    // A cap that worked by never generating an offer would satisfy the assertion above, so
    // pin the other side too: twelve able clubs should fill the queue.
    assert_eq!(
        worst_seen, 3,
        "the queue should still fill to the cap, saw {worst_seen}"
    );
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
    player.contract_end = Some("2026-09-01".to_string());
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

/// Outgoing bids close through `upsert_transfer_offer` rather than the closing helper, so that
/// path has to honour the same rule: an offer the selling club turns down keeps its arrival date
/// and records when talks ended.
#[test]
fn a_rejected_outgoing_bid_records_its_closure_without_moving_the_arrival_date() {
    let mut target = make_player("player-outgoing-reject");
    target.team_id = Some("team-2".to_string());
    target.market_value = 5_000_000;
    target.transfer_listed = true;

    let mut game = make_game_with_player(target, vec![], 50_000_000, 40_000_000);
    game.clock.advance_days(9);
    let bid_day = game.clock.current_date.format("%Y-%m-%d").to_string();

    // Far below what the selling club would entertain, so the bid is turned down outright.
    make_transfer_bid(&mut game, "player-outgoing-reject", 1)
        .expect("a bid should return an outcome");

    let offer = &find_player(&game, "player-outgoing-reject").transfer_offers[0];
    assert_eq!(offer.status, TransferOfferStatus::Rejected);
    assert_eq!(
        offer.closed_on.as_deref(),
        Some(bid_day.as_str()),
        "a rejected outgoing bid should record when it closed"
    );
    assert_eq!(
        offer.date, bid_day,
        "an offer created already rejected arrived the same day it closed"
    );
}

/// The other half of the same rule: the cap counts both deal types, so live loan talks consume
/// the budget a permanent bid would otherwise use.
#[test]
fn pending_loan_offers_count_towards_the_same_cap_as_transfer_bids() {
    let mut player = make_user_player("player-user-crosstype-cap");
    player.loan_listed = true;
    player.transfer_listed = true;
    player.contract_end = Some("2026-09-01".to_string());
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

/// Same rule on the permanent side: a counter the club walks away from closes the offer without
/// rewriting when it arrived.
#[test]
fn a_counter_that_ends_talks_preserves_the_transfer_offer_arrival_date() {
    let mut player = make_user_player("player-counter-keeps-arrival");
    player.transfer_listed = true;
    player.market_value = 1_000_000;
    player
        .transfer_offers
        .push(make_pending_incoming_offer("offer-1", 900_000));

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.teams[1].finance = 6_000_000;
    game.teams[1].transfer_budget = 3_000_000;
    game.clock.advance_days(4);

    // Far above anything the buyer would entertain, so talks end rather than continue.
    counter_offer(
        &mut game,
        "player-counter-keeps-arrival",
        "offer-1",
        900_000_000,
    )
    .expect("countering should return an outcome");

    let offer = &find_player(&game, "player-counter-keeps-arrival").transfer_offers[0];
    assert_eq!(offer.status, TransferOfferStatus::Rejected);
    assert_eq!(offer.date, "2026-08-01", "arrival date must be preserved");
    assert!(offer.closed_on.is_some(), "closure should be recorded");
}

/// A deal agreed in a closed window registers months later, so by the time a failed registration
/// withdraws the offer its arrival date is already older than the retention window. Without a
/// closure stamp the prune falls back to arrival and drops the record on the spot, instead of
/// keeping it the usual 120 days after it was withdrawn.
#[test]
fn a_failed_scheduled_registration_records_the_withdrawal_date() {
    let mut player = make_user_player("player-failed-registration");
    player
        .transfer_offers
        .push(make_pending_incoming_offer("offer-scheduled", 1_400_000));
    player.transfer_offers[0].date = "2026-08-01".to_string();

    let mut game = make_game_with_player(
        player,
        vec!["player-failed-registration".to_string()],
        5_000_000,
        2_000_000,
    );
    game.clock.current_date = Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap();
    game.season_context.transfer_window.status = TransferWindowStatus::Closed;
    game.season_context.transfer_window.opens_on = Some("2027-01-01".to_string());
    game.teams[1].finance = 3_000_000;
    game.teams[1].transfer_budget = 3_000_000;

    respond_to_offer(
        &mut game,
        "player-failed-registration",
        "offer-scheduled",
        true,
    )
    .expect("accepting in a closed window should schedule registration");

    // The buyer's finances collapse before the window opens, so registration cannot go through.
    game.clock.current_date = Utc.with_ymd_and_hms(2027, 1, 1, 12, 0, 0).unwrap();
    game.season_context.transfer_window.status = TransferWindowStatus::Open;
    game.teams[1].finance = 0;
    game.teams[1].transfer_budget = 0;
    process_pending_transfer_registrations(&mut game);

    let offer = &find_player(&game, "player-failed-registration").transfer_offers[0];
    assert_eq!(offer.status, TransferOfferStatus::Withdrawn);
    assert_eq!(
        offer.closed_on.as_deref(),
        Some("2027-01-01"),
        "a failed registration should record when the offer was withdrawn"
    );

    // Retention runs from the withdrawal, not from an arrival five months earlier.
    game.clock.current_date = Utc.with_ymd_and_hms(2027, 2, 1, 12, 0, 0).unwrap();
    evaluate_transfer_market(&mut game);
    assert_eq!(
        find_player(&game, "player-failed-registration")
            .transfer_offers
            .len(),
        1,
        "the withdrawn offer should still be inside its retention window"
    );
}

/// Terminal offers are kept for a while so the UI can show recent history, then dropped —
/// otherwise every rejected approach stays on the player for the life of the save.
#[test]
fn closed_offers_are_pruned_once_they_fall_outside_the_retention_window() {
    let mut game = make_loan_pileup_game("player-user-loan-retention", 12);

    for _ in 0..60 {
        generate_incoming_transfer_offers(&mut game);
        game.clock.advance_days(1);
    }
    let before = find_player(&game, "player-user-loan-retention")
        .loan_offers
        .len();

    game.clock.advance_days(200);
    generate_incoming_transfer_offers(&mut game);

    let after = find_player(&game, "player-user-loan-retention")
        .loan_offers
        .len();

    assert!(
        after < before,
        "closed offers older than the retention window should be pruned ({before} -> {after})"
    );
}

/// A player who is not listed can still draw interest — a contract running down, a big valuation,
/// low morale. That is intended. What is not intended is the same club asking again the day after
/// being turned down, which is what a manager actually experiences as harassment.
#[test]
fn a_club_that_is_turned_down_does_not_come_straight_back() {
    let mut player = make_user_player("player-persistent-suitor");
    player.transfer_listed = false;
    player.contract_end = Some("2026-11-01".to_string());
    player.market_value = 1_400_000;

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.teams[1].finance = 9_000_000;
    game.teams[1].transfer_budget = 6_000_000;
    let suitor = game.teams[1].id.clone();

    let mut approaches = 0_usize;
    for _ in 0..30 {
        generate_incoming_transfer_offers(&mut game);

        // The manager turns down everything this club sends, every time.
        let pending: Vec<String> = find_player(&game, "player-persistent-suitor")
            .transfer_offers
            .iter()
            .filter(|offer| {
                offer.status == TransferOfferStatus::Pending && offer.from_team_id == suitor
            })
            .map(|offer| offer.id.clone())
            .collect();
        for offer_id in pending {
            approaches += 1;
            respond_to_offer(&mut game, "player-persistent-suitor", &offer_id, false)
                .expect("rejecting an incoming offer should succeed");
        }

        game.clock.advance_days(1);
    }

    // Exactly one, not "a small number": the loop covers days 0 to 29, which is the whole
    // cooldown, so a second approach anywhere in it means the window is shorter than advertised.
    assert_eq!(
        approaches, 1,
        "a rejected club should approach once and then wait out the cooldown, \
         but it approached {approaches} times over days 0 to 29"
    );
}

/// The cooldown covers expiry as well as refusal, and that half needs its own test: every other
/// test here rejects the offer, so dropping `Withdrawn` from the cooldown match leaves all of them
/// green while talks that went cold silently stop cooling the club.
#[test]
fn a_club_whose_talks_expired_also_waits_before_asking_again() {
    let mut game = make_persistent_suitor_game("player-expired-suitor", 0);
    let suitor = game.teams[1].id.clone();

    generate_incoming_transfer_offers(&mut game);
    assert!(
        find_player(&game, "player-expired-suitor")
            .transfer_offers
            .iter()
            .any(|offer| offer.status == TransferOfferStatus::Pending),
        "the club should open talks on the first day"
    );

    // Nobody answers, so the offer goes stale on its own rather than being refused.
    game.clock.advance_days(15);
    generate_incoming_transfer_offers(&mut game);
    let player = find_player(&game, "player-expired-suitor");
    assert!(
        player
            .transfer_offers
            .iter()
            .any(|offer| offer.status == TransferOfferStatus::Withdrawn),
        "the offer should have expired"
    );
    assert!(
        !player
            .transfer_offers
            .iter()
            .any(|offer| offer.status == TransferOfferStatus::Pending),
        "the club should not reopen talks the moment its own offer went cold"
    );

    // Still inside the window measured from when talks ended.
    game.clock.advance_days(20);
    generate_incoming_transfer_offers(&mut game);
    assert!(
        !find_player(&game, "player-expired-suitor")
            .transfer_offers
            .iter()
            .any(|offer| offer.status == TransferOfferStatus::Pending),
        "the club should still be cooling off inside the window"
    );

    // Past it, interest may legitimately revive.
    game.clock.advance_days(20);
    generate_incoming_transfer_offers(&mut game);
    assert!(
        find_player(&game, "player-expired-suitor")
            .transfer_offers
            .iter()
            .any(|offer| {
                offer.status == TransferOfferStatus::Pending && offer.from_team_id == suitor
            }),
        "the club should be free to try again once the cooldown has expired"
    );
}

fn make_persistent_suitor_game(player_id: &str, ai_teams: usize) -> Game {
    let mut player = make_user_player(player_id);
    player.transfer_listed = false;
    player.contract_end = Some("2026-11-01".to_string());
    player.market_value = 1_400_000;

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.teams[1].finance = 9_000_000;
    game.teams[1].transfer_budget = 6_000_000;
    for index in 0..ai_teams {
        game.teams.push(make_ai_team(
            &format!("team-suitor-{index}"),
            &format!("Suitor {index}"),
            9_000_000,
            6_000_000,
        ));
    }
    game
}

/// Rejecting one club must not close the market. The queue has three slots and a club sitting out
/// its cooldown does not occupy one, so somebody else can still come in.
#[test]
fn turning_one_club_away_does_not_stop_the_others_bidding() {
    let mut game = make_persistent_suitor_game("player-other-suitors", 6);
    let refused = game.teams[1].id.clone();

    for _ in 0..10 {
        generate_incoming_transfer_offers(&mut game);
        let from_refused: Vec<String> = find_player(&game, "player-other-suitors")
            .transfer_offers
            .iter()
            .filter(|offer| {
                offer.status == TransferOfferStatus::Pending && offer.from_team_id == refused
            })
            .map(|offer| offer.id.clone())
            .collect();
        for offer_id in from_refused {
            respond_to_offer(&mut game, "player-other-suitors", &offer_id, false)
                .expect("rejecting an incoming offer should succeed");
        }
        game.clock.advance_days(1);
    }

    let others = find_player(&game, "player-other-suitors")
        .transfer_offers
        .iter()
        .filter(|offer| {
            offer.status == TransferOfferStatus::Pending && offer.from_team_id != refused
        })
        .count();
    assert!(
        others > 0,
        "other clubs should still be able to approach after one was turned away"
    );
}

/// A cooldown, not a ban — the club is allowed back once enough time has passed, otherwise one
/// rejection would permanently remove a suitor from a player's market.
#[test]
fn a_rejected_club_may_approach_again_once_the_cooldown_has_passed() {
    let mut game = make_persistent_suitor_game("player-suitor-returns", 0);
    let suitor = game.teams[1].id.clone();

    generate_incoming_transfer_offers(&mut game);
    let offer_id = find_player(&game, "player-suitor-returns").transfer_offers[0]
        .id
        .clone();
    respond_to_offer(&mut game, "player-suitor-returns", &offer_id, false)
        .expect("rejecting an incoming offer should succeed");

    // The last day still inside the window. Testing the boundary rather than a comfortable
    // margin is what makes the length of the cooldown an asserted fact instead of a guess.
    game.clock.advance_days(29);
    generate_incoming_transfer_offers(&mut game);
    assert!(
        !find_player(&game, "player-suitor-returns")
            .transfer_offers
            .iter()
            .any(|offer| offer.status == TransferOfferStatus::Pending),
        "the club should still be cooling off on the last day of the window"
    );

    // The first day outside it.
    game.clock.advance_days(1);
    generate_incoming_transfer_offers(&mut game);
    assert!(
        find_player(&game, "player-suitor-returns")
            .transfer_offers
            .iter()
            .any(|offer| {
                offer.status == TransferOfferStatus::Pending && offer.from_team_id == suitor
            }),
        "the club should be free to try again once the cooldown has expired"
    );
}

/// The cooldown spans both deal types, so a refused permanent bid cannot be re-run as a loan
/// approach the next day — which is the same harassment wearing a different hat.
#[test]
fn a_club_refused_a_transfer_cannot_return_immediately_as_a_loan_approach() {
    let mut game = make_persistent_suitor_game("player-suitor-switches", 0);
    let suitor = game.teams[1].id.clone();

    generate_incoming_transfer_offers(&mut game);
    let offer_id = find_player(&game, "player-suitor-switches").transfer_offers[0]
        .id
        .clone();
    respond_to_offer(&mut game, "player-suitor-switches", &offer_id, false)
        .expect("rejecting an incoming offer should succeed");

    // The manager now makes him available on loan, which would otherwise open a fresh route in.
    if let Some(player) = game
        .players
        .iter_mut()
        .find(|player| player.id == "player-suitor-switches")
    {
        player.loan_listed = true;
    }

    for _ in 0..10 {
        game.clock.advance_days(1);
        generate_incoming_transfer_offers(&mut game);
    }

    assert!(
        !find_player(&game, "player-suitor-switches")
            .loan_offers
            .iter()
            .any(|offer| offer.from_team_id == suitor),
        "a club refused a permanent bid should not reappear as a loan approach inside the cooldown"
    );
}

// ---------------------------------------------------------------------------
// Completing a deal has to leave the selling club's books straight. The two
// completion paths, a permanent sale and an exercised buy option, each got this
// right where the other got it wrong.
// ---------------------------------------------------------------------------

/// Selling a player has to release every role he held, not only his place in the XI. The loan and
/// release paths both call `remove_player_references`; the permanent sale trimmed the XI by hand
/// and left the rest, so a departed player could still be the old club's captain.
#[test]
fn selling_a_player_releases_every_role_he_held() {
    let mut player = make_user_player("player-departing-captain");
    player.transfer_listed = true;
    player.market_value = 900_000;
    player
        .transfer_offers
        .push(make_pending_incoming_offer("offer-captain", 1_000_000));

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.teams[1].finance = 9_000_000;
    game.teams[1].transfer_budget = 6_000_000;

    let seller = game.teams[0].id.clone();
    if let Some(team) = game.teams.iter_mut().find(|team| team.id == seller) {
        team.starting_xi_ids = vec!["player-departing-captain".to_string()];
        team.match_roles.captain = Some("player-departing-captain".to_string());
        team.match_roles.penalty_taker = Some("player-departing-captain".to_string());
        team.match_roles.corner_taker = Some("player-departing-captain".to_string());
    }

    respond_to_offer(&mut game, "player-departing-captain", "offer-captain", true)
        .expect("accepting the offer should complete the sale");

    let old_club = game.teams.iter().find(|team| team.id == seller).unwrap();
    assert!(
        !old_club
            .starting_xi_ids
            .contains(&"player-departing-captain".to_string()),
        "the sold player should leave the XI"
    );
    assert_eq!(
        old_club.match_roles.captain, None,
        "a sold player cannot still captain the club that sold him"
    );
    assert_eq!(old_club.match_roles.penalty_taker, None);
    assert_eq!(old_club.match_roles.corner_taker, None);
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

/// A completed transfer has to reach the competition its clubs actually play in.
/// `Game::sync_legacy_league` overwrites `game.league` with a clone of a competition, so a record
/// written only there is discarded rather than merely misfiled.
#[test]
fn a_completed_transfer_reaches_the_competition_log() {
    let mut player = make_user_player("player-logged-transfer");
    player.transfer_listed = true;
    player.market_value = 1_500_000;
    player
        .transfer_offers
        .push(make_pending_incoming_offer("offer-logged", 1_600_000));

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.teams[1].finance = 9_000_000;
    game.teams[1].transfer_budget = 6_000_000;

    game.teams
        .push(make_ai_team("team-3", "Other Rovers", 1_000_000, 500_000));
    game.teams
        .push(make_ai_team("team-4", "Other Albion", 1_000_000, 500_000));

    // Two divisions, and the one neither club plays in comes first, so a fix that simply appends to
    // `competitions[0]` — or that leans on the single-competition fallback — files the move wrongly
    // and fails here.
    game.competitions.push(ofm_core::schedule::generate_league(
        "Other Division",
        2026,
        &["team-3".to_string(), "team-4".to_string()],
        game.clock.current_date,
    ));
    game.competitions.push(ofm_core::schedule::generate_league(
        "Their Division",
        2026,
        &["team-1".to_string(), "team-2".to_string()],
        game.clock.current_date,
    ));

    respond_to_offer(&mut game, "player-logged-transfer", "offer-logged", true)
        .expect("accepting the offer should complete the sale");

    let logged_in = |name: &str| {
        game.competitions
            .iter()
            .find(|competition| competition.name == name)
            .expect("the division should still be there")
            .transfer_log
            .iter()
            .any(|entry| entry.player_id == "player-logged-transfer")
    };

    assert!(
        logged_in("Their Division"),
        "the completed transfer should be recorded against the division the two clubs play in, \
         not only on the legacy league mirror that gets overwritten"
    );
    assert!(
        !logged_in("Other Division"),
        "the move belongs to the clubs' own division, not to whichever competition comes first"
    );
}
