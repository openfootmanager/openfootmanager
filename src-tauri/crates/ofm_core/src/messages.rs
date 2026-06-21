mod match_messages;
pub use match_messages::{match_result_message, pre_match_message};

use domain::message::*;
use rand::RngExt;
use std::collections::HashMap;

/// Helper to build a HashMap<String, String> from key-value pairs.
fn params(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// Helper to create a MessageAction with an i18n label key.
fn action(id: &str, label: &str, label_key: &str, action_type: ActionType) -> MessageAction {
    MessageAction {
        id: id.to_string(),
        label: label.to_string(),
        action_type,
        resolved: false,
        label_key: Some(label_key.to_string()),
    }
}

/// Message template system — generates rich messages with variations.
pub fn welcome_message(team_name: &str, team_id: &str, date: &str) -> InboxMessage {
    let mut rng = rand::rng();
    let idx = rng.random_range(0..3);

    InboxMessage::new(
        "welcome_1".to_string(),
        String::new(),
        String::new(),
        String::new(),
        date.to_string(),
    )
    .with_category(MessageCategory::Welcome)
    .with_priority(MessagePriority::High)
    .with_sender_role("")
    .with_action(action(
        "review_squad",
        "",
        "be.msg.welcome.actionReview",
        ActionType::NavigateTo {
            route: "/dashboard?tab=Squad".to_string(),
        },
    ))
    .with_action(action(
        "ack_welcome",
        "",
        "be.msg.welcome.actionThank",
        ActionType::Acknowledge,
    ))
    .with_context(MessageContext {
        team_id: Some(team_id.to_string()),
        ..Default::default()
    })
    .with_i18n(
        &format!("be.msg.welcome.subject{}", idx),
        &format!("be.msg.welcome.body{}", idx),
        params(&[("team", team_name)]),
    )
    .with_sender_i18n("be.sender.boardOfDirectors", "be.role.chairman")
}

pub fn season_schedule_message(league_name: &str, season_start: &str, date: &str) -> InboxMessage {
    let mut rng = rand::rng();
    let idx = rng.random_range(0..2);

    InboxMessage::new(
        "season_1".to_string(),
        String::new(),
        String::new(),
        String::new(),
        date.to_string(),
    )
    .with_category(MessageCategory::LeagueInfo)
    .with_priority(MessagePriority::Normal)
    .with_sender_role("")
    .with_action(action(
        "view_schedule",
        "",
        "be.msg.schedule.actionView",
        ActionType::NavigateTo {
            route: "/dashboard?tab=Schedule".to_string(),
        },
    ))
    .with_i18n(
        "be.msg.schedule.subject",
        &format!("be.msg.schedule.body{}", idx),
        params(&[("league", league_name), ("start", season_start)]),
    )
    .with_sender_i18n("be.sender.leagueOffice", "be.role.competitionSecretary")
}

pub fn staff_advice_message(team_name: &str, team_id: &str, date: &str) -> InboxMessage {
    InboxMessage::new(
        "staff_advice_1".to_string(),
        String::new(),
        String::new(),
        String::new(),
        date.to_string(),
    )
    .with_category(MessageCategory::Training)
    .with_priority(MessagePriority::High)
    .with_sender_role("")
    .with_action(action(
        "view_staff",
        "",
        "be.msg.staffAdvice.actionView",
        ActionType::NavigateTo {
            route: "/dashboard?tab=Staff".to_string(),
        },
    ))
    .with_context(MessageContext {
        team_id: Some(team_id.to_string()),
        ..Default::default()
    })
    .with_i18n(
        "be.msg.staffAdvice.subject",
        "be.msg.staffAdvice.body",
        params(&[("team", team_name)]),
    )
    .with_sender_i18n("be.sender.assistantManager", "be.role.assistantManager")
}

pub fn board_expectations_message(team_name: &str, team_id: &str, date: &str) -> InboxMessage {
    InboxMessage::new(
        "board_expect_1".to_string(),
        String::new(),
        String::new(),
        String::new(),
        date.to_string(),
    )
    .with_category(MessageCategory::BoardDirective)
    .with_priority(MessagePriority::High)
    .with_sender_role("")
    .with_action(action(
        "ack_objectives",
        "",
        "be.msg.boardExpect.actionAccept",
        ActionType::Acknowledge,
    ))
    .with_context(MessageContext {
        team_id: Some(team_id.to_string()),
        ..Default::default()
    })
    .with_i18n(
        "be.msg.boardExpect.subject",
        "be.msg.boardExpect.body",
        params(&[("team", team_name)]),
    )
    .with_sender_i18n("be.sender.boardOfDirectors", "be.role.chairman")
}

pub fn transfer_complete_message(player_name: &str, fee: u64, date: &str) -> InboxMessage {
    let fee_display =
        crate::currency::format_compact_money(fee, crate::currency::DEFAULT_CURRENCY_CODE)
            .unwrap_or_else(|| format!("{}{}", crate::currency::default_currency_symbol(), fee));

    let id = format!("transfer_{}", uuid::Uuid::new_v4());
    InboxMessage::new(
        id,
        String::new(),
        String::new(),
        String::new(),
        date.to_string(),
    )
    .with_category(MessageCategory::Transfer)
    .with_priority(MessagePriority::Normal)
    .with_sender_role("")
    .with_i18n(
        "be.msg.transferComplete.subject",
        "be.msg.transferComplete.body",
        HashMap::from([
            ("player".to_string(), player_name.to_string()),
            ("fee".to_string(), fee_display),
        ]),
    )
    .with_sender_i18n("be.sender.transferCommittee", "be.role.directorOfFootball")
}

pub fn incoming_transfer_offer_message(
    offer_id: &str,
    player_id: &str,
    player_name: &str,
    buying_team_name: &str,
    fee: u64,
    date: &str,
) -> InboxMessage {
    let fee_display =
        crate::currency::format_compact_money(fee, crate::currency::DEFAULT_CURRENCY_CODE)
            .unwrap_or_else(|| format!("{}{}", crate::currency::default_currency_symbol(), fee));

    InboxMessage::new(
        format!("transfer_offer_{}", offer_id),
        String::new(),
        String::new(),
        String::new(),
        date.to_string(),
    )
    .with_category(MessageCategory::Transfer)
    .with_priority(MessagePriority::High)
    .with_sender_role("")
    .with_action(action(
        "view_transfers",
        "",
        "be.msg.transferOffer.actionReview",
        ActionType::NavigateTo {
            route: "/dashboard?tab=Transfers".to_string(),
        },
    ))
    .with_context(MessageContext {
        player_id: Some(player_id.to_string()),
        ..Default::default()
    })
    .with_i18n(
        "be.msg.transferOffer.subject",
        "be.msg.transferOffer.body",
        HashMap::from([
            ("player".to_string(), player_name.to_string()),
            ("team".to_string(), buying_team_name.to_string()),
            ("fee".to_string(), fee_display),
        ]),
    )
    .with_sender_i18n("be.sender.directorOfFootball", "be.role.directorOfFootball")
}

/// A single per-player inbox thread that collapses repeated incoming interest
/// into one updating digest instead of one message per club, capping spam.
/// The stable id (`transfer_interest_<player_id>`) lets callers upsert it.
pub fn transfer_interest_digest_message(
    player_id: &str,
    player_name: &str,
    interested_club_count: usize,
    latest_team_name: &str,
    latest_fee: u64,
    date: &str,
) -> InboxMessage {
    let fee_display =
        crate::currency::format_compact_money(latest_fee, crate::currency::DEFAULT_CURRENCY_CODE)
            .unwrap_or_else(|| {
                format!(
                    "{}{}",
                    crate::currency::default_currency_symbol(),
                    latest_fee
                )
            });

    InboxMessage::new(
        format!("transfer_interest_{}", player_id),
        String::new(),
        String::new(),
        String::new(),
        date.to_string(),
    )
    .with_category(MessageCategory::Transfer)
    .with_priority(MessagePriority::High)
    .with_sender_role("")
    .with_action(action(
        "view_transfers",
        "",
        "be.msg.transferInterest.actionReview",
        ActionType::NavigateTo {
            route: "/dashboard?tab=Transfers".to_string(),
        },
    ))
    .with_context(MessageContext {
        player_id: Some(player_id.to_string()),
        ..Default::default()
    })
    .with_i18n(
        "be.msg.transferInterest.subject",
        "be.msg.transferInterest.body",
        HashMap::from([
            ("player".to_string(), player_name.to_string()),
            ("n".to_string(), interested_club_count.to_string()),
            ("team".to_string(), latest_team_name.to_string()),
            ("fee".to_string(), fee_display),
        ]),
    )
    .with_sender_i18n("be.sender.directorOfFootball", "be.role.directorOfFootball")
}

#[allow(clippy::too_many_arguments)]
pub fn incoming_loan_offer_message(
    offer_id: &str,
    player_id: &str,
    player_name: &str,
    borrowing_team_name: &str,
    wage_contribution_pct: u8,
    buy_option_fee: Option<u64>,
    end_date: &str,
    date: &str,
) -> InboxMessage {
    let buy_option_display = buy_option_fee
        .and_then(|fee| {
            crate::currency::format_compact_money(fee, crate::currency::DEFAULT_CURRENCY_CODE)
        })
        .unwrap_or_else(|| "—".to_string());

    InboxMessage::new(
        format!("loan_offer_{}", offer_id),
        String::new(),
        String::new(),
        String::new(),
        date.to_string(),
    )
    .with_category(MessageCategory::Transfer)
    .with_priority(MessagePriority::High)
    .with_sender_role("")
    .with_action(action(
        "view_transfers",
        "",
        "be.msg.loanOffer.actionReview",
        ActionType::NavigateTo {
            route: "/dashboard?tab=Transfers".to_string(),
        },
    ))
    .with_context(MessageContext {
        player_id: Some(player_id.to_string()),
        ..Default::default()
    })
    .with_i18n(
        "be.msg.loanOffer.subject",
        "be.msg.loanOffer.body",
        HashMap::from([
            ("player".to_string(), player_name.to_string()),
            ("team".to_string(), borrowing_team_name.to_string()),
            (
                "wageContribution".to_string(),
                wage_contribution_pct.to_string(),
            ),
            ("endDate".to_string(), end_date.to_string()),
            ("buyOption".to_string(), buy_option_display),
        ]),
    )
    .with_sender_i18n("be.sender.directorOfFootball", "be.role.directorOfFootball")
}

#[allow(clippy::too_many_arguments)]
pub fn loan_development_report_message(
    message_id: &str,
    player_id: &str,
    player_name: &str,
    loan_team_name: &str,
    days_on_loan: i64,
    ovr_before: u8,
    ovr_after: u8,
    attribute_gains: u8,
    final_report: bool,
    date: &str,
) -> InboxMessage {
    InboxMessage::new(
        message_id.to_string(),
        String::new(),
        String::new(),
        String::new(),
        date.to_string(),
    )
    .with_category(MessageCategory::Training)
    .with_priority(MessagePriority::Normal)
    .with_sender_role("")
    .with_context(MessageContext {
        player_id: Some(player_id.to_string()),
        ..Default::default()
    })
    .with_i18n(
        "be.msg.loanDevelopmentReport.subject",
        if final_report {
            "be.msg.loanDevelopmentReport.bodyFinal"
        } else {
            "be.msg.loanDevelopmentReport.body"
        },
        HashMap::from([
            ("player".to_string(), player_name.to_string()),
            ("team".to_string(), loan_team_name.to_string()),
            ("days".to_string(), days_on_loan.to_string()),
            ("ovrBefore".to_string(), ovr_before.to_string()),
            ("ovrAfter".to_string(), ovr_after.to_string()),
            ("attributeGains".to_string(), attribute_gains.to_string()),
        ]),
    )
    .with_sender_i18n("be.sender.assistantManager", "be.role.assistantManager")
}

pub fn loan_buy_option_exercised_message(
    player_id: &str,
    player_name: &str,
    fee: u64,
    date: &str,
) -> InboxMessage {
    let fee_display =
        crate::currency::format_compact_money(fee, crate::currency::DEFAULT_CURRENCY_CODE)
            .unwrap_or_else(|| format!("{}{}", crate::currency::default_currency_symbol(), fee));

    InboxMessage::new(
        format!("loan_buy_option_{}_{}", player_id, date),
        String::new(),
        String::new(),
        String::new(),
        date.to_string(),
    )
    .with_category(MessageCategory::Transfer)
    .with_priority(MessagePriority::High)
    .with_sender_role("")
    .with_context(MessageContext {
        player_id: Some(player_id.to_string()),
        ..Default::default()
    })
    .with_i18n(
        "be.msg.loanBuyOptionExercised.subject",
        "be.msg.loanBuyOptionExercised.body",
        HashMap::from([
            ("player".to_string(), player_name.to_string()),
            ("fee".to_string(), fee_display),
        ]),
    )
    .with_sender_i18n("be.sender.transferCommittee", "be.role.directorOfFootball")
}
