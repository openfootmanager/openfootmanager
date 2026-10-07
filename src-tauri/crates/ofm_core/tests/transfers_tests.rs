#[path = "transfers_tests/loans.rs"]
mod loans;

use chrono::{TimeZone, Utc};
use domain::league::{CompetitionType, League};
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
use ofm_core::finances::calc_wages;
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
    player.stage_contract_end(Some("2028-06-30".to_string()));
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
    give_every_club_squad_depth(&mut game);
    game
}

/// Squad depth for every club in `game`, so that one player leaving never takes
/// a club below the squad floor. These tests are about the deal, not about the
/// floor — `squad_floor_*` below covers that — and a one-player club is below
/// the floor before any deal is struck. Safe to call again after adding clubs.
/// The depth is inert on purpose: no wage,
/// no market value and a long contract, so it neither draws bids nor moves a
/// budget.
fn give_every_club_squad_depth(game: &mut Game) {
    let team_ids: Vec<String> = game.teams.iter().map(|team| team.id.clone()).collect();
    for team_id in team_ids {
        // A senior more than the minimum in every group, sixteen in all: one
        // player leaving never takes the club below the fifteen-senior floor.
        let depth = [3, 5, 5, 3];
        for ((group, _), count) in ofm_core::squad_floor::MIN_PLAYERS_PER_GROUP
            .into_iter()
            .zip(depth)
        {
            for index in 0..count {
                let id = format!("depth-{team_id}-{group:?}-{index}");
                if game.players.iter().any(|player| player.id == id) {
                    continue;
                }
                let mut player = Player::new(
                    id.clone(),
                    id.clone(),
                    id,
                    "1996-01-01".to_string(),
                    "England".to_string(),
                    group.clone(),
                    PlayerAttributes {
                        handling: 30,
                        reflexes: 30,
                        ..default_attrs()
                    },
                );
                player.team_id = Some(team_id.clone());
                player.stage_contract_end(Some("2031-06-30".to_string()));
                player.stage_wage(0);
                player.market_value = 0;
                player.morale = 70;
                game.players.push(player);
            }
        }
    }
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
fn incoming_transfer_offers_do_not_arrive_when_window_is_closed() {
    let mut player = make_user_player("player-window-closed");
    player.stage_contract_end(Some("2026-09-01".to_string()));
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

/// Joining a new club is signing a new contract. It starts on the day the move
/// happens, which for a bid made with the window closed is the day it registers and
/// not the day of the bid, and it must not keep the selling club's start date.
#[test]
fn a_permanent_transfer_starts_a_new_contract_on_the_day_it_registers() {
    let mut player = make_player("player-new-contract");
    player.stage_contract_start(Some("2019-07-01".to_string()));
    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.clock.current_date = Utc.with_ymd_and_hms(2026, 12, 20, 12, 0, 0).unwrap();
    game.season_context.transfer_window.status = TransferWindowStatus::Closed;
    game.season_context.transfer_window.opens_on = Some("2027-01-01".to_string());

    make_transfer_bid(&mut game, "player-new-contract", 2_000_000)
        .expect("an accepted closed-window bid schedules registration");

    let scheduled = game
        .players
        .iter()
        .find(|player| player.id == "player-new-contract")
        .unwrap();
    assert_eq!(
        scheduled.contract_start(),
        Some("2019-07-01"),
        "until the move registers the player is still on the selling club's contract"
    );

    game.clock.current_date = Utc.with_ymd_and_hms(2027, 1, 1, 12, 0, 0).unwrap();
    game.season_context.transfer_window.status = TransferWindowStatus::Open;
    process_pending_transfer_registrations(&mut game);

    let registered = game
        .players
        .iter()
        .find(|player| player.id == "player-new-contract")
        .unwrap();
    assert_eq!(registered.team_id.as_deref(), Some("team-1"));
    assert_eq!(
        registered.contract_start(),
        Some("2027-01-01"),
        "the new contract starts the day the transfer registers, not on the bid date \
         or the selling club's start"
    );
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
fn expiring_contract_lowers_resistance_to_sale() {
    let mut player = make_player("player-expiring");
    player.stage_contract_end(Some("2026-08-31".to_string()));

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
    player.stage_contract_end(Some("2026-09-01".to_string()));
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
    player.stage_contract_end(Some("2026-09-01".to_string()));
    player.market_value = 1_200_000;
    player.transfer_listed = true;

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.teams
        .push(make_ai_team("team-3", "Seller FC", 3_000_000, 1_000_000));
    game.teams[1].finance = 6_000_000;
    game.teams[1].transfer_budget = 3_000_000;
    give_every_club_squad_depth(&mut game);
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
    first.stage_contract_end(Some("2026-09-01".to_string()));
    first.market_value = 1_200_000;
    first.transfer_listed = true;

    let mut second = make_player("player-ai-limit-2");
    second.team_id = Some("team-3".to_string());
    second.stage_contract_end(Some("2026-09-01".to_string()));
    second.market_value = 1_100_000;
    second.transfer_listed = true;

    let mut third = make_player("player-ai-limit-3");
    third.team_id = Some("team-3".to_string());
    third.stage_contract_end(Some("2026-09-01".to_string()));
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
    give_every_club_squad_depth(&mut game);
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
    player.stage_contract_end(Some("2026-09-01".to_string()));
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
    first.stage_contract_end(Some("2026-09-01".to_string()));
    first.market_value = 1_200_000;
    first.natural_position = Position::Forward;

    let mut second = make_user_player("player-message-ids-2");
    second.stage_contract_end(Some("2026-09-01".to_string()));
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
    player.stage_contract_end(Some("2026-09-01".to_string()));
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
    player.stage_contract_end(Some("2026-09-01".to_string()));
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
            player.stage_contract_end(Some("2026-09-01".to_string()));
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
        player.stage_contract_end(Some("2026-09-01".to_string()));
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
    risky.stage_contract_end(Some("2026-09-01".to_string()));
    risky.market_value = 1_100_000;

    let mut stable = make_user_player("player-stable");
    stable.stage_contract_end(Some("2028-06-30".to_string()));
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
    open_player.stage_contract_end(Some("2028-06-30".to_string()));
    open_player.morale = 35;
    open_player.stats.appearances = 1;

    let mut open_game = make_game_with_player(open_player, vec![], 5_000_000, 2_000_000);
    open_game.teams[0].reputation = 700;
    open_game.teams[1].reputation = 350;
    let open_result =
        make_transfer_bid(&mut open_game, "player-open", 1_050_000).expect("open-player bid");

    let mut content_player = make_player("player-content");
    content_player.stage_contract_end(Some("2028-06-30".to_string()));
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
    player.stage_contract_end(Some("2028-06-30".to_string()));
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
    incoming.stage_contract_end(Some("2028-06-30".to_string()));
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
    player.stage_wage(260_000);

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
    player.stage_contract_end(Some("2026-11-01".to_string()));
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
    player.stage_contract_end(Some("2026-11-01".to_string()));
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

/// Selling across divisions has to reach the screen the manager actually reads. The news roundup
/// and the world transfer tab both read `game.league.transfer_log`, and `sync_legacy_league` fills
/// that mirror from the user's own competition. So filing the manager's sale under the *buyer's*
/// division hides it: the record exists, and nothing ever shows it.
#[test]
fn a_sale_to_another_division_still_reaches_the_users_own_log() {
    let mut player = make_user_player("player-sold-abroad");
    player.transfer_listed = true;
    player.market_value = 1_500_000;
    player
        .transfer_offers
        .push(make_pending_incoming_offer("offer-sold-abroad", 1_600_000));

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.teams[1].finance = 9_000_000;
    game.teams[1].transfer_budget = 6_000_000;
    game.teams
        .push(make_ai_team("team-3", "User Rivals", 1_000_000, 500_000));
    game.teams
        .push(make_ai_team("team-4", "Buyer Rivals", 1_000_000, 500_000));

    // The buyer's division comes first, so routing that looks at the buying club before the
    // manager's own club files the sale where the manager can never see it.
    game.competitions.push(ofm_core::schedule::generate_league(
        "Buyer Division",
        2026,
        &["team-2".to_string(), "team-4".to_string()],
        game.clock.current_date,
    ));
    game.competitions.push(ofm_core::schedule::generate_league(
        "User Division",
        2026,
        &["team-1".to_string(), "team-3".to_string()],
        game.clock.current_date,
    ));

    respond_to_offer(&mut game, "player-sold-abroad", "offer-sold-abroad", true)
        .expect("accepting the offer should complete the sale");

    let logged_in = |name: &str| {
        game.competitions
            .iter()
            .find(|competition| competition.name == name)
            .expect("the division should still be there")
            .transfer_log
            .iter()
            .any(|entry| entry.player_id == "player-sold-abroad")
    };

    assert!(
        logged_in("User Division"),
        "the manager's own sale belongs in the manager's own division"
    );
    assert!(
        !logged_in("Buyer Division"),
        "one record, in one log — the sale should not also be filed under the buyer"
    );

    // The assertion that matters. This mirror is the only transfer log the news roundup and the
    // world transfer tab ever read, so a record that misses it is a record nobody sees.
    assert!(
        game.league
            .as_ref()
            .expect("the legacy mirror should be populated")
            .transfer_log
            .iter()
            .any(|entry| entry.player_id == "player-sold-abroad"),
        "the completed sale should reach the legacy mirror the transfer screens read"
    );
}

/// The same drift, without crossing a division. A club plays in a cup as well as its league, and
/// the cup happens to be listed first. `sync_legacy_league` always mirrors the league, so filing
/// the sale under the cup hides it just as thoroughly as filing it under another division.
#[test]
fn a_sale_is_filed_under_the_league_the_mirror_shows_not_the_cup() {
    let mut player = make_user_player("player-sold-in-cup-season");
    player.transfer_listed = true;
    player.market_value = 1_500_000;
    player.transfer_offers.push(make_pending_incoming_offer(
        "offer-sold-in-cup-season",
        1_600_000,
    ));

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.teams[1].finance = 9_000_000;
    game.teams[1].transfer_budget = 6_000_000;
    game.teams
        .push(make_ai_team("team-3", "League Rivals", 1_000_000, 500_000));

    let mut cup = ofm_core::schedule::generate_league(
        "Domestic Cup",
        2026,
        &["team-1".to_string(), "team-2".to_string()],
        game.clock.current_date,
    );
    cup.kind = CompetitionType::Cup;
    game.competitions.push(cup);
    game.competitions.push(ofm_core::schedule::generate_league(
        "User Division",
        2026,
        &["team-1".to_string(), "team-3".to_string()],
        game.clock.current_date,
    ));

    respond_to_offer(
        &mut game,
        "player-sold-in-cup-season",
        "offer-sold-in-cup-season",
        true,
    )
    .expect("accepting the offer should complete the sale");

    assert!(
        game.league
            .as_ref()
            .expect("the legacy mirror should be populated")
            .transfer_log
            .iter()
            .any(|entry| entry.player_id == "player-sold-in-cup-season"),
        "the sale should land in the league the mirror shows, not in the cup listed before it"
    );
}

/// The last resort has to keep the record. When no competition lists either club, the fallback
/// only reached for `competitions[0]` if there was exactly one competition; with more than one it
/// wrote into `game.league` instead. That mirror is rebuilt from `game.competitions` on the next
/// sync and is not what the save reads, so the record was written and then thrown away — the very
/// failure this path exists to stop.
#[test]
fn a_transfer_between_clubs_outside_every_competition_is_still_kept() {
    let mut player = make_user_player("player-sold-off-the-map");
    player.transfer_listed = true;
    player.market_value = 1_500_000;
    player.transfer_offers.push(make_pending_incoming_offer(
        "offer-sold-off-the-map",
        1_600_000,
    ));

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.teams[1].finance = 9_000_000;
    game.teams[1].transfer_budget = 6_000_000;
    game.teams
        .push(make_ai_team("team-3", "Far Rovers", 1_000_000, 500_000));
    game.teams
        .push(make_ai_team("team-4", "Far Albion", 1_000_000, 500_000));

    // Two competitions, neither of which lists the buying or the selling club.
    game.competitions.push(ofm_core::schedule::generate_league(
        "Far Division",
        2026,
        &["team-3".to_string(), "team-4".to_string()],
        game.clock.current_date,
    ));
    game.competitions.push(ofm_core::schedule::generate_league(
        "Far Cup Group",
        2026,
        &["team-3".to_string(), "team-4".to_string()],
        game.clock.current_date,
    ));
    game.sync_legacy_league();

    respond_to_offer(
        &mut game,
        "player-sold-off-the-map",
        "offer-sold-off-the-map",
        true,
    )
    .expect("accepting the offer should complete the sale");

    // The turn loop syncs the mirror again on its own schedule; anything held only there is gone.
    game.sync_legacy_league();

    assert!(
        game.competitions.iter().any(|competition| competition
            .transfer_log
            .iter()
            .any(|entry| entry.player_id == "player-sold-off-the-map")),
        "a completed transfer must be kept in a competition log, which is what the save reads, \
         not in the mirror the next sync overwrites"
    );
}

/// Take `team_id` down to exactly the minimum in forwards: the fixture's own
/// forward plus one depth forward. Any forward leaving now leaves it short.
fn leave_club_at_the_forward_floor(game: &mut Game, team_id: &str) {
    let spares = [
        format!("depth-{team_id}-Forward-1"),
        format!("depth-{team_id}-Forward-2"),
    ];
    game.players.retain(|player| !spares.contains(&player.id));
}

const WOULD_LEAVE_SHORT_OF_FORWARDS: &str =
    "be.error.squadFloor.wouldLeaveShort?group=common.positionGroups.Forward";

/// The bid is refused before the offer is recorded as agreed: the command layer
/// mutates the live game in place, so a refusal after that point would leave an
/// agreed offer on a player who never moved.
#[test]
fn squad_floor_a_bid_that_would_leave_the_seller_short_is_refused_before_anything_is_agreed() {
    let player = make_player("player-floor-bid");
    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    leave_club_at_the_forward_floor(&mut game, "team-2");

    let result = make_transfer_bid(&mut game, "player-floor-bid", 2_000_000);

    assert_eq!(result.err().as_deref(), Some(WOULD_LEAVE_SHORT_OF_FORWARDS));
    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-floor-bid")
        .unwrap();
    assert_eq!(player.team_id.as_deref(), Some("team-2"));
    assert!(
        player.transfer_offers.iter().all(|offer| !matches!(
            offer.status,
            TransferOfferStatus::Accepted | TransferOfferStatus::PendingRegistration
        )),
        "a refused bid left an agreed offer behind: {:?}",
        player.transfer_offers
    );
}

/// An agreement struck while the seller had players to spare, falling due
/// after it has lost them, lapses instead of taking the club below the floor.
#[test]
fn squad_floor_a_scheduled_sale_lapses_if_the_seller_has_since_reached_the_floor() {
    let player = make_player("player-floor-scheduled");
    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.clock.current_date = Utc.with_ymd_and_hms(2026, 12, 20, 12, 0, 0).unwrap();
    game.season_context.transfer_window.status = TransferWindowStatus::Closed;
    game.season_context.transfer_window.opens_on = Some("2027-01-01".to_string());
    make_transfer_bid(&mut game, "player-floor-scheduled", 2_000_000)
        .expect("the seller had depth when the deal was agreed");

    leave_club_at_the_forward_floor(&mut game, "team-2");
    game.clock.current_date = Utc.with_ymd_and_hms(2027, 1, 1, 12, 0, 0).unwrap();
    game.season_context.transfer_window.status = TransferWindowStatus::Open;
    process_pending_transfer_registrations(&mut game);

    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-floor-scheduled")
        .unwrap();
    assert_eq!(player.team_id.as_deref(), Some("team-2"));
    assert_eq!(
        player.transfer_offers[0].status,
        TransferOfferStatus::Withdrawn
    );
    assert!(
        player
            .movement_history
            .iter()
            .all(|entry| entry.kind != PlayerMovementKind::PermanentTransfer),
        "the refused sale was half-applied"
    );
}

#[test]
fn squad_floor_the_player_cannot_accept_a_sale_that_leaves_their_own_squad_short() {
    let mut player = make_user_player("player-floor-own-sale");
    player
        .transfer_offers
        .push(make_pending_incoming_offer("offer-floor", 1_500_000));
    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    leave_club_at_the_forward_floor(&mut game, "team-1");

    let result = respond_to_offer(&mut game, "player-floor-own-sale", "offer-floor", true);

    assert_eq!(result.err().as_deref(), Some(WOULD_LEAVE_SHORT_OF_FORWARDS));
    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-floor-own-sale")
        .unwrap();
    assert_eq!(player.team_id.as_deref(), Some("team-1"));
    assert_eq!(
        player.transfer_offers[0].status,
        TransferOfferStatus::Pending
    );
}

/// Given an AI club of seventeen seniors and a buyer with money to burn, when
/// the buyer bids big for every one of them, then two sales go through and
/// every later one is refused: the club keeps fifteen, whoever it sells.
#[test]
fn squad_floor_a_club_that_sells_aggressively_never_goes_below_fifteen() {
    let player = make_player("player-sell-all");
    let mut game = make_game_with_player(player, vec![], 1_000_000_000, 1_000_000_000);
    let seller_seniors = |game: &Game| {
        game.players
            .iter()
            .filter(|p| p.team_id.as_deref() == Some("team-2"))
            .count()
    };
    assert_eq!(seller_seniors(&game), 17);
    let targets: Vec<String> = game
        .players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some("team-2"))
        .map(|p| p.id.clone())
        .collect();

    let mut sold = 0;
    let mut refused = 0;
    for target in targets {
        match make_transfer_bid(&mut game, &target, 50_000_000) {
            Ok(outcome) if outcome.decision == TransferNegotiationDecision::Accepted => sold += 1,
            Ok(_) => {}
            Err(error) if error.starts_with("be.error.squadFloor.") => refused += 1,
            Err(error) => panic!("unexpected refusal: {error}"),
        }
    }

    assert_eq!(sold, 2);
    assert_eq!(refused, 15);
    assert_eq!(seller_seniors(&game), 15);
}

/// The best target on the market belongs to a club at the floor in his group,
/// the second best to a club with depth. The buyer signs the second: it does
/// not sell the first club short, and it does not spend its one approach of
/// the day on a sale that was always going to be refused.
#[test]
fn squad_floor_ai_buyers_pass_over_a_player_whose_club_cannot_sell_him() {
    let mut floor_bound = make_player("player-floor-ai");
    floor_bound.team_id = Some("team-3".to_string());
    floor_bound.stage_contract_end(Some("2026-09-01".to_string()));
    floor_bound.market_value = 1_200_000;
    floor_bound.transfer_listed = true;
    let mut available = make_player("player-floor-ai-alt");
    available.team_id = Some("team-4".to_string());
    available.stage_contract_end(Some("2026-09-01".to_string()));
    available.market_value = 1_000_000;
    available.transfer_listed = true;

    let mut game = make_game_with_player(floor_bound, vec![], 5_000_000, 2_000_000);
    game.players.push(available);
    // Neither selling club can afford to buy, so team-2 is the only buyer.
    game.teams
        .push(make_ai_team("team-3", "Seller FC", 3_000_000, 0));
    game.teams
        .push(make_ai_team("team-4", "Other Seller", 3_000_000, 0));
    game.teams[1].finance = 6_000_000;
    game.teams[1].transfer_budget = 3_000_000;
    give_every_club_squad_depth(&mut game);
    leave_club_at_the_forward_floor(&mut game, "team-3");
    attach_transfer_log_league(&mut game);

    evaluate_transfer_market(&mut game);

    let team_of = |id: &str| {
        game.players
            .iter()
            .find(|player| player.id == id)
            .and_then(|player| player.team_id.clone())
    };
    assert_eq!(team_of("player-floor-ai").as_deref(), Some("team-3"));
    assert_eq!(team_of("player-floor-ai-alt").as_deref(), Some("team-2"));
}

// ---------------------------------------------------------------------------
// Contract history: a move creates a real new contract with the buyer's terms.
// ---------------------------------------------------------------------------

use domain::contract_ledger::ContractSource;

fn entry_of_kind(
    player: &Player,
    kind: PlayerMovementKind,
) -> &domain::player::PlayerMovementEntry {
    player
        .movement_history
        .iter()
        .rev()
        .find(|entry| entry.kind == kind)
        .unwrap_or_else(|| panic!("no {kind:?} entry in the ledger"))
}

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
        Some("2029-08-01"),
        "a 26-year-old's standard contract is three years from the day of the move"
    );
    assert!(
        record.weekly_wage > 7_777 && record.weekly_wage.is_multiple_of(1_000),
        "the wage is the buyer's standard wage, not the seller's 7,777: {}",
        record.weekly_wage
    );
    assert_eq!(player.contract_end(), Some("2029-08-01"));
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
    assert_eq!(record.end.as_deref(), Some("2030-01-01"));
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

/// Enough players in every group that the squad floor is met without the target, so the
/// floor's waiver of the wage policy cannot apply to the club being tested.
fn give_depth(game: &mut Game, team_id: &str) {
    for ((group, _), count) in ofm_core::squad_floor::MIN_PLAYERS_PER_GROUP
        .into_iter()
        .zip([3, 6, 6, 4])
    {
        for index in 0..count {
            let mut depth = make_player(&format!("depth-{team_id}-{group:?}-{index}"));
            depth.team_id = Some(team_id.to_string());
            depth.position = group.clone();
            depth.natural_position = group.clone();
            game.players.push(depth);
        }
    }
}

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
