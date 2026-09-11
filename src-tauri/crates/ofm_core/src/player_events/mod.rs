mod message_builders;
mod responses;

pub use responses::{
    PlayerResponseEffect, ResponseBandWeights, ResponseOutcomeBand, apply_player_response,
    build_response_band_weights, pick_response_band,
};

use crate::contracts::{contract_warning_stage, has_let_expire_intent};
use crate::game::Game;
use domain::message::InboxMessage;
use rand::RngExt;

use message_builders::{
    bench_complaint_message, contract_concern_message, happy_player_message, low_morale_message,
    takeover_contract_review_message,
};

fn should_generate_contract_concern(stage: crate::contracts::ContractWarningStage) -> bool {
    !matches!(stage, crate::contracts::ContractWarningStage::TwelveMonths)
}

pub fn generate_takeover_contract_review_message(game: &mut Game) {
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let user_team_id = match game.manager.team_id.as_deref() {
        Some(id) => id,
        None => return,
    };

    // Keyed on the day of the takeover, not the club alone: a manager who leaves
    // and later returns is taking over again and wants the briefing again. With
    // the club as the whole key, the ledger would answer "already sent" forever.
    let summary_id = format!("contract_review_takeover_{}_{}", user_team_id, today);
    if crate::inbox::already_emitted(game, &summary_id) {
        return;
    }

    let current_date = game.clock.current_date.date_naive();
    let mut total_expiring_this_season = 0;
    let mut urgent_contracts = 0;
    let mut final_weeks_contracts = 0;

    for player in &game.players {
        if player.team_id.as_deref() != Some(user_team_id) {
            continue;
        }

        let Some(stage) = contract_warning_stage(player.contract_end.as_deref(), current_date)
        else {
            continue;
        };

        total_expiring_this_season += 1;

        match stage {
            crate::contracts::ContractWarningStage::FinalWeeks => {
                urgent_contracts += 1;
                final_weeks_contracts += 1;
            }
            crate::contracts::ContractWarningStage::ThreeMonths
            | crate::contracts::ContractWarningStage::SixMonths => {
                urgent_contracts += 1;
            }
            crate::contracts::ContractWarningStage::TwelveMonths => {}
        }
    }

    if total_expiring_this_season == 0 {
        return;
    }

    let review = takeover_contract_review_message(
        &summary_id,
        total_expiring_this_season,
        urgent_contracts,
        final_weeks_contracts,
        &today,
    );
    crate::inbox::emit(game, review);
}

fn talk_cooldown_active(player: &domain::player::Player, today: &str) -> bool {
    player.morale_core.talk_cooldown_until.as_deref() == Some(today)
}

pub fn generate_contract_concern_messages(game: &mut Game, apply_morale_pressure: bool) {
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let user_team_id = match game.manager.team_id.clone() {
        Some(id) => id,
        None => return,
    };
    let current_date = game.clock.current_date.date_naive();
    // The ledger, not the mailbox: a message the player deleted was still sent.
    // Borrowed, not cloned — the ledger is never pruned, so it only grows.
    let existing_ids = &game.emitted_events;
    let mut new_messages: Vec<InboxMessage> = Vec::new();

    for player in game.players.iter_mut() {
        if player.team_id.as_deref() != Some(&user_team_id) {
            continue;
        }

        if has_let_expire_intent(player) {
            continue;
        }

        let Some(stage) = contract_warning_stage(player.contract_end.as_deref(), current_date)
        else {
            continue;
        };

        if !should_generate_contract_concern(stage) {
            continue;
        }

        // The contract itself is in the key. Keyed on player and stage alone, a
        // renewal would inherit the old deal's stages from the ledger and the
        // player would never be warned about the new one.
        let contract_end = player.contract_end.clone().unwrap_or_default();
        let msg_id = format!(
            "contract_concern_{}_{}_{}",
            player.id,
            contract_end,
            stage.message_suffix()
        );

        if existing_ids.contains(&msg_id) {
            continue;
        }

        let Ok(end_date) = chrono::NaiveDate::parse_from_str(&contract_end, "%Y-%m-%d") else {
            continue;
        };

        // The morale hit rides with the message, not with the attempt. It used
        // to be applied before the message was built, so a player who deleted
        // the message took the hit again every time it regenerated.
        if apply_morale_pressure {
            player.morale = (player.morale as i16 - stage.morale_pressure()).clamp(5, 100) as u8;
        }

        let days_remaining = (end_date - current_date).num_days();
        new_messages.push(contract_concern_message(
            &msg_id,
            &player.id,
            &player.match_name,
            days_remaining,
            &today,
        ));
    }

    crate::inbox::emit_all(game, new_messages);
}

