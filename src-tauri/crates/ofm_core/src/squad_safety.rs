use crate::game::Game;
use crate::player_rating::{effective_rating_for_assignment, formation_slots};
use domain::player::{ContractExitIntent, Player, Position};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const NO_TEAM_ASSIGNED_ERROR: &str = "be.error.noTeamAssigned";
const PLAYER_NOT_FOUND_ERROR: &str = "be.error.playerNotFound";
const PLAYER_NOT_IN_CLUB_ERROR: &str = "be.error.playerNotInClub";
const TEAM_NOT_FOUND_ERROR: &str = "be.error.teamNotFound";

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
        .ok_or(NO_TEAM_ASSIGNED_ERROR.to_string())?;
    let player = game
        .players
        .iter()
        .find(|candidate| candidate.id == player_id)
        .ok_or(PLAYER_NOT_FOUND_ERROR.to_string())?;

    if player.team_id.as_deref() != Some(team_id) {
        return Err(PLAYER_NOT_IN_CLUB_ERROR.to_string());
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
        .ok_or(TEAM_NOT_FOUND_ERROR.to_string())?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::player::PlayerAttributes;
    use domain::team::Team;

    fn default_attrs() -> PlayerAttributes {
        PlayerAttributes {
            pace: 60,
            stamina: 60,
            strength: 60,
            agility: 60,
            passing: 60,
            shooting: 60,
            tackling: 60,
            dribbling: 60,
            defending: 60,
            positioning: 60,
            vision: 60,
            decisions: 60,
            composure: 60,
            aggression: 60,
            teamwork: 60,
            leadership: 60,
            handling: 30,
            reflexes: 30,
            aerial: 60,
        }
    }

    fn make_game() -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 11, 1, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "mgr1".to_string(),
            "Alex".to_string(),
            "Boss".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("team-1".to_string());

        let mut team = Team::new(
            "team-1".to_string(),
            "Old FC".to_string(),
            "OLD".to_string(),
            "England".to_string(),
            "Oldville".to_string(),
            "Old Ground".to_string(),
            20_000,
        );
        team.manager_id = Some("mgr1".to_string());

        let mut player = Player::new(
            "player-1".to_string(),
            "J. Smith".to_string(),
            "John Smith".to_string(),
            "2000-01-01".to_string(),
            "England".to_string(),
            Position::Forward,
            default_attrs(),
        );
        player.team_id = Some("team-2".to_string());

        Game::new(clock, manager, vec![team], vec![player], vec![], vec![])
    }

    #[test]
    fn project_user_team_release_safety_returns_backend_key_for_other_club_player() {
        let game = make_game();

        let result = project_user_team_release_safety(&game, "player-1");

        assert_eq!(result.unwrap_err(), PLAYER_NOT_IN_CLUB_ERROR);
    }
}
