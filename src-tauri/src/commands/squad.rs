use chrono::Datelike;
use log::info;
use std::sync::Arc;
use tauri::State;

use ofm_core::game::Game;
use ofm_core::state::StateManager;

use crate::commands::util::{mutate_active_game, user_team_mut};

fn parse_squad_role(squad_role: &str) -> Option<domain::player::SquadRole> {
    match squad_role {
        "Senior" => Some(domain::player::SquadRole::Senior),
        "Youth" => Some(domain::player::SquadRole::Youth),
        _ => None,
    }
}

fn player_age_on(current_date: chrono::NaiveDate, date_of_birth: &str) -> Option<i32> {
    let dob = chrono::NaiveDate::parse_from_str(date_of_birth, "%Y-%m-%d").ok()?;
    let mut age = current_date.year() - dob.year();

    if (current_date.month(), current_date.day()) < (dob.month(), dob.day()) {
        age -= 1;
    }

    Some(age)
}

pub fn set_formation_internal(state: &StateManager, formation: &str) -> Result<Game, String> {
    mutate_active_game(state, |game| {
        let team_id = game
            .manager
            .team_id
            .clone()
            .ok_or("be.error.noTeamAssigned".to_string())?;

        // Note: `player.position` is intentionally NOT mutated here. A player's
        // stored position is their natural position; the position they are
        // *deployed* in is derived on demand from the formation + starting XI
        // (see `player_rating::deployed_position`). The previous stat-ranked
        // bucket overwrite corrupted role validation and match simulation
        // (issue #257).
        let team = user_team_mut(game)?;
        team.formation = formation.to_string();

        reconcile_player_roles(game, &team_id);

        Ok(())
    })
}

pub fn set_starting_xi_internal(
    state: &StateManager,
    player_ids: Vec<String>,
) -> Result<Game, String> {
    mutate_active_game(state, |game| {
        let team_id = game
            .manager
            .team_id
            .clone()
            .ok_or("be.error.noTeamAssigned".to_string())?;

        let team = user_team_mut(game)?;
        team.starting_xi_ids = player_ids;

        reconcile_player_roles(game, &team_id);

        Ok(())
    })
}

#[tauri::command]
pub fn set_formation(
    state: State<'_, Arc<StateManager>>,
    formation: String,
) -> Result<Game, String> {
    info!("[cmd] set_formation: {}", formation);
    set_formation_internal(&state, &formation)
}

#[tauri::command]
pub fn set_starting_xi(
    state: State<'_, Arc<StateManager>>,
    player_ids: Vec<String>,
) -> Result<Game, String> {
    info!("[cmd] set_starting_xi: {} players", player_ids.len());
    set_starting_xi_internal(&state, player_ids)
}

#[tauri::command]
pub fn set_play_style(
    state: State<'_, Arc<StateManager>>,
    play_style: String,
) -> Result<Game, String> {
    info!("[cmd] set_play_style: {}", play_style);
    set_play_style_internal(&state, &play_style)
}

pub fn set_play_style_internal(state: &StateManager, play_style: &str) -> Result<Game, String> {
    mutate_active_game(state, |game| {
        let style = match play_style {
            "Attacking" => domain::team::PlayStyle::Attacking,
            "Defensive" => domain::team::PlayStyle::Defensive,
            "Possession" => domain::team::PlayStyle::Possession,
            "Counter" => domain::team::PlayStyle::Counter,
            "HighPress" => domain::team::PlayStyle::HighPress,
            _ => domain::team::PlayStyle::Balanced,
        };

        let team = user_team_mut(game)?;
        team.play_style = style;

        Ok(())
    })
}

#[tauri::command]
pub fn set_team_match_roles(
    state: State<'_, Arc<StateManager>>,
    match_roles: domain::team::MatchRoles,
) -> Result<Game, String> {
    info!("[cmd] set_team_match_roles");
    set_team_match_roles_internal(&state, match_roles)
}

pub fn set_team_match_roles_internal(
    state: &StateManager,
    match_roles: domain::team::MatchRoles,
) -> Result<Game, String> {
    mutate_active_game(state, |game| {
        let team = user_team_mut(game)?;
        team.match_roles = match_roles;

        Ok(())
    })
}

#[tauri::command]
pub fn set_training(
    state: State<'_, Arc<StateManager>>,
    focus: String,
    intensity: String,
) -> Result<Game, String> {
    info!(
        "[cmd] set_training: focus={}, intensity={}",
        focus, intensity
    );
    set_training_internal(&state, &focus, &intensity)
}

