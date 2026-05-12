pub(crate) mod data;
pub mod definitions;
mod generation;
pub mod world_io;

pub use definitions::*;
pub use world_io::*;

use domain::player::{Player, Position};
use domain::staff::{Staff, StaffRole};
use domain::team::Team;
use domain::team::TeamColors;
use log::{debug, info};
use rand::RngExt;
use uuid::Uuid;

use generation::*;

const MAX_OPENING_EXPIRING_CONTRACTS: usize = 2;
const MIN_OPENING_RUNWAY_WEEKS: i64 = 16;
const OPENING_SHORT_CONTRACT_END: &str = "2027-06-30";
const OPENING_YOUTH_ACADEMY_SIZE: usize = 3;
const OPENING_YOUTH_MAX_AGE: i32 = 21;

fn target_wage_usage_percent(reputation: u32) -> i64 {
    if reputation >= 750 {
        95
    } else if reputation >= 550 {
        94
    } else {
        93
    }
}

fn normalized_wage_budget(annual_wage_bill: i64, reputation: u32) -> i64 {
    let annual_wage_bill = annual_wage_bill.max(0);
    let usage_target = target_wage_usage_percent(reputation);
    ((annual_wage_bill * 100) + usage_target - 1) / usage_target
}

fn normalize_opening_contracts(players: &mut [Player]) {
    let mut expiring_indices: Vec<usize> = players
        .iter()
        .enumerate()
        .filter(|(_, player)| player.contract_end.as_deref() == Some(OPENING_SHORT_CONTRACT_END))
        .map(|(index, _)| index)
        .collect();

    expiring_indices.sort_by_key(|index| players[*index].date_of_birth.clone());

    for index in expiring_indices
        .into_iter()
        .skip(MAX_OPENING_EXPIRING_CONTRACTS)
    {
        if let Some(contract_end) = players[index].contract_end.as_deref()
            && let Ok(year) = contract_end[0..4].parse::<i32>()
        {
            players[index].contract_end = Some(format!("{}-06-30", year + 1));
        }
    }
}

fn opening_player_age(date_of_birth: &str) -> Option<i32> {
    use chrono::{Datelike, NaiveDate};

    let opening_date = NaiveDate::from_ymd_opt(2026, 7, 1)?;
    let birth_date = NaiveDate::parse_from_str(date_of_birth, "%Y-%m-%d").ok()?;
    let mut age = opening_date.year() - birth_date.year();

    if (birth_date.month(), birth_date.day()) > (opening_date.month(), opening_date.day()) {
        age -= 1;
    }

    Some(age)
}

fn is_opening_youth_candidate(player: &Player) -> bool {
    use domain::player::Position;

    player.position != Position::Goalkeeper
        && opening_player_age(&player.date_of_birth).is_some_and(|age| age <= OPENING_YOUTH_MAX_AGE)
}

fn sort_opening_youth_indices(players: &[Player], indices: &mut [usize]) {
    indices.sort_by(|left, right| {
        players[*right]
            .date_of_birth
            .cmp(&players[*left].date_of_birth)
            .then_with(|| players[*left].ovr.cmp(&players[*right].ovr))
    });
}

fn apply_opening_youth_assignments(players: &mut [Player], candidate_indices: Vec<usize>) -> usize {
    use domain::player::SquadRole;

    let mut assigned = 0;

    for index in candidate_indices
        .into_iter()
        .take(OPENING_YOUTH_ACADEMY_SIZE)
    {
        if players[index].squad_role != SquadRole::Youth {
            players[index].squad_role = SquadRole::Youth;
            assigned += 1;
        }
    }

    assigned
}

fn seed_opening_youth_academy(players: &mut [Player]) {
    let mut eligible_indices: Vec<usize> = players
        .iter()
        .enumerate()
        .filter(|(_, player)| is_opening_youth_candidate(player))
        .map(|(index, _)| index)
        .collect();

    sort_opening_youth_indices(players, &mut eligible_indices);
    apply_opening_youth_assignments(players, eligible_indices);
}

pub fn repair_opening_youth_academies(game: &mut crate::game::Game) -> bool {
    use chrono::Duration;
    use domain::player::SquadRole;

    if game
        .players
        .iter()
        .any(|player| player.squad_role == SquadRole::Youth)
    {
        return false;
    }

    if game.clock.current_date > game.clock.start_date + Duration::days(30) {
        return false;
    }

    let team_ids: Vec<String> = game.teams.iter().map(|team| team.id.clone()).collect();
    let mut repaired = false;

    for team_id in team_ids {
        let mut candidate_indices: Vec<usize> = game
            .players
            .iter()
            .enumerate()
            .filter(|(_, player)| player.team_id.as_deref() == Some(team_id.as_str()))
            .filter(|(_, player)| is_opening_youth_candidate(player))
            .map(|(index, _)| index)
            .collect();

        sort_opening_youth_indices(&game.players, &mut candidate_indices);
        repaired |= apply_opening_youth_assignments(&mut game.players, candidate_indices) > 0;
    }

    repaired
}

pub fn generate_youth_academy_recruit(team: &Team, target_position: Option<&Position>) -> Player {
    generate_youth_academy_recruit_with_nationality(team, target_position, None)
}

