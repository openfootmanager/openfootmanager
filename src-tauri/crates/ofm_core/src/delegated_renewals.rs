use crate::contract_wage_policy::renewal_wage_policy_allows;
use crate::contracts::{
    ContractWarningStage, DelegatedRenewalCase, DelegatedRenewalOptions, DelegatedRenewalReport,
    DelegatedRenewalResultStatus, contract_warning_stage, expected_contract_years, expected_wage,
    has_active_manager_block, has_let_expire_intent, round_up_to_nearest_thousand,
};
use crate::game::Game;
use chrono::{Months, NaiveDate};
use domain::message::{
    DelegatedRenewalCaseData as DelegatedRenewalCaseMessageData, DelegatedRenewalReportData,
    InboxMessage, MessageCategory, MessageContext, MessagePriority,
};
use domain::player::{ContractRenewalState, Player, RenewalSessionOutcome, RenewalSessionStatus};
use domain::staff::StaffRole;
use std::collections::{HashMap, HashSet};

pub fn delegate_renewals(
    game: &mut Game,
    options: DelegatedRenewalOptions,
) -> Result<DelegatedRenewalReport, String> {
    let manager_team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("No team assigned".to_string())?;
    let team = game
        .teams
        .iter()
        .find(|candidate| candidate.id == manager_team_id)
        .ok_or("Manager team not found".to_string())?
        .clone();
    let assistant = game
        .staff
        .iter()
        .find(|staff| {
            staff.team_id.as_deref() == Some(team.id.as_str())
                && staff.role == StaffRole::AssistantManager
        })
        .ok_or("No assistant manager assigned to your team".to_string())?;
    let current_date = game.clock.current_date.date_naive();
    let today = current_date.format("%Y-%m-%d").to_string();
    let max_years = options.max_contract_years.max(1);
    let selected_ids = options
        .player_ids
        .clone()
        .map(|ids| ids.into_iter().collect::<HashSet<_>>());
    let candidate_indices: Vec<usize> = game
        .players
        .iter()
        .enumerate()
        .filter_map(|(index, player)| {
            if player.team_id.as_deref() != Some(team.id.as_str()) || player.contract_end.is_none()
            {
                return None;
            }

            if selected_ids.is_none() && has_let_expire_intent(player) {
                return None;
            }

            if let Some(selected_ids) = selected_ids.as_ref() {
                if selected_ids.contains(&player.id) {
                    return Some(index);
                }

                return None;
            }

            if contract_warning_stage(player.contract_end.as_deref(), current_date).is_some() {
                return Some(index);
            }

            None
        })
        .collect();
    let mut report = DelegatedRenewalReport {
        success_count: 0,
        failure_count: 0,
        stalled_count: 0,
        cases: Vec::new(),
    };

    for player_index in candidate_indices {
        let player = &game.players[player_index];
        let expected_wage = expected_wage(player, &team, current_date);
        let expected_years = expected_contract_years(player, current_date);
        let agreed_years = expected_years.min(max_years);
        let max_wage = round_up_to_nearest_thousand(
            player
                .wage
                .saturating_mul(100 + options.max_wage_increase_pct)
                / 100,
        );

        let mut case = DelegatedRenewalCase {
            player_id: player.id.clone(),
            player_name: player.match_name.clone(),
            status: DelegatedRenewalResultStatus::Failed,
            agreed_wage: None,
            agreed_years: None,
            note: String::new(),
            note_key: None,
            note_params: HashMap::new(),
        };

        if has_let_expire_intent(player) {
            report.failure_count += 1;
            case.note = "You already marked this contract to expire.".to_string();
            case.note_key = Some("be.msg.delegatedRenewals.notes.markedLetExpire".to_string());
            report.cases.push(case);
            continue;
        }

        if has_active_manager_block(player, current_date) {
            report.failure_count += 1;
            case.note = "You told me not to reopen contract talks yet.".to_string();
            case.note_key = Some("be.msg.delegatedRenewals.notes.managerBlocked".to_string());
            report.cases.push(case);
            continue;
        }

        if max_wage < expected_wage || max_years < expected_years {
            report.stalled_count += 1;
            case.status = DelegatedRenewalResultStatus::Stalled;
            case.note = format!(
                "Their camp want around €{}/wk for {} years, which is beyond the delegation limits.",
                expected_wage, expected_years
            );
            case.note_key = Some("be.msg.delegatedRenewals.notes.beyondLimits".to_string());
            case.note_params = delegated_note_params(&[
                ("wage", expected_wage.to_string()),
                ("years", expected_years.to_string()),
            ]);
            let player = &mut game.players[player_index];
            let state = player
                .morale_core
                .renewal_state
                .get_or_insert_with(ContractRenewalState::default);
            state.status = RenewalSessionStatus::Stalled;
            state.last_assistant_attempt_date = Some(today.clone());
            state.last_outcome = Some(RenewalSessionOutcome::Stalled);
            state.conversation_round = 0;
            report.cases.push(case);
            continue;
        }

        let delegation_score = assistant_delegation_score(assistant, player, current_date);

        if delegation_score >= 95 {
            let agreed_wage = expected_wage.min(max_wage);
            if !renewal_wage_policy_allows(game, &team, player.wage, agreed_wage) {
                report.stalled_count += 1;
                case.status = DelegatedRenewalResultStatus::Stalled;
                case.note = "I could progress the talks, but board wage limits prevented this renewal from being completed.".to_string();
                case.note_key = Some("be.msg.delegatedRenewals.notes.boardWagePolicy".to_string());
                case.note_params =
                    delegated_note_params(&[("budget", team.wage_budget.to_string())]);
                let player = &mut game.players[player_index];
                let state = player
                    .morale_core
                    .renewal_state
                    .get_or_insert_with(ContractRenewalState::default);
                state.status = RenewalSessionStatus::Stalled;
                state.last_assistant_attempt_date = Some(today.clone());
                state.last_outcome = Some(RenewalSessionOutcome::Stalled);
                state.conversation_round = 0;
                report.cases.push(case);
                continue;
            }

            let new_contract_end = current_date
                .checked_add_months(Months::new(agreed_years * 12))
                .ok_or("Unable to calculate delegated contract end date".to_string())?;
            let player = &mut game.players[player_index];
            player.wage = agreed_wage;
            player.contract_end = Some(new_contract_end.format("%Y-%m-%d").to_string());
            let state = player
                .morale_core
                .renewal_state
                .get_or_insert_with(ContractRenewalState::default);
            state.status = RenewalSessionStatus::Agreed;
            state.manager_blocked_until = None;
            state.last_assistant_attempt_date = Some(today.clone());
            state.last_outcome = Some(RenewalSessionOutcome::AcceptedByAssistant);
            state.conversation_round = 0;
            state.exit_intent = None;

            report.success_count += 1;
            case.status = DelegatedRenewalResultStatus::Successful;
            case.agreed_wage = Some(player.wage);
            case.agreed_years = Some(agreed_years);
            case.note = "I was able to close this one without needing you to step in.".to_string();
            case.note_key = Some("be.msg.delegatedRenewals.notes.completed".to_string());
            report.cases.push(case);
            continue;
        }

        if delegation_score >= 72 {
            report.stalled_count += 1;
            case.status = DelegatedRenewalResultStatus::Stalled;
            case.note = format!(
                "They would listen, but they still want about €{}/wk for {} years and prefer to hear from you directly.",
                expected_wage, expected_years
            );
            case.note_key = Some("be.msg.delegatedRenewals.notes.prefersManager".to_string());
            case.note_params = delegated_note_params(&[
                ("wage", expected_wage.to_string()),
                ("years", expected_years.to_string()),
            ]);
            let player = &mut game.players[player_index];
            let state = player
                .morale_core
                .renewal_state
                .get_or_insert_with(ContractRenewalState::default);
            state.status = RenewalSessionStatus::Open;
            state.last_assistant_attempt_date = Some(today.clone());
            state.last_outcome = Some(RenewalSessionOutcome::Stalled);
            state.conversation_round = 0;
            report.cases.push(case);
            continue;
        }

        report.failure_count += 1;
        case.note = "They are not willing to commit through me under the current relationship and contract situation.".to_string();
        case.note_key = Some("be.msg.delegatedRenewals.notes.relationshipBlocked".to_string());
        let player = &mut game.players[player_index];
        let state = player
            .morale_core
            .renewal_state
            .get_or_insert_with(ContractRenewalState::default);
        state.status = RenewalSessionStatus::Stalled;
        state.last_assistant_attempt_date = Some(today.clone());
        state.last_outcome = Some(RenewalSessionOutcome::RejectedByPlayer);
        state.conversation_round = 0;
        report.cases.push(case);
    }

    if !report.cases.is_empty() {
        let team_name = team.name.clone();
        let message_id_suffix = game.messages.len();
        game.messages.push(delegated_renewal_report_message(
            &team.id,
            &team_name,
            &today,
            &report,
            message_id_suffix,
        ));
    }

    Ok(report)
}

