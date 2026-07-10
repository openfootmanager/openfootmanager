use log::info;
use serde::Serialize;
use std::sync::Arc;
use tauri::State;

use domain::negotiation::NegotiationFeedback;
use domain::player::ContractTalksStatus;
use ofm_core::contracts::{
    ContractTerminationPreview, ContractTerminationResult, DelegatedRenewalOptions,
    DelegatedRenewalReport, RenewalDecision, RenewalFinancialProjection, RenewalOffer,
};
use ofm_core::game::Game;
use ofm_core::squad_safety::SquadSafetyReport;
use ofm_core::state::StateManager;

use crate::commands::util::mutate_active_game;

#[derive(Debug, Clone, Serialize)]
pub struct RenewalCommandResponse {
    pub outcome: RenewalDecision,
    pub game: Game,
    pub suggested_wage: Option<u32>,
    pub suggested_years: Option<u32>,
    pub session_status: String,
    pub is_terminal: bool,
    pub cooled_off: bool,
    pub feedback: Option<NegotiationFeedback>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DelegatedRenewalCommandResponse {
    pub game: Game,
    pub report: DelegatedRenewalReport,
}

#[derive(Debug, Clone, Serialize)]
pub struct RenewalFinancialProjectionCommandResponse {
    pub projection: RenewalFinancialProjection,
}

#[derive(Debug, Clone, Serialize)]
pub struct FreeAgentContractCommandResponse {
    pub outcome: RenewalDecision,
    pub game: Game,
    pub suggested_wage: Option<u32>,
    pub suggested_years: Option<u32>,
    pub session_status: String,
    pub is_terminal: bool,
    pub cooled_off: bool,
    pub feedback: Option<NegotiationFeedback>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FreeAgentContractProjectionCommandResponse {
    pub projection: RenewalFinancialProjection,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContractExitIntentCommandResponse {
    pub game: Game,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContractTerminationPreviewCommandResponse {
    pub preview: ContractTerminationPreview,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContractTerminationCommandResponse {
    pub game: Game,
    pub severance_cost: i64,
    pub squad_safety: SquadSafetyReport,
}

fn serialize_session_status(status: ContractTalksStatus) -> String {
    match status {
        ContractTalksStatus::Idle => "idle",
        ContractTalksStatus::Open => "open",
        ContractTalksStatus::Agreed => "agreed",
        ContractTalksStatus::Blocked => "blocked",
        ContractTalksStatus::Stalled => "stalled",
    }
    .to_string()
}

#[tauri::command]
pub async fn propose_renewal(
    state: State<'_, Arc<StateManager>>,
    player_id: String,
    weekly_wage: u32,
    contract_years: u32,
) -> Result<RenewalCommandResponse, String> {
    propose_renewal_internal(&state, &player_id, weekly_wage, contract_years)
}

#[tauri::command]
pub async fn delegate_renewals(
    state: State<'_, Arc<StateManager>>,
    player_ids: Option<Vec<String>>,
    max_wage_increase_pct: u32,
    max_contract_years: u32,
) -> Result<DelegatedRenewalCommandResponse, String> {
    delegate_renewals_internal(
        &state,
        player_ids,
        max_wage_increase_pct,
        max_contract_years,
    )
}

#[tauri::command]
pub async fn preview_renewal_financial_impact(
    state: State<'_, Arc<StateManager>>,
    player_id: String,
    weekly_wage: u32,
) -> Result<RenewalFinancialProjectionCommandResponse, String> {
    preview_renewal_financial_impact_internal(&state, &player_id, weekly_wage)
}

#[tauri::command]
pub async fn offer_free_agent_contract(
    state: State<'_, Arc<StateManager>>,
    player_id: String,
    weekly_wage: u32,
    contract_years: u32,
) -> Result<FreeAgentContractCommandResponse, String> {
    offer_free_agent_contract_internal(&state, &player_id, weekly_wage, contract_years)
}

#[tauri::command]
pub async fn preview_free_agent_contract_impact(
    state: State<'_, Arc<StateManager>>,
    player_id: String,
    weekly_wage: u32,
) -> Result<FreeAgentContractProjectionCommandResponse, String> {
    preview_free_agent_contract_impact_internal(&state, &player_id, weekly_wage)
}

#[tauri::command]
pub async fn set_contract_exit_intent(
    state: State<'_, Arc<StateManager>>,
    player_id: String,
    reason: Option<String>,
) -> Result<ContractExitIntentCommandResponse, String> {
    set_contract_exit_intent_internal(&state, &player_id, reason)
}

#[tauri::command]
pub async fn clear_contract_exit_intent(
    state: State<'_, Arc<StateManager>>,
    player_id: String,
) -> Result<ContractExitIntentCommandResponse, String> {
    clear_contract_exit_intent_internal(&state, &player_id)
}

#[tauri::command]
pub async fn preview_contract_termination(
    state: State<'_, Arc<StateManager>>,
    player_id: String,
) -> Result<ContractTerminationPreviewCommandResponse, String> {
    preview_contract_termination_internal(&state, &player_id)
}

#[tauri::command]
pub async fn terminate_contract_now(
    state: State<'_, Arc<StateManager>>,
    player_id: String,
) -> Result<ContractTerminationCommandResponse, String> {
    terminate_contract_now_internal(&state, &player_id)
}

pub fn propose_renewal_internal(
    state: &StateManager,
    player_id: &str,
    weekly_wage: u32,
    contract_years: u32,
) -> Result<RenewalCommandResponse, String> {
    info!(
        "[cmd] propose_renewal: player_id={}, weekly_wage={}, contract_years={}",
        player_id, weekly_wage, contract_years
    );

    // propose_renewal validates (team/player) before mutating wage/contract.
    state
        .update_game(|game| {
            let outcome = ofm_core::contracts::propose_renewal(
                game,
                player_id,
                RenewalOffer {
                    weekly_wage,
                    contract_years,
                },
            )?;
            Ok(RenewalCommandResponse {
                outcome: outcome.decision,
                game: game.clone(),
                suggested_wage: outcome.suggested_wage,
                suggested_years: outcome.suggested_years,
                session_status: serialize_session_status(outcome.session_status),
                is_terminal: outcome.is_terminal,
                cooled_off: outcome.cooled_off,
                feedback: outcome.feedback,
            })
        })
        .unwrap_or_else(|| Err("be.error.noActiveGameSession".to_string()))
}

pub fn delegate_renewals_internal(
    state: &StateManager,
    player_ids: Option<Vec<String>>,
    max_wage_increase_pct: u32,
    max_contract_years: u32,
) -> Result<DelegatedRenewalCommandResponse, String> {
    info!(
        "[cmd] delegate_renewals: player_ids={:?}, max_wage_increase_pct={}, max_contract_years={}",
        player_ids, max_wage_increase_pct, max_contract_years
    );

    // delegate_renewals collects per-player results into a report rather than
    // erroring mid-iteration, so in-place mutation is safe.
    state
        .update_game(|game| {
            let report = ofm_core::contracts::delegate_renewals(
                game,
                DelegatedRenewalOptions {
                    player_ids,
                    max_wage_increase_pct,
                    max_contract_years,
                },
            )?;
            Ok(DelegatedRenewalCommandResponse {
                game: game.clone(),
                report,
            })
        })
        .unwrap_or_else(|| Err("be.error.noActiveGameSession".to_string()))
}

pub fn preview_renewal_financial_impact_internal(
    state: &StateManager,
    player_id: &str,
    weekly_wage: u32,
) -> Result<RenewalFinancialProjectionCommandResponse, String> {
    info!(
        "[cmd] preview_renewal_financial_impact: player_id={}, weekly_wage={}",
        player_id, weekly_wage
    );

    let projection = state
        .get_game(|game| {
            ofm_core::contracts::project_renewal_financial_impact(game, player_id, weekly_wage)
        })
        .ok_or_else(|| "be.error.noActiveGameSession".to_string())??;

    Ok(RenewalFinancialProjectionCommandResponse { projection })
}

pub fn offer_free_agent_contract_internal(
    state: &StateManager,
    player_id: &str,
    weekly_wage: u32,
    contract_years: u32,
) -> Result<FreeAgentContractCommandResponse, String> {
    info!(
        "[cmd] offer_free_agent_contract: player_id={}, weekly_wage={}, contract_years={}",
        player_id, weekly_wage, contract_years
    );

    // offer_free_agent_contract validates (team/player/free-agent) before mutating.
    state
        .update_game(|game| {
            let outcome = ofm_core::contracts::offer_free_agent_contract(
                game,
                player_id,
                RenewalOffer {
                    weekly_wage,
                    contract_years,
                },
            )?;
            Ok(FreeAgentContractCommandResponse {
                outcome: outcome.decision,
                game: game.clone(),
                suggested_wage: outcome.suggested_wage,
                suggested_years: outcome.suggested_years,
                session_status: serialize_session_status(outcome.session_status),
                is_terminal: outcome.is_terminal,
                cooled_off: outcome.cooled_off,
                feedback: outcome.feedback,
            })
        })
        .unwrap_or_else(|| Err("be.error.noActiveGameSession".to_string()))
}

pub fn preview_free_agent_contract_impact_internal(
    state: &StateManager,
    player_id: &str,
    weekly_wage: u32,
) -> Result<FreeAgentContractProjectionCommandResponse, String> {
    info!(
        "[cmd] preview_free_agent_contract_impact: player_id={}, weekly_wage={}",
        player_id, weekly_wage
    );

    let projection = state
        .get_game(|game| {
            ofm_core::contracts::project_free_agent_contract_impact(game, player_id, weekly_wage)
        })
        .ok_or_else(|| "be.error.noActiveGameSession".to_string())??;

    Ok(FreeAgentContractProjectionCommandResponse { projection })
}

pub fn set_contract_exit_intent_internal(
    state: &StateManager,
    player_id: &str,
    reason: Option<String>,
) -> Result<ContractExitIntentCommandResponse, String> {
    info!("[cmd] set_contract_exit_intent: player_id={}", player_id);

    mutate_active_game(state, |game| {
        ofm_core::contracts::set_contract_exit_intent(game, player_id, reason)
    })
    .map(|game| ContractExitIntentCommandResponse { game })
}

pub fn clear_contract_exit_intent_internal(
    state: &StateManager,
    player_id: &str,
) -> Result<ContractExitIntentCommandResponse, String> {
    info!("[cmd] clear_contract_exit_intent: player_id={}", player_id);

    mutate_active_game(state, |game| {
        ofm_core::contracts::clear_contract_exit_intent(game, player_id)
    })
    .map(|game| ContractExitIntentCommandResponse { game })
}

pub fn preview_contract_termination_internal(
    state: &StateManager,
    player_id: &str,
) -> Result<ContractTerminationPreviewCommandResponse, String> {
    info!(
        "[cmd] preview_contract_termination: player_id={}",
        player_id
    );

    let preview = state
        .get_game(|game| ofm_core::contracts::preview_contract_termination(game, player_id))
        .ok_or_else(|| "be.error.noActiveGameSession".to_string())??;

    Ok(ContractTerminationPreviewCommandResponse { preview })
}

pub fn terminate_contract_now_internal(
    state: &StateManager,
    player_id: &str,
) -> Result<ContractTerminationCommandResponse, String> {
    info!("[cmd] terminate_contract_now: player_id={}", player_id);

    // terminate_contract_now previews and checks squad safety before mutating.
    state
        .update_game(|game| {
            let ContractTerminationResult {
                severance_cost,
                squad_safety,
            } = ofm_core::contracts::terminate_contract_now(game, player_id)?;
            Ok(ContractTerminationCommandResponse {
                game: game.clone(),
                severance_cost,
                squad_safety,
            })
        })
        .unwrap_or_else(|| Err("be.error.noActiveGameSession".to_string()))
}

#[cfg(test)]
mod tests {
    use super::{
        clear_contract_exit_intent_internal, delegate_renewals_internal,
        offer_free_agent_contract_internal, preview_contract_termination_internal,
        preview_free_agent_contract_impact_internal, preview_renewal_financial_impact_internal,
        propose_renewal_internal, set_contract_exit_intent_internal,
        terminate_contract_now_internal,
    };
    use chrono::{TimeZone, Utc};
    use db::save_manager::SaveManager;
    use domain::manager::Manager;
    use domain::player::{
        Player, PlayerAttributes, PlayerMovementKind, Position, ContractTalksStatus,
    };
    use domain::season::TransferWindowStatus;
    use domain::staff::{Staff, StaffAttributes, StaffRole};
    use domain::team::Team;
    use ofm_core::clock::GameClock;
    use ofm_core::contracts::RenewalDecision;
    use ofm_core::game::Game;
    use ofm_core::state::StateManager;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempSaveDir {
        path: PathBuf,
    }

    impl TempSaveDir {
        fn new() -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!("ofm-contract-tests-{}", unique));
            fs::create_dir_all(&path).expect("temporary saves dir should be created");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempSaveDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

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

    fn make_player() -> Player {
        let mut player = Player::new(
            "player-1".to_string(),
            "J. Smith".to_string(),
            "John Smith".to_string(),
            "2000-01-01".to_string(),
            "England".to_string(),
            Position::Forward,
            default_attrs(),
        );
        player.team_id = Some("team-1".to_string());
        player.contract_end = Some("2026-10-15".to_string());
        player.wage = 12_000;
        player.morale = 75;
        player.market_value = 350_000;
        player
    }

    fn make_player_with_position(id: &str, position: Position) -> Player {
        let mut player = make_player();
        player.id = id.to_string();
        player.match_name = id.to_string();
        player.full_name = format!("Player {}", id);
        player.position = position.clone();
        player.natural_position = position;
        player
    }

    fn make_assistant_manager() -> Staff {
        let mut staff = Staff::new(
            "staff-1".to_string(),
            "Alex".to_string(),
            "Assistant".to_string(),
            "1985-01-01".to_string(),
            StaffRole::AssistantManager,
            StaffAttributes {
                coaching: 82,
                judging_ability: 76,
                judging_potential: 74,
                physiotherapy: 30,
            },
        );
        staff.team_id = Some("team-1".to_string());
        staff
    }

    fn make_team() -> Team {
        let mut team = Team::new(
            "team-1".to_string(),
            "Alpha FC".to_string(),
            "ALP".to_string(),
            "England".to_string(),
            "London".to_string(),
            "Alpha Ground".to_string(),
            30_000,
        );
        team.manager_id = Some("manager-1".to_string());
        team.reputation = 50;
        team.wage_budget = 50_000;
        team
    }

    fn make_game() -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "manager-1".to_string(),
            "Jane".to_string(),
            "Doe".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("team-1".to_string());

        Game::new(
            clock,
            manager,
            vec![make_team()],
            vec![make_player()],
            vec![make_assistant_manager()],
            vec![],
        )
    }

    fn make_squad_game() -> Game {
        let mut game = make_game();
        game.players = vec![
            make_player_with_position("gk-1", Position::Goalkeeper),
            make_player_with_position("player-1", Position::Forward),
            make_player_with_position("player-2", Position::Forward),
            make_player_with_position("player-3", Position::Defender),
            make_player_with_position("player-4", Position::Defender),
            make_player_with_position("player-5", Position::Defender),
            make_player_with_position("player-6", Position::Defender),
            make_player_with_position("player-7", Position::Midfielder),
            make_player_with_position("player-8", Position::Midfielder),
            make_player_with_position("player-9", Position::Midfielder),
            make_player_with_position("player-10", Position::Midfielder),
            make_player_with_position("player-11", Position::Forward),
        ];
        game
    }

    fn make_free_agent_game() -> Game {
        let mut game = make_game();
        let player = &mut game.players[0];
        player.team_id = None;
        player.contract_end = None;
        player.wage = 0;
        player.market_value = 600_000;
        game.season_context.transfer_window.status = TransferWindowStatus::Open;
        game
    }

    fn make_retired_free_agent_game() -> Game {
        let mut game = make_free_agent_game();
        game.players[0].retired = true;
        game
    }

    #[test]
    fn propose_renewal_internal_returns_response_and_updates_state() {
        let state = StateManager::new();
        state.set_game(make_game());

        let response = propose_renewal_internal(&state, "player-1", 15_000, 3).expect("response");

        assert!(matches!(response.outcome, RenewalDecision::Accepted));
        assert!(response.is_terminal);
        let player = response
            .game
            .players
            .iter()
            .find(|player| player.id == "player-1")
            .expect("player should exist");
        assert_eq!(player.wage, 15_000);
        assert_eq!(player.contract_end.as_deref(), Some("2029-08-01"));

        let stored_game = state.get_game(|game| game.clone()).expect("stored game");
        let stored_player = stored_game
            .players
            .iter()
            .find(|player| player.id == "player-1")
            .expect("stored player should exist");
        assert_eq!(stored_player.wage, 15_000);
        assert_eq!(stored_player.contract_end.as_deref(), Some("2029-08-01"));
    }

    #[test]
    fn delegate_renewals_internal_returns_report_and_updates_state() {
        let state = StateManager::new();
        state.set_game(make_game());

        let response =
            delegate_renewals_internal(&state, Some(vec!["player-1".to_string()]), 35, 3)
                .expect("response");

        assert_eq!(response.report.success_count, 1);
        assert_eq!(response.report.failure_count, 0);
        assert_eq!(response.report.stalled_count, 0);
        let player = response
            .game
            .players
            .iter()
            .find(|player| player.id == "player-1")
            .expect("player should exist");
        assert_eq!(player.contract_end.as_deref(), Some("2029-08-01"));
        assert!(response
            .game
            .messages
            .iter()
            .any(|message| message.id.starts_with("delegated_renewals_")));

        let stored_game = state.get_game(|game| game.clone()).expect("stored game");
        let stored_player = stored_game
            .players
            .iter()
            .find(|player| player.id == "player-1")
            .expect("stored player should exist");
        assert_eq!(stored_player.contract_end.as_deref(), Some("2029-08-01"));
    }

    #[test]
    fn renewal_changes_only_persist_after_explicit_save() {
        let temp_dir = TempSaveDir::new();
        let mut save_manager = SaveManager::init(temp_dir.path()).expect("save manager");
        let game = make_game();
        let save_id = save_manager
            .create_save(&game, "Renewal Persistence Test")
            .expect("save should be created");

        let state = StateManager::new();
        state.set_game(
            save_manager
                .load_game(&save_id)
                .expect("saved game should load"),
        );
        state.set_save_id(save_id.clone());

        let response = propose_renewal_internal(&state, "player-1", 15_000, 3).expect("response");
        assert!(matches!(response.outcome, RenewalDecision::Accepted));

        let persisted_before_manual_save = save_manager
            .load_game(&save_id)
            .expect("save should remain readable");
        let persisted_player = persisted_before_manual_save
            .players
            .iter()
            .find(|player| player.id == "player-1")
            .expect("persisted player should exist");
        assert_eq!(persisted_player.wage, 12_000);
        assert_eq!(persisted_player.contract_end.as_deref(), Some("2026-10-15"));

        let updated_game = state
            .get_game(|game| game.clone())
            .expect("updated game state");
        save_manager
            .save_game(&updated_game, &save_id)
            .expect("manual save should persist renewal");

        let persisted_after_manual_save = save_manager
            .load_game(&save_id)
            .expect("updated save should load");
        let saved_player = persisted_after_manual_save
            .players
            .iter()
            .find(|player| player.id == "player-1")
            .expect("saved player should exist");
        assert_eq!(saved_player.wage, 15_000);
        assert_eq!(saved_player.contract_end.as_deref(), Some("2029-08-01"));
    }

    #[test]
    fn delegated_renewal_changes_only_persist_after_explicit_save() {
        let temp_dir = TempSaveDir::new();
        let mut save_manager = SaveManager::init(temp_dir.path()).expect("save manager");
        let game = make_game();
        let save_id = save_manager
            .create_save(&game, "Delegated Renewal Persistence Test")
            .expect("save should be created");

        let state = StateManager::new();
        state.set_game(
            save_manager
                .load_game(&save_id)
                .expect("saved game should load"),
        );
        state.set_save_id(save_id.clone());

        let response =
            delegate_renewals_internal(&state, Some(vec!["player-1".to_string()]), 35, 3)
                .expect("delegated renewal should succeed");
        assert_eq!(response.report.success_count, 1);

        let persisted_before_manual_save = save_manager
            .load_game(&save_id)
            .expect("save should remain readable");
        let persisted_player = persisted_before_manual_save
            .players
            .iter()
            .find(|player| player.id == "player-1")
            .expect("persisted player should exist");
        assert_eq!(persisted_player.contract_end.as_deref(), Some("2026-10-15"));
        assert!(persisted_before_manual_save
            .messages
            .iter()
            .all(|message| !message.id.starts_with("delegated_renewals_")));

        let updated_game = state
            .get_game(|game| game.clone())
            .expect("updated game state");
        save_manager
            .save_game(&updated_game, &save_id)
            .expect("manual save should persist delegated renewal");

        let persisted_after_manual_save = save_manager
            .load_game(&save_id)
            .expect("updated save should load");
        let saved_player = persisted_after_manual_save
            .players
            .iter()
            .find(|player| player.id == "player-1")
            .expect("saved player should exist");
        assert_eq!(saved_player.contract_end.as_deref(), Some("2029-08-01"));
        assert!(persisted_after_manual_save
            .messages
            .iter()
            .any(|message| message.id.starts_with("delegated_renewals_")));
    }

    #[test]
    fn preview_renewal_financial_impact_internal_returns_projection() {
        let state = StateManager::new();
        state.set_game(make_game());

        let response = preview_renewal_financial_impact_internal(&state, "player-1", 15_000)
            .expect("response");

        assert_eq!(response.projection.annual_wage_budget, 50_000);
        assert_eq!(response.projection.current_annual_wage_bill, 12_000);
        assert_eq!(response.projection.projected_annual_wage_bill, 15_000);
        assert!(response.projection.policy_allows);
    }

    #[test]
    fn offer_free_agent_contract_internal_returns_response_and_updates_state() {
        let state = StateManager::new();
        state.set_game(make_free_agent_game());

        let response =
            offer_free_agent_contract_internal(&state, "player-1", 4_000, 3).expect("response");

        assert!(matches!(response.outcome, RenewalDecision::Accepted));
        assert_eq!(response.session_status, "agreed");
        assert_eq!(response.game.players[0].team_id.as_deref(), Some("team-1"));
        assert_eq!(
            response.game.players[0].contract_end.as_deref(),
            Some("2029-08-01")
        );
        assert!(response.game.players[0]
            .movement_history
            .iter()
            .any(|entry| {
                entry.kind == PlayerMovementKind::FreeAgentSigning
                    && entry.to_team_id.as_deref() == Some("team-1")
            }));

        let stored_game = state.get_game(|game| game.clone()).expect("stored game");
        assert_eq!(stored_game.players[0].team_id.as_deref(), Some("team-1"));
        assert!(stored_game.players[0].movement_history.iter().any(|entry| {
            entry.kind == PlayerMovementKind::FreeAgentSigning
                && entry.to_team_name.as_deref() == Some("Alpha FC")
        }));
    }

    #[test]
    fn preview_free_agent_contract_impact_internal_returns_projection() {
        let state = StateManager::new();
        state.set_game(make_free_agent_game());

        let response = preview_free_agent_contract_impact_internal(&state, "player-1", 4_000)
            .expect("response");

        assert_eq!(response.projection.current_annual_wage_bill, 0);
        assert_eq!(response.projection.projected_annual_wage_bill, 4_000);
        assert!(response.projection.policy_allows);
    }

    #[test]
    fn preview_free_agent_contract_impact_internal_rejects_retired_players() {
        let state = StateManager::new();
        state.set_game(make_retired_free_agent_game());

        let error = preview_free_agent_contract_impact_internal(&state, "player-1", 4_000)
            .expect_err("retired players should be rejected");

        assert_eq!(error, "be.error.contracts.playerNotFreeAgent");
    }

    #[test]
    fn offer_free_agent_contract_internal_rejects_retired_players() {
        let state = StateManager::new();
        state.set_game(make_retired_free_agent_game());

        let error = offer_free_agent_contract_internal(&state, "player-1", 4_000, 3)
            .expect_err("retired players should be rejected");

        assert_eq!(error, "be.error.contracts.playerNotFreeAgent");
    }

    #[test]
    fn serialize_session_status_uses_frontend_casing() {
        assert_eq!(
            super::serialize_session_status(ContractTalksStatus::Idle),
            "idle"
        );
        assert_eq!(
            super::serialize_session_status(ContractTalksStatus::Open),
            "open"
        );
        assert_eq!(
            super::serialize_session_status(ContractTalksStatus::Agreed),
            "agreed"
        );
        assert_eq!(
            super::serialize_session_status(ContractTalksStatus::Blocked),
            "blocked"
        );
        assert_eq!(
            super::serialize_session_status(ContractTalksStatus::Stalled),
            "stalled"
        );
    }

    #[test]
    fn contract_exit_intent_internal_updates_state() {
        let state = StateManager::new();
        state.set_game(make_game());

        let marked =
            set_contract_exit_intent_internal(&state, "player-1", None).expect("intent response");
        let marked_player = marked
            .game
            .players
            .iter()
            .find(|player| player.id == "player-1")
            .expect("player should exist");
        assert!(marked_player
            .morale_core
            .renewal_state
            .as_ref()
            .and_then(|state| state.exit_intent.as_ref())
            .is_some());

        let cleared =
            clear_contract_exit_intent_internal(&state, "player-1").expect("clear response");
        let cleared_player = cleared
            .game
            .players
            .iter()
            .find(|player| player.id == "player-1")
            .expect("player should exist");
        assert!(cleared_player
            .morale_core
            .renewal_state
            .as_ref()
            .and_then(|state| state.exit_intent.as_ref())
            .is_none());
    }

    #[test]
    fn terminate_contract_now_internal_returns_updated_game() {
        let state = StateManager::new();
        state.set_game(make_squad_game());

        let preview =
            preview_contract_termination_internal(&state, "player-1").expect("preview response");
        assert!(preview.preview.squad_safety.can_field_matchday_squad);

        let response =
            terminate_contract_now_internal(&state, "player-1").expect("termination response");
        assert_eq!(response.severance_cost, 132_000);
        assert!(response.squad_safety.can_field_matchday_squad);
        let player = response
            .game
            .players
            .iter()
            .find(|player| player.id == "player-1")
            .expect("player should exist");
        assert_eq!(player.team_id, None);
        assert!(player.movement_history.iter().any(|entry| {
            entry.kind == PlayerMovementKind::Released
                && entry.from_team_id.as_deref() == Some("team-1")
        }));

        let stored_game = state.get_game(|game| game.clone()).expect("stored game");
        let stored_player = stored_game
            .players
            .iter()
            .find(|player| player.id == "player-1")
            .expect("stored player should exist");
        assert_eq!(stored_player.team_id, None);
        assert!(stored_player.movement_history.iter().any(|entry| {
            entry.kind == PlayerMovementKind::Released
                && entry.from_team_name.as_deref() == Some("Alpha FC")
        }));
    }
}
