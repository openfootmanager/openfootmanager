use log::info;
use std::sync::Arc;
use tauri::State;

use ofm_core::state::StateManager;

#[tauri::command]
pub fn check_season_complete(state: State<'_, Arc<StateManager>>) -> Result<bool, String> {
    log::debug!("[cmd] check_season_complete");
    state
        .get_game(ofm_core::end_of_season::is_season_complete)
        .ok_or_else(|| "be.error.noActiveGameSession".to_string())
}

#[tauri::command]
pub fn advance_to_next_season(
    state: State<'_, Arc<StateManager>>,
) -> Result<serde_json::Value, String> {
    advance_to_next_season_internal(&state)
}

pub fn advance_to_next_season_internal(state: &StateManager) -> Result<serde_json::Value, String> {
    let _operation = crate::application::live_session::idle_operation(state)?;
    info!("[cmd] advance_to_next_season");
    state
        .update_game(|game| {
            let summary = ofm_core::end_of_season::advance_to_next_season(game)?;

            if game.manager.team_id.is_none() {
                Ok(serde_json::json!({
                    "action": "fired",
                    "game": game,
                    "summary": summary,
                }))
            } else {
                Ok(serde_json::json!({
                    "game": game,
                    "summary": summary,
                }))
            }
        })
        .unwrap_or_else(|| Err("be.error.noActiveGameSession".to_string()))
}

#[tauri::command]
pub fn get_season_awards(
    state: State<'_, Arc<StateManager>>,
) -> Result<ofm_core::season_awards::SeasonAwards, String> {
    log::debug!("[cmd] get_season_awards");
    state
        .get_game(|game| {
            // The awards screen shows the race within the user's own division.
            let user_team_id = game.manager.team_id.clone().unwrap_or_default();
            match ofm_core::end_of_season::user_division(game, &user_team_id) {
                Some(division) => {
                    ofm_core::season_awards::compute_division_season_awards(game, division)
                }
                None => ofm_core::season_awards::compute_season_awards(game),
            }
        })
        .ok_or_else(|| "be.error.noActiveGameSession".to_string())
}

#[cfg(test)]
mod tests {
    use super::advance_to_next_season_internal;
    use chrono::{TimeZone, Utc};
    use domain::league::{
        Fixture, FixtureCompetition, FixtureStatus, League, MatchResult, StandingEntry,
    };
    use domain::manager::Manager;
    use domain::player::{Player, PlayerAttributes, Position};
    use domain::team::Team;
    use ofm_core::clock::GameClock;
    use ofm_core::game::{BoardObjective, Game, ObjectiveType};
    use ofm_core::state::StateManager;

    fn completed_checkpoint() -> Game {
        let mut manager = Manager::new(
            "manager".into(),
            "Test".into(),
            "Manager".into(),
            "1980-01-01".into(),
            "England".into(),
        );
        manager.hire("home".into());
        let teams = [("home", "Home FC"), ("away", "Away FC")]
            .into_iter()
            .map(|(id, name)| {
                Team::new(
                    id.into(),
                    name.into(),
                    name[..3].into(),
                    "England".into(),
                    "London".into(),
                    "Ground".into(),
                    10_000,
                )
            })
            .collect();
        let players = ["home", "away"]
            .into_iter()
            .map(|team| {
                let mut player = Player::new(
                    format!("{team}-player"),
                    team.into(),
                    format!("{team} Player"),
                    "1995-01-01".into(),
                    "England".into(),
                    Position::Forward,
                    PlayerAttributes {
                        pace: 65,
                        stamina: 65,
                        strength: 65,
                        agility: 65,
                        passing: 65,
                        shooting: 65,
                        tackling: 65,
                        dribbling: 65,
                        defending: 65,
                        positioning: 65,
                        vision: 65,
                        decisions: 65,
                        composure: 65,
                        aggression: 50,
                        teamwork: 65,
                        leadership: 50,
                        handling: 20,
                        reflexes: 30,
                        aerial: 60,
                    },
                );
                player.team_id = Some(team.into());
                player.stats.appearances = 2;
                player.stats.goals = if team == "home" { 3 } else { 1 };
                player
            })
            .collect();
        let fixtures = [
            ("first", "home", "away", 2, 1),
            ("return", "away", "home", 0, 1),
        ]
        .into_iter()
        .map(|(id, home, away, home_goals, away_goals)| Fixture {
            id: id.into(),
            competition_id: "league".into(),
            date: "2025-06-01".into(),
            matchday: 1,
            home_team_id: home.into(),
            away_team_id: away.into(),
            status: FixtureStatus::Completed,
            result: Some(MatchResult {
                home_goals,
                away_goals,
                ..Default::default()
            }),
            ..Default::default()
        })
        .collect();
        let mut home = StandingEntry::new("home".into());
        home.played = 2;
        home.won = 2;
        home.points = 6;
        home.goals_for = 3;
        home.goals_against = 1;
        let mut away = StandingEntry::new("away".into());
        away.played = 2;
        away.lost = 2;
        away.goals_for = 1;
        away.goals_against = 3;
        let league = League {
            id: "league".into(),
            name: "Test League".into(),
            season: 2025,
            participant_ids: vec!["home".into(), "away".into()],
            country_id: Some("ENG".into()),
            fixtures,
            standings: vec![home, away],
            ..Default::default()
        };
        let mut game = Game::new(
            GameClock::new(Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap()),
            manager,
            teams,
            players,
            vec![],
            vec![],
        );
        game.seed = 17;
        game.league = Some(league.clone());
        game.competitions = vec![league];
        game
    }