fn assistant_delegation_score(
    assistant: &domain::staff::Staff,
    player: &Player,
    current_date: NaiveDate,
) -> i32 {
    let assistant_quality = (i32::from(assistant.attributes.coaching) * 4
        + i32::from(assistant.attributes.judging_ability) * 3
        + i32::from(assistant.attributes.judging_potential) * 3)
        / 10;
    let trust_bonus = i32::from(player.morale_core.manager_trust) / 3;
    let morale_bonus = i32::from(player.morale) / 2;
    let urgency_bonus = match contract_warning_stage(player.contract_end.as_deref(), current_date) {
        Some(ContractWarningStage::FinalWeeks) => 18,
        Some(ContractWarningStage::ThreeMonths) => 14,
        Some(ContractWarningStage::SixMonths) => 10,
        Some(ContractWarningStage::TwelveMonths) => 6,
        None => 2,
    };
    let importance_penalty = if player.market_value >= 2_000_000 {
        22
    } else if player.market_value >= 750_000 {
        10
    } else {
        0
    };
    let issue_penalty = player
        .morale_core
        .unresolved_issue
        .as_ref()
        .map(|issue| i32::from(issue.severity) / 2)
        .unwrap_or(0);

    assistant_quality + trust_bonus + morale_bonus + urgency_bonus
        - importance_penalty
        - issue_penalty
}