pub fn generate_youth_academy_recruit_with_nationality(
    team: &Team,
    target_position: Option<&Position>,
    nationality_override: Option<&str>,
) -> Player {
    use domain::player::SquadRole;

    let mut rng = rand::rng();
    let names_def = default_names_definition();
    let country_codes: Vec<String> = names_def.pools.keys().cloned().collect();
    let nationality = nationality_override
        .map(str::to_string)
        .unwrap_or_else(|| pick_nationality_from_def(&team.country, &country_codes, &mut rng));
    let youth_slots: &[usize] = match target_position.map(Position::to_group_position) {
        Some(Position::Defender) => &[8],
        Some(Position::Midfielder) => &[15],
        Some(Position::Forward) => &[21],
        _ => &[8, 15, 21],
    };
    let slot_index = youth_slots[rng.random_range(0..youth_slots.len())];
    let mut player =
        generate_random_player_from_def(&team.id, slot_index, &nationality, &names_def, &mut rng);
    player.squad_role = SquadRole::Youth;
    player.transfer_listed = false;
    player.loan_listed = false;
    player
}

fn normalize_generated_team(team: &mut Team, players: &mut [Player]) {
    seed_opening_youth_academy(players);
    normalize_opening_contracts(players);

    let annual_wage_bill: i64 = players.iter().map(|player| player.wage as i64).sum();
    let weekly_wage_spend = (annual_wage_bill + 51) / 52;

    team.wage_budget = normalized_wage_budget(annual_wage_bill, team.reputation);
    team.finance = team
        .finance
        .max(weekly_wage_spend.saturating_mul(MIN_OPENING_RUNWAY_WEEKS));
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
    let teams_def = data_dir
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
        let team_player_start = players.len();

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

        normalize_generated_team(&mut team, &mut players[team_player_start..]);
        teams_out.push(team);
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
    use super::data::{NATIONALITY_POOLS, TEAM_TEMPLATES};
    use super::*;
    use domain::player::{Position, SquadRole};

    #[test]
    fn test_generate_world_team_count() {
        let (teams, players, staff) = generate_world(None);
        assert_eq!(teams.len(), 16);
        assert_eq!(players.len(), 16 * 22);
        assert_eq!(staff.len(), 16 * 4 + 12);
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
    fn test_generate_world_normalizes_opening_financials() {
        for _ in 0..8 {
            let (teams, players, _) = generate_world(None);
            for team in &teams {
                let annual_wages: i64 = players
                    .iter()
                    .filter(|player| player.team_id.as_deref() == Some(team.id.as_str()))
                    .map(|player| player.wage as i64)
                    .sum();
                let weekly_wage_spend = (annual_wages + 51) / 52;
                let usage_percent = (annual_wages * 100) / std::cmp::max(1, team.wage_budget);

                assert!(
                    annual_wages <= team.wage_budget,
                    "{} started over budget: wages={} budget={}",
                    team.name,
                    annual_wages,
                    team.wage_budget
                );
                assert!(
                    (90..=96).contains(&usage_percent),
                    "{} opened outside target wage band: {}%",
                    team.name,
                    usage_percent
                );
                assert!(
                    team.finance >= weekly_wage_spend * MIN_OPENING_RUNWAY_WEEKS,
                    "{} opened without the minimum wage runway",
                    team.name
                );
            }
        }
    }

    #[test]
    fn test_generate_world_seeds_opening_youth_academies() {
        let (teams, players, _) = generate_world(None);

        for team in &teams {
            let youth_players: Vec<_> = players
                .iter()
                .filter(|player| player.team_id.as_deref() == Some(team.id.as_str()))
                .filter(|player| player.squad_role == SquadRole::Youth)
                .collect();

            assert_eq!(
                youth_players.len(),
                OPENING_YOUTH_ACADEMY_SIZE,
                "{} should open with {} youth academy players",
                team.name,
                OPENING_YOUTH_ACADEMY_SIZE
            );
            assert!(
                youth_players.iter().all(|player| {
                    opening_player_age(&player.date_of_birth)
                        .is_some_and(|age| age <= OPENING_YOUTH_MAX_AGE)
                }),
                "{} has an overage opening youth player",
                team.name
            );
            assert!(
                youth_players
                    .iter()
                    .all(|player| player.position != Position::Goalkeeper),
                "{} should keep opening youth players in outfield reserve slots",
                team.name
            );
        }
    }

    #[test]
    fn test_generate_world_limits_immediate_contract_pressure() {
        for _ in 0..8 {
            let (teams, players, _) = generate_world(None);
            for team in &teams {
                let expiring_contracts = players
                    .iter()
                    .filter(|player| player.team_id.as_deref() == Some(team.id.as_str()))
                    .filter(|player| {
                        player.contract_end.as_deref() == Some(OPENING_SHORT_CONTRACT_END)
                    })
                    .count();

                assert!(
                    expiring_contracts <= MAX_OPENING_EXPIRING_CONTRACTS,
                    "{} started with {} immediate renewal cases",
                    team.name,
                    expiring_contracts
                );
            }
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
        let names: Vec<&str> = TEAM_TEMPLATES.iter().map(|t| t.name).collect();
        let unique: std::collections::HashSet<&str> = names.iter().cloned().collect();
        assert_eq!(names.len(), unique.len(), "Duplicate team names found");
    }

    #[test]
    fn test_world_data_wrapper() {
        let world = generate_world_data(None);
        assert_eq!(world.teams.len(), 16);
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
