//! Where the user's career stands, for screens that must send the player somewhere.

use crate::game::Game;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CareerStage {
    /// A career that has not taken its first club yet (created over MCP or abandoned at team
    /// selection): the next step is choosing one.
    AwaitingClub,
    /// A manager who has had a club and has none now: job offers, not team selection.
    Unemployed,
    Employed,
}

/// A manager with no club who has never held one has not started yet; every club a manager takes
/// opens a career entry (`job_offers::hire_manager`), so an empty history means no club so far.
pub fn career_stage(game: &Game) -> CareerStage {
    if game.manager.team_id.is_some() {
        CareerStage::Employed
    } else if game.manager.career_history.is_empty()
        && game.manager.career_stats.matches_managed == 0
    {
        CareerStage::AwaitingClub
    } else {
        CareerStage::Unemployed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use chrono::{TimeZone, Utc};
    use domain::manager::{Manager, ManagerCareerEntry};

    fn game_with(manager: Manager) -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 7, 1, 12, 0, 0).unwrap());
        Game::new(clock, manager, vec![], vec![], vec![], vec![])
    }

    fn manager() -> Manager {
        Manager::new(
            "mgr1".to_string(),
            "Alex".to_string(),
            "Boss".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        )
    }

    /// Given a manager who has never held a club and has none now
    /// When the career stage is read
    /// Then the career is awaiting its first club
    #[test]
    fn a_manager_who_never_held_a_club_awaits_one() {
        assert_eq!(
            career_stage(&game_with(manager())),
            CareerStage::AwaitingClub
        );
    }

    /// Given a manager with a club
    /// When the career stage is read
    /// Then the manager is employed
    #[test]
    fn a_manager_with_a_club_is_employed() {
        let mut manager = manager();
        manager.hire("team1".to_string());

        assert_eq!(career_stage(&game_with(manager)), CareerStage::Employed);
    }

    /// Given a manager who was sacked (a closed career entry, no club now)
    /// When the career stage is read
    /// Then the manager is unemployed, not back at the start
    #[test]
    fn a_sacked_manager_is_unemployed() {
        let mut manager = manager();
        manager.career_history.push(ManagerCareerEntry::open(
            "team1".to_string(),
            "Test FC".to_string(),
            "2026-07-01".to_string(),
        ));
        manager.fire("2026-10-01");

        assert_eq!(career_stage(&game_with(manager)), CareerStage::Unemployed);
    }

    /// Given a clubless manager with matches on record but no history entry (an older save)
    /// When the career stage is read
    /// Then the manager is unemployed: a manager who has managed is never "new"
    #[test]
    fn a_manager_with_matches_but_no_history_is_unemployed() {
        let mut manager = manager();
        manager.career_stats.matches_managed = 12;

        assert_eq!(career_stage(&game_with(manager)), CareerStage::Unemployed);
    }

    /// Given each stage,
    /// when it is serialised for the frontend,
    /// then it is the camelCase name the TypeScript union spells.
    #[test]
    fn the_stage_serialises_in_camel_case() {
        let names: Vec<String> = [
            CareerStage::AwaitingClub,
            CareerStage::Unemployed,
            CareerStage::Employed,
        ]
        .iter()
        .map(|stage| {
            serde_json::to_value(stage)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();

        assert_eq!(names, ["awaitingClub", "unemployed", "employed"]);
    }
}
