//! The managers directory: every active manager with his club, reputation and record, and the
//! clubs that have none.
//!
//! A manager's record is public (round 1 of the manager profile plan: the board's and the fans'
//! opinion of him are his club's own business, his record is not), so the directory needs no
//! knowledge gate. Managers who retired into the archive are not listed.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::game::Game;
use crate::manager_reputation::ManagerReputation;
use domain::manager::Manager;

/// Which managers the directory lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ManagersStatus {
    All,
    /// In charge of a club, a stand-in included.
    Employed,
    /// Between jobs.
    Unemployed,
}

/// The club a manager is in charge of.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct ManagerClub {
    pub id: String,
    pub name: String,
}

/// A manager's career record as stored: it moves at each season's end.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct ManagerRecord {
    pub matches: u32,
    pub trophies: u32,
}

/// One manager as the directory lists him.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct ManagerSummary {
    pub id: String,
    pub display_name: String,
    pub nationality: String,
    /// None while he is between jobs.
    pub club: Option<ManagerClub>,
    /// Holding the job until a permanent appointment.
    pub interim: bool,
    pub reputation: ManagerReputation,
    pub record: ManagerRecord,
}

/// A club with no manager.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct Vacancy {
    pub club_id: String,
    pub club_name: String,
    pub club_reputation: u32,
}

/// The directory: managers by club name, then by name, those between jobs last; and the vacant
/// clubs by name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct ManagersPage {
    pub managers: Vec<ManagerSummary>,
    pub vacancies: Vec<Vacancy>,
}

/// Every active manager, each once: the managers list, with the user's live record in place of the
/// copy it holds of him (or added, when it holds none).
fn active_managers(game: &Game) -> Vec<&Manager> {
    let mut managers: Vec<&Manager> = game
        .managers
        .iter()
        .filter(|manager| manager.id != game.manager.id)
        .collect();
    managers.push(&game.manager);
    managers
}

fn summary(game: &Game, manager: &Manager) -> ManagerSummary {
    let club = manager.team_id.as_deref().and_then(|club_id| {
        game.teams
            .iter()
            .find(|team| team.id == club_id)
            .map(|team| ManagerClub {
                id: team.id.clone(),
                name: team.name.clone(),
            })
    });
    ManagerSummary {
        id: manager.id.clone(),
        display_name: manager.display_name.clone(),
        nationality: manager.nationality.clone(),
        club,
        interim: manager.interim,
        reputation: ManagerReputation::of(manager.reputation),
        record: ManagerRecord {
            matches: manager.career_stats.matches_managed,
            trophies: manager.career_stats.trophies,
        },
    }
}

fn listed(status: ManagersStatus, summary: &ManagerSummary) -> bool {
    match status {
        ManagersStatus::All => true,
        ManagersStatus::Employed => summary.club.is_some(),
        ManagersStatus::Unemployed => summary.club.is_none(),
    }
}

fn vacancies(game: &Game) -> Vec<Vacancy> {
    let users_club = game.manager.team_id.as_deref();
    let mut vacancies: Vec<Vacancy> = game
        .teams
        .iter()
        .filter(|team| team.manager_id.is_none() && Some(team.id.as_str()) != users_club)
        .map(|team| Vacancy {
            club_id: team.id.clone(),
            club_name: team.name.clone(),
            club_reputation: team.reputation,
        })
        .collect();
    vacancies.sort_by(|left, right| {
        left.club_name
            .cmp(&right.club_name)
            .then_with(|| left.club_id.cmp(&right.club_id))
    });
    vacancies
}

