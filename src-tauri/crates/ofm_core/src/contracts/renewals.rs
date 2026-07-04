//! Contract renewals: evaluating and proposing new terms to the manager's own
//! players, the AI-delegated bulk renewal path, and the negotiation-state helpers
//! (rounds, cooling, manager blocks, relationship gating, feedback) behind them.
//! Byte-faithful extraction from the original contracts.rs.

use super::*;

pub(crate) fn renewal_outcome(
    decision: RenewalDecision,
    suggested_wage: Option<u32>,
    suggested_years: Option<u32>,
    session_status: RenewalSessionStatus,
    is_terminal: bool,
    cooled_off: bool,
    feedback: Option<NegotiationFeedback>,
) -> RenewalOutcome {
    RenewalOutcome {
        decision,
        suggested_wage,
        suggested_years,
        session_status,
        is_terminal,
        cooled_off,
        feedback,
    }
}

pub fn project_renewal_financial_impact(
    game: &Game,
    player_id: &str,
    offered_wage: u32,
) -> Result<RenewalFinancialProjection, String> {
    owned_player(game, player_id)?;
    project_renewal_financial_impact_service(game, player_id, offered_wage)
}

pub fn evaluate_renewal_offer(
    player: &Player,
    team: &Team,
    current_date: NaiveDate,
    offer: &RenewalOffer,
) -> RenewalOutcome {
    let round = next_renewal_round(player, None);
    let expected_wage = expected_wage(player, team, current_date);
    let expected_years = expected_contract_years(player, current_date);
    let minimum_wage = minimum_acceptable_wage(player.wage);

    if offer.contract_years == 0 || offer.contract_years > MAX_CONTRACT_YEARS {
        let feedback = build_renewal_feedback(
            player,
            current_date,
            RenewalDecision::Rejected,
            RenewalSessionStatus::Stalled,
            round,
            expected_wage,
            false,
        );
        return renewal_outcome(
            RenewalDecision::Rejected,
            None,
            None,
            RenewalSessionStatus::Stalled,
            false,
            false,
            Some(feedback),
        );
    }

    if is_insulting_wage_offer(player.wage, expected_wage, offer.weekly_wage) {
        let feedback = build_renewal_feedback(
            player,
            current_date,
            RenewalDecision::Rejected,
            RenewalSessionStatus::Blocked,
            round,
            expected_wage,
            false,
        );
        return renewal_outcome(
            RenewalDecision::Rejected,
            None,
            None,
            RenewalSessionStatus::Blocked,
            true,
            false,
            Some(feedback),
        );
    }

    if offer.weekly_wage < minimum_wage {
        let feedback = build_renewal_feedback(
            player,
            current_date,
            RenewalDecision::Rejected,
            RenewalSessionStatus::Stalled,
            round,
            expected_wage,
            false,
        );
        return renewal_outcome(
            RenewalDecision::Rejected,
            None,
            None,
            RenewalSessionStatus::Stalled,
            false,
            false,
            Some(feedback),
        );
    }

    if offer.weekly_wage >= expected_wage && offer.contract_years >= expected_years {
        let feedback = build_renewal_feedback(
            player,
            current_date,
            RenewalDecision::Accepted,
            RenewalSessionStatus::Agreed,
            round,
            expected_wage,
            false,
        );
        return renewal_outcome(
            RenewalDecision::Accepted,
            None,
            None,
            RenewalSessionStatus::Agreed,
            true,
            false,
            Some(feedback),
        );
    }

    let feedback = build_renewal_feedback(
        player,
        current_date,
        RenewalDecision::CounterOffer,
        RenewalSessionStatus::Open,
        round,
        expected_wage,
        false,
    );

    renewal_outcome(
        RenewalDecision::CounterOffer,
        Some(expected_wage),
        Some(expected_years),
        RenewalSessionStatus::Open,
        false,
        false,
        Some(feedback),
    )
}

