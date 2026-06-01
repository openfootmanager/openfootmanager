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

fn action(id: &str, label_key: &str, action_type: ActionType) -> MessageAction {
    MessageAction {
        id: id.to_string(),
        label: String::new(),
        action_type,
        resolved: false,
        label_key: Some(label_key.to_string()),
    }
}

fn option(id: &str, label_key: &str, description_key: &str) -> ActionOption {
    ActionOption {
        id: id.to_string(),
        label: String::new(),
        description: String::new(),
        label_key: Some(label_key.to_string()),
        description_key: Some(description_key.to_string()),
    }
}

pub(crate) fn low_morale_message(
    msg_id: &str,
    player_id: &str,
    player_name: &str,
    morale: u8,
    date: &str,
) -> InboxMessage {
    let idx = rand::rng().random_range(0..2);

    InboxMessage::new(
        msg_id.to_string(),
        String::new(),
        String::new(),
        String::new(),
        date.to_string(),
    )
    .with_category(MessageCategory::PlayerMorale)
    .with_priority(MessagePriority::High)
    .with_sender_role("")
    .with_action(action(
        "respond",
        "be.msg.playerEvent.respond",
        ActionType::ChooseOption {
            options: vec![
                option(
                    "encourage",
                    "be.msg.playerEvent.options.moraleCrisis.encourage.label",
                    "be.msg.playerEvent.options.moraleCrisis.encourage.description",
                ),
                option(
                    "promise_time",
                    "be.msg.playerEvent.options.moraleCrisis.promiseTime.label",
                    "be.msg.playerEvent.options.moraleCrisis.promiseTime.description",
                ),
                option(
                    "work_harder",
                    "be.msg.playerEvent.options.moraleCrisis.workHarder.label",
                    "be.msg.playerEvent.options.moraleCrisis.workHarder.description",
                ),
            ],
        },
    ))
    .with_context(MessageContext {
        player_id: Some(player_id.to_string()),
        ..Default::default()
    })
    .with_i18n(
        "be.msg.moraleCrisis.subject",
        &format!("be.msg.moraleCrisis.body{}", idx),
        params(&[("player", player_name), ("morale", &morale.to_string())]),
    )
    .with_sender_i18n("be.sender.player", "be.role.player")
}

pub(crate) fn bench_complaint_message(
    msg_id: &str,
    player_id: &str,
    player_name: &str,
    date: &str,
) -> InboxMessage {
    let idx = rand::rng().random_range(0..2);

    InboxMessage::new(
        msg_id.to_string(),
        String::new(),
        String::new(),
        String::new(),
        date.to_string(),
    )
    .with_category(MessageCategory::PlayerMorale)
    .with_priority(MessagePriority::Normal)
    .with_sender_role("")
    .with_action(action(
        "respond",
        "be.msg.playerEvent.respond",
        ActionType::ChooseOption {
            options: vec![
                option(
                    "explain",
                    "be.msg.playerEvent.options.benchComplaint.explain.label",
                    "be.msg.playerEvent.options.benchComplaint.explain.description",
                ),
                option(
                    "promise_chance",
                    "be.msg.playerEvent.options.benchComplaint.promiseChance.label",
                    "be.msg.playerEvent.options.benchComplaint.promiseChance.description",
                ),
                option(
                    "prove_yourself",
                    "be.msg.playerEvent.options.benchComplaint.proveYourself.label",
                    "be.msg.playerEvent.options.benchComplaint.proveYourself.description",
                ),
            ],
        },
    ))
    .with_context(MessageContext {
        player_id: Some(player_id.to_string()),
        ..Default::default()
    })
    .with_i18n(
        "be.msg.benchComplaint.subject",
        &format!("be.msg.benchComplaint.body{}", idx),
        params(&[("player", player_name)]),
    )
    .with_sender_i18n("be.sender.player", "be.role.player")
}