pub fn set_training_internal(
    state: &StateManager,
    focus: &str,
    intensity: &str,
) -> Result<Game, String> {
    mutate_active_game(state, |game| {
        let training_focus = match focus {
            "Physical" => domain::team::TrainingFocus::Physical,
            "Technical" => domain::team::TrainingFocus::Technical,
            "Tactical" => domain::team::TrainingFocus::Tactical,
            "Defending" => domain::team::TrainingFocus::Defending,
            "Attacking" => domain::team::TrainingFocus::Attacking,
            "Recovery" => domain::team::TrainingFocus::Recovery,
            _ => domain::team::TrainingFocus::Physical,
        };

        let training_intensity = match intensity {
            "Low" => domain::team::TrainingIntensity::Low,
            "Medium" => domain::team::TrainingIntensity::Medium,
            "High" => domain::team::TrainingIntensity::High,
            _ => domain::team::TrainingIntensity::Medium,
        };

        let team = user_team_mut(game)?;
        team.training_focus = training_focus;
        team.training_intensity = training_intensity;

        Ok(())
    })
}

#[tauri::command]
pub fn set_training_schedule(
    state: State<'_, Arc<StateManager>>,
    schedule: String,
) -> Result<Game, String> {
    info!("[cmd] set_training_schedule: {}", schedule);
    set_training_schedule_internal(&state, &schedule)
}

pub fn set_training_schedule_internal(
    state: &StateManager,
    schedule: &str,
) -> Result<Game, String> {
    mutate_active_game(state, |game| {
        let training_schedule = match schedule {
            "Intense" => domain::team::TrainingSchedule::Intense,
            "Balanced" => domain::team::TrainingSchedule::Balanced,
            "Light" => domain::team::TrainingSchedule::Light,
            _ => domain::team::TrainingSchedule::Balanced,
        };

        let team = user_team_mut(game)?;
        team.training_schedule = training_schedule;

        Ok(())
    })
}

#[tauri::command]
pub fn set_training_groups(
    state: State<'_, Arc<StateManager>>,
    groups: Vec<domain::team::TrainingGroup>,
) -> Result<Game, String> {
    info!("[cmd] set_training_groups: {} groups", groups.len());
    set_training_groups_internal(&state, groups)
}

pub fn set_training_groups_internal(
    state: &StateManager,
    groups: Vec<domain::team::TrainingGroup>,
) -> Result<Game, String> {
    mutate_active_game(state, |game| {
        let team = user_team_mut(game)?;
        team.training_groups = groups;

        Ok(())
    })
}

#[tauri::command]
pub fn set_player_training_focus(
    state: State<'_, Arc<StateManager>>,
    player_id: String,
    focus: Option<String>,
) -> Result<Game, String> {
    set_player_training_focus_internal(&state, &player_id, focus.as_deref())
}

pub fn set_player_training_focus_internal(
    state: &StateManager,
    player_id: &str,
    focus: Option<&str>,
) -> Result<Game, String> {
    info!(
        "[cmd] set_player_training_focus: player={}, focus={:?}",
        player_id, focus
    );
    mutate_active_game(state, |game| {
        let team_id = game
            .manager
            .team_id
            .clone()
            .ok_or("be.error.noTeamAssigned".to_string())?;

        let training_focus = focus.and_then(|f| match f {
            "Physical" => Some(domain::team::TrainingFocus::Physical),
            "Technical" => Some(domain::team::TrainingFocus::Technical),
            "Tactical" => Some(domain::team::TrainingFocus::Tactical),
            "Defending" => Some(domain::team::TrainingFocus::Defending),
            "Attacking" => Some(domain::team::TrainingFocus::Attacking),
            "Recovery" => Some(domain::team::TrainingFocus::Recovery),
            _ => None,
        });

        if let Some(player) = game
            .players
            .iter_mut()
            .find(|p| p.id == player_id && p.team_id.as_deref() == Some(team_id.as_str()))
        {
            player.training_focus = training_focus;
        } else {
            return Err("be.error.playerNotFound".to_string());
        }

        Ok(())
    })
}

#[tauri::command]
pub fn set_player_squad_role(
    state: State<'_, Arc<StateManager>>,
    player_id: String,
    squad_role: String,
) -> Result<Game, String> {
    set_player_squad_role_internal(&state, &player_id, &squad_role)
}

