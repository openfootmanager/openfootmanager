//! Looking up someone who has stopped playing, wherever the game keeps him.

use domain::player::Player;
use domain::retired_player::RetiredPlayer;

use crate::game::Game;

/// Moves everyone the roster still flags as retired into the archive.
pub fn archive_retired_players(_game: &mut Game, _retired_on: &str, _retired_season: u32) {}

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

    fn team(id: &str) -> domain::team::Team {
        domain::team::Team::new(
            id.into(),
            id.into(),
            id.into(),
            "ENG".into(),
            "Town".into(),
            "Ground".into(),
            20_000,
        )
    }

    fn game_with_teams(players: Vec<Player>, teams: Vec<domain::team::Team>) -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2040, 6, 1, 12, 0, 0).unwrap());
        let manager = Manager::new(
            "mgr".into(),
            "Alex".into(),
            "Manager".into(),
            "1980-01-01".into(),
            "England".into(),
        );
        Game::new(clock, manager, teams, players, vec![], vec![])
    }

    /// Given a retired player on the roster and an active one, when retirees are archived,
    /// then the retiree is in the archive and off the roster, and the active player stays.
    #[test]
    fn a_retired_player_moves_from_the_roster_to_the_archive() {
        let mut game = game_with_teams(
            vec![
                person("p-old", "Ada Old", true),
                person("p-active", "Ann Active", false),
            ],
            vec![],
        );

        archive_retired_players(&mut game, "2040-06-30", 2040);

        assert_eq!(game.players.len(), 1);
        assert_eq!(game.players[0].id, "p-active");
        assert_eq!(game.retired_players.len(), 1);
        assert_eq!(game.retired_players[0].id, "p-old");
        assert_eq!(game.retired_players[0].retired_on, "2040-06-30");
        assert_eq!(game.retired_players[0].retired_season, 2040);
    }

    /// Given the move has already happened, when it runs again, then nothing is duplicated.
    #[test]
    fn archiving_twice_changes_nothing() {
        let mut game = game_with_teams(vec![person("p-old", "Ada Old", true)], vec![]);
        archive_retired_players(&mut game, "2040-06-30", 2040);
        let after_first = game.retired_players.clone();

        archive_retired_players(&mut game, "2041-06-30", 2041);

        assert_eq!(game.retired_players, after_first);
        assert!(game.players.is_empty());
    }

    /// Given a person who is both archived and still flagged retired on the roster, when
    /// retirees are archived, then he leaves the roster and the first archive record stays.
    #[test]
    fn an_already_archived_person_leaves_the_roster_without_a_second_record() {
        let mut game = game_with_teams(vec![person("p-old", "Roster Copy", true)], vec![]);
        game.retired_players.push(RetiredPlayer::from_player(
            &person("p-old", "Ada Old", true),
            "2039-06-30",
            2039,
        ));

        archive_retired_players(&mut game, "2040-06-30", 2040);

        assert!(game.players.is_empty());
        assert_eq!(game.retired_players.len(), 1);
        assert_eq!(game.retired_players[0].full_name, "Ada Old");
    }

    /// Given a player who retires while out on loan, when retirees are archived, then neither
    /// the parent club nor the loan club keeps any reference to him.
    #[test]
    fn a_player_who_retires_on_loan_leaves_no_trace_at_either_club() {
        let mut parent = team("parent");
        let mut borrower = team("borrower");
        parent.starting_xi_ids = vec!["p-loan".into(), "p-other".into()];
        borrower.starting_xi_ids = vec!["p-loan".into()];
        let mut lent = person("p-loan", "Lent Out", true);
        lent.active_loan = Some(domain::player::ActiveLoan {
            parent_team_id: "parent".into(),
            loan_team_id: "borrower".into(),
            start_date: "2040-01-01".into(),
            end_date: "2040-12-31".into(),
            wage_contribution_pct: 50,
            buy_option_fee: None,
            loan_start_minutes: 0,
            loan_start_appearances: 0,
            development_reported_minutes: 0,
            development_reported_appearances: 0,
        });
        let mut game = game_with_teams(vec![lent], vec![parent, borrower]);

        archive_retired_players(&mut game, "2040-06-30", 2040);

        let xi = |id: &str| {
            game.teams
                .iter()
                .find(|team| team.id == id)
                .unwrap()
                .starting_xi_ids
                .clone()
        };
        assert_eq!(xi("parent"), vec!["p-other".to_string()]);
        assert!(xi("borrower").is_empty());
    }

    /// Given a retiree from before retirement dates were recorded, when archived at load,
    /// then his retirement date is the one in his own movement history.
    #[test]
    fn a_legacy_retiree_keeps_the_date_his_own_history_gives() {
        use domain::player::{PlayerMovementEntry, PlayerMovementKind};
        let mut old = person("p-old", "Ada Old", true);
        old.record_movement(PlayerMovementEntry::new(
            "2031-05-20",
            PlayerMovementKind::Retired,
        ))
        .unwrap();
        let mut game = game_with_teams(vec![old], vec![]);

        archive_retired_players(&mut game, "2040-06-30", 2040);

        assert_eq!(game.retired_players[0].retired_on, "2031-05-20");
    }
}
