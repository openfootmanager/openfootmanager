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

fn action(id: &str, label: &str, label_key: &str, action_type: ActionType) -> MessageAction {
    MessageAction {
        id: id.to_string(),
        label: label.to_string(),
        action_type,
        resolved: false,
        label_key: Some(label_key.to_string()),
    }
}

fn option(
    id: &str,
    label: &str,
    label_key: &str,
    description: &str,
    description_key: &str,
) -> ActionOption {
    ActionOption {
        id: id.to_string(),
        label: label.to_string(),
        description: description.to_string(),
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
    let mut rng = rand::rng();
    let variations = [
        format!(
            "Boss, {} has asked for a private meeting. They seem really down lately and want to talk about their situation at the club.\n\n\
            Their morale is at {} — you should address this before it affects the dressing room.",
            player_name, morale
        ),
        format!(
            "{} has been looking dejected in training. They've requested a chat with you about their current state of mind.\n\n\
            Morale: {}. How you handle this could make or break their confidence.",
            player_name, morale
        ),
    ];
    let idx = rng.random_range(0..variations.len());

    InboxMessage::new(
        msg_id.to_string(),
        format!("{} — Morale Crisis", player_name),
        variations[idx].clone(),
        player_name.to_string(),
        date.to_string(),
    )
    .with_category(MessageCategory::PlayerMorale)
    .with_priority(MessagePriority::High)
    .with_sender_role("Player")
    .with_action(action(
        "respond",
        "Respond",
        "be.msg.playerEvent.respond",
        ActionType::ChooseOption {
            options: vec![
                option(
                    "encourage",
                    "Encourage them",
                    "be.msg.playerEvent.options.moraleCrisis.encourage.label",
                    "Show empathy and encourage the player to keep working hard.",
                    "be.msg.playerEvent.options.moraleCrisis.encourage.description",
                ),
                option(
                    "promise_time",
                    "Promise more playing time",
                    "be.msg.playerEvent.options.moraleCrisis.promiseTime.label",
                    "Tell them they'll get their chance — bigger morale boost but sets expectations.",
                    "be.msg.playerEvent.options.moraleCrisis.promiseTime.description",
                ),
                option(
                    "work_harder",
                    "Tell them to work harder",
                    "be.msg.playerEvent.options.moraleCrisis.workHarder.label",
                    "Tough love approach — could backfire or motivate them.",
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
    let mut rng = rand::rng();
    let variations = [
        format!(
            "Boss, {} has come to see you. They're frustrated about their lack of game time in recent matches and want to know what they need to do to get back in the team.\n\n\
            \"I feel like I've been training well, but I'm not getting a chance to show it on the pitch.\"",
            player_name
        ),
        format!(
            "{} knocked on your office door looking unhappy. They haven't featured in the last few matches and want answers.\n\n\
            \"I came to this club to play football. If I'm not in your plans, I'd rather you tell me straight.\"",
            player_name
        ),
    ];
    let idx = rng.random_range(0..variations.len());

    InboxMessage::new(
        msg_id.to_string(),
        format!("{} — Wants More Game Time", player_name),
        variations[idx].clone(),
        player_name.to_string(),
        date.to_string(),
    )
    .with_category(MessageCategory::PlayerMorale)
    .with_priority(MessagePriority::Normal)
    .with_sender_role("Player")
    .with_action(action(
        "respond",
        "Respond",
        "be.msg.playerEvent.respond",
        ActionType::ChooseOption {
            options: vec![
                option(
                    "explain",
                    "Explain the situation",
                    "be.msg.playerEvent.options.benchComplaint.explain.label",
                    "Calmly explain squad competition and rotation. Steady morale boost.",
                    "be.msg.playerEvent.options.benchComplaint.explain.description",
                ),
                option(
                    "promise_chance",
                    "Promise them a chance soon",
                    "be.msg.playerEvent.options.benchComplaint.promiseChance.label",
                    "They'll be happier but will expect to start in upcoming matches.",
                    "be.msg.playerEvent.options.benchComplaint.promiseChance.description",
                ),
                option(
                    "prove_yourself",
                    "Tell them to prove themselves",
                    "be.msg.playerEvent.options.benchComplaint.proveYourself.label",
                    "Challenge them to earn their place. Risky — could motivate or frustrate.",
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
    let mut rng = rand::rng();
    let variations = [
        format!(
            "{} stopped by your office with a big smile. They're feeling great about their form and the team's direction.\n\n\
            \"Just wanted to say I'm really enjoying my football right now, boss. The mood in the dressing room is fantastic.\"",
            player_name
        ),
        format!(
            "Your assistant mentions that {} has been in excellent spirits lately. They approached you after training.\n\n\
            \"Boss, I'm loving every minute here. Keep things going like this and I'll run through walls for you.\"",
            player_name
        ),
    ];
    let idx = rng.random_range(0..variations.len());

    InboxMessage::new(
        msg_id.to_string(),
        format!("{} — Feeling Great", player_name),
        variations[idx].clone(),
        player_name.to_string(),
        date.to_string(),
    )
    .with_category(MessageCategory::PlayerMorale)
    .with_priority(MessagePriority::Low)
    .with_sender_role("Player")
    .with_action(action(
        "respond",
        "Respond",
        "be.msg.playerEvent.respond",
        ActionType::ChooseOption {
            options: vec![
                option(
                    "praise_back",
                    "Return the praise",
                    "be.msg.playerEvent.options.happyPlayer.praiseBack.label",
                    "Tell them how much you value their contribution.",
                    "be.msg.playerEvent.options.happyPlayer.praiseBack.description",
                ),
                option(
                    "stay_professional",
                    "Stay professional",
                    "be.msg.playerEvent.options.happyPlayer.stayProfessional.label",
                    "Acknowledge their form but keep things measured.",
                    "be.msg.playerEvent.options.happyPlayer.stayProfessional.description",
                ),
                option(
                    "higher_expectations",
                    "Set higher expectations",
                    "be.msg.playerEvent.options.happyPlayer.higherExpectations.label",
                    "Challenge them to reach an even higher level. Could push or pressure.",
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
    let mut rng = rand::rng();
    let variations = [
        format!(
            "{} has approached you regarding their contract situation. With only {} days remaining on their deal, they want to know where they stand.\n\n\
            \"Boss, my contract is running down. I need to know if I'm part of your plans going forward or if I should start looking elsewhere.\"",
            player_name, days_remaining
        ),
        format!(
            "Your assistant flags that {}'s contract expires in roughly {} month(s). The player has been asking around the dressing room about their future.\n\n\
            It might be wise to have a conversation before they become unsettled — or before other clubs start circling.",
            player_name, months
        ),
    ];
    let idx = rng.random_range(0..variations.len());

    InboxMessage::new(
        msg_id.to_string(),
        format!("{} — Contract Running Down", player_name),
        variations[idx].clone(),
        "Assistant Manager".to_string(),
        date.to_string(),
    )
    .with_category(MessageCategory::Contract)
    .with_priority(MessagePriority::High)
    .with_sender_role("Assistant Manager")
    .with_action(action(
        "respond",
        "Respond",
        "be.msg.playerEvent.respond",
        ActionType::ChooseOption {
            options: vec![
                option(
                    "reassure",
                    "Reassure them about renewal",
                    "be.msg.playerEvent.options.contractConcern.reassure.label",
                    "Tell them you want them to stay. Big morale boost.",
                    "be.msg.playerEvent.options.contractConcern.reassure.description",
                ),
                option(
                    "noncommittal",
                    "Be noncommittal",
                    "be.msg.playerEvent.options.contractConcern.noncommittal.label",
                    "Keep your options open. Player may become unsettled.",
                    "be.msg.playerEvent.options.contractConcern.noncommittal.description",
                ),
                option(
                    "no_renewal",
                    "Tell them you won't renew",
                    "be.msg.playerEvent.options.contractConcern.noRenewal.label",
                    "Honest but brutal. Morale will tank.",
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
    let (body, body_key) = match (urgent_contracts, final_weeks_contracts) {
        (0, _) => (
            format!(
                "Your assistant has reviewed the squad contracts after your arrival. {} player(s) are due to come up for renewal this season, but none require an immediate decision today.\n\nStart mapping out who you want to keep so the situation stays under control.",
                total_expiring_this_season,
            ),
            "be.msg.takeoverContractReview.bodyNoneUrgent",
        ),
        (_, 0) => (
            format!(
                "Your assistant has reviewed the squad contracts after your arrival. {} player(s) are due to come up for renewal this season, and {} already need attention inside the next six months.\n\nYou do not need to solve everything at once, but these cases should be near the top of your early workload.",
                total_expiring_this_season, urgent_contracts,
            ),
            "be.msg.takeoverContractReview.bodyUrgent",
        ),
        _ => (
            format!(
                "Your assistant has reviewed the squad contracts after your arrival. {} player(s) are due to come up for renewal this season, {} need attention inside the next six months, and {} are already in the final month.\n\nStabilize the urgent cases first, then work through the wider renewal list in stages.",
                total_expiring_this_season, urgent_contracts, final_weeks_contracts,
            ),
            "be.msg.takeoverContractReview.bodyFinalMonth",
        ),
    };

    InboxMessage::new(
        msg_id.to_string(),
        "Assistant Manager - Contract Review".to_string(),
        body,
        "Assistant Manager".to_string(),
        date.to_string(),
    )
    .with_category(MessageCategory::Contract)
    .with_priority(MessagePriority::High)
    .with_sender_role("Assistant Manager")
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
        "Review Squad Contracts",
        "be.msg.takeoverContractReview.actionReview",
        ActionType::NavigateTo {
            route: "/dashboard?tab=Squad".to_string(),
        },
    ))
    .with_action(action(
        "ack",
        "Acknowledge",
        "be.msg.event.ack",
        ActionType::Acknowledge,
    ))
    .with_context(MessageContext::default())
}

#[cfg(test)]
mod tests {
    use super::takeover_contract_review_message;

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
