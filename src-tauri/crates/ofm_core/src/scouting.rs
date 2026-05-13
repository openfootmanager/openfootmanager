use crate::game::{
    Game, ScoutingAssignment, YouthScoutingAssignment, YouthScoutingObjective, YouthScoutingRegion,
};
use domain::message::*;
use domain::player::{Player, Position, SquadRole};
use domain::staff::StaffRole;
use rand::RngExt;
use std::collections::HashMap;
use uuid::Uuid;

fn params(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// Scouts can only handle one assignment at a time across player and youth scouting.
pub fn scout_max_assignments(judging_ability: u8) -> usize {
    let _ = judging_ability;
    1
}

fn scout_assignment_count(game: &Game, scout_id: &str) -> usize {
    game.scouting_assignments
        .iter()
        .filter(|assignment| assignment.scout_id == scout_id)
        .count()
        + game
            .youth_scouting_assignments
            .iter()
            .filter(|assignment| assignment.scout_id == scout_id)
            .count()
}

fn resolve_user_scout<'a>(
    game: &'a Game,
    scout_id: &str,
) -> Result<&'a domain::staff::Staff, String> {
    let user_team_id = game.manager.team_id.as_ref().ok_or("No team")?;

    let scout = game
        .staff
        .iter()
        .find(|staff_member| staff_member.id == scout_id)
        .ok_or("Scout not found")?;
    if scout.role != StaffRole::Scout {
        return Err("Staff member is not a scout".to_string());
    }
    if scout.team_id.as_ref() != Some(user_team_id) {
        return Err("Scout does not belong to your team".to_string());
    }

    Ok(scout)
}

fn assignment_days_for_player_scouting(judging_ability: u8) -> u32 {
    if judging_ability >= 80 {
        2
    } else if judging_ability >= 60 {
        3
    } else if judging_ability >= 40 {
        4
    } else {
        5
    }
}

fn assignment_days_for_youth_scouting(
    judging_potential: u8,
    region: YouthScoutingRegion,
    objective: YouthScoutingObjective,
) -> u32 {
    let base = if judging_potential >= 80 {
        4
    } else if judging_potential >= 60 {
        5
    } else if judging_potential >= 40 {
        6
    } else {
        7
    };

    let region_modifier = match region {
        YouthScoutingRegion::Domestic => 0,
        YouthScoutingRegion::International => 1,
    };
    let objective_modifier = match objective {
        YouthScoutingObjective::Balanced => 0,
        YouthScoutingObjective::HighPotential => 1,
        YouthScoutingObjective::ReadySoon => 0,
    };

    base + region_modifier + objective_modifier
}

/// Send a scout to evaluate a player. Returns an error string if invalid.
pub fn send_scout(game: &mut Game, scout_id: &str, player_id: &str) -> Result<(), String> {
    let user_team_id = game.manager.team_id.as_ref().ok_or("No team")?;
    let scout = resolve_user_scout(game, scout_id)?;

    // Validate player exists and is not on user's team
    let player = game
        .players
        .iter()
        .find(|p| p.id == player_id)
        .ok_or("Player not found")?;
    if player.team_id.as_deref() == Some(user_team_id.as_str()) {
        return Err("Cannot scout your own players".to_string());
    }

    // Check scout capacity across both player and youth scouting.
    let max_slots = scout_max_assignments(scout.attributes.judging_ability);
    let current_count = scout_assignment_count(game, scout_id);
    if current_count >= max_slots {
        return Err(format!(
            "Scout is already assigned to another scouting task ({}/{} assignments).",
            current_count, max_slots
        ));
    }

    // Check if player is already being scouted
    if game
        .scouting_assignments
        .iter()
        .any(|a| a.player_id == player_id)
    {
        return Err("This player is already being scouted".to_string());
    }

    // Create assignment (2-5 days depending on scout quality)
    let days = assignment_days_for_player_scouting(scout.attributes.judging_ability);

    game.scouting_assignments.push(ScoutingAssignment {
        id: Uuid::new_v4().to_string(),
        scout_id: scout_id.to_string(),
        player_id: player_id.to_string(),
        days_remaining: days,
    });

    Ok(())
}

