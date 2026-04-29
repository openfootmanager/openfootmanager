pub(crate) mod data;
pub mod definitions;
mod generation;
pub mod world_io;

pub use definitions::*;
pub use world_io::*;

use domain::player::Player;
use domain::staff::{Staff, StaffRole};
use domain::team::TeamColors;
use log::{debug, info};
use rand::RngExt;
use uuid::Uuid;

use generation::*;

fn expand_structure_teams(teams_def: &mut TeamsDefinition) {
    if !teams_def.teams.is_empty() {
        return;
    }
    let Some(structure) = teams_def.structure.clone() else {
        return;
    };
    for nation in structure.nations {
        for league in nation.leagues {
            for index in 1..=league.club_count {
                let name = format!("{} {} Club {:02}", nation.code, league.name, index);
                let city = format!("{} City {:02}", nation.code, index);
                teams_def.teams.push(TeamDef {
                    name: name.clone(),
                    short_name: format!("{}{:02}", nation.code, index),
                    city: city.clone(),
                    country: nation.name.clone(),
                    league: league.name.clone(),
                    colors: TeamColorsDef {
                        primary: "#1d4ed8".to_string(),
                        secondary: "#ffffff".to_string(),
                    },
                    play_style: "Balanced".to_string(),
                    stadium_name: format!("{city} Arena"),
                    reputation_range: Some([300, 900]),
                    finance_range: Some([500_000, 10_000_000]),
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// World generation
// ---------------------------------------------------------------------------

/// Generate a random world (raw tuple — used by `generate_world_data`).
/// Loads definition files from `data_dir` if provided; falls back to hardcoded defaults.
pub fn generate_world(
    data_dir: Option<&std::path::Path>,
) -> (Vec<domain::team::Team>, Vec<Player>, Vec<Staff>) {
    info!("[generator] generate_world: data_dir={:?}", data_dir);
    let mut rng = rand::rng();
    let mut teams_out = Vec::new();
    let mut players = Vec::new();
    let mut staff = Vec::new();

    // Load definitions (external file → hardcoded fallback)
    let names_def = data_dir
        .and_then(|dir| {
            let path = dir.join("default_names.json");
            let result = load_names_definition(&path);
            if result.is_some() {
                info!("[generator] loaded names from {:?}", path);
            } else {
                debug!("[generator] no names file at {:?}, using defaults", path);
            }
            result
        })
        .unwrap_or_else(default_names_definition);
    let mut teams_def = data_dir
        .and_then(|dir| {
            let path = dir.join("default_teams.json");
            let result = load_teams_definition(&path);
            if result.is_some() {
                info!("[generator] loaded teams from {:?}", path);
            } else {
                debug!("[generator] no teams file at {:?}, using defaults", path);
            }
            result
        })
        .unwrap_or_else(default_teams_definition);
    expand_structure_teams(&mut teams_def);

    let country_codes: Vec<String> = names_def.pools.keys().cloned().collect();

    for tdef in &teams_def.teams {
        let team_id = Uuid::new_v4().to_string();
        let short_name = if tdef.short_name.is_empty() {
            tdef.name
                .split_whitespace()
                .filter_map(|w| w.chars().next())
                .collect::<String>()
                .to_uppercase()
                .chars()
                .take(3)
                .collect()
        } else {
            tdef.short_name.clone()
        };
        let stadium = if tdef.stadium_name.is_empty() {
            format!("{} Arena", tdef.city)
        } else {
            tdef.stadium_name.clone()
        };

        let rep_range = tdef.reputation_range.unwrap_or([300, 900]);
        let fin_range = tdef.finance_range.unwrap_or([500_000, 10_000_000]);

        let mut team = domain::team::Team::new(
            team_id.clone(),
            tdef.name.clone(),
            short_name,
            tdef.country.clone(),
            tdef.city.clone(),
            stadium,
            rng.random_range(10000..80000),
        );
        team.finance = rng.random_range(fin_range[0]..fin_range[1]);
        team.reputation = rng.random_range(rep_range[0]..rep_range[1]);
        team.wage_budget = (team.finance as f64 * 0.06) as i64;
        team.transfer_budget = (team.finance as f64 * 0.15) as i64;
        team.founded_year = rng.random_range(1880..1960);
        team.colors = TeamColors {
            primary: tdef.colors.primary.clone(),
            secondary: tdef.colors.secondary.clone(),
        };
        team.play_style = play_style_from_str(&tdef.play_style);
        team.domestic_league = tdef.league.clone();
        teams_out.push(team);

        // Generate 22 players
        for j in 0..22 {
            let nationality = pick_nationality_from_def(&tdef.country, &country_codes, &mut rng);
            let mut player =
                generate_random_player_from_def(&team_id, j, &nationality, &names_def, &mut rng);
            if rng.random_range(0..100) < 12 {
                player.transfer_listed = true;
            } else if rng.random_range(0..100) < 8 {
                player.loan_listed = true;
            }
            players.push(player);
        }

        // Generate 4 staff per team
        let roles = [
            StaffRole::AssistantManager,
            StaffRole::Coach,
            StaffRole::Scout,
            StaffRole::Physio,
        ];
        for role in &roles {
            let nationality = pick_nationality_from_def(&tdef.country, &country_codes, &mut rng);
            let s = generate_random_staff_from_def(
                &team_id,
                role.clone(),
                &nationality,
                &names_def,
                &mut rng,
            );
            staff.push(s);
        }
    }

    // Generate free-agent staff
    let free_roles = [
        StaffRole::Coach,
        StaffRole::Scout,
        StaffRole::Physio,
        StaffRole::Coach,
        StaffRole::AssistantManager,
        StaffRole::Scout,
        StaffRole::Physio,
        StaffRole::Coach,
        StaffRole::Coach,
        StaffRole::Physio,
        StaffRole::Scout,
        StaffRole::AssistantManager,
    ];
    for role in &free_roles {
        let nat = &country_codes[rng.random_range(0..country_codes.len())];
        let s = generate_random_staff_unattached_from_def(role.clone(), nat, &names_def, &mut rng);
        staff.push(s);
    }

    info!(
        "[generator] world generated: {} teams, {} players, {} staff",
        teams_out.len(),
        players.len(),
        staff.len()
    );
    (teams_out, players, staff)
}

#[cfg(test)]
mod tests {
    use super::data::NATIONALITY_POOLS;
    use super::*;
    use domain::player::Position;

    #[test]
    fn test_generate_world_team_count() {
        let (teams, players, staff) = generate_world(None);
        assert_eq!(teams.len(), 80);
        assert_eq!(players.len(), 80 * 22);
        assert_eq!(staff.len(), 80 * 4 + 12);
    }

    #[test]
    fn test_generate_world_all_players_assigned() {
        let (teams, players, _) = generate_world(None);
        let team_ids: Vec<&str> = teams.iter().map(|t| t.id.as_str()).collect();
        for p in &players {
            assert!(p.team_id.is_some(), "Player {} has no team", p.full_name);
            assert!(
                team_ids.contains(&p.team_id.as_deref().unwrap()),
                "Player has unknown team"
            );
        }
    }

    #[test]
    fn test_generate_world_positions_per_team() {
        let (teams, players, _) = generate_world(None);
        for team in &teams {
            let team_players: Vec<_> = players
                .iter()
                .filter(|p| p.team_id.as_deref() == Some(&team.id))
                .collect();
            assert_eq!(team_players.len(), 22);
            let gk = team_players
                .iter()
                .filter(|p| p.position == Position::Goalkeeper)
                .count();
            assert!(gk >= 2, "Team {} has only {} GK", team.name, gk);
        }
    }

    #[test]
    fn test_pick_name_from_def() {
        let mut rng = rand::rng();
        let names_def = default_names_definition();
        // Known nationality (ISO alpha-2)
        let (first, last) = pick_name_from_def("ES", &names_def, &mut rng);
        assert!(!first.is_empty());
        assert!(!last.is_empty());
        // Football identity falls back to GB pool if a dedicated pool does not exist yet.
        let (eng_first, eng_last) = pick_name_from_def("ENG", &names_def, &mut rng);
        assert!(!eng_first.is_empty());
        assert!(!eng_last.is_empty());
        // Unknown code falls back to any pool
        let (first2, last2) = pick_name_from_def("ZZ", &names_def, &mut rng);
        assert!(!first2.is_empty());
        assert!(!last2.is_empty());
    }

    #[test]
    fn test_pick_nationality_weighted() {
        let mut rng = rand::rng();
        let codes: Vec<String> = NATIONALITY_POOLS
            .iter()
            .map(|p| p.nationality.to_string())
            .collect();
        let mut eng_count = 0;
        for _ in 0..100 {
            let nat = pick_nationality_from_def("England", &codes, &mut rng);
            if nat == "ENG" {
                eng_count += 1;
            }
        }
        assert!(
            eng_count > 30,
            "ENG players should be weighted: got {}/100",
            eng_count
        );
    }

    #[test]
    fn test_all_nationalities_use_short_uppercase_codes() {
        let (_, players, staff) = generate_world(None);
        for p in &players {
            assert_eq!(
                p.nationality.len() == 2 || p.nationality.len() == 3,
                true,
                "Player {} has invalid nationality code: {}",
                p.full_name,
                p.nationality
            );
            assert!(
                p.nationality.chars().all(|c| c.is_ascii_uppercase()),
                "Player {} nationality not uppercase: {}",
                p.full_name,
                p.nationality
            );
        }
        for s in &staff {
            assert_eq!(
                s.nationality.len() == 2 || s.nationality.len() == 3,
                true,
                "Staff {} has invalid nationality code: {}",
                s.first_name,
                s.nationality
            );
        }
    }

    #[test]
    fn test_team_templates_have_unique_names() {
        let teams_def = default_teams_definition();
        let names: Vec<&str> = teams_def.teams.iter().map(|t| t.name.as_str()).collect();
        let unique: std::collections::HashSet<&str> = names.iter().cloned().collect();
        assert_eq!(names.len(), unique.len(), "Duplicate team names found");
    }

    #[test]
    fn test_world_data_wrapper() {
        let world = generate_world_data(None);
        assert_eq!(world.teams.len(), 80);
        assert!(!world.name.is_empty());
        assert!(!world.description.is_empty());
    }

    #[test]
    fn test_definition_file_roundtrip() {
        let names_def = default_names_definition();
        let json = serde_json::to_string(&names_def).unwrap();
        let parsed: NamesDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.pools.len(), names_def.pools.len());

        let teams_def = default_teams_definition();
        let json2 = serde_json::to_string(&teams_def).unwrap();
        let parsed2: TeamsDefinition = serde_json::from_str(&json2).unwrap();
        assert_eq!(parsed2.teams.len(), teams_def.teams.len());
    }

    #[test]
    fn test_default_names_include_british_home_nation_pools() {
        let names_def = default_names_definition();

        for code in ["ENG", "SCO", "WAL", "NIR", "IE", "GB"] {
            let pool = names_def
                .pools
                .get(code)
                .unwrap_or_else(|| panic!("missing pool {code}"));
            assert!(
                !pool.first_names.is_empty(),
                "pool {code} should have first names"
            );
            assert!(
                !pool.last_names.is_empty(),
                "pool {code} should have last names"
            );
        }
    }
}
