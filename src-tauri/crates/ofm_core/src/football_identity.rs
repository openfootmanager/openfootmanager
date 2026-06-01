use crate::game::Game;
use domain::identity::{derive_birth_country_code, normalize_football_nation_code};
use domain::manager::Manager;
use domain::player::Player;
use domain::staff::Staff;
use domain::team::Team;
use std::collections::HashMap;

pub fn upgrade_game_football_identities(game: &mut Game) -> bool {
    let mut changed = false;

    changed |=
        upgrade_world_football_identities(&mut game.teams, &mut game.players, &mut game.staff);
    changed |= upgrade_world_manager_identities(&game.teams, &mut game.managers);

    let team_nations = build_team_nation_map(&game.teams);

    changed |= upgrade_manager_identity(&mut game.manager, &team_nations);

    changed
}

pub fn upgrade_world_manager_identities(teams: &[Team], managers: &mut [Manager]) -> bool {
    let mut changed = false;
    let team_nations = build_team_nation_map(teams);

    for manager in managers.iter_mut() {
        changed |= upgrade_manager_identity(manager, &team_nations);
    }

    changed
}

pub fn upgrade_world_football_identities(
    teams: &mut [Team],
    players: &mut [Player],
    staff: &mut [Staff],
) -> bool {
    let mut changed = false;

    for team in teams.iter_mut() {
        changed |= upgrade_team_identity(team);
    }

    let team_nations = build_team_nation_map(teams);

    for player in players.iter_mut() {
        changed |= upgrade_player_identity(player, &team_nations);
    }

    for staff_member in staff.iter_mut() {
        changed |= upgrade_staff_identity(staff_member, &team_nations);
    }

    changed
}

fn build_team_nation_map(teams: &[Team]) -> HashMap<&str, &str> {
    teams
        .iter()
        .map(|team| (team.id.as_str(), team.football_nation.as_str()))
        .collect()
}

fn normalize_optional_birth_country(value: Option<String>, fallback: &str) -> Option<String> {
    match value {
        Some(existing) if !existing.trim().is_empty() => derive_birth_country_code(&existing),
        _ => derive_birth_country_code(fallback),
    }
}

fn resolve_legacy_nationality(nationality: &str, football_nation: &str) -> String {
    let normalized_nationality = normalize_football_nation_code(nationality);
    if normalized_nationality == "GB" && !football_nation.is_empty() && football_nation != "GB" {
        football_nation.to_string()
    } else {
        nationality.to_string()
    }
}

fn normalize_existing_or_fallback(existing: &str, fallback: &str) -> String {
    if existing.trim().is_empty() {
        normalize_football_nation_code(fallback)
    } else {
        normalize_football_nation_code(existing)
    }
}

fn inherit_team_football_nation(
    current_football_nation: &str,
    team_id: Option<&str>,
    team_nations: &HashMap<&str, &str>,
) -> Option<String> {
    if current_football_nation != "GB" {
        return None;
    }

    let team_nation = team_id.and_then(|id| team_nations.get(id).copied())?;
    if team_nation == "GB" || team_nation.is_empty() {
        None
    } else {
        Some(team_nation.to_string())
    }
}

fn upgrade_manager_identity(manager: &mut Manager, team_nations: &HashMap<&str, &str>) -> bool {
    let original_nationality = manager.nationality.clone();
    let mut football_nation =
        normalize_existing_or_fallback(&manager.football_nation, &original_nationality);
    if let Some(inherited) =
        inherit_team_football_nation(&football_nation, manager.team_id.as_deref(), team_nations)
    {
        football_nation = inherited;
    }
    let birth_country =
        normalize_optional_birth_country(manager.birth_country.clone(), &original_nationality);
    let nationality = resolve_legacy_nationality(&original_nationality, &football_nation);

    let changed = manager.nationality != nationality
        || manager.football_nation != football_nation
        || manager.birth_country != birth_country;
    manager.nationality = nationality;
    manager.football_nation = football_nation;
    manager.birth_country = birth_country;
    changed
}

fn upgrade_player_identity(player: &mut Player, team_nations: &HashMap<&str, &str>) -> bool {
    let original_nationality = player.nationality.clone();
    let mut football_nation =
        normalize_existing_or_fallback(&player.football_nation, &original_nationality);
    if let Some(inherited) =
        inherit_team_football_nation(&football_nation, player.team_id.as_deref(), team_nations)
    {
        football_nation = inherited;
    }
    let birth_country =
        normalize_optional_birth_country(player.birth_country.clone(), &original_nationality);
    let nationality = resolve_legacy_nationality(&original_nationality, &football_nation);

    let changed = player.nationality != nationality
        || player.football_nation != football_nation
        || player.birth_country != birth_country;
    player.nationality = nationality;
    player.football_nation = football_nation;
    player.birth_country = birth_country;
    changed
}

fn upgrade_staff_identity(staff: &mut Staff, team_nations: &HashMap<&str, &str>) -> bool {
    let original_nationality = staff.nationality.clone();
    let mut football_nation =
        normalize_existing_or_fallback(&staff.football_nation, &original_nationality);
    if let Some(inherited) =
        inherit_team_football_nation(&football_nation, staff.team_id.as_deref(), team_nations)
    {
        football_nation = inherited;
    }
    let birth_country =
        normalize_optional_birth_country(staff.birth_country.clone(), &original_nationality);
    let nationality = resolve_legacy_nationality(&original_nationality, &football_nation);

    let changed = staff.nationality != nationality
        || staff.football_nation != football_nation
        || staff.birth_country != birth_country;
    staff.nationality = nationality;
    staff.football_nation = football_nation;
    staff.birth_country = birth_country;
    changed
}