pub(crate) fn happy_player_message(
    msg_id: &str,
    player_id: &str,
    player_name: &str,
    date: &str,
) -> InboxMessage {
    let idx = rand::rng().random_range(0..2);

    InboxMessage::new(
        msg_id.to_string(),
        String::new(),
        String::new(),
        String::new(),
        date.to_string(),
    )
    .with_category(MessageCategory::PlayerMorale)
    .with_priority(MessagePriority::Low)
    .with_sender_role("")
    .with_action(action(
        "respond",
        "be.msg.playerEvent.respond",
        ActionType::ChooseOption {
            options: vec![
                option(
                    "praise_back",
                    "be.msg.playerEvent.options.happyPlayer.praiseBack.label",
                    "be.msg.playerEvent.options.happyPlayer.praiseBack.description",
                ),
                option(
                    "stay_professional",
                    "be.msg.playerEvent.options.happyPlayer.stayProfessional.label",
                    "be.msg.playerEvent.options.happyPlayer.stayProfessional.description",
                ),
                option(
                    "higher_expectations",
                    "be.msg.playerEvent.options.happyPlayer.higherExpectations.label",
                    "be.msg.playerEvent.options.happyPlayer.higherExpectations.description",
                ),
            ],
        },
    ))
    .with_context(MessageContext {
        player_id: Some(player_id.to_string()),
        ..Default::default()
    })
    .with_i18n(
        "be.msg.happyPlayer.subject",
        &format!("be.msg.happyPlayer.body{}", idx),
        params(&[("player", player_name)]),
    )
    .with_sender_i18n("be.sender.player", "be.role.player")
}

pub(crate) fn contract_concern_message(
    msg_id: &str,
    player_id: &str,
    player_name: &str,
    days_remaining: i64,
    date: &str,
) -> InboxMessage {
    let months = (days_remaining as f64 / 30.0).ceil() as u32;
    let idx = rand::rng().random_range(0..2);

    InboxMessage::new(
        msg_id.to_string(),
        String::new(),
        String::new(),
        String::new(),
        date.to_string(),
    )
    .with_category(MessageCategory::Contract)
    .with_priority(MessagePriority::High)
    .with_sender_role("")
    .with_action(action(
        "respond",
        "be.msg.playerEvent.respond",
        ActionType::ChooseOption {
            options: vec![
                option(
                    "reassure",
                    "be.msg.playerEvent.options.contractConcern.reassure.label",
                    "be.msg.playerEvent.options.contractConcern.reassure.description",
                ),
                option(
                    "noncommittal",
                    "be.msg.playerEvent.options.contractConcern.noncommittal.label",
                    "be.msg.playerEvent.options.contractConcern.noncommittal.description",
                ),
                option(
                    "no_renewal",
                    "be.msg.playerEvent.options.contractConcern.noRenewal.label",
                    "be.msg.playerEvent.options.contractConcern.noRenewal.description",
                ),
            ],
        },
    ))
    .with_context(MessageContext {
        player_id: Some(player_id.to_string()),
        ..Default::default()
    })
    .with_i18n(
        "be.msg.contractConcern.subject",
        &format!("be.msg.contractConcern.body{}", idx),
        params(&[
            ("player", player_name),
            ("days", &days_remaining.to_string()),
            ("months", &months.to_string()),
        ]),
    )
    .with_sender_i18n("be.sender.assistantManager", "be.role.assistantManager")
}

pub(crate) fn takeover_contract_review_message(
    msg_id: &str,
    total_expiring_this_season: usize,
    urgent_contracts: usize,
    final_weeks_contracts: usize,
    date: &str,
) -> InboxMessage {
    let body_key = match (urgent_contracts, final_weeks_contracts) {
        (0, _) => "be.msg.takeoverContractReview.bodyNoneUrgent",
        (_, 0) => "be.msg.takeoverContractReview.bodyUrgent",
        _ => "be.msg.takeoverContractReview.bodyFinalMonth",
    };

    InboxMessage::new(
        msg_id.to_string(),
        String::new(),
        String::new(),
        String::new(),
        date.to_string(),
    )
    .with_category(MessageCategory::Contract)
    .with_priority(MessagePriority::High)
    .with_sender_role("")
    .with_i18n(
        "be.msg.takeoverContractReview.subject",
        body_key,
        params(&[
            ("totalExpiring", &total_expiring_this_season.to_string()),
            ("urgentContracts", &urgent_contracts.to_string()),
            ("finalMonthContracts", &final_weeks_contracts.to_string()),
        ]),
    )
    .with_sender_i18n("be.sender.assistantManager", "be.role.assistantManager")
    .with_action(action(
        "view_squad",
        "be.msg.takeoverContractReview.actionReview",
        ActionType::NavigateTo {
            route: "/dashboard?tab=Squad".to_string(),
        },
    ))
    .with_action(action(
        "ack",
        "be.msg.event.ack",
        ActionType::Acknowledge,
    ))
    .with_context(MessageContext::default())
}

