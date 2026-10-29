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

/// The strongest thing the club knows, in this order: its own player, then a player in its division,
/// then one a scout reported on, then nothing. A scouted player at a club in the manager's division is
/// `Known` (live), not `Scouted` (a snapshot). An unemployed manager has no club and no division, so
/// he knows only the players his scouts reported on.
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

/// The weekly wage the club may see: exact for its own player, rounded for one its scout reported
/// on, and nothing for anyone else. A known rival's wage is his club's business, like his condition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VisibleWage {
    Exact {
        weekly: u32,
    },
    /// Rounded to two significant figures: what a scout would hear, not the contract's figure.
    Approximate {
        weekly: u32,
    },
    Hidden,
}

pub fn visible_wage(game: &Game, player: &Player) -> VisibleWage {
    match player_knowledge(game, player) {
        PlayerKnowledge::Own => VisibleWage::Exact {
            weekly: player.wage(),
        },
        PlayerKnowledge::Scouted { .. } => VisibleWage::Approximate {
            weekly: two_significant_figures(player.wage()),
        },
        PlayerKnowledge::Known | PlayerKnowledge::Unknown => VisibleWage::Hidden,
    }
}

/// The player's condition as the club may see it. Its own player's always; an opponent's on the
/// day the two clubs meet, as the team sheet and the warm-up show it (maintainer, 29 Oct); nobody
/// else's, a rival in the club's own division included.
pub fn visible_condition(game: &Game, player: &Player) -> Option<u8> {
    let shown = player_knowledge(game, player) == PlayerKnowledge::Own
        || plays_the_club_today(game, player);
    shown.then_some(player.condition)
}

/// Whether the player's club meets the manager's club today, in any competition. A loanee counts
/// for the club he plays for, whose team sheet he is on.
fn plays_the_club_today(game: &Game, player: &Player) -> bool {
    let (Some(club), Some(opponent)) = (game.manager.team_id.as_deref(), player.team_id.as_deref())
    else {
        return false;
    };
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    game.competitions
        .iter()
        .flat_map(|competition| &competition.fixtures)
        .any(|fixture| {
            fixture.date == today
                && ((fixture.home_team_id == club && fixture.away_team_id == opponent)
                    || (fixture.away_team_id == club && fixture.home_team_id == opponent))
        })
}

