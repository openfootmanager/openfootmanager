pub mod clubs;
pub mod competition_def;
pub(crate) mod data;
pub mod definitions;
pub mod file_format;
mod generation;
pub mod package;
pub mod world_io;

pub use clubs::WorldGenConfig;
pub use competition_def::*;
pub use definitions::*;
pub use file_format::{load_definition_file, parse_definition_str};
pub use package::{
    hash_package_file, is_unreadable, load_world_package, load_world_package_files,
    load_world_package_from_ofm,
    merge_world_packages, read_logo_from_ofm, read_package_manifest_from_ofm, validate_package_stack,
    validate_references, ConflictSeverity, ConfederationDef, CountryDef, PackageError, PackageInfo,
    PackageLock, PlayerDef, StaffDef, StackConflict, WorldMetaDef, WorldPackage, MAX_ARCHIVE_BYTES,
};
pub use world_io::*;

use domain::league::{CompetitionFormat, CompetitionScope};
use domain::player::{Player, Position};
use domain::staff::{Staff, StaffRole};
use domain::team::Team;
use domain::team::TeamColors;
use log::info;
use rand::RngExt;
use uuid::Uuid;

use chrono::Datelike;
use generation::*;

const MAX_OPENING_EXPIRING_CONTRACTS: usize = 2;
const MIN_OPENING_RUNWAY_WEEKS: i64 = 16;
const OPENING_SHORT_CONTRACT_END: &str = "2027-06-30";
const OPENING_YOUTH_ACADEMY_SIZE: usize = 3;
const OPENING_YOUTH_MAX_AGE: i32 = 21;
const AVAILABLE_STAFF_MARKET_ROTATION_DAYS: i64 = 30;

fn standard_available_staff_roles() -> [StaffRole; 12] {
    [
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
    ]
}

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
    let country_codes = sorted_country_codes(&names_def);
    let nationality = nationality_override
        .map(generation::canonicalize_generated_nationality)
        .unwrap_or_else(|| pick_nationality_from_def(&team.country, &country_codes, &mut rng));
    let youth_slots = youth_slots_for_target(target_position.map(Position::to_group_position));
    let slot_index = youth_slots[rng.random_range(0..youth_slots.len())];
    let mut player =
        generate_random_player_from_def(&team.id, slot_index, &nationality, &names_def, &mut rng);
    player.squad_role = SquadRole::Youth;
    player.transfer_listed = false;
    player.loan_listed = false;
    player
}