pub fn set_player_squad_role_internal(
    state: &StateManager,
    player_id: &str,
    squad_role: &str,
) -> Result<Game, String> {
    info!(
        "[cmd] set_player_squad_role: player={}, squad_role={}",
        player_id, squad_role
    );
    mutate_active_game(state, |game| {
        let team_id = game
            .manager
            .team_id
            .clone()
            .ok_or("be.error.noTeamAssigned".to_string())?;
        let target_role =
            parse_squad_role(squad_role).ok_or("be.error.invalidSquadRole".to_string())?;
        let current_date = game.clock.current_date.date_naive();

        let player_index = game
            .players
            .iter()
            .position(|player| player.id == player_id)
            .ok_or("be.error.playerNotFound".to_string())?;

        if game.players[player_index].team_id.as_deref() != Some(team_id.as_str()) {
            return Err("be.error.playerNotInSquad".to_string());
        }

        if matches!(target_role, domain::player::SquadRole::Youth) {
            let age = player_age_on(current_date, &game.players[player_index].date_of_birth)
                .ok_or("be.error.invalidDateOfBirth".to_string())?;
            if age > 21 {
                return Err("be.error.youthAcademyOverage".to_string());
            }
        }

        // Drop the player from the XI first. It is the only step here that can
        // fail, and mutate_active_game mutates the live game, so doing it after
        // the squad-role write would leave that write in place on an error.
        if matches!(target_role, domain::player::SquadRole::Youth) {
            user_team_mut(game)?
                .starting_xi_ids
                .retain(|id| id != player_id);
        }

        game.players[player_index].squad_role = target_role;

        Ok(())
    })
}

#[tauri::command]
pub fn auto_select_set_pieces(
    state: State<'_, Arc<StateManager>>,
    player_ids: Vec<String>,
) -> Result<serde_json::Value, String> {
    log::debug!("[cmd] auto_select_set_pieces: {} players", player_ids.len());
    auto_select_set_pieces_internal(&state, &player_ids)
}

pub fn auto_select_set_pieces_internal(
    state: &StateManager,
    player_ids: &[String],
) -> Result<serde_json::Value, String> {
    state
        .get_game(|game| {
            let (captain, penalty, free_kick, corner) =
                ofm_core::live_match_manager::auto_select_set_pieces(game, player_ids);
            serde_json::json!({
                "captain": captain,
                "penalty_taker": penalty,
                "free_kick_taker": free_kick,
                "corner_taker": corner,
            })
        })
        .ok_or_else(|| "be.error.noActiveGameSession".to_string())
}

pub fn assign_jersey_number_internal(
    state: &StateManager,
    player_id: &str,
    jersey_number: Option<u8>,
) -> Result<Game, String> {
    mutate_active_game(state, |game| {
        let team_id = game
            .manager
            .team_id
            .clone()
            .ok_or("be.error.noTeamAssigned".to_string())?;

        if let Some(n) = jersey_number {
            if !(1..=99).contains(&n) {
                return Err("be.error.jerseyNumberOutOfRange".to_string());
            }
            let conflict = game.players.iter().any(|p| {
                p.id != player_id
                    && p.team_id.as_deref() == Some(team_id.as_str())
                    && p.jersey_number == Some(n)
            });
            if conflict {
                return Err("be.error.jerseyNumberTaken".to_string());
            }
        }

        let player = game
            .players
            .iter_mut()
            .find(|p| p.id == player_id && p.team_id.as_deref() == Some(team_id.as_str()))
            .ok_or("be.error.playerNotFound".to_string())?;

        player.jersey_number = jersey_number;
        Ok(())
    })
}

#[tauri::command]
pub fn assign_jersey_number(
    state: State<'_, Arc<StateManager>>,
    player_id: String,
    jersey_number: Option<u8>,
) -> Result<Game, String> {
    info!(
        "[cmd] assign_jersey_number: player={}, number={:?}",
        player_id, jersey_number
    );
    assign_jersey_number_internal(&state, &player_id, jersey_number)
}

pub fn set_team_kit_pattern_internal(
    state: &StateManager,
    kit_pattern: domain::team::KitPattern,
) -> Result<Game, String> {
    mutate_active_game(state, |game| {
        if game.season_context.phase != domain::season::SeasonPhase::Preseason {
            return Err("be.error.kitChangesLockedInSeason".to_string());
        }

        let team_id = game
            .manager
            .team_id
            .clone()
            .ok_or("be.error.noTeamAssigned".to_string())?;

        let team = game
            .teams
            .iter_mut()
            .find(|t| t.id == team_id)
            .ok_or("be.error.teamNotFound".to_string())?;

        team.kit_pattern = kit_pattern;
        Ok(())
    })
}

#[tauri::command]
pub fn set_team_kit_pattern(
    state: State<'_, Arc<StateManager>>,
    kit_pattern: domain::team::KitPattern,
) -> Result<Game, String> {
    info!("[cmd] set_team_kit_pattern: {:?}", kit_pattern);
    set_team_kit_pattern_internal(&state, kit_pattern)
}