fn two_significant_figures(value: u32) -> u32 {
    let mut step = 1;
    while value / step >= 100 {
        step *= 10;
    }
    // Round half up to the step: 1,250 → 1,300, 15,843 → 16,000.
    value.saturating_add(step / 2) / step * step
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

    /// A knockout cup: a competition that is not anyone's division.
    fn cup(id: &str, clubs: &[&str]) -> League {
        let mut cup = division(id, clubs);
        cup.kind = domain::league::CompetitionType::Cup;
        cup.rules.format = domain::league::CompetitionFormat::Knockout;
        cup
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

    fn paid(mut player: Player, weekly: u32) -> Player {
        player.stage_wage(weekly);
        player
    }

    /// Given the club's own player on 15,843 a week
    /// When the wage it may see is asked
    /// Then it is the exact figure
    #[test]
    fn the_clubs_own_player_shows_his_exact_wage() {
        let game = world(
            Some(MY_CLUB),
            vec![paid(player("p", Some(MY_CLUB)), 15_843)],
        );
        let player = &game.players[0];

        assert_eq!(
            visible_wage(&game, player),
            VisibleWage::Exact { weekly: 15_843 }
        );
    }

    /// Given a player in another division on 15,843 a week, whom a scout reported on
    /// When the wage the club may see is asked
    /// Then it is about 16,000: two significant figures, never the contract's figure
    #[test]
    fn a_scouted_player_shows_an_approximate_wage() {
        let mut game = world(
            Some(MY_CLUB),
            vec![paid(player("p", Some("faraway")), 15_843)],
        );
        scouted(&mut game, "p", "2026-09-01");
        let player = &game.players[0];

        assert_eq!(
            visible_wage(&game, player),
            VisibleWage::Approximate { weekly: 16_000 }
        );
    }

    /// Given a rival in the manager's own division, and a player elsewhere nobody scouted
    /// When the wage the club may see is asked
    /// Then neither shows one: a known rival's wage is his club's business
    #[test]
    fn a_known_or_unknown_players_wage_is_hidden() {
        let game = world(
            Some(MY_CLUB),
            vec![
                paid(player("known", Some("rival")), 15_843),
                paid(player("unknown", Some("faraway")), 15_843),
            ],
        );

        assert_eq!(visible_wage(&game, &game.players[0]), VisibleWage::Hidden);
        assert_eq!(visible_wage(&game, &game.players[1]), VisibleWage::Hidden);
    }

    /// Given wages of 999, 1,250, 12,345 and 7 a week
    /// When they are rounded for a scout's ear
    /// Then they keep two significant figures: 1,000, 1,300, 12,000 and 7
    #[test]
    fn an_approximate_wage_keeps_two_significant_figures() {
        let rounded = |weekly| {
            let mut game = world(Some(MY_CLUB), vec![paid(player("p", None), weekly)]);
            scouted(&mut game, "p", "2026-09-01");
            visible_wage(&game, &game.players[0])
        };

        assert_eq!(rounded(999), VisibleWage::Approximate { weekly: 1_000 });
        assert_eq!(rounded(1_250), VisibleWage::Approximate { weekly: 1_300 });
        assert_eq!(rounded(12_345), VisibleWage::Approximate { weekly: 12_000 });
        assert_eq!(rounded(7), VisibleWage::Approximate { weekly: 7 });
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

    /// A player on `condition`.
    fn tired(mut player: Player, condition: u8) -> Player {
        player.condition = condition;
        player
    }

    /// Schedules `home` against `away` in division one on `date`.
    fn fixture_on(game: &mut Game, date: &str, home: &str, away: &str) {
        fixture_in(game, "division-one", date, home, away);
    }

    /// Schedules `home` against `away` in `competition_id` on `date`.
    fn fixture_in(game: &mut Game, competition_id: &str, date: &str, home: &str, away: &str) {
        let competition = game
            .competitions
            .iter_mut()
            .find(|competition| competition.id == competition_id)
            .unwrap();
        competition.fixtures.push(domain::league::Fixture {
            id: format!("{home}-{away}-{date}"),
            competition_id: competition_id.to_string(),
            matchday: 1,
            date: date.to_string(),
            home_team_id: home.to_string(),
            away_team_id: away.to_string(),
            competition: domain::league::FixtureCompetition::League,
            status: domain::league::FixtureStatus::Scheduled,
            result: None,
            postponement: None,
        });
    }

    fn condition_of(game: &Game, player_id: &str) -> Option<u8> {
        let player = game.players.iter().find(|p| p.id == player_id).unwrap();
        visible_condition(game, player)
    }

    /// Given the club's own player on 64% condition, with no match today
    /// When his condition is asked
    /// Then it shows: the club's own player's condition is always the club's to see
    #[test]
    fn the_clubs_own_players_condition_always_shows() {
        let game = world(Some(MY_CLUB), vec![tired(player("p", Some(MY_CLUB)), 64)]);

        assert_eq!(condition_of(&game, "p"), Some(64));
    }

    /// Given "rival" playing the club today (15 Sep, away), and one of rival's players on 71%
    /// When his condition is asked
    /// Then it shows, as the team sheet and the warm-up would show it
    #[test]
    fn an_opponents_condition_shows_on_the_day_they_meet() {
        let mut game = world(Some(MY_CLUB), vec![tired(player("r", Some("rival")), 71)]);
        fixture_on(&mut game, "2026-09-15", "rival", MY_CLUB);

        assert_eq!(condition_of(&game, "r"), Some(71));
    }

    /// Given the club drawn at home today against "faraway", a club from another division, in a
    ///      cup (a competition other than the club's division), and one of faraway's players on 58%
    /// When his condition is asked
    /// Then it shows: any competition's matchday hands over the team sheet, not the league's alone
    #[test]
    fn an_opponents_condition_shows_on_a_cup_tie_too() {
        let mut game = world(Some(MY_CLUB), vec![tired(player("f", Some("faraway")), 58)]);
        game.competitions
            .push(cup("cup", &[MY_CLUB, "rival", "faraway", "other"]));
        fixture_in(&mut game, "cup", "2026-09-15", MY_CLUB, "faraway");

        assert_eq!(condition_of(&game, "f"), Some(58));
    }

    /// Given "rival" playing the club tomorrow, not today
    /// When one of rival's players' condition is asked today
    /// Then it does not show: a division rival is known, but his condition is his club's business
    #[test]
    fn an_opponents_condition_is_hidden_before_matchday() {
        let mut game = world(Some(MY_CLUB), vec![tired(player("r", Some("rival")), 71)]);
        fixture_on(&mut game, "2026-09-16", MY_CLUB, "rival");

        assert_eq!(condition_of(&game, "r"), None);
    }

    /// Given "rival" and "faraway" meeting each other today, the club not playing them
    /// When a "faraway" player's condition is asked
    /// Then it does not show: only the club's own opponent's team sheet is in its hands
    #[test]
    fn a_matchday_between_two_other_clubs_shows_nothing() {
        let mut game = world(Some(MY_CLUB), vec![tired(player("f", Some("faraway")), 71)]);
        fixture_on(&mut game, "2026-09-15", "rival", "faraway");

        assert_eq!(condition_of(&game, "f"), None);
    }

    /// Given a manager with no club, and "rival" playing "faraway" today
    /// When a rival player's condition is asked
    /// Then it does not show: a manager without a club has no opponent
    #[test]
    fn an_unemployed_manager_sees_no_opponents_condition() {
        let mut game = world(None, vec![tired(player("r", Some("rival")), 71)]);
        fixture_on(&mut game, "2026-09-15", "rival", "faraway");

        assert_eq!(condition_of(&game, "r"), None);
    }
}
