//! A club never runs out of players, whatever the day throws at it.
//!
//! `squad_floor`'s own tests pin the rule. These pin that a day of the game
//! actually applies it: at kick-off, and after the steps that take players away.

use super::*;
use ofm_core::squad_floor::squad_shortfall;

fn registered(game: &Game, team_id: &str) -> usize {
    game.players
        .iter()
        .filter(|player| player.team_id.as_deref() == Some(team_id))
        .count()
}

/// A club with nobody on its books used to reach the match engine with an
/// empty side. It now signs a squad from the free-agent market at kick-off,
/// and the players it signed are the ones who play — the day's later check
/// would refill the club too, but only after a match played by nobody.
#[test]
fn a_club_with_nobody_registered_signs_a_squad_before_kick_off() {
    let mut game = make_game_with_match();
    game.players
        .retain(|player| player.team_id.as_deref() != Some("team2"));
    add_free_agent_pool(&mut game, "a");
    add_free_agent_pool(&mut game, "b");

    turn::process_day(&mut game);

    let fixture = &game.league.as_ref().unwrap().fixtures[0];
    assert_eq!(fixture.status, FixtureStatus::Completed);
    // Eleven signed players started it; on the one match path a manager may
    // have brought more of them on from the bench.
    assert!(fielded_count(&game, "team2") >= 11);
    assert!(squad_shortfall(&game, "team2").is_empty());
}

/// The player's club, without a goalkeeper on the day of a match, is topped up
/// before kick-off and told who arrived.
#[test]
fn the_players_club_without_a_keeper_is_topped_up_before_its_match() {
    let mut game = make_game_with_match();
    game.players.retain(|player| player.id != "t1_gk");
    add_free_agent_pool(&mut game, "a");
    add_free_agent_pool(&mut game, "b");

    turn::process_day(&mut game);

    let fixture = &game.league.as_ref().unwrap().fixtures[0];
    assert_eq!(fixture.status, FixtureStatus::Completed);
    assert!(squad_shortfall(&game, "team1").is_empty());
    assert!(
        game.messages
            .iter()
            .any(|message| { message.body_key.as_deref() == Some("be.msg.squadToppedUp.body") })
    );
}

/// The player's own match, started live: the same top-up happens before the
/// session is built, and the keeper who was signed is on the pitch.
#[test]
fn a_live_match_kicks_off_with_a_keeper_signed_for_a_club_that_had_none() {
    use ofm_core::live_match_manager::{MatchMode, kick_off_live_match};

    let mut game = make_game_with_match();
    game.players.retain(|player| player.id != "t1_gk");
    add_free_agent_pool(&mut game, "a");

    let session =
        kick_off_live_match(&mut game, 0, MatchMode::Live, false).expect("the match kicks off");

    assert!(squad_shortfall(&game, "team1").is_empty());
    let keeper_on_the_pitch = session.snapshot().home_team.players.iter().any(|player| {
        game.players.iter().any(|signed| {
            signed.id == player.id && signed.position.to_group_position() == Position::Goalkeeper
        })
    });
    assert!(
        keeper_on_the_pitch,
        "the home side kicked off without a keeper"
    );
    assert!(
        game.messages
            .iter()
            .any(|message| { message.body_key.as_deref() == Some("be.msg.squadToppedUp.body") })
    );
}

/// Given an AI club with nobody, no academy and an empty market, and a fixture
/// today, the day still finishes and the fixture is played: nobody is created
/// to fill the side, and the gap is logged instead.
#[test]
fn an_unfillable_ai_club_still_finishes_the_day() {
    let mut game = make_game_with_match();
    game.players
        .retain(|player| player.team_id.as_deref() != Some("team2"));
    let players_before = game.players.len();

    turn::process_day(&mut game);

    let fixture = &game.league.as_ref().unwrap().fixtures[0];
    assert_eq!(fixture.status, FixtureStatus::Completed);
    assert_eq!(game.players.len(), players_before, "a player was created");
    assert_eq!(registered(&game, "team2"), 0);
}

