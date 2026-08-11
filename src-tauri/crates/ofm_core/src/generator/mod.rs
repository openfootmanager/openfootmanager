pub mod clubs;
pub mod competition_def;
pub mod definitions;
pub mod file_format;
mod generation;
pub mod package;
pub mod scaffold;
pub mod world_io;

pub use clubs::WorldGenConfig;
pub use competition_def::*;
pub use definitions::*;
pub use file_format::{load_definition_file, parse_definition_str};
pub use package::{
    extract_package_assets, hash_package_file, is_manifest_metadata_error, is_unreadable,
    is_valid_package_id,
    load_world_package, load_world_package_files, load_world_package_from_ofm,
    qualify_package_asset_paths, merge_world_packages, read_logo_from_ofm,
    read_package_manifest_from_ofm, validate_manifest, validate_package,
    validate_package_stack,
    validate_references, ConflictSeverity, ConfederationDef, CountryDef, PackageError, PackageInfo,
    PackageLock, PlayerDef, StaffDef, StackConflict, WorldMetaDef, WorldPackage, MAX_ARCHIVE_BYTES,
    RESERVED_PACKAGE_ID,
};
pub use scaffold::{
    entity_template, manifest_json, names_json, new_package_meta, scaffold_package, slugify,
    EntityKind,
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

/// Earliest year a world may open in.
///
/// `baseYear` is author-supplied with no lower bound, and birth years are
/// derived by subtracting an age from the opening year, so an absurdly small
/// value underflows. Mirrors the career floor in `commands/game.rs`.
pub const MIN_OPENING_YEAR: u32 = 1900;

/// Latest year a world may open in.
///
/// `baseYear` is an unbounded `i32` in the manifest, and the year is formatted
/// straight into contract and birth dates. A wild value does not crash, but it
/// produces dates no parser accepts, which then degrade silently into
/// unparseable ages — better to pin the era into a range dates can represent.
///
/// Removable once either half of that stops being true: `baseYear` validated at
/// manifest load, so an out-of-range era is refused with a message instead of
/// silently clamped here; or dates carried as a real year type rather than a
/// four-character prefix, so the formatting has no range to fall out of.
pub const MAX_OPENING_YEAR: u32 = 2999;

/// The year a world opens in when nothing declares one.
///
/// Correct only for **contemporary** worlds. A historical world should never
/// reach this — it carries its era from the career start year, or failing that
/// from the package manifest's `baseYear`. Generation used to hard code a
/// literal year in place of this, which silently aged every historical
/// package's squads by decades and drifted out of date on its own.
///
/// The result is always within `MIN_OPENING_YEAR..=MAX_OPENING_YEAR`. It is read
/// from the system clock, which is not something this code controls — a machine
/// set to 1970 or to 5000 would otherwise hand a fully procedural world an era
/// the date formatting cannot represent, and `generate_world_with_rng` takes
/// this value with no clamp of its own.
pub fn default_opening_year() -> u32 {
    use chrono::Datelike;
    // Clamp in signed space first: casting a negative year to u32 would wrap it
    // into the far future rather than falling back to the floor.
    chrono::Utc::now()
        .year()
        .clamp(MIN_OPENING_YEAR as i32, MAX_OPENING_YEAR as i32) as u32
}

fn opening_player_age(date_of_birth: &str, opening_year: i32) -> Option<i32> {
    use chrono::{Datelike, NaiveDate};

    let opening_date = NaiveDate::from_ymd_opt(opening_year, 7, 1)?;
    let birth_date = NaiveDate::parse_from_str(date_of_birth, "%Y-%m-%d").ok()?;
    let mut age = opening_date.year() - birth_date.year();

    if (birth_date.month(), birth_date.day()) > (opening_date.month(), opening_date.day()) {
        age -= 1;
    }

    Some(age)
}

fn is_opening_youth_candidate(player: &Player, opening_year: i32) -> bool {
    use domain::player::Position;

    player.position != Position::Goalkeeper
        && opening_player_age(&player.date_of_birth, opening_year)
            .is_some_and(|age| age <= OPENING_YOUTH_MAX_AGE)
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

fn seed_opening_youth_academy(players: &mut [Player], opening_year: i32) {
    let mut eligible_indices: Vec<usize> = players
        .iter()
        .enumerate()
        .filter(|(_, player)| is_opening_youth_candidate(player, opening_year))
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

    let opening_year = game.clock.start_date.year();

    for team_id in team_ids {
        let mut candidate_indices: Vec<usize> = game
            .players
            .iter()
            .enumerate()
            .filter(|(_, player)| player.team_id.as_deref() == Some(team_id.as_str()))
            .filter(|(_, player)| is_opening_youth_candidate(player, opening_year))
            .map(|(index, _)| index)
            .collect();

        sort_opening_youth_indices(&game.players, &mut candidate_indices);
        repaired |= apply_opening_youth_assignments(&mut game.players, candidate_indices) > 0;
    }

    repaired
}

/// Generate a youth prospect who is joining **now**.
///
/// `current_year` is the year the recruit arrives, not the year the world opened:
/// a prospect scouted five seasons into a career is fifteen in *that* season. The
/// two coincide only for the opening intake, which is why the distinction is
/// worth naming — a call site that passed the world's opening year here would
/// quietly produce a squad of players five years too old.
pub fn generate_youth_academy_recruit(
    team: &Team,
    target_position: Option<&Position>,
    current_year: u32,
) -> Player {
    generate_youth_academy_recruit_with_nationality(team, target_position, None, current_year)
}

/// As [`generate_youth_academy_recruit`], with the prospect's nationality forced
/// rather than drawn from the club's country. See there for `current_year`.
pub fn generate_youth_academy_recruit_with_nationality(
    team: &Team,
    target_position: Option<&Position>,
    nationality_override: Option<&str>,
    current_year: u32,
) -> Player {
    use domain::player::SquadRole;

    let mut rng = rand::rng();
    let names_def = default_names_definition();
    let country_codes = generation::nationality_distribution();
    let nationality = nationality_override
        .map(generation::canonicalize_generated_nationality)
        .unwrap_or_else(|| {
            // `team_local_nationality`, not `team.country`: a club carries both a
            // location and a football identity, and where they differ the
            // football identity is the one a youth intake should draw on.
            pick_nationality_from_def(team_local_nationality(team), country_codes, &mut rng)
        });
    let youth_slots = youth_slots_for_target(target_position.map(Position::to_group_position));
    let slot_index = youth_slots[rng.random_range(0..youth_slots.len())];
    let mut player =
        generate_random_player_from_def(
            &team.id,
            slot_index,
            &nationality,
            current_year,
            &names_def,
            &mut rng,
        );
    player.squad_role = SquadRole::Youth;
    player.transfer_listed = false;
    player.loan_listed = false;
    player
}

/// Generate a senior free-agent player for a national squad. `squad_slot`
/// follows the standard squad layout (GK 0-1, DEF 2-8, MID 9-15, FWD 16-21)
/// and drives the position; the player belongs to no club and holds no
/// contract, so clubs may sign them afterwards.
pub fn generate_national_team_player(
    nationality: &str,
    squad_slot: usize,
    opening_year: u32,
) -> Player {
    let mut rng = rand::rng();
    let names_def = default_names_definition();
    let nationality = generation::canonicalize_generated_nationality(nationality);
    // Avoid the youth-reserved slots so the player generates at a senior age.
    let slot = senior_slot(squad_slot % SQUAD_SLOTS);
    let mut player =
        generate_random_player_from_def(
            "national-pool",
            slot,
            &nationality,
            opening_year,
            &names_def,
            &mut rng,
        );
    player.team_id = None;
    player.contract_end = None;
    player.wage = 0;
    player.transfer_listed = false;
    player.loan_listed = false;
    player
}

fn normalize_generated_team(team: &mut Team, players: &mut [Player], opening_year: i32) {
    seed_opening_youth_academy(players, opening_year);
    normalize_opening_contracts(players);

    let annual_wage_bill: i64 = players.iter().map(|player| player.wage as i64).sum();
    let weekly_wage_spend = (annual_wage_bill + 51) / 52;

    team.wage_budget = normalized_wage_budget(annual_wage_bill, team.reputation);
    team.finance = team
        .finance
        .max(weekly_wage_spend.saturating_mul(MIN_OPENING_RUNWAY_WEEKS));
}

/// The country a club's *people* should be drawn from.
///
/// `country` is where the club is; `football_nation` is the identity it plays
/// under, and the two differ (a Welsh club in the English pyramid). Anyone
/// generated for the club follows the football identity when it is set.
///
/// Named without `staff` because players use it too — the youth intake drew
/// straight from `team.country` and quietly disagreed with every other path.
fn team_local_nationality(team: &Team) -> &str {
    if team.football_nation.is_empty() {
        team.country.as_str()
    } else {
        team.football_nation.as_str()
    }
}

fn create_staff_generator_context() -> (definitions::NamesDefinition, Vec<String>) {
    let names_def = default_names_definition();
    (names_def, generation::nationality_distribution().clone())
}

fn generate_missing_team_staff(world: &mut WorldData, opening_year: u32) -> bool {
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
                team_local_nationality(team),
                &country_codes,
                &mut rng,
            );
            generated_staff.push(generate_random_staff_from_def(
                &team.id,
                role.clone(),
                &nationality,
                opening_year,
                &names_def,
                &mut rng,
            ));
        }
    }

    let changed = !generated_staff.is_empty();
    world.staff.extend(generated_staff);
    changed
}

