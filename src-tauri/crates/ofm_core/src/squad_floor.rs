//! A club never runs out of players.
//!
//! The floor is how many players of each position group a club must have
//! registered — injured or not — to put a side out at all, with a keeper to
//! spare. It is one rule with one home: the generator builds clubs to it, and
//! every path that can take players away from a club answers to it.

use crate::game::Game;
use domain::player::Position;

/// Players a club must keep registered in each position group, in
/// `[GK, DEF, MID, FWD]` order. Twelve in all: an eleven and a second keeper,
/// because a club whose only keeper is injured still has to put one in goal.
pub const MIN_PLAYERS_PER_GROUP: [(Position, usize); 4] = [
    (Position::Goalkeeper, 2),
    (Position::Defender, 4),
    (Position::Midfielder, 4),
    (Position::Forward, 2),
];

/// The floor for one position group, or 0 for a group with none.
pub fn group_floor(group: &Position) -> usize {
    MIN_PLAYERS_PER_GROUP
        .iter()
        .find(|(position, _)| position == group)
        .map(|(_, floor)| *floor)
        .unwrap_or(0)
}

/// Every position group this club is short in, with how many players it is
/// short by. Empty when the club is at or above the floor everywhere.
///
/// Counts every player registered to the club — injured players included,
/// since they are still the club's to field in a crisis, and players out on
/// loan excluded, since they are registered to their borrower.
pub fn squad_shortfall(game: &Game, team_id: &str) -> Vec<(Position, usize)> {
    MIN_PLAYERS_PER_GROUP
        .iter()
        .filter_map(|(group, floor)| {
            let have = game
                .players
                .iter()
                .filter(|player| player.team_id.as_deref() == Some(team_id))
                .filter(|player| player.position.to_group_position() == *group)
                .count();
            (have < *floor).then(|| (group.clone(), floor - have))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::player::{Player, PlayerAttributes};
    use domain::team::Team;

    fn attrs() -> PlayerAttributes {
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
            handling: 60,
            reflexes: 60,
            aerial: 60,
        }
    }

    pub(super) fn player(id: &str, team_id: Option<&str>, position: Position) -> Player {
        let mut player = Player::new(
            id.to_string(),
            id.to_string(),
            id.to_string(),
            "1998-01-01".to_string(),
            "England".to_string(),
            position,
            attrs(),
        );
        player.team_id = team_id.map(str::to_string);
        player
    }

    /// A club with exactly `per_group` players in each group, `[GK, DEF, MID, FWD]`.
    pub(super) fn club_with(per_group: [usize; 4]) -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "mgr".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("user".to_string());
        let team = Team::new(
            "club".to_string(),
            "Club".to_string(),
            "CLB".to_string(),
            "England".to_string(),
            "London".to_string(),
            "Ground".to_string(),
            20_000,
        );
        let mut players = Vec::new();
        for ((group, _), count) in MIN_PLAYERS_PER_GROUP.iter().zip(per_group) {
            for i in 0..count {
                players.push(player(
                    &format!("{group:?}{i}"),
                    Some("club"),
                    group.clone(),
                ));
            }
        }
        Game::new(clock, manager, vec![team], players, vec![], vec![])
    }

    #[test]
    fn a_club_at_the_floor_is_short_of_nothing() {
        let game = club_with([2, 4, 4, 2]);
        assert!(squad_shortfall(&game, "club").is_empty());
    }

    #[test]
    fn a_club_is_short_by_group_and_by_how_many() {
        let game = club_with([0, 3, 4, 5]);
        assert_eq!(
            squad_shortfall(&game, "club"),
            vec![(Position::Goalkeeper, 2), (Position::Defender, 1)]
        );
    }

    #[test]
    fn a_club_with_nobody_is_short_everywhere() {
        let game = club_with([0, 0, 0, 0]);
        assert_eq!(
            squad_shortfall(&game, "club"),
            MIN_PLAYERS_PER_GROUP.to_vec()
        );
    }

    /// A player out on loan is registered to his borrower, and an injured one
    /// is still the club's to field. Granular positions count in their group.
    #[test]
    fn injured_players_count_and_loaned_out_players_do_not() {
        let mut game = club_with([2, 4, 4, 1]);
        let mut injured = player("injured_striker", Some("club"), Position::Striker);
        injured.injury = Some(domain::player::Injury {
            name: "common.injuries.calfStrain".to_string(),
            days_remaining: 30,
        });
        game.players.push(injured);
        game.players
            .push(player("loaned_out", Some("borrower"), Position::Goalkeeper));
        assert!(squad_shortfall(&game, "club").is_empty());
    }
}
