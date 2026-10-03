use super::evaluate_transfer_market;
use crate::clock::GameClock;
use crate::game::Game;
use chrono::{TimeZone, Utc};
use domain::manager::Manager;
use domain::player::{Player, PlayerAttributes, Position, TransferOfferStatus};
use domain::season::TransferWindowStatus;
use domain::team::Team;

fn make_team(id: &str, name: &str, reputation: u32) -> Team {
    let mut team = Team::new(
        id.to_string(),
        name.to_string(),
        name[..3].to_string(),
        "England".to_string(),
        "Testville".to_string(),
        format!("{} Ground", name),
        25_000,
    );
    team.reputation = reputation;
    team.finance = 5_000_000;
    team.transfer_budget = 5_000_000;
    team.wage_budget = 2_000_000;
    team
}

fn sample_attributes() -> PlayerAttributes {
    PlayerAttributes {
        pace: 68,
        stamina: 66,
        strength: 64,
        agility: 67,
        passing: 65,
        shooting: 72,
        tackling: 38,
        dribbling: 69,
        defending: 35,
        positioning: 66,
        vision: 63,
        decisions: 61,
        composure: 62,
        aggression: 48,
        teamwork: 58,
        leadership: 44,
        handling: 12,
        reflexes: 14,
        aerial: 40,
    }
}

fn make_game() -> Game {
    let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 1, 12, 12, 0, 0).unwrap());
    let mut manager = Manager::new(
        "mgr-user".to_string(),
        "Alex".to_string(),
        "Boss".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("team1".to_string());

    let mut player = Player::new(
        "player-award".to_string(),
        "Golden".to_string(),
        "Golden Boot".to_string(),
        "1998-04-01".to_string(),
        "England".to_string(),
        Position::Forward,
        sample_attributes(),
    );
    player.team_id = Some("team1".to_string());
    player.market_value = 600_000;
    player.stage_wage(18_000);
    player.morale = 58;
    player.stage_contract_end(Some("2027-06-30".to_string()));
    player.stats.appearances = 6;
    player.stats.goals = 19;

    let mut game = Game::new(
        clock,
        manager,
        vec![
            make_team("team1", "Alpha FC", 620),
            make_team("team2", "Beta FC", 690),
        ],
        vec![player],
        vec![],
        vec![],
    );
    game.season_context.transfer_window.status = TransferWindowStatus::Open;
    game
}

#[test]
fn evaluate_transfer_market_targets_award_leaderboard_user_player() {
    let mut game = make_game();

    evaluate_transfer_market(&mut game);

    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-award")
        .expect("award leaderboard player should exist");

    assert!(
        player.transfer_offers.iter().any(|offer| {
            offer.from_team_id == "team2" && offer.status == TransferOfferStatus::Pending
        }),
        "Award-leaderboard players should attract AI bids even when their base transfer-interest score is otherwise too low"
    );
    assert!(
        game.messages
            .iter()
            .any(|message| { message.context.player_id.as_deref() == Some("player-award") }),
        "The incoming bid should surface through the usual inbox flow"
    );
}

#[test]
fn dormant_clubs_outside_the_active_scope_skip_the_market() {
    use domain::league::{League, StandingEntry};

    let mut game = make_game();
    // team3 plays in the actively-simulated competition; team2 is moved into
    // a dormant competition the player isn't simulating in full.
    game.teams.push(make_team("team3", "Gamma FC", 700));

    let active = League {
        id: "active-league".to_string(),
        standings: vec![
            StandingEntry::new("team1".to_string()),
            StandingEntry::new("team3".to_string()),
        ],
        ..Default::default()
    };
    let dormant = League {
        id: "dormant-league".to_string(),
        standings: vec![StandingEntry::new("team2".to_string())],
        ..Default::default()
    };
    game.competitions = vec![active, dormant];
    game.active_competition_ids = vec!["active-league".to_string()];

    evaluate_transfer_market(&mut game);

    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-award")
        .expect("award leaderboard player should exist");

    assert!(
        player
            .transfer_offers
            .iter()
            .any(|offer| offer.from_team_id == "team3"),
        "an active club should still bid on the user's standout player"
    );
    assert!(
        !player
            .transfer_offers
            .iter()
            .any(|offer| offer.from_team_id == "team2"),
        "a dormant club outside the active simulation scope must not shop the market"
    );
}