pub fn start_youth_scouting(
    game: &mut Game,
    scout_id: &str,
    region: YouthScoutingRegion,
    objective: YouthScoutingObjective,
    target_position: Option<Position>,
) -> Result<(), String> {
    let scout = resolve_user_scout(game, scout_id)?;
    let max_slots = scout_max_assignments(scout.attributes.judging_ability);
    let current_count = scout_assignment_count(game, scout_id);
    if current_count >= max_slots {
        return Err(format!(
            "Scout is already assigned to another scouting task ({}/{} assignments).",
            current_count, max_slots
        ));
    }

    let target_position = target_position.map(|position| position.to_group_position());
    if game.youth_scouting_assignments.iter().any(|assignment| {
        assignment.region == region
            && assignment.objective == objective
            && assignment.target_position == target_position
    }) {
        return Err("A matching youth recruitment search is already active".to_string());
    }

    let days =
        assignment_days_for_youth_scouting(scout.attributes.judging_potential, region, objective);
    game.youth_scouting_assignments
        .push(YouthScoutingAssignment {
            id: Uuid::new_v4().to_string(),
            scout_id: scout_id.to_string(),
            region,
            objective,
            target_position,
            days_remaining: days,
        });

    Ok(())
}

pub fn cancel_youth_scouting(game: &mut Game, assignment_id: &str) -> Result<(), String> {
    let original_len = game.youth_scouting_assignments.len();
    game.youth_scouting_assignments
        .retain(|assignment| assignment.id != assignment_id);

    if game.youth_scouting_assignments.len() == original_len {
        return Err("Youth recruitment assignment not found".to_string());
    }

    Ok(())
}

pub fn reassign_youth_scouting(
    game: &mut Game,
    assignment_id: &str,
    scout_id: &str,
) -> Result<(), String> {
    let assignment_index = game
        .youth_scouting_assignments
        .iter()
        .position(|assignment| assignment.id == assignment_id)
        .ok_or("Youth recruitment assignment not found")?;
    let current_scout_id = game.youth_scouting_assignments[assignment_index]
        .scout_id
        .clone();
    if current_scout_id == scout_id {
        return Err("Scout is already assigned to this search".to_string());
    }

    let scout = resolve_user_scout(game, scout_id)?;
    let max_slots = scout_max_assignments(scout.attributes.judging_ability);
    let current_count = scout_assignment_count(game, scout_id);
    if current_count >= max_slots {
        return Err(format!(
            "Scout is already assigned to another scouting task ({}/{} assignments).",
            current_count, max_slots
        ));
    }

    game.youth_scouting_assignments[assignment_index].scout_id = scout_id.to_string();
    Ok(())
}

/// Process scouting assignments daily. Called from process_day().
/// Decrements days, delivers reports when complete.
pub fn process_scouting(game: &mut Game) {
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let mut completed: Vec<ScoutingAssignment> = Vec::new();
    let mut completed_youth: Vec<YouthScoutingAssignment> = Vec::new();

    for assignment in game.scouting_assignments.iter_mut() {
        if assignment.days_remaining > 0 {
            assignment.days_remaining -= 1;
        }
        if assignment.days_remaining == 0 {
            completed.push(assignment.clone());
        }
    }

    for assignment in game.youth_scouting_assignments.iter_mut() {
        if assignment.days_remaining > 0 {
            assignment.days_remaining -= 1;
        }
        if assignment.days_remaining == 0 {
            completed_youth.push(assignment.clone());
        }
    }

    // Remove completed assignments
    game.scouting_assignments.retain(|a| a.days_remaining > 0);
    game.youth_scouting_assignments
        .retain(|assignment| assignment.days_remaining > 0);

    // Generate reports for completed assignments
    for assignment in &completed {
        let scout = game.staff.iter().find(|s| s.id == assignment.scout_id);
        let player = game.players.iter().find(|p| p.id == assignment.player_id);

        if let (Some(scout), Some(player)) = (scout, player) {
            let scout_name = format!("{} {}", scout.first_name, scout.last_name);
            let judging_ability = scout.attributes.judging_ability;
            let judging_potential = scout.attributes.judging_potential;
            let team_name = player
                .team_id
                .as_ref()
                .and_then(|tid| game.teams.iter().find(|t| &t.id == tid))
                .map(|t| t.name.clone());

            let msg = build_scout_report(
                &assignment.id,
                &scout_name,
                &player.id,
                &player.match_name,
                &player.nationality,
                &player.date_of_birth,
                &format!("{:?}", player.position),
                &player.attributes,
                player.morale,
                player.condition,
                player.ovr,
                player.potential,
                judging_ability,
                judging_potential,
                team_name.as_deref(),
                &today,
            );
            game.messages.push(msg);
        }
    }

    for assignment in &completed_youth {
        complete_youth_scouting_assignment(game, assignment, &today);
    }
}

