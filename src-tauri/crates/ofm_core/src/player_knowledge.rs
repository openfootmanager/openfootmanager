//! What the manager's club knows about a player: the one rule every screen that shows a player's
//! rating, potential, condition or wage reads, so none of them decides it for itself.

use std::collections::{HashMap, HashSet};

use domain::player::Player;
use serde::Serialize;

use crate::game::Game;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PlayerKnowledge {
    /// The club's own player, including one it has loaned out or loaned in.
    Own,
    /// A player at a club in the manager's own division: the staff see him every season.
    Known,
    /// A player the club has a scout's report on, and whether that report still describes him.
    Scouted { scouted_on: String, current: bool },
    /// Nothing beyond what is public.
    Unknown,
}

pub fn player_knowledge(game: &Game, player: &Player) -> PlayerKnowledge {
    knowledge_from(game, &Viewer::of(game), player)
}

/// The manager's club and the league table it plays in, worked out once per request rather than
/// once per player.
struct Viewer<'a> {
    club: Option<&'a str>,
    division: Option<&'a str>,
}

impl<'a> Viewer<'a> {
    fn of(game: &'a Game) -> Self {
        let club = game.manager.team_id.as_deref();
        Self {
            club,
            division: club.and_then(|club| division_of(game, club)),
        }
    }
}

fn knowledge_from(game: &Game, viewer: &Viewer, player: &Player) -> PlayerKnowledge {
    if viewer
        .club
        .is_some_and(|club| is_at_or_owned_by(player, club))
    {
        return PlayerKnowledge::Own;
    }
    if viewer
        .division
        .is_some_and(|division| plays_in(game, player, division))
    {
        return PlayerKnowledge::Known;
    }
    scouted_or_unknown(game, player)
}

/// The club holds his contract (a loaned-out player's `team_id` is the borrower) or he plays for it
/// (a loaned-in player's `team_id` is the club).
fn is_at_or_owned_by(player: &Player, club_id: &str) -> bool {
    player.team_id.as_deref() == Some(club_id)
        || player
            .active_loan
            .as_ref()
            .is_some_and(|loan| loan.parent_team_id == club_id)
}

/// The league table a club contests. The legacy `game.league` never stands in for one.
fn division_of<'a>(game: &'a Game, club: &str) -> Option<&'a str> {
    crate::end_of_season::club_league_table(game, club).map(|league| league.id.as_str())
}

fn plays_in(game: &Game, player: &Player, division: &str) -> bool {
    player
        .team_id
        .as_deref()
        .is_some_and(|club| division_of(game, club) == Some(division))
}

fn scouted_or_unknown(game: &Game, player: &Player) -> PlayerKnowledge {
    match crate::slices::scouted_report::query_scouted_report(game, &player.id) {
        Some(view) => PlayerKnowledge::Scouted {
            current: !view.out_of_date,
            scouted_on: view.report.scouted_on,
        },
        None => PlayerKnowledge::Unknown,
    }
}

