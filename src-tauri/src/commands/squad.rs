use chrono::Datelike;
use log::info;
use tauri::State;

use ofm_core::game::Game;
use ofm_core::state::StateManager;

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

#[tauri::command]
pub fn set_formation(state: State<'_, StateManager>, formation: String) -> Result<Game, String> {
    info!("[cmd] set_formation: {}", formation);
    let mut game = state
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;

    let team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("No team assigned".to_string())?;

    // Parse formation into (def, mid, fwd) counts
    let parts: Vec<usize> = formation
        .split('-')
        .filter_map(|s| s.parse().ok())
        .collect();
    let (num_def, num_mid, num_fwd) = match parts.len() {
        3 => (parts[0], parts[1], parts[2]),
        4 => (parts[0], parts[1] + parts[2], parts[3]),
        _ => (4, 4, 2),
    };

    if let Some(team) = game.teams.iter_mut().find(|t| t.id == team_id) {
        team.formation = formation;
    }

    // Reassign positions for outfield players on this team
    let player_ids: Vec<String> = game
        .players
        .iter()
        .filter(|p| {
            p.team_id.as_deref() == Some(&team_id)
                && p.position != domain::player::Position::Goalkeeper
        })
        .map(|p| p.id.clone())
        .collect();

    // Sort by defensive ability (most defensive first)
    let mut sorted_ids = player_ids.clone();
    sorted_ids.sort_by(|a_id, b_id| {
        let pa = game.players.iter().find(|p| p.id == *a_id).unwrap();
        let pb = game.players.iter().find(|p| p.id == *b_id).unwrap();
        let def_a = pa.attributes.defending as u16
            + pa.attributes.tackling as u16
            + pa.attributes.strength as u16;
        let def_b = pb.attributes.defending as u16
            + pb.attributes.tackling as u16
            + pb.attributes.strength as u16;
        def_b.cmp(&def_a)
    });

    // Assign positions
    for (slot, pid) in sorted_ids.iter().enumerate() {
        let new_pos = if slot < num_def {
            domain::player::Position::Defender
        } else if slot < num_def + num_mid {
            domain::player::Position::Midfielder
        } else if slot < num_def + num_mid + num_fwd {
            domain::player::Position::Forward
        } else {
            continue;
        };
        if let Some(player) = game.players.iter_mut().find(|p| p.id == *pid) {
            player.position = new_pos;
        }
    }

    state.set_game(game.clone());
    Ok(game)
}

#[tauri::command]
pub fn set_starting_xi(
    state: State<'_, StateManager>,
    player_ids: Vec<String>,
) -> Result<Game, String> {
    info!("[cmd] set_starting_xi: {} players", player_ids.len());
    let mut game = state
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;

    let team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("No team assigned".to_string())?;

    if let Some(team) = game.teams.iter_mut().find(|t| t.id == team_id) {
        team.starting_xi_ids = player_ids;
    }

    state.set_game(game.clone());
    Ok(game)
}

#[tauri::command]
pub fn set_play_style(state: State<'_, StateManager>, play_style: String) -> Result<Game, String> {
    info!("[cmd] set_play_style: {}", play_style);
    let mut game = state
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;

    let team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("No team assigned".to_string())?;

    let style = match play_style.as_str() {
        "Attacking" => domain::team::PlayStyle::Attacking,
        "Defensive" => domain::team::PlayStyle::Defensive,
        "Possession" => domain::team::PlayStyle::Possession,
        "Counter" => domain::team::PlayStyle::Counter,
        "HighPress" => domain::team::PlayStyle::HighPress,
        _ => domain::team::PlayStyle::Balanced,
    };

    if let Some(team) = game.teams.iter_mut().find(|t| t.id == team_id) {
        team.play_style = style;
    }

    state.set_game(game.clone());
    Ok(game)
}

#[tauri::command]
pub fn set_team_match_roles(
    state: State<'_, StateManager>,
    match_roles: domain::team::MatchRoles,
) -> Result<Game, String> {
    info!("[cmd] set_team_match_roles");
    let mut game = state
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;

    let team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("No team assigned".to_string())?;

    if let Some(team) = game.teams.iter_mut().find(|t| t.id == team_id) {
        team.match_roles = match_roles;
    }

    state.set_game(game.clone());
    Ok(game)
}

#[tauri::command]
pub fn set_training(
    state: State<'_, StateManager>,
    focus: String,
    intensity: String,
) -> Result<Game, String> {
    info!(
        "[cmd] set_training: focus={}, intensity={}",
        focus, intensity
    );
    let mut game = state
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;

    let team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("No team assigned".to_string())?;

    let training_focus = match focus.as_str() {
        "Physical" => domain::team::TrainingFocus::Physical,
        "Technical" => domain::team::TrainingFocus::Technical,
        "Tactical" => domain::team::TrainingFocus::Tactical,
        "Defending" => domain::team::TrainingFocus::Defending,
        "Attacking" => domain::team::TrainingFocus::Attacking,
        "Recovery" => domain::team::TrainingFocus::Recovery,
        _ => domain::team::TrainingFocus::Physical,
    };

    let training_intensity = match intensity.as_str() {
        "Low" => domain::team::TrainingIntensity::Low,
        "Medium" => domain::team::TrainingIntensity::Medium,
        "High" => domain::team::TrainingIntensity::High,
        _ => domain::team::TrainingIntensity::Medium,
    };

    if let Some(team) = game.teams.iter_mut().find(|t| t.id == team_id) {
        team.training_focus = training_focus;
        team.training_intensity = training_intensity;
    }

    state.set_game(game.clone());
    Ok(game)
}