fn contract_end_loan_game(user_is_parent: bool) -> Game {
    let mut game = make_game();
    let player = &mut game.players[0];
    player.team_id = Some(if user_is_parent { "team1" } else { "team2" }.to_string());
    player.loan_listed = true;
    player.stage_contract_end(Some("2026-04-12".to_string()));
    player.stage_wage(1_000);
    player.adopt_legacy_contract();

    // These scenarios exercise the contract boundary; both clubs can spare a player.
    for team_id in ["team1", "team2"] {
        for (position, count) in [
            (Position::Goalkeeper, 3),
            (Position::Defender, 5),
            (Position::Midfielder, 5),
            (Position::Forward, 3),
        ] {
            for index in 0..count {
                let id = format!("depth-{team_id}-{position:?}-{index}");
                let mut player = Player::new(
                    id.clone(),
                    id.clone(),
                    id,
                    "1998-01-01".to_string(),
                    "England".to_string(),
                    position.clone(),
                    sample_attributes(),
                );
                player.team_id = Some(team_id.to_string());
                game.players.push(player);
            }
        }
    }
    game
}

fn add_contract_end_loan_offer(game: &mut Game, end_date: &str) {
    game.players[0].loan_offers.push(domain::player::LoanOffer {
        id: "loan-contract-end".to_string(),
        from_team_id: "team2".to_string(),
        parent_team_id: "team1".to_string(),
        start_date: "2026-01-12".to_string(),
        end_date: end_date.to_string(),
        wage_contribution_pct: 50,
        buy_option_fee: None,
        last_manager_wage_contribution_pct: None,
        last_manager_end_date: None,
        last_manager_buy_option_fee: None,
        negotiation_round: 1,
        suggested_wage_contribution_pct: None,
        suggested_end_date: None,
        suggested_buy_option_fee: None,
        status: domain::player::LoanOfferStatus::Pending,
        date: "2026-01-12".to_string(),
        closed_on: None,
    });
}

/// Given a short contract, when the AI proposes a loan, then it can end on the contract date.
#[test]
fn an_ai_loan_default_can_end_on_the_contract_date() {
    let game = contract_end_loan_game(true);
    assert_eq!(
        super::default_loan_end_date(game.clock.current_date.date_naive(), &game.players[0]),
        Some("2026-04-12".to_string()),
    );
}

/// Given exactly thirty contract days left, when the AI proposes a loan, then the minimum term is available.
#[test]
fn an_ai_loan_default_allows_exactly_thirty_contract_days() {
    let mut game = make_game();
    game.players[0].stage_contract_end(Some("2026-02-11".to_string()));
    assert_eq!(
        super::default_loan_end_date(game.clock.current_date.date_naive(), &game.players[0]),
        Some("2026-02-11".to_string()),
    );
}

/// Given fewer than thirty contract days left, when the AI proposes a loan, then no term is available.
#[test]
fn an_ai_loan_default_refuses_fewer_than_thirty_contract_days() {
    let mut game = make_game();
    game.players[0].stage_contract_end(Some("2026-02-10".to_string()));
    assert_eq!(
        super::default_loan_end_date(game.clock.current_date.date_naive(), &game.players[0]),
        None
    );
}

/// Given a listed player, when the user bids through the contract date, then the loan registers.
#[test]
fn an_outgoing_loan_can_end_on_the_contract_date() {
    let mut game = contract_end_loan_game(false);
    let outcome =
        super::make_loan_offer(&mut game, "player-award", "2026-04-12", 100, None).unwrap();
    assert_eq!(outcome.decision, super::LoanOfferDecision::Accepted);
    assert_eq!(
        game.players[0].active_loan.as_ref().unwrap().end_date,
        "2026-04-12"
    );
}