/// Given an AI club of exactly fifteen seniors, one of them a defender on loan
/// from the player's club until three weeks from now, and free agents on the
/// market, when the days up to and past the loan's end are played, then the
/// defender goes home and the club never needs an emergency top-up: its weekly
/// planning saw it at the floor and brought players in before the loss.
#[test]
fn weekly_planning_covers_a_loss_in_the_middle_of_a_season() {
    let mut game = make_game_without_match_today();
    for (position, id) in [
        (Position::Goalkeeper, "t2_extra_gk"),
        (Position::Midfielder, "t2_extra_mid"),
        (Position::Forward, "t2_extra_fwd"),
    ] {
        game.players.push(make_player(id, id, "team2", position));
    }
    let loan_end = (game.clock.current_date + chrono::Duration::days(20))
        .format("%Y-%m-%d")
        .to_string();
    let loanee = game.players.iter_mut().find(|p| p.id == "t1_def0").unwrap();
    loanee.team_id = Some("team2".to_string());
    loanee.active_loan = Some(domain::player::ActiveLoan {
        parent_team_id: "team1".to_string(),
        loan_team_id: "team2".to_string(),
        start_date: "2025-01-01".to_string(),
        end_date: loan_end,
        wage_contribution_pct: 100,
        buy_option_fee: None,
        loan_start_minutes: 0,
        loan_start_appearances: 0,
        development_reported_minutes: 0,
        development_reported_appearances: 0,
    });
    assert_eq!(registered(&game, "team2"), 15);
    add_free_agent_pool(&mut game, "a");

    for _ in 0..30 {
        turn::process_day(&mut game);
    }

    let loanee = game.players.iter().find(|p| p.id == "t1_def0").unwrap();
    assert_eq!(loanee.team_id.as_deref(), Some("team1"), "the loan ended");
    let emergencies: Vec<_> = game
        .squad_floor_top_ups
        .iter()
        .filter(|top_up| top_up.team_id == "team2")
        .collect();
    assert!(emergencies.is_empty(), "emergency top-ups: {emergencies:?}");
}

/// Given the player's club without a keeper on the day of its match, and a
/// free-agent keeper on the market, when the manager delegates the match, then
/// the one call that plays the user's matchday kicks off through the squad
/// floor's gate: the keeper is signed before the side is picked, and the
/// manager is told.
#[test]
fn the_users_delegated_match_kicks_off_with_a_keeper_signed() {
    use ofm_core::matchday;

    let mut game = make_game_with_match();
    game.players.retain(|player| player.id != "t1_gk");
    add_free_agent_pool(&mut game, "a");

    matchday::play_user_matchday_with_capture(&mut game, None, 0, &mut |_| {})
        .expect("the delegated match is played");

    assert!(
        game.players.iter().any(|player| {
            player.team_id.as_deref() == Some("team1")
                && player.position.to_group_position() == Position::Goalkeeper
        }),
        "the player's club played its match with no keeper signed"
    );
    assert!(
        game.messages
            .iter()
            .any(|message| { message.body_key.as_deref() == Some("be.msg.squadToppedUp.body") })
    );
}

/// Every contract at both clubs ends today. The AI club is back at the floor
/// by the end of the day; the player's club is warned and left to choose.
#[test]
fn after_contracts_expire_an_ai_club_is_topped_up_and_the_players_club_is_warned() {
    let mut game = make_game_without_match_today();
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    for player in game.players.iter_mut() {
        player.stage_contract_end(Some(today.clone()));
    }

    turn::process_day(&mut game);

    assert!(squad_shortfall(&game, "team2").is_empty());
    assert_eq!(registered(&game, "team1"), 0);
    assert!(
        game.messages
            .iter()
            .any(|message| { message.body_key.as_deref() == Some("be.msg.squadBelowFloor.body") })
    );
}
