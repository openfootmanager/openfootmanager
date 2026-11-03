//! Who has stopped playing: moving them into the archive, and looking them up there.

use domain::player::{Player, PlayerMovementKind};
use domain::retired_player::RetiredPlayer;

use crate::game::Game;

/// Moves everyone the roster flags as retired into the archive, dated `retired_on` in
/// `retired_season`. Used when the season's retirements have just been decided.
pub fn archive_retired_players(game: &mut Game, retired_on: &str, retired_season: u32) {
    archive_retirees_dated_by(game, |_| (retired_on.to_string(), retired_season));
}

/// Moves retirees found on the roster of a loaded save (one that predates the archive). Each
/// keeps the date his own movement history gives, and the season of his last career entry.
/// Returns how many left the roster.
pub fn archive_legacy_retirees(game: &mut Game) -> usize {
    let fallback_date = game.clock.current_date.format("%Y-%m-%d").to_string();
    archive_retirees_dated_by(game, |player| {
        let retired_on = player
            .movement_history
            .into_iter()
            .rev()
            .find(|movement| movement.kind == PlayerMovementKind::Retired)
            .map_or_else(|| fallback_date.clone(), |movement| movement.date.clone());
        let season = player
            .career
            .iter()
            .map(|entry| entry.season)
            .max()
            .unwrap_or(0);
        (retired_on, season)
    })
}

fn archive_retirees_dated_by(
    game: &mut Game,
    dated_by: impl Fn(&Player) -> (String, u32),
) -> usize {
    let (retired, active): (Vec<Player>, Vec<Player>) = std::mem::take(&mut game.players)
        .into_iter()
        .partition(|player| player.retired);
    game.players = active;
    let moved = retired.len();

    for player in retired {
        release_loan_references(game, &player);
        let already_archived = game
            .retired_players
            .iter()
            .any(|record| record.id == player.id);
        if !already_archived {
            let (retired_on, retired_season) = dated_by(&player);
            game.retired_players.push(RetiredPlayer::from_player(
                &player,
                &retired_on,
                retired_season,
            ));
        }
    }
    moved
}

/// A player who retires while out on loan is still named by both clubs.
fn release_loan_references(game: &mut Game, player: &Player) {
    let Some(loan) = &player.active_loan else {
        return;
    };
    for team_id in [&loan.parent_team_id, &loan.loan_team_id] {
        if let Some(team) = game.teams.iter_mut().find(|team| &team.id == team_id) {
            team.remove_player_references(&player.id);
        }
    }
}

pub fn find_retiree(game: &Game, player_id: &str) -> Option<Player> {
    game.retired_players
        .iter()
        .find(|retired| retired.id == player_id)
        .map(|retired| retired.to_player())
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

        archive_legacy_retirees(&mut game);

        assert_eq!(game.retired_players[0].retired_on, "2031-05-20");
    }

    /// Given a retiree from a save that predates the archive, with career entries in three seasons,
    /// When archived at load,
    /// Then he is filed under the latest of them.
    #[test]
    fn a_legacy_retiree_is_filed_under_his_last_career_season() {
        use domain::player::CareerEntry;
        let mut old = person("p-old", "Ada Old", true);
        for season in [2030, 2034, 2032] {
            old.career.push(CareerEntry {
                season,
                team_id: "t".into(),
                team_name: "T".into(),
                appearances: 10,
                goals: 1,
                assists: 1,
            });
        }
        let mut game = game_with_teams(vec![old], vec![]);

        archive_legacy_retirees(&mut game);

        assert_eq!(game.retired_players[0].retired_season, 2034);
    }
}
