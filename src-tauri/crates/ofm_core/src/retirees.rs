//! Looking up someone who has stopped playing, wherever the game keeps him.

use domain::player::Player;

use crate::game::Game;

/// The archive first, then the roster, so the answer is the same on either side of the move
/// that takes retirees out of the roster.
pub fn find_retiree(game: &Game, player_id: &str) -> Option<Player> {
    game.retired_players
        .iter()
        .find(|retired| retired.id == player_id)
        .map(|retired| retired.to_player())
        .or_else(|| {
            game.players
                .iter()
                .find(|player| player.retired && player.id == player_id)
                .cloned()
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::player::{PlayerAttributes, Position};
    use domain::retired_player::RetiredPlayer;

    use crate::clock::GameClock;

    fn person(id: &str, name: &str, retired: bool) -> Player {
        let value = 60;
        let attributes = PlayerAttributes {
            pace: value,
            stamina: value,
            strength: value,
            agility: value,
            passing: value,
            shooting: value,
            tackling: value,
            dribbling: value,
            defending: value,
            positioning: value,
            vision: value,
            decisions: value,
            composure: value,
            aggression: value,
            teamwork: value,
            leadership: value,
            handling: value,
            reflexes: value,
            aerial: value,
        };
        let mut player = Player::new(
            id.into(),
            name.into(),
            name.into(),
            "1988-01-01".into(),
            "England".into(),
            Position::Midfielder,
            attributes,
        );
        player.retired = retired;
        player
    }

    fn game_with(players: Vec<Player>) -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2040, 6, 1, 12, 0, 0).unwrap());
        let manager = Manager::new(
            "mgr".into(),
            "Alex".into(),
            "Manager".into(),
            "1980-01-01".into(),
            "England".into(),
        );
        Game::new(clock, manager, vec![], players, vec![], vec![])
    }

    /// Given a retiree in the archive, when looked up, then he is found as a retired player.
    #[test]
    fn an_archived_retiree_is_found() {
        let mut game = game_with(vec![]);
        game.retired_players.push(RetiredPlayer::from_player(
            &person("p-old", "Ada Old", true),
            "2039-06-30",
            2039,
        ));

        let found = find_retiree(&game, "p-old").expect("he is in the archive");

        assert_eq!(found.full_name, "Ada Old");
        assert!(found.retired);
    }

    /// Given a retiree still on the roster, when looked up, then he is found there.
    #[test]
    fn a_retiree_still_on_the_roster_is_found() {
        let game = game_with(vec![person("p-old", "Ada Old", true)]);

        assert_eq!(find_retiree(&game, "p-old").unwrap().full_name, "Ada Old");
    }

    /// Given a person in both places, when looked up, then the archive's record wins.
    #[test]
    fn the_archive_wins_over_the_roster() {
        let mut game = game_with(vec![person("p-old", "Roster Copy", true)]);
        game.retired_players.push(RetiredPlayer::from_player(
            &person("p-old", "Ada Old", true),
            "2039-06-30",
            2039,
        ));

        assert_eq!(find_retiree(&game, "p-old").unwrap().full_name, "Ada Old");
    }

    /// Given an active player or an unknown id, when looked up, then nobody is found:
    /// this is for retirees, and a live player is read from the roster.
    #[test]
    fn active_players_and_strangers_are_not_retirees() {
        let game = game_with(vec![person("p-active", "Active", false)]);

        assert!(find_retiree(&game, "p-active").is_none());
        assert!(find_retiree(&game, "p-nobody").is_none());
    }
}
