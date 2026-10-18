use crate::game::Game;
use crate::match_day_eligibility::match_day_eligible_players;
use domain::player::Player;
use domain::team::{TrainingIntensity, TrainingSchedule};
use serde::Serialize;
use std::collections::HashMap;

/// Below this condition a player is exhausted.
const EXHAUSTED_BELOW: u8 = 40;
/// Below this condition a player is in a critical state.
const CRITICAL_BELOW: u8 = 25;

/// How fit the club's squad is, as the staff's warnings and the screens both read it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SquadFitness {
    /// The healthy first-team players the figures are over.
    pub players: usize,
    pub average_condition: f64,
    /// Players below 40.
    pub exhausted_count: usize,
    /// Players below 25.
    pub critical_count: usize,
}

/// The club's healthy first-team players: the match-day pool (seniors and called-up youth) without
/// anyone injured. Injured players are not in the squad that can play, so they are not counted.
fn healthy_first_team<'a>(game: &'a Game, team_id: &str) -> Vec<&'a Player> {
    match_day_eligible_players(&game.players, team_id)
        .into_iter()
        .filter(|player| player.injury.is_none())
        .collect()
}

fn fitness_of(players: &[&Player]) -> Option<SquadFitness> {
    if players.is_empty() {
        return None;
    }
    let below = |limit: u8| players.iter().filter(|p| p.condition < limit).count();
    Some(SquadFitness {
        players: players.len(),
        average_condition: players.iter().map(|p| p.condition as f64).sum::<f64>()
            / players.len() as f64,
        exhausted_count: below(EXHAUSTED_BELOW),
        critical_count: below(CRITICAL_BELOW),
    })
}

/// The squad fitness summary for `team_id`; none when no first-team player is fit to be counted.
pub fn squad_fitness(game: &Game, team_id: &str) -> Option<SquadFitness> {
    fitness_of(&healthy_first_team(game, team_id))
}

