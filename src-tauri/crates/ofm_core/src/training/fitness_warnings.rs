use crate::game::Game;
use crate::match_day_eligibility::{club_players, match_day_eligible_players};
use domain::player::Player;
use domain::team::{TrainingIntensity, TrainingSchedule};
use serde::Serialize;
use std::collections::HashMap;

/// Below this condition a player is exhausted.
const EXHAUSTED_BELOW: u8 = 40;
/// Below this condition a player is in a critical state.
const CRITICAL_BELOW: u8 = 25;

/// How tired a player is, by the two cut-offs the staff's warnings and the screens share.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum FitnessStatus {
    Fit,
    /// Below 40, and not critical.
    Exhausted,
    /// Below 25.
    Critical,
}

/// The one rule for a condition's status: every count and every per-player colour reads it.
pub fn fitness_status(condition: u8) -> FitnessStatus {
    if condition < CRITICAL_BELOW {
        FitnessStatus::Critical
    } else if condition < EXHAUSTED_BELOW {
        FitnessStatus::Exhausted
    } else {
        FitnessStatus::Fit
    }
}

/// How fit the club's squad is, as the staff's warnings and the screens both read it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SquadFitness {
    /// The healthy first-team players the figures are over.
    pub players: usize,
    /// Their average condition; none when there is no healthy first-team player to average.
    pub average_condition: Option<f64>,
    /// Players below 40, the critical ones among them.
    pub exhausted_count: usize,
    /// Players below 25.
    pub critical_count: usize,
    /// Every player of the club, injured and academy players included (the lists show them), by
    /// player id. The counts above leave the injured and the uncalled academy out.
    pub statuses: HashMap<String, FitnessStatus>,
}

/// The club's healthy first-team players: the match-day pool (seniors and called-up youth) without
/// anyone injured. Injured players are not in the squad that can play, so they are not counted.
fn healthy_first_team<'a>(game: &'a Game, team_id: &str) -> Vec<&'a Player> {
    match_day_eligible_players(&game.players, team_id)
        .into_iter()
        .filter(|player| player.injury.is_none())
        .collect()
}

/// The squad fitness summary for `team_id`.
pub fn squad_fitness(game: &Game, team_id: &str) -> SquadFitness {
    let healthy = healthy_first_team(game, team_id);
    let status_of = |player: &&Player| fitness_status(player.condition);
    SquadFitness {
        players: healthy.len(),
        average_condition: (!healthy.is_empty()).then(|| {
            healthy.iter().map(|p| f64::from(p.condition)).sum::<f64>() / healthy.len() as f64
        }),
        exhausted_count: healthy
            .iter()
            .filter(|p| status_of(p) != FitnessStatus::Fit)
            .count(),
        critical_count: healthy
            .iter()
            .filter(|p| status_of(p) == FitnessStatus::Critical)
            .count(),
        statuses: club_players(&game.players, team_id)
            .into_iter()
            .map(|player| (player.id.clone(), fitness_status(player.condition)))
            .collect(),
    }
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
    let SquadFitness {
        average_condition: Some(avg_condition),
        exhausted_count,
        critical_count,
        ..
    } = squad_fitness(game, &user_team_id)
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
        let exhausted_names: Vec<String> = healthy_first_team(game, &user_team_id)
            .iter()
            .filter(|p| fitness_status(p.condition) == FitnessStatus::Critical)
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
