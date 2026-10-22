//! A manager's record: the one place a league season is added to it, for the user and for every
//! other club's manager alike.

use domain::league::{CompetitionFormat, League};
use domain::manager::{Manager, ManagerCareerEntry};
use domain::team::Team;

use crate::game::Game;

/// One club's finished league season, as a manager's record takes it.
pub(crate) struct LeagueSeason<'a> {
    pub team_id: &'a str,
    pub team_name: &'a str,
    pub won: u32,
    pub drawn: u32,
    pub lost: u32,
    pub position: u32,
    /// The day the stint is opened on, if the manager has none open at this club.
    pub on: &'a str,
}

/// Adds the season to the manager's totals and to his open stint at the club, opening one if he
/// has none there. Winning the league is a trophy.
pub(crate) fn credit_league_season(manager: &mut Manager, season: &LeagueSeason) {
    let matches = season.won + season.drawn + season.lost;
    let stats = &mut manager.career_stats;
    stats.matches_managed += matches;
    stats.wins += season.won;
    stats.draws += season.drawn;
    stats.losses += season.lost;
    if season.position == 1 {
        stats.trophies += 1;
    }
    if stats.best_finish.is_none_or(|best| best > season.position) {
        stats.best_finish = Some(season.position);
    }

    let open_stint = manager
        .career_history
        .iter_mut()
        .find(|entry| entry.team_id == season.team_id && entry.end_date.is_none());
    match open_stint {
        Some(entry) => {
            entry.matches += matches;
            entry.wins += season.won;
            entry.draws += season.drawn;
            entry.losses += season.lost;
            if entry
                .best_league_position
                .is_none_or(|best| best > season.position)
            {
                entry.best_league_position = Some(season.position);
            }
        }
        None => manager.career_history.push(ManagerCareerEntry {
            team_id: season.team_id.to_string(),
            team_name: season.team_name.to_string(),
            start_date: season.on.to_string(),
            end_date: None,
            matches,
            wins: season.won,
            draws: season.drawn,
            losses: season.lost,
            best_league_position: Some(season.position),
        }),
    }
}

/// Credits every club's manager, other than the user's, with his club's season in each league-table
/// edition archived from `game.edition_archive[first_new..]`. An edition is archived once, at the
/// first rollover after it finished, so no season is credited twice and none is credited half-played.
/// The user's manager is credited from his own division at his season's end
/// (`process_end_of_season`), so he is left out here.
pub(crate) fn credit_managers_of_newly_archived_leagues(game: &mut Game, first_new: usize) {
    // Borrowed field by field, so the archive is read while the managers are written.
    let Game {
        competitions,
        edition_archive,
        teams,
        managers,
        manager_id: users_manager_id,
        ..
    } = game;
    let new_league_editions = edition_archive
        .get(first_new..)
        .unwrap_or_default()
        .iter()
        .filter(|edition| is_league_table(competitions, &edition.competition_id));

    for edition in new_league_editions {
        for (position, standing) in (1..).zip(&edition.standings) {
            let Some(manager) = managers.iter_mut().find(|manager| {
                manager.id != *users_manager_id
                    && manager.team_id.as_deref() == Some(standing.team_id.as_str())
            }) else {
                continue;
            };
            credit_league_season(
                manager,
                &LeagueSeason {
                    team_id: &standing.team_id,
                    team_name: team_name(teams, &standing.team_id),
                    won: standing.won,
                    drawn: standing.drawn,
                    lost: standing.lost,
                    position,
                    on: &edition.completed_on,
                },
            );
        }
    }
}

fn is_league_table(competitions: &[League], competition_id: &str) -> bool {
    competitions.iter().any(|competition| {
        competition.id == competition_id
            && competition.rules.format == CompetitionFormat::LeagueTable
    })
}