/// Check squad fitness and generate staff warning messages when players are exhausted.
/// Called after training processing on each day.
pub fn check_squad_fitness_warnings(game: &mut Game) {
    let user_team_id = match &game.manager.team_id {
        Some(id) => id.clone(),
        None => return,
    };

    let date = game.clock.current_date.to_rfc3339();
    let today_str = game.clock.current_date.format("%Y-%m-%d").to_string();

    // The same summary the screens show.
    let team_players = healthy_first_team(game, &user_team_id);
    let Some(SquadFitness {
        average_condition: avg_condition,
        exhausted_count,
        critical_count,
        ..
    }) = fitness_of(&team_players)
    else {
        return;
    };

    // Deduplicate: only one warning per day
    let msg_id = format!("fitness_warn_{}", today_str);
    if crate::inbox::already_emitted(game, &msg_id) {
        return;
    }

    // Get team schedule for context
    let schedule = game
        .teams
        .iter()
        .find(|t| t.id == user_team_id)
        .map(|t| t.training_schedule.clone())
        .unwrap_or_default();

    let intensity = game
        .teams
        .iter()
        .find(|t| t.id == user_team_id)
        .map(|t| t.training_intensity.clone())
        .unwrap_or_default();

    // Determine if we need a physio/staff role sender
    let has_physio = game.staff.iter().any(|s| {
        s.team_id.as_deref() == Some(&user_team_id)
            && matches!(s.role, domain::staff::StaffRole::Physio)
    });

    let sender_name = if has_physio {
        game.staff
            .iter()
            .find(|s| {
                s.team_id.as_deref() == Some(&user_team_id)
                    && matches!(s.role, domain::staff::StaffRole::Physio)
            })
            .map(|s| format!("{} {}", s.first_name, s.last_name))
            .unwrap_or_default()
    } else {
        game.staff
            .iter()
            .find(|s| {
                s.team_id.as_deref() == Some(&user_team_id)
                    && matches!(s.role, domain::staff::StaffRole::AssistantManager)
            })
            .map(|s| format!("{} {}", s.first_name, s.last_name))
            .unwrap_or_default()
    };

    use domain::message::*;

    // Critical: multiple players below 25 condition
    if critical_count >= 3 {
        let exhausted_names: Vec<String> = team_players
            .iter()
            .filter(|p| p.condition < CRITICAL_BELOW)
            .take(5)
            .map(|p| format!("{} ({}%)", p.match_name, p.condition))
            .collect();

        let mut msg = InboxMessage::new(msg_id, String::new(), String::new(), sender_name, date)
            .with_category(MessageCategory::Training)
            .with_priority(MessagePriority::Urgent)
            .with_sender_role("")
            .with_action(MessageAction {
                id: "go_training".to_string(),
                label: String::new(),
                action_type: ActionType::NavigateTo {
                    route: "/dashboard?tab=Training".to_string(),
                },
                resolved: false,
                label_key: Some("be.msg.fitness.actionAdjust".to_string()),
            })
            .with_context(MessageContext {
                team_id: Some(user_team_id),
                ..Default::default()
            })
            .with_i18n(
                "be.msg.fitness.critical.subject",
                &format!(
                    "be.msg.fitness.critical.body.{}",
                    match schedule {
                        TrainingSchedule::Intense => "intense",
                        TrainingSchedule::Balanced => "balanced",
                        TrainingSchedule::Light => "light",
                    }
                ),
                {
                    let mut p = HashMap::new();
                    p.insert("criticalCount".to_string(), critical_count.to_string());
                    p.insert("players".to_string(), exhausted_names.join("\n"));
                    p.insert("avgCondition".to_string(), format!("{:.0}", avg_condition));
                    let sched_key = match schedule {
                        TrainingSchedule::Intense => "intense",
                        TrainingSchedule::Balanced => "balanced",
                        TrainingSchedule::Light => "light",
                    };
                    p.insert("schedule".to_string(), sched_key.to_string());
                    let int_key = match intensity {
                        TrainingIntensity::High => "high",
                        _ => "",
                    };
                    p.insert("intensity".to_string(), int_key.to_string());
                    p
                },
            );

        if has_physio {
            msg = msg.with_sender_i18n("be.sender.headPhysio", "be.role.headPhysio");
        } else {
            msg = msg.with_sender_i18n("be.sender.assistantManager", "be.role.assistantManager");
        }

        crate::inbox::emit(game, msg);
        return;
    }

    // Warning: average condition below 50 or many exhausted players
    if avg_condition < 50.0 || exhausted_count >= 4 {
        let mut msg = InboxMessage::new(msg_id, String::new(), String::new(), sender_name, date)
            .with_category(MessageCategory::Training)
            .with_priority(MessagePriority::High)
            .with_sender_role("")
            .with_action(MessageAction {
                id: "go_training".to_string(),
                label: String::new(),
                action_type: ActionType::NavigateTo {
                    route: "/dashboard?tab=Training".to_string(),
                },
                resolved: false,
                label_key: Some("be.msg.fitness.actionAdjust".to_string()),
            })
            .with_context(MessageContext {
                team_id: Some(user_team_id),
                ..Default::default()
            })
            .with_i18n(
                "be.msg.fitness.warning.subject",
                &format!(
                    "be.msg.fitness.warning.body.{}",
                    match schedule {
                        TrainingSchedule::Intense => "intense",
                        TrainingSchedule::Balanced => "balanced",
                        TrainingSchedule::Light => "light",
                    }
                ),
                {
                    let mut p = HashMap::new();
                    p.insert("avgCondition".to_string(), format!("{:.0}", avg_condition));
                    p.insert("exhaustedCount".to_string(), exhausted_count.to_string());
                    let sched_key = match schedule {
                        TrainingSchedule::Intense => "intense",
                        TrainingSchedule::Balanced => "balanced",
                        TrainingSchedule::Light => "light",
                    };
                    p.insert("schedule".to_string(), sched_key.to_string());
                    p
                },
            );

        if has_physio {
            msg = msg.with_sender_i18n("be.sender.headPhysio", "be.role.headPhysio");
        } else {
            msg = msg.with_sender_i18n("be.sender.assistantManager", "be.role.assistantManager");
        }

        crate::inbox::emit(game, msg);
    }
}