fn generate_standard_available_staff_for_teams(teams: &[Team], opening_year: u32) -> Vec<Staff> {
    let mut rng = rand::rng();
    let (names_def, country_codes) = create_staff_generator_context();
    let fallback_seed = teams
        .first()
        .map(team_local_nationality)
        .unwrap_or("England");

    standard_available_staff_roles()
        .into_iter()
        .map(|role| {
            let nationality = if country_codes.is_empty() {
                generation::canonicalize_generated_nationality(fallback_seed)
            } else {
                let seed_country = teams
                    .get(rng.random_range(0..teams.len().max(1)))
                    .map(team_local_nationality)
                    .unwrap_or(fallback_seed);
                pick_nationality_from_def(seed_country, &country_codes, &mut rng)
            };
            generate_random_staff_unattached_from_def(
                role,
                &nationality,
                opening_year,
                &names_def,
                &mut rng,
            )
        })
        .collect()
}

fn available_staff_count(staff: &[Staff]) -> usize {
    staff
        .iter()
        .filter(|staff_member| staff_member.team_id.is_none())
        .count()
}

fn replace_available_staff_market(staff: &mut Vec<Staff>, teams: &[Team], opening_year: u32) {
    staff.retain(|staff_member| staff_member.team_id.is_some());
    staff.extend(generate_standard_available_staff_for_teams(teams, opening_year));
}

pub fn replenish_available_staff_market(
    staff: &mut Vec<Staff>,
    teams: &[Team],
    opening_year: u32,
) -> bool {
    if available_staff_count(staff) > 0 {
        return false;
    }

    replace_available_staff_market(staff, teams, opening_year);
    true
}

pub fn normalize_imported_world_for_career_start(world: &mut WorldData, opening_year: u32) {
    generate_missing_team_staff(world, opening_year);
    let _ = replenish_available_staff_market(&mut world.staff, &world.teams, opening_year);
}