/// Check all player-related events and generate inbox messages.
/// Called once per day from process_day().
pub fn check_player_events(game: &mut Game) {
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let user_team_id = match game.manager.team_id.clone() {
        Some(id) => id,
        None => return,
    };

    // The ledger, not the mailbox: a message the player deleted was still sent.
    // Borrowed, not cloned — the ledger is never pruned, so it only grows.
    let existing_ids = &game.emitted_events;
    // These three are conditions, not moments — a player can be unhappy again
    // next year. The season in the key is what lets them recur; without it the
    // ledger would take each player's first complaint as his last.
    let season = crate::inbox::recurrence_season(game);

    let mut new_messages: Vec<InboxMessage> = Vec::new();

    let mut rng = rand::rng();

    // Global daily cap: at most 2 player-initiated messages per day
    let today_message_count = game
        .messages
        .iter()
        .filter(|m| {
            m.date == today
                && (m.id.starts_with("morale_talk_")
                    || m.id.starts_with("bench_complaint_")
                    || m.id.starts_with("happy_player_"))
        })
        .count();
    let daily_cap: usize = 2;

    // --- 1. Low morale meeting requests (morale < 30, 20% daily chance) ---
    for player in game.players.iter() {
        if new_messages.len() + today_message_count >= daily_cap {
            break;
        }
        if player.team_id.as_deref() != Some(&user_team_id) {
            continue;
        }
        if player.injury.is_some() {
            continue;
        }
        if talk_cooldown_active(player, &today) {
            continue;
        }

        let msg_id = format!("morale_talk_{}_{}", player.id, season);
        if existing_ids.contains(&msg_id) {
            continue;
        }

        if player.morale < 30 && rng.random_range(0..5) == 0 {
            new_messages.push(low_morale_message(
                &msg_id,
                &player.id,
                &player.match_name,
                player.morale,
                &today,
            ));
        }
    }

    // --- 2. Benched player complaints ---
    // Players with zero appearances but decent OVR complain occasionally.
    // Uses appearances count (reliable) instead of unreliable scorer-only tracking.
    if let Some(league) = &game.league {
        let user_matches_played = league
            .fixtures
            .iter()
            .filter(|f| {
                f.status == domain::league::FixtureStatus::Completed
                    && (f.home_team_id == user_team_id || f.away_team_id == user_team_id)
            })
            .count();

        // Only start bench complaints after 5 team matches have been played
        if user_matches_played >= 5 {
            for player in game.players.iter() {
                if new_messages.len() + today_message_count >= daily_cap {
                    break;
                }
                if player.team_id.as_deref() != Some(&user_team_id) {
                    continue;
                }
                if player.injury.is_some() {
                    continue;
                }
                if player.position == domain::player::Position::Goalkeeper {
                    continue;
                }
                if talk_cooldown_active(player, &today) {
                    continue;
                }

                let msg_id = format!("bench_complaint_{}_{}", player.id, season);
                if existing_ids.contains(&msg_id) {
                    continue;
                }

                // The player already carries a position-weighted rating; a flat
                // average here silently held keepers below the threshold.
                let ovr = player.ovr;

                // Player must have decent OVR, low morale, and few appearances
                // relative to team matches. 10% daily chance to avoid flooding.
                let app_ratio = if user_matches_played > 0 {
                    player.stats.appearances as f64 / user_matches_played as f64
                } else {
                    1.0
                };
                if ovr >= 55
                    && player.morale < 50
                    && app_ratio < 0.3
                    && rng.random_range(0..10) == 0
                {
                    new_messages.push(bench_complaint_message(
                        &msg_id,
                        &player.id,
                        &player.match_name,
                        &today,
                    ));
                }
            }
        }
    }

    // --- 3. Happy player / high morale praise (1% daily chance) ---
    {
        for player in game.players.iter() {
            if new_messages.len() + today_message_count >= daily_cap {
                break;
            }
            if player.team_id.as_deref() != Some(&user_team_id) {
                continue;
            }
            if talk_cooldown_active(player, &today) {
                continue;
            }

            let msg_id = format!("happy_player_{}_{}", player.id, season);
            if existing_ids.contains(&msg_id) {
                continue;
            }

            if player.morale >= 90 && rng.random_range(0..100) == 0 {
                new_messages.push(happy_player_message(
                    &msg_id,
                    &player.id,
                    &player.match_name,
                    &today,
                ));
            }
        }
    }

    crate::inbox::emit_all(game, new_messages);
    generate_contract_concern_messages(game, true);
}
