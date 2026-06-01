use super::{action, params};
use domain::message::*;
use rand::RngExt;

fn option(id: &str, label_key: &str, description_key: &str) -> ActionOption {
    ActionOption {
        id: id.to_string(),
        label: String::new(),
        description: String::new(),
        label_key: Some(label_key.to_string()),
        description_key: Some(description_key.to_string()),
    }
}

// ---------------------------------------------------------------------------
// Periodic / condition-triggered message builders
// ---------------------------------------------------------------------------

pub(super) fn mood_report_message(
    msg_id: &str,
    avg_morale: f64,
    low_count: usize,
    high_count: usize,
    total: usize,
    date: &str,
) -> InboxMessage {
    let mood = if avg_morale >= 75.0 {
        "common.moods.excellent"
    } else if avg_morale >= 60.0 {
        "common.moods.good"
    } else if avg_morale >= 45.0 {
        "common.moods.mixed"
    } else {
        "common.moods.poor"
    };

    InboxMessage::new(
        msg_id.to_string(),
        String::new(),
        String::new(),
        String::new(),
        date.to_string(),
    )
    .with_category(MessageCategory::PlayerMorale)
    .with_priority(if low_count >= 3 || avg_morale < 40.0 {
        MessagePriority::High
    } else {
        MessagePriority::Low
    })
    .with_sender_role("")
    .with_action(action(
        "ack",
        "",
        "be.msg.event.ack",
        ActionType::Acknowledge,
    ))
    .with_i18n("be.msg.moodReport.subject", "be.msg.moodReport.body", {
        let mut p = params(&[("mood", mood)]);
        p.insert("avgMorale".to_string(), format!("{:.0}", avg_morale));
        p.insert("highCount".to_string(), high_count.to_string());
        p.insert("lowCount".to_string(), low_count.to_string());
        p.insert("total".to_string(), total.to_string());
        p
    })
    .with_sender_i18n("be.sender.assistantManager", "be.role.assistantManager")
}

pub(super) fn board_confidence_message(msg_id: &str, date: &str) -> InboxMessage {
    let mut rng = rand::rng();
    let idx = rng.random_range(0..2);

    InboxMessage::new(
        msg_id.to_string(),
        String::new(),
        String::new(),
        String::new(),
        date.to_string(),
    )
    .with_category(MessageCategory::BoardDirective)
    .with_priority(MessagePriority::Urgent)
    .with_sender_role("")
    .with_action(action(
        "respond",
        "",
        "be.msg.event.respond",
        ActionType::ChooseOption {
            options: vec![
                option(
                    "reassure_board",
                    "be.msg.boardConfidence.options.reassureBoard.label",
                    "be.msg.boardConfidence.options.reassureBoard.description",
                ),
                option(
                    "accept_pressure",
                    "be.msg.boardConfidence.options.acceptPressure.label",
                    "be.msg.boardConfidence.options.acceptPressure.description",
                ),
                option(
                    "blame_circumstances",
                    "be.msg.boardConfidence.options.blameCircumstances.label",
                    "be.msg.boardConfidence.options.blameCircumstances.description",
                ),
            ],
        },
    ))
    .with_i18n(
        "be.msg.boardConfidence.subject",
        &format!("be.msg.boardConfidence.body{}", idx),
        params(&[]),
    )
    .with_sender_i18n("be.sender.boardOfDirectors", "be.role.chairman")
}

