use crate::game::Game;
use crate::player_rating::{effective_rating_for_assignment, formation_slots};
use domain::player::{ContractExitIntent, Player, Position};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SquadSafetyIssue {
    TooFewHealthyPlayers,
    NoHealthyGoalkeeper,
    IncompleteFormation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SquadSafetyReport {
    pub team_id: String,
    pub projected_roster_size: usize,
    pub healthy_players: usize,
    pub healthy_goalkeepers: usize,
    pub effective_xi_size: usize,
    pub can_field_matchday_squad: bool,
    pub missing_reasons: Vec<SquadSafetyIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlannedExitSafetyReport {
    pub departing_player_ids: Vec<String>,
    pub departing_player_names: Vec<String>,
    pub squad_safety: SquadSafetyReport,
}

pub fn user_team_squad_safety(game: &Game) -> Option<SquadSafetyReport> {
    let team_id = game.manager.team_id.as_deref()?;
    evaluate_team_squad_safety(game, team_id, &HashSet::new()).ok()
}

pub fn project_user_team_release_safety(
    game: &Game,
    player_id: &str,
) -> Result<SquadSafetyReport, String> {
    let team_id = game
        .manager
        .team_id
        .as_deref()
        .ok_or("No team assigned".to_string())?;
    let player = game
        .players
        .iter()
        .find(|candidate| candidate.id == player_id)
        .ok_or("Player not found".to_string())?;

    if player.team_id.as_deref() != Some(team_id) {
        return Err("Player does not belong to your club".to_string());
    }

    let excluded_player_ids = HashSet::from([player_id.to_string()]);
    evaluate_team_squad_safety(game, team_id, &excluded_player_ids)
}

pub fn project_user_team_planned_exit_safety(game: &Game) -> Option<PlannedExitSafetyReport> {
    let team_id = game.manager.team_id.as_deref()?;
    let departing_players: Vec<&Player> = game
        .players
        .iter()
        .filter(|player| {
            player.team_id.as_deref() == Some(team_id) && has_let_expire_intent(player)
        })
        .collect();

    if departing_players.is_empty() {
        return None;
    }

    let departing_player_ids: Vec<String> = departing_players
        .iter()
        .map(|player| player.id.clone())
        .collect();
    let departing_player_names: Vec<String> = departing_players
        .iter()
        .map(|player| player.match_name.clone())
        .collect();
    let excluded_player_ids = departing_player_ids.iter().cloned().collect();
    let squad_safety = evaluate_team_squad_safety(game, team_id, &excluded_player_ids).ok()?;

    Some(PlannedExitSafetyReport {
        departing_player_ids,
        departing_player_names,
        squad_safety,
    })
}

pub fn evaluate_team_squad_safety(
    game: &Game,
    team_id: &str,
    excluded_player_ids: &HashSet<String>,
) -> Result<SquadSafetyReport, String> {
    let team = game
        .teams
        .iter()
        .find(|candidate| candidate.id == team_id)
        .ok_or("Team not found".to_string())?;
    let roster: Vec<&Player> = game
        .players
        .iter()
        .filter(|player| {
            player.team_id.as_deref() == Some(team_id) && !excluded_player_ids.contains(&player.id)
        })
        .collect();
    let healthy_roster: Vec<&Player> = roster
        .iter()
        .copied()
        .filter(|player| player.injury.is_none())
        .collect();
    let healthy_goalkeepers = healthy_roster
        .iter()
        .filter(|player| is_goalkeeper(player))
        .count();
    let effective_xi_size = build_formation_xi_size(&healthy_roster, &team.formation);
    let mut missing_reasons = Vec::new();

    if healthy_roster.len() < 11 {
        missing_reasons.push(SquadSafetyIssue::TooFewHealthyPlayers);
    }

    if healthy_goalkeepers == 0 {
        missing_reasons.push(SquadSafetyIssue::NoHealthyGoalkeeper);
    }

    if effective_xi_size < 11 {
        missing_reasons.push(SquadSafetyIssue::IncompleteFormation);
    }

    Ok(SquadSafetyReport {
        team_id: team.id.clone(),
        projected_roster_size: roster.len(),
        healthy_players: healthy_roster.len(),
        healthy_goalkeepers,
        effective_xi_size,
        can_field_matchday_squad: missing_reasons.is_empty(),
        missing_reasons,
    })
}

fn build_formation_xi_size(healthy_roster: &[&Player], formation: &str) -> usize {
    let mut used = HashSet::new();
    let mut xi_size = 0;

    for slot in formation_slots(formation).iter().take(11) {
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

        used.insert(player.id.clone());
        xi_size += 1;
    }

    xi_size
}

fn is_goalkeeper(player: &Player) -> bool {
    player.position == Position::Goalkeeper || player.natural_position == Position::Goalkeeper
}

fn has_let_expire_intent(player: &Player) -> bool {
    player
        .morale_core
        .renewal_state
        .as_ref()
        .and_then(|state| state.exit_intent.as_ref())
        .is_some_and(|intent| matches!(intent, ContractExitIntent::LetExpire { .. }))
}