/// Generate a senior free-agent player for a national squad. `squad_slot`
/// follows the standard squad layout (GK 0-1, DEF 2-8, MID 9-15, FWD 16-21)
/// and drives the position; the player belongs to no club and holds no
/// contract, so clubs may sign them afterwards.
pub fn generate_national_team_player(nationality: &str, squad_slot: usize) -> Player {
    let mut rng = rand::rng();
    let names_def = default_names_definition();
    let nationality = generation::canonicalize_generated_nationality(nationality);
    // Avoid the youth-reserved slots so the player generates at a senior age.
    let slot = senior_slot(squad_slot % SQUAD_SLOTS);
    let mut player =
        generate_random_player_from_def("national-pool", slot, &nationality, &names_def, &mut rng);
    player.team_id = None;
    player.contract_end = None;
    player.wage = 0;
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

fn team_staff_seed_nationality(team: &Team) -> &str {
    if team.football_nation.is_empty() {
        team.country.as_str()
    } else {
        team.football_nation.as_str()
    }
}

/// Country codes from the name pools in a stable, sorted order.
///
/// `pools` is a `HashMap`, and Rust randomizes key-iteration order per
/// instance. Generation indexes into these codes with the RNG, so without a
/// stable order the same seed produces different worlds. Sorting is what makes
/// seeded world generation actually reproducible.
fn sorted_country_codes(names_def: &definitions::NamesDefinition) -> Vec<String> {
    let mut codes: Vec<String> = names_def.pools.keys().cloned().collect();
    codes.sort();
    codes
}

fn create_staff_generator_context() -> (definitions::NamesDefinition, Vec<String>) {
    let names_def = default_names_definition();
    let country_codes = sorted_country_codes(&names_def);
    (names_def, country_codes)
}

fn generate_missing_team_staff(world: &mut WorldData) -> bool {
    let mut rng = rand::rng();
    let (names_def, country_codes) = create_staff_generator_context();
    let mut generated_staff = Vec::new();
    let roles = [
        StaffRole::AssistantManager,
        StaffRole::Coach,
        StaffRole::Scout,
        StaffRole::Physio,
    ];

    for team in &world.teams {
        for role in &roles {
            let has_role = world.staff.iter().any(|staff_member| {
                staff_member.team_id.as_deref() == Some(team.id.as_str())
                    && &staff_member.role == role
            });
            if has_role {
                continue;
            }

            let nationality = pick_nationality_from_def(
                team_staff_seed_nationality(team),
                &country_codes,
                &mut rng,
            );
            generated_staff.push(generate_random_staff_from_def(
                &team.id,
                role.clone(),
                &nationality,
                &names_def,
                &mut rng,
            ));
        }
    }

    let changed = !generated_staff.is_empty();
    world.staff.extend(generated_staff);
    changed
}

fn generate_standard_available_staff_for_teams(teams: &[Team]) -> Vec<Staff> {
    let mut rng = rand::rng();
    let (names_def, country_codes) = create_staff_generator_context();
    let fallback_seed = teams
        .first()
        .map(team_staff_seed_nationality)
        .unwrap_or("England");

    standard_available_staff_roles()
        .into_iter()
        .map(|role| {
            let nationality = if country_codes.is_empty() {
                generation::canonicalize_generated_nationality(fallback_seed)
            } else {
                let seed_country = teams
                    .get(rng.random_range(0..teams.len().max(1)))
                    .map(team_staff_seed_nationality)
                    .unwrap_or(fallback_seed);
                pick_nationality_from_def(seed_country, &country_codes, &mut rng)
            };
            generate_random_staff_unattached_from_def(role, &nationality, &names_def, &mut rng)
        })
        .collect()
}

fn available_staff_count(staff: &[Staff]) -> usize {
    staff
        .iter()
        .filter(|staff_member| staff_member.team_id.is_none())
        .count()
}

fn replace_available_staff_market(staff: &mut Vec<Staff>, teams: &[Team]) {
    staff.retain(|staff_member| staff_member.team_id.is_some());
    staff.extend(generate_standard_available_staff_for_teams(teams));
}

pub fn replenish_available_staff_market(staff: &mut Vec<Staff>, teams: &[Team]) -> bool {
    if available_staff_count(staff) > 0 {
        return false;
    }

    replace_available_staff_market(staff, teams);
    true
}

pub fn normalize_imported_world_for_career_start(world: &mut WorldData) {
    generate_missing_team_staff(world);
    let _ = replenish_available_staff_market(&mut world.staff, &world.teams);
}

pub fn process_available_staff_market(game: &mut crate::game::Game) -> bool {
    use chrono::NaiveDate;

    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let available_count = available_staff_count(&game.staff);

    if available_count == 0 {
        replace_available_staff_market(&mut game.staff, &game.teams);
        game.available_staff_market_last_activity_date = Some(today);
        return true;
    }

    let Some(last_activity) = game.available_staff_market_last_activity_date.as_deref() else {
        game.available_staff_market_last_activity_date = Some(today);
        return true;
    };

    let Ok(last_activity_date) = NaiveDate::parse_from_str(last_activity, "%Y-%m-%d") else {
        game.available_staff_market_last_activity_date = Some(today);
        return true;
    };

    let current_date = game.clock.current_date.date_naive();
    if (current_date - last_activity_date).num_days() < AVAILABLE_STAFF_MARKET_ROTATION_DAYS {
        return false;
    }

    replace_available_staff_market(&mut game.staff, &game.teams);
    game.available_staff_market_last_activity_date = Some(today);
    true
}

/// Ensure the unemployed manager and scout pools each meet a floor of `team_count * 2`.
///
/// Called at the end of every season after retiree conversion so there are always
/// enough candidates for the player to consider hiring.  The function only adds
/// entries — it never removes any.
pub fn replenish_manager_and_scout_market(game: &mut crate::game::Game) {
    let team_count = game.teams.len();
    let floor = team_count * 2;

    let user_manager_id = if game.manager_id.is_empty() {
        game.manager.id.clone()
    } else {
        game.manager_id.clone()
    };

    // --- Managers ---
    let unemployed_mgr_count = game
        .managers
        .iter()
        .filter(|m| m.id != user_manager_id && m.team_id.is_none())
        .count();

    if unemployed_mgr_count < floor {
        let needed = floor - unemployed_mgr_count;
        let (names_def, country_codes) = create_staff_generator_context();
        let current_year = game.clock.current_date.year() as u32;
        let mut rng = rand::rng();
        for _ in 0..needed {
            let nationality = if country_codes.is_empty() {
                "ENG".to_string()
            } else {
                let idx = rng.random_range(0..country_codes.len());
                country_codes[idx].clone()
            };
            let mgr = generation::generate_random_unemployed_manager(
                &nationality,
                &names_def,
                current_year,
                &mut rng,
            );
            game.managers.push(mgr);
        }
    }

    // --- Scouts ---
    let unemployed_scout_count = game
        .staff
        .iter()
        .filter(|s| s.team_id.is_none() && matches!(s.role, StaffRole::Scout))
        .count();

    if unemployed_scout_count < floor {
        let needed = floor - unemployed_scout_count;
        let (names_def, country_codes) = create_staff_generator_context();
        let mut rng = rand::rng();
        for _ in 0..needed {
            let nationality = if country_codes.is_empty() {
                "ENG".to_string()
            } else {
                let idx = rng.random_range(0..country_codes.len());
                country_codes[idx].clone()
            };
            let scout = generate_random_staff_unattached_from_def(
                StaffRole::Scout,
                &nationality,
                &names_def,
                &mut rng,
            );
            game.staff.push(scout);
        }
    }
}

// ---------------------------------------------------------------------------
// World generation
// ---------------------------------------------------------------------------

/// Generate a random world (raw tuple — used by `generate_world_data`).
/// Loads definition files from `data_dir` if provided; otherwise procedurally
/// generates the full standard world (every catalogued nation with a real
/// league pyramid). Entropy-seeded — each call (e.g. each "New Game") produces
/// a different world.
pub fn generate_world(
    data_dir: Option<&std::path::Path>,
) -> (Vec<domain::team::Team>, Vec<Player>, Vec<Staff>) {
    generate_world_with(&clubs::WorldGenConfig::standard(), data_dir)
}

/// Build a club (without players) from a definition. Uses the definition's
/// stable `id` when set (world packages); otherwise a fresh UUID.
fn build_team(tdef: &TeamDef, rng: &mut impl rand::Rng) -> domain::team::Team {
    let team_id = if tdef.id.is_empty() {
        Uuid::new_v4().to_string()
    } else {
        tdef.id.clone()
    };
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
        team_id,
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
    team.founded_year = tdef.founded_year.unwrap_or(rng.random_range(1880..1960));
    team.colors = TeamColors {
        primary: tdef.colors.primary.clone(),
        secondary: tdef.colors.secondary.clone(),
    };
    team.play_style = play_style_from_str(&tdef.play_style);
    team.media.logo = tdef.logo.clone();
    if let Some(ref pattern_str) = tdef.kit_pattern
        && let Ok(pattern) = pattern_str.parse()
    {
        team.kit_pattern = pattern;
    }
    team
}

/// Build a club with a full generated squad (22 players) and staff, normalised
/// to a sensible opening wage budget. Shared by the random world and world
/// packages.
fn build_club(
    tdef: &TeamDef,
    country_codes: &[String],
    names_def: &NamesDefinition,
    rng: &mut impl rand::Rng,
) -> (domain::team::Team, Vec<Player>, Vec<Staff>) {
    let mut team = build_team(tdef, rng);
    let team_id = team.id.clone();

    let mut team_players = Vec::with_capacity(SQUAD_SLOTS);
    for slot in 0..SQUAD_SLOTS {
        let nationality = pick_nationality_from_def(&tdef.country, country_codes, rng);
        let mut player =
            generate_random_player_from_def(&team_id, slot, &nationality, names_def, rng);
        if rng.random_range(0..100) < 12 {
            player.transfer_listed = true;
        } else if rng.random_range(0..100) < 8 {
            player.loan_listed = true;
        }
        team_players.push(player);
    }

    let mut team_staff = Vec::with_capacity(4);
    for role in [
        StaffRole::AssistantManager,
        StaffRole::Coach,
        StaffRole::Scout,
        StaffRole::Physio,
    ] {
        let nationality = pick_nationality_from_def(&tdef.country, country_codes, rng);
        team_staff.push(generate_random_staff_from_def(
            &team_id,
            role,
            &nationality,
            names_def,
            rng,
        ));
    }

    normalize_generated_team(&mut team, &mut team_players);
    (team, team_players, team_staff)
}

/// Position-group floor for a finished squad, or 0 for a group with no floor.
fn group_floor(group: &Position) -> usize {
    MIN_PLAYERS_PER_GROUP
        .iter()
        .find(|(position, _)| position == group)
        .map(|(_, floor)| *floor)
        .unwrap_or(0)
}

/// Index of a position group within [`MIN_PLAYERS_PER_GROUP`].
fn group_index(group: &Position) -> Option<usize> {
    MIN_PLAYERS_PER_GROUP
        .iter()
        .position(|(position, _)| position == group)
}

/// Drop generated backfill players — the ones no authored player displaced —
/// until the squad is down to `target`, so a club that already defines a full
/// squad opens with exactly the players its author wrote instead of being padded
/// out to the generated squad size (#349).
///
/// Never drops an authored player, and never takes a position group below its
/// [`MIN_PLAYERS_PER_GROUP`] floor: an author who writes 28 outfield players and
/// no goalkeeper still gets goalkeepers rather than an unplayable club. When
/// every remaining backfill player is holding up a floor the squad simply stays
/// above `target`.
///
/// Removal order is the group furthest above its floor first, and within a group
/// the least valuable backfill first — seniors before youth-aged prospects, so a
/// squad that still needs backfill keeps the players
/// [`seed_opening_youth_academy`] draws its opening academy from.
///
/// Note the deliberate consequence for a club whose authored squad is already
/// full and entirely senior: no backfill survives, so
/// [`seed_opening_youth_academy`] finds no youth-aged players and the club opens
/// without an academy. That is the authored squad winning, which is what #349
/// asks for — packages seed an academy by authoring `youth: true` players (the
/// world editor's Youth section) rather than by having one generated for them.
fn trim_backfill_players(players: &mut Vec<Player>, placed: &[bool], authored_count: usize) {
    let target = authored_count.max(SQUAD_SLOTS);
    if players.len() <= target {
        return;
    }

    let removable: Vec<usize> = (0..players.len())
        .filter(|index| !placed.get(*index).copied().unwrap_or(false))
        .collect();

    let mut doomed = vec![false; players.len()];
    let mut to_drop = players.len() - target;

    while to_drop > 0 {
        // Recount survivors each pass so the floors are honoured as we go.
        let mut counts = [0usize; MIN_PLAYERS_PER_GROUP.len()];
        for (index, player) in players.iter().enumerate() {
            if !doomed[index]
                && let Some(group) = group_index(&player.position.to_group_position())
            {
                counts[group] += 1;
            }
        }

        let pick = removable
            .iter()
            .copied()
            .filter(|index| !doomed[*index])
            .filter_map(|index| {
                let group = players[index].position.to_group_position();
                let surviving = group_index(&group).map(|slot| counts[slot]).unwrap_or(0);
                let surplus = surviving.checked_sub(group_floor(&group))?;
                (surplus > 0).then_some((index, surplus))
            })
            .max_by_key(|(index, surplus)| {
                (
                    *surplus,
                    !is_opening_youth_candidate(&players[*index]),
                    std::cmp::Reverse(players[*index].ovr),
                )
            });

        let Some((index, _)) = pick else {
            // Every remaining backfill player is propping up a group floor.
            break;
        };

        doomed[index] = true;
        to_drop -= 1;
    }

    let mut survives = doomed.into_iter().map(|dropped| !dropped);
    players.retain(|_| survives.next().unwrap_or(true));
}

/// Build a club for a world package: start from a generated squad, then swap
/// each hand-authored player in over a generated one of the same position (so
/// squad balance is kept), appending if the position is already full. Generated
/// players left over are then trimmed back by [`trim_backfill_players`] — the
/// generated squad is a backfill, not a floor to pad the authored squad on top of.
fn build_package_club(
    tdef: &TeamDef,
    authored: &[&package::PlayerDef],
    country_codes: &[String],
    names_def: &NamesDefinition,
    rng: &mut impl rand::Rng,
) -> (domain::team::Team, Vec<Player>, Vec<Staff>) {
    let (mut team, mut players, staff) = build_club(tdef, country_codes, names_def, rng);
    if authored.is_empty() {
        return (team, players, staff);
    }

    let mut placed = vec![false; players.len()];
    for def in authored {
        let authored_player = generate_player_from_def(def, &team.id, names_def, rng);
        let group = authored_player.position.to_group_position();
        let slot = players
            .iter()
            .enumerate()
            .find(|(index, player)| !placed[*index] && player.position.to_group_position() == group)
            .map(|(index, _)| index);
        match slot {
            Some(index) => {
                players[index] = authored_player;
                placed[index] = true;
            }
            None => {
                players.push(authored_player);
                placed.push(true);
            }
        }
    }

    trim_backfill_players(&mut players, &placed, authored.len());

    // Authored wages may differ from the players they replaced, so re-normalise
    // the opening wage budget to the final squad.
    normalize_generated_team(&mut team, &mut players);
    (team, players, staff)
}

/// Human-readable name for the built-in confederations, falling back to the id.
fn builtin_region_name(id: &str) -> String {
    match id {
        "europe" => "Europe",
        "south-america" => "South America",
        "north-america" => "North America",
        "central-america" => "Central America",
        "africa" => "Africa",
        "asia" => "Asia",
        "oceania" => "Oceania",
        other => other,
    }
    .to_string()
}

/// Build the region list for a package: one region per confederation, with each
/// club's country assigned to its confederation (package-defined first,
/// otherwise the built-in catalog).
fn regions_from_package(
    package: &package::WorldPackage,
    teams: &[domain::team::Team],
) -> Vec<WorldRegionDefinition> {
    use std::collections::{BTreeMap, BTreeSet, HashMap};

    let mut region_names: BTreeMap<String, String> = BTreeMap::new();
    let mut region_countries: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for confederation in &package.confederations {
        region_names.insert(confederation.id.clone(), confederation.name.clone());
        region_countries
            .entry(confederation.id.clone())
            .or_default();
    }

    let country_region: HashMap<&str, &str> = package
        .countries
        .iter()
        .filter(|country| !country.confederation.is_empty())
        .map(|country| (country.id.as_str(), country.confederation.as_str()))
        .collect();

    for team in teams {
        let code = if team.football_nation.is_empty() {
            team.country.as_str()
        } else {
            team.football_nation.as_str()
        };
        let region = country_region
            .get(code)
            .copied()
            .unwrap_or_else(|| crate::nations::region_for_code(code));
        region_names
            .entry(region.to_string())
            .or_insert_with(|| builtin_region_name(region));
        region_countries
            .entry(region.to_string())
            .or_default()
            .insert(code.to_string());
    }

    region_countries
        .into_iter()
        .map(|(id, codes)| WorldRegionDefinition {
            name: region_names.get(&id).cloned().unwrap_or_else(|| id.clone()),
            id,
            country_codes: codes.into_iter().collect(),
        })
        .collect()
}

/// Generate procedural filler club definitions for a given country code. Used
/// to pad thin packages that have only one authored team. Matches the country's
/// `NationGen` from the standard set when available; falls back to generic names.
fn filler_club_defs(country: &str, count: usize, rng: &mut impl rand::Rng) -> Vec<definitions::TeamDef> {
    const GENERIC_CITIES: &[&str] = &[
        "Northtown", "Eastford", "Westbridge", "Southport", "Riverside",
        "Hillside", "Lakewood", "Oakdale", "Greenfield", "Pinecrest",
        "Fairview", "Clearwater", "Springfield", "Millbrook", "Stonehaven",
    ];
    let known = clubs::STANDARD_NATIONS.iter().any(|n| n.code == country);
    let nation = clubs::STANDARD_NATIONS
        .iter()
        .find(|n| n.code == country)
        .map(|n| clubs::NationGen { tiers: 1, ..*n }) // force single-division
        .unwrap_or(clubs::NationGen {
            code: "??",
            style: clubs::NamingStyle::Generic,
            tiers: 1,
            strength: 3,
            cities: GENERIC_CITIES,
        });
    let config = clubs::WorldGenConfig {
        clubs_per_division: count,
        nations: vec![nation],
    };
    let mut defs = clubs::generate_club_defs(&config, rng);
    // When the authored team's country isn't in STANDARD_NATIONS, the generator
    // uses code "??" as a placeholder. Patch all generated defs to carry the
    // real country so filler clubs don't appear foreign in region lookups.
    if !known {
        for def in &mut defs {
            def.country = country.to_string();
        }
    }
    defs
}

/// Synthesise the fallback league played over `team_ids` when a database package
/// declares teams but no competitions. Author overrides (`cfg`) tune the name,
/// legs, and scope; each falls back to the built-in default when unset.
fn build_fallback_competition(
    cfg: Option<&package::FallbackLeagueConfig>,
    team_ids: Vec<String>,
) -> CompetitionDefinition {
    let custom_name = cfg.and_then(|c| c.name.clone()).filter(|n| !n.is_empty());
    // Only 1 (single) or 2 (double round-robin) are meaningful; ignore others.
    let legs = cfg
        .and_then(|c| c.legs)
        .filter(|&l| l == 1 || l == 2)
        .unwrap_or(2);
    let scope = cfg
        .and_then(|c| c.scope.clone())
        .unwrap_or(CompetitionScope::Domestic);

    CompetitionDefinition {
        id: "ofm-fallback-league".to_string(),
        // A custom name is used verbatim; otherwise keep the localized default
        // name (driven by name_key, with `name` as the raw fallback).
        name: custom_name.clone().unwrap_or_else(|| "Default League".to_string()),
        r#type: domain::league::CompetitionType::League,
        scope,
        priority: 10,
        format: FormatDef {
            kind: CompetitionFormat::LeagueTable,
            legs: Some(legs),
            group_size: None,
            qualifiers_per_group: None,
            best_third_qualifiers: None,
        },
        participants: ParticipantSpec {
            explicit: Some(team_ids),
            selector: None,
        },
        region_id: None,
        country_id: None,
        required_region_ids: Vec::new(),
        berths: Vec::new(),
        season_start_month: None,
        season_start_day: None,
        // Clear name_key when a custom name is set so the author's name isn't
        // overridden by the localized default.
        name_key: if custom_name.is_some() {
            None
        } else {
            Some("be.competition.fallbackLeagueName".to_string())
        },
        logo: None,
    }
}

/// Build a runnable [`WorldData`] from a validated world package: clubs with
/// generated squads, regions from the package's confederations/countries, and
/// the package's competitions as embedded definitions (resolved at game start).
/// Call only after [`package::load_world_package`] reports no errors.
pub fn build_world_data_from_package(package: &package::WorldPackage) -> WorldData {
    let mut rng = rand::rng();
    let names_def = {
        let mut merged = default_names_definition();
        if let Some(pkg_names) = &package.names {
            for (key, pool) in &pkg_names.pools {
                if !pool.first_names.is_empty() && !pool.last_names.is_empty() {
                    merged.pools.insert(key.clone(), pool.clone());
                }
            }
        }
        merged
    };
    let country_codes: Vec<String> = names_def.pools.keys().cloned().collect();

    // Group hand-authored players and staff by the club they belong to.
    let mut authored_by_club: std::collections::HashMap<&str, Vec<&package::PlayerDef>> =
        std::collections::HashMap::new();
    for player in &package.players {
        if !player.club.is_empty() {
            authored_by_club
                .entry(player.club.as_str())
                .or_default()
                .push(player);
        }
    }
    let mut authored_staff_by_club: std::collections::HashMap<&str, Vec<&package::StaffDef>> =
        std::collections::HashMap::new();
    for s in &package.staff {
        authored_staff_by_club
            .entry(s.club.as_str())
            .or_default()
            .push(s);
    }
    const NO_AUTHORED: &[&package::PlayerDef] = &[];
    const NO_AUTHORED_STAFF: &[&package::StaffDef] = &[];

    let mut teams = Vec::new();
    let mut players = Vec::new();
    let mut staff = Vec::new();
    for tdef in &package.teams {
        let authored = authored_by_club
            .get(tdef.id.as_str())
            .map(Vec::as_slice)
            .unwrap_or(NO_AUTHORED);
        let authored_staff = authored_staff_by_club
            .get(tdef.id.as_str())
            .map(Vec::as_slice)
            .unwrap_or(NO_AUTHORED_STAFF);
        let (team, team_players, mut team_staff) =
            build_package_club(tdef, authored, &country_codes, &names_def, &mut rng);
        // Replace auto-generated staff with authored versions, consuming each slot
        // at most once so multiple authored staff of the same role all survive.
        let mut replaced_staff_slots = vec![false; team_staff.len()];
        for sdef in authored_staff {
            let authored_member = generation::generate_staff_from_authored_def(
                sdef, Some(&team.id), &names_def, &mut rng,
            );
            // Only the original auto-generated slots are replacement candidates;
            // already-placed authored staff (appended below) are never overwritten.
            let slot = team_staff
                .iter()
                .take(replaced_staff_slots.len())
                .enumerate()
                .find(|(idx, s)| !replaced_staff_slots[*idx] && s.role == authored_member.role)
                .map(|(idx, _)| idx);
            if let Some(pos) = slot {
                team_staff[pos] = authored_member;
                replaced_staff_slots[pos] = true;
            } else {
                team_staff.push(authored_member);
            }
        }
        teams.push(team);
        players.extend(team_players);
        staff.extend(team_staff);
    }
    // Unattached authored staff (no club) go directly into the staff list.
    for sdef in package.staff.iter().filter(|s| s.club.is_empty()) {
        staff.push(generation::generate_staff_from_authored_def(sdef, None, &names_def, &mut rng));
    }

    let mut build_notices: Vec<String> = Vec::new();

    // Thin-package fill: exactly 1 authored team + no competitions → generate
    // THIN_PACKAGE_MIN_TEAMS-1 procedural opponents so the auto-fallback league
    // below can form a playable season. Gated on no competitions so a 1-team
    // package that defines its own competition doesn't get unexpected fillers.
    const THIN_PACKAGE_MIN_TEAMS: usize = 8; // double round-robin → 14 matchdays
    if package.competitions.is_empty() && teams.len() == 1 {
        build_notices.push("be.error.notice.fallbackTeamsFilled".to_string());
        let country = teams[0].country.clone();
        for def in &filler_club_defs(&country, THIN_PACKAGE_MIN_TEAMS - 1, &mut rng) {
            let (team, team_players, team_staff) =
                build_club(def, &country_codes, &names_def, &mut rng);
            teams.push(team);
            players.extend(team_players);
            staff.extend(team_staff);
        }
    }

    let regions = regions_from_package(package, &teams);

    let competition_definitions = if !package.competitions.is_empty() {
        Some(CompetitionDefinitionFile {
            format_version: SUPPORTED_DEFINITION_FORMAT_VERSION,
            competitions: package.competitions.clone(),
        })
    } else if teams.len() >= 2 {
        // Auto-fallback: synthesise a single-division league over all authored teams.
        // A `package_type: "database"` package without any competition defs is
        // probably a work-in-progress; generate a playable default so the game
        // can still start. The notice key is collected on WorldData.build_notices
        // for the frontend to surface; it is not persisted to the save.
        build_notices.push("be.error.notice.fallbackLeagueGenerated".to_string());
        let explicit: Vec<String> = teams.iter().map(|t| t.id.clone()).collect();
        let cfg = package.meta.as_ref().and_then(|m| m.fallback_league.as_ref());
        let fallback = build_fallback_competition(cfg, explicit);
        Some(CompetitionDefinitionFile {
            format_version: SUPPORTED_DEFINITION_FORMAT_VERSION,
            competitions: vec![fallback],
        })
    } else {
        None
    };

    let meta = package.meta.clone().unwrap_or_default();
    let mut world = WorldData {
        name: if meta.name.is_empty() {
            "Imported World".to_string()
        } else {
            meta.name.clone()
        },
        description: meta.description.clone(),
        teams,
        players,
        staff,
        competition_definitions,
        regions,
        default_active_regions: meta.default_active_regions.clone(),
        default_active_competitions: meta.default_active_competitions.clone(),
        extra_translations: package.extra_translations.clone(),
        build_notices,
        ..WorldData::default()
    };
    if let Some(base_year) = meta.base_year {
        world.metadata.base_year = Some(base_year);
    }
    world
}

/// Find a data file by stem, accepting JSON or YAML (`.json`/`.yaml`/`.yml`).
fn find_definition_file(dir: &std::path::Path, stem: &str) -> Option<std::path::PathBuf> {
    ["json", "yaml", "yml"]
        .iter()
        .map(|ext| dir.join(format!("{stem}.{ext}")))
        .find(|path| path.exists())
}

/// Generate a world from an explicit generation config (entropy-seeded). The
/// shipped game uses [`WorldGenConfig::standard`]; tests use a smaller config
/// for speed.
pub fn generate_world_with(
    config: &clubs::WorldGenConfig,
    data_dir: Option<&std::path::Path>,
) -> (Vec<domain::team::Team>, Vec<Player>, Vec<Staff>) {
    generate_world_with_rng(rand::rng(), config, data_dir)
}

/// Generate a world deterministically from `seed`. The same seed always
/// produces an identical world. Intended for reproducible tests/scenarios.
pub fn generate_world_seeded(
    seed: u64,
    data_dir: Option<&std::path::Path>,
) -> (Vec<domain::team::Team>, Vec<Player>, Vec<Staff>) {
    use rand::SeedableRng;
    generate_world_with_rng(
        rand::rngs::StdRng::seed_from_u64(seed),
        &clubs::WorldGenConfig::standard(),
        data_dir,
    )
}

/// Core world generation from an explicit rng and config.
fn generate_world_with_rng(
    mut rng: impl rand::Rng,
    config: &clubs::WorldGenConfig,
    data_dir: Option<&std::path::Path>,
) -> (Vec<domain::team::Team>, Vec<Player>, Vec<Staff>) {
    info!("[generator] generate_world: data_dir={:?}", data_dir);
    let mut teams_out = Vec::new();
    let mut players = Vec::new();
    let mut staff = Vec::new();

    // Load name pools (external JSON/YAML file → hardcoded fallback)
    let names_def = data_dir
        .and_then(|dir| find_definition_file(dir, "default_names"))
        .and_then(|path| {
            let result = load_names_definition(&path);
            if result.is_some() {
                info!("[generator] loaded names from {:?}", path);
            }
            result
        })
        .unwrap_or_else(default_names_definition);

    // Clubs come from an external curated JSON/YAML file when present, otherwise
    // from the procedural generator driven by `config`.
    let team_defs: Vec<TeamDef> = data_dir
        .and_then(|dir| find_definition_file(dir, "default_teams"))
        .and_then(|path| {
            let result = load_teams_definition(&path);
            if result.is_some() {
                info!("[generator] loaded teams from {:?}", path);
            }
            result
        })
        .map(|def| def.teams)
        .unwrap_or_else(|| clubs::generate_club_defs(config, &mut rng));

    let country_codes = sorted_country_codes(&names_def);

    for tdef in &team_defs {
        let (team, team_players, team_staff) =
            build_club(tdef, &country_codes, &names_def, &mut rng);
        players.extend(team_players);
        staff.extend(team_staff);
        teams_out.push(team);
    }

    // Generate free-agent staff
    for role in standard_available_staff_roles() {
        let nat = &country_codes[rng.random_range(0..country_codes.len())];
        let s = generate_random_staff_unattached_from_def(role, nat, &names_def, &mut rng);
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
    use crate::clock::GameClock;
    use crate::game::Game;
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::player::{Position, SquadRole};
    use domain::staff::{Staff, StaffAttributes, StaffRole};
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    // -- authored squad backfill trimming (#349) ----------------------------

    fn test_team_def() -> TeamDef {
        serde_json::from_value(serde_json::json!({
            "id": "fc-test",
            "name": "FC Test",
            "shortName": "TST",
            "city": "Testville",
            "country": "ENG",
            "colors": { "primary": "#ff0000", "secondary": "#ffffff" }
        }))
        .expect("team def should deserialize")
    }

    /// A senior authored player at `position`, tagged so tests can tell authored
    /// players apart from generated backfill via `match_name`.
    fn authored_player(index: usize, position: Position) -> package::PlayerDef {
        let mut def: package::PlayerDef =
            serde_json::from_str("{}").expect("PlayerDef should default");
        def.id = format!("authored-{index}");
        def.first_name = "Test".to_string();
        def.last_name = format!("Authored{index}");
        def.club = "fc-test".to_string();
        def.nationality = "ENG".to_string();
        def.position = position;
        def.date_of_birth = Some("1998-01-01".to_string());
        def.overall = Some(70);
        def
    }

    /// An authored squad with the given per-group counts, all senior.
    fn authored_squad(gk: usize, def: usize, mid: usize, fwd: usize) -> Vec<package::PlayerDef> {
        let groups = [
            (Position::Goalkeeper, gk),
            (Position::Defender, def),
            (Position::Midfielder, mid),
            (Position::Forward, fwd),
        ];
        let mut squad = Vec::new();
        for (position, count) in groups {
            for _ in 0..count {
                squad.push(authored_player(squad.len(), position.clone()));
            }
        }
        squad
    }

    fn build_test_package_club(authored: &[package::PlayerDef]) -> Vec<Player> {
        let tdef = test_team_def();
        let names_def = default_names_definition();
        let country_codes = sorted_country_codes(&names_def);
        let mut rng = StdRng::seed_from_u64(42);
        let refs: Vec<&package::PlayerDef> = authored.iter().collect();
        let (_team, players, _staff) =
            build_package_club(&tdef, &refs, &country_codes, &names_def, &mut rng);
        players
    }

    fn count_group(players: &[Player], group: Position) -> usize {
        players
            .iter()
            .filter(|player| player.position.to_group_position() == group)
            .count()
    }

    fn count_authored(players: &[Player]) -> usize {
        players
            .iter()
            .filter(|player| player.match_name.starts_with("Authored"))
            .count()
    }

    #[test]
    fn authored_full_squad_is_not_padded_with_generated_players() {
        // #349: a package defining a complete 28-man squad opened with 32 players
        // because generated players the authored squad never displaced survived.
        // The generated squad is a backfill, so those leftovers are trimmed.
        let authored = authored_squad(2, 4, 11, 11);
        let players = build_test_package_club(&authored);

        assert_eq!(players.len(), 28, "authored squad should not be padded");
        assert_eq!(count_authored(&players), 28, "every authored player kept");
    }

    #[test]
    fn small_authored_squad_is_still_topped_up() {
        // The flip side: a package that only names a handful of players still
        // gets a full, playable squad generated around them.
        let authored = authored_squad(1, 2, 1, 1);
        let players = build_test_package_club(&authored);

        assert_eq!(players.len(), SQUAD_SLOTS);
        assert_eq!(count_authored(&players), 5);
    }

    #[test]
    fn trim_never_leaves_an_authored_squad_without_goalkeepers() {
        // 28 authored outfield players and no keeper: trimming to the authored
        // count would open an unplayable club, so the goalkeeper floor wins and
        // the squad stays above target.
        let authored = authored_squad(0, 8, 10, 10);
        let players = build_test_package_club(&authored);

        assert_eq!(count_authored(&players), 28, "every authored player kept");
        assert_eq!(
            count_group(&players, Position::Goalkeeper),
            group_floor(&Position::Goalkeeper),
            "generated keepers backfill the missing group"
        );
        assert_eq!(players.len(), 30);
    }

    #[test]
    fn trim_keeps_the_opening_youth_academy_intact() {
        // Trimming prefers dropping senior backfill over youth-aged prospects, so
        // a club whose authored squad is all seniors still opens with an academy.
        let authored = authored_squad(10, 0, 0, 0);
        let players = build_test_package_club(&authored);

        assert_eq!(players.len(), SQUAD_SLOTS);
        assert_eq!(
            players
                .iter()
                .filter(|player| player.squad_role == SquadRole::Youth)
                .count(),
            OPENING_YOUTH_ACADEMY_SIZE,
            "youth-aged backfill should survive the trim"
        );
    }

    #[test]
    fn fallback_competition_uses_defaults_when_unconfigured() {
        let comp = build_fallback_competition(None, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(comp.name, "Default League");
        assert_eq!(
            comp.name_key.as_deref(),
            Some("be.competition.fallbackLeagueName")
        );
        assert_eq!(comp.format.legs, Some(2));
        assert_eq!(comp.scope, CompetitionScope::Domestic);
        assert_eq!(
            comp.participants.explicit,
            Some(vec!["a".to_string(), "b".to_string()])
        );
    }

    #[test]
    fn fallback_competition_honors_author_overrides() {
        let cfg = package::FallbackLeagueConfig {
            name: Some("Premier Division".to_string()),
            legs: Some(1),
            scope: Some(CompetitionScope::Continental),
        };
        let comp = build_fallback_competition(Some(&cfg), vec!["a".to_string()]);
        assert_eq!(comp.name, "Premier Division");
        // A custom name clears the localized default key so it isn't overridden.
        assert_eq!(comp.name_key, None);
        assert_eq!(comp.format.legs, Some(1));
        assert_eq!(comp.scope, CompetitionScope::Continental);
    }

    #[test]
    fn fallback_competition_ignores_out_of_range_legs() {
        let cfg = package::FallbackLeagueConfig { name: None, legs: Some(7), scope: None };
        let comp = build_fallback_competition(Some(&cfg), vec!["a".to_string()]);
        assert_eq!(comp.format.legs, Some(2)); // 7 is meaningless → default 2
    }

    #[test]
    fn fallback_competition_validates_for_every_allowed_scope() {
        let team_ids: std::collections::HashSet<&str> = ["a", "b"].into_iter().collect();
        let mut country_codes: std::collections::HashSet<&str> = std::collections::HashSet::new();
        let mut region_ids: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for nation in crate::nations::NATION_CATALOG {
            country_codes.insert(nation.code);
            region_ids.insert(nation.region_id);
        }
        let ctx = WorldValidationContext { team_ids, country_codes, region_ids };

        for scope in [
            CompetitionScope::Domestic,
            CompetitionScope::Regional,
            CompetitionScope::Continental,
            CompetitionScope::International,
        ] {
            let cfg = package::FallbackLeagueConfig { name: None, legs: None, scope: Some(scope.clone()) };
            let comp = build_fallback_competition(Some(&cfg), vec!["a".to_string(), "b".to_string()]);
            let file = CompetitionDefinitionFile {
                format_version: SUPPORTED_DEFINITION_FORMAT_VERSION,
                competitions: vec![comp],
            };
            let errors = validate_definitions(&file, &ctx);
            assert!(errors.is_empty(), "scope {scope:?} produced errors: {errors:?}");
        }
    }

    #[test]
    fn generate_national_team_player_is_a_senior_free_agent() {
        let player = generate_national_team_player("JP", 5);

        assert_eq!(player.nationality, "JP");
        assert_eq!(
            player.team_id, None,
            "national-pool players belong to no club"
        );
        assert_eq!(player.contract_end, None);
        assert_eq!(player.position, Position::Defender, "slot 5 is a defender");
        assert!(player.ovr > 0, "derived ratings must be computed");
        assert_eq!(player.squad_role, SquadRole::Senior);
    }

    #[test]
    fn test_generate_world_team_count() {
        let config = WorldGenConfig::compact();
        let expected = config.total_clubs();
        let (teams, players, staff) = generate_world_with(&config, None);
        assert_eq!(teams.len(), expected);
        assert_eq!(players.len(), expected * 22);
        assert_eq!(staff.len(), expected * 4 + 12);
    }

    #[test]
    fn standard_world_fills_every_nation_and_spans_confederations() {
        let config = WorldGenConfig::standard();
        let (teams, _, _) = generate_world_with(&config, None);
        assert_eq!(teams.len(), config.total_clubs());

        // Every configured nation fields at least a full division.
        for nation in &config.nations {
            let count = teams
                .iter()
                .filter(|team| team.football_nation == nation.code)
                .count();
            assert!(
                count >= config.clubs_per_division,
                "{} only generated {} clubs",
                nation.code,
                count
            );
        }
    }

    #[test]
    fn test_generate_world_all_players_assigned() {
        let (teams, players, _) = generate_world_with(&WorldGenConfig::compact(), None);
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
        let (teams, players, _) = generate_world_with(&WorldGenConfig::compact(), None);
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
            let (teams, players, _) = generate_world_with(&WorldGenConfig::compact(), None);
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
        let (teams, players, _) = generate_world_with(&WorldGenConfig::compact(), None);

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
            let (teams, players, _) = generate_world_with(&WorldGenConfig::compact(), None);
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
    fn test_pick_nationality_defaults_generated_gb_to_eng() {
        let mut rng = rand::rng();
        let codes = vec!["GB".to_string()];

        for _ in 0..100 {
            let nat = pick_nationality_from_def("Spain", &codes, &mut rng);
            assert!(nat == "ES" || nat == "ENG", "unexpected nationality: {nat}");
            assert_ne!(nat, "GB");
        }
    }

    #[test]
    fn test_youth_recruit_override_defaults_gb_to_eng() {
        let team = domain::team::Team::new(
            "team-1".to_string(),
            "London FC".to_string(),
            "LON".to_string(),
            "England".to_string(),
            "London".to_string(),
            "Ground".to_string(),
            20000,
        );

        let player = generate_youth_academy_recruit_with_nationality(&team, None, Some("GB"));

        assert_eq!(player.nationality, "ENG");
        assert_eq!(player.football_nation, "ENG");
    }

    #[test]
    fn test_youth_recruit_targets_goalkeeper() {
        let team = domain::team::Team::new(
            "team-1".to_string(),
            "London FC".to_string(),
            "LON".to_string(),
            "England".to_string(),
            "London".to_string(),
            "Ground".to_string(),
            20000,
        );

        for _ in 0..16 {
            let player = generate_youth_academy_recruit_with_nationality(
                &team,
                Some(&Position::Goalkeeper),
                None,
            );
            assert_eq!(
                player.position,
                Position::Goalkeeper,
                "targeted youth recruit must be a goalkeeper",
            );
            assert!(
                opening_player_age(&player.date_of_birth)
                    .is_some_and(|age| age <= OPENING_YOUTH_MAX_AGE),
                "targeted youth recruit must be youth-aged",
            );
        }
    }

    #[test]
    fn test_national_team_player_remaps_youth_goalkeeper_slot() {
        // Slot 1 is youth-reserved; the national-team generator must remap it to a
        // senior goalkeeper slot. The remapped slot keeps the goalkeeper group while
        // drawing from the senior age range, so over many draws the ages must reach
        // past the youth cap (the youth-reserved slot would cap every player at it).
        let mut saw_senior_age = false;
        for _ in 0..64 {
            let player = generate_national_team_player("GB", 1);
            assert_eq!(
                player.position,
                Position::Goalkeeper,
                "national-team slot 1 must produce a goalkeeper",
            );
            if opening_player_age(&player.date_of_birth)
                .is_some_and(|age| age > OPENING_YOUTH_MAX_AGE)
            {
                saw_senior_age = true;
            }
        }
        assert!(
            saw_senior_age,
            "national-team goalkeeper must draw from the senior age range",
        );
    }

    #[test]
    fn test_all_nationalities_use_short_uppercase_codes() {
        let (_, players, staff) = generate_world_with(&WorldGenConfig::compact(), None);
        for p in &players {
            assert!(
                p.nationality.len() == 2 || p.nationality.len() == 3,
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
            assert!(
                s.nationality.len() == 2 || s.nationality.len() == 3,
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
    fn generate_world_loads_a_yaml_teams_file() {
        let dir = std::env::temp_dir().join(format!("ofm-world-yaml-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("default_teams.yaml"),
            "teams:\n  - name: Istanbul United\n    city: Istanbul\n    country: TR\n    colors:\n      primary: \"#ff0000\"\n      secondary: \"#ffffff\"\n",
        )
        .unwrap();

        let (teams, players, _) = generate_world_with(&WorldGenConfig::compact(), Some(&dir));
        assert_eq!(
            teams.len(),
            1,
            "the YAML teams file should drive generation"
        );
        assert_eq!(teams[0].name, "Istanbul United");
        assert_eq!(players.len(), 22);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_world_data_wrapper() {
        let world = generate_world_data(None);
        assert_eq!(world.teams.len(), WorldGenConfig::standard().total_clubs());
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

    fn make_import_team(id: &str, name: &str) -> Team {
        let mut team = Team::new(
            id.to_string(),
            name.to_string(),
            name[..3].to_string(),
            "England".to_string(),
            "London".to_string(),
            format!("{name} Ground"),
            25_000,
        );
        team.football_nation = "ENG".to_string();
        team
    }

    fn make_import_player(
        team_id: &str,
        id: &str,
        position: Position,
        date_of_birth: &str,
    ) -> Player {
        let mut player = Player::new(
            id.to_string(),
            format!("{id} M"),
            format!("{id} Player"),
            date_of_birth.to_string(),
            "ENG".to_string(),
            position,
            domain::player::PlayerAttributes {
                pace: 62,
                stamina: 62,
                strength: 62,
                agility: 62,
                passing: 62,
                shooting: 62,
                tackling: 62,
                dribbling: 62,
                defending: 62,
                positioning: 62,
                vision: 62,
                decisions: 62,
                composure: 62,
                aggression: 50,
                teamwork: 62,
                leadership: 50,
                handling: 20,
                reflexes: 20,
                aerial: 62,
            },
        );
        player.team_id = Some(team_id.to_string());
        player.ovr = 62;
        player.potential = 68;
        player
    }

    fn make_import_staff(id: &str, team_id: Option<&str>, role: StaffRole) -> Staff {
        let mut staff = Staff::new(
            id.to_string(),
            "Pat".to_string(),
            "Staff".to_string(),
            "1980-01-01".to_string(),
            role,
            StaffAttributes {
                coaching: 60,
                judging_ability: 60,
                judging_potential: 60,
                physiotherapy: 60,
            },
        );
        staff.nationality = "ENG".to_string();
        staff.team_id = team_id.map(str::to_string);
        staff
    }

    fn make_roster_baseline_world_without_staff() -> WorldData {
        let teams = vec![
            make_import_team("team-1", "Alpha FC"),
            make_import_team("team-2", "Beta FC"),
        ];
        let mut players = Vec::new();
        for team in &teams {
            players.push(make_import_player(
                &team.id,
                &format!("{}-gk", team.id),
                Position::Goalkeeper,
                "1998-01-01",
            ));
            players.push(make_import_player(
                &team.id,
                &format!("{}-d1", team.id),
                Position::Defender,
                "2008-01-01",
            ));
            players.push(make_import_player(
                &team.id,
                &format!("{}-m1", team.id),
                Position::Midfielder,
                "2007-01-01",
            ));
            players.push(make_import_player(
                &team.id,
                &format!("{}-f1", team.id),
                Position::Forward,
                "2006-01-01",
            ));
            for index in 0..8 {
                players.push(make_import_player(
                    &team.id,
                    &format!("{}-senior-{index}", team.id),
                    Position::Defender,
                    "1997-01-01",
                ));
            }
        }

        WorldData {
            name: "Imported Baseline".to_string(),
            description: "Missing staff import".to_string(),
            teams,
            players,
            staff: vec![],
            managers: vec![],
            league: None,
            news: vec![],
            stats: domain::stats::StatsState::default(),
            world_history: domain::world_history::WorldHistoryArchive::default(),
            metadata: WorldDataMetadata::default(),
            ..Default::default()
        }
    }

    fn make_staff_market_game(available_staff: Vec<Staff>) -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap());
        let manager = Manager::new(
            "mgr-user".to_string(),
            "Alex".to_string(),
            "Boss".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        Game::new(
            clock,
            manager,
            vec![make_import_team("team-1", "Alpha FC")],
            vec![],
            available_staff,
            vec![],
        )
    }

    #[test]
    fn normalize_imported_world_backfills_missing_team_staff_and_available_pool() {
        let mut world = make_roster_baseline_world_without_staff();

        normalize_imported_world_for_career_start(&mut world);

        for team in &world.teams {
            for role in [
                StaffRole::AssistantManager,
                StaffRole::Coach,
                StaffRole::Scout,
                StaffRole::Physio,
            ] {
                let count = world
                    .staff
                    .iter()
                    .filter(|staff_member| {
                        staff_member.team_id.as_deref() == Some(team.id.as_str())
                            && staff_member.role == role
                    })
                    .count();
                assert_eq!(count, 1, "{} should have exactly one {:?}", team.name, role);
            }
        }
        assert_eq!(
            world
                .staff
                .iter()
                .filter(|staff_member| staff_member.team_id.is_none())
                .count(),
            12
        );
    }

    #[test]
    fn normalize_imported_world_preserves_existing_staff_and_available_pool() {
        let mut world = make_roster_baseline_world_without_staff();
        world.staff = vec![
            make_import_staff(
                "assistant-existing",
                Some("team-1"),
                StaffRole::AssistantManager,
            ),
            make_import_staff("free-existing", None, StaffRole::Scout),
        ];

        normalize_imported_world_for_career_start(&mut world);

        assert!(
            world
                .staff
                .iter()
                .any(|staff_member| staff_member.id == "assistant-existing")
        );
        assert!(
            world
                .staff
                .iter()
                .any(|staff_member| staff_member.id == "free-existing")
        );
        assert_eq!(
            world
                .staff
                .iter()
                .filter(|staff_member| staff_member.team_id.is_none())
                .count(),
            1
        );
    }

    #[test]
    fn process_available_staff_market_does_not_rotate_before_thirty_days() {
        let initial_staff = vec![make_import_staff("free-1", None, StaffRole::Coach)];
        let mut game = make_staff_market_game(initial_staff);
        game.available_staff_market_last_activity_date = Some("2026-07-03".to_string());

        let changed = process_available_staff_market(&mut game);

        assert!(!changed);
        assert_eq!(
            game.staff
                .iter()
                .filter(|staff_member| staff_member.team_id.is_none())
                .map(|staff_member| staff_member.id.as_str())
                .collect::<Vec<_>>(),
            vec!["free-1"]
        );
        assert_eq!(
            game.available_staff_market_last_activity_date.as_deref(),
            Some("2026-07-03")
        );
    }

    #[test]
    fn process_available_staff_market_rotates_after_thirty_days() {
        let initial_staff = vec![make_import_staff("free-1", None, StaffRole::Coach)];
        let mut game = make_staff_market_game(initial_staff);
        game.available_staff_market_last_activity_date = Some("2026-07-02".to_string());

        let changed = process_available_staff_market(&mut game);

        assert!(changed);
        assert_eq!(
            game.staff
                .iter()
                .filter(|staff_member| staff_member.team_id.is_none())
                .count(),
            12
        );
        assert!(
            !game
                .staff
                .iter()
                .any(|staff_member| staff_member.id == "free-1")
        );
        assert_eq!(
            game.available_staff_market_last_activity_date.as_deref(),
            Some("2026-08-01")
        );
    }

    #[test]
    fn process_available_staff_market_replenishes_when_empty() {
        let mut game = make_staff_market_game(vec![]);

        let changed = process_available_staff_market(&mut game);

        assert!(changed);
        assert_eq!(
            game.staff
                .iter()
                .filter(|staff_member| staff_member.team_id.is_none())
                .count(),
            12
        );
        assert_eq!(
            game.available_staff_market_last_activity_date.as_deref(),
            Some("2026-08-01")
        );
    }
}
