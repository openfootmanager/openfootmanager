use log::info;
use std::sync::Arc;
use tauri::State;

use ofm_core::game::Game;
use ofm_core::state::{Career, StateManager};

use crate::commands::util::{mutate_active_game_for, user_team_id, user_team_mut};

#[tauri::command]
pub fn hire_staff(
    state: State<'_, Arc<StateManager>>,
    staff_id: String,
    career_token: u64,
) -> Result<Game, String> {
    hire_staff_internal(&state, Career::Token(career_token), &staff_id)
}

pub fn hire_staff_internal(
    state: &StateManager,
    career: Career,
    staff_id: &str,
) -> Result<Game, String> {
    info!("[cmd] hire_staff: staff_id={}", staff_id);
    mutate_active_game_for(state, career, |game| {
        let team_id = user_team_id(game)?;

        // Read and validate before writing anything: mutate_active_game mutates
        // the live game, so a write made before an error is returned would stick.
        let staff_wage = {
            let staff = game
                .staff
                .iter()
                .find(|s| s.id == staff_id)
                .ok_or("be.error.staffMemberNotFound".to_string())?;

            if staff.team_id.is_some() {
                return Err("be.error.staffMemberAlreadyEmployed".to_string());
            }

            staff.wage
        };

        // Deduct wage from team budget
        let team = user_team_mut(game)?;
        team.season_expenses += staff_wage as i64;

        game.staff
            .iter_mut()
            .find(|s| s.id == staff_id)
            .ok_or("be.error.staffMemberNotFound".to_string())?
            .team_id = Some(team_id);

        game.available_staff_market_last_activity_date =
            Some(game.clock.current_date.format("%Y-%m-%d").to_string());
        ofm_core::generator::process_available_staff_market(game);

        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::{hire_staff_internal, release_staff_internal};
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::staff::{Staff, StaffAttributes, StaffRole};
    use domain::team::Team;
    use ofm_core::clock::GameClock;
    use ofm_core::game::Game;
    use ofm_core::state::Career;
    use ofm_core::state::StateManager;

    fn make_team() -> Team {
        let mut team = Team::new(
            "team-1".to_string(),
            "User FC".to_string(),
            "USR".to_string(),
            "England".to_string(),
            "London".to_string(),
            "User Ground".to_string(),
            25_000,
        );
        team.manager_id = Some("manager-1".to_string());
        team
    }

    fn make_staff() -> Staff {
        let mut staff = Staff::new(
            "staff-1".to_string(),
            "Alex".to_string(),
            "Coach".to_string(),
            "1985-01-01".to_string(),
            StaffRole::Coach,
            StaffAttributes {
                coaching: 70,
                judging_ability: 50,
                judging_potential: 50,
                physiotherapy: 30,
            },
        );
        staff.wage = 12_000;
        staff
    }

    fn make_employed_staff() -> Staff {
        let mut staff = make_staff();
        staff.team_id = Some("team-1".to_string());
        staff
    }

    fn make_game() -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "manager-1".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("team-1".to_string());

        Game::new(
            clock,
            manager,
            vec![make_team()],
            vec![],
            vec![make_staff()],
            vec![],
        )
    }

    fn make_game_with_employed_staff() -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "manager-1".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("team-1".to_string());

        Game::new(
            clock,
            manager,
            vec![make_team()],
            vec![],
            vec![make_employed_staff()],
            vec![],
        )
    }

    /// The manager holds a team id that no club in the world has. Both commands
    /// used to swallow that: the wage adjustment sat behind `if let Some(team)`
    /// with no else, so the staff record changed and the club's books did not.
    fn with_dangling_team_id(mut game: Game) -> Game {
        game.teams[0].id = "team-elsewhere".to_string();
        game
    }

    /// Given career B is active (token 2) and a screen holds career A's token (1)
    /// When a hire and a release arrive with A's token
    /// Then both are refused with the career-changed key and career B's staff are untouched.
    #[test]
    fn staff_writes_for_a_replaced_career_are_refused() {
        let state = StateManager::new();
        let mut game = make_game_with_employed_staff();
        game.career_token = 2;
        state.set_game(game);
        let before = serde_json::to_value(state.get_game(|g| g.clone()).unwrap()).unwrap();
        let stale = Career::Token(1);

        let hire = hire_staff_internal(&state, stale, "staff-1").err();
        let release = release_staff_internal(&state, stale, "staff-1").err();

        assert_eq!(hire, Some("be.error.careerChanged".to_string()));
        assert_eq!(release, Some("be.error.careerChanged".to_string()));
        let after = serde_json::to_value(state.get_game(|g| g.clone()).unwrap()).unwrap();
        assert_eq!(after, before);
    }

    #[test]
    fn hire_staff_internal_reports_a_team_id_that_matches_no_club() {
        let state = StateManager::new();
        state.set_game(with_dangling_team_id(make_game()));

        let result = hire_staff_internal(&state, Career::Active, "staff-1");

        assert_eq!(result.err(), Some("be.error.teamNotFound".into()));

        // The wage adjustment is the fallible step and now runs first, so the
        // staff member must not have been hired either.
        let stored = state.get_game(|game| game.clone()).expect("stored game");
        let staff = stored
            .staff
            .iter()
            .find(|staff| staff.id == "staff-1")
            .expect("stored staff should exist");
        assert!(staff.team_id.is_none());
        assert_eq!(stored.teams[0].season_expenses, 0);
    }

    #[test]
    fn release_staff_internal_reports_a_team_id_that_matches_no_club() {
        let state = StateManager::new();
        state.set_game(with_dangling_team_id(make_game_with_employed_staff()));

        let result = release_staff_internal(&state, Career::Active, "staff-1");

        assert_eq!(result.err(), Some("be.error.teamNotFound".into()));

        // Likewise: the staff member stays employed rather than being released
        // while the command reports failure.
        let stored = state.get_game(|game| game.clone()).expect("stored game");
        let staff = stored
            .staff
            .iter()
            .find(|staff| staff.id == "staff-1")
            .expect("stored staff should exist");
        assert_eq!(staff.team_id.as_deref(), Some("team-1"));
    }

    #[test]
    fn hire_staff_internal_updates_state() {
        let state = StateManager::new();
        state.set_game(make_game());

        let response = hire_staff_internal(&state, Career::Active, "staff-1").expect("response");
        let staff = response
            .staff
            .iter()
            .find(|staff| staff.id == "staff-1")
            .unwrap();
        let team = response
            .teams
            .iter()
            .find(|team| team.id == "team-1")
            .unwrap();
        let available_staff = response
            .staff
            .iter()
            .filter(|staff| staff.team_id.is_none())
            .count();

        assert_eq!(staff.team_id.as_deref(), Some("team-1"));
        assert_eq!(team.season_expenses, 12_000);
        assert_eq!(available_staff, 12);
        assert_eq!(
            response
                .available_staff_market_last_activity_date
                .as_deref(),
            Some("2026-08-01")
        );

        let stored_game = state.get_game(|game| game.clone()).expect("stored game");
        let stored_staff = stored_game
            .staff
            .iter()
            .find(|staff| staff.id == "staff-1")
            .expect("stored staff should exist");
        let stored_team = stored_game
            .teams
            .iter()
            .find(|team| team.id == "team-1")
            .expect("stored team should exist");
        assert_eq!(stored_staff.team_id.as_deref(), Some("team-1"));
        assert_eq!(stored_team.season_expenses, 12_000);
        assert_eq!(
            stored_game
                .available_staff_market_last_activity_date
                .as_deref(),
            Some("2026-08-01")
        );
    }

    #[test]
    fn release_staff_internal_updates_state() {
        let state = StateManager::new();
        state.set_game(make_game_with_employed_staff());

        let response = release_staff_internal(&state, Career::Active, "staff-1").expect("response");
        let staff = response
            .staff
            .iter()
            .find(|staff| staff.id == "staff-1")
            .unwrap();

        assert!(staff.team_id.is_none());

        let stored_game = state.get_game(|game| game.clone()).expect("stored game");
        let stored_staff = stored_game
            .staff
            .iter()
            .find(|staff| staff.id == "staff-1")
            .expect("stored staff should exist");
        assert!(stored_staff.team_id.is_none());
    }
}

