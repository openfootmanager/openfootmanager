//! What a manager sees and sets of the shootout orders in a live session: their own side's order,
//! never the opponent's, which stays hidden until each kick is taken.

use chrono::{TimeZone, Utc};
use domain::league::{Fixture, FixtureCompetition, FixtureStatus, League, StandingEntry};
use domain::manager::Manager;
use domain::player::{Player, PlayerAttributes, Position};
use domain::team::Team;
use engine::{MatchCommand, MatchPhase, ShootoutOrderSnapshot, Side};
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::live_match_manager::{self, LiveMatchSession, MatchMode};

const NOT_YOUR_SIDE: &str = "be.error.liveMatch.shootoutOrderNotYourSide";

fn attributes() -> PlayerAttributes {
    PlayerAttributes {
        pace: 65,
        stamina: 65,
        strength: 65,
        agility: 65,
        passing: 65,
        shooting: 65,
        tackling: 65,
        dribbling: 65,
        defending: 65,
        positioning: 65,
        vision: 65,
        decisions: 65,
        composure: 65,
        aggression: 65,
        teamwork: 65,
        leadership: 65,
        handling: 65,
        reflexes: 65,
        aerial: 65,
    }
}

fn squad(team_id: &str) -> Vec<Player> {
    let mut players = Vec::new();
    for (position, count) in [
        (Position::Goalkeeper, 2),
        (Position::Defender, 6),
        (Position::Midfielder, 6),
        (Position::Forward, 4),
    ] {
        for index in 0..count {
            let id = format!("{team_id}_{position:?}{index}");
            let mut player = Player::new(
                id.clone(),
                id.clone(),
                id,
                "1995-01-01".to_string(),
                "England".to_string(),
                position.clone(),
                attributes(),
            );
            player.team_id = Some(team_id.to_string());
            player.condition = 100;
            players.push(player);
        }
    }
    players
}

fn club(id: &str) -> Team {
    Team::new(
        id.to_string(),
        format!("{id} FC"),
        id[..3].to_uppercase(),
        "England".to_string(),
        "London".to_string(),
        "Stadium".to_string(),
        30_000,
    )
}

