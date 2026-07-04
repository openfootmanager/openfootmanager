//! Manager-initiated contract termination and the let-expire exit-intent flags:
//! previewing severance + squad-safety, executing an early release, and the
//! supporting intent setters/queries. Byte-faithful extraction from the original
//! contracts.rs.

use super::*;

pub fn set_contract_exit_intent(
    game: &mut Game,
    player_id: &str,
    reason: Option<String>,
) -> Result<(), String> {
    let player_index = owned_player_index(game, player_id)?;
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let player = &mut game.players[player_index];

    if player.contract_end.is_none() {
        return Err(ERR_PLAYER_HAS_NO_ACTIVE_CONTRACT.to_string());
    }

    let state = player
        .morale_core
        .renewal_state
        .get_or_insert_with(ContractRenewalState::default);
    state.status = RenewalSessionStatus::Blocked;
    state.manager_blocked_until = None;
    state.last_attempt_date = Some(today.clone());
    state.last_outcome = Some(RenewalSessionOutcome::BlockedByManager);
    state.conversation_round = 0;
    state.exit_intent = Some(ContractExitIntent::LetExpire {
        set_on: today,
        reason,
    });

    Ok(())
}

pub fn clear_contract_exit_intent(game: &mut Game, player_id: &str) -> Result<(), String> {
    let player_index = owned_player_index(game, player_id)?;
    let Some(state) = game.players[player_index]
        .morale_core
        .renewal_state
        .as_mut()
    else {
        return Ok(());
    };
    let had_exit_intent = state.exit_intent.is_some();

    state.exit_intent = None;

    if had_exit_intent && state.status == RenewalSessionStatus::Blocked {
        state.status = RenewalSessionStatus::Idle;
        state.manager_blocked_until = None;
        state.last_outcome = None;
        state.conversation_round = 0;
    }

    Ok(())
}

pub fn preview_contract_termination(
    game: &Game,
    player_id: &str,
) -> Result<ContractTerminationPreview, String> {
    let player = owned_player(game, player_id)?;

    if player.contract_end.is_none() {
        return Err(ERR_PLAYER_HAS_NO_ACTIVE_CONTRACT.to_string());
    }

    if player.active_loan.is_some() {
        return Err(ERR_PLAYER_ON_ACTIVE_LOAN.to_string());
    }

    let current_date = game.clock.current_date.date_naive();
    Ok(ContractTerminationPreview {
        player_id: player.id.clone(),
        player_name: player.match_name.clone(),
        severance_cost: termination_severance_cost(player, current_date),
        squad_safety: project_user_team_release_safety(game, player_id)?,
    })
}

pub fn terminate_contract_now(
    game: &mut Game,
    player_id: &str,
) -> Result<ContractTerminationResult, String> {
    let preview = preview_contract_termination(game, player_id)?;

    if !preview.squad_safety.can_field_matchday_squad {
        return Err(ERR_TERMINATION_WOULD_LEAVE_MATCHDAY_SQUAD_SHORT.to_string());
    }

    let player_index = owned_player_index(game, player_id)?;
    let team_id = contract_owner_team_id(&game.players[player_index])
        .ok_or(ERR_PLAYER_NOT_OWNED_BY_CLUB.to_string())?
        .to_string();
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();

    if let Some(team) = game
        .teams
        .iter_mut()
        .find(|candidate| candidate.id == team_id)
    {
        team.finance -= preview.severance_cost;
        team.season_expenses += preview.severance_cost;
        team.financial_ledger.push(FinancialTransaction {
            date: today,
            description: backend_text_with_param(
                "be.msg.contractTerminated.ledgerDescription",
                "player",
                &preview.player_name,
            ),
            amount: -preview.severance_cost,
            kind: FinancialTransactionKind::ContractTermination,
        });
    }

    release_player_contract(
        game,
        player_index,
        ContractReleaseReason::ManagerTermination {
            severance_cost: preview.severance_cost,
        },
    );

    Ok(ContractTerminationResult {
        severance_cost: preview.severance_cost,
        squad_safety: preview.squad_safety,
    })
}

pub fn has_let_expire_intent(player: &Player) -> bool {
    player
        .morale_core
        .renewal_state
        .as_ref()
        .and_then(|state| state.exit_intent.as_ref())
        .is_some_and(|intent| matches!(intent, ContractExitIntent::LetExpire { .. }))
}

pub(crate) fn termination_severance_cost(player: &Player, current_date: NaiveDate) -> i64 {
    let remaining_days = contract_days_remaining(player.contract_end.as_deref(), current_date)
        .unwrap_or(0)
        .max(0);
    let remaining_weeks = (remaining_days + 6) / 7;

    remaining_weeks * i64::from(player.wage)
}

pub(crate) fn contract_terminated_message(
    player_id: &str,
    player_name: &str,
    team_name: &str,
    severance_cost: i64,
    date: &str,
) -> InboxMessage {
    let mut i18n_params = std::collections::HashMap::new();
    i18n_params.insert("player".to_string(), player_name.to_string());
    i18n_params.insert("team".to_string(), team_name.to_string());
    i18n_params.insert("severance".to_string(), severance_cost.to_string());

    InboxMessage::new(
        format!("contract_terminated_{}", player_id),
        String::new(),
        String::new(),
        String::new(),
        date.to_string(),
    )
    .with_category(MessageCategory::Contract)
    .with_priority(MessagePriority::Urgent)
    .with_sender_role("")
    .with_i18n(
        "be.msg.contractTerminated.subject",
        "be.msg.contractTerminated.body",
        i18n_params,
    )
    .with_sender_i18n("be.sender.assistantManager", "be.role.assistantManager")
}