#[cfg(test)]
mod tests {
    use super::{
        low_morale_message, takeover_contract_review_message,
    };
    use domain::message::ActionType;

    #[test]
    fn low_morale_message_uses_i18n_keys_without_raw_fallbacks() {
        let message = low_morale_message("morale_1", "player-1", "Alex Star", 31, "2026-07-01");

        assert_eq!(message.subject_key.as_deref(), Some("be.msg.moraleCrisis.subject"));
        assert!(matches!(
            message.body_key.as_deref(),
            Some("be.msg.moraleCrisis.body0") | Some("be.msg.moraleCrisis.body1")
        ));
        assert_eq!(message.sender_key.as_deref(), Some("be.sender.player"));
        assert_eq!(message.sender_role_key.as_deref(), Some("be.role.player"));
        assert!(message.subject.is_empty());
        assert!(message.body.is_empty());
        assert!(message.sender.is_empty());
        assert_eq!(message.actions[0].label_key.as_deref(), Some("be.msg.playerEvent.respond"));
        assert!(message.actions[0].label.is_empty());
        assert!(matches!(
            message.actions[0].action_type,
            ActionType::ChooseOption { .. }
        ));
    }

    #[test]
    fn takeover_contract_review_message_uses_i18n_keys_and_variant_body_keys() {
        let no_urgency = takeover_contract_review_message("review_1", 2, 0, 0, "2026-07-01");
        assert_eq!(
            no_urgency.subject_key.as_deref(),
            Some("be.msg.takeoverContractReview.subject")
        );
        assert_eq!(
            no_urgency.body_key.as_deref(),
            Some("be.msg.takeoverContractReview.bodyNoneUrgent")
        );
        assert_eq!(
            no_urgency.sender_key.as_deref(),
            Some("be.sender.assistantManager")
        );
        assert_eq!(
            no_urgency.sender_role_key.as_deref(),
            Some("be.role.assistantManager")
        );
        assert!(no_urgency.subject.is_empty());
        assert!(no_urgency.body.is_empty());
        assert!(no_urgency.sender.is_empty());
        assert!(no_urgency.actions[0].label.is_empty());
        assert!(no_urgency.actions[1].label.is_empty());
        assert_eq!(
            no_urgency.actions[0].label_key.as_deref(),
            Some("be.msg.takeoverContractReview.actionReview")
        );
        assert_eq!(
            no_urgency.actions[1].label_key.as_deref(),
            Some("be.msg.event.ack")
        );
        assert_eq!(
            no_urgency
                .i18n_params
                .get("totalExpiring")
                .map(String::as_str),
            Some("2")
        );
        assert_eq!(
            no_urgency
                .i18n_params
                .get("urgentContracts")
                .map(String::as_str),
            Some("0")
        );
        assert_eq!(
            no_urgency
                .i18n_params
                .get("finalMonthContracts")
                .map(String::as_str),
            Some("0")
        );

        let urgent = takeover_contract_review_message("review_2", 4, 2, 0, "2026-07-01");
        assert_eq!(
            urgent.body_key.as_deref(),
            Some("be.msg.takeoverContractReview.bodyUrgent")
        );

        let final_month = takeover_contract_review_message("review_3", 5, 3, 1, "2026-07-01");
        assert_eq!(
            final_month.body_key.as_deref(),
            Some("be.msg.takeoverContractReview.bodyFinalMonth")
        );
    }
}