fn complete_youth_scouting_assignment(
    game: &mut Game,
    assignment: &YouthScoutingAssignment,
    date: &str,
) {
    let Some(scout) = game
        .staff
        .iter()
        .find(|staff_member| staff_member.id == assignment.scout_id)
        .cloned()
    else {
        return;
    };
    let Some(user_team_id) = game.manager.team_id.clone() else {
        return;
    };
    let Some(team) = game
        .teams
        .iter()
        .find(|candidate| candidate.id == user_team_id)
        .cloned()
    else {
        return;
    };

    let prospects = generate_youth_recruitment_candidates(
        &team,
        assignment.region,
        assignment.objective,
        assignment.target_position.as_ref(),
    );
    if prospects.is_empty() {
        return;
    }

    let scout_name = format!("{} {}", scout.first_name, scout.last_name);
    game.messages.push(build_youth_recruitment_report(
        &assignment.id,
        &scout_name,
        &team.id,
        &team.name,
        &prospects,
        assignment.region,
        assignment.objective,
        assignment.target_position.as_ref(),
        date,
    ));
}

fn build_youth_recruitment_report(
    assignment_id: &str,
    scout_name: &str,
    team_id: &str,
    team_name: &str,
    prospects: &[Player],
    region: YouthScoutingRegion,
    objective: YouthScoutingObjective,
    target_position: Option<&Position>,
    date: &str,
) -> InboxMessage {
    let target_position = target_position.map(|position| position.to_group_position());
    let body = match target_position.as_ref() {
        Some(target_position) => format!(
            "{} has returned {} youth prospects for {} after a {} {} search focused on {}.",
            scout_name,
            prospects.len(),
            team_name,
            region_label(region),
            objective_label(objective),
            format!("{:?}", target_position).to_lowercase(),
        ),
        None => format!(
            "{} has returned {} youth prospects for {} after a {} {} search.",
            scout_name,
            prospects.len(),
            team_name,
            region_label(region),
            objective_label(objective),
        ),
    };

    let message = InboxMessage::new(
        format!("youth-scout-{}", assignment_id),
        "Youth prospect found".to_string(),
        body,
        scout_name.to_string(),
        date.to_string(),
    )
    .with_category(MessageCategory::ScoutReport)
    .with_sender_role("Scout");

    let message = prospects.iter().fold(message, |message, prospect| {
        message.with_action(MessageAction {
            id: format!("prospect:{}", prospect.id),
            label: prospect.full_name.clone(),
            action_type: ActionType::ChooseOption {
                options: youth_prospect_options(),
            },
            resolved: false,
            label_key: None,
        })
    });

    message.with_context(MessageContext {
        team_id: Some(team_id.to_string()),
        youth_target_position: target_position.map(|position| format!("{:?}", position)),
        youth_search_region: Some(format!("{:?}", region)),
        youth_search_objective: Some(format!("{:?}", objective)),
        youth_prospects: Some(prospects.to_vec()),
        ..MessageContext::default()
    })
}

fn youth_prospect_options() -> Vec<ActionOption> {
    vec![
        ActionOption {
            id: "sign".to_string(),
            label: "Sign to academy".to_string(),
            description: "Add this player directly to your youth academy now.".to_string(),
            label_key: None,
            description_key: None,
        },
        ActionOption {
            id: "shortlist".to_string(),
            label: "Shortlist".to_string(),
            description: "Keep this player for a later decision without committing now."
                .to_string(),
            label_key: None,
            description_key: None,
        },
        ActionOption {
            id: "discard".to_string(),
            label: "Discard".to_string(),
            description: "Remove this prospect from the report.".to_string(),
            label_key: None,
            description_key: None,
        },
    ]
}

