//! Signing free agents: the affordability projection, the signing flow that puts
//! a contract-less player on the manager's books, and the announcement message.
//! Byte-faithful extraction from the original contracts.rs.

use super::*;

pub fn project_free_agent_contract_impact(
    game: &Game,
    player_id: &str,
    offered_wage: u32,
) -> Result<RenewalFinancialProjection, String> {
    let manager_team_id = game
        .manager
        .team_id
        .clone()
        .ok_or(ERR_NO_TEAM_ASSIGNED.to_string())?;
    let team = game
        .teams
        .iter()
        .find(|candidate| candidate.id == manager_team_id)
        .ok_or(ERR_MANAGED_TEAM_NOT_FOUND.to_string())?;
    let player = game
        .players
        .iter()
        .find(|candidate| candidate.id == player_id)
        .ok_or(ERR_PLAYER_NOT_FOUND.to_string())?;

    if player.retired || player.team_id.is_some() {
        return Err(ERR_PLAYER_NOT_FREE_AGENT.to_string());
    }

    Ok(project_contract_offer_financial_impact(
        game,
        team,
        0,
        offered_wage,
    ))
}

pub fn offer_free_agent_contract(
    game: &mut Game,
    player_id: &str,
    offer: RenewalOffer,
) -> Result<RenewalOutcome, String> {
    let manager_team_id = game
        .manager
        .team_id
        .clone()
        .ok_or(ERR_NO_TEAM_ASSIGNED.to_string())?;

    let team = game
        .teams
        .iter()
        .find(|candidate| candidate.id == manager_team_id)
        .ok_or(ERR_MANAGED_TEAM_NOT_FOUND.to_string())?
        .clone();

    let player_index = game
        .players
        .iter()
        .position(|candidate| candidate.id == player_id)
        .ok_or(ERR_PLAYER_NOT_FOUND.to_string())?;

    if game.players[player_index].retired || game.players[player_index].team_id.is_some() {
        return Err(ERR_PLAYER_NOT_FREE_AGENT.to_string());
    }

    let current_date = game.clock.current_date.date_naive();
    let cooled_off = cool_stale_renewal_session(&mut game.players[player_index], current_date);
    let today = current_date.format("%Y-%m-%d").to_string();
    let round = next_renewal_round(&game.players[player_index], Some(today.as_str()));
    let expected_wage = expected_wage(&game.players[player_index], &team, current_date);
    let expected_years = expected_contract_years(&game.players[player_index], current_date);
    let reference_wage = reference_player_wage(&game.players[player_index]);
    let minimum_wage = minimum_acceptable_wage(reference_wage);

    if offer.contract_years == 0 || offer.contract_years > MAX_CONTRACT_YEARS {
        return Ok(renewal_outcome(
            RenewalDecision::Rejected,
            None,
            None,
            RenewalSessionStatus::Stalled,
            false,
            cooled_off,
            Some(build_renewal_feedback(
                &game.players[player_index],
                current_date,
                RenewalDecision::Rejected,
                RenewalSessionStatus::Stalled,
                round,
                expected_wage,
                false,
            )),
        ));
    }

    if is_insulting_wage_offer(reference_wage, expected_wage, offer.weekly_wage) {
        let blocked_until = renewal_blocked_until(current_date);
        let player = &mut game.players[player_index];
        let state = player
            .morale_core
            .renewal_state
            .get_or_insert_with(ContractRenewalState::default);
        state.last_attempt_date = Some(today.clone());
        state.conversation_round = round;
        state.status = RenewalSessionStatus::Blocked;
        state.manager_blocked_until = blocked_until;
        state.last_outcome = Some(RenewalSessionOutcome::BlockedByManager);

        return Ok(renewal_outcome(
            RenewalDecision::Rejected,
            None,
            None,
            RenewalSessionStatus::Blocked,
            true,
            cooled_off,
            Some(build_renewal_feedback(
                player,
                current_date,
                RenewalDecision::Rejected,
                RenewalSessionStatus::Blocked,
                round,
                expected_wage,
                false,
            )),
        ));
    }

    if offer.weekly_wage < minimum_wage {
        return Ok(renewal_outcome(
            RenewalDecision::Rejected,
            None,
            None,
            RenewalSessionStatus::Stalled,
            false,
            cooled_off,
            Some(build_renewal_feedback(
                &game.players[player_index],
                current_date,
                RenewalDecision::Rejected,
                RenewalSessionStatus::Stalled,
                round,
                expected_wage,
                false,
            )),
        ));
    }

    if offer.weekly_wage >= expected_wage && offer.contract_years >= expected_years {
        if !renewal_wage_policy_allows(game, &team, 0, offer.weekly_wage) {
            return Err(renewal_wage_policy_error_message(&team));
        }

        let new_contract_end = current_date
            .checked_add_months(Months::new(offer.contract_years * 12))
            .ok_or(ERR_UNABLE_TO_CALCULATE_CONTRACT_END_DATE.to_string())?;

        let resolved_jersey_number =
            crate::roster::resolve_jersey_for(game, &game.players[player_index], &team);

        let player = &mut game.players[player_index];
        player.team_id = Some(team.id.clone());
        player.jersey_number = resolved_jersey_number;
        player.wage = offer.weekly_wage;
        player.contract_end = Some(new_contract_end.format("%Y-%m-%d").to_string());
        player.transfer_listed = false;
        player.loan_listed = false;
        player.transfer_offers.clear();
        player.movement_history.push(PlayerMovementEntry {
            date: today.clone(),
            kind: PlayerMovementKind::FreeAgentSigning,
            from_team_id: None,
            from_team_name: None,
            to_team_id: Some(team.id.clone()),
            to_team_name: Some(team.name.clone()),
            fee: None,
            loan_end_date: None,
        });
        if matches!(
            player
                .morale_core
                .unresolved_issue
                .as_ref()
                .map(|issue| &issue.category),
            Some(domain::player::PlayerIssueCategory::Contract)
        ) {
            player.morale_core.unresolved_issue = None;
        }
        player.morale_core.renewal_state = None;
        player.morale = (i16::from(player.morale) + 6).clamp(0, 100) as u8;
        player.morale_core.manager_trust = player.morale_core.manager_trust.max(55);

        game.messages.push(free_agent_signed_message(
            &player.id,
            &player.full_name,
            &team.name,
            offer.contract_years,
            &today,
        ));

        return Ok(renewal_outcome(
            RenewalDecision::Accepted,
            None,
            None,
            RenewalSessionStatus::Agreed,
            true,
            cooled_off,
            Some(build_renewal_feedback(
                player,
                current_date,
                RenewalDecision::Accepted,
                RenewalSessionStatus::Agreed,
                round,
                expected_wage,
                false,
            )),
        ));
    }

    let player = &mut game.players[player_index];
    let state = player
        .morale_core
        .renewal_state
        .get_or_insert_with(ContractRenewalState::default);
    state.last_attempt_date = Some(today);
    state.conversation_round = round;
    state.status = RenewalSessionStatus::Open;
    state.manager_blocked_until = None;
    state.last_outcome = Some(RenewalSessionOutcome::Stalled);

    Ok(renewal_outcome(
        RenewalDecision::CounterOffer,
        Some(expected_wage),
        Some(expected_years),
        RenewalSessionStatus::Open,
        false,
        cooled_off,
        Some(build_renewal_feedback(
            player,
            current_date,
            RenewalDecision::CounterOffer,
            RenewalSessionStatus::Open,
            round,
            expected_wage,
            false,
        )),
    ))
}

pub(crate) fn free_agent_signed_message(
    player_id: &str,
    player_name: &str,
    team_name: &str,
    contract_years: u32,
    date: &str,
) -> InboxMessage {
    let mut i18n_params = std::collections::HashMap::new();
    i18n_params.insert("player".to_string(), player_name.to_string());
    i18n_params.insert("team".to_string(), team_name.to_string());
    i18n_params.insert("years".to_string(), contract_years.to_string());

    InboxMessage::new(
        format!("free_agent_signed_{}", player_id),
        String::new(),
        String::new(),
        String::new(),
        date.to_string(),
    )
    .with_category(MessageCategory::Contract)
    .with_priority(MessagePriority::Urgent)
    .with_sender_role("")
    .with_i18n(
        "be.msg.freeAgentSigned.subject",
        "be.msg.freeAgentSigned.body",
        i18n_params,
    )
    .with_sender_i18n("be.sender.assistantManager", "be.role.assistantManager")
}
