//! Roster invariants shared across the engine's join-a-team paths.
//!
//! The persistence layer enforces "no two players at one club share a jersey
//! number" via a unique index, but enforcement at save time only catches
//! violations *after* the in-memory state has already produced one. This
//! module is the single choke point that prevents such states from arising.

use crate::game::Game;
use domain::player::Player;
use domain::team::Team;

/// Decide which jersey number `player` should wear at `team`.
///
/// * If the player's current jersey is free at `team`, returns it (no churn).
/// * Otherwise picks the lowest available number in `1..=99` at the destination.
/// * Returns `None` only when all 99 slots at the destination are already
///   taken — unreachable for any realistic squad size, but the caller should
///   still handle it.
///
/// Pure: does not mutate anything. The caller writes the returned value into
/// `player.jersey_number` as part of whichever team-change side-effect it is
/// already performing.
///
/// The player about to be moved is excluded from the destination's occupied
/// set by id, so the function is safe to call even when the player happens
/// to already be at `team` (e.g. an idempotent reassignment).
pub fn resolve_jersey_for(game: &Game, player: &Player, team: &Team) -> Option<u8> {
    let occupied: std::collections::HashSet<u8> = game
        .players
        .iter()
        .filter(|other| other.id != player.id && other.team_id.as_deref() == Some(team.id.as_str()))
        .filter_map(|other| other.jersey_number)
        .collect();
    match player.jersey_number {
        Some(current) if !occupied.contains(&current) => Some(current),
        _ => (1u8..=99).find(|number| !occupied.contains(number)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::player::{Player, PlayerAttributes, Position};
    use domain::team::Team;

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

    fn make_player(id: &str, team_id: Option<&str>, jersey: Option<u8>) -> Player {
        let mut player = Player::new(
            id.to_string(),
            id.to_string(),
            id.to_string(),
            "2000-01-01".to_string(),
            "England".to_string(),
            Position::Forward,
            default_attrs(),
        );
        player.team_id = team_id.map(|t| t.to_string());
        player.jersey_number = jersey;
        player
    }

    fn make_team(id: &str) -> Team {
        Team::new(
            id.to_string(),
            id.to_string(),
            id.to_string(),
            "England".to_string(),
            "City".to_string(),
            "Ground".to_string(),
            20_000,
        )
    }

    fn make_game(players: Vec<Player>) -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap());
        let manager = Manager::new(
            "m-1".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        Game::new(
            clock,
            manager,
            vec![make_team("team-a")],
            players,
            vec![],
            vec![],
        )
    }

    #[test]
    fn keeps_preferred_jersey_when_free_at_destination() {
        let moving = make_player("p-moving", None, Some(6));
        let game = make_game(vec![make_player("p-existing", Some("team-a"), Some(10))]);
        assert_eq!(
            resolve_jersey_for(&game, &moving, &make_team("team-a")),
            Some(6)
        );
    }

    #[test]
    fn reassigns_when_preferred_jersey_is_taken() {
        let moving = make_player("p-moving", None, Some(6));
        let game = make_game(vec![make_player("p-existing", Some("team-a"), Some(6))]);
        assert_eq!(
            resolve_jersey_for(&game, &moving, &make_team("team-a")),
            Some(1)
        );
    }

    #[test]
    fn picks_lowest_free_when_player_has_no_preferred_jersey() {
        let moving = make_player("p-moving", None, None);
        let players = vec![
            make_player("p-1", Some("team-a"), Some(1)),
            make_player("p-2", Some("team-a"), Some(2)),
            make_player("p-3", Some("team-a"), Some(4)),
        ];
        let game = make_game(players);
        assert_eq!(
            resolve_jersey_for(&game, &moving, &make_team("team-a")),
            Some(3)
        );
    }

    #[test]
    fn excludes_moving_player_from_occupancy() {
        // Moving player already at the destination wearing #6 — resolver must
        // not see their own jersey as a conflict.
        let moving = make_player("p-moving", Some("team-a"), Some(6));
        let game = make_game(vec![moving.clone()]);
        assert_eq!(
            resolve_jersey_for(&game, &moving, &make_team("team-a")),
            Some(6)
        );
    }

    #[test]
    fn ignores_players_at_other_teams_and_free_agents() {
        let moving = make_player("p-moving", None, Some(6));
        let players = vec![
            make_player("p-other-team", Some("team-z"), Some(6)),
            make_player("p-free-agent", None, Some(6)),
        ];
        let game = make_game(players);
        assert_eq!(
            resolve_jersey_for(&game, &moving, &make_team("team-a")),
            Some(6)
        );
    }

    #[test]
    fn returns_none_when_destination_is_full() {
        let moving = make_player("p-moving", None, None);
        let mut players = Vec::new();
        for n in 1u8..=99 {
            players.push(make_player(&format!("p-{}", n), Some("team-a"), Some(n)));
        }
        let game = make_game(players);
        assert_eq!(
            resolve_jersey_for(&game, &moving, &make_team("team-a")),
            None
        );
    }
}
