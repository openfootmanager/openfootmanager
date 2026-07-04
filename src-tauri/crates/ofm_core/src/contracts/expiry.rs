//! Contract expiry: the daily sweep that warns on approaching expiries and
//! releases players whose deals have run out, plus the shared release routine
//! that both expiry and manager-termination funnel through. Byte-faithful
//! extraction from the original contracts.rs.

use super::*;

pub fn contract_warning_stage(
    contract_end: Option<&str>,
    current_date: NaiveDate,
) -> Option<ContractWarningStage> {
    let days_remaining = contract_days_remaining(contract_end, current_date)?;

    if days_remaining <= 0 {
        return None;
    }

    if days_remaining <= 30 {
        return Some(ContractWarningStage::FinalWeeks);
    }

    if days_remaining <= 90 {
        return Some(ContractWarningStage::ThreeMonths);
    }

    if days_remaining <= 180 {
        return Some(ContractWarningStage::SixMonths);
    }

    if days_remaining <= 365 {
        return Some(ContractWarningStage::TwelveMonths);
    }

    None
}

pub fn process_contract_expiries(game: &mut Game) {
    let current_date = game.clock.current_date.date_naive();

    let expired_player_indices: Vec<usize> = game
        .players
        .iter()
        .enumerate()
        .filter_map(|(index, player)| {
            let days_remaining =
                contract_days_remaining(player.contract_end.as_deref(), current_date)?;
            if player.team_id.is_some() && days_remaining <= 0 {
                Some(index)
            } else {
                None
            }
        })
        .collect();

    for player_index in expired_player_indices {
        release_player_contract(game, player_index, ContractReleaseReason::Expired);
    }
}

pub(crate) fn remove_player_from_team_references(team: &mut Team, player_id: &str) {
    team.remove_player_references(player_id);
}

pub(crate) fn release_player_contract(
    game: &mut Game,
    player_index: usize,
    reason: ContractReleaseReason,
) {
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let player_id = game.players[player_index].id.clone();
    let player_name = game.players[player_index].match_name.clone();
    let team_id = game.players[player_index]
        .active_loan
        .as_ref()
        .map(|loan| loan.parent_team_id.clone())
        .or_else(|| game.players[player_index].team_id.clone());

    let Some(team_id) = team_id.as_deref() else {
        return;
    };
    let Some(team_name) = game
        .teams
        .iter()
        .find(|candidate| candidate.id == team_id)
        .map(|team| team.name.clone())
    else {
        return;
    };

    for team in &mut game.teams {
        remove_player_from_team_references(team, &player_id);
    }

    let player = &mut game.players[player_index];
    player.team_id = None;
    player.active_loan = None;
    player.contract_end = None;
    player.wage = 0;
    player.transfer_listed = false;
    player.loan_listed = false;
    player.transfer_offers.clear();
    player.loan_offers.clear();
    player.morale_core.renewal_state = None;
    player.movement_history.push(PlayerMovementEntry {
        date: today.clone(),
        kind: PlayerMovementKind::Released,
        from_team_id: Some(team_id.to_string()),
        from_team_name: Some(team_name.clone()),
        to_team_id: None,
        to_team_name: None,
        fee: None,
        loan_end_date: None,
    });

    let message = match reason {
        ContractReleaseReason::Expired => {
            contract_expired_message(&player_id, &player_name, &team_name, &today)
        }
        ContractReleaseReason::ManagerTermination { severance_cost } => {
            contract_terminated_message(
                &player_id,
                &player_name,
                &team_name,
                severance_cost,
                &today,
            )
        }
    };

    game.messages.push(message);
}

pub(crate) fn contract_expired_message(
    player_id: &str,
    player_name: &str,
    team_name: &str,
    date: &str,
) -> InboxMessage {
    let mut i18n_params = std::collections::HashMap::new();
    i18n_params.insert("player".to_string(), player_name.to_string());
    i18n_params.insert("team".to_string(), team_name.to_string());

    InboxMessage::new(
        format!("contract_expired_{}", player_id),
        String::new(),
        String::new(),
        String::new(),
        date.to_string(),
    )
    .with_category(MessageCategory::Contract)
    .with_priority(MessagePriority::Urgent)
    .with_sender_role("")
    .with_i18n(
        "be.msg.contractExpired.subject",
        "be.msg.contractExpired.body",
        i18n_params,
    )
    .with_sender_i18n("be.sender.assistantManager", "be.role.assistantManager")
}