/// Drop any assigned player function (role) that is no longer valid for the
/// granular field position the player now occupies in the starting XI.
///
/// The backend only tracks coarse position buckets on `player.position`, so the
/// granular slot is derived from the team's formation + starting XI order (the
/// same mapping the UI uses). This keeps player functions dependent on the
/// player's *current* field position: e.g. a striker's `Poacher` is cleared
/// when they are moved into a defensive slot.
fn reconcile_player_roles(game: &mut Game, team_id: &str) {
    // Natural position of every player on the team, for validating roles of
    // players who are not in the starting XI (bench players have no slot).
    let natural_positions: std::collections::HashMap<String, domain::player::Position> = game
        .players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some(team_id))
        .map(|p| (p.id.clone(), p.natural_position.clone()))
        .collect();

    let Some(team) = game.teams.iter_mut().find(|t| t.id == team_id) else {
        return;
    };
    if team.player_roles.is_empty() {
        return;
    }

    let slots = ofm_core::player_rating::formation_slots(&team.formation);
    // Validate EVERY assigned role, not just starters: a role that was valid in
    // a player's slot must be cleared once they are benched (validated against
    // their natural position there), otherwise the stale assignment persists and
    // leaks back into match simulation.
    let invalid: Vec<String> = team
        .player_roles
        .iter()
        .filter_map(|(player_id, role)| {
            let position = team
                .starting_xi_ids
                .iter()
                .position(|id| id == player_id)
                .and_then(|slot_index| slots.get(slot_index).cloned())
                .or_else(|| natural_positions.get(player_id).cloned())?;
            if role_valid_for_position(role, &position) {
                None
            } else {
                Some(player_id.clone())
            }
        })
        .collect();

    for player_id in invalid {
        team.player_roles.remove(&player_id);
    }
}

fn role_valid_for_position(
    role: &domain::team::PlayerRole,
    pos: &domain::player::Position,
) -> bool {
    use domain::player::Position as P;
    use domain::team::PlayerRole as R;
    match pos {
        P::Goalkeeper => matches!(role, R::Standard | R::BallPlayingKeeper | R::SweeperKeeper),
        P::CenterBack => matches!(
            role,
            R::Standard | R::Stopper | R::CoverCB | R::BallPlayingCB
        ),
        P::RightBack | P::LeftBack | P::RightWingBack | P::LeftWingBack => {
            matches!(
                role,
                R::Standard | R::AttackingFB | R::DefensiveFB | R::InvertedFB | R::WingBack
            )
        }
        P::DefensiveMidfielder => {
            matches!(
                role,
                R::Standard | R::AnchorMan | R::BallWinner | R::DeepLyingPlaymaker
            )
        }
        P::CentralMidfielder => {
            matches!(role, R::Standard | R::BoxToBox | R::Carrilero | R::Mezzala)
        }
        P::AttackingMidfielder => {
            matches!(role, R::Standard | R::AdvancedPlaymaker | R::ShadowStriker)
        }
        P::RightMidfielder | P::LeftMidfielder | P::RightWinger | P::LeftWinger => {
            matches!(
                role,
                R::Standard | R::WideForward | R::InsideForward | R::InvertedWinger
            )
        }
        P::Striker => matches!(
            role,
            R::Standard
                | R::Poacher
                | R::TargetMan
                | R::DeepLyingForward
                | R::False9
                | R::PressingForward
                | R::CompleteForward
        ),
        // Legacy coarse-bucket positions: allow all roles in the broad group
        P::Defender => !matches!(
            role,
            R::BallPlayingKeeper
                | R::SweeperKeeper
                | R::AnchorMan
                | R::BallWinner
                | R::DeepLyingPlaymaker
                | R::BoxToBox
                | R::Carrilero
                | R::Mezzala
                | R::AdvancedPlaymaker
                | R::ShadowStriker
                | R::WideForward
                | R::InsideForward
                | R::InvertedWinger
                | R::Poacher
                | R::TargetMan
                | R::DeepLyingForward
                | R::False9
                | R::PressingForward
                | R::CompleteForward
        ),
        P::Midfielder => !matches!(
            role,
            R::BallPlayingKeeper
                | R::SweeperKeeper
                | R::Stopper
                | R::CoverCB
                | R::BallPlayingCB
                | R::AttackingFB
                | R::DefensiveFB
                | R::InvertedFB
                | R::WingBack
                | R::Poacher
                | R::TargetMan
                | R::DeepLyingForward
                | R::False9
                | R::PressingForward
                | R::CompleteForward
        ),
        P::Forward => !matches!(
            role,
            R::BallPlayingKeeper
                | R::SweeperKeeper
                | R::Stopper
                | R::CoverCB
                | R::BallPlayingCB
                | R::AttackingFB
                | R::DefensiveFB
                | R::InvertedFB
                | R::WingBack
                | R::AnchorMan
                | R::BallWinner
                | R::DeepLyingPlaymaker
                | R::BoxToBox
                | R::Carrilero
                | R::Mezzala
                | R::AdvancedPlaymaker
                | R::ShadowStriker
        ),
    }
}

