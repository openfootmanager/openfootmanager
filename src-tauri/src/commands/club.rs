use log::info;
use tauri::State;

use ofm_core::finances::{self, FinanceHealthLevel};
use ofm_core::game::Game;
use ofm_core::state::StateManager;

#[tauri::command]
pub fn upgrade_facility(state: State<'_, StateManager>, facility: String) -> Result<Game, String> {
    upgrade_facility_internal(&state, &facility)
}

fn upgrade_facility_internal(state: &StateManager, facility: &str) -> Result<Game, String> {
    info!("[cmd] upgrade_facility: {}", facility);
    let mut game = state
        .get_game(|g| g.clone())
        .ok_or("be.error.noActiveGameSession".to_string())?;

    let team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("be.error.noTeamAssigned".to_string())?;

    let facility_type = match facility {
        "Training" => domain::team::FacilityType::Training,
        "Medical" => domain::team::FacilityType::Medical,
        "Scouting" => domain::team::FacilityType::Scouting,
        _ => return Err("be.error.unknownFacilityType".to_string()),
    };

    let snapshot = finances::team_finance_snapshot(&game, &team_id)
        .ok_or("be.error.managedTeamNotFound".to_string())?;
    if snapshot.currently_over_budget {
        return Err("be.error.finance.facilityUpgradeOverBudget".to_string());
    }
    if matches!(
        snapshot.overall_status,
        FinanceHealthLevel::Warning | FinanceHealthLevel::Critical
    ) {
        return Err("be.error.finance.facilityUpgradeCritical".to_string());
    }

    let team = game
        .teams
        .iter_mut()
        .find(|team| team.id == team_id)
        .ok_or("be.error.managedTeamNotFound".to_string())?;

    ofm_core::club::upgrade_facility(team, facility_type)?;

    state.set_game(game.clone());
    Ok(game)
}

#[cfg(test)]
mod tests {
    use super::upgrade_facility_internal;
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::player::{Player, PlayerAttributes, Position};
    use domain::team::Team;
    use ofm_core::clock::GameClock;
    use ofm_core::game::Game;
    use ofm_core::state::StateManager;

    fn default_attrs() -> PlayerAttributes {
        PlayerAttributes {
            pace: 60,
            stamina: 60,
            strength: 60,
            agility: 60,
            passing: 60,
            shooting: 60,
            tackling: 60,
            dribbling: 60,
            defending: 60,
            positioning: 60,
            vision: 60,
            decisions: 60,
            composure: 60,
            aggression: 60,
            teamwork: 60,
            leadership: 60,
            handling: 30,
            reflexes: 30,
            aerial: 60,
        }
    }

    fn make_player(id: &str, team_id: &str, wage: u32) -> Player {
        let mut player = Player::new(
            id.to_string(),
            "Player".to_string(),
            "Player".to_string(),
            "1995-01-01".to_string(),
            "England".to_string(),
            Position::Forward,
            default_attrs(),
        );
        player.team_id = Some(team_id.to_string());
        player.wage = wage;
        player
    }

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
        team.finance = 1_000_000;
        team.manager_id = Some("manager-1".to_string());
        team
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

        Game::new(clock, manager, vec![make_team()], vec![], vec![], vec![])
    }

    #[test]
    fn upgrade_facility_internal_updates_state() {
        let state = StateManager::new();
        state.set_game(make_game());

        let response = upgrade_facility_internal(&state, "Medical").expect("response");
        let team = response
            .teams
            .iter()
            .find(|team| team.id == "team-1")
            .unwrap();

        assert_eq!(team.facilities.medical, 2);
        assert_eq!(team.finance, 750_000);

        let stored_game = state.get_game(|game| game.clone()).expect("stored game");
        let stored_team = stored_game
            .teams
            .iter()
            .find(|team| team.id == "team-1")
            .expect("stored team should exist");
        assert_eq!(stored_team.facilities.medical, 2);
        assert_eq!(stored_team.finance, 750_000);
    }

    #[test]
    fn upgrade_facility_internal_rejects_over_budget_clubs() {
        let state = StateManager::new();
        let mut game = make_game();
        game.teams[0].wage_budget = 100_000;
        game.players
            .push(make_player("player-1", "team-1", 220_000));
        state.set_game(game);

        let error = upgrade_facility_internal(&state, "Training").expect_err("should fail");

        assert_eq!(error, "be.error.finance.facilityUpgradeOverBudget");

        let stored_game = state
            .get_game(|current| current.clone())
            .expect("stored game");
        let stored_team = stored_game
            .teams
            .iter()
            .find(|team| team.id == "team-1")
            .expect("stored team should exist");
        assert_eq!(stored_team.facilities.training, 1);
        assert_eq!(stored_team.finance, 1_000_000);
    }

    #[test]
    fn upgrade_facility_internal_rejects_warning_finance_clubs() {
        let state = StateManager::new();
        let mut game = make_game();
        game.teams[0].finance = 40_000;
        game.teams[0].wage_budget = 1_000_000;
        game.players.push(make_player("player-1", "team-1", 260_000));
        state.set_game(game);

        let error = upgrade_facility_internal(&state, "Medical").expect_err("should fail");

        assert_eq!(error, "be.error.finance.facilityUpgradeCritical");
    }
}
