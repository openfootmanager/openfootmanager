use chrono::{TimeZone, Utc};
use domain::league::League;
use domain::manager::Manager;
use domain::message::MessageCategory;
use domain::news::{NewsArticle, NewsCategory};
use domain::player::{
    Player, PlayerAttributes, PlayerIssueCategory, Position, TransferOffer, TransferOfferStatus,
};
use domain::season::TransferWindowStatus;
use domain::team::Team;
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::transfers::{
    TransferNegotiationDecision, counter_offer, evaluate_transfer_market,
    generate_incoming_transfer_offers, make_transfer_bid, respond_to_offer,
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
fn transfer_bid_is_rejected_when_window_is_closed() {
    let player = make_player("player-bid-closed");
    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.season_context.transfer_window.status = TransferWindowStatus::Closed;

    let error = make_transfer_bid(&mut game, "player-bid-closed", 1_000_000)
        .expect_err("closed transfer window should reject bids");

    assert_eq!(error, "Transfer window is closed");
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

    assert_eq!(error, "Transfer budget too low");
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
    let mut player = make_user_player("player-message-ids");
    player.contract_end = Some("2026-09-01".to_string());
    player.market_value = 1_200_000;

    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
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