/// Given an incoming offer, when accepted through the contract date, then the AI borrower registers it.
#[test]
fn an_incoming_loan_can_end_on_the_contract_date() {
    let mut game = contract_end_loan_game(true);
    add_contract_end_loan_offer(&mut game, "2026-04-12");
    super::respond_to_loan_offer(&mut game, "player-award", "loan-contract-end", true).unwrap();
    assert_eq!(
        game.players[0].active_loan.as_ref().unwrap().end_date,
        "2026-04-12"
    );
}

/// Given an incoming offer, when countered through the contract date, then the accepted counter registers.
#[test]
fn a_countered_loan_can_end_on_the_contract_date() {
    let mut game = contract_end_loan_game(true);
    add_contract_end_loan_offer(&mut game, "2026-04-01");
    let outcome = super::counter_loan_offer(
        &mut game,
        "player-award",
        "loan-contract-end",
        "2026-04-12",
        60,
        None,
    )
    .unwrap();
    assert_eq!(outcome.decision, super::LoanOfferDecision::Accepted);
    assert_eq!(
        game.players[0].active_loan.as_ref().unwrap().end_date,
        "2026-04-12"
    );
}

/// Given a closed window, when a loan ending with the contract is agreed and reloaded,
/// then it registers once when the window opens with the same end date.
#[test]
fn a_pending_contract_end_loan_registers_after_reload() {
    let mut game = contract_end_loan_game(false);
    game.season_context.transfer_window.status = TransferWindowStatus::Closed;
    game.season_context.transfer_window.opens_on = Some("2026-02-01".to_string());
    super::make_loan_offer(&mut game, "player-award", "2026-04-12", 100, None).unwrap();
    assert!(game.players[0].active_loan.is_none());
    let mut game: Game = serde_json::from_str(&serde_json::to_string(&game).unwrap()).unwrap();
    game.clock.current_date = Utc.with_ymd_and_hms(2026, 2, 1, 12, 0, 0).unwrap();
    game.season_context.transfer_window.status = TransferWindowStatus::Open;
    super::process_pending_loan_registrations(&mut game);
    super::process_pending_loan_registrations(&mut game);
    assert_eq!(game.players[0].team_id.as_deref(), Some("team1"));
    assert_eq!(
        game.players[0].active_loan.as_ref().unwrap().end_date,
        "2026-04-12"
    );
    assert_eq!(
        game.players[0].loan_offers[0].status,
        domain::player::LoanOfferStatus::Accepted
    );
    assert_eq!(
        game.players[0]
            .movement_history
            .iter()
            .filter(|entry| { entry.kind == domain::player::PlayerMovementKind::LoanStart })
            .count(),
        1,
    );
}

fn open_loan_window_and_register(game: &mut Game) {
    game.clock.current_date = Utc.with_ymd_and_hms(2026, 2, 1, 12, 0, 0).unwrap();
    game.season_context.transfer_window.status = TransferWindowStatus::Open;
    super::process_pending_loan_registrations(game);
    assert_eq!(game.players[0].team_id.as_deref(), Some("team2"));
    assert_eq!(
        game.players[0].active_loan.as_ref().unwrap().end_date,
        "2026-04-12"
    );
}

/// Given a closed window and an incoming loan ending with the contract,
/// when accepted, then it waits for the window and registers with its agreed end.
#[test]
fn a_pending_incoming_loan_can_end_on_the_contract_date() {
    let mut game = contract_end_loan_game(true);
    add_contract_end_loan_offer(&mut game, "2026-04-12");
    game.season_context.transfer_window.status = TransferWindowStatus::Closed;
    game.season_context.transfer_window.opens_on = Some("2026-02-01".to_string());
    super::respond_to_loan_offer(&mut game, "player-award", "loan-contract-end", true).unwrap();
    assert!(game.players[0].active_loan.is_none());
    open_loan_window_and_register(&mut game);
}