fn delegated_note_params(entries: &[(&str, String)]) -> HashMap<String, String> {
    entries
        .iter()
        .map(|(key, value)| ((*key).to_string(), value.clone()))
        .collect()
}

fn delegated_renewal_report_message(
    team_id: &str,
    team_name: &str,
    date: &str,
    report: &DelegatedRenewalReport,
    id_suffix: usize,
) -> InboxMessage {
    InboxMessage::new(
        format!("delegated_renewals_{}_{}", date, id_suffix),
        "Assistant Report — Contract Renewals".to_string(),
        format!(
            "Boss, I went through our renewal list at {}. {} completed, {} still pending, {} failed.",
            team_name, report.success_count, report.stalled_count, report.failure_count
        ),
        "Assistant Manager".to_string(),
        date.to_string(),
    )
    .with_category(MessageCategory::Contract)
    .with_priority(MessagePriority::High)
    .with_sender_role("Assistant Manager")
    .with_i18n(
        "be.msg.delegatedRenewals.subject",
        "be.msg.delegatedRenewals.body",
        delegated_note_params(&[
            ("team", team_name.to_string()),
            ("successes", report.success_count.to_string()),
            ("stalled", report.stalled_count.to_string()),
            ("failures", report.failure_count.to_string()),
        ]),
    )
    .with_sender_i18n("be.sender.assistantManager", "be.role.assistantManager")
    .with_context(MessageContext {
        team_id: Some(team_id.to_string()),
        delegated_renewal_report: Some(DelegatedRenewalReportData {
            success_count: report.success_count,
            failure_count: report.failure_count,
            stalled_count: report.stalled_count,
            cases: report
                .cases
                .iter()
                .map(|case| DelegatedRenewalCaseMessageData {
                    player_id: case.player_id.clone(),
                    player_name: case.player_name.clone(),
                    status: match case.status {
                        DelegatedRenewalResultStatus::Successful => "successful".to_string(),
                        DelegatedRenewalResultStatus::Failed => "failed".to_string(),
                        DelegatedRenewalResultStatus::Stalled => "stalled".to_string(),
                    },
                    agreed_wage: case.agreed_wage,
                    agreed_years: case.agreed_years,
                    note_key: case.note_key.clone(),
                    note_params: case.note_params.clone(),
                })
                .collect(),
        }),
        ..Default::default()
    })
}