fn region_label(region: YouthScoutingRegion) -> &'static str {
    match region {
        YouthScoutingRegion::Domestic => "domestic",
        YouthScoutingRegion::International => "international",
    }
}

fn objective_label(objective: YouthScoutingObjective) -> &'static str {
    match objective {
        YouthScoutingObjective::Balanced => "balanced",
        YouthScoutingObjective::HighPotential => "high-potential",
        YouthScoutingObjective::ReadySoon => "ready-now",
    }
}

fn prospect_score(player: &Player, objective: YouthScoutingObjective) -> (u8, u8) {
    match objective {
        YouthScoutingObjective::Balanced => (
            player.ovr.saturating_add(player.potential / 2),
            player.potential,
        ),
        YouthScoutingObjective::HighPotential => (player.potential, player.ovr),
        YouthScoutingObjective::ReadySoon => (player.ovr, player.potential),
    }
}

fn generate_youth_recruitment_candidates(
    team: &domain::team::Team,
    region: YouthScoutingRegion,
    objective: YouthScoutingObjective,
    target_position: Option<&Position>,
) -> Vec<Player> {
    let pool_size = match objective {
        YouthScoutingObjective::Balanced => 4,
        YouthScoutingObjective::HighPotential => 6,
        YouthScoutingObjective::ReadySoon => 6,
    };
    let domestic_nationality = if team.football_nation.is_empty() {
        Some(team.country.as_str())
    } else {
        Some(team.football_nation.as_str())
    };

    let mut prospects: Vec<Player> = (0..pool_size)
        .map(|_| {
            let mut prospect = crate::generator::generate_youth_academy_recruit_with_nationality(
                team,
                target_position,
                match region {
                    YouthScoutingRegion::Domestic => domestic_nationality,
                    YouthScoutingRegion::International => None,
                },
            );
            prospect.team_id = None;
            prospect.squad_role = SquadRole::Youth;
            prospect
        })
        .collect();

    prospects.sort_by(|left, right| {
        prospect_score(right, objective).cmp(&prospect_score(left, objective))
    });
    prospects.truncate(3);
    prospects
}

pub struct YouthRecruitmentEffect {
    pub message: String,
}

pub fn apply_youth_recruitment_response(
    game: &mut Game,
    message_id: &str,
    action_id: &str,
    option_id: &str,
) -> Option<YouthRecruitmentEffect> {
    let message_index = game
        .messages
        .iter()
        .position(|message| message.id == message_id)?;
    let action_index = game.messages[message_index]
        .actions
        .iter()
        .position(|action| action.id == action_id)?;
    let prospect_id = action_id.strip_prefix("prospect:")?;

    let prospects = game.messages[message_index]
        .context
        .youth_prospects
        .clone()?;
    let prospect_index = prospects
        .iter()
        .position(|prospect| prospect.id == prospect_id)?;
    let prospect = prospects[prospect_index].clone();

    match option_id {
        "discard" => {
            let mut remaining = prospects;
            remaining.remove(prospect_index);
            let message = &mut game.messages[message_index];
            message.context.youth_prospects = Some(remaining);
            message.actions.remove(action_index);

            Some(YouthRecruitmentEffect {
                message: format!("{} removed from the youth report.", prospect.full_name),
            })
        }
        "sign" => {
            let mut signed_player = prospect;
            signed_player.team_id = game.manager.team_id.clone();
            signed_player.squad_role = SquadRole::Youth;
            let player_id = signed_player.id.clone();
            let player_name = signed_player.full_name.clone();
            game.players.push(signed_player);

            let message = &mut game.messages[message_index];
            message.context.player_id = Some(player_id);
            message.context.youth_prospects = None;
            for action in &mut message.actions {
                action.resolved = true;
            }

            Some(YouthRecruitmentEffect {
                message: format!("{} joined your youth academy.", player_name),
            })
        }
        "shortlist" => {
            let sender = game.messages[message_index].sender.clone();
            let date = game.messages[message_index].date.clone();
            let team_id = game.messages[message_index].context.team_id.clone();
            let youth_target_position = game.messages[message_index]
                .context
                .youth_target_position
                .clone();
            let youth_search_region = game.messages[message_index]
                .context
                .youth_search_region
                .clone();
            let youth_search_objective = game.messages[message_index]
                .context
                .youth_search_objective
                .clone();
            let prospect_name = prospect.full_name.clone();
            let shortlist_options = youth_prospect_options();

            game.messages.push(
                InboxMessage::new(
                    format!("youth-shortlist-{}", prospect.id),
                    "Youth prospect shortlisted".to_string(),
                    format!(
                        "{} has been kept on your shortlist for a later academy decision.",
                        prospect_name
                    ),
                    sender,
                    date,
                )
                .with_category(MessageCategory::ScoutReport)
                .with_sender_role("Scout")
                .with_action(MessageAction {
                    id: format!("prospect:{}", prospect.id),
                    label: prospect.full_name.clone(),
                    action_type: ActionType::ChooseOption {
                        options: vec![shortlist_options[0].clone(), shortlist_options[2].clone()],
                    },
                    resolved: false,
                    label_key: None,
                })
                .with_context(MessageContext {
                    team_id,
                    youth_target_position,
                    youth_search_region,
                    youth_search_objective,
                    youth_prospects: Some(vec![prospect]),
                    ..MessageContext::default()
                }),
            );

            let message = &mut game.messages[message_index];
            message.context.youth_prospects = None;
            for action in &mut message.actions {
                action.resolved = true;
            }

            Some(YouthRecruitmentEffect {
                message: format!("{} added to your youth shortlist.", prospect_name),
            })
        }
        _ => None,
    }
}