/// A fixture between alpha (home) and bravo (away), with the user managing `managed`.
fn game_managing(managed: Option<&str>) -> Game {
    let date = Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap();
    let mut manager = Manager::new(
        "mgr".to_string(),
        "Probe".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.team_id = managed.map(str::to_string);
    let mut players = squad("alpha");
    players.extend(squad("bravo"));
    let league = League {
        id: "league1".to_string(),
        name: "Test League".to_string(),
        season: 1,
        fixtures: vec![Fixture {
            id: "fix1".to_string(),
            matchday: 1,
            date: "2025-06-15".to_string(),
            home_team_id: "alpha".to_string(),
            away_team_id: "bravo".to_string(),
            competition: FixtureCompetition::League,
            status: FixtureStatus::Scheduled,
            result: None,
            ..Default::default()
        }],
        standings: vec![
            StandingEntry::new("alpha".to_string()),
            StandingEntry::new("bravo".to_string()),
        ],
        ..Default::default()
    };
    let mut game = Game::new(
        GameClock::new(date),
        manager,
        vec![club("alpha"), club("bravo")],
        players,
        vec![],
        vec![],
    );
    game.league = Some(league);
    game
}

/// A live session played level to the start of its shootout: no kick taken yet.
fn session_at_the_shootout(managed: Option<&str>) -> LiveMatchSession {
    let game = game_managing(managed);
    let mut session =
        live_match_manager::create_live_match(&game, 0, MatchMode::Live, true).unwrap();
    while session.match_state.phase() != MatchPhase::PenaltyShootout {
        if matches!(
            session.match_state.phase(),
            MatchPhase::FullTime | MatchPhase::ExtraTimeEnd
        ) {
            session.match_state.test_set_score(0, 0);
        }
        session.step();
    }
    session
}

/// The order of `side` as the session's snapshot shows it.
fn shown_order(session: &LiveMatchSession, side: Side) -> Option<ShootoutOrderSnapshot> {
    let shootout = session
        .snapshot()
        .penalty_shootout
        .expect("a shootout is on");
    match side {
        Side::Home => shootout.home_order,
        Side::Away => shootout.away_order,
    }
}

/// Given a manager of either club at the start of a shootout,
/// When they look at the match,
/// Then they see their own side's order and not the opponent's.
#[test]
fn a_manager_sees_their_own_shootout_order_and_not_the_opponents() {
    for (managed, own, opponent) in [
        ("alpha", Side::Home, Side::Away),
        ("bravo", Side::Away, Side::Home),
    ] {
        let session = session_at_the_shootout(Some(managed));
        let own_order = session.match_state.shootout_order(own);
        assert!(own_order.is_some(), "the engine has {managed}'s order");
        assert_eq!(shown_order(&session, own), own_order, "{managed}");
        assert_eq!(shown_order(&session, opponent), None, "{managed}");
    }
}

/// Given a match between two clubs the user does not manage,
/// When the user looks at it at the start of its shootout,
/// Then neither side's order is shown.
#[test]
fn a_match_between_two_other_clubs_shows_neither_order() {
    let session = session_at_the_shootout(None);
    assert_eq!(shown_order(&session, Side::Home), None);
    assert_eq!(shown_order(&session, Side::Away), None);
}

/// Given the home manager at the start of a shootout,
/// When they set their own side's order in reverse,
/// Then it is accepted and the snapshot shows it.
#[test]
fn a_manager_sets_their_own_shootout_order() {
    let mut session = session_at_the_shootout(Some("alpha"));
    let mut reversed = session
        .match_state
        .shootout_order(Side::Home)
        .expect("a shootout is on")
        .player_ids;
    reversed.reverse();
    session
        .apply_command(MatchCommand::SetShootoutOrder {
            side: Side::Home,
            player_ids: reversed.clone(),
        })
        .expect("the manager's own side, the eligible players, before the first kick");
    assert_eq!(
        shown_order(&session, Side::Home).map(|order| order.player_ids),
        Some(reversed)
    );
}

/// Given the home manager at the start of a shootout,
/// When they try to set the away side's order, even to a valid one,
/// Then it is refused and the away side keeps its order.
#[test]
fn a_manager_cannot_set_the_opponents_shootout_order() {
    let mut session = session_at_the_shootout(Some("alpha"));
    let away_order = session.match_state.shootout_order(Side::Away);
    let mut reversed = away_order.clone().expect("a shootout is on").player_ids;
    reversed.reverse();
    assert_eq!(
        session.apply_command(MatchCommand::SetShootoutOrder {
            side: Side::Away,
            player_ids: reversed,
        }),
        Err(NOT_YOUR_SIDE.to_string())
    );
    assert_eq!(session.match_state.shootout_order(Side::Away), away_order);
}

/// Given the away manager at the start of a shootout,
/// When they set an order without naming a side, as an agent's tool does,
/// Then it is their own side's order that changes.
#[test]
fn an_order_set_without_a_side_is_the_managers_own() {
    let mut session = session_at_the_shootout(Some("bravo"));
    let mut reversed = session
        .match_state
        .shootout_order(Side::Away)
        .expect("a shootout is on")
        .player_ids;
    reversed.reverse();
    session
        .set_own_shootout_order(reversed.clone())
        .expect("the manager's own side, the eligible players, before the first kick");
    assert_eq!(
        shown_order(&session, Side::Away).map(|order| order.player_ids),
        Some(reversed)
    );
}

/// Given a match between two clubs the user does not manage,
/// When the user tries to set an order without naming a side,
/// Then it is refused: there is no side of theirs to set.
#[test]
fn a_user_who_manages_neither_side_cannot_set_an_order() {
    let mut session = session_at_the_shootout(None);
    let home_order = session.match_state.shootout_order(Side::Home);
    let player_ids = home_order.clone().expect("a shootout is on").player_ids;
    assert_eq!(
        session.set_own_shootout_order(player_ids),
        Err(NOT_YOUR_SIDE.to_string())
    );
    assert_eq!(session.match_state.shootout_order(Side::Home), home_order);
}
