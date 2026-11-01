//! How each of the manager's club's players stands at every position (natural, can play, out of
//! position), so the squad and tactics screens show the backend's grade instead of recomputing it.
//! Only the club's own players: a rival's familiarity is not the manager's to read.

use std::collections::BTreeMap;

use domain::player::Position;
use serde::Serialize;
use ts_rs::TS;

use crate::game::Game;
use crate::player_rating::{Familiarity, familiarity};

/// One player's familiarity at each of the 17 positions, keyed by the position's name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct PlayerFamiliarity {
    pub player_id: String,
    pub by_position: BTreeMap<String, Familiarity>,
}

/// The manager's club's players, in squad order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct FamiliarityProjection {
    pub players: Vec<PlayerFamiliarity>,
}

/// The familiarity of every player of the manager's club at every position.
/// Fails with a translation key when the manager has no club.
pub fn familiarity_projection(game: &Game) -> Result<FamiliarityProjection, String> {
    let club = game
        .manager
        .team_id
        .as_deref()
        .ok_or("be.error.noTeamAssigned")?;
    let players = game
        .players
        .iter()
        .filter(|player| player.team_id.as_deref() == Some(club))
        .map(|player| PlayerFamiliarity {
            player_id: player.id.clone(),
            by_position: Position::ALL
                .iter()
                .map(|position| (format!("{position:?}"), familiarity(player, position)))
                .collect(),
        })
        .collect();
    Ok(FamiliarityProjection { players })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::uniform_attributes;
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::player::Player;

    fn player(id: &str, team_id: &str, natural: Position, alternates: Vec<Position>) -> Player {
        let mut player = Player::new(
            id.to_string(),
            id.to_string(),
            id.to_string(),
            "2000-01-01".to_string(),
            "GB".to_string(),
            Position::Midfielder,
            uniform_attributes(60),
        );
        player.natural_position = natural;
        player.alternate_positions = alternates;
        player.team_id = Some(team_id.to_string());
        player
    }

    fn game(club: Option<&str>, players: Vec<Player>) -> Game {
        let mut manager = Manager::new(
            "m1".to_string(),
            "A".to_string(),
            "B".to_string(),
            "1980-01-01".to_string(),
            "GB".to_string(),
        );
        manager.team_id = club.map(str::to_string);
        let clock =
            crate::clock::GameClock::new(Utc.with_ymd_and_hms(2026, 9, 15, 12, 0, 0).unwrap());
        Game::new(clock, manager, vec![], players, vec![], vec![])
    }

    /// Given a club with a central midfielder who can also play defensive midfield,
    /// When the projection is read,
    /// Then each of the 17 positions carries exactly what `familiarity` says for it.
    #[test]
    fn every_position_is_what_familiarity_says() {
        let own = player(
            "own",
            "mine",
            Position::CentralMidfielder,
            vec![Position::DefensiveMidfielder],
        );
        let g = game(Some("mine"), vec![own.clone()]);

        let projection = familiarity_projection(&g).unwrap();

        assert_eq!(projection.players.len(), 1);
        let row = &projection.players[0];
        assert_eq!(row.by_position.len(), 17);
        for position in Position::ALL {
            assert_eq!(
                row.by_position[&format!("{position:?}")],
                familiarity(&own, &position)
            );
        }
        assert_eq!(row.by_position["CentralMidfielder"], Familiarity::Natural);
        assert_eq!(row.by_position["DefensiveMidfielder"], Familiarity::CanPlay);
        assert_eq!(row.by_position["Striker"], Familiarity::OutOfPosition);
    }

    /// Given a player of another club,
    /// When the projection is read,
    /// Then he is absent: it is the manager's club only.
    #[test]
    fn a_player_of_another_club_is_absent() {
        let g = game(
            Some("mine"),
            vec![
                player("own", "mine", Position::Striker, vec![]),
                player("rival", "other", Position::Striker, vec![]),
            ],
        );

        let ids: Vec<_> = familiarity_projection(&g)
            .unwrap()
            .players
            .into_iter()
            .map(|row| row.player_id)
            .collect();

        assert_eq!(ids, vec!["own".to_string()]);
    }

    /// Given a manager without a club,
    /// When the projection is read,
    /// Then the error is a translation key.
    #[test]
    fn no_club_is_a_translation_key() {
        let g = game(None, vec![]);

        assert_eq!(
            familiarity_projection(&g),
            Err("be.error.noTeamAssigned".to_string())
        );
    }

    /// Given the projection,
    /// When it is serialised,
    /// Then it has the documented shape the generated TypeScript type mirrors.
    #[test]
    fn it_serialises_in_the_documented_shape() {
        let g = game(
            Some("mine"),
            vec![player("own", "mine", Position::Striker, vec![])],
        );

        let json = serde_json::to_value(familiarity_projection(&g).unwrap()).unwrap();

        assert_eq!(json["players"][0]["player_id"], "own");
        assert_eq!(json["players"][0]["by_position"]["Striker"], "natural");
    }
}