pub fn process_available_staff_market(game: &mut crate::game::Game) -> bool {
    use chrono::NaiveDate;

    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let current_year = game.clock.current_date.year() as u32;
    let available_count = available_staff_count(&game.staff);

    if available_count == 0 {
        replace_available_staff_market(&mut game.staff, &game.teams, current_year);
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

    replace_available_staff_market(&mut game.staff, &game.teams, current_year);
    game.available_staff_market_last_activity_date = Some(today);
    true
}

/// Ensure the unemployed manager and scout pools each meet a floor of `team_count * 2`.
///
/// Called at the end of every season after retiree conversion so there are always
/// enough candidates for the player to consider hiring.  The function only adds
/// entries — it never removes any.
pub fn replenish_manager_and_scout_market(game: &mut crate::game::Game) {
    // The market is topped up while a career runs, so new faces are aged against
    // the running clock rather than the year the world opened in.
    let market_year = game.clock.current_date.year() as u32;
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
                market_year,
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
                market_year,
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
/// Generates the full standard world — every generation nation with a real
/// league pyramid — from the definition files `sources` resolves. Entropy-seeded,
/// so each call (e.g. each "New Game") produces a different world.
pub fn generate_world(
    sources: &definitions::DefinitionSources,
) -> (Vec<domain::team::Team>, Vec<Player>, Vec<Staff>) {
    // The config is built *from* the sources rather than before them: it now
    // carries the nations list, so resolving it first is what lets a player's
    // `default_nations` override actually reach generation.
    generate_world_with(&clubs::WorldGenConfig::standard_from(sources), sources)
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
    // Inclusive *and* ordered, because `rand` panics on any empty range and that
    // panic is uniquely expensive here: `build_team` runs inside `start_new_game`,
    // an async Tauri command, so unwinding leaves the IPC call unanswered and the
    // player watches a spinner forever with no error (#441).
    //
    // Inclusive fixes `[640, 640]`, a club pinned to one value, and makes the
    // authored upper bound reachable. Ordering the bounds is what makes the panic
    // unreachable rather than merely unlikely: package validation does reject
    // `min > max`, but `build_team` also serves the procedural generator and
    // `default_teams` definition files, which nothing validates. Two comparisons
    // are a cheap price for a failure mode with no diagnostic.
    let (fin_lo, fin_hi) = (fin_range[0].min(fin_range[1]), fin_range[0].max(fin_range[1]));
    let (rep_lo, rep_hi) = (rep_range[0].min(rep_range[1]), rep_range[0].max(rep_range[1]));
    team.finance = rng.random_range(fin_lo..=fin_hi);
    team.reputation = rng.random_range(rep_lo..=rep_hi);
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
    opening_year: u32,
    names_def: &NamesDefinition,
    rng: &mut impl rand::Rng,
) -> (domain::team::Team, Vec<Player>, Vec<Staff>) {
    let mut team = build_team(tdef, rng);
    let team_id = team.id.clone();

    let mut team_players = Vec::with_capacity(SQUAD_SLOTS);
    for slot in 0..SQUAD_SLOTS {
        let nationality = pick_nationality_from_def(&tdef.country, country_codes, rng);
        let mut player =
            generate_random_player_from_def(&team_id, slot, &nationality, opening_year, names_def, rng);
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
            opening_year,
            names_def,
            rng,
        ));
    }

    normalize_generated_team(&mut team, &mut team_players, opening_year as i32);
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
fn trim_backfill_players(
    players: &mut Vec<Player>,
    placed: &[bool],
    authored_count: usize,
    opening_year: i32,
) {
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
                    !is_opening_youth_candidate(&players[*index], opening_year),
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
    opening_year: u32,
    names_def: &NamesDefinition,
    rng: &mut impl rand::Rng,
) -> (domain::team::Team, Vec<Player>, Vec<Staff>) {
    let (mut team, mut players, staff) =
        build_club(tdef, country_codes, opening_year, names_def, rng);
    if authored.is_empty() {
        return (team, players, staff);
    }

    let mut placed = vec![false; players.len()];
    for def in authored {
        let authored_player =
            generate_player_from_def(def, &team.id, opening_year, names_def, rng);
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

    trim_backfill_players(&mut players, &placed, authored.len(), opening_year as i32);

    // Authored wages may differ from the players they replaced, so re-normalise
    // the opening wage budget to the final squad.
    normalize_generated_team(&mut team, &mut players, opening_year as i32);
    (team, players, staff)
}

/// Human-readable name for the built-in confederations, falling back to the id.
pub(super) fn builtin_region_name(id: &str) -> String {
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
fn filler_club_defs(
    country: &str,
    count: usize,
    sources: &definitions::DefinitionSources,
    rng: &mut impl rand::Rng,
) -> Vec<definitions::TeamDef> {
    // From `sources`, not the embedded default: a player who redefined a
    // nation's cities should see those cities in the clubs that pad a thin
    // package from that country too, not only in a fully generated world.
    let standard = clubs::WorldGenConfig::standard_from(sources);
    let known = standard.nations.iter().any(|n| n.code == country);
    let nation = standard
        .nations
        .iter()
        .find(|n| n.code == country)
        .map(|n| clubs::NationGen { tiers: 1, ..n.clone() }) // force single-division
        .unwrap_or_else(|| clubs::NationGen {
            code: "??".to_string(),
            style: clubs::NamingStyle::Generic,
            tiers: 1,
            strength: 3,
            cities: standard.generic_cities.clone(),
        });
    let config = clubs::WorldGenConfig {
        clubs_per_division: count,
        nations: vec![nation],
        color_palette: standard.color_palette.clone(),
        generic_cities: standard.generic_cities.clone(),
    };
    let mut defs = clubs::generate_club_defs(&config, rng);
    // When the authored team's country isn't a generation nation, the generator
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
/// Build world data from a package. `opening_year` is the year the career will
/// actually start in and always wins, because the clock is what the player
/// experiences; a package that declares `baseYear` supplies it when the caller
/// has no opinion (inspection, tooling), and failing both, the real calendar
/// year. Ages, contracts and market values are all measured from it, so this is
/// what keeps a 1962 database from generating players born in the 1990s.
pub fn build_world_data_from_package(
    package: &package::WorldPackage,
    opening_year: Option<u32>,
    sources: &definitions::DefinitionSources,
) -> WorldData {
    let opening_year = opening_year
        .or_else(|| {
            package
                .meta
                .as_ref()
                .and_then(|meta| meta.base_year)
                // Clamp while the value is still signed. Casting first would
                // discard a negative `baseYear` entirely and fall through to the
                // contemporary default, so `baseYear: -50` opened a modern world
                // while `baseYear: 5` correctly clamped to the floor — the same
                // authoring mistake landing in two different eras.
                .map(|year| year.clamp(MIN_OPENING_YEAR as i32, MAX_OPENING_YEAR as i32) as u32)
        })
        .unwrap_or_else(default_opening_year)
        .clamp(MIN_OPENING_YEAR, MAX_OPENING_YEAR);
    let mut rng = rand::rng();
    let names_def = {
        // From `sources`, so a package's own pools layer on top of whatever the
        // player resolved rather than on top of the embedded copy — the same
        // definitions the rest of generation is using.
        let mut merged = definitions::names_definition(sources);
        if let Some(pkg_names) = &package.names {
            for (key, pool) in &pkg_names.pools {
                if !pool.first_names.is_empty() && !pool.last_names.is_empty() {
                    merged.pools.insert(key.clone(), pool.clone());
                }
            }
        }
        merged
    };
    // The catalog distribution plus whatever countries this package declares.
    // A package exists to describe a world the catalog does not contain, so its
    // own countries have to be drawable — otherwise a club in one of them is
    // squadded entirely from elsewhere, and the country it declared is a label
    // no player ever carries.
    let country_codes = generation::nationality_distribution_including(
        package.countries.iter().map(|country| country.id.as_str()),
    );

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
            build_package_club(tdef, authored, &country_codes, opening_year, &names_def, &mut rng);
        // Replace auto-generated staff with authored versions, consuming each slot
        // at most once so multiple authored staff of the same role all survive.
        let mut replaced_staff_slots = vec![false; team_staff.len()];
        for sdef in authored_staff {
            let authored_member = generation::generate_staff_from_authored_def(
                sdef,
                Some(&team.id),
                opening_year,
                &names_def,
                &mut rng,
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
        staff.push(generation::generate_staff_from_authored_def(
            sdef,
            None,
            opening_year,
            &names_def,
            &mut rng,
        ));
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
        for def in &filler_club_defs(&country, THIN_PACKAGE_MIN_TEAMS - 1, sources, &mut rng) {
            let (team, team_players, team_staff) =
                build_club(def, &country_codes, opening_year, &names_def, &mut rng);
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

/// Generate a world from an explicit generation config (entropy-seeded). The
/// shipped game uses [`WorldGenConfig::standard`]; tests use a smaller config
/// for speed.
pub fn generate_world_with(
    config: &clubs::WorldGenConfig,
    sources: &definitions::DefinitionSources,
) -> (Vec<domain::team::Team>, Vec<Player>, Vec<Staff>) {
    generate_world_with_rng(rand::rng(), config, sources)
}

/// Generate a world deterministically from `seed`. The same seed always
/// produces an identical world. Intended for reproducible tests/scenarios.
pub fn generate_world_seeded(
    seed: u64,
    sources: &definitions::DefinitionSources,
) -> (Vec<domain::team::Team>, Vec<Player>, Vec<Staff>) {
    use rand::SeedableRng;
    generate_world_with_rng(
        rand::rngs::StdRng::seed_from_u64(seed),
        &clubs::WorldGenConfig::standard_from(sources),
        sources,
    )
}

/// Core world generation from an explicit rng and config.
fn generate_world_with_rng(
    mut rng: impl rand::Rng,
    config: &clubs::WorldGenConfig,
    sources: &definitions::DefinitionSources,
) -> (Vec<domain::team::Team>, Vec<Player>, Vec<Staff>) {
    // A procedurally generated world is contemporary, so it opens in the real
    // calendar year rather than a year baked in at build time.
    let opening_year = default_opening_year();
    let mut teams_out = Vec::new();
    let mut players = Vec::new();
    let mut staff = Vec::new();

    // Name pools: a player's override if there is one, else the shipped file.
    let names_def = definitions::names_definition(sources);

    // Clubs are always generated from `config`, which is itself built from the
    // nations definition. A curated club list is hand-authored content and
    // belongs in a `.ofm` package: as a definition file it *replaced*
    // procedural generation outright, so shipping one would silently cut a
    // ~440-club world down to whatever the file happened to hold.
    let team_defs: Vec<TeamDef> = clubs::generate_club_defs(config, &mut rng);

    let country_codes = generation::nationality_distribution();

    for tdef in &team_defs {
        let (team, team_players, team_staff) =
            build_club(tdef, country_codes, opening_year, &names_def, &mut rng);
        players.extend(team_players);
        staff.extend(team_staff);
        teams_out.push(team);
    }

    // Generate free-agent staff
    for role in standard_available_staff_roles() {
        let nat = &country_codes[rng.random_range(0..country_codes.len())];
        let s =
            generate_random_staff_unattached_from_def(role, nat, opening_year, &names_def, &mut rng);
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
    use super::*;
    use crate::clock::GameClock;
    use crate::game::Game;
    use chrono::{TimeZone, Utc};

    /// Opening year for tests that need *an* era, not a specific one.
    const TEST_OPENING_YEAR: u32 = 2026;

    /// The season the motivating historical world models.
    const HISTORICAL_OPENING_YEAR: u32 = 1962;

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

    /// A club whose reputation is pinned to a single value — one rating typed into
    /// both the min and max boxes in the world editor, or a bulk import mapping a
    /// single club rating onto both bounds — must still build. `[640, 640]` reached
    /// `rng.random_range(640..640)`, an empty range, which `rand` asserts on; the
    /// panic unwound inside an async Tauri command, so no IPC response was ever
    /// sent and world creation hung forever rather than reporting an error (#441).
    #[test]
    fn a_club_pinned_to_one_value_still_builds() {
        let mut rng = StdRng::seed_from_u64(7);
        let mut tdef = test_team_def();
        tdef.reputation_range = Some([640, 640]);
        tdef.finance_range = Some([1_000_000, 1_000_000]);

        let team = build_team(&tdef, &mut rng);

        assert_eq!(team.reputation, 640);
        assert_eq!(team.finance, 1_000_000);
    }

    /// `..=` alone does not close the panic class — `rand` treats `900..=300` as
    /// empty and asserts on it exactly as it did on `640..640`. Package validation
    /// rejects a reversed range, but `build_team` also serves the procedural
    /// generator and `default_teams` definition files, which nothing validates, so
    /// the ordering has to happen here rather than be assumed of the caller.
    #[test]
    fn a_club_with_reversed_bounds_builds_instead_of_panicking() {
        let mut rng = StdRng::seed_from_u64(13);
        let mut tdef = test_team_def();
        tdef.reputation_range = Some([900, 300]);
        tdef.finance_range = Some([9_000_000, 1_000_000]);

        let team = build_team(&tdef, &mut rng);

        assert!((300..=900).contains(&team.reputation));
        assert!((1_000_000..=9_000_000).contains(&team.finance));
    }

    /// The upper bound must be reachable, not just non-panicking: a two-value range
    /// has to be able to produce both values, or every such club silently pins to
    /// the minimum.
    #[test]
    fn team_ranges_can_reach_their_upper_bound() {
        let mut rng = StdRng::seed_from_u64(11);
        let mut tdef = test_team_def();
        // Two-value ranges, so "drew the top" cannot be confused with "drew
        // somewhere in the middle". Both fields changed, so both are asserted.
        tdef.reputation_range = Some([500, 501]);
        tdef.finance_range = Some([1_000_000, 1_000_001]);

        let (saw_top_reputation, saw_top_finance) = (0..64).fold(
            (false, false),
            |(reputation, finance), _| {
                let team = build_team(&tdef, &mut rng);
                (
                    reputation || team.reputation == 501,
                    finance || team.finance == 1_000_001,
                )
            },
        );

        assert!(
            saw_top_reputation,
            "501 should be reachable from the reputation range [500, 501]"
        );
        assert!(
            saw_top_finance,
            "1000001 should be reachable from the finance range [1000000, 1000001]"
        );
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
        build_test_package_club_in_year(authored, TEST_OPENING_YEAR)
    }

    /// An authored player born in `birth_year`, for era-sensitive assertions.
    fn authored_player_born(index: usize, position: Position, birth_year: i32) -> package::PlayerDef {
        let mut def = authored_player(index, position);
        def.date_of_birth = Some(format!("{birth_year}-10-23"));
        def
    }

    #[test]
    fn authored_player_ages_against_the_world_opening_year() {
        // Pelé was 21 in 1962. Generation used to measure every authored player
        // against a hardcoded modern year, which made him 86 and crushed his
        // value and wage into the "past it" bracket.
        let authored = vec![authored_player_born(0, Position::Forward, 1940)];

        let players = build_test_package_club_in_year(&authored, HISTORICAL_OPENING_YEAR);
        let star = players
            .iter()
            .find(|player| player.match_name == "Authored0")
            .expect("authored player should be in the squad");

        let age = opening_player_age(&star.date_of_birth, HISTORICAL_OPENING_YEAR as i32)
            .expect("authored dob should parse");
        assert_eq!(age, 21, "a 1940-born player is 21 in 1962");

        let contract_end_year: i32 = star
            .contract_end
            .as_deref()
            .expect("authored player should get a contract")[0..4]
            .parse()
            .expect("contract end should start with a year");
        assert!(
            (HISTORICAL_OPENING_YEAR as i32..HISTORICAL_OPENING_YEAR as i32 + 10)
                .contains(&contract_end_year),
            "contract should expire in the world's era, got {contract_end_year}"
        );
    }

    #[test]
    fn generated_squad_filler_is_born_in_the_world_era() {
        // Authored squads are topped up to SQUAD_SLOTS. Those fillers used to be
        // born in the 1990s-2000s regardless of era, so a 1962 club fielded
        // players who would not be born for another thirty years.
        let authored = vec![authored_player_born(0, Position::Forward, 1940)];

        let players = build_test_package_club_in_year(&authored, HISTORICAL_OPENING_YEAR);
        let filler: Vec<&Player> = players
            .iter()
            .filter(|player| player.match_name != "Authored0")
            .collect();

        assert!(!filler.is_empty(), "squad should be topped up with filler");
        for player in filler {
            let birth_year: i32 = player.date_of_birth[0..4]
                .parse()
                .expect("generated dob should start with a year");
            assert!(
                birth_year < HISTORICAL_OPENING_YEAR as i32,
                "{} was born in {birth_year}, after a {HISTORICAL_OPENING_YEAR} world opened",
                player.full_name
            );
        }
    }

    #[test]
    fn youth_eligibility_is_measured_from_the_world_opening_year() {
        // An 18-year-old in 1962 was born in 1944. Measured against a hardcoded
        // modern year they read as 82, so no authored historical player could
        // ever qualify for the opening academy.
        let authored = vec![authored_player_born(0, Position::Forward, 1944)];
        let players = build_test_package_club_in_year(&authored, HISTORICAL_OPENING_YEAR);
        let prospect = players
            .iter()
            .find(|player| player.match_name == "Authored0")
            .expect("authored player should be in the squad");

        assert!(
            is_opening_youth_candidate(prospect, HISTORICAL_OPENING_YEAR as i32),
            "a 1944-born player is 18 in 1962 and belongs in the academy",
        );
        assert!(
            !is_opening_youth_candidate(prospect, TEST_OPENING_YEAR as i32),
            "the same player is far too old to be a prospect in a modern world",
        );
    }

    fn build_test_package_club_in_year(
        authored: &[package::PlayerDef],
        opening_year: u32,
    ) -> Vec<Player> {
        let tdef = test_team_def();
        let names_def = default_names_definition();
        let country_codes = generation::nationality_distribution();
        let mut rng = StdRng::seed_from_u64(42);
        let refs: Vec<&package::PlayerDef> = authored.iter().collect();
        let (_team, players, _staff) = build_package_club(
            &tdef,
            &refs,
            country_codes,
            opening_year,
            &names_def,
            &mut rng,
        );
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
    fn authored_overall_round_trips_to_the_same_position_weighted_rating() {
        // A package author who writes `overall: 86` must get a player the game
        // rates ~86 in their position. `attributes_for_overall` shapes the
        // spread and `ovr_from_attributes` scores it; the two have to agree, or
        // every authored squad drifts off the rating its author intended.
        use rand::SeedableRng;

        let positions = [
            Position::Goalkeeper,
            Position::CenterBack,
            Position::RightBack,
            Position::DefensiveMidfielder,
            Position::CentralMidfielder,
            Position::AttackingMidfielder,
            Position::RightWinger,
            Position::Striker,
        ];

        for position in &positions {
            for target in [55u8, 70, 86] {
                let runs = 200u64;
                let mut sum = 0f64;
                for seed in 0..runs {
                    let mut rng = StdRng::seed_from_u64(seed);
                    let attrs = generation::attributes_for_overall(target, position, &mut rng);
                    sum += crate::player_rating::ovr_from_attributes(&attrs, position);
                }
                let mean = sum / runs as f64;
                let drift = (mean - target as f64).abs();
                assert!(
                    drift <= 3.0,
                    "{position:?} authored at {target} averages {mean:.1} (drift {drift:.1})",
                );
            }
        }
    }

    #[test]
    fn specialist_keeper_is_valued_on_keeping_not_on_outfield_attributes() {
        // Market value and wage used to be sized from a flat average of eleven
        // outfield attributes that excluded handling, reflexes and aerial
        // entirely. An elite keeper therefore priced like a journeyman: in the
        // 1962 world that surfaced this, an 86-rated Gilmar was valued below
        // every outfielder in the squad, including a 78-rated full-back.
        let mut keeper = authored_player(0, Position::Goalkeeper);
        keeper.date_of_birth = Some("1994-01-01".to_string());
        keeper.overall = None;
        keeper.attributes = Some(domain::player::PlayerAttributes {
            pace: 58, stamina: 70, strength: 74, agility: 84,
            passing: 58, shooting: 20, tackling: 30, dribbling: 30,
            defending: 55, positioning: 88, vision: 68, decisions: 84,
            composure: 88, aggression: 45, teamwork: 78, leadership: 84,
            handling: 87, reflexes: 90, aerial: 83,
        });

        // A striker of comparable standing, same age.
        let mut striker = authored_player(1, Position::Striker);
        striker.date_of_birth = Some("1994-01-01".to_string());
        striker.overall = Some(86);

        let players = build_test_package_club(&[keeper, striker]);
        let keeper = players
            .iter()
            .find(|p| p.match_name == "Authored0")
            .expect("keeper should be in the squad");
        let striker = players
            .iter()
            .find(|p| p.match_name == "Authored1")
            .expect("striker should be in the squad");

        assert!(
            keeper.ovr >= 80,
            "the keeper should rate as elite, got {}",
            keeper.ovr,
        );
        assert!(
            keeper.market_value * 2 >= striker.market_value,
            "an elite keeper ({} ovr, {}) must not be valued a fraction of a \
             comparable striker ({} ovr, {})",
            keeper.ovr,
            keeper.market_value,
            striker.ovr,
            striker.market_value,
        );
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
        let player = generate_national_team_player("JP", 5, TEST_OPENING_YEAR);

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

    /// #452's headline: a generated world could only ever contain ~16
    /// nationalities — 14 of them European — because the draw was over the 17
    /// name-pool keys rather than over the nations that exist.
    #[test]
    fn a_generated_world_is_not_limited_to_the_name_pool_nationalities() {
        let (_teams, players, staff) = generate_world_with(&WorldGenConfig::standard(), None);

        let nationalities: std::collections::HashSet<&str> = players
            .iter()
            .map(|player| player.nationality.as_str())
            .chain(staff.iter().map(|member| member.nationality.as_str()))
            .collect();

        assert!(
            nationalities.len() > 40,
            "a world should field far more than the 16 name-pool nationalities, got {}: {:?}",
            nationalities.len(),
            nationalities
        );

        // The old pool was 14 European nations plus BR and AR, so "some
        // non-European nationality exists" is the assertion that actually
        // distinguishes the fix from the bug.
        let beyond_the_old_pool = nationalities.iter().any(|code| {
            !matches!(*code, "BR" | "AR")
                && crate::nations::region_for_code(code) != "europe"
                && crate::nations::nation_by_code(code).is_some()
        });
        assert!(
            beyond_the_old_pool,
            "no nationality outside the old Europe+BR/AR pool: {nationalities:?}"
        );
    }

    #[test]
    fn test_generate_world_team_count() {
        let config = WorldGenConfig::compact();
        let expected = config.total_clubs();
        let (teams, players, staff) = generate_world_with(&config, &definitions::DefinitionSources::embedded_only());
        assert_eq!(teams.len(), expected);
        assert_eq!(players.len(), expected * 22);
        assert_eq!(staff.len(), expected * 4 + 12);
    }

    #[test]
    fn standard_world_fills_every_nation_and_spans_confederations() {
        let config = WorldGenConfig::standard();
        let (teams, _, _) = generate_world_with(&config, &definitions::DefinitionSources::embedded_only());
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
        let (teams, players, _) = generate_world_with(&WorldGenConfig::compact(), &definitions::DefinitionSources::embedded_only());
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
        let (teams, players, _) = generate_world_with(&WorldGenConfig::compact(), &definitions::DefinitionSources::embedded_only());
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
            let (teams, players, _) = generate_world_with(&WorldGenConfig::compact(), &definitions::DefinitionSources::embedded_only());
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
        let (teams, players, _) = generate_world_with(&WorldGenConfig::compact(), &definitions::DefinitionSources::embedded_only());

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
                    opening_player_age(&player.date_of_birth, TEST_OPENING_YEAR as i32)
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
            let (teams, players, _) = generate_world_with(&WorldGenConfig::compact(), &definitions::DefinitionSources::embedded_only());
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

    /// The fallback used to take `pools.keys().min()` — the lexicographically
    /// smallest key, which with the shipped pools is always `AR`. Every one of the
    /// ~190 nationalities without a pool of its own was therefore given Argentine
    /// names, deterministically.
    #[test]
    fn an_unpooled_nationality_is_not_pinned_to_one_pool() {
        let names_def = default_names_definition();
        // Only the AR-exclusive names, because half of AR's first names also
        // appear in European pools (Federico, Lucas, Sergio …). Asserting "no
        // Argentine name at all" would fail on those shared entries and prove
        // nothing about which pool was chosen.
        let others: std::collections::HashSet<&String> = names_def
            .pools
            .iter()
            .filter(|(code, _)| *code != "AR")
            .flat_map(|(_, pool)| pool.first_names.iter())
            .collect();
        let ar_only: std::collections::HashSet<&String> = names_def.pools["AR"]
            .first_names
            .iter()
            .filter(|name| !others.contains(*name))
            .collect();
        assert!(!ar_only.is_empty(), "the AR pool must have exclusive names");
        let mut rng = StdRng::seed_from_u64(5);

        let drawn: Vec<String> = (0..64)
            .map(|_| pick_name_from_def("PL", &names_def, &mut rng).0)
            .collect();

        assert!(
            !drawn.iter().any(|name| ar_only.contains(name)),
            "a Polish player drew an Argentine-only name: {drawn:?}"
        );
    }

    /// The region preference has to be shown working on a nationality whose region
    /// is *not* `europe`: `region_for_code` answers `europe` for anything it does
    /// not recognise, so a European test case cannot tell a successful lookup
    /// apart from everything falling to the default. Uruguay is South American and
    /// unpooled, and `AR`/`BR` are the only South American pools shipped.
    #[test]
    fn the_name_fallback_resolves_a_non_default_region() {
        let names_def = default_names_definition();
        let south_american: std::collections::HashSet<&String> = ["AR", "BR"]
            .iter()
            .flat_map(|code| names_def.pools[*code].first_names.iter())
            .collect();
        let mut rng = StdRng::seed_from_u64(17);

        for _ in 0..64 {
            let (first, _) = pick_name_from_def("UY", &names_def, &mut rng);
            assert!(
                south_american.contains(&first),
                "Uruguay should borrow from a South American pool, drew {first}"
            );
        }
    }

    /// A pool whose key is not a catalogued nation cannot be region-matched, and
    /// must not be treated as European just because the lookup defaults there —
    /// otherwise a package keyed `BRA`/`JPN` hands a Pole a Japanese name while
    /// a Brazilian matches none of them.
    #[test]
    fn uncatalogued_pool_keys_are_not_filed_as_european() {
        let pools = std::collections::HashMap::from([
            (
                "JPN".to_string(),
                definitions::NamePool {
                    first_names: vec!["Kenji".to_string()],
                    last_names: vec!["Sato".to_string()],
                },
            ),
            (
                "SE".to_string(),
                definitions::NamePool {
                    first_names: vec!["Erik".to_string()],
                    last_names: vec!["Svensson".to_string()],
                },
            ),
        ]);
        let names_def = definitions::NamesDefinition {
            version: 1,
            description: String::new(),
            pools,
        };
        let mut rng = StdRng::seed_from_u64(23);

        for _ in 0..32 {
            let (first, _) = pick_name_from_def("PL", &names_def, &mut rng);
            assert_eq!(
                first, "Erik",
                "only the catalogued European pool should match a European nationality"
            );
        }
    }

    /// A nationality with no pool should borrow from its own part of the world.
    /// `BR` sorts before `SE`, so the old lexicographic fallback handed a Polish
    /// player a Brazilian name.
    #[test]
    fn the_name_fallback_prefers_a_pool_from_the_same_region() {
        let pools = std::collections::HashMap::from([
            (
                "SE".to_string(),
                definitions::NamePool {
                    first_names: vec!["Erik".to_string()],
                    last_names: vec!["Svensson".to_string()],
                },
            ),
            (
                "BR".to_string(),
                definitions::NamePool {
                    first_names: vec!["Joao".to_string()],
                    last_names: vec!["Silva".to_string()],
                },
            ),
        ]);
        let names_def = definitions::NamesDefinition {
            version: 1,
            description: String::new(),
            pools,
        };
        let mut rng = StdRng::seed_from_u64(1);

        for _ in 0..32 {
            let (first, _) = pick_name_from_def("PL", &names_def, &mut rng);
            assert_eq!(
                first, "Erik",
                "a European nationality should borrow from the European pool"
            );
        }
    }

    /// With nothing in its own region to borrow from, any pool will do — but the
    /// choice must still vary rather than pinning to one key, and it must never
    /// panic on a pool set that cannot satisfy the region preference.
    #[test]
    fn the_name_fallback_still_works_when_no_regional_pool_exists() {
        let pools = std::collections::HashMap::from([(
            "SE".to_string(),
            definitions::NamePool {
                first_names: vec!["Erik".to_string()],
                last_names: vec!["Svensson".to_string()],
            },
        )]);
        let names_def = definitions::NamesDefinition {
            version: 1,
            description: String::new(),
            pools,
        };
        let mut rng = StdRng::seed_from_u64(2);

        let (first, last) = pick_name_from_def("JP", &names_def, &mut rng);

        assert_eq!(first, "Erik");
        assert_eq!(last, "Svensson");
    }

    #[test]
    fn test_pick_nationality_weighted() {
        let mut rng = rand::rng();
        let codes: Vec<String> =
            default_names_definition().pools.keys().cloned().collect();
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

    /// A club's location and its football identity can differ — a Welsh club in
    /// the English pyramid. Every other generated person follows the football
    /// identity; the youth intake read `team.country` directly and quietly
    /// disagreed.
    #[test]
    fn a_youth_recruit_follows_the_clubs_football_nation_not_its_location() {
        let mut team = domain::team::Team::new(
            "team-1".to_string(),
            "Wrexham".to_string(),
            "WRX".to_string(),
            "England".to_string(),
            "Wrexham".to_string(),
            "Ground".to_string(),
            20000,
        );
        team.football_nation = "WAL".to_string();

        let drawn: Vec<String> = (0..200)
            .map(|_| {
                generate_youth_academy_recruit_with_nationality(&team, None, None, TEST_OPENING_YEAR)
                    .nationality
            })
            .collect();

        let welsh = drawn.iter().filter(|code| *code == "WAL").count();
        let english = drawn.iter().filter(|code| *code == "ENG").count();
        // The local weight is 60%, so the football identity should dominate the
        // location outright rather than merely edge it.
        assert!(
            welsh > english,
            "the Welsh identity should drive the local draw: WAL {welsh}, ENG {english}"
        );
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

        let player =
            generate_youth_academy_recruit_with_nationality(&team, None, Some("GB"), TEST_OPENING_YEAR);

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
                TEST_OPENING_YEAR,
            );
            assert_eq!(
                player.position,
                Position::Goalkeeper,
                "targeted youth recruit must be a goalkeeper",
            );
            assert!(
                opening_player_age(&player.date_of_birth, TEST_OPENING_YEAR as i32)
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
            let player = generate_national_team_player("GB", 1, TEST_OPENING_YEAR);
            assert_eq!(
                player.position,
                Position::Goalkeeper,
                "national-team slot 1 must produce a goalkeeper",
            );
            if opening_player_age(&player.date_of_birth, TEST_OPENING_YEAR as i32)
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
        let (_, players, staff) = generate_world_with(&WorldGenConfig::compact(), &definitions::DefinitionSources::embedded_only());
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

    /// Tier 1 of the definition search path: a file the player dropped in their
    /// own data directory beats the shipped one.
    ///
    /// This replaces `generate_world_loads_a_yaml_teams_file`, which pinned the
    /// same mechanism for a curated *teams* file. That override is retired — it
    /// replaced procedural generation outright, so a shipped one would have cut
    /// a ~440-club world down to its own contents — and the capability now sits
    /// where it belongs, on the nations that drive generation.
    #[test]
    fn a_player_supplied_nations_file_overrides_the_shipped_one() {
        let dir = std::env::temp_dir().join(format!("ofm-world-yaml-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("default_nations.yaml"),
            "clubsPerDivision: 4\ncolorPalette:\n  - primary: \"#ff0000\"\n    secondary: \"#ffffff\"\ngenericCities: [Testville]\nnations:\n  - code: TR\n    style: Generic\n    tiers: 1\n    strength: 3\n    cities: [Istanbul, Ankara, Izmir, Bursa]\n",
        )
        .unwrap();

        let sources = definitions::DefinitionSources::searching([dir.clone()]);
        let (teams, players, _) =
            generate_world_with(&WorldGenConfig::standard_from(&sources), &sources);

        assert_eq!(teams.len(), 4, "the override should drive generation");
        assert!(
            teams.iter().all(|team| team.country == "TR"),
            "every club should come from the overridden nation: {:?}",
            teams.iter().map(|t| &t.country).collect::<Vec<_>>()
        );
        assert_eq!(players.len(), 4 * 22);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_world_data_wrapper() {
        let world = generate_world_data(&definitions::DefinitionSources::embedded_only());
        assert_eq!(world.teams.len(), WorldGenConfig::standard().total_clubs());
        assert!(!world.name.is_empty());
        assert!(!world.description.is_empty());
    }

    /// A definition must survive serialize → deserialize *field for field*.
    ///
    /// This used to compare only `pools.len()` / `teams.len()`, which is why
    /// nobody noticed that the shipped `default_teams.json` was written in
    /// snake_case while `TeamDef` serializes camelCase — it parsed through
    /// `#[serde(alias)]`, but re-serializing produced a different file than the
    /// one checked in. A length check cannot see that; this can.
    #[test]
    fn a_definition_survives_a_serde_round_trip_field_for_field() {
        let names_def = default_names_definition();
        let json = serde_json::to_string(&names_def).unwrap();
        let parsed: NamesDefinition = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.pools.len(), names_def.pools.len());
        for (code, pool) in &names_def.pools {
            let round_tripped = parsed.pools.get(code).expect("pool survives the round trip");
            assert_eq!(&round_tripped.first_names, &pool.first_names, "{code} first names");
            assert_eq!(&round_tripped.last_names, &pool.last_names, "{code} last names");
        }

        let nations_def = definitions::nations_definition(&definitions::DefinitionSources::embedded_only());
        let json = serde_json::to_string(&nations_def).unwrap();
        let parsed: definitions::NationsDefinition = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.nations.len(), nations_def.nations.len());
        for (before, after) in nations_def.nations.iter().zip(&parsed.nations) {
            assert_eq!(before.code, after.code);
            assert_eq!(before.cities, after.cities, "{} cities", before.code);
            assert_eq!(before.style, after.style, "{} style", before.code);
            assert_eq!(before.tiers, after.tiers, "{} tiers", before.code);
            assert_eq!(before.strength, after.strength, "{} strength", before.code);
        }
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

        normalize_imported_world_for_career_start(&mut world, TEST_OPENING_YEAR);

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

        normalize_imported_world_for_career_start(&mut world, TEST_OPENING_YEAR);

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
