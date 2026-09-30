//! An AI club should not be frozen the way it was generated.
//!
//! Slices 6 and 7 gave clubs a tactical identity and a set of player roles at
//! *world generation*, deliberately without a backfill. Every save made before
//! them still has the neutral blueprint and an empty role map on every AI club,
//! and a squad since rebuilt by transfers still wears the jobs its first eleven
//! earned. Both are the same gap: nothing revisits the decision.

use chrono::{TimeZone, Utc};
use domain::league::{Fixture, FixtureCompetition, FixtureStatus, League, StandingEntry};
use domain::manager::Manager;
use domain::player::{Player, PlayerAttributes, Position};
use domain::team::{PlayStyle, TacticsPhaseSettings, Team};
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::turn;

// ---------------------------------------------------------------------------
// A small world: one club the player manages, one club per identity beside it.
// ---------------------------------------------------------------------------

const EVERY_STYLE: [PlayStyle; 6] = [
    PlayStyle::Balanced,
    PlayStyle::Attacking,
    PlayStyle::Defensive,
    PlayStyle::Possession,
    PlayStyle::HighPress,
    PlayStyle::Counter,
];

fn attrs() -> PlayerAttributes {
    PlayerAttributes {
        pace: 65,
        stamina: 70,
        strength: 60,
        agility: 60,
        passing: 65,
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

fn make_player(id: &str, team_id: &str, position: Position) -> Player {
    let mut player = Player::new(
        id.to_string(),
        id.to_string(),
        id.to_string(),
        "1998-01-01".to_string(),
        "England".to_string(),
        position,
        attrs(),
    );
    player.team_id = Some(team_id.to_string());
    player.condition = 100;
    player.morale = 70;
    player
}

fn make_squad(team_id: &str) -> Vec<Player> {
    let mut squad = vec![make_player(
        &format!("{team_id}_gk"),
        team_id,
        Position::Goalkeeper,
    )];
    for i in 0..5 {
        squad.push(make_player(
            &format!("{team_id}_def{i}"),
            team_id,
            Position::Defender,
        ));
    }
    for i in 0..5 {
        squad.push(make_player(
            &format!("{team_id}_mid{i}"),
            team_id,
            Position::Midfielder,
        ));
    }
    for i in 0..4 {
        squad.push(make_player(
            &format!("{team_id}_fwd{i}"),
            team_id,
            Position::Forward,
        ));
    }
    squad
}

/// A world in the shape a pre-slice-6 save loads in: every club neutral, every
/// role map empty.
fn make_world_frozen_as_it_was_generated() -> Game {
    let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 8, 3, 12, 0, 0).unwrap());
    let mut manager = Manager::new(
        "mgr".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("club0".to_string());

    let mut teams = Vec::new();
    let mut players = Vec::new();
    for (index, style) in EVERY_STYLE.iter().enumerate() {
        let id = format!("club{index}");
        let mut team = Team::new(
            id.clone(),
            format!("Club {index}"),
            format!("C{index}"),
            "England".to_string(),
            "London".to_string(),
            "Ground".to_string(),
            20_000,
        );
        team.play_style = style.clone();
        team.tactics_phase = TacticsPhaseSettings::default();
        team.player_roles.clear();
        players.extend(make_squad(&id));
        teams.push(team);
    }

    let participants: Vec<String> = teams.iter().map(|t| t.id.clone()).collect();
    let mut league = League::new(
        "league".to_string(),
        "Test League".to_string(),
        2026,
        &participants,
    );
    league.standings = participants
        .iter()
        .cloned()
        .map(StandingEntry::new)
        .collect();
    // Three rounds a week apart, so the world plays football while the month runs.
    for round in 0..3u32 {
        let date = (clock.current_date + chrono::Duration::days(7 * (round as i64 + 1)))
            .format("%Y-%m-%d")
            .to_string();
        for pair in 0..3usize {
            league.fixtures.push(Fixture {
                id: format!("fix{round}-{pair}"),
                competition_id: "league".to_string(),
                matchday: round + 1,
                date: date.clone(),
                home_team_id: format!("club{}", (pair + round as usize) % 6),
                away_team_id: format!("club{}", (pair + 3 + round as usize) % 6),
                competition: FixtureCompetition::League,
                status: FixtureStatus::Scheduled,
                result: None,
            });
        }
    }

    let mut game = Game::new(clock, manager, teams, players, vec![], vec![]);
    game.league = Some(league.clone());
    game.competitions = vec![league];
    game
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn a_club_carried_over_from_an_older_save_does_not_stay_neutral_forever() {
    let mut game = make_world_frozen_as_it_was_generated();

    for _ in 0..30 {
        turn::process_day(&mut game);
    }

    for team in &game.teams {
        // Balanced is excluded on purpose: its blueprint *is* the neutral
        // settings, so it can never prove anything either way.
        if team.id == "club0" || team.play_style == PlayStyle::Balanced {
            continue;
        }
        assert_ne!(
            team.tactics_phase,
            TacticsPhaseSettings::default(),
            "{} is a {:?} club and is still playing the neutral blueprint after a \
             month of simulated days",
            team.id,
            team.play_style
        );
    }
}

#[test]
fn the_club_the_player_manages_is_never_overwritten() {
    let mut game = make_world_frozen_as_it_was_generated();
    // This player clears the specialist margin. If the review visits the user's
    // club, it will assign a role and the assertion below will catch it.
    for id in ["club0_fwd0", "club1_fwd0"] {
        let poacher = game
            .players
            .iter_mut()
            .find(|player| player.id == id)
            .unwrap();
        poacher.attributes.shooting = 99;
        poacher.attributes.positioning = 99;
        poacher.attributes.composure = 99;
    }
    let chosen = TacticsPhaseSettings {
        defensive_line: domain::team::DefensiveLine::High,
        ..Default::default()
    };
    if let Some(team) = game.teams.iter_mut().find(|t| t.id == "club0") {
        team.tactics_phase = chosen.clone();
    }

    for _ in 0..30 {
        turn::process_day(&mut game);
    }

    let user_club = game.teams.iter().find(|t| t.id == "club0").unwrap();
    assert_eq!(
        user_club.tactics_phase, chosen,
        "the player's own tactical settings were rewritten underneath them"
    );
    assert!(
        user_club.player_roles.is_empty(),
        "the player's own role assignments were rewritten underneath them"
    );
    let ai_control = game.teams.iter().find(|team| team.id == "club1").unwrap();
    assert!(
        ai_control.player_roles.contains_key("club1_fwd0"),
        "the identical AI player did not earn a role, so this cannot prove exclusion"
    );
}