pub fn propose_renewal(
    game: &mut Game,
    player_id: &str,
    offer: RenewalOffer,
) -> Result<RenewalOutcome, String> {
    let manager_team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("be.error.noTeamAssigned".to_string())?;

    let team = game
        .teams
        .iter()
        .find(|candidate| candidate.id == manager_team_id)
        .ok_or("be.error.managedTeamNotFound".to_string())?
        .clone();

    let player_index = game
        .players
        .iter()
        .position(|candidate| candidate.id == player_id)
        .ok_or("be.error.playerNotFound".to_string())?;

    if contract_owner_team_id(&game.players[player_index]) != Some(team.id.as_str()) {
        return Err(ERR_PLAYER_NOT_OWNED_BY_CLUB.to_string());
    }

    if offer.contract_years == 0 || offer.contract_years > MAX_CONTRACT_YEARS {
        let current_date = game.clock.current_date.date_naive();
        let round = next_renewal_round(&game.players[player_index], None);
        let expected_wage = expected_wage(&game.players[player_index], &team, current_date);
        return Ok(renewal_outcome(
            RenewalDecision::Rejected,
            None,
            None,
            RenewalSessionStatus::Stalled,
            false,
            false,
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

    let current_date = game.clock.current_date.date_naive();
    let cooled_off = cool_stale_renewal_session(&mut game.players[player_index], current_date);
    let today = current_date.format("%Y-%m-%d").to_string();
    let round = next_renewal_round(&game.players[player_index], Some(today.as_str()));

    if has_active_manager_block(&game.players[player_index], current_date) {
        return Ok(renewal_outcome(
            RenewalDecision::Rejected,
            None,
            None,
            RenewalSessionStatus::Blocked,
            true,
            cooled_off,
            Some(build_renewal_feedback(
                &game.players[player_index],
                current_date,
                RenewalDecision::Rejected,
                RenewalSessionStatus::Blocked,
                round,
                0,
                false,
            )),
        ));
    }

    if let Some(state) = game.players[player_index]
        .morale_core
        .renewal_state
        .as_ref()
        && state.status == RenewalSessionStatus::Agreed
        && state.last_attempt_date.as_deref() == Some(today.as_str())
    {
        return Ok(renewal_outcome(
            RenewalDecision::Rejected,
            None,
            None,
            RenewalSessionStatus::Agreed,
            true,
            cooled_off,
            Some(build_renewal_feedback(
                &game.players[player_index],
                current_date,
                RenewalDecision::Accepted,
                RenewalSessionStatus::Agreed,
                round,
                game.players[player_index].wage,
                false,
            )),
        ));
    }

    let expected_wage = expected_wage(&game.players[player_index], &team, current_date);
    let mut outcome =
        evaluate_renewal_offer(&game.players[player_index], &team, current_date, &offer);
    outcome.cooled_off = cooled_off;
    let relationship_blocked = outcome.session_status != RenewalSessionStatus::Blocked
        && should_manual_renewal_fail_on_relationship(
            &game.players[player_index],
            expected_wage,
            offer.weekly_wage,
        );

    if relationship_blocked {
        outcome = renewal_outcome(
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
                true,
            )),
        );
    }

    if outcome.decision == RenewalDecision::Accepted {
        if !renewal_wage_policy_allows(
            game,
            &team,
            game.players[player_index].wage,
            offer.weekly_wage,
        ) {
            return Err(renewal_wage_policy_error_message(&team));
        }

        let new_contract_end = current_date
            .checked_add_months(Months::new(offer.contract_years * 12))
            .ok_or(ERR_UNABLE_TO_CALCULATE_CONTRACT_END_DATE.to_string())?;

        let player = &mut game.players[player_index];
        player.wage = offer.weekly_wage;
        player.contract_end = Some(new_contract_end.format("%Y-%m-%d").to_string());
        let state = player
            .morale_core
            .renewal_state
            .get_or_insert_with(ContractRenewalState::default);
        state.status = RenewalSessionStatus::Agreed;
        state.manager_blocked_until = None;
        state.last_attempt_date = Some(today);
        state.last_outcome = Some(RenewalSessionOutcome::AcceptedByManager);
        state.conversation_round = round;
        state.exit_intent = None;
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

    match outcome.decision {
        RenewalDecision::Rejected => {
            state.status = outcome.session_status.clone();
            if outcome.session_status == RenewalSessionStatus::Blocked {
                state.manager_blocked_until = renewal_blocked_until(current_date);
                state.last_outcome = Some(RenewalSessionOutcome::BlockedByManager);
            } else {
                state.manager_blocked_until = None;
                state.last_outcome = Some(RenewalSessionOutcome::RejectedByPlayer);
            }
        }
        RenewalDecision::CounterOffer => {
            state.status = RenewalSessionStatus::Open;
            state.manager_blocked_until = None;
            state.last_outcome = Some(RenewalSessionOutcome::Stalled);
        }
        RenewalDecision::Accepted => {}
    }

    if outcome.feedback.is_none() {
        outcome.feedback = Some(build_renewal_feedback(
            player,
            current_date,
            outcome.decision.clone(),
            outcome.session_status.clone(),
            round,
            expected_wage,
            relationship_blocked,
        ));
    }

    Ok(outcome)
}

pub fn delegate_renewals(
    game: &mut Game,
    options: DelegatedRenewalOptions,
) -> Result<DelegatedRenewalReport, String> {
    delegate_renewals_service(game, options)
}

pub(crate) fn renewal_blocked_until(current_date: NaiveDate) -> Option<String> {
    current_date
        .checked_add_days(Days::new(INSULTING_RENEWAL_BLOCK_DAYS))
        .map(|date| date.format("%Y-%m-%d").to_string())
}

pub(crate) fn next_renewal_round(player: &Player, today: Option<&str>) -> u8 {
    let Some(state) = player.morale_core.renewal_state.as_ref() else {
        return 1;
    };

    if let Some(today) = today
        && state.last_attempt_date.as_deref() != Some(today)
    {
        return 1;
    }

    state.conversation_round.saturating_add(1).max(1)
}

pub(crate) fn cool_stale_renewal_session(player: &mut Player, current_date: NaiveDate) -> bool {
    let Some(state) = player.morale_core.renewal_state.as_mut() else {
        return false;
    };

    if matches!(
        state.status,
        RenewalSessionStatus::Blocked | RenewalSessionStatus::Agreed | RenewalSessionStatus::Idle
    ) {
        return false;
    }

    let Some(last_attempt_date) = state.last_attempt_date.as_deref() else {
        return false;
    };

    let Ok(last_attempt) = NaiveDate::parse_from_str(last_attempt_date, "%Y-%m-%d") else {
        return false;
    };

    if (current_date - last_attempt).num_days() < RENEWAL_SESSION_STALE_DAYS {
        return false;
    }

    state.status = RenewalSessionStatus::Idle;
    state.last_outcome = None;
    state.conversation_round = 0;
    true
}

pub(crate) fn build_renewal_feedback(
    player: &Player,
    current_date: NaiveDate,
    decision: RenewalDecision,
    session_status: RenewalSessionStatus,
    round: u8,
    expected_wage: u32,
    relationship_blocked: bool,
) -> NegotiationFeedback {
    let trust = player.morale_core.manager_trust;
    let remaining_days = remaining_contract_days(player, current_date);
    let urgency_pressure = if remaining_days <= 90 {
        24
    } else if remaining_days <= 180 {
        16
    } else if remaining_days <= 365 {
        8
    } else {
        2
    };
    let morale_pressure = if player.morale <= 40 {
        24
    } else if player.morale <= 60 {
        12
    } else {
        0
    };
    let trust_pressure = if trust <= 25 {
        26
    } else if trust <= 40 {
        12
    } else {
        0
    };
    let value_pressure = if player.market_value >= 2_000_000 {
        12
    } else if player.market_value >= 750_000 {
        6
    } else {
        0
    };
    let tension = (22 + urgency_pressure + morale_pressure + trust_pressure + value_pressure)
        .clamp(10, 92) as u8;
    let patience = (100_i32 - i32::from(round.saturating_sub(1)) * 18 - i32::from(tension) / 3)
        .clamp(18, 92) as u8;

    let (mood, headline_key, detail_key) = if session_status == RenewalSessionStatus::Blocked {
        (
            NegotiationMood::Guarded,
            "playerProfile.renewalFeedbackBlockedHeadline",
            Some("playerProfile.renewalFeedbackBlockedDetail"),
        )
    } else if decision == RenewalDecision::Accepted && round >= 2 {
        (
            NegotiationMood::Positive,
            "playerProfile.renewalFeedbackAcceptedLateHeadline",
            Some("playerProfile.renewalFeedbackAcceptedLateDetail"),
        )
    } else if decision == RenewalDecision::Accepted {
        (
            NegotiationMood::Positive,
            "playerProfile.renewalFeedbackAcceptedHeadline",
            Some("playerProfile.renewalFeedbackAcceptedDetail"),
        )
    } else if relationship_blocked || tension >= 70 {
        (
            NegotiationMood::Tense,
            "playerProfile.renewalFeedbackTenseHeadline",
            Some("playerProfile.renewalFeedbackTenseDetail"),
        )
    } else if expected_wage > player.wage || round >= 2 {
        (
            NegotiationMood::Firm,
            "playerProfile.renewalFeedbackFirmHeadline",
            Some("playerProfile.renewalFeedbackFirmDetail"),
        )
    } else {
        (
            NegotiationMood::Calm,
            "playerProfile.renewalFeedbackCalmHeadline",
            Some("playerProfile.renewalFeedbackCalmDetail"),
        )
    };

    NegotiationFeedback {
        mood,
        headline_key: headline_key.to_string(),
        detail_key: detail_key.map(str::to_string),
        tension,
        patience,
        round,
        params: HashMap::new(),
    }
}

pub(crate) fn should_manual_renewal_fail_on_relationship(
    player: &Player,
    expected_wage: u32,
    offered_wage: u32,
) -> bool {
    let trust = player.morale_core.manager_trust;
    let relationship_margin = if trust <= 20 {
        2_000
    } else if trust <= 30 {
        1_000
    } else {
        0
    };

    relationship_margin > 0 && offered_wage < expected_wage.saturating_add(relationship_margin)
}

pub(crate) fn has_active_manager_block(player: &Player, current_date: NaiveDate) -> bool {
    let Some(state) = player.morale_core.renewal_state.as_ref() else {
        return false;
    };

    if state.status != RenewalSessionStatus::Blocked {
        return false;
    }

    let Some(blocked_until) = state.manager_blocked_until.as_deref() else {
        return true;
    };

    NaiveDate::parse_from_str(blocked_until, "%Y-%m-%d")
        .map(|blocked_until| blocked_until >= current_date)
        .unwrap_or(true)
}