fn team_name<'a>(teams: &'a [Team], team_id: &str) -> &'a str {
    teams
        .iter()
        .find(|team| team.id == team_id)
        .map(|team| team.name.as_str())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use chrono::{TimeZone, Utc};
    use domain::edition_archive::CompletedEdition;
    use domain::league::StandingEntry;

    fn standing(team_id: &str, won: u32, drawn: u32, lost: u32) -> StandingEntry {
        StandingEntry {
            team_id: team_id.to_string(),
            played: won + drawn + lost,
            won,
            drawn,
            lost,
            goals_for: 0,
            goals_against: 0,
            points: won * 3 + drawn,
        }
    }

    fn manager(id: &str, club: Option<&str>) -> Manager {
        let mut manager = Manager::new(
            id.to_string(),
            id.to_string(),
            "Surname".to_string(),
            "1975-01-01".to_string(),
            "GB".to_string(),
        );
        if let Some(club) = club {
            manager.hire(club.to_string());
        }
        manager
    }

    fn edition(id: &str, standings: Vec<StandingEntry>) -> CompletedEdition {
        CompletedEdition {
            competition_id: id.to_string(),
            season: 2026,
            completed_on: "2027-05-22".to_string(),
            champion_id: standings[0].team_id.clone(),
            participant_ids: standings.iter().map(|s| s.team_id.clone()).collect(),
            standings,
            groups: vec![],
            knockout_rounds: vec![],
            fixtures: vec![],
        }
    }

    /// The user manages "mine"; "rival" and "third" play with him in "division-one", and their
    /// managers are "rival-boss" and "third-boss". "cup" is a knockout competition.
    fn world() -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2027, 5, 30, 12, 0, 0).unwrap());
        let clubs = ["mine", "rival", "third"];
        let teams: Vec<Team> = clubs
            .iter()
            .map(|id| {
                let mut team = Team::new(
                    id.to_string(),
                    format!("{id} FC"),
                    id.to_uppercase(),
                    "GB".to_string(),
                    "City".to_string(),
                    "Ground".to_string(),
                    20_000,
                );
                team.manager_id = None;
                team
            })
            .collect();
        let mut game = Game::new(
            clock,
            manager("me", Some("mine")),
            teams,
            vec![],
            vec![],
            vec![],
        );
        game.managers = vec![
            manager("me", Some("mine")),
            manager("rival-boss", Some("rival")),
            manager("third-boss", Some("third")),
        ];
        let ids: Vec<String> = clubs.iter().map(|c| c.to_string()).collect();
        game.competitions = vec![League::new("division-one".into(), "One".into(), 2026, &ids)];
        let mut cup = League::new("cup".into(), "Cup".into(), 2026, &ids);
        cup.rules.format = CompetitionFormat::Knockout;
        game.competitions.push(cup);
        game
    }

    fn boss<'a>(game: &'a Game, id: &str) -> &'a Manager {
        game.managers.iter().find(|m| m.id == id).unwrap()
    }

    /// Given a league season that finished with "rival" first (20 W, 5 D, 3 L) and "third" last
    /// When it is archived at the rollover
    /// Then each club's manager has that season in his totals and his stint, and the champion's
    ///      manager has a trophy
    #[test]
    fn every_clubs_manager_takes_his_clubs_league_season() {
        let mut game = world();
        game.edition_archive.push(edition(
            "division-one",
            vec![
                standing("rival", 20, 5, 3),
                standing("mine", 15, 6, 7),
                standing("third", 2, 4, 22),
            ],
        ));

        credit_managers_of_newly_archived_leagues(&mut game, 0);

        let rival_boss = boss(&game, "rival-boss");
        assert_eq!(rival_boss.career_stats.matches_managed, 28);
        assert_eq!(rival_boss.career_stats.wins, 20);
        assert_eq!(rival_boss.career_stats.trophies, 1);
        assert_eq!(rival_boss.career_stats.best_finish, Some(1));
        let stint = rival_boss.career_history.last().unwrap();
        assert_eq!(
            (stint.team_id.as_str(), stint.wins, stint.losses),
            ("rival", 20, 3)
        );
        let third_boss = boss(&game, "third-boss");
        assert_eq!(third_boss.career_stats.losses, 22);
        assert_eq!(third_boss.career_stats.best_finish, Some(3));
        assert_eq!(third_boss.career_stats.trophies, 0);
    }

    /// Given the same archived season
    /// When the managers are credited
    /// Then the user's manager is not, since his own season end already credited him
    #[test]
    fn the_users_manager_is_not_credited_twice() {
        let mut game = world();
        game.edition_archive.push(edition(
            "division-one",
            vec![standing("mine", 20, 5, 3), standing("rival", 15, 6, 7)],
        ));

        credit_managers_of_newly_archived_leagues(&mut game, 0);

        assert_eq!(game.manager.career_stats.matches_managed, 0);
        assert_eq!(boss(&game, "me").career_stats.matches_managed, 0);
    }

    /// Given an edition archived at an earlier rollover, and a new one archived now
    /// When the managers are credited from the new one on
    /// Then only the new season is counted: the older one was credited when it was archived
    #[test]
    fn only_newly_archived_seasons_are_credited() {
        let mut game = world();
        game.edition_archive
            .push(edition("division-one", vec![standing("rival", 10, 0, 0)]));
        game.edition_archive
            .push(edition("division-one", vec![standing("rival", 3, 0, 1)]));

        credit_managers_of_newly_archived_leagues(&mut game, 1);

        assert_eq!(boss(&game, "rival-boss").career_stats.matches_managed, 4);
    }

    /// Given a cup edition archived at the rollover
    /// When the managers are credited
    /// Then nobody's league record moves: a cup is not a league season
    #[test]
    fn a_cup_edition_credits_no_league_record() {
        let mut game = world();
        game.edition_archive
            .push(edition("cup", vec![standing("rival", 4, 0, 0)]));

        credit_managers_of_newly_archived_leagues(&mut game, 0);

        assert_eq!(boss(&game, "rival-boss").career_stats.matches_managed, 0);
        assert_eq!(boss(&game, "rival-boss").career_stats.trophies, 0);
    }

    /// Given a club in the archived table with no manager (a vacancy)
    /// When the managers are credited
    /// Then the club is skipped and every other club's manager is still credited
    #[test]
    fn a_club_without_a_manager_is_skipped() {
        let mut game = world();
        game.managers.retain(|m| m.id != "third-boss");
        game.edition_archive.push(edition(
            "division-one",
            vec![standing("rival", 20, 5, 3), standing("third", 2, 4, 22)],
        ));

        credit_managers_of_newly_archived_leagues(&mut game, 0);

        assert_eq!(boss(&game, "rival-boss").career_stats.matches_managed, 28);
    }

    /// Given a manager back at a club he left before, that first spell closed in his history
    /// When a league season at the club is added to his record
    /// Then it opens a new stint, dated from the season, and the closed spell keeps its numbers
    #[test]
    fn a_season_back_at_a_former_club_opens_a_new_stint() {
        let mut returning = manager("returning", None);
        returning.career_history.push(ManagerCareerEntry {
            team_id: "rival".to_string(),
            team_name: "rival FC".to_string(),
            start_date: "2020-07-01".to_string(),
            end_date: Some("2023-05-30".to_string()),
            matches: 114,
            wins: 50,
            draws: 30,
            losses: 34,
            best_league_position: Some(2),
        });

        credit_league_season(
            &mut returning,
            &LeagueSeason {
                team_id: "rival",
                team_name: "rival FC",
                won: 20,
                drawn: 5,
                lost: 3,
                position: 1,
                on: "2027-05-22",
            },
        );

        let spells: Vec<(&str, Option<&str>, u32)> = returning
            .career_history
            .iter()
            .map(|spell| {
                (
                    spell.start_date.as_str(),
                    spell.end_date.as_deref(),
                    spell.matches,
                )
            })
            .collect();
        assert_eq!(
            spells,
            vec![
                ("2020-07-01", Some("2023-05-30"), 114),
                ("2027-05-22", None, 28)
            ]
        );
    }
}