fn upgrade_team_identity(team: &mut Team) -> bool {
    let mut football_nation = normalize_existing_or_fallback(&team.football_nation, &team.country);
    if football_nation == "GB" {
        football_nation = infer_legacy_british_team_nation(team).unwrap_or(football_nation);
    }
    let changed = team.football_nation != football_nation;
    team.football_nation = football_nation;
    changed
}

fn infer_legacy_british_team_nation(team: &Team) -> Option<String> {
    let team_name = team.name.trim();
    let city = team.city.trim();

    match (team_name, city) {
        ("London FC", "London")
        | ("Manchester City", "Manchester")
        | ("Liverpool Athletic", "Liverpool")
        | ("Newcastle Town", "Newcastle") => Some("ENG".to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use crate::game::Game;
    use chrono::TimeZone;
    use domain::manager::Manager;
    use domain::player::{Player, PlayerAttributes, Position};
    use domain::staff::{Staff, StaffAttributes, StaffRole};
    use domain::team::Team;

    fn sample_attrs() -> PlayerAttributes {
        PlayerAttributes {
            pace: 70,
            stamina: 70,
            strength: 70,
            agility: 70,
            passing: 70,
            shooting: 70,
            tackling: 70,
            dribbling: 70,
            defending: 70,
            positioning: 70,
            vision: 70,
            decisions: 70,
            composure: 70,
            aggression: 70,
            teamwork: 70,
            leadership: 70,
            handling: 20,
            reflexes: 20,
            aerial: 60,
        }
    }

    #[test]
    fn upgrade_game_football_identities_populates_new_fields() {
        let clock = GameClock::new(chrono::Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap());
        let mut manager = Manager::new(
            "mgr".to_string(),
            "Ada".to_string(),
            "Lovelace".to_string(),
            "1980-01-01".to_string(),
            "British".to_string(),
        );
        manager.hire("t1".to_string());
        let mut player = Player::new(
            "p1".to_string(),
            "J. Smith".to_string(),
            "John Smith".to_string(),
            "2000-01-01".to_string(),
            "GB".to_string(),
            Position::Midfielder,
            sample_attrs(),
        );
        player.football_nation.clear();
        player.birth_country = None;
        player.team_id = Some("t1".to_string());
        let mut staff = Staff::new(
            "s1".to_string(),
            "Sam".to_string(),
            "Coach".to_string(),
            "1980-01-01".to_string(),
            StaffRole::Coach,
            StaffAttributes {
                coaching: 70,
                judging_ability: 70,
                judging_potential: 70,
                physiotherapy: 30,
            },
        );
        staff.nationality = "British".to_string();
        staff.team_id = Some("t1".to_string());
        let mut team = Team::new(
            "t1".to_string(),
            "London FC".to_string(),
            "LON".to_string(),
            "GB".to_string(),
            "London".to_string(),
            "Arena".to_string(),
            50000,
        );
        team.football_nation.clear();

        let mut game = Game::new(
            clock,
            manager,
            vec![team],
            vec![player],
            vec![staff],
            vec![],
        );
        game.manager.nationality = "British".to_string();
        game.manager.football_nation.clear();
        game.manager.birth_country = None;
        game.players[0].nationality = "GB".to_string();
        game.players[0].football_nation.clear();
        game.players[0].birth_country = None;
        game.staff[0].nationality = "British".to_string();
        game.staff[0].football_nation.clear();
        game.staff[0].birth_country = None;
        game.teams[0].football_nation.clear();
        let changed = upgrade_game_football_identities(&mut game);

        assert!(changed);
        assert_eq!(game.manager.nationality, "ENG");
        assert_eq!(game.manager.football_nation, "ENG");
        assert_eq!(game.manager.birth_country, None);
        assert_eq!(game.players[0].nationality, "ENG");
        assert_eq!(game.players[0].football_nation, "ENG");
        assert_eq!(game.players[0].birth_country, None);
        assert_eq!(game.staff[0].nationality, "ENG");
        assert_eq!(game.staff[0].football_nation, "ENG");
        assert_eq!(game.teams[0].football_nation, "ENG");
    }

    #[test]
    fn upgrade_game_football_identities_keeps_ambiguous_gb_without_evidence() {
        let clock = GameClock::new(chrono::Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap());
        let manager = Manager::new(
            "mgr".to_string(),
            "Ada".to_string(),
            "Lovelace".to_string(),
            "1980-01-01".to_string(),
            "British".to_string(),
        );
        let mut player = Player::new(
            "p1".to_string(),
            "J. Smith".to_string(),
            "John Smith".to_string(),
            "2000-01-01".to_string(),
            "GB".to_string(),
            Position::Midfielder,
            sample_attrs(),
        );
        player.football_nation.clear();
        player.birth_country = None;

        let mut game = Game::new(clock, manager, vec![], vec![player], vec![], vec![]);
        game.players[0].nationality = "GB".to_string();
        game.players[0].football_nation.clear();
        game.players[0].birth_country = None;

        let changed = upgrade_game_football_identities(&mut game);

        assert!(changed);
        assert_eq!(game.players[0].nationality, "GB");
        assert_eq!(game.players[0].football_nation, "GB");
        assert_eq!(game.players[0].birth_country, None);
    }
}