/// Given a closed window and an incoming loan, when countered through the contract date,
/// then the accepted counter waits for the window and registers with its agreed end.
#[test]
fn a_pending_countered_loan_can_end_on_the_contract_date() {
    let mut game = contract_end_loan_game(true);
    add_contract_end_loan_offer(&mut game, "2026-04-01");
    game.season_context.transfer_window.status = TransferWindowStatus::Closed;
    game.season_context.transfer_window.opens_on = Some("2026-02-01".to_string());
    let outcome = super::counter_loan_offer(
        &mut game,
        "player-award",
        "loan-contract-end",
        "2026-04-12",
        60,
        None,
    )
    .unwrap();
    assert_eq!(outcome.decision, super::LoanOfferDecision::Accepted);
    assert!(game.players[0].active_loan.is_none());
    open_loan_window_and_register(&mut game);
}

/// Given a saved pending loan ending beyond the contract, when registration runs,
/// then the agreement is withdrawn and the player stays with the parent.
#[test]
fn a_pending_loan_cannot_outlive_the_contract() {
    let mut game = contract_end_loan_game(true);
    add_contract_end_loan_offer(&mut game, "2026-04-13");
    game.players[0].loan_offers[0].status = domain::player::LoanOfferStatus::PendingRegistration;
    game.players[0].loan_offers[0].start_date = "2026-02-01".to_string();
    game.clock.current_date = Utc.with_ymd_and_hms(2026, 2, 1, 12, 0, 0).unwrap();
    super::process_pending_loan_registrations(&mut game);
    assert_eq!(game.players[0].team_id.as_deref(), Some("team1"));
    assert!(game.players[0].active_loan.is_none());
    assert_eq!(
        game.players[0].loan_offers[0].status,
        domain::player::LoanOfferStatus::Withdrawn
    );
}

/// Given a loan running to the contract date, when contract expiry and loan returns run,
/// then the player is released rather than restored to the parent club.
#[test]
fn a_contract_end_loan_does_not_restore_an_expired_player() {
    let mut game = contract_end_loan_game(false);
    super::make_loan_offer(&mut game, "player-award", "2026-04-12", 100, None).unwrap();
    game.clock.current_date = Utc.with_ymd_and_hms(2026, 4, 12, 12, 0, 0).unwrap();
    crate::contracts::process_contract_expiries(&mut game);
    super::process_loan_returns(&mut game);
    assert!(game.players[0].team_id.is_none());
    assert!(game.players[0].active_loan.is_none());
    assert!(game.players[0].contract_end().is_none());
}

/// Given a requested end after the contract, when bidding, then no offer or loan is created.
#[test]
fn an_outgoing_loan_cannot_outlive_the_contract() {
    let mut game = contract_end_loan_game(false);
    assert!(super::make_loan_offer(&mut game, "player-award", "2026-04-13", 100, None).is_err());
    assert!(game.players[0].loan_offers.is_empty());
    assert!(game.players[0].active_loan.is_none());
}

/// Given an incoming offer ending after the contract, when accepted, then it remains pending.
#[test]
fn an_incoming_loan_cannot_outlive_the_contract() {
    let mut game = contract_end_loan_game(true);
    add_contract_end_loan_offer(&mut game, "2026-04-13");
    assert!(
        super::respond_to_loan_offer(&mut game, "player-award", "loan-contract-end", true).is_err()
    );
    assert_eq!(
        game.players[0].loan_offers[0].status,
        domain::player::LoanOfferStatus::Pending
    );
    assert!(game.players[0].active_loan.is_none());
}

/// Given an incoming offer, when countered beyond the contract, then its terms remain unchanged.
#[test]
fn a_countered_loan_cannot_outlive_the_contract() {
    let mut game = contract_end_loan_game(true);
    add_contract_end_loan_offer(&mut game, "2026-04-01");
    assert!(
        super::counter_loan_offer(
            &mut game,
            "player-award",
            "loan-contract-end",
            "2026-04-13",
            60,
            None
        )
        .is_err()
    );
    assert_eq!(game.players[0].loan_offers[0].end_date, "2026-04-01");
    assert!(game.players[0].active_loan.is_none());
}