pub(super) fn fan_petition_message(msg_id: &str, team_name: &str, date: &str) -> InboxMessage {
    let idx = rand::rng().random_range(0..3);

    InboxMessage::new(
        msg_id.to_string(),
        String::new(),
        String::new(),
        String::new(),
        date.to_string(),
    )
    .with_category(MessageCategory::Media)
    .with_priority(MessagePriority::Normal)
    .with_sender_role("")
    .with_action(action(
        "respond", "", "be.msg.event.respond",
        ActionType::ChooseOption {
            options: vec![
                option(
                    "listen_fans",
                    "be.msg.fanPetition.options.listenFans.label",
                    "be.msg.fanPetition.options.listenFans.description",
                ),
                option(
                    "ignore_fans",
                    "be.msg.fanPetition.options.ignoreFans.label",
                    "be.msg.fanPetition.options.ignoreFans.description",
                ),
                option(
                    "address_publicly",
                    "be.msg.fanPetition.options.addressPublicly.label",
                    "be.msg.fanPetition.options.addressPublicly.description",
                ),
            ],
        },
    ))
    .with_i18n(
        &format!("be.msg.fanPetition.subject{}", idx),
        &format!("be.msg.fanPetition.body{}", idx),
        params(&[("team", team_name)]),
    )
    .with_sender_i18n("be.sender.communityManager", "be.role.communityManager")
}

pub(super) fn rival_interest_message(
    msg_id: &str,
    player_id: &str,
    player_name: &str,
    rival_name: &str,
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
    .with_category(MessageCategory::Transfer)
    .with_priority(MessagePriority::Normal)
    .with_sender_role("")
    .with_action(action(
        "respond",
        "",
        "be.msg.event.respond",
        ActionType::ChooseOption {
            options: vec![
                option(
                    "not_for_sale",
                    "be.msg.rivalInterest.options.notForSale.label",
                    "be.msg.rivalInterest.options.notForSale.description",
                ),
                option(
                    "open_to_offers",
                    "be.msg.rivalInterest.options.openToOffers.label",
                    "be.msg.rivalInterest.options.openToOffers.description",
                ),
                option(
                    "no_comment",
                    "be.msg.rivalInterest.options.noComment.label",
                    "be.msg.rivalInterest.options.noComment.description",
                ),
            ],
        },
    ))
    .with_context(MessageContext {
        player_id: Some(player_id.to_string()),
        ..Default::default()
    })
    .with_i18n(
        "be.msg.rivalInterest.subject",
        &format!("be.msg.rivalInterest.body{}", idx),
        params(&[("player", player_name), ("rival", rival_name)]),
    )
    .with_sender_i18n("be.sender.directorOfFootball", "be.role.directorOfFootball")
}

#[cfg(test)]
mod tests {
    use super::{board_confidence_message, mood_report_message};
    use domain::message::ActionType;

    #[test]
    fn mood_report_message_uses_i18n_keys_without_raw_fallbacks() {
        let message = mood_report_message("mood_1", 61.0, 2, 4, 22, "2026-07-01");

        assert_eq!(message.subject_key.as_deref(), Some("be.msg.moodReport.subject"));
        assert_eq!(message.body_key.as_deref(), Some("be.msg.moodReport.body"));
        assert_eq!(message.sender_key.as_deref(), Some("be.sender.assistantManager"));
        assert_eq!(message.sender_role_key.as_deref(), Some("be.role.assistantManager"));
        assert!(message.subject.is_empty());
        assert!(message.body.is_empty());
        assert!(message.sender.is_empty());
        assert_eq!(message.actions[0].label_key.as_deref(), Some("be.msg.event.ack"));
        assert!(message.actions[0].label.is_empty());
    }

    #[test]
    fn board_confidence_message_uses_keyed_options_without_raw_fallbacks() {
        let message = board_confidence_message("board_1", "2026-07-01");

        assert_eq!(message.subject_key.as_deref(), Some("be.msg.boardConfidence.subject"));
        assert!(matches!(
            message.body_key.as_deref(),
            Some("be.msg.boardConfidence.body0") | Some("be.msg.boardConfidence.body1")
        ));
        assert_eq!(message.sender_key.as_deref(), Some("be.sender.boardOfDirectors"));
        assert_eq!(message.sender_role_key.as_deref(), Some("be.role.chairman"));
        assert!(message.subject.is_empty());
        assert!(message.body.is_empty());
        assert!(message.sender.is_empty());
        assert_eq!(message.actions[0].label_key.as_deref(), Some("be.msg.event.respond"));
        assert!(message.actions[0].label.is_empty());
        assert!(matches!(
            message.actions[0].action_type,
            ActionType::ChooseOption { .. }
        ));
    }
}
