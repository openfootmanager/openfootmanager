use super::format_money;
use crate::game::Game;
use domain::team::{Sponsorship, SponsorshipBonusCriterion};
use rand::RngExt;

fn parse_amount_param(value: &str) -> Option<u64> {
    let digits: String = value.chars().filter(|ch| ch.is_ascii_digit()).collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse::<u64>().ok()
    }
}

/// Apply the effect of a sponsor offer choice.
pub fn apply_event_response(
    game: &mut Game,
    message_id: &str,
    _action_id: &str,
    option_id: &str,
) -> Option<String> {
    if message_id.starts_with("sponsor_") {
        let user_team_id = game.manager.team_id.clone()?;
        match option_id {
            "accept" => {
                let amount = game
                    .messages
                    .iter()
                    .find(|m| m.id == message_id)
                    .and_then(|m| m.i18n_params.get("amount"))
                    .and_then(|amount| parse_amount_param(amount))
                    .unwrap_or(100_000);
                let sponsor_name = game
                    .messages
                    .iter()
                    .find(|m| m.id == message_id)
                    .and_then(|m| m.i18n_params.get("sponsor"))
                    .cloned()
                    .unwrap_or_else(|| "Sponsor".to_string());
                if let Some(team) = game.teams.iter_mut().find(|t| t.id == user_team_id) {
                    team.sponsorship = Some(Sponsorship {
                        sponsor_name,
                        base_value: amount as i64,
                        remaining_weeks: 12,
                        bonus_criteria: vec![SponsorshipBonusCriterion::UnbeatenRun {
                            required_matches: 3,
                            bonus_amount: amount as i64 / 4,
                        }],
                    });
                }
                // Mark resolved
                if let Some(msg) = game.messages.iter_mut().find(|m| m.id == message_id) {
                    for a in msg.actions.iter_mut() {
                        a.resolved = true;
                    }
                }
                Some(format!(
                    "Sponsorship deal signed! You will receive €{}/week for 12 weeks.",
                    format_money(amount)
                ))
            }
            "decline" => {
                if let Some(msg) = game.messages.iter_mut().find(|m| m.id == message_id) {
                    for a in msg.actions.iter_mut() {
                        a.resolved = true;
                    }
                }
                Some("Sponsorship declined.".to_string())
            }
            _ => None,
        }
    } else if message_id.starts_with("board_confidence_") {
        match option_id {
            "reassure_board" => {
                if let Some(msg) = game.messages.iter_mut().find(|m| m.id == message_id) {
                    for a in msg.actions.iter_mut() {
                        a.resolved = true;
                    }
                }
                Some("You reassured the board. They'll give you more time — for now.".to_string())
            }
            "accept_pressure" => {
                if let Some(msg) = game.messages.iter_mut().find(|m| m.id == message_id) {
                    for a in msg.actions.iter_mut() {
                        a.resolved = true;
                    }
                }
                Some(
                    "You acknowledged the pressure. The board appreciates your honesty."
                        .to_string(),
                )
            }
            "blame_circumstances" => {
                if let Some(msg) = game.messages.iter_mut().find(|m| m.id == message_id) {
                    for a in msg.actions.iter_mut() {
                        a.resolved = true;
                    }
                }
                Some(
                    "The board isn't entirely convinced by excuses, but they'll wait and see."
                        .to_string(),
                )
            }
            _ => None,
        }
    } else if message_id.starts_with("fan_petition_") {
        match option_id {
            "listen_fans" => {
                // Small morale boost across squad
                let user_team_id = game.manager.team_id.clone().unwrap_or_default();
                let mut rng = rand::rng();
                for p in game.players.iter_mut() {
                    if p.team_id.as_deref() == Some(&user_team_id) {
                        p.morale = (p.morale as i16 + rng.random_range(1..=3)).clamp(10, 100) as u8;
                    }
                }
                if let Some(msg) = game.messages.iter_mut().find(|m| m.id == message_id) {
                    for a in msg.actions.iter_mut() {
                        a.resolved = true;
                    }
                }
                Some("You engaged with the fans. Squad morale improved slightly.".to_string())
            }
            "ignore_fans" => {
                if let Some(msg) = game.messages.iter_mut().find(|m| m.id == message_id) {
                    for a in msg.actions.iter_mut() {
                        a.resolved = true;
                    }
                }
                Some(
                    "You decided to focus on football matters. The fans are a little disappointed."
                        .to_string(),
                )
            }
            "address_publicly" => {
                if let Some(msg) = game.messages.iter_mut().find(|m| m.id == message_id) {
                    for a in msg.actions.iter_mut() {
                        a.resolved = true;
                    }
                }
                Some("Your public address was well received. Fan confidence is up.".to_string())
            }
            _ => None,
        }
    } else if message_id.starts_with("rival_interest_") {
        let player_id = game
            .messages
            .iter()
            .find(|m| m.id == message_id)
            .and_then(|m| m.context.player_id.clone());
        match option_id {
            "not_for_sale" => {
                // Player morale boost — they feel valued
                if let Some(pid) = &player_id
                    && let Some(p) = game.players.iter_mut().find(|p| p.id == *pid)
                {
                    let mut rng = rand::rng();
                    p.morale = (p.morale as i16 + rng.random_range(3..=8)).clamp(10, 100) as u8;
                }
                if let Some(msg) = game.messages.iter_mut().find(|m| m.id == message_id) {
                    for a in msg.actions.iter_mut() {
                        a.resolved = true;
                    }
                }
                Some(
                    "You made it clear the player is not for sale. They're feeling valued."
                        .to_string(),
                )
            }
            "open_to_offers" => {
                // Player morale drop — they feel uncertain
                if let Some(pid) = &player_id
                    && let Some(p) = game.players.iter_mut().find(|p| p.id == *pid)
                {
                    let mut rng = rand::rng();
                    p.morale = (p.morale as i16 - rng.random_range(3..=8)).clamp(10, 100) as u8;
                }
                if let Some(msg) = game.messages.iter_mut().find(|m| m.id == message_id) {
                    for a in msg.actions.iter_mut() {
                        a.resolved = true;
                    }
                }
                Some("You indicated you'd listen to offers. The player is unsettled.".to_string())
            }
            "no_comment" => {
                if let Some(msg) = game.messages.iter_mut().find(|m| m.id == message_id) {
                    for a in msg.actions.iter_mut() {
                        a.resolved = true;
                    }
                }
                Some("No comment. The rumour mill continues...".to_string())
            }
            _ => None,
        }
    } else {
        None
    }
}
