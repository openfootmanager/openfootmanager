use super::{action, format_money, params};
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
        format!("Sponsorship Offer — {}", sponsor),
        format!(
            "Good news, boss! {} has expressed interest in becoming a sponsor of {}.\n\n\
            They're offering a weekly payment of €{} over the next 12 weeks in exchange for advertising space at the training ground.\n\n\
            This seems like a reasonable deal, but it's your call.",
            sponsor, team_name, format_money(amount)
        ),
        "Commercial Director".to_string(),
        date.to_string(),
    )
    .with_category(MessageCategory::Finance)
    .with_priority(MessagePriority::Normal)
    .with_sender_role("Commercial Director")
    .with_action(action(
        "respond", "Respond", "be.msg.event.respond",
        ActionType::ChooseOption {
            options: vec![
                ActionOption {
                    id: "accept".to_string(),
                    label: "Accept the deal".to_string(),
                    description: format!("Receive €{} in sponsorship income.", format_money(amount)),
                    label_key: Some("be.msg.sponsor.options.accept.label".to_string()),
                    description_key: Some("be.msg.sponsor.options.accept.description".to_string()),
                },
                ActionOption {
                    id: "decline".to_string(),
                    label: "Decline politely".to_string(),
                    description: "Turn down the offer. No financial impact.".to_string(),
                    label_key: Some("be.msg.sponsor.options.decline.label".to_string()),
                    description_key: Some("be.msg.sponsor.options.decline.description".to_string()),
                },
            ],
        },
    ))
    .with_i18n(
        "be.msg.sponsor.subject",
        "be.msg.sponsor.body",
        params(&[("sponsor", sponsor), ("team", team_name), ("amount", &format_money(amount))]),
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
    let mut rng = rand::rng();
    let variations = [
        format!(
            "Bad news from the training ground. {} has picked up a {} during today's session.\n\n\
            The medical team estimates {} days on the sidelines. We'll monitor the recovery closely.",
            player_name,
            injury_name.to_lowercase(),
            days
        ),
        format!(
            "Unfortunately, {} went down in training today with a {}.\n\n\
            Initial assessment: out for approximately {} days. We'll keep you updated on their progress.",
            player_name,
            injury_name.to_lowercase(),
            days
        ),
    ];
    let idx = rng.random_range(0..variations.len());

    InboxMessage::new(
        msg_id.to_string(),
        format!("Injury — {} ({})", player_name, injury_name),
        variations[idx].clone(),
        "Head Physio".to_string(),
        date.to_string(),
    )
    .with_category(MessageCategory::Injury)
    .with_priority(MessagePriority::High)
    .with_sender_role("Head Physio")
    .with_action(action(
        "ack",
        "Understood",
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
    let mut rng = rand::rng();

    let (subject, body) = if is_positive {
        let stories = [
            (
                format!("Positive Press — {}", player_name),
                format!(
                    "The local papers are running a very positive piece on {} today.\n\n\
                    They're highlighting their excellent recent form and commitment to {}. \
                    This kind of coverage is great for the player's confidence and the club's image.",
                    player_name, team_name
                ),
            ),
            (
                format!("Media Praise for {}", player_name),
                format!(
                    "Sports journalists have singled out {} for praise in their latest column.\n\n\
                    \"One of the standout performers at {} this season\" — great for morale.",
                    player_name, team_name
                ),
            ),
        ];
        stories[rng.random_range(0..stories.len())].clone()
    } else {
        let stories = [
            (
                format!("Negative Press — {}", player_name),
                format!(
                    "Some unflattering coverage of {} appeared in the tabloids today.\n\n\
                    Journalists are questioning their form and commitment to {}. \
                    This could affect the player's morale — you might want to have a word.",
                    player_name, team_name
                ),
            ),
            (
                format!("Media Criticism — {}", player_name),
                format!(
                    "The press are being harsh on {} in today's papers.\n\n\
                    \"Is {} getting the best out of their squad?\" — the article questions your use of the player. \
                    Worth keeping an eye on how this affects the dressing room.",
                    player_name, team_name
                ),
            ),
        ];
        stories[rng.random_range(0..stories.len())].clone()
    };

    InboxMessage::new(
        msg_id.to_string(),
        subject,
        body,
        "Press Officer".to_string(),
        date.to_string(),
    )
    .with_category(MessageCategory::Media)
    .with_priority(if is_positive {
        MessagePriority::Low
    } else {
        MessagePriority::Normal
    })
    .with_sender_role("Press Officer")
    .with_action(action(
        "ack",
        "Noted",
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
        format!("International Call-Up — {}", player_name),
        format!(
            "{} has been called up to the {} national team for an upcoming international window.\n\n\
            This is a great honor for the player and reflects well on the club. \
            They'll be in good spirits when they return, though keep an eye on their fatigue levels.",
            player_name, nationality
        ),
        "International Liaison".to_string(),
        date.to_string(),
    )
    .with_category(MessageCategory::LeagueInfo)
    .with_priority(MessagePriority::Normal)
    .with_sender_role("International Liaison")
    .with_action(action("ack", "Acknowledged", "be.msg.event.ack", ActionType::Acknowledge))
    .with_i18n(
        "be.msg.intlCallup.subject",
        "be.msg.intlCallup.body",
        params(&[("player", player_name), ("nationality", nationality)]),
    )
    .with_sender_i18n("be.sender.intlLiaison", "be.role.intlLiaison")
}

pub(super) fn community_event_message(msg_id: &str, team_name: &str, date: &str) -> InboxMessage {
    let mut rng = rand::rng();
    let events = [
        (
            "Community Open Day",
            format!(
                "{} hosted a community open day at the training ground today.\n\n\
                Fans got to meet the players and watch a training session. \
                The atmosphere was fantastic and it's done wonders for team spirit.",
                team_name
            ),
        ),
        (
            "Youth Coaching Session",
            format!(
                "Several first-team players from {} volunteered for a youth coaching session at a local school.\n\n\
                Great PR for the club, and the players seem energized by the experience.",
                team_name
            ),
        ),
        (
            "Charity Match Announcement",
            format!(
                "The club has organized a charity initiative in partnership with a local foundation.\n\n\
                {} continues to build strong ties with the community. The board is pleased with the positive image.",
                team_name
            ),
        ),
    ];
    let idx = rng.random_range(0..events.len());
    let (subject, body) = &events[idx];

    InboxMessage::new(
        msg_id.to_string(),
        subject.to_string(),
        body.clone(),
        "Community Manager".to_string(),
        date.to_string(),
    )
    .with_category(MessageCategory::System)
    .with_priority(MessagePriority::Low)
    .with_sender_role("Community Manager")
    .with_action(action(
        "ack",
        "Great",
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