/// The managers directory, filtered by `status`. Unpaged: a world holds a few thousand managers at
/// most, and the screen sorts and searches them itself.
pub fn query_managers_page(game: &Game, status: ManagersStatus) -> ManagersPage {
    let mut managers: Vec<ManagerSummary> = active_managers(game)
        .into_iter()
        .map(|manager| summary(game, manager))
        .filter(|summary| listed(status, summary))
        .collect();
    managers.sort_by(|left, right| {
        let club_name =
            |summary: &ManagerSummary| summary.club.as_ref().map(|club| club.name.clone());
        left.club
            .is_none()
            .cmp(&right.club.is_none())
            .then_with(|| club_name(left).cmp(&club_name(right)))
            .then_with(|| left.display_name.cmp(&right.display_name))
            .then_with(|| left.id.cmp(&right.id))
    });
    ManagersPage {
        managers,
        vacancies: vacancies(game),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use crate::manager_reputation::ReputationBand;
    use crate::rated_tone::RatedTone;
    use chrono::{TimeZone, Utc};
    use domain::team::Team;

    fn club(id: &str, name: &str, reputation: u32, manager_id: Option<&str>) -> Team {
        let mut team = Team::new(
            id.into(),
            name.into(),
            id.into(),
            "ENG".into(),
            "Town".into(),
            "Ground".into(),
            20_000,
        );
        team.reputation = reputation;
        team.manager_id = manager_id.map(Into::into);
        team
    }

    fn manager(id: &str, name: &str, club_id: Option<&str>, reputation: u32) -> Manager {
        let mut manager = Manager::new(
            id.into(),
            name.into(),
            "Surname".into(),
            "1975-01-01".into(),
            "ENG".into(),
        );
        if let Some(club_id) = club_id {
            manager.hire(club_id.into());
        }
        manager.reputation = reputation;
        manager
    }

    /// The user (at Zeta), an AI manager at Alpha, a stand-in covering Beta, two managers between
    /// jobs, one manager in the retired archive, and Gamma with nobody in charge.
    fn world() -> Game {
        let user = manager("user", "Uma", Some("zeta"), 500);
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 9, 1, 12, 0, 0).unwrap());
        let mut game = Game::new(
            clock,
            user,
            vec![
                club("zeta", "Zeta", 400, Some("user")),
                club("alpha", "Alpha", 800, Some("ai")),
                club("beta", "Beta", 600, Some("cover")),
                club("gamma", "Gamma", 300, None),
            ],
            vec![],
            vec![],
            vec![],
        );
        let mut ai = manager("ai", "Ada", Some("alpha"), 680);
        ai.career_stats.matches_managed = 120;
        ai.career_stats.trophies = 2;
        let mut cover = manager("cover", "Cy", Some("beta"), 300);
        cover.interim = true;
        game.managers.extend([
            ai,
            cover,
            manager("free-b", "Bo", None, 240),
            manager("free-a", "Al", None, 455),
        ]);
        game.retired_managers
            .push(manager("gone", "Gus", None, 650));
        game
    }

    fn ids(page: &ManagersPage) -> Vec<&str> {
        page.managers
            .iter()
            .map(|summary| summary.id.as_str())
            .collect()
    }

    /// Given the user, an AI manager, a stand-in, two managers between jobs and a retired one
    /// When the whole directory is asked for
    /// Then every active manager is listed once, by club name and then by name, those between
    ///      jobs last; the retired one is not
    #[test]
    fn the_directory_lists_every_active_manager_by_club_then_name() {
        let page = query_managers_page(&world(), ManagersStatus::All);

        assert_eq!(ids(&page), vec!["ai", "cover", "user", "free-a", "free-b"]);
    }

    /// Given an AI manager at Alpha with a record
    /// When the directory lists him
    /// Then he carries his club, his record and his reputation's band and tone
    #[test]
    fn a_manager_carries_his_club_record_and_reputation() {
        let page = query_managers_page(&world(), ManagersStatus::All);

        let ai = &page.managers[0];
        assert_eq!(
            (ai.display_name.as_str(), ai.club.clone(), ai.interim),
            (
                "Ada Surname",
                Some(ManagerClub {
                    id: "alpha".into(),
                    name: "Alpha".into()
                }),
                false
            )
        );
        assert_eq!(
            ai.record,
            ManagerRecord {
                matches: 120,
                trophies: 2
            }
        );
        assert_eq!(
            ai.reputation,
            ManagerReputation {
                value: 680,
                band: ReputationBand::WorldClass,
                tone: RatedTone::Top
            }
        );
    }

    /// Given a stand-in covering Beta
    /// When the directory lists him
    /// Then he is marked interim
    #[test]
    fn a_stand_in_is_marked_interim() {
        let page = query_managers_page(&world(), ManagersStatus::All);

        let interim: Vec<&str> = page
            .managers
            .iter()
            .filter(|summary| summary.interim)
            .map(|summary| summary.id.as_str())
            .collect();
        assert_eq!(interim, vec!["cover"]);
    }

    /// Given the user's live record has moved on from the copy the managers list holds
    /// When the directory lists him
    /// Then it reads the live record
    #[test]
    fn the_user_is_listed_from_his_live_record() {
        let mut game = world();
        game.manager.reputation = 640;

        let page = query_managers_page(&game, ManagersStatus::All);

        let user = page
            .managers
            .iter()
            .find(|summary| summary.id == "user")
            .expect("the user is listed");
        assert_eq!(user.reputation.value, 640);
    }

    /// Given the same world
    /// When only the employed, then only those between jobs, are asked for
    /// Then the first lists the three in charge of a club (the stand-in included) and the second
    ///      the two without one
    #[test]
    fn the_status_filter_splits_employed_from_between_jobs() {
        let game = world();

        assert_eq!(
            ids(&query_managers_page(&game, ManagersStatus::Employed)),
            vec!["ai", "cover", "user"]
        );
        assert_eq!(
            ids(&query_managers_page(&game, ManagersStatus::Unemployed)),
            vec!["free-a", "free-b"]
        );
    }

    /// Given Gamma with nobody in charge, and the user's club
    /// When the directory is asked for, under any filter
    /// Then Gamma is the one vacancy, with its reputation; the user's club is not vacant
    #[test]
    fn a_club_without_a_manager_is_a_vacancy() {
        let expected = vec![Vacancy {
            club_id: "gamma".into(),
            club_name: "Gamma".into(),
            club_reputation: 300,
        }];
        let game = world();
        for status in [
            ManagersStatus::All,
            ManagersStatus::Employed,
            ManagersStatus::Unemployed,
        ] {
            assert_eq!(query_managers_page(&game, status).vacancies, expected);
        }
    }

    /// Given the status the screen sends
    /// When it is read off the wire
    /// Then "all", "employed" and "unemployed" name the three filters
    #[test]
    fn the_status_reads_from_the_wire() {
        let read = |text: &str| serde_json::from_str::<ManagersStatus>(text).unwrap();
        assert_eq!(
            [
                read("\"all\""),
                read("\"employed\""),
                read("\"unemployed\"")
            ],
            [
                ManagersStatus::All,
                ManagersStatus::Employed,
                ManagersStatus::Unemployed
            ]
        );
    }
}
