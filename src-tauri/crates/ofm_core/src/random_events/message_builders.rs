use super::{action, params};
use domain::message::*;
use rand::RngExt;

// ---------------------------------------------------------------------------
// Message builders
// ---------------------------------------------------------------------------

pub(crate) fn sponsor_offer_message(
    msg_id: &str,
    team_name: &str,
    sponsor: &str,
    amount: u64,
    date: &str,
) -> InboxMessage {
    InboxMessage::new(
        msg_id.to_string(),
        String::new(),
        String::new(),
        String::new(),
        date.to_string(),
    )
    .with_category(MessageCategory::Finance)
    .with_priority(MessagePriority::Normal)
    .with_sender_role("")
    .with_action(action(
        "respond", "", "be.msg.event.respond",
        ActionType::ChooseOption {
            options: vec![
                ActionOption {
                    id: "accept".to_string(),
                    label: String::new(),
                    description: String::new(),
                    label_key: Some("be.msg.sponsor.options.accept.label".to_string()),
                    description_key: Some("be.msg.sponsor.options.accept.description".to_string()),
                },
                ActionOption {
                    id: "decline".to_string(),
                    label: String::new(),
                    description: String::new(),
                    label_key: Some("be.msg.sponsor.options.decline.label".to_string()),
                    description_key: Some("be.msg.sponsor.options.decline.description".to_string()),
                },
            ],
        },
    ))
    .with_i18n(
        "be.msg.sponsor.subject",
        "be.msg.sponsor.body",
        params(&[("sponsor", sponsor), ("team", team_name), ("amount", &amount.to_string())]),
    )
    .with_sender_i18n("be.sender.commercialDirector", "be.role.commercialDirector")
}

pub(super) fn training_injury_message(
    msg_id: &str,
    player_id: &str,
    player_name: &str,
    injury_name: &str,
    days: u32,
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
    .with_category(MessageCategory::Injury)
    .with_priority(MessagePriority::High)
    .with_sender_role("")
    .with_action(action(
        "ack",
        "",
        "be.msg.event.ack",
        ActionType::Acknowledge,
    ))
    .with_context(MessageContext {
        player_id: Some(player_id.to_string()),
        ..Default::default()
    })
    .with_i18n(
        "be.msg.trainingInjury.subject",
        &format!("be.msg.trainingInjury.body{}", idx),
        params(&[
            ("player", player_name),
            ("injury", injury_name),
            ("days", &days.to_string()),
        ]),
    )
    .with_sender_i18n("be.sender.headPhysio", "be.role.headPhysio")
}

pub(super) fn media_story_message(
    msg_id: &str,
    team_name: &str,
    player_id: &str,
    player_name: &str,
    is_positive: bool,
    date: &str,
) -> InboxMessage {
    InboxMessage::new(
        msg_id.to_string(),
        String::new(),
        String::new(),
        String::new(),
        date.to_string(),
    )
    .with_category(MessageCategory::Media)
    .with_priority(if is_positive {
        MessagePriority::Low
    } else {
        MessagePriority::Normal
    })
    .with_sender_role("")
    .with_action(action(
        "ack",
        "",
        "be.msg.event.ack",
        ActionType::Acknowledge,
    ))
    .with_context(MessageContext {
        player_id: Some(player_id.to_string()),
        ..Default::default()
    })
    .with_i18n(
        if is_positive {
            "be.msg.mediaPositive.subject"
        } else {
            "be.msg.mediaNegative.subject"
        },
        if is_positive {
            "be.msg.mediaPositive.body"
        } else {
            "be.msg.mediaNegative.body"
        },
        params(&[("player", player_name), ("team", team_name)]),
    )
    .with_sender_i18n("be.sender.pressOfficer", "be.role.pressOfficer")
}

pub(super) fn international_callup_message(
    msg_id: &str,
    player_name: &str,
    nationality: &str,
    date: &str,
) -> InboxMessage {
    InboxMessage::new(
        msg_id.to_string(),
        String::new(),
        String::new(),
        String::new(),
        date.to_string(),
    )
    .with_category(MessageCategory::LeagueInfo)
    .with_priority(MessagePriority::Normal)
    .with_sender_role("")
    .with_action(action("ack", "", "be.msg.event.ack", ActionType::Acknowledge))
    .with_i18n(
        "be.msg.intlCallup.subject",
        "be.msg.intlCallup.body",
        params(&[("player", player_name), ("nationality", nationality)]),
    )
    .with_sender_i18n("be.sender.intlLiaison", "be.role.intlLiaison")
}

pub(super) fn community_event_message(msg_id: &str, team_name: &str, date: &str) -> InboxMessage {
    let idx = rand::rng().random_range(0..3);

    InboxMessage::new(
        msg_id.to_string(),
        String::new(),
        String::new(),
        String::new(),
        date.to_string(),
    )
    .with_category(MessageCategory::System)
    .with_priority(MessagePriority::Low)
    .with_sender_role("")
    .with_action(action(
        "ack",
        "",
        "be.msg.event.ack",
        ActionType::Acknowledge,
    ))
    .with_i18n(
        &format!("be.msg.community.subject{}", idx),
        &format!("be.msg.community.body{}", idx),
        params(&[("team", team_name)]),
    )
    .with_sender_i18n("be.sender.communityManager", "be.role.communityManager")
}