#[tauri::command]
pub fn set_training_schedule(
    state: State<'_, StateManager>,
    schedule: String,
) -> Result<Game, String> {
    info!("[cmd] set_training_schedule: {}", schedule);
    let mut game = state
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;

    let team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("No team assigned".to_string())?;

    let training_schedule = match schedule.as_str() {
        "Intense" => domain::team::TrainingSchedule::Intense,
        "Balanced" => domain::team::TrainingSchedule::Balanced,
        "Light" => domain::team::TrainingSchedule::Light,
        _ => domain::team::TrainingSchedule::Balanced,
    };

    if let Some(team) = game.teams.iter_mut().find(|t| t.id == team_id) {
        team.training_schedule = training_schedule;
    }

    state.set_game(game.clone());
    Ok(game)
}

#[tauri::command]
pub fn set_training_groups(
    state: State<'_, StateManager>,
    groups: Vec<domain::team::TrainingGroup>,
) -> Result<Game, String> {
    info!("[cmd] set_training_groups: {} groups", groups.len());
    let mut game = state
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;

    let team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("No team assigned".to_string())?;

    if let Some(team) = game.teams.iter_mut().find(|t| t.id == team_id) {
        team.training_groups = groups;
    }

    state.set_game(game.clone());
    Ok(game)
}

#[tauri::command]
pub fn set_player_training_focus(
    state: State<'_, StateManager>,
    player_id: String,
    focus: Option<String>,
) -> Result<Game, String> {
    info!(
        "[cmd] set_player_training_focus: player={}, focus={:?}",
        player_id, focus
    );
    let mut game = state
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;

    let training_focus = focus.and_then(|f| match f.as_str() {
        "Physical" => Some(domain::team::TrainingFocus::Physical),
        "Technical" => Some(domain::team::TrainingFocus::Technical),
        "Tactical" => Some(domain::team::TrainingFocus::Tactical),
        "Defending" => Some(domain::team::TrainingFocus::Defending),
        "Attacking" => Some(domain::team::TrainingFocus::Attacking),
        "Recovery" => Some(domain::team::TrainingFocus::Recovery),
        _ => None,
    });

    if let Some(player) = game.players.iter_mut().find(|p| p.id == player_id) {
        player.training_focus = training_focus;
    } else {
        return Err(format!("Player not found: {}", player_id));
    }

    state.set_game(game.clone());
    Ok(game)
}

#[tauri::command]
pub fn set_player_squad_role(
    state: State<'_, StateManager>,
    player_id: String,
    squad_role: String,
) -> Result<Game, String> {
    set_player_squad_role_internal(&state, &player_id, &squad_role)
}

fn set_player_squad_role_internal(
    state: &StateManager,
    player_id: &str,
    squad_role: &str,
) -> Result<Game, String> {
    info!(
        "[cmd] set_player_squad_role: player={}, squad_role={}",
        player_id, squad_role
    );
    let mut game = state
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;

    let team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("No team assigned".to_string())?;
    let target_role = parse_squad_role(squad_role).ok_or("Invalid squad role".to_string())?;
    let current_date = game.clock.current_date.date_naive();

    let player_index = game
        .players
        .iter()
        .position(|player| player.id == player_id)
        .ok_or("Player not found".to_string())?;

    if game.players[player_index].team_id.as_deref() != Some(team_id.as_str()) {
        return Err("Player is not in your squad".to_string());
    }

    if matches!(target_role, domain::player::SquadRole::Youth) {
        let age = player_age_on(current_date, &game.players[player_index].date_of_birth)
            .ok_or("Invalid player date of birth".to_string())?;
        if age > 21 {
            return Err("Only players aged 21 or under can join the youth academy".to_string());
        }
    }

    game.players[player_index].squad_role = target_role;

    if matches!(target_role, domain::player::SquadRole::Youth) {
        if let Some(team) = game.teams.iter_mut().find(|team| team.id == team_id) {
            team.starting_xi_ids.retain(|id| id != player_id);
        }
    }

    state.set_game(game.clone());
    Ok(game)
}

#[tauri::command]
pub fn auto_select_set_pieces(
    state: State<'_, StateManager>,
    player_ids: Vec<String>,
) -> Result<serde_json::Value, String> {
    log::debug!("[cmd] auto_select_set_pieces: {} players", player_ids.len());
    let game = state
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;

    let (captain, penalty, free_kick, corner) =
        ofm_core::live_match_manager::auto_select_set_pieces(&game, &player_ids);

    Ok(serde_json::json!({
        "captain": captain,
        "penalty_taker": penalty,
        "free_kick_taker": free_kick,
        "corner_taker": corner,
    }))
}

#[cfg(test)]
mod tests {
    use super::set_player_squad_role_internal;
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::player::{Player, PlayerAttributes, Position, SquadRole};
    use domain::team::Team;
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
        let mut team = Team::new(
            "team-1".to_string(),
            "User FC".to_string(),
            "USR".to_string(),
            "England".to_string(),
            "London".to_string(),
            "User Ground".to_string(),
            25_000,
        );
        team.manager_id = Some("manager-1".to_string());
        team.starting_xi_ids = vec!["player-1".to_string()];
        team
    }

    fn make_player(date_of_birth: &str) -> Player {
        let mut player = Player::new(
            "player-1".to_string(),
            "P. One".to_string(),
            "Player One".to_string(),
            date_of_birth.to_string(),
            "England".to_string(),
            Position::Forward,
            default_attrs(),
        );
        player.team_id = Some("team-1".to_string());
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
    fn set_player_squad_role_internal_rejects_overage_youth_assignment() {
        let state = StateManager::new();
        state.set_game(make_game(make_player("1998-01-01")));

        let error = set_player_squad_role_internal(&state, "player-1", "Youth").expect_err("error");

        assert_eq!(
            error,
            "Only players aged 21 or under can join the youth academy"
        );
    }
}