#[tauri::command]
pub fn set_player_role(
    state: State<'_, Arc<StateManager>>,
    player_id: String,
    role: Option<String>,
) -> Result<Game, String> {
    set_player_role_internal(&state, player_id, role)
}

pub fn set_player_role_internal(
    state: &StateManager,
    player_id: String,
    role: Option<String>,
) -> Result<Game, String> {
    info!("[cmd] set_player_role: player={} role={:?}", player_id, role);
    mutate_active_game(state, |game| {
        let team_id = game
            .manager
            .team_id
            .clone()
            .ok_or("be.error.noTeamAssigned".to_string())?;

        // Validate the role against where the player is actually deployed (the
        // granular slot from formation + starting XI), falling back to their
        // natural position when they are not in the starting XI. Using the
        // deployed slot keeps the validator in agreement with the role options
        // the UI offers and fixes spurious rejections (issue #257).
        let natural_position = game
            .players
            .iter()
            .find(|p| p.id == player_id && p.team_id.as_deref() == Some(&team_id))
            .map(|p| p.natural_position.clone())
            .ok_or_else(|| "be.error.playerNotOnTeam".to_string())?;
        let validation_position = game
            .teams
            .iter()
            .find(|t| t.id == team_id)
            .and_then(|team| ofm_core::player_rating::deployed_position(team, &player_id))
            .unwrap_or(natural_position);

        let team = user_team_mut(game)?;
        match role {
            Some(r) => {
                let role_enum = r
                    .parse::<domain::team::PlayerRole>()
                    .map_err(|_| "be.error.invalidPlayerRole".to_string())?;
                if !role_valid_for_position(&role_enum, &validation_position) {
                    return Err("be.error.roleNotValidForPosition".to_string());
                }
                team.player_roles.insert(player_id.clone(), role_enum);
            }
            None => {
                team.player_roles.remove(&player_id);
            }
        }

        Ok(())
    })
}