/// The knowledge of each requested player. An id no player has is left out of the answer.
pub fn players_knowledge(game: &Game, player_ids: &[String]) -> HashMap<String, PlayerKnowledge> {
    let wanted: HashSet<&str> = player_ids.iter().map(String::as_str).collect();
    let viewer = Viewer::of(game);
    game.players
        .iter()
        .filter(|player| wanted.contains(player.id.as_str()))
        .map(|player| (player.id.clone(), knowledge_from(game, &viewer, player)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use crate::game::ScoutedPlayer;
    use crate::test_support::uniform_attributes;
    use chrono::{TimeZone, Utc};
    use domain::league::League;
    use domain::manager::Manager;
    use domain::player::{ActiveLoan, Position};

    const MY_CLUB: &str = "mine";

    fn player(id: &str, team_id: Option<&str>) -> Player {
        let mut player = Player::new(
            id.to_string(),
            id.to_string(),
            id.to_string(),
            "2000-01-01".to_string(),
            "GB".to_string(),
            Position::Midfielder,
            uniform_attributes(60),
        );
        player.team_id = team_id.map(str::to_string);
        player
    }

    fn loan(parent: &str, borrower: &str) -> ActiveLoan {
        ActiveLoan {
            parent_team_id: parent.to_string(),
            loan_team_id: borrower.to_string(),
            start_date: "2026-07-01".to_string(),
            end_date: "2027-06-30".to_string(),
            wage_contribution_pct: 100,
            buy_option_fee: None,
            loan_start_minutes: 0,
            loan_start_appearances: 0,
            development_reported_minutes: 0,
            development_reported_appearances: 0,
        }
    }

    fn division(id: &str, clubs: &[&str]) -> League {
        let clubs: Vec<String> = clubs.iter().map(|club| club.to_string()).collect();
        League::new(id.to_string(), id.to_string(), 2026, &clubs)
    }

    /// My club and "rival" play in division one; "faraway" plays in division two.
    fn world(manager_club: Option<&str>, players: Vec<Player>) -> Game {
        let mut manager = Manager::new(
            "m1".to_string(),
            "A".to_string(),
            "B".to_string(),
            "1980-01-01".to_string(),
            "GB".to_string(),
        );
        manager.team_id = manager_club.map(str::to_string);
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 9, 15, 12, 0, 0).unwrap());
        let mut game = Game::new(clock, manager, vec![], players, vec![], vec![]);
        game.competitions = vec![
            division("division-one", &[MY_CLUB, "rival"]),
            division("division-two", &["faraway", "other"]),
        ];
        game
    }

    fn scouted(game: &mut Game, player_id: &str, scouted_on: &str) {
        game.scouted_players.push(ScoutedPlayer {
            player_id: player_id.to_string(),
            scouted_on: scouted_on.to_string(),
            attributes: uniform_attributes(60),
        });
    }

    fn knowledge_of(game: &Game, player_id: &str) -> PlayerKnowledge {
        let player = game.players.iter().find(|p| p.id == player_id).unwrap();
        player_knowledge(game, player)
    }

    /// Given a player at the manager's club
    /// When the club's knowledge of him is asked
    /// Then he is the club's own
    #[test]
    fn a_player_at_the_club_is_its_own() {
        let game = world(Some(MY_CLUB), vec![player("p", Some(MY_CLUB))]);

        assert_eq!(knowledge_of(&game, "p"), PlayerKnowledge::Own);
    }

    /// Given a player the club has loaned out to a club in another division
    /// When the club's knowledge of him is asked
    /// Then he is still its own: it holds his contract
    #[test]
    fn a_player_the_club_loaned_out_is_still_its_own() {
        let mut loanee = player("p", Some("faraway"));
        loanee.active_loan = Some(loan(MY_CLUB, "faraway"));
        let game = world(Some(MY_CLUB), vec![loanee]);

        assert_eq!(knowledge_of(&game, "p"), PlayerKnowledge::Own);
    }

    /// Given a player another division's club has loaned to the manager's club
    /// When the club's knowledge of him is asked
    /// Then he is its own: he trains with it every day
    #[test]
    fn a_player_loaned_in_is_the_clubs_own() {
        let mut loanee = player("p", Some(MY_CLUB));
        loanee.active_loan = Some(loan("faraway", MY_CLUB));
        let game = world(Some(MY_CLUB), vec![loanee]);

        assert_eq!(knowledge_of(&game, "p"), PlayerKnowledge::Own);
    }

    /// Given a player at a club in the manager's own division
    /// When the club's knowledge of him is asked
    /// Then he is known, without a scout
    #[test]
    fn a_rival_in_the_same_division_is_known() {
        let game = world(Some(MY_CLUB), vec![player("p", Some("rival"))]);

        assert_eq!(knowledge_of(&game, "p"), PlayerKnowledge::Known);
    }

    /// Given a player at a club in another division, never scouted
    /// When the club's knowledge of him is asked
    /// Then nothing is known
    #[test]
    fn a_player_in_another_division_is_unknown() {
        let game = world(Some(MY_CLUB), vec![player("p", Some("faraway"))]);

        assert_eq!(knowledge_of(&game, "p"), PlayerKnowledge::Unknown);
    }

    /// Given a player in another division whom a scout reported on
    /// When the club's knowledge of him is asked
    /// Then it is the scout's report, dated, current or not exactly as the report itself says
    #[test]
    fn a_scouted_player_is_known_through_the_report_and_its_own_verdict() {
        for scouted_on in ["2026-03-10", "2026-09-01"] {
            let mut game = world(Some(MY_CLUB), vec![player("p", Some("faraway"))]);
            game.season_context.season_start = Some("2026-08-01".to_string());
            scouted(&mut game, "p", scouted_on);
            let report_is_out_of_date =
                crate::slices::scouted_report::query_scouted_report(&game, "p")
                    .unwrap()
                    .out_of_date;

            assert_eq!(
                knowledge_of(&game, "p"),
                PlayerKnowledge::Scouted {
                    scouted_on: scouted_on.to_string(),
                    current: !report_is_out_of_date,
                }
            );
        }
    }

    /// Given a free agent, never scouted, and one a scout reported on
    /// When the club's knowledge of each is asked
    /// Then the first is unknown (no club puts him in anyone's division) and the second is scouted
    #[test]
    fn a_free_agent_is_unknown_unless_scouted() {
        let mut game = world(
            Some(MY_CLUB),
            vec![player("free", None), player("free-scouted", None)],
        );
        scouted(&mut game, "free-scouted", "2026-09-01");

        assert_eq!(knowledge_of(&game, "free"), PlayerKnowledge::Unknown);
        assert!(matches!(
            knowledge_of(&game, "free-scouted"),
            PlayerKnowledge::Scouted { .. }
        ));
    }

    /// Given an unemployed manager
    /// When his knowledge of a player in division one, and of a scouted one, is asked
    /// Then no division is his, so the first is unknown, and the scouted one is still scouted
    #[test]
    fn an_unemployed_manager_knows_only_what_was_scouted() {
        let mut game = world(
            None,
            vec![player("p", Some("rival")), player("s", Some("faraway"))],
        );
        scouted(&mut game, "s", "2026-09-01");

        assert_eq!(knowledge_of(&game, "p"), PlayerKnowledge::Unknown);
        assert!(matches!(
            knowledge_of(&game, "s"),
            PlayerKnowledge::Scouted { .. }
        ));
    }

    /// Given two clubs in no league table, sharing only the old single league a legacy save keeps
    /// When the knowledge of the other club's player is asked
    /// Then the old league does not stand in for a division: he is unknown
    #[test]
    fn a_club_outside_every_table_knows_no_division() {
        let mut game = world(Some("homeless"), vec![player("p", Some("neighbour"))]);
        game.league = Some(division("legacy", &["homeless", "neighbour"]));

        assert_eq!(knowledge_of(&game, "p"), PlayerKnowledge::Unknown);
    }

    /// Given a request for several players, one of them an id no player has
    /// When their knowledge is asked in one go
    /// Then each real player is answered and the unknown id is left out
    #[test]
    fn a_bulk_request_answers_each_player_and_skips_unknown_ids() {
        let game = world(
            Some(MY_CLUB),
            vec![
                player("own", Some(MY_CLUB)),
                player("rival-player", Some("rival")),
            ],
        );
        let ids = ["own", "rival-player", "nobody"].map(str::to_string);

        let answer = players_knowledge(&game, &ids);

        assert_eq!(answer.len(), 2);
        assert_eq!(answer["own"], PlayerKnowledge::Own);
        assert_eq!(answer["rival-player"], PlayerKnowledge::Known);
    }
}
