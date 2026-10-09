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

pub(super) fn sample_attributes() -> PlayerAttributes {
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

pub(super) fn make_game() -> Game {
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

// Shared fixtures for the acceptance routes, with enough depth to permit a departure.
pub(super) fn acceptance_game(owner: &str, deferred: bool) -> Game {
    let mut game = make_game();
    game.players[0].team_id = Some(owner.to_string());
    game.players[0].loan_listed = true;
    for (position, count) in [
        (Position::Goalkeeper, 3),
        (Position::Defender, 5),
        (Position::Midfielder, 5),
        (Position::Forward, 3),
    ] {
        for index in 0..count {
            let mut player = Player::new(
                format!("depth-{position:?}-{index}"),
                "Depth".into(),
                "Player".into(),
                "1998-01-01".into(),
                "England".into(),
                position.clone(),
                sample_attributes(),
            );
            player.team_id = Some(owner.to_string());
            game.players.push(player);
        }
    }
    if deferred {
        game.season_context.transfer_window.status = TransferWindowStatus::Closed;
        game.season_context.transfer_window.opens_on = Some("2026-07-02".into());
    }
    game
}

pub(super) fn incoming_transfer(game: &mut Game) -> String {
    super::bids::upsert_transfer_offer(
        &mut game.players[0],
        "team2",
        600_000,
        TransferOfferStatus::Pending,
        "2026-01-12",
        None,
        1,
        None,
        None,
    )
}

pub(super) fn incoming_loan(game: &mut Game) -> String {
    super::loans::upsert_loan_offer(
        &mut game.players[0],
        "team2",
        "team1",
        "2026-01-12",
        "2026-12-20",
        60,
        Some(600_000),
        domain::player::LoanOfferStatus::Pending,
        "2026-01-12",
    )
}

pub(super) fn assert_acceptance_notice<'a>(
    game: &'a Game,
    key: &str,
    start: &str,
) -> &'a domain::message::InboxMessage {
    assert_eq!(game.messages.len(), 1, "one notice for the agreed deal");
    let message = &game.messages[0];
    assert_eq!(message.body_key.as_deref(), Some(key));
    assert_eq!(message.context.player_id.as_deref(), Some("player-award"));
    assert_eq!(message.context.team_id.as_deref(), Some("team1"));
    assert_eq!(
        message.i18n_params.get("player").map(String::as_str),
        Some("Golden Boot")
    );
    if !start.is_empty() {
        assert_eq!(
            message.i18n_params.get("start").map(String::as_str),
            Some(start)
        );
    }
    assert_eq!(message.category, domain::message::MessageCategory::Transfer);
    assert_eq!(
        message.sender_key.as_deref(),
        Some("be.sender.transferCommittee")
    );
    assert_eq!(
        message.sender_role_key.as_deref(),
        Some("be.role.directorOfFootball")
    );
    assert!(game.emitted_events.contains(&message.id));
    message
}