fn build_scout_report(
    assignment_id: &str,
    scout_name: &str,
    player_id: &str,
    player_name: &str,
    nationality: &str,
    dob: &str,
    position: &str,
    attrs: &domain::player::PlayerAttributes,
    morale: u8,
    condition: u8,
    player_ovr: u8,
    player_potential: u8,
    judging_ability: u8,
    judging_potential: u8,
    team_name: Option<&str>,
    date: &str,
) -> InboxMessage {
    let mut rng = rand::rng();

    // Accuracy: higher judging = less noise on reported attributes
    let noise_range = if judging_ability >= 80 {
        2
    } else if judging_ability >= 60 {
        5
    } else if judging_ability >= 40 {
        8
    } else {
        12
    };

    let mut fuzz = |val: u8| -> u8 {
        let delta: i16 = rng.random_range(-(noise_range as i16)..=(noise_range as i16));
        ((val as i16) + delta).clamp(1, 99) as u8
    };

    // Build fuzzed attribute values
    let all_fuzzed: [(u8, &str); 6] = [
        (fuzz(attrs.pace), "Pace"),
        (fuzz(attrs.shooting), "Shooting"),
        (fuzz(attrs.passing), "Passing"),
        (fuzz(attrs.dribbling), "Dribbling"),
        (fuzz(attrs.defending), "Defending"),
        (fuzz(attrs.strength), "Physical"),
    ];

    // Discovery mechanic: scout ability determines how many attrs are revealed
    // 80+: all 6 attrs + condition + morale
    // 60-79: 5 attrs + condition
    // 40-59: 3 attrs
    // <40: 2 attrs
    let reveal_count: usize = if judging_ability >= 80 {
        6
    } else if judging_ability >= 60 {
        5
    } else if judging_ability >= 40 {
        3
    } else {
        2
    };

    // Shuffle indices to determine which attrs are hidden
    let mut indices: Vec<usize> = (0..6).collect();
    for i in (1..indices.len()).rev() {
        let j = rng.random_range(0..=i);
        indices.swap(i, j);
    }
    let revealed: std::collections::HashSet<usize> =
        indices[..reveal_count].iter().cloned().collect();

    let to_opt = |idx: usize| -> Option<u8> {
        if revealed.contains(&idx) {
            Some(all_fuzzed[idx].0)
        } else {
            None
        }
    };

    let pace = to_opt(0);
    let shooting = to_opt(1);
    let passing = to_opt(2);
    let dribbling = to_opt(3);
    let defending = to_opt(4);
    let physical = to_opt(5);

    let reported_condition = if judging_ability >= 60 {
        Some(condition)
    } else {
        None
    };
    let reported_morale = if judging_ability >= 80 {
        Some(morale)
    } else {
        None
    };

    // Overall rating assessment based on the player's position-weighted OVR (fuzzed by scout ability).
    // Fall back to attribute average if OVR is unavailable (legacy players).
    let rating_base = if player_ovr > 0 {
        let delta: i16 = rng.random_range(-(noise_range as i16)..=(noise_range as i16));
        ((player_ovr as i16) + delta).clamp(1, 99) as u32
    } else {
        let revealed_vals: Vec<u32> = (0..6).filter_map(|i| to_opt(i).map(|v| v as u32)).collect();
        if revealed_vals.is_empty() {
            0
        } else {
            revealed_vals.iter().sum::<u32>() / revealed_vals.len() as u32
        }
    };

    let rating_key = if rating_base >= 80 {
        "common.scoutRatings.excellent"
    } else if rating_base >= 70 {
        "common.scoutRatings.veryGood"
    } else if rating_base >= 60 {
        "common.scoutRatings.good"
    } else if rating_base >= 50 {
        "common.scoutRatings.average"
    } else {
        "common.scoutRatings.belowAverage"
    };

    // Potential assessment: use the player's actual potential (fuzzed) when the scout
    // has sufficient judging_potential skill.  High-potential scouts can also spot
    // Wonderkid-level talent accurately.
    let potential_key = if judging_potential >= 70 {
        let fuzzed_potential = if player_potential > 0 {
            let delta: i16 = rng.random_range(-(noise_range as i16)..=(noise_range as i16));
            ((player_potential as i16) + delta).clamp(1, 99) as u32
        } else {
            rating_base // fallback to fuzzed OVR if no potential stored
        };
        if fuzzed_potential >= 85 {
            "common.scoutPotential.worldClass"
        } else if fuzzed_potential >= 70 {
            "common.scoutPotential.strong"
        } else {
            "common.scoutPotential.moderate"
        }
    } else {
        "common.scoutPotential.unclear"
    };

    // Confidence level
    let confidence_key = if judging_ability >= 80 {
        "common.scoutConfidence.high"
    } else if judging_ability >= 60 {
        "common.scoutConfidence.moderate"
    } else {
        "common.scoutConfidence.low"
    };

    // Build structured report data for the player card
    let report_data = ScoutReportData {
        player_id: player_id.to_string(),
        player_name: player_name.to_string(),
        position: position.to_string(),
        nationality: nationality.to_string(),
        dob: dob.to_string(),
        team_name: team_name.map(|s| s.to_string()),
        pace,
        shooting,
        passing,
        dribbling,
        defending,
        physical,
        condition: reported_condition,
        morale: reported_morale,
        avg_rating: Some(rating_base),
        rating_key: rating_key.to_string(),
        potential_key: potential_key.to_string(),
        confidence_key: confidence_key.to_string(),
    };

    // Fallback body text (used when i18n key is not found)
    let body = format!(
        "Scout report on {} completed by {}.",
        player_name, scout_name
    );

    let msg_id = format!("scout_report_{}", assignment_id);

    InboxMessage::new(
        msg_id,
        format!("Scout Report — {}", player_name),
        body,
        scout_name.to_string(),
        date.to_string(),
    )
    .with_category(MessageCategory::ScoutReport)
    .with_priority(MessagePriority::Normal)
    .with_sender_role("Scout")
    .with_action(MessageAction {
        id: "ack".to_string(),
        label: "Noted".to_string(),
        action_type: ActionType::Acknowledge,
        resolved: false,
        label_key: Some("be.msg.event.ack".to_string()),
    })
    .with_context(MessageContext {
        player_id: Some(player_id.to_string()),
        scout_report: Some(report_data),
        ..Default::default()
    })
    .with_i18n("be.msg.scoutReport.subject", "be.msg.scoutReport.body", {
        let mut p = params(&[("player", player_name), ("scout", scout_name)]);
        p.insert("ratingDesc".to_string(), rating_key.to_string());
        p.insert("potentialDesc".to_string(), potential_key.to_string());
        p.insert("confidence".to_string(), confidence_key.to_string());
        p
    })
    .with_sender_i18n("be.sender.scout", "be.role.scout")
}