    /// Given all required fixtures completed, when the shared command rolls over,
    /// then the season increments, the table and player stats reset, and fixtures regenerate.
    #[test]
    fn season_advance_rolls_the_season_over() {
        let state = StateManager::new();
        state.set_game(completed_checkpoint());
        let response = advance_to_next_season_internal(&state).unwrap();
        assert_eq!(response["summary"]["season"], 2025);
        let after = state.get_game(Clone::clone).unwrap();
        let league = &after.competitions[0];
        assert_eq!(league.season, 2026);
        assert_eq!(
            league
                .fixtures
                .iter()
                .filter(|fixture| fixture.competition == FixtureCompetition::League)
                .count(),
            2
        );
        assert!(league
            .fixtures
            .iter()
            .all(|fixture| fixture.status == FixtureStatus::Scheduled && fixture.result.is_none()));
        assert!(league
            .standings
            .iter()
            .all(|row| row.played == 0 && row.points == 0));
        assert!(after
            .players
            .iter()
            .all(|player| player.stats.goals == 0 && player.stats.appearances == 0));
    }

    /// Given a required scheduled fixture, when rollover is requested,
    /// then the season-not-complete key is returned and the whole game stays unchanged.
    #[test]
    fn rolling_over_before_completion_is_refused() {
        let state = StateManager::new();
        let mut game = completed_checkpoint();
        game.competitions[0].fixtures[0].status = FixtureStatus::Scheduled;
        game.league = Some(game.competitions[0].clone());
        let before = serde_json::to_value(&game).unwrap();
        state.set_game(game);
        assert_eq!(
            advance_to_next_season_internal(&state).unwrap_err(),
            "be.error.seasonNotComplete"
        );
        assert_eq!(
            state
                .get_game(|game| serde_json::to_value(game).unwrap())
                .unwrap(),
            before
        );
    }

    /// Given a rolled-over season, when rollover is requested again,
    /// then it is refused with no second date tick, prize payment or reset.
    #[test]
    fn rolling_over_twice_is_refused() {
        let state = StateManager::new();
        state.set_game(completed_checkpoint());
        advance_to_next_season_internal(&state).unwrap();
        let before = state
            .get_game(|game| serde_json::to_value(game).unwrap())
            .unwrap();
        assert_eq!(
            advance_to_next_season_internal(&state).unwrap_err(),
            "be.error.seasonNotComplete"
        );
        assert_eq!(
            state
                .get_game(|game| serde_json::to_value(game).unwrap())
                .unwrap(),
            before
        );
    }

    /// Given a finished season whose failed objective triggers dismissal,
    /// when it rolls over, then the new season, summary and firing are one operation.
    #[test]
    fn the_board_can_fire_at_rollover() {
        let state = StateManager::new();
        let mut game = completed_checkpoint();
        game.manager.satisfaction = 20;
        game.manager.warning_stage = 1;
        game.board_objectives.push(BoardObjective {
            id: "objective".into(),
            objective_type: ObjectiveType::LeaguePosition,
            description: "Win the league".into(),
            target: 1,
            met: false,
        });
        state.set_game(game);
        let response = advance_to_next_season_internal(&state).unwrap();
        assert_eq!(response["action"], "fired");
        assert_eq!(response["summary"]["season"], 2025);
        assert_eq!(response["game"]["competitions"][0]["season"], 2026);
        assert!(response["game"]["manager"]["team_id"].is_null());
    }

    /// Given one saved checkpoint loaded twice, when the shared command used by both adapters runs,
    /// then both games are identical, including generated fixture identities.
    #[test]
    fn the_tool_and_the_command_agree() {
        let checkpoint = serde_json::to_string(&completed_checkpoint()).unwrap();
        let command = StateManager::new();
        let tool = StateManager::new();
        command.set_game(serde_json::from_str(&checkpoint).unwrap());
        tool.set_game(serde_json::from_str(&checkpoint).unwrap());
        let command_result = advance_to_next_season_internal(&command).unwrap();
        let tool_result = advance_to_next_season_internal(&tool).unwrap();
        let ids = |result: &serde_json::Value| {
            result["game"]["competitions"][0]["fixtures"]
                .as_array()
                .unwrap()
                .iter()
                .map(|fixture| fixture["id"].clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            ids(&command_result),
            ids(&tool_result),
            "same checkpoint must regenerate identical fixture IDs"
        );
        assert!(
            command_result["game"] == tool_result["game"],
            "same checkpoint must produce an identical game"
        );
    }
}