#[tauri::command]
pub fn release_staff(
    state: State<'_, Arc<StateManager>>,
    staff_id: String,
    career_token: u64,
) -> Result<Game, String> {
    release_staff_internal(&state, Career::Token(career_token), &staff_id)
}

pub fn release_staff_internal(
    state: &StateManager,
    career: Career,
    staff_id: &str,
) -> Result<Game, String> {
    info!("[cmd] release_staff: staff_id={}", staff_id);
    mutate_active_game_for(state, career, |game| {
        let team_id = user_team_id(game)?;

        // Read and validate before writing anything: mutate_active_game mutates
        // the live game, so a write made before an error is returned would stick.
        let staff_wage = {
            let staff = game
                .staff
                .iter()
                .find(|s| s.id == staff_id)
                .ok_or("be.error.staffMemberNotFound".to_string())?;

            if staff.team_id.as_deref() != Some(&team_id) {
                return Err("be.error.staffMemberNotInTeam".to_string());
            }

            staff.wage
        };

        let team = user_team_mut(game)?;
        team.season_expenses = team.season_expenses.saturating_sub(staff_wage as i64);

        game.staff
            .iter_mut()
            .find(|s| s.id == staff_id)
            .ok_or("be.error.staffMemberNotFound".to_string())?
            .team_id = None;

        Ok(())
    })
}
