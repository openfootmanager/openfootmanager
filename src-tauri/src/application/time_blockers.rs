use log::info;

use ofm_core::contracts::contract_warning_stage;
use ofm_core::game::Game;
use ofm_core::player_rating::{effective_rating_for_assignment, formation_slots, natural_ovr};
use ofm_core::squad_safety::{project_user_team_planned_exit_safety, user_team_squad_safety};

fn user_team_context<'a>(
    game: &'a Game,
) -> Option<(&'a domain::team::Team, Vec<&'a domain::player::Player>)> {
    let user_team_id = game.manager.team_id.as_deref()?;
    let team = game.teams.iter().find(|team| team.id == user_team_id)?;
    let roster = game
        .players
        .iter()
        .filter(|player| player.team_id.as_deref() == Some(user_team_id))
        .collect();

    Some((team, roster))
}

fn build_blocker(
    id: &str,
    severity: &str,
    text: String,
    tab: &str,
    text_key: Option<&str>,
    text_params: Option<serde_json::Value>,
) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "severity": severity,
        "text": text,
        "text_key": text_key,
        "text_params": text_params,
        "tab": tab
    })
}

fn build_effective_healthy_starting_xi_ids(
    saved_xi_ids: &[String],
    roster: &[&domain::player::Player],
    formation: &str,
) -> Vec<String> {
    let healthy_roster: Vec<&domain::player::Player> = roster
        .iter()
        .copied()
        .filter(|player| player.injury.is_none())
        .collect();
    let by_id: std::collections::HashMap<&str, &domain::player::Player> = healthy_roster
        .iter()
        .map(|player| (player.id.as_str(), *player))
        .collect();
    let mut used = std::collections::HashSet::new();
    let mut valid_saved_ids = Vec::new();

    for id in saved_xi_ids {
        if by_id.contains_key(id.as_str()) && used.insert(id.clone()) {
            valid_saved_ids.push(id.clone());
        }
    }

    let mut remaining_players: Vec<&domain::player::Player> = healthy_roster
        .iter()
        .copied()
        .filter(|player| !used.contains(&player.id))
        .collect();
    remaining_players.sort_by(|left, right| {
        natural_ovr(right)
            .partial_cmp(&natural_ovr(left))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let slots = formation_slots(formation);

    if valid_saved_ids.len() >= 8 {
        let mut xi_ids = valid_saved_ids;
        while xi_ids.len() < 11 {
            let slot = slots.get(xi_ids.len());
            let best_index = remaining_players
                .iter()
                .enumerate()
                .max_by(|(_, left), (_, right)| {
                    let left_rating = slot.map_or_else(
                        || natural_ovr(left),
                        |slot| effective_rating_for_assignment(left, slot),
                    );
                    let right_rating = slot.map_or_else(
                        || natural_ovr(right),
                        |slot| effective_rating_for_assignment(right, slot),
                    );
                    left_rating
                        .partial_cmp(&right_rating)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(index, _)| index);

            let Some(best_index) = best_index else {
                break;
            };

            let player = remaining_players.remove(best_index);
            xi_ids.push(player.id.clone());
        }
        xi_ids.truncate(11);
        return xi_ids;
    }

    let mut xi_ids = Vec::new();

    for slot in slots.iter().take(11) {
        let best_player = healthy_roster
            .iter()
            .copied()
            .filter(|player| !used.contains(&player.id))
            .max_by(|left, right| {
                effective_rating_for_assignment(left, slot)
                    .partial_cmp(&effective_rating_for_assignment(right, slot))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

        let Some(player) = best_player else {
            break;
        };

        if used.insert(player.id.clone()) {
            xi_ids.push(player.id.clone());
        }
    }

    xi_ids
}

fn injured_starting_xi_blocker(
    xi_ids: &[String],
    roster: &[&domain::player::Player],
) -> Option<serde_json::Value> {
    let injured_in_xi: Vec<_> = xi_ids
        .iter()
        .filter_map(|id| {
            roster
                .iter()
                .find(|player| player.id == *id && player.injury.is_some())
        })
        .map(|player| player.match_name.clone())
        .collect();

    (!injured_in_xi.is_empty()).then(|| {
        let injured_count = injured_in_xi.len().to_string();
        let injured_players = injured_in_xi.join(", ");
        build_blocker(
            "injured_xi",
            "warn",
            format!(
                "{} injured player(s) in Starting XI: {}",
                injured_count, injured_players
            ),
            "Squad",
            Some("notifications.blockers.injuredXi"),
            Some(serde_json::json!({
                "count": injured_count,
                "players": injured_players,
            })),
        )
    })
}

fn incomplete_starting_xi_blocker(
    effective_healthy_xi_ids: &[String],
    roster: &[&domain::player::Player],
) -> Option<serde_json::Value> {
    let healthy_xi = effective_healthy_xi_ids.len();

    (healthy_xi < 11 && roster.len() >= 11).then(|| {
        let healthy_count = healthy_xi.to_string();
        build_blocker(
            "incomplete_xi",
            "warn",
            format!(
                "Starting XI has only {} healthy players — set your lineup",
                healthy_count
            ),
            "Squad",
            Some("notifications.blockers.incompleteXi"),
            Some(serde_json::json!({
                "count": healthy_count,
            })),
        )
    })
}

fn squad_size_crisis_blocker(game: &Game) -> Option<serde_json::Value> {
    let safety = user_team_squad_safety(game)?;

    (safety.projected_roster_size < 11).then(|| {
        let roster_size = safety.projected_roster_size.to_string();
        build_blocker(
            "squad_size_crisis",
            "warn",
            format!(
                "Squad has only {} contracted player(s) — sign players before match day",
                roster_size
            ),
            "Squad",
            Some("notifications.blockers.squadSizeCrisis"),
            Some(serde_json::json!({
                "count": roster_size,
            })),
        )
    })
}

fn planned_contract_exit_crisis_blocker(game: &Game) -> Option<serde_json::Value> {
    let planned_exit_report = project_user_team_planned_exit_safety(game)?;

    (!planned_exit_report.squad_safety.can_field_matchday_squad).then(|| {
        let mut names = planned_exit_report.departing_player_names;
        names.sort();
        let listed_names = names.into_iter().take(3).collect::<Vec<_>>().join(", ");
        let healthy_players = planned_exit_report.squad_safety.healthy_players.to_string();
        let healthy_goalkeepers = planned_exit_report.squad_safety.healthy_goalkeepers.to_string();
        build_blocker(
            "planned_contract_exit_crisis",
            "warn",
            format!(
                "Planned contract exits would leave only {} healthy player(s) and {} goalkeeper(s): {}",
                healthy_players,
                healthy_goalkeepers,
                listed_names
            ),
            "Squad",
            Some("notifications.blockers.plannedContractExitCrisis"),
            Some(serde_json::json!({
                "healthyPlayers": healthy_players,
                "goalkeepers": healthy_goalkeepers,
                "players": listed_names,
            })),
        )
    })
}

fn urgent_unread_messages_blocker(game: &Game) -> Option<serde_json::Value> {
    let urgent_unread = game
        .messages
        .iter()
        .filter(|message| {
            !message.read && message.priority == domain::message::MessagePriority::Urgent
        })
        .count();

    (urgent_unread > 0).then(|| {
        let unread_count = urgent_unread.to_string();
        build_blocker(
            "urgent_messages",
            "info",
            format!("{} urgent unread message(s)", unread_count),
            "Inbox",
            Some("notifications.blockers.urgentMessages"),
            Some(serde_json::json!({
                "count": unread_count,
            })),
        )
    })
}

fn key_contract_risk_blocker(
    roster: &[&domain::player::Player],
    effective_healthy_xi_ids: &[String],
    current_date: chrono::NaiveDate,
) -> Option<serde_json::Value> {
    let effective_xi_id_set: std::collections::HashSet<&str> = effective_healthy_xi_ids
        .iter()
        .map(String::as_str)
        .collect();

    let mut effective_xi_players: Vec<&domain::player::Player> = roster
        .iter()
        .copied()
        .filter(|player| effective_xi_id_set.contains(player.id.as_str()))
        .collect();
    effective_xi_players.sort_by(|left, right| {
        natural_ovr(right)
            .partial_cmp(&natural_ovr(left))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let risky_key_players: Vec<&str> = effective_xi_players
        .into_iter()
        .take(3)
        .filter(|player| {
            contract_warning_stage(player.contract_end.as_deref(), current_date).is_some()
        })
        .map(|player| player.match_name.as_str())
        .collect();

    (!risky_key_players.is_empty()).then(|| {
        let players = risky_key_players.join(", ");
        build_blocker(
            "key_contract_risk",
            "warn",
            format!("Key player contract risk in squad planning: {}", players),
            "Squad",
            Some("notifications.blockers.keyContractRisk"),
            Some(serde_json::json!({
                "players": players,
            })),
        )
    })
}

fn contract_wage_risk_blocker(
    team: &domain::team::Team,
    roster: &[&domain::player::Player],
    current_date: chrono::NaiveDate,
) -> Option<serde_json::Value> {
    let at_risk_wages: u32 = roster
        .iter()
        .copied()
        .filter(|player| {
            contract_warning_stage(player.contract_end.as_deref(), current_date).is_some()
        })
        .map(|player| player.wage)
        .sum();

    let wage_budget = team.wage_budget.max(0) as u32;
    (wage_budget > 0 && at_risk_wages > wage_budget).then(|| {
        let at_risk_wages_text = at_risk_wages.to_string();
        build_blocker(
            "contract_wage_risk",
            "warn",
            format!(
                "{} of wages are tied to at-risk contracts — review your wage budget",
                at_risk_wages_text
            ),
            "Finances",
            Some("notifications.blockers.contractWageRisk"),
            Some(serde_json::json!({
                "amount": at_risk_wages_text,
            })),
        )
    })
}

pub fn compute_blocking_actions(game: &Game) -> Vec<serde_json::Value> {
    let mut blockers = Vec::new();
    let (team, roster) = match user_team_context(game) {
        Some(context) => context,
        None => {
            info!("[cmd] compute_blocking_actions: no user team context");
            return blockers;
        }
    };
    let saved_xi_ids = &team.starting_xi_ids;
    let current_date = game.clock.current_date.date_naive();
    let effective_healthy_xi_ids =
        build_effective_healthy_starting_xi_ids(saved_xi_ids, &roster, &team.formation);

    if let Some(blocker) = injured_starting_xi_blocker(saved_xi_ids, &roster) {
        blockers.push(blocker);
    }

    if let Some(blocker) = incomplete_starting_xi_blocker(&effective_healthy_xi_ids, &roster) {
        blockers.push(blocker);
    }

    if let Some(blocker) = squad_size_crisis_blocker(game) {
        blockers.push(blocker);
    }

    if let Some(blocker) = planned_contract_exit_crisis_blocker(game) {
        blockers.push(blocker);
    }

    if let Some(blocker) =
        key_contract_risk_blocker(&roster, &effective_healthy_xi_ids, current_date)
    {
        blockers.push(blocker);
    }

    if let Some(blocker) = contract_wage_risk_blocker(team, &roster, current_date) {
        blockers.push(blocker);
    }

    if let Some(blocker) = urgent_unread_messages_blocker(game) {
        blockers.push(blocker);
    }

    let blocker_ids: Vec<String> = blockers
        .iter()
        .filter_map(|blocker| blocker.get("id").and_then(|id| id.as_str()))
        .map(|id| id.to_string())
        .collect();

    info!(
        "[cmd] compute_blocking_actions: date={}, team={}, roster={}, xi={}, blockers={:?}",
        game.clock.current_date.format("%Y-%m-%d"),
        team.id,
        roster.len(),
        effective_healthy_xi_ids.len(),
        blocker_ids
    );

    blockers
}