// One `Option<String>` per tactical dial: this is the IPC signature the UI sends,
// and each dial is independently optional. Bundling them into a struct would move
// the same field count behind a serde type without making any call site simpler.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn set_tactics_phase(
    state: State<'_, Arc<StateManager>>,
    build_up_style: Option<String>,
    width: Option<String>,
    tempo: Option<String>,
    defensive_line: Option<String>,
    pressing_intensity: Option<String>,
    defensive_shape: Option<String>,
    marking_style: Option<String>,
    counter_press_duration: Option<String>,
    break_speed: Option<String>,
) -> Result<Game, String> {
    use domain::team::*;
    info!("[cmd] set_tactics_phase");
    mutate_active_game(&state, |game| {
        let team = user_team_mut(game)?;
        let p = &mut team.tactics_phase;
        if let Some(v) = build_up_style {
            p.build_up_style = match v.as_str() {
                "Short" => BuildUpStyle::Short,
                "Long" => BuildUpStyle::Long,
                _ => BuildUpStyle::Mixed,
            };
        }
        if let Some(v) = width {
            p.width = match v.as_str() {
                "Narrow" => PitchWidth::Narrow,
                "Wide" => PitchWidth::Wide,
                _ => PitchWidth::Normal,
            };
        }
        if let Some(v) = tempo {
            p.tempo = match v.as_str() {
                "Patient" => Tempo::Patient,
                _ => Tempo::Direct,
            };
        }
        if let Some(v) = defensive_line {
            p.defensive_line = match v.as_str() {
                "VeryLow" => DefensiveLine::VeryLow,
                "Low" => DefensiveLine::Low,
                "High" => DefensiveLine::High,
                _ => DefensiveLine::Medium,
            };
        }
        if let Some(v) = pressing_intensity {
            p.pressing_intensity = match v.as_str() {
                "Passive" => PressingIntensity::Passive,
                "Aggressive" => PressingIntensity::Aggressive,
                _ => PressingIntensity::Medium,
            };
        }
        if let Some(v) = defensive_shape {
            p.defensive_shape = match v.as_str() {
                "Stretched" => DefensiveShape::Stretched,
                "Compact" => DefensiveShape::Compact,
                _ => DefensiveShape::Normal,
            };
        }
        if let Some(v) = marking_style {
            p.marking_style = match v.as_str() {
                "ManToMan" => MarkingStyle::ManToMan,
                "Mixed" => MarkingStyle::Mixed,
                _ => MarkingStyle::Zonal,
            };
        }
        if let Some(v) = counter_press_duration {
            p.counter_press_duration = match v.as_str() {
                "Short" => CounterPressDuration::Short,
                "Long" => CounterPressDuration::Long,
                _ => CounterPressDuration::None,
            };
        }
        if let Some(v) = break_speed {
            p.break_speed = match v.as_str() {
                "Slow" => BreakSpeed::Slow,
                "Fast" => BreakSpeed::Fast,
                _ => BreakSpeed::Medium,
            };
        }

        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::{
        set_formation_internal, set_player_role_internal, set_player_squad_role_internal,
        set_player_training_focus_internal,
    };
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::player::{Player, PlayerAttributes, Position, SquadRole};
    use domain::team::Team;
    use domain::team::TrainingFocus;
    use ofm_core::clock::GameClock;
    use ofm_core::game::Game;
    use ofm_core::state::StateManager;

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

    fn make_user_team() -> Team {
        let mut team = make_team("team-1", "User FC", "USR");
        team.manager_id = Some("manager-1".to_string());
        team.starting_xi_ids = vec!["player-1".to_string()];
        team
    }

    fn make_team(id: &str, name: &str, short_name: &str) -> Team {
        Team::new(
            id.to_string(),
            name.to_string(),
            short_name.to_string(),
            "England".to_string(),
            "London".to_string(),
            format!("{} Ground", name),
            25_000,
        )
    }

    fn make_player(date_of_birth: &str) -> Player {
        make_player_for_team("player-1", "team-1", date_of_birth)
    }

    fn make_player_for_team(id: &str, team_id: &str, date_of_birth: &str) -> Player {
        let mut player = Player::new(
            id.to_string(),
            "P. One".to_string(),
            "Player One".to_string(),
            date_of_birth.to_string(),
            "England".to_string(),
            Position::Forward,
            default_attrs(),
        );
        player.team_id = Some(team_id.to_string());
        player
    }

    fn make_game(player: Player) -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "manager-1".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("team-1".to_string());

        Game::new(
            clock,
            manager,
            vec![make_user_team()],
            vec![player],
            vec![],
            vec![],
        )
    }

    /// The manager holds a team id, but no club in the world has it. This used
    /// to be a silent no-op: the lookup was written `if let Some(team) = ...`
    /// with no else, so the command reported success having changed nothing.
    fn make_game_with_dangling_team_id(player: Player) -> Game {
        let mut game = make_game(player);
        game.teams[0].id = "team-elsewhere".to_string();
        game
    }

    #[test]
    fn set_formation_internal_reports_a_team_id_that_matches_no_club() {
        let state = StateManager::new();
        state.set_game(make_game_with_dangling_team_id(make_player("1998-01-01")));

        let result = set_formation_internal(&state, "4-3-3");

        assert_eq!(result.err(), Some("be.error.teamNotFound".into()));

        let stored = state.get_game(|game| game.clone()).expect("stored game");
        assert_ne!(stored.teams[0].formation, "4-3-3");
    }

    #[test]
    fn set_formation_internal_reports_an_unemployed_manager() {
        let state = StateManager::new();
        let mut game = make_game(make_player("1998-01-01"));
        game.manager.team_id = None;
        state.set_game(game);

        assert_eq!(
            set_formation_internal(&state, "4-3-3").err(),
            Some("be.error.noTeamAssigned".into())
        );
    }

    #[test]
    fn set_player_squad_role_internal_reports_a_team_id_that_matches_no_club() {
        let state = StateManager::new();
        state.set_game(make_game_with_dangling_team_id(make_player("2008-01-01")));

        let result = set_player_squad_role_internal(&state, "player-1", "Youth");

        assert_eq!(result.err(), Some("be.error.teamNotFound".into()));

        // The XI edit is the fallible step and runs first, so the squad-role
        // write must not have happened either.
        let stored = state.get_game(|game| game.clone()).expect("stored game");
        assert_ne!(stored.players[0].squad_role, SquadRole::Youth);
    }

    #[test]
    fn set_player_squad_role_internal_updates_state_and_removes_from_xi() {
        let state = StateManager::new();
        state.set_game(make_game(make_player("2008-01-01")));

        let response =
            set_player_squad_role_internal(&state, "player-1", "Youth").expect("response");

        assert_eq!(response.players[0].squad_role, SquadRole::Youth);
        assert!(response.teams[0].starting_xi_ids.is_empty());

        let stored_game = state.get_game(|game| game.clone()).expect("stored game");
        assert_eq!(stored_game.players[0].squad_role, SquadRole::Youth);
        assert!(stored_game.teams[0].starting_xi_ids.is_empty());
    }

    #[test]
    fn reconcile_player_roles_clears_function_invalid_for_current_slot() {
        use domain::team::PlayerRole;

        let mut game = make_game(make_player("1998-01-01"));
        {
            let team = &mut game.teams[0];
            team.formation = "4-4-2".to_string();
            // Slot 0 in 4-4-2 is the Goalkeeper; a striker function is invalid there.
            team.starting_xi_ids = vec!["player-1".to_string()];
            team
                .player_roles
                .insert("player-1".to_string(), PlayerRole::Poacher);
        }

        super::reconcile_player_roles(&mut game, "team-1");

        assert!(!game.teams[0].player_roles.contains_key("player-1"));
    }

    #[test]
    fn reconcile_player_roles_keeps_function_valid_for_current_slot() {
        use domain::team::PlayerRole;

        let mut game = make_game(make_player("1998-01-01"));
        {
            let team = &mut game.teams[0];
            team.formation = "4-4-2".to_string();
            // Standard is valid for every position and must be preserved.
            team.starting_xi_ids = vec!["player-1".to_string()];
            team
                .player_roles
                .insert("player-1".to_string(), PlayerRole::Standard);
        }

        super::reconcile_player_roles(&mut game, "team-1");

        assert_eq!(
            game.teams[0].player_roles.get("player-1"),
            Some(&PlayerRole::Standard)
        );
    }

    #[test]
    fn reconcile_player_roles_clears_invalid_role_for_benched_player() {
        use domain::team::PlayerRole;

        let mut game = make_game(make_player("1998-01-01"));
        {
            let team = &mut game.teams[0];
            team.formation = "4-4-2".to_string();
            // player-1 (a natural Forward) is benched; a centre-back function is
            // invalid for their natural position and must be cleared.
            team.starting_xi_ids = vec![];
            team
                .player_roles
                .insert("player-1".to_string(), PlayerRole::Stopper);
        }

        super::reconcile_player_roles(&mut game, "team-1");

        assert!(!game.teams[0].player_roles.contains_key("player-1"));
    }

    #[test]
    fn reconcile_player_roles_keeps_valid_role_for_benched_player() {
        use domain::team::PlayerRole;

        let mut game = make_game(make_player("1998-01-01"));
        {
            let team = &mut game.teams[0];
            team.formation = "4-4-2".to_string();
            // A forward function is valid for a benched forward and is preserved.
            team.starting_xi_ids = vec![];
            team
                .player_roles
                .insert("player-1".to_string(), PlayerRole::Poacher);
        }

        super::reconcile_player_roles(&mut game, "team-1");

        assert_eq!(
            game.teams[0].player_roles.get("player-1"),
            Some(&PlayerRole::Poacher)
        );
    }

    #[test]
    fn set_formation_internal_does_not_mutate_player_position() {
        let state = StateManager::new();
        // player-1 is a Forward and the only outfield player on the team.
        state.set_game(make_game(make_player("1998-01-01")));

        set_formation_internal(&state, "4-4-2").expect("formation set");

        let stored = state.get_game(|game| game.clone()).expect("stored game");
        // Previously set_formation stat-ranked this player into the Defender
        // bucket; their stored position must now be left untouched.
        assert_eq!(stored.players[0].position, Position::Forward);
        assert_eq!(stored.teams[0].formation, "4-4-2");
    }

    #[test]
    fn set_player_role_internal_validates_against_deployed_slot() {
        use domain::team::PlayerRole;

        let mut game = make_game(make_player("1998-01-01"));
        {
            // Deploy the (natural Forward) player at the right-back slot of 4-4-2
            // (slot index 4). The validator must judge the role by that slot.
            let team = &mut game.teams[0];
            team.formation = "4-4-2".to_string();
            team.starting_xi_ids = vec![
                "x0".to_string(),
                "x1".to_string(),
                "x2".to_string(),
                "x3".to_string(),
                "player-1".to_string(),
            ];
        }
        let state = StateManager::new();
        state.set_game(game);

        // A full-back role is valid at the RB slot even though the player's
        // natural position is Forward (the issue #257 repro).
        let result =
            set_player_role_internal(&state, "player-1".to_string(), Some("DefensiveFB".to_string()))
                .expect("DefensiveFB is valid at the RB slot");
        assert_eq!(
            result.teams[0].player_roles.get("player-1"),
            Some(&PlayerRole::DefensiveFB)
        );

        // A midfield role is not valid at the RB slot.
        let error = set_player_role_internal(
            &state,
            "player-1".to_string(),
            Some("BoxToBox".to_string()),
        )
        .expect_err("BoxToBox is not valid at the RB slot");
        assert_eq!(error, "be.error.roleNotValidForPosition");
    }

    /// Pins the backend validator to the canonical position->roles table. This
    /// MUST stay in lock-step with the front-end mirror in
    /// src/lib/playerRoles.ts (guarded there by playerRoles.test.ts). If you
    /// change one side, change the other.
    #[test]
    fn role_valid_for_position_matches_canonical_table() {
        use domain::player::Position as P;
        use domain::team::PlayerRole as R;

        let all_roles = [
            R::Standard,
            R::BallPlayingKeeper,
            R::SweeperKeeper,
            R::Stopper,
            R::CoverCB,
            R::BallPlayingCB,
            R::AttackingFB,
            R::DefensiveFB,
            R::InvertedFB,
            R::WingBack,
            R::AnchorMan,
            R::BallWinner,
            R::DeepLyingPlaymaker,
            R::BoxToBox,
            R::Carrilero,
            R::Mezzala,
            R::AdvancedPlaymaker,
            R::ShadowStriker,
            R::WideForward,
            R::InsideForward,
            R::InvertedWinger,
            R::Poacher,
            R::TargetMan,
            R::DeepLyingForward,
            R::False9,
            R::PressingForward,
            R::CompleteForward,
        ];

        // One representative per granular branch (FB and wide-mid positions share
        // a branch in role_valid_for_position).
        let canonical: &[(P, &[R])] = &[
            (P::Goalkeeper, &[R::Standard, R::BallPlayingKeeper, R::SweeperKeeper]),
            (P::CenterBack, &[R::Standard, R::Stopper, R::CoverCB, R::BallPlayingCB]),
            (
                P::RightBack,
                &[R::Standard, R::AttackingFB, R::DefensiveFB, R::InvertedFB, R::WingBack],
            ),
            (
                P::DefensiveMidfielder,
                &[R::Standard, R::AnchorMan, R::BallWinner, R::DeepLyingPlaymaker],
            ),
            (
                P::CentralMidfielder,
                &[R::Standard, R::BoxToBox, R::Carrilero, R::Mezzala],
            ),
            (
                P::AttackingMidfielder,
                &[R::Standard, R::AdvancedPlaymaker, R::ShadowStriker],
            ),
            (
                P::RightMidfielder,
                &[R::Standard, R::WideForward, R::InsideForward, R::InvertedWinger],
            ),
            (
                P::Striker,
                &[
                    R::Standard,
                    R::Poacher,
                    R::TargetMan,
                    R::DeepLyingForward,
                    R::False9,
                    R::PressingForward,
                    R::CompleteForward,
                ],
            ),
            // Legacy coarse buckets (deny-list branches): the union of the
            // group's detailed roles.
            (
                P::Defender,
                &[
                    R::Standard,
                    R::Stopper,
                    R::CoverCB,
                    R::BallPlayingCB,
                    R::AttackingFB,
                    R::DefensiveFB,
                    R::InvertedFB,
                    R::WingBack,
                ],
            ),
            (
                P::Midfielder,
                &[
                    R::Standard,
                    R::AnchorMan,
                    R::BallWinner,
                    R::DeepLyingPlaymaker,
                    R::BoxToBox,
                    R::Carrilero,
                    R::Mezzala,
                    R::AdvancedPlaymaker,
                    R::ShadowStriker,
                    R::WideForward,
                    R::InsideForward,
                    R::InvertedWinger,
                ],
            ),
            (
                P::Forward,
                &[
                    R::Standard,
                    R::WideForward,
                    R::InsideForward,
                    R::InvertedWinger,
                    R::Poacher,
                    R::TargetMan,
                    R::DeepLyingForward,
                    R::False9,
                    R::PressingForward,
                    R::CompleteForward,
                ],
            ),
        ];

        for (pos, expected) in canonical {
            for role in &all_roles {
                let want = expected.contains(role);
                assert_eq!(
                    super::role_valid_for_position(role, pos),
                    want,
                    "role {:?} at {:?}: expected valid={}",
                    role,
                    pos,
                    want
                );
            }
        }
    }

    #[test]
    fn set_player_squad_role_internal_rejects_overage_youth_assignment() {
        let state = StateManager::new();
        state.set_game(make_game(make_player("1998-01-01")));

        let error = set_player_squad_role_internal(&state, "player-1", "Youth").expect_err("error");

        assert_eq!(error, "be.error.youthAcademyOverage");
    }

    #[test]
    fn set_player_training_focus_internal_rejects_players_from_other_teams() {
        let state = StateManager::new();
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "manager-1".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("team-1".to_string());

        let other_player = make_player_for_team("player-2", "team-2", "2004-01-01");
        let game = Game::new(
            clock,
            manager,
            vec![make_user_team(), make_team("team-2", "Rivals FC", "RIV")],
            vec![make_player("2008-01-01"), other_player],
            vec![],
            vec![],
        );
        state.set_game(game);

        let error = set_player_training_focus_internal(&state, "player-2", Some("Technical"))
            .expect_err("cross-team player should be rejected");

        assert_eq!(error, "be.error.playerNotFound");

        let stored_game = state.get_game(|game| game.clone()).expect("stored game");
        let other_player = stored_game
            .players
            .iter()
            .find(|player| player.id == "player-2")
            .expect("other player");
        assert_eq!(other_player.training_focus, None);

        let user_player = stored_game
            .players
            .iter()
            .find(|player| player.id == "player-1")
            .expect("user player");
        assert_ne!(user_player.training_focus, Some(TrainingFocus::Technical));
    }
}
