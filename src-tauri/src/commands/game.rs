use log::info;
use std::sync::Arc;
use tauri::{Manager as TauriManager, State};

use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};

use db::{save_index::SaveEntry, save_manager::SaveManager};
use domain::league::{
    CompetitionFormat, CompetitionScope, CompetitionType, FixtureCompetition, League,
};
use domain::manager::Manager;
use domain::national_team::NationalTeam;
use domain::stats::StatsState;
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::state::StateManager;

use crate::SaveManagerState;

fn load_world_data_from_path(world_source: &str) -> Result<ofm_core::generator::WorldData, String> {
    let path = world_source.strip_prefix("file:").unwrap_or(world_source);
    ofm_core::generator::load_world_from_path(std::path::Path::new(path))
        .map_err(|_| "be.error.worldReadFileFailed".to_string())
}

/// Load a world from a modular package directory (recursively scanned, schema
/// typed). Rejects an invalid package so a broken mod never loads half-applied.
fn load_world_data_from_package(dir: &str) -> Result<ofm_core::generator::WorldData, String> {
    let (package, errors) = ofm_core::generator::load_world_package(std::path::Path::new(dir));
    if !errors.is_empty() {
        return Err("be.error.package.invalid".to_string());
    }
    ofm_core::generator::build_world_from_package(&package)
}

pub(crate) fn map_save_manager_lock_error<T>(
    result: std::sync::LockResult<T>,
) -> Result<T, String> {
    result.map_err(|_| "be.error.saveManagerUnavailable".to_string())
}

fn require_active_stats_state(state: &StateManager) -> Result<StatsState, String> {
    state
        .get_stats_state(|stats| stats.clone())
        .ok_or("be.error.noActiveStatsSession".to_string())
}

fn default_league_name() -> String {
    ["Premier", "Division"].join(" ")
}

const DEFAULT_GENERATED_HISTORY_DEPTH_YEARS: u32 = 12;
const MAX_GENERATED_HISTORY_DEPTH_YEARS: u32 = 24;

fn long_date_format() -> String {
    ['%', 'B', ' ', '%', 'd', ',', ' ', '%', 'Y']
        .into_iter()
        .collect()
}

pub(crate) fn default_save_name(manager_name: &str) -> String {
    let mut save_name = manager_name.to_string();
    save_name.push('\'');
    save_name.push('s');
    save_name.push(' ');
    save_name.push_str("Career");
    save_name
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawStartupOptions {
    #[serde(default)]
    start_year: Option<i32>,
    #[serde(default)]
    start_phase: Option<String>,
    #[serde(default)]
    history_depth_years: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StartPhase {
    SeasonStart,
    MidSeason,
}

impl StartPhase {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "seasonStart" => Some(Self::SeasonStart),
            "midSeason" => Some(Self::MidSeason),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::SeasonStart => "seasonStart",
            Self::MidSeason => "midSeason",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StartupOptions {
    start_year: i32,
    start_phase: StartPhase,
    history_depth_years: u32,
}

fn default_start_year() -> i32 {
    chrono::Utc::now().year().max(2020)
}

fn default_history_depth_years() -> u32 {
    DEFAULT_GENERATED_HISTORY_DEPTH_YEARS
}

fn start_date_for_year(start_year: i32) -> Result<chrono::DateTime<Utc>, String> {
    // Use June 1 in World Cup years so a fresh career opens just before the
    // tournament, keeping the WC in June rather than scheduling it in July.
    let month = if ofm_core::world_cup::is_world_cup_summer(start_year) {
        6
    } else {
        7
    };
    Utc.with_ymd_and_hms(start_year, month, 1, 0, 0, 0)
        .single()
        .ok_or_else(|| "be.error.createManager.invalidStartYear".to_string())
}

fn current_date_for_phase(
    start_year: i32,
    start_phase: StartPhase,
) -> Result<chrono::DateTime<Utc>, String> {
    let start_date = start_date_for_year(start_year)?;
    Ok(match start_phase {
        StartPhase::SeasonStart => start_date,
        StartPhase::MidSeason => start_date + Duration::days(120),
    })
}

fn age_on_date(birth_date: chrono::NaiveDate, reference_date: chrono::NaiveDate) -> i64 {
    let mut age = i64::from(reference_date.year() - birth_date.year());
    let has_had_birthday =
        (reference_date.month(), reference_date.day()) >= (birth_date.month(), birth_date.day());
    if !has_had_birthday {
        age -= 1;
    }
    age
}

pub(crate) fn start_phase_for_game(game: &Game) -> StartPhase {
    if game.clock.current_date > game.clock.start_date {
        StartPhase::MidSeason
    } else {
        StartPhase::SeasonStart
    }
}

fn preseason_season_start(clock: &GameClock) -> chrono::DateTime<Utc> {
    clock.start_date + Duration::days(30)
}

fn preseason_league_year(clock: &GameClock) -> u32 {
    let year = clock.start_date.year() + i32::from(clock.start_date.month() == 12);
    u32::try_from(year).unwrap_or(2020)
}

fn normalize_startup_options(raw: Option<RawStartupOptions>) -> Result<StartupOptions, String> {
    let raw = raw.unwrap_or_default();
    let start_year = raw.start_year.unwrap_or_else(default_start_year);
    if start_year < 2020 {
        return Err("be.error.createManager.startYearMin".to_string());
    }

    let start_phase = match raw.start_phase.as_deref() {
        None | Some("") => StartPhase::SeasonStart,
        Some(value) => StartPhase::parse(value)
            .ok_or_else(|| "be.error.createManager.invalidStartPhase".to_string())?,
    };
    let history_depth_years = raw
        .history_depth_years
        .unwrap_or_else(default_history_depth_years);
    if history_depth_years > MAX_GENERATED_HISTORY_DEPTH_YEARS {
        return Err("be.error.createManager.historyDepthMax".to_string());
    }

    Ok(StartupOptions {
        start_year,
        start_phase,
        history_depth_years,
    })
}

fn apply_generated_past_history(game: &mut Game, startup_options: &StartupOptions) {
    ofm_core::history_generation::generate_past_world_history(
        game,
        startup_options.start_year,
        startup_options.history_depth_years,
    );
}

fn load_world_data(world_source: Option<&str>) -> Result<ofm_core::generator::WorldData, String> {
    match world_source {
        None | Some("random") => Ok(ofm_core::generator::generate_world_data(None)),
        Some(source) => {
            let raw = source.strip_prefix("file:").unwrap_or(source);
            if std::path::Path::new(raw).is_dir() {
                load_world_data_from_package(raw)
            } else {
                load_world_data_from_path(source)
            }
        }
    }
}

/// Load world data from a stack of installed `.ofm` packages (by id).
/// Packages are merged in order with last-wins semantics for duplicate ids.
/// Also returns the package lockfile entries for saving alongside the game.
fn load_world_data_from_package_ids(
    packages_dir: &std::path::Path,
    package_ids: &[String],
) -> Result<(ofm_core::generator::WorldData, Vec<ofm_core::generator::PackageLock>), String> {
    let mut loaded = Vec::with_capacity(package_ids.len());
    let mut lockfile = Vec::with_capacity(package_ids.len());
    for id in package_ids {
        let path = packages_dir.join(format!("{id}.ofm"));
        let (pkg, errors) = ofm_core::generator::load_world_package_from_ofm(&path);
        if !errors.is_empty() {
            return Err("be.error.package.invalid".to_string());
        }
        let version = pkg.meta.as_ref().map(|m| m.version.clone()).unwrap_or_default();
        let hash = ofm_core::generator::hash_package_file(&path).unwrap_or_default();
        lockfile.push(ofm_core::generator::PackageLock { id: id.clone(), version, hash });
        loaded.push(pkg);
    }
    let (merged, errors) = ofm_core::generator::merge_world_packages(loaded);
    if !errors.is_empty() {
        return Err("be.error.package.invalid".to_string());
    }
    let world = ofm_core::generator::build_world_from_package(&merged)?;
    if world.teams.is_empty() {
        return Err("be.error.package.noDatabasePackage".to_string());
    }
    Ok((world, lockfile))
}

fn world_start_year(
    startup_options: &StartupOptions,
    metadata: &ofm_core::generator::WorldDataMetadata,
) -> i32 {
    match metadata.kind {
        ofm_core::generator::WorldDataKind::HistoricalSnapshot => {
            metadata.base_year.unwrap_or(startup_options.start_year)
        }
        ofm_core::generator::WorldDataKind::RosterBaseline => startup_options.start_year,
    }
}

fn game_clock_for_world(
    startup_options: &StartupOptions,
    metadata: &ofm_core::generator::WorldDataMetadata,
) -> Result<GameClock, String> {
    let start_year = world_start_year(startup_options, metadata);
    let mut clock = GameClock::new(start_date_for_year(start_year)?);
    clock.current_date = match metadata.kind {
        ofm_core::generator::WorldDataKind::HistoricalSnapshot => metadata
            .snapshot_date
            .as_deref()
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
            .map(|value| value.with_timezone(&Utc))
            .unwrap_or(current_date_for_phase(
                start_year,
                startup_options.start_phase,
            )?),
        ofm_core::generator::WorldDataKind::RosterBaseline => {
            current_date_for_phase(startup_options.start_year, startup_options.start_phase)?
        }
    };
    Ok(clock)
}

fn build_game_from_world_data(
    clock: GameClock,
    manager: Manager,
    startup_options: &StartupOptions,
    world: ofm_core::generator::WorldData,
) -> (Game, StatsState) {
    // Resolve any authored competition definitions while we still hold the
    // world (validation already passed at load). These replace the auto-built
    // foundation competitions.
    let game_start = clock.start_date;
    let defined_competitions: Vec<League> = world
        .competition_definitions
        .as_ref()
        .map(|file| {
            let mut comps = ofm_core::generator::resolve_definitions(
                file,
                &world,
                preseason_league_year(&clock),
                game_start,
            );
            for comp in &mut comps {
                let (_, is_mid_season) = ofm_core::generator::start_date_at_game_open(
                    game_start,
                    comp.season_start_month,
                    comp.season_start_day,
                );
                if is_mid_season {
                    ofm_core::catchup::simulate_past_fixtures(comp, &world.players, game_start);
                }
            }
            comps
        })
        .unwrap_or_default();

    let ofm_core::generator::WorldData {
        teams,
        players,
        staff,
        managers,
        competitions,
        national_teams,
        default_active_regions,
        default_active_competitions,
        league,
        news,
        stats,
        world_history,
        metadata,
        extra_translations,
        ..
    } = world;

    let mut game = Game::new(clock, manager, teams, players, staff, vec![]);
    if game
        .staff
        .iter()
        .any(|staff_member| staff_member.team_id.is_none())
    {
        game.available_staff_market_last_activity_date =
            Some(game.clock.current_date.format("%Y-%m-%d").to_string());
    }
    ofm_core::generator::repair_opening_youth_academies(&mut game);

    // Authored definitions take precedence over both the snapshot's stored
    // competitions and the auto-built foundations.
    let competitions = if defined_competitions.is_empty() {
        competitions
    } else {
        defined_competitions
    };

    match metadata.kind {
        ofm_core::generator::WorldDataKind::HistoricalSnapshot => {
            game.managers.extend(
                managers
                    .into_iter()
                    .filter(|existing_manager| existing_manager.id != game.manager.id),
            );
            game.competitions = competitions;
            game.national_teams = national_teams;
            game.active_region_ids = default_active_regions;
            game.active_competition_ids = default_active_competitions;
            game.league = league;
            game.promote_legacy_league();
            game.news = news;
            game.world_history = world_history;
            game.extra_translations = extra_translations;
            ensure_multi_competition_foundations(&mut game);
            ofm_core::season_context::refresh_game_context(&mut game);
            (game, stats)
        }
        ofm_core::generator::WorldDataKind::RosterBaseline => {
            // Authored definitions, if any, become the world's competitions;
            // otherwise ensure_multi_competition_foundations auto-builds them.
            game.competitions = competitions;
            game.extra_translations = extra_translations;
            // Build the league/division foundations *before* generating history so
            // each club's past seasons are attributed to its real ~20-team
            // division. Otherwise history runs with no competitions and treats the
            // whole world as one mega-league (≈880-match seasons).
            ensure_multi_competition_foundations(&mut game);
            apply_generated_past_history(&mut game, startup_options);
            (game, StatsState::default())
        }
    }
}

fn infer_region_id(country_code: &str) -> String {
    ofm_core::nations::region_for_code(country_code).to_string()
}

fn infer_team_region_id(team: &domain::team::Team) -> String {
    if !team.football_nation.is_empty() {
        return infer_region_id(&team.football_nation);
    }
    infer_region_id(&team.country)
}

fn competition_required_region_ids(competition: &League) -> Vec<String> {
    let mut region_ids = competition.required_region_ids.clone();
    if matches!(
        competition.scope,
        CompetitionScope::Domestic | CompetitionScope::Regional
    ) {
        if let Some(region_id) = &competition.region_id {
            region_ids.push(region_id.clone());
        }
    }
    region_ids.sort();
    region_ids.dedup();
    region_ids
}

fn build_national_teams(game: &Game) -> Vec<NationalTeam> {
    use std::collections::BTreeMap;

    let mut players_by_nation: BTreeMap<String, Vec<&domain::player::Player>> = BTreeMap::new();
    for player in &game.players {
        let nation = if player.football_nation.is_empty() {
            player.nationality.clone()
        } else {
            player.football_nation.clone()
        };
        players_by_nation.entry(nation).or_default().push(player);
    }

    players_by_nation
        .into_iter()
        .map(|(nation, mut players)| {
            players.sort_by(|left, right| right.ovr.cmp(&left.ovr));
            let nation_label = ofm_core::nations::nation_display_name(&nation);
            let mut national_team = NationalTeam::new(
                format!("nt-{}", nation.to_lowercase()),
                format!("{} National Team", nation_label),
                nation.clone(),
                Some(game.region_for_country(&nation)),
            );
            national_team.squad_player_ids = players
                .into_iter()
                .take(23)
                .map(|player| player.id.clone())
                .collect();
            national_team
        })
        .collect()
}

/// Pick continental-cup entrants: the strongest clubs by reputation from each
/// region, capped so the bracket stays manageable. Entrants are returned
/// strongest-first so the top seeds receive any knockout byes.
fn select_continental_entrants(
    teams: &[domain::team::Team],
    per_region: usize,
    max_entrants: usize,
) -> Vec<String> {
    use std::collections::BTreeMap;

    let reputation_then_id = |left: &&domain::team::Team, right: &&domain::team::Team| {
        right
            .reputation
            .cmp(&left.reputation)
            .then_with(|| left.id.cmp(&right.id))
    };

    let mut teams_by_region: BTreeMap<String, Vec<&domain::team::Team>> = BTreeMap::new();
    for team in teams {
        teams_by_region
            .entry(infer_team_region_id(team))
            .or_default()
            .push(team);
    }

    let mut entrants: Vec<&domain::team::Team> = Vec::new();
    for regional_teams in teams_by_region.values_mut() {
        regional_teams.sort_by(reputation_then_id);
        entrants.extend(regional_teams.iter().take(per_region).copied());
    }

    entrants.sort_by(reputation_then_id);
    entrants
        .into_iter()
        .take(max_entrants)
        .map(|team| team.id.clone())
        .collect()
}

/// Target number of clubs in a division. Countries are chunked into divisions
/// of this size: a 40-club major becomes two 20-club tiers, a 20-club nation a
/// single league. Smaller imported worlds run a single league per country.
const TOP_DIVISION_SIZE: usize = 20;

/// Stable id of the generated world's continental club competition.
const CONTINENTAL_CHAMPIONS_CUP_ID: &str = "continental-champions-cup";
/// Top finishers of each first division that earn a continental berth — matches
/// the inferred `CONTINENTAL_LEAGUE_SLOTS` so built-in qualification is unchanged.
const CONTINENTAL_QUALIFYING_POSITIONS: u32 = 4;

/// Split a country's clubs (passed strongest-first) into divisions of
/// `division_size`, strongest tier first. A trailing remainder smaller than
/// half a division is folded up so no tier is left tiny.
fn split_into_divisions(sorted_team_ids: &[String], division_size: usize) -> Vec<Vec<String>> {
    let division_size = division_size.max(2);
    if sorted_team_ids.len() <= division_size {
        return vec![sorted_team_ids.to_vec()];
    }
    let mut divisions: Vec<Vec<String>> = sorted_team_ids
        .chunks(division_size)
        .map(<[String]>::to_vec)
        .collect();
    if divisions.len() >= 2 && divisions.last().map(Vec::len).unwrap_or(0) < division_size / 2 {
        let tail = divisions.pop().expect("len >= 2");
        divisions.last_mut().expect("len >= 1").extend(tail);
    }
    divisions
}

fn division_tier_name(tier: usize, division_count: usize) -> &'static str {
    if division_count <= 1 {
        "League"
    } else if tier == 0 {
        "First Division"
    } else {
        "Second Division"
    }
}

fn division_tier_name_key(tier: usize, division_count: usize) -> &'static str {
    if division_count <= 1 {
        "tournaments.competitions.league"
    } else if tier == 0 {
        "tournaments.competitions.firstDivision"
    } else {
        "tournaments.competitions.secondDivision"
    }
}

/// Name a division within a country's pyramid.
fn division_name(country: &str, tier: usize, division_count: usize) -> String {
    format!("{country} {}", division_tier_name(tier, division_count))
}

/// Default league-start month for a region. South American leagues start in
/// March, Asian in February, Oceanian in October; everything else in August.
fn default_season_month_for_region(region_id: &str) -> u8 {
    match region_id {
        "south-america" => 3,
        "asia" => 2,
        "oceania" => 10,
        _ => 8,
    }
}

fn brazil_state_region(city: &str) -> Option<&'static str> {
    match city {
        "São Paulo" | "Rio" | "Belo Horizonte" | "Santos" | "Campinas" | "Bragantino"
        | "Juiz de Fora" | "Vitória" => Some("southeast"),
        "Porto Alegre" | "Curitiba" | "Florianópolis" => Some("south"),
        "Salvador" | "Recife" | "Fortaleza" | "Natal" | "Maceió" => Some("northeast"),
        "Goiânia" | "Belém" | "Manaus" | "Cuiabá" => Some("north-central-west"),
        _ => None,
    }
}

/// Build the generated world's competitions as `CompetitionDefinition`s with
/// explicit participant lists, paired with their staggered start dates. Built-in
/// competitions then flow through the same `build_explicit_competition` core as
/// imported definitions (see [`build_foundation_competitions`]).
///
/// `game_start` is the game anchor (July 1 in normal years; June 1 in World Cup
/// years so the WC opens in June). Each competition's start date is derived from
/// its region's default season month via
/// [`ofm_core::generator::start_date_at_game_open`].
/// When a player picks SeasonStart, find the team's primary competition and
/// return its actual calendar season-start date. Returns None when the
/// competition uses the default August start (no clock adjustment needed).
fn team_season_anchor(game: &Game, team_id: &str) -> Option<DateTime<Utc>> {
    let team = game.teams.iter().find(|team| team.id == team_id)?;
    let country = if team.football_nation.is_empty() {
        &team.country
    } else {
        &team.football_nation
    };
    if country == "BR" {
        let season_year = game.clock.start_date.year();
        return Utc
            .with_ymd_and_hms(season_year - 1, 12, 15, 0, 0, 0)
            .single();
    }
    let competition = game.competitions.iter().find(|c| {
        c.kind == CompetitionType::League && c.participant_ids.iter().any(|id| id == team_id)
    })?;
    if competition.region_id.as_deref() == Some("south-america") {
        return competition
            .fixtures
            .iter()
            .filter(|fixture| fixture.competition != FixtureCompetition::Friendly)
            .filter_map(|fixture| chrono::NaiveDate::parse_from_str(&fixture.date, "%Y-%m-%d").ok())
            .min()
            .and_then(|date| date.and_hms_opt(0, 0, 0))
            .map(|date| {
                DateTime::<Utc>::from_naive_utc_and_offset(date, Utc) - Duration::days(30)
            });
    }
    let month = competition.season_start_month;
    let day = competition.season_start_day;
    if month == 8 && day == 1 {
        return None; // northern-hemisphere default — no adjustment
    }
    let year = game.clock.start_date.year();
    Utc.with_ymd_and_hms(year, u32::from(month), u32::from(day), 0, 0, 0)
        .single()
}

fn build_foundation_competition_plan(
    game: &Game,
    game_start: DateTime<Utc>,
) -> Vec<(ofm_core::generator::CompetitionDefinition, DateTime<Utc>)> {
    use domain::league::{Berth, BerthRule};
    use ofm_core::generator::{CompetitionDefinition, FormatDef, ParticipantSpec};
    use std::collections::BTreeMap;

    // Default berth into the continental cup; reproduces the inferred field so a
    // freshly generated world's qualification is unchanged.
    let continental_berth = |rule: BerthRule| Berth {
        target: CONTINENTAL_CHAMPIONS_CUP_ID.to_string(),
        rule,
        fallback_to: None,
    };

    let make_format = |kind: CompetitionFormat| FormatDef {
        kind,
        legs: None,
        group_size: None,
        qualifiers_per_group: None,
        best_third_qualifiers: None,
    };

    let mut teams_by_country: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for team in &game.teams {
        teams_by_country
            .entry(team.football_nation.clone())
            .or_default()
            .push(team.id.clone());
    }

    let reputation: std::collections::HashMap<&str, u32> = game
        .teams
        .iter()
        .map(|team| (team.id.as_str(), team.reputation))
        .collect();

    let mut planned: Vec<(CompetitionDefinition, DateTime<Utc>)> = Vec::new();
    let mut priority = 0u32;
    for (country, mut team_ids) in teams_by_country {
        if team_ids.len() < 2 {
            continue;
        }
        // Strongest first so divisions are seeded by quality and cup byes go to
        // the best clubs.
        team_ids.sort_by(|left, right| {
            reputation
                .get(right.as_str())
                .cmp(&reputation.get(left.as_str()))
                .then_with(|| left.cmp(right))
        });
        let region_id = infer_region_id(&country);
        // Human-readable nation name for competition titles ("ES" → "Spain").
        let country_label = ofm_core::nations::nation_display_name(&country);
        let country_slug = country.to_lowercase();

        let league_month = if country == "BR" {
            1
        } else {
            default_season_month_for_region(&region_id)
        };
        let (league_start, _) = ofm_core::generator::start_date_at_game_open(
            game_start,
            league_month,
            if country == "BR" { 28 } else { 1 },
        );

        // One or two divisions depending on how many clubs the country has.
        let divisions = split_into_divisions(&team_ids, TOP_DIVISION_SIZE);
        let division_count = divisions.len();

        if ofm_core::nations::is_split_season_country(&country) {
            // Split-season format: Apertura (first half, Feb) + Clausura (second
            // half, Jul). Only the Clausura carries promotion/relegation berths
            // since it closes the year.
            let (apertura_start, _) =
                ofm_core::generator::start_date_at_game_open(game_start, 2, 1);
            let (clausura_start, _) =
                ofm_core::generator::start_date_at_game_open(game_start, 7, 1);

            for (tier, division_ids) in divisions.iter().enumerate() {
                let clausura_berths = if tier == 0 {
                    vec![continental_berth(BerthRule::PositionRange {
                        from: 1,
                        to: CONTINENTAL_QUALIFYING_POSITIONS,
                    })]
                } else {
                    Vec::new()
                };
                let make_def = |id: &str, name: &str, month: u8, berths: Vec<Berth>, p: u32| {
                    CompetitionDefinition {
                        id: id.to_string(),
                        name: name.to_string(),
                        r#type: CompetitionType::League,
                        scope: CompetitionScope::Domestic,
                        region_id: Some(region_id.clone()),
                        country_id: Some(country.clone()),
                        required_region_ids: vec![region_id.clone()],
                        priority: p,
                        format: make_format(CompetitionFormat::LeagueTable),
                        participants: ParticipantSpec {
                            explicit: Some(division_ids.clone()),
                            selector: None,
                        },
                        berths,
                        season_start_month: Some(month),
                        season_start_day: Some(1),
                        name_key: None,
                        logo: None,
                    }
                };
                let tier_suffix = format!("d{}", tier + 1);
                planned.push((
                    make_def(
                        &format!("{country_slug}-{tier_suffix}-apertura"),
                        &format!(
                            "{country_label} {} Apertura",
                            division_tier_name(tier, division_count)
                        ),
                        2,
                        Vec::new(),
                        priority,
                    ),
                    apertura_start,
                ));
                priority += 1;
                planned.push((
                    make_def(
                        &format!("{country_slug}-{tier_suffix}-clausura"),
                        &format!(
                            "{country_label} {} Clausura",
                            division_tier_name(tier, division_count)
                        ),
                        7,
                        clausura_berths,
                        priority,
                    ),
                    clausura_start,
                ));
                priority += 1;
            }
        } else {
            for (tier, division_ids) in divisions.iter().enumerate() {
                let berths = if tier == 0 {
                    vec![continental_berth(BerthRule::PositionRange {
                        from: 1,
                        to: CONTINENTAL_QUALIFYING_POSITIONS,
                    })]
                } else {
                    Vec::new()
                };
                let actual_start = if country == "BR" && tier > 0 {
                    ofm_core::generator::start_date_at_game_open(game_start, 3, 21).0
                } else {
                    league_start
                };
                planned.push((
                    CompetitionDefinition {
                        id: format!("{country_slug}-d{}", tier + 1),
                        name: division_name(&country_label, tier, division_count),
                        r#type: CompetitionType::League,
                        scope: CompetitionScope::Domestic,
                        region_id: Some(region_id.clone()),
                        country_id: Some(country.clone()),
                        required_region_ids: vec![region_id.clone()],
                        priority,
                        format: make_format(CompetitionFormat::LeagueTable),
                        participants: ParticipantSpec {
                            explicit: Some(division_ids.clone()),
                            selector: None,
                        },
                        berths,
                        season_start_month: Some(if country == "BR" && tier > 0 {
                            actual_start.month() as u8
                        } else {
                            league_month
                        }),
                        season_start_day: Some(if country == "BR" {
                            if tier == 0 {
                                28
                            } else {
                                actual_start.day() as u8
                            }
                        } else {
                            1
                        }),
                        name_key: Some(division_tier_name_key(tier, division_count).to_string()),
                        logo: None,
                    },
                    actual_start,
                ));
                priority += 1;
            }
        }

        // National cup contested by every club in the country.
        let cup_month = if ofm_core::nations::is_split_season_country(&country) {
            2
        } else {
            league_month
        };
        let (actual_cup_start, _) =
            ofm_core::generator::start_date_at_game_open(game_start, cup_month, 1);
        let cup_actual_start = actual_cup_start + Duration::days(35);
        planned.push((
            CompetitionDefinition {
                id: format!("{country_slug}-cup"),
                name: format!("{country_label} Cup"),
                r#type: CompetitionType::Cup,
                scope: CompetitionScope::Domestic,
                region_id: Some(region_id.clone()),
                country_id: Some(country.clone()),
                required_region_ids: vec![region_id.clone()],
                priority,
                format: make_format(CompetitionFormat::Knockout),
                participants: ParticipantSpec {
                    explicit: Some(team_ids.clone()),
                    selector: None,
                },
                berths: vec![continental_berth(BerthRule::CupWinner)],
                season_start_month: Some(cup_actual_start.month() as u8),
                season_start_day: Some(cup_actual_start.day() as u8),
                name_key: Some("tournaments.competitions.nationalCup".to_string()),
                logo: None,
            },
            cup_actual_start,
        ));
        priority += 1;

        if country == "BR" {
            let labels = [
                (
                    "southeast",
                    "Southeast State Series",
                    "competitionNames.brazilStateSoutheast",
                ),
                (
                    "south",
                    "South State Series",
                    "competitionNames.brazilStateSouth",
                ),
                (
                    "northeast",
                    "Northeast State Series",
                    "competitionNames.brazilStateNortheast",
                ),
                (
                    "north-central-west",
                    "North/Central-West State Series",
                    "competitionNames.brazilStateNorthCentralWest",
                ),
            ];
            let mut pools: BTreeMap<&str, Vec<String>> =
                labels.iter().map(|(id, _, _)| (*id, Vec::new())).collect();
            let mut unknown = Vec::new();
            for team_id in &team_ids {
                let city = game
                    .teams
                    .iter()
                    .find(|team| &team.id == team_id)
                    .map(|team| team.city.as_str())
                    .unwrap_or("");
                if let Some(pool) = brazil_state_region(city) {
                    pools.get_mut(pool).unwrap().push(team_id.clone());
                } else {
                    unknown.push(team_id.clone());
                }
            }
            unknown.sort();
            for team_id in unknown {
                let smallest = labels
                    .iter()
                    .map(|(id, _, _)| *id)
                    .min_by_key(|id| (pools[*id].len(), *id))
                    .unwrap();
                pools.get_mut(smallest).unwrap().push(team_id);
            }
            let state_start = ofm_core::generator::start_date_at_game_open(game_start, 1, 11).0;
            for (id, name, name_key) in labels {
                let participants = pools.remove(id).unwrap_or_default();
                if participants.len() < 2 {
                    continue;
                }
                planned.push((
                    CompetitionDefinition {
                        id: format!("br-state-{id}"),
                        name: name.to_string(),
                        r#type: CompetitionType::Cup,
                        scope: CompetitionScope::Regional,
                        region_id: Some(region_id.clone()),
                        country_id: Some(country.clone()),
                        required_region_ids: vec![region_id.clone()],
                        priority,
                        format: FormatDef {
                            kind: CompetitionFormat::GroupAndKnockout,
                            legs: Some(1),
                            group_size: Some(4),
                            qualifiers_per_group: Some(2),
                            best_third_qualifiers: None,
                        },
                        participants: ParticipantSpec {
                            explicit: Some(participants),
                            selector: None,
                        },
                        berths: Vec::new(),
                        season_start_month: Some(1),
                        season_start_day: Some(11),
                        name_key: Some(name_key.to_string()),
                        logo: None,
                    },
                    state_start,
                ));
                priority += 1;
            }
        }
    }

    let continental_team_ids = select_continental_entrants(&game.teams, 2, 16);
    if continental_team_ids.len() >= 4 {
        let mut feeder_regions: Vec<String> = game
            .teams
            .iter()
            .filter(|team| continental_team_ids.contains(&team.id))
            .map(infer_team_region_id)
            .collect();
        feeder_regions.sort();
        feeder_regions.dedup();
        // With a big enough field, the continental cup opens with a group
        // stage; smaller fields go straight to a knockout bracket.
        let format_kind = if continental_team_ids.len() >= 8 {
            CompetitionFormat::GroupAndKnockout
        } else {
            CompetitionFormat::Knockout
        };
        // Continental cup starts in October regardless of hemisphere (it draws
        // from multiple regions and is keyed to the European calendar).
        let (continental_start, _) =
            ofm_core::generator::start_date_at_game_open(game_start, 10, 1);
        planned.push((
            CompetitionDefinition {
                id: "continental-champions-cup".to_string(),
                name: "Continental Champions Cup".to_string(),
                r#type: CompetitionType::ContinentalClub,
                scope: CompetitionScope::Continental,
                name_key: Some("tournaments.competitions.continentalChampionsCup".to_string()),
                region_id: None,
                country_id: None,
                required_region_ids: feeder_regions,
                priority,
                format: make_format(format_kind),
                participants: ParticipantSpec {
                    explicit: Some(continental_team_ids),
                    selector: None,
                },
                berths: Vec::new(),
                season_start_month: Some(10),
                season_start_day: Some(1),
                logo: None,
            },
            continental_start,
        ));
    }

    planned
}

fn finalize_brazil_state_competition(competition: &mut League) {
    competition.rules.counts_in_season_flow = false;
    competition.rules.knockout_round_gap_days = 7;
}

fn build_foundation_competitions(game: &Game) -> Vec<League> {
    let game_start = game.clock.start_date;
    let season = preseason_league_year(&game.clock);
    build_foundation_competition_plan(game, game_start)
        .iter()
        .filter_map(|(def, start)| {
            let mut competition =
                ofm_core::generator::build_explicit_competition(def, season, *start)?;
            // FM-style: if this competition's season already began before the game
            // anchor date, simulate the missing matchdays so the player joins a
            // living in-progress season rather than a blank table.
            if *start <= game_start {
                ofm_core::catchup::simulate_past_fixtures(
                    &mut competition,
                    &game.players,
                    game_start,
                );
            }
            if competition.id.starts_with("br-state-") {
                finalize_brazil_state_competition(&mut competition);
            }
            Some(competition)
        })
        .collect()
}

fn rebuild_competitions_for_management_date(game: &mut Game, management_date: DateTime<Utc>) {
    let players = &game.players;
    for competition in &mut game.competitions {
        let (start, is_mid_season) = ofm_core::generator::start_date_at_game_open(
            management_date,
            competition.season_start_month,
            competition.season_start_day,
        );
        let season = start.year() as u32;
        match competition.rules.format {
            CompetitionFormat::LeagueTable => {
                ofm_core::schedule::regenerate_league_for_season(competition, season, start)
            }
            CompetitionFormat::GroupAndKnockout => {
                ofm_core::group_stage::regenerate_for_season(competition, season, start)
            }
            CompetitionFormat::Knockout => {
                ofm_core::schedule::regenerate_knockout_for_season(competition, season, start)
            }
        }
        if is_mid_season {
            ofm_core::catchup::simulate_past_fixtures(competition, players, management_date);
        }
    }

    let existing: std::collections::HashSet<String> = game
        .competitions
        .iter()
        .map(|competition| competition.id.clone())
        .collect();
    let season = preseason_league_year(&game.clock);
    let mut missing_states: Vec<(League, DateTime<Utc>)> =
        build_foundation_competition_plan(game, management_date)
            .into_iter()
            .filter(|(definition, _)| {
                definition.id.starts_with("br-state-") && !existing.contains(&definition.id)
            })
            .filter_map(|(definition, start)| {
                let mut competition =
                    ofm_core::generator::build_explicit_competition(&definition, season, start)?;
                finalize_brazil_state_competition(&mut competition);
                Some((competition, start))
            })
            .collect();
    for (competition, start) in &mut missing_states {
        if *start <= management_date {
            ofm_core::catchup::simulate_past_fixtures(competition, &game.players, management_date);
        }
    }
    game.competitions
        .extend(missing_states.into_iter().map(|(c, _)| c));
}

fn ensure_multi_competition_foundations(game: &mut Game) {
    if game.national_teams.is_empty() {
        game.national_teams = build_national_teams(game);
    }
    if game.competitions.is_empty() {
        game.competitions = build_foundation_competitions(game);
    }
    if game.active_region_ids.is_empty() {
        game.active_region_ids = game
            .competitions
            .iter()
            .filter_map(|competition| competition.region_id.clone())
            .collect();
        game.active_region_ids.sort();
        game.active_region_ids.dedup();
    }
    if game.active_competition_ids.is_empty() {
        game.active_competition_ids = game
            .competitions
            .iter()
            .map(|competition| competition.id.clone())
            .collect();
    }
    ensure_international_windows(game);
    game.sync_legacy_league();
}

/// Schedule national-team friendlies on international windows and keep club
/// fixtures off those dates, so call-ups never clash with club matches.
/// Idempotent: existing national-team fixtures (e.g. from a loaded save) are
/// left untouched, and shifting already-clear club fixtures is a no-op.
fn ensure_international_windows(game: &mut Game) {
    // A career that opens during a World Cup summer stages the tournament right
    // away: the World Cup is otherwise created only at season rollover, which a
    // fresh save beginning in a cup summer (e.g. mid-2026) never reaches, so the
    // edition would simply never happen. It fills the summer break, so no window
    // friendlies/qualifiers are scheduled when it runs.
    let now = game.clock.current_date;
    let opens_in_world_cup_summer =
        ofm_core::world_cup::is_world_cup_summer(now.year()) && (6..=8).contains(&now.month());
    if opens_in_world_cup_summer
        && ofm_core::world_cup::schedule_world_cup_if_due(game, now + Duration::days(2))
    {
        for national_team in game.national_teams.iter_mut() {
            national_team.fixtures.clear();
        }
        return;
    }

    let window_dates =
        ofm_core::national_team::international_window_dates(preseason_season_start(&game.clock));
    if window_dates.is_empty() {
        return;
    }

    let needs_fixtures = game
        .national_teams
        .iter()
        .all(|team| team.fixtures.is_empty());
    let qualifying_running = game
        .competitions
        .iter()
        .any(ofm_core::world_cup::is_world_cup_qualifying);
    if needs_fixtures && !qualifying_running {
        // A career starting the season before a World Cup opens with the
        // qualifying campaign; any other season opens with friendlies.
        if ofm_core::world_cup::season_leads_into_world_cup(preseason_season_start(&game.clock)) {
            ofm_core::world_cup::schedule_world_cup_qualifying(
                game,
                preseason_season_start(&game.clock).year() + 1,
                &window_dates,
            );
        } else {
            ofm_core::national_team::schedule_national_team_friendlies(
                &mut game.national_teams,
                &window_dates,
                &mut rand::rng(),
            );
        }
    }

    for competition in &mut game.competitions {
        ofm_core::schedule::shift_fixtures_off_reserved_dates(competition, &window_dates);
    }
    ofm_core::schedule::append_south_american_preseason_friendlies(
        &mut game.competitions,
        &window_dates,
    );
    ofm_core::schedule::append_other_preseason_friendlies(&mut game.competitions, &window_dates);
}

fn resolve_simulation_scope(
    game: &Game,
    team_id: &str,
    requested_region_ids: Option<Vec<String>>,
    requested_competition_ids: Option<Vec<String>>,
) -> Result<(Vec<String>, Vec<String>), String> {
    use std::collections::BTreeSet;

    let managed_team = game
        .teams
        .iter()
        .find(|team| team.id == team_id)
        .ok_or("be.error.teamNotFound".to_string())?;

    let mut active_region_ids: BTreeSet<String> = requested_region_ids
        .unwrap_or_default()
        .into_iter()
        .collect();
    active_region_ids.insert(infer_team_region_id(managed_team));

    let mut active_competition_ids: BTreeSet<String> = requested_competition_ids
        .unwrap_or_default()
        .into_iter()
        .filter(|competition_id| {
            game.competitions
                .iter()
                .any(|competition| competition.id == *competition_id)
        })
        .collect();

    for competition in game.competitions.iter().filter(|competition| {
        competition
            .participant_ids
            .iter()
            .any(|participant_id| participant_id == team_id)
    }) {
        active_competition_ids.insert(competition.id.clone());
    }

    if active_competition_ids.is_empty() {
        for competition in &game.competitions {
            let required_regions = competition_required_region_ids(competition);
            if required_regions.is_empty()
                || required_regions
                    .iter()
                    .all(|region_id| active_region_ids.contains(region_id))
            {
                active_competition_ids.insert(competition.id.clone());
            }
        }
    }

    for competition in game
        .competitions
        .iter()
        .filter(|competition| active_competition_ids.contains(&competition.id))
    {
        for region_id in competition_required_region_ids(competition) {
            active_region_ids.insert(region_id);
        }
    }

    let mut resolved_region_ids: Vec<String> = active_region_ids.into_iter().collect();
    resolved_region_ids.sort();

    let mut resolved_competition_ids: Vec<String> = active_competition_ids.into_iter().collect();
    resolved_competition_ids.sort_by_key(|competition_id| {
        game.competitions
            .iter()
            .find(|competition| competition.id == *competition_id)
            .map(|competition| competition.priority)
            .unwrap_or(u32::MAX)
    });

    Ok((resolved_region_ids, resolved_competition_ids))
}

fn has_existing_world_context(game: &Game, stats_state: &StatsState) -> bool {
    !game.competitions.is_empty()
        || game.league.is_some()
        || !game.news.is_empty()
        || !stats_state.player_matches.is_empty()
        || !stats_state.team_matches.is_empty()
}

fn bootstrap_existing_world_takeover(
    game: &mut Game,
    team_id: &str,
    stats_state: StatsState,
) -> Result<StatsState, String> {
    let team = game
        .teams
        .iter()
        .find(|t| t.id == team_id)
        .ok_or("be.error.teamNotFound".to_string())?;
    let team_name = team.name.clone();

    ofm_core::ai_hiring::seed_ai_managers(game);

    let takeover_date = game.clock.current_date.format("%Y-%m-%d").to_string();
    let incumbent_manager_id = game
        .teams
        .iter()
        .find(|candidate| candidate.id == team_id)
        .and_then(|candidate| candidate.manager_id.clone());

    if incumbent_manager_id.as_deref() != Some(game.manager.id.as_str()) {
        let fired = ofm_core::firing::fire_ai_manager_for_team(game, team_id, &takeover_date);
        if !fired {
            if let Some(team) = game
                .teams
                .iter_mut()
                .find(|candidate| candidate.id == team_id)
            {
                team.manager_id = None;
            }
        }
        ofm_core::job_offers::hire_manager(game, team_id, &takeover_date)?;
    }

    let staff_msg = ofm_core::messages::staff_advice_message(&team_name, team_id, &takeover_date);
    game.messages.push(staff_msg);
    ofm_core::player_events::generate_takeover_contract_review_message(game);
    ofm_core::season_context::refresh_game_context(game);

    Ok(stats_state)
}

pub(crate) fn create_new_save(
    save_manager: &mut SaveManager,
    game: &Game,
    stats_state: &StatsState,
    save_name: &str,
) -> Result<String, String> {
    save_manager.create_save_with_stats(game, stats_state, save_name)
}

fn bootstrap_season_start(game: &mut Game, team_id: &str) -> Result<StatsState, String> {
    let team = game
        .teams
        .iter()
        .find(|t| t.id == team_id)
        .ok_or("be.error.teamNotFound".to_string())?;
    let team_name = team.name.clone();

    game.manager.hire(team_id.to_string());
    if let Some(t) = game.teams.iter_mut().find(|t| t.id == team_id) {
        t.manager_id = Some(game.manager.id.clone());
    }
    game.manager_id = game.manager.id.clone();
    ofm_core::ai_hiring::seed_ai_managers(game);

    let season_start = preseason_season_start(&game.clock);
    let team_ids: Vec<String> = game.teams.iter().map(|t| t.id.clone()).collect();
    let league_name = default_league_name();
    let mut league = ofm_core::schedule::generate_league(
        &league_name,
        preseason_league_year(&game.clock),
        &team_ids,
        season_start,
    );
    let friendlies = ofm_core::schedule::generate_preseason_friendlies(&team_ids, season_start, 4);
    ofm_core::schedule::append_fixtures(&mut league, friendlies);
    game.league = Some(league);
    ofm_core::season_context::refresh_game_context(game);

    let date_str = game.clock.current_date.to_rfc3339();
    let welcome_msg = ofm_core::messages::welcome_message(&team_name, team_id, &date_str);
    game.messages.push(welcome_msg);

    let season_msg = ofm_core::messages::season_schedule_message(
        &league_name,
        &season_start.format(&long_date_format()).to_string(),
        &date_str,
    );
    game.messages.push(season_msg);

    let team_names: Vec<String> = game.teams.iter().map(|team| team.name.clone()).collect();
    game.news.push(ofm_core::news::season_preview_article(
        &team_names,
        &date_str,
    ));

    let staff_msg = ofm_core::messages::staff_advice_message(&team_name, team_id, &date_str);
    game.messages.push(staff_msg);

    ofm_core::player_events::generate_takeover_contract_review_message(game);

    Ok(StatsState::default())
}

fn competitive_fixture_count_for_team(game: &Game, team_id: &str) -> usize {
    game.league
        .as_ref()
        .map(|league| {
            league
                .fixtures
                .iter()
                .filter(|fixture| {
                    fixture.counts_for_league_standings()
                        && (fixture.home_team_id == team_id || fixture.away_team_id == team_id)
                })
                .count()
        })
        .unwrap_or_default()
}

fn completed_competitive_fixture_count_for_team(game: &Game, team_id: &str) -> usize {
    game.league
        .as_ref()
        .map(|league| {
            league
                .fixtures
                .iter()
                .filter(|fixture| {
                    fixture.counts_for_league_standings()
                        && fixture.status == domain::league::FixtureStatus::Completed
                        && (fixture.home_team_id == team_id || fixture.away_team_id == team_id)
                })
                .count()
        })
        .unwrap_or_default()
}

fn bootstrap_midseason_takeover(game: &mut Game, team_id: &str) -> Result<StatsState, String> {
    let team = game
        .teams
        .iter()
        .find(|t| t.id == team_id)
        .ok_or("be.error.teamNotFound".to_string())?;
    let team_name = team.name.clone();

    ofm_core::ai_hiring::seed_ai_managers(game);

    let season_start = preseason_season_start(&game.clock);
    let league_name = default_league_name();
    let team_ids: Vec<String> = game.teams.iter().map(|t| t.id.clone()).collect();
    game.league = Some(ofm_core::schedule::generate_league(
        &league_name,
        preseason_league_year(&game.clock),
        &team_ids,
        season_start,
    ));
    game.clock.current_date = season_start;
    ofm_core::season_context::refresh_game_context(game);

    let total_fixtures = competitive_fixture_count_for_team(game, team_id);
    let target_completed = (total_fixtures / 2).max(1);
    let mut stats_state = StatsState::default();
    let mut safeguard_days = 0usize;
    while completed_competitive_fixture_count_for_team(game, team_id) < target_completed {
        let mut captures = Vec::new();
        ofm_core::turn::process_day_with_capture(game, &mut |capture| captures.push(capture));
        for capture in captures {
            stats_state.append(capture);
        }
        safeguard_days += 1;
        if safeguard_days > 240 {
            break;
        }
    }

    let takeover_date = game.clock.current_date.format("%Y-%m-%d").to_string();
    let _ = ofm_core::firing::fire_ai_manager_for_team(game, team_id, &takeover_date);
    ofm_core::job_offers::hire_manager(game, team_id, &takeover_date)?;

    let staff_msg = ofm_core::messages::staff_advice_message(&team_name, team_id, &takeover_date);
    game.messages.push(staff_msg);
    ofm_core::player_events::generate_takeover_contract_review_message(game);
    ofm_core::season_context::refresh_game_context(game);

    Ok(stats_state)
}

pub(crate) fn bootstrap_team_selection(
    game: &mut Game,
    team_id: &str,
    start_phase: StartPhase,
    stats_state: StatsState,
) -> Result<StatsState, String> {
    let stats_state = if has_existing_world_context(game, &stats_state) {
        bootstrap_existing_world_takeover(game, team_id, stats_state)?
    } else {
        match start_phase {
            StartPhase::SeasonStart => bootstrap_season_start(game, team_id)?,
            StartPhase::MidSeason => bootstrap_midseason_takeover(game, team_id)?,
        }
    };

    ofm_core::transfers::seed_opening_ai_loan_market(game);
    Ok(stats_state)
}

/// Step 1: Create manager + generate world. No team assigned yet.
/// Returns the Game object so the frontend can show team selection.
/// One validation problem in a competition-definition file, shaped for the UI.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompetitionDefinitionIssue {
    code: String,
    competition_id: String,
    params: std::collections::HashMap<String, String>,
}

fn parse_competition_definitions(
    source: &str,
) -> Result<ofm_core::generator::CompetitionDefinitionFile, String> {
    // Accept either JSON or YAML so definitions can be hand-authored in either.
    ofm_core::generator::parse_definition_str(source)
        .map_err(|_| "be.error.competitionDef.parseFailed".to_string())
}

fn validate_against_world(
    file: &ofm_core::generator::CompetitionDefinitionFile,
    world: &ofm_core::generator::WorldData,
) -> Vec<CompetitionDefinitionIssue> {
    let ctx = ofm_core::generator::WorldValidationContext::from_world(world);
    ofm_core::generator::validate_definitions(file, &ctx)
        .into_iter()
        .map(|error| CompetitionDefinitionIssue {
            code: error.code,
            competition_id: error.competition_id,
            params: error.params.into_iter().collect(),
        })
        .collect()
}

/// Validate a standalone competition-definition file against a world. Returns
/// the full list of problems (empty = valid) so the new-game UI can show them
/// before the player commits.
#[tauri::command]
pub fn validate_competition_definitions(
    world_source: Option<String>,
    definitions_json: String,
) -> Result<Vec<CompetitionDefinitionIssue>, String> {
    let file = parse_competition_definitions(&definitions_json)?;
    let world = load_world_data(world_source.as_deref())?;
    Ok(validate_against_world(&file, &world))
}

/// One problem found while loading a world package, shaped for the UI.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageIssue {
    pub code: String,
    pub file: String,
    pub params: std::collections::HashMap<String, String>,
}

/// Validate a modular world-package directory. Returns the full list of problems
/// (empty = valid) so the new-game UI can show them before the player commits.
#[tauri::command]
pub fn validate_world_package(path: String) -> Result<Vec<PackageIssue>, String> {
    let (_package, errors) = ofm_core::generator::load_world_package(std::path::Path::new(&path));
    Ok(errors
        .into_iter()
        .map(|error| PackageIssue {
            code: error.code,
            file: error.file,
            params: error.params.into_iter().collect(),
        })
        .collect())
}

/// A world package summarised for the import card: a display name (falling back
/// to the folder name when the package declares none), club/player counts, and
/// any validation problems (empty = ready to start).
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldPackageInspection {
    name: String,
    team_count: usize,
    player_count: usize,
    issues: Vec<PackageIssue>,
}

fn package_folder_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "World Package".to_string())
}

/// Validate and summarise a world package for the new-game picker. On any
/// validation problem the issues are returned (with a folder-name fallback) and
/// the world isn't built; otherwise the built world's name and counts come back.
#[tauri::command]
pub fn inspect_world_package(path: String) -> Result<WorldPackageInspection, String> {
    let (package, errors) = ofm_core::generator::load_world_package(std::path::Path::new(&path));
    let issues: Vec<PackageIssue> = errors
        .into_iter()
        .map(|error| PackageIssue {
            code: error.code,
            file: error.file,
            params: error.params.into_iter().collect(),
        })
        .collect();

    let fallback_name = package_folder_name(&path);
    if !issues.is_empty() {
        return Ok(WorldPackageInspection {
            name: fallback_name,
            team_count: 0,
            player_count: 0,
            issues,
        });
    }

    let world = ofm_core::generator::build_world_from_package(&package)?;
    let name = if world.name.trim().is_empty() {
        fallback_name
    } else {
        world.name.clone()
    };
    Ok(WorldPackageInspection {
        name,
        team_count: world.teams.len(),
        player_count: world.players.len(),
        issues: Vec::new(),
    })
}

/// world_source: "random" (default) or a file path to a JSON world database.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn start_new_game(
    state: State<'_, Arc<StateManager>>,
    app_handle: tauri::AppHandle,
    first_name: String,
    last_name: String,
    dob: String,
    nationality: String,
    startup_options: Option<RawStartupOptions>,
    world_source: Option<String>,
    competition_definitions_json: Option<String>,
    package_ids: Option<Vec<String>>,
) -> Result<Game, String> {
    // Validate inputs
    let first_name = first_name.trim().to_string();
    let last_name = last_name.trim().to_string();
    if first_name.is_empty() || last_name.is_empty() {
        return Err("be.error.createManager.nameRequired".to_string());
    }
    if first_name.len() > 30 || last_name.len() > 30 {
        return Err("be.error.createManager.nameMaxLength".to_string());
    }
    let nationality = nationality.trim().to_string();
    if nationality.is_empty() {
        return Err("be.error.createManager.nationalityRequired".to_string());
    }

    // Validate DOB against the selected career start date.
    let birth_date = chrono::NaiveDate::parse_from_str(&dob, "%Y-%m-%d")
        .map_err(|_| "be.error.createManager.invalidDobFormat".to_string())?;

    let startup_options = normalize_startup_options(startup_options)?;
    let (mut world, package_lockfile) = if let Some(ids) = package_ids.as_deref().filter(|ids| !ids.is_empty()) {
        let packages_dir = app_handle
            .path()
            .app_data_dir()
            .map_err(|e| e.to_string())?
            .join("packages");
        load_world_data_from_package_ids(&packages_dir, ids)?
    } else {
        (load_world_data(world_source.as_deref())?, vec![])
    };

    // Layer a user-picked standalone definition file onto the world. It is
    // validated strictly; the UI has already shown any details via
    // validate_competition_definitions.
    if let Some(json) = &competition_definitions_json {
        let file = parse_competition_definitions(json)?;
        if !validate_against_world(&file, &world).is_empty() {
            return Err("be.error.competitionDef.invalidStandalone".to_string());
        }
        world.competition_definitions = Some(file);
    }

    let clock = game_clock_for_world(&startup_options, &world.metadata)?;
    let is_non_random = package_ids.as_deref().is_some_and(|ids| !ids.is_empty())
        || matches!(world_source.as_deref(), Some(source) if source != "random");
    if is_non_random {
        ofm_core::generator::normalize_imported_world_for_career_start(&mut world);
    }
    let reference_date = clock.current_date.date_naive();
    let age = age_on_date(birth_date, reference_date);
    if age < 30 {
        return Err("be.error.createManager.minAge".to_string());
    }
    if age > 99 {
        return Err("be.error.createManager.invalidDob".to_string());
    }

    let manager = Manager::new(
        "mgr_user".to_string(),
        first_name,
        last_name,
        dob,
        nationality,
    );
    info!(
        "[cmd] start_new_game: {} {} (nationality={}, start_year={}, start_phase={}, history_depth_years={}, world_source={:?})",
        manager.first_name,
        manager.last_name,
        manager.nationality,
        startup_options.start_year,
        startup_options.start_phase.as_str(),
        startup_options.history_depth_years,
        world_source
    );

    let (mut new_game, stats_state) =
        build_game_from_world_data(clock, manager, &startup_options, world);

    new_game.package_lockfile = package_lockfile;

    info!(
        "[cmd] start_new_game: world generated with {} teams, {} players, {} staff",
        new_game.teams.len(),
        new_game.players.len(),
        new_game.staff.len()
    );
    state.set_game(new_game.clone());
    state.set_stats_state(stats_state);
    Ok(new_game)
}

/// Step 2: User picks a team. Assigns manager, generates welcome message, saves to DB.
#[tauri::command]
pub async fn select_team(
    state: State<'_, Arc<StateManager>>,
    sm_state: State<'_, Arc<SaveManagerState>>,
    team_id: String,
    active_region_ids: Option<Vec<String>>,
    active_competition_ids: Option<Vec<String>>,
) -> Result<Game, String> {
    info!("[cmd] select_team: team_id={}", team_id);
    let mut game = state
        .get_game(|g: &Game| g.clone())
        .ok_or("be.error.noActiveGameSession".to_string())?;
    let current_stats_state = state
        .get_stats_state(|stats| stats.clone())
        .unwrap_or_default();
    ensure_multi_competition_foundations(&mut game);

    // Hemisphere fix: when the player picks SeasonStart for a southern-
    // hemisphere (or other non-August-start) club, align the game clock to
    // that club's actual season-start date and rebuild competitions from that
    // anchor so the player arrives at the beginning of their season, not July.
    if start_phase_for_game(&game) == StartPhase::SeasonStart {
        if let Some(actual_start) = team_season_anchor(&game, &team_id) {
            if actual_start < game.clock.current_date {
                game.clock.current_date = actual_start;
                game.clock.start_date = actual_start;
                rebuild_competitions_for_management_date(&mut game, actual_start);
                game.national_teams.clear();
                ensure_multi_competition_foundations(&mut game);
            }
        }
    }

    let (resolved_region_ids, resolved_competition_ids) =
        resolve_simulation_scope(&game, &team_id, active_region_ids, active_competition_ids)?;
    game.active_region_ids = resolved_region_ids;
    game.active_competition_ids = resolved_competition_ids;

    let start_phase = start_phase_for_game(&game);
    let stats_state =
        bootstrap_team_selection(&mut game, &team_id, start_phase, current_stats_state)?;

    // Upgrade generic (legacy-bucket) positions to granular on new-game creation
    // so the frontend sees the same granular positions immediately, rather than
    // only after the first save/reload cycle (where load_game applies this same
    // upgrade).
    ofm_core::player_identity::upgrade_game_player_identities(&mut game);

    // Save to new per-save DB
    let manager_name = format!("{} {}", game.manager.first_name, game.manager.last_name);
    let save_name = default_save_name(&manager_name);

    let mut sm = map_save_manager_lock_error(sm_state.0.lock())?;
    let save_id = create_new_save(&mut sm, &game, &stats_state, &save_name)?;
    state.set_save_id(save_id);

    state.set_game(game.clone());
    state.set_stats_state(stats_state);
    Ok(game)
}

#[tauri::command]
pub async fn get_saves(
    sm_state: State<'_, Arc<SaveManagerState>>,
) -> Result<Vec<SaveEntry>, String> {
    log::debug!("[cmd] get_saves");
    let mut sm = map_save_manager_lock_error(sm_state.0.lock())?;
    sm.load_saves()
}

#[tauri::command]
pub async fn delete_save(
    sm_state: State<'_, Arc<SaveManagerState>>,
    save_id: String,
) -> Result<bool, String> {
    info!("[cmd] delete_save: save_id={}", save_id);
    let mut sm = map_save_manager_lock_error(sm_state.0.lock())?;
    sm.delete_save(&save_id)
}

#[tauri::command]
pub async fn load_game(
    state: State<'_, Arc<StateManager>>,
    sm_state: State<'_, Arc<SaveManagerState>>,
    save_id: String,
) -> Result<String, String> {
    info!("[cmd] load_game: save_id={}", save_id);
    let mut sm = map_save_manager_lock_error(sm_state.0.lock())?;
    let mut game = sm.load_game(&save_id)?;
    let stats_state = sm.load_stats_state(&save_id)?;
    ofm_core::ai_hiring::seed_ai_managers(&mut game);
    ofm_core::season_context::refresh_game_context(&mut game);

    let mgr_name = format!("{} {}", game.manager.first_name, game.manager.last_name);

    state.set_save_id(save_id);
    state.set_game(game);
    state.set_stats_state(stats_state);
    Ok(mgr_name)
}

#[tauri::command]
pub async fn get_active_game(state: State<'_, Arc<StateManager>>) -> Result<Game, String> {
    log::debug!("[cmd] get_active_game");
    state
        .get_game(|g: &Game| g.clone())
        .ok_or("be.error.noActiveGameSession".to_string())
}

#[tauri::command]
pub async fn get_active_save_id(
    state: State<'_, Arc<StateManager>>,
) -> Result<Option<String>, String> {
    log::debug!("[cmd] get_active_save_id");
    Ok(state.get_save_id())
}

#[tauri::command]
pub async fn save_game(
    state: State<'_, Arc<StateManager>>,
    sm_state: State<'_, Arc<SaveManagerState>>,
) -> Result<(), String> {
    info!("[cmd] save_game");
    let game = state
        .get_game(|g: &Game| g.clone())
        .ok_or("be.error.noActiveGameSession".to_string())?;

    let save_id = state
        .get_save_id()
        .ok_or("be.error.noActiveSaveSession".to_string())?;

    let mut sm = map_save_manager_lock_error(sm_state.0.lock())?;
    let stats_state = require_active_stats_state(&state)?;
    sm.save_game_with_stats(&game, &stats_state, &save_id)
}

/// Save the current game and clear the active session so the player returns to the main menu.
#[tauri::command]
pub async fn exit_to_menu(
    state: State<'_, Arc<StateManager>>,
    sm_state: State<'_, Arc<SaveManagerState>>,
) -> Result<(), String> {
    info!("[cmd] exit_to_menu");
    let game = state
        .get_game(|g: &Game| g.clone())
        .ok_or("be.error.noActiveGameSession")?;

    // Auto-save
    if let Some(save_id) = state.get_save_id() {
        let mut sm = map_save_manager_lock_error(sm_state.0.lock())?;
        let stats_state = require_active_stats_state(&state)?;
        sm.save_game_with_stats(&game, &stats_state, &save_id)?;
    }

    // Clear the in-memory game state
    state.clear_game();
    state.clear_save_id();

    Ok(())
}

/// Bootstrap a game for MCP auto-start.
/// Creates a manager, loads world, selects team, and saves.
/// Returns the save ID.
#[cfg(feature = "mcp")]
pub fn bootstrap_game_for_mcp(
    state_manager: &StateManager,
    save_manager_state: &crate::SaveManagerState,
    world_path: &str,
    team_id: Option<&str>,
    manager_first_name: &str,
    manager_last_name: &str,
    manager_nationality: &str,
) -> Result<String, String> {
    // Step 1: Load world data
    let mut world = load_world_data_from_path(world_path)?;

    // Normalize imported world for career start (same as start_new_game does for non-random imports)
    ofm_core::generator::normalize_imported_world_for_career_start(&mut world);

    // Step 2: Find the existing user manager in the world data.
    // HistoricalSnapshot exports include the user manager (id "mgr_user") already
    // assigned to their team. Reusing it preserves the team assignment, career
    // history, and all manager state — no takeover/hiring logic needed.
    // If not found (e.g. RosterBaseline world), fall back to creating a fresh one.
    let manager = if let Some(idx) = world.managers.iter().position(|m| m.id == "mgr_user") {
        let mut existing = world.managers.remove(idx);
        info!(
            "[mcp-bootstrap] Reusing existing manager {} {} (team_id={:?})",
            existing.first_name, existing.last_name, existing.team_id
        );
        // Apply CLI overrides for name/nationality if provided
        if manager_first_name != "Agent" {
            existing.first_name = manager_first_name.to_string();
        }
        if manager_last_name != "Manager" {
            existing.last_name = manager_last_name.to_string();
        }
        if manager_nationality != "England" {
            existing.nationality = manager_nationality.to_string();
        }
        existing
    } else {
        // No existing user manager — create a fresh one (DOB set to make age ~45)
        let startup_options = normalize_startup_options(None)?;
        let reference_date = game_clock_for_world(&startup_options, &world.metadata)?
            .current_date
            .date_naive();
        let dob = reference_date - chrono::Duration::days(45 * 365);
        let dob_str = dob.format("%Y-%m-%d").to_string();

        let fresh = Manager::new(
            "mgr_user".to_string(),
            manager_first_name.to_string(),
            manager_last_name.to_string(),
            dob_str,
            manager_nationality.to_string(),
        );
        info!(
            "[mcp-bootstrap] Created fresh manager {} {}",
            fresh.first_name, fresh.last_name
        );
        fresh
    };

    // Step 3: Build game from world data
    let startup_options = normalize_startup_options(None)?;
    let clock = game_clock_for_world(&startup_options, &world.metadata)?;
    let (mut game, current_stats_state) =
        build_game_from_world_data(clock, manager, &startup_options, world);

    info!(
        "[mcp-bootstrap] Built game: {} teams, {} players, manager.team_id={:?}",
        game.teams.len(),
        game.players.len(),
        game.manager.team_id,
    );

    // Step 4: If the manager already has a team assigned (reused from world data),
    // we don't need the takeover logic. Just refresh context and proceed.
    // Otherwise, run the normal team selection bootstrap.
    let stats_state = if game.manager.team_id.is_some() {
        ofm_core::ai_hiring::seed_ai_managers(&mut game);
        ofm_core::season_context::refresh_game_context(&mut game);
        ofm_core::transfers::seed_opening_ai_loan_market(&mut game);
        current_stats_state
    } else {
        // Manager has no team — need an explicit team_id to assign one
        let tid = team_id.ok_or(
            "--mcp-auto-start requires a team_id when the world's manager has no team. Format: \"world.json,team_id\""
                .to_string(),
        )?;
        let start_phase = start_phase_for_game(&game);
        bootstrap_team_selection(&mut game, tid, start_phase, current_stats_state)?
    };

    info!(
        "[mcp-bootstrap] Manager assigned to team_id={:?}",
        game.manager.team_id
    );

    // Step 5: Create initial save
    let manager_name = format!("{} {}", game.manager.first_name, game.manager.last_name);
    let save_name = default_save_name(&manager_name);
    let mut sm = map_save_manager_lock_error(save_manager_state.0.lock())?;
    let save_id = create_new_save(&mut sm, &game, &stats_state, &save_name)?;

    // Step 6: Set state
    state_manager.set_game(game);
    state_manager.set_stats_state(stats_state);
    state_manager.set_save_id(save_id.clone());

    info!("[mcp-bootstrap] Game saved with ID: {}", save_id);

    Ok(save_id)
}

#[cfg(test)]
mod tests {
    use super::{
        age_on_date, apply_generated_past_history, bootstrap_team_selection, brazil_state_region,
        build_foundation_competitions, build_game_from_world_data, create_new_save,
        current_date_for_phase, ensure_international_windows, game_clock_for_world,
        load_world_data_from_path, map_save_manager_lock_error, normalize_startup_options,
        package_folder_name, parse_competition_definitions, preseason_league_year,
        preseason_season_start, rebuild_competitions_for_management_date,
        require_active_stats_state, resolve_simulation_scope, select_continental_entrants,
        split_into_divisions, start_date_for_year, RawStartupOptions, StartPhase, StartupOptions,
        DEFAULT_GENERATED_HISTORY_DEPTH_YEARS, MAX_GENERATED_HISTORY_DEPTH_YEARS,
    };
    use chrono::{TimeZone, Utc};
    use db::save_manager::SaveManager;
    use domain::{
        league::{
            CompetitionFormat, CompetitionScope, CompetitionType, FixtureCompetition, League,
        },
        manager::Manager,
        news::{NewsArticle, NewsCategory},
        stats::{PlayerMatchStatsRecord, TeamMatchStatsRecord},
        world_history::{HistoricalSeasonAwardsRecord, WorldHistoryArchive},
    };
    use ofm_core::{
        clock::GameClock,
        game::Game,
        generator::{WorldData, WorldDataKind, WorldDataMetadata},
        season_context::refresh_game_context,
        state::StateManager,
    };
    use std::sync::Mutex;

    fn manager_for(team_id: &str) -> Manager {
        let mut manager = Manager::new(
            "mgr".to_string(),
            "A".to_string(),
            "B".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire(team_id.to_string());
        manager
    }

    #[test]
    fn world_cup_summer_career_stages_and_surfaces_the_tournament() {
        use ofm_core::world_cup::is_world_cup_competition;
        // A career opening in the 2026 World Cup summer.
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 7, 1, 12, 0, 0).unwrap());
        let mut game = Game::new(
            clock,
            manager_for("team-1"),
            vec![nation_team("team-1", "ES", 500)],
            vec![],
            vec![],
            vec![],
        );
        // A non-empty active scope so staging registers the tournament as active.
        game.active_competition_ids = vec!["dummy".to_string()];

        ensure_international_windows(&mut game);

        let world_cup = game
            .competitions
            .iter()
            .find(|competition| is_world_cup_competition(competition))
            .expect("a World Cup summer career stages the tournament");
        assert!(
            game.active_competition_ids.contains(&world_cup.id),
            "the World Cup is surfaced in the active scope"
        );
        assert!(
            game.news
                .iter()
                .any(|article| article.id.starts_with("world_cup_kickoff_")),
            "a kickoff news article is published"
        );
    }

    #[test]
    fn non_world_cup_year_career_stages_no_tournament() {
        use ofm_core::world_cup::is_world_cup_competition;
        let clock = GameClock::new(Utc.with_ymd_and_hms(2027, 7, 1, 12, 0, 0).unwrap());
        let mut game = Game::new(
            clock,
            manager_for("team-1"),
            vec![nation_team("team-1", "ES", 500)],
            vec![],
            vec![],
            vec![],
        );

        ensure_international_windows(&mut game);

        assert!(
            !game.competitions.iter().any(is_world_cup_competition),
            "no World Cup is staged outside a cup summer"
        );
    }

    #[test]
    fn package_folder_name_falls_back_to_the_directory_name() {
        assert_eq!(package_folder_name("/mods/My World"), "My World");
        assert_eq!(package_folder_name("turkish-league"), "turkish-league");
        // No usable component → a sensible default rather than an empty name.
        assert_eq!(package_folder_name(""), "World Package");
    }

    fn nation_team(id: &str, nation: &str, reputation: u32) -> domain::team::Team {
        let mut team = domain::team::Team::new(
            id.to_string(),
            id.to_string(),
            id.to_string(),
            nation.to_string(),
            "City".to_string(),
            "Stadium".to_string(),
            10_000,
        );
        team.football_nation = nation.to_string();
        team.reputation = reputation;
        team
    }

    /// Characterization test: locks the STRUCTURE of the generated foundation
    /// world (kinds, scopes, regions, countries, priorities, participant and
    /// fixture counts, formats) so the Phase E "unify built-ins through the
    /// resolver" refactor can prove it preserves behavior (modulo ids).
    #[test]
    fn foundation_competitions_structure_is_stable() {
        // A 30-club nation (→ two divisions: 20 + 10), a 6-club nation (one
        // division), and a 1-club nation (skipped). All in one region, so the
        // continental field stays under four entrants and no continental cup
        // is created — keeping the structure fully deterministic.
        let mut teams = Vec::new();
        for index in 0..30 {
            teams.push(nation_team(
                &format!("esp-{index:02}"),
                "ESP",
                1000 - index as u32,
            ));
        }
        for index in 0..6 {
            teams.push(nation_team(
                &format!("fra-{index}"),
                "FRA",
                500 - index as u32,
            ));
        }
        teams.push(nation_team("and-0", "AND", 100));

        let clock = GameClock::new(start_date_for_year(2032).unwrap());
        let manager = domain::manager::Manager::new(
            "mgr".to_string(),
            "A".to_string(),
            "B".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let game = Game::new(clock, manager, teams, vec![], vec![], vec![]);

        let competitions = build_foundation_competitions(&game);

        type CompetitionSummary = (
            CompetitionType,
            CompetitionScope,
            Option<String>,
            Option<String>,
            usize,
            u32,
            CompetitionFormat,
        );

        let summary: Vec<CompetitionSummary> = competitions
            .iter()
            .map(|competition| {
                (
                    competition.kind.clone(),
                    competition.scope.clone(),
                    competition.region_id.clone(),
                    competition.country_id.clone(),
                    competition.participant_ids.len(),
                    competition.priority,
                    competition.rules.format.clone(),
                )
            })
            .collect();

        let europe = || Some("europe".to_string());
        assert_eq!(
            summary,
            vec![
                (
                    CompetitionType::League,
                    CompetitionScope::Domestic,
                    europe(),
                    Some("ESP".to_string()),
                    20,
                    0,
                    CompetitionFormat::LeagueTable
                ),
                (
                    CompetitionType::League,
                    CompetitionScope::Domestic,
                    europe(),
                    Some("ESP".to_string()),
                    10,
                    1,
                    CompetitionFormat::LeagueTable
                ),
                (
                    CompetitionType::Cup,
                    CompetitionScope::Domestic,
                    europe(),
                    Some("ESP".to_string()),
                    30,
                    2,
                    CompetitionFormat::Knockout
                ),
                (
                    CompetitionType::League,
                    CompetitionScope::Domestic,
                    europe(),
                    Some("FRA".to_string()),
                    6,
                    3,
                    CompetitionFormat::LeagueTable
                ),
                (
                    CompetitionType::Cup,
                    CompetitionScope::Domestic,
                    europe(),
                    Some("FRA".to_string()),
                    6,
                    4,
                    CompetitionFormat::Knockout
                ),
            ],
        );

        // League tables carry a full double round robin and a standings row per
        // club; the refactor must preserve both.
        let top_division = &competitions[0];
        assert_eq!(top_division.standings.len(), 20);
        assert_eq!(top_division.fixtures.len(), 20 * 19);
        assert_eq!(competitions[3].fixtures.len(), 6 * 5);

        // No continental cup for a single-region field.
        assert!(!competitions
            .iter()
            .any(|competition| competition.kind == CompetitionType::ContinentalClub));

        // Default continental berths: first division awards positions 1–4, the
        // cup awards its winner, the second division awards nothing.
        use domain::league::BerthRule;
        let top_division = &competitions[0];
        assert_eq!(top_division.berths.len(), 1);
        assert_eq!(top_division.berths[0].target, "continental-champions-cup");
        assert!(matches!(
            top_division.berths[0].rule,
            BerthRule::PositionRange { from: 1, to: 4 }
        ));
        assert!(
            competitions[1].berths.is_empty(),
            "second division awards no berth"
        );
        let cup = &competitions[2];
        assert!(matches!(
            cup.berths.first().map(|berth| &berth.rule),
            Some(BerthRule::CupWinner)
        ));
    }

    fn default_player_attributes() -> domain::player::PlayerAttributes {
        domain::player::PlayerAttributes {
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
            aggression: 50,
            teamwork: 60,
            leadership: 50,
            handling: 20,
            reflexes: 20,
            aerial: 60,
        }
    }

    fn make_bootstrap_test_game() -> Game {
        let clock = GameClock::new(start_date_for_year(2032).unwrap());
        let manager = domain::manager::Manager::new(
            "mgr-user".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let teams = vec![
            domain::team::Team::new(
                "team1".to_string(),
                "Alpha FC".to_string(),
                "AFC".to_string(),
                "England".to_string(),
                "London".to_string(),
                "Alpha Park".to_string(),
                20_000,
            ),
            domain::team::Team::new(
                "team2".to_string(),
                "Beta FC".to_string(),
                "BFC".to_string(),
                "England".to_string(),
                "Manchester".to_string(),
                "Beta Park".to_string(),
                22_000,
            ),
        ];
        let staff = vec![
            {
                let mut staff = domain::staff::Staff::new(
                    "staff1".to_string(),
                    "Pat".to_string(),
                    "Coach".to_string(),
                    "1978-01-01".to_string(),
                    domain::staff::StaffRole::AssistantManager,
                    domain::staff::StaffAttributes {
                        coaching: 70,
                        judging_ability: 65,
                        judging_potential: 64,
                        physiotherapy: 40,
                    },
                );
                staff.nationality = "England".to_string();
                staff.team_id = Some("team1".to_string());
                staff
            },
            {
                let mut staff = domain::staff::Staff::new(
                    "staff2".to_string(),
                    "Lee".to_string(),
                    "Coach".to_string(),
                    "1979-01-01".to_string(),
                    domain::staff::StaffRole::AssistantManager,
                    domain::staff::StaffAttributes {
                        coaching: 72,
                        judging_ability: 66,
                        judging_potential: 65,
                        physiotherapy: 39,
                    },
                );
                staff.nationality = "England".to_string();
                staff.team_id = Some("team2".to_string());
                staff
            },
        ];

        let mut players = Vec::new();
        for team_id in ["team1", "team2"] {
            for index in 0..11 {
                let position = if index == 0 {
                    domain::player::Position::Goalkeeper
                } else if index < 5 {
                    domain::player::Position::Defender
                } else if index < 8 {
                    domain::player::Position::Midfielder
                } else {
                    domain::player::Position::Forward
                };
                let mut player = domain::player::Player::new(
                    format!("{}-player-{}", team_id, index),
                    format!("{} P{}", team_id, index),
                    format!("{} Player {}", team_id, index),
                    format!("199{}-01-01", index),
                    "England".to_string(),
                    position,
                    default_player_attributes(),
                );
                player.team_id = Some(team_id.to_string());
                player.ovr = 62 + index as u8;
                player.potential = 68 + index as u8;
                players.push(player);
            }
        }

        Game::new(clock, manager, teams, players, staff, vec![])
    }

    #[test]
    fn select_continental_entrants_takes_top_clubs_per_region_by_reputation() {
        let make = |id: &str, nation: &str, reputation: u32| {
            let mut team = domain::team::Team::new(
                id.to_string(),
                id.to_string(),
                id.to_string(),
                "Country".to_string(),
                "City".to_string(),
                "Stadium".to_string(),
                10_000,
            );
            team.football_nation = nation.to_string();
            team.reputation = reputation;
            team
        };
        let teams = vec![
            make("eng-a", "GB", 900),
            make("eng-b", "GB", 800),
            make("eng-c", "GB", 700), // third in Europe -> excluded by per_region
            make("bra-a", "BR", 850),
            make("bra-b", "BR", 600),
        ];

        let entrants = select_continental_entrants(&teams, 2, 16);

        // Top two per region, ordered strongest-first across regions.
        assert_eq!(
            entrants,
            vec![
                "eng-a".to_string(),
                "bra-a".to_string(),
                "eng-b".to_string(),
                "bra-b".to_string(),
            ]
        );
    }

    #[test]
    fn parse_competition_definitions_accepts_yaml_and_json() {
        let yaml = "\
formatVersion: 1
competitions:
  - id: tr-1
    name: Super Lig
    type: League
    scope: Domestic
    format:
      kind: LeagueTable
    participants:
      selector:
        kind: allInCountry
        country: TR
";
        let parsed = parse_competition_definitions(yaml).expect("YAML should parse");
        assert_eq!(parsed.competitions.len(), 1);
        assert_eq!(parsed.competitions[0].id, "tr-1");

        let json = r#"{"formatVersion":1,"competitions":[{"id":"tr-1","name":"Super Lig","type":"League","scope":"Domestic","format":{"kind":"LeagueTable"},"participants":{"selector":{"kind":"allInCountry","country":"TR"}}}]}"#;
        let parsed_json = parse_competition_definitions(json).expect("JSON should parse");
        assert_eq!(parsed_json.competitions[0].id, "tr-1");

        assert!(parse_competition_definitions("not: [valid").is_err());
    }

    fn temp_pkg_dir(tag: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("ofm-pkg-cmd-{tag}-{nanos}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    #[ignore = "perf harness; run: cargo test -p openfootmanager perf_baseline -- --ignored --nocapture"]
    fn perf_baseline() {
        use std::time::Instant;

        let t = Instant::now();
        let world = ofm_core::generator::generate_world_data(None);
        let gen = t.elapsed();
        let teams = world.teams.len();
        let players = world.players.len();

        let manager = domain::manager::Manager::new(
            "mgr-user".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let startup_options = StartupOptions {
            start_year: 2026,
            start_phase: StartPhase::SeasonStart,
            history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
        };
        let clock = game_clock_for_world(&startup_options, &world.metadata).unwrap();

        let t = Instant::now();
        let (mut game, _stats) =
            build_game_from_world_data(clock, manager, &startup_options, world);
        let build = t.elapsed();

        let competitions = game.competitions.len();
        let active = game.active_competition_ids.len();

        const DAYS: u32 = 30;
        let t = Instant::now();
        for _ in 0..DAYS {
            ofm_core::turn::process_day(&mut game);
        }
        let days = t.elapsed();

        eprintln!(
            "PERF teams={teams} players={players} competitions={competitions} active_competition_ids={active}"
        );
        eprintln!("PERF world-gen         = {gen:?}");
        eprintln!("PERF build-game        = {build:?}  (foundations + history)");
        eprintln!(
            "PERF {DAYS}x process_day   = {days:?}  ({:?}/day)",
            days / DAYS
        );
    }

    #[test]
    fn loads_a_world_from_a_package_directory() {
        let dir = temp_pkg_dir("load");
        std::fs::write(
            dir.join("confed.yaml"),
            "schema: confederation\nid: galaxy\nname: Galaxy\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("country.yaml"),
            "schema: country\nid: ZZ\nname: Zedland\nconfederation: galaxy\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("teams.yaml"),
            "schema: team\nitems:\n  - { id: zed-fc, name: Zed FC, city: Zedtown, country: ZZ, colors: { primary: \"#000\", secondary: \"#fff\" } }\n  - { id: zed-utd, name: Zed United, city: Zedford, country: ZZ, colors: { primary: \"#111\", secondary: \"#fff\" } }\n",
        )
        .unwrap();

        let world =
            super::load_world_data(Some(dir.to_string_lossy().as_ref())).expect("package loads");
        assert!(world.teams.iter().any(|t| t.id == "zed-fc"));
        assert!(world.teams.iter().any(|t| t.id == "zed-utd"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn validate_world_package_reports_problems_and_passes_clean_packages() {
        let team = "schema: team\nid: {id}\nname: {name}\ncity: X\ncountry: ES\ncolors: { primary: \"#000\", secondary: \"#fff\" }\n";

        let valid = temp_pkg_dir("valid");
        std::fs::write(
            valid.join("team.yaml"),
            team.replace("{id}", "zed-fc").replace("{name}", "Zed FC"),
        )
        .unwrap();
        let clean = super::validate_world_package(valid.to_string_lossy().to_string()).unwrap();
        assert!(clean.is_empty(), "a clean package should have no issues");

        let broken = temp_pkg_dir("broken");
        std::fs::write(
            broken.join("a.yaml"),
            team.replace("{id}", "dup").replace("{name}", "A"),
        )
        .unwrap();
        std::fs::write(
            broken.join("b.yaml"),
            team.replace("{id}", "dup").replace("{name}", "B"),
        )
        .unwrap();
        let issues = super::validate_world_package(broken.to_string_lossy().to_string()).unwrap();
        assert!(!issues.is_empty(), "a duplicate id should be reported");

        std::fs::remove_dir_all(&valid).ok();
        std::fs::remove_dir_all(&broken).ok();
    }

    #[test]
    fn split_into_divisions_chunks_a_major_into_two_tiers() {
        let clubs: Vec<String> = (0..40).map(|i| format!("club-{i:02}")).collect();

        let divisions = split_into_divisions(&clubs, 20);

        assert_eq!(divisions.len(), 2);
        assert_eq!(divisions[0].len(), 20);
        assert_eq!(divisions[1].len(), 20);
        // Strongest tier first; the second tier starts where the first ends.
        assert_eq!(divisions[0][0], "club-00");
        assert_eq!(divisions[1][0], "club-20");
    }

    #[test]
    fn split_into_divisions_keeps_a_single_league_at_division_size() {
        let clubs: Vec<String> = (0..20).map(|i| format!("club-{i:02}")).collect();

        let divisions = split_into_divisions(&clubs, 20);

        assert_eq!(divisions.len(), 1);
        assert_eq!(divisions[0].len(), 20);
    }

    #[test]
    fn split_into_divisions_keeps_a_single_tier_for_small_countries() {
        let clubs: Vec<String> = (0..7).map(|i| format!("club-{i}")).collect();

        let divisions = split_into_divisions(&clubs, 20);

        assert_eq!(divisions.len(), 1);
        assert_eq!(divisions[0].len(), 7);
    }

    #[test]
    fn split_into_divisions_folds_a_tiny_remainder_up() {
        // 25 clubs → 20 + 5; the 5-club tail folds up rather than forming a
        // tiny second division.
        let clubs: Vec<String> = (0..25).map(|i| format!("club-{i:02}")).collect();

        let divisions = split_into_divisions(&clubs, 20);

        assert_eq!(divisions.len(), 1);
        assert_eq!(divisions[0].len(), 25);
    }

    #[test]
    fn brazil_foundations_use_the_2026_calendar_and_regional_state_series() {
        let cities = [
            "São Paulo",
            "Rio",
            "Belo Horizonte",
            "Porto Alegre",
            "Salvador",
            "Recife",
            "Curitiba",
            "Fortaleza",
            "Goiânia",
            "Santos",
            "Campinas",
            "Belém",
            "Manaus",
            "Vitória",
            "Natal",
            "Florianópolis",
            "Cuiabá",
            "Maceió",
            "Bragantino",
            "Juiz de Fora",
        ];
        let mut teams: Vec<_> = (0..40)
            .map(|index| {
                let mut team = nation_team(&format!("br-{index}"), "BR", 1000 - index);
                team.city = cities[index as usize % cities.len()].to_string();
                team
            })
            .collect();
        let clock = GameClock::new(Utc.with_ymd_and_hms(2025, 12, 15, 0, 0, 0).unwrap());
        let mut game = Game::new(
            clock,
            manager_for("br-0"),
            std::mem::take(&mut teams),
            vec![],
            vec![],
            vec![],
        );
        game.competitions = build_foundation_competitions(&game);
        ofm_core::schedule::append_south_american_preseason_friendlies(&mut game.competitions, &[]);

        let serie_a = game
            .competitions
            .iter()
            .find(|competition| competition.id == "br-d1")
            .unwrap();
        let serie_b = game
            .competitions
            .iter()
            .find(|competition| competition.id == "br-d2")
            .unwrap();
        assert_eq!(serie_a.season_start_day, 28);
        assert_eq!(serie_a.season_start_month, 1);
        assert_eq!(serie_b.season_start_day, 21);
        assert_eq!(serie_b.season_start_month, 3);
        assert!(serie_a
            .fixtures
            .iter()
            .any(|fixture| fixture.competition == FixtureCompetition::League
                && fixture.date == "2026-01-28"));
        assert!(serie_b
            .fixtures
            .iter()
            .any(|fixture| fixture.competition == FixtureCompetition::League
                && fixture.date == "2026-03-21"));
        let friendly_dates: Vec<&str> = serie_a
            .fixtures
            .iter()
            .filter(|fixture| fixture.competition == FixtureCompetition::Friendly)
            .map(|fixture| fixture.date.as_str())
            .collect();
        assert_eq!(
            friendly_dates
                .iter()
                .copied()
                .collect::<std::collections::HashSet<_>>(),
            ["2025-12-21", "2025-12-28", "2026-01-04"]
                .into_iter()
                .collect()
        );

        let states: Vec<_> = game
            .competitions
            .iter()
            .filter(|competition| competition.id.starts_with("br-state-"))
            .collect();
        assert_eq!(states.len(), 4);
        assert!(states
            .iter()
            .all(|competition| !competition.rules.counts_in_season_flow
                && competition.rules.group_stage_legs == 1
                && competition.name_key.is_some()));
        for team in &game.teams {
            assert_eq!(
                states
                    .iter()
                    .filter(|competition| competition.participant_ids.contains(&team.id))
                    .count(),
                1
            );
        }
    }

    #[test]
    fn management_date_rebuild_preserves_authored_competition_identity() {
        let teams = vec![
            nation_team("br-a", "BR", 500),
            nation_team("br-b", "BR", 400),
        ];
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap());
        let mut game = Game::new(clock, manager_for("br-a"), teams, vec![], vec![], vec![]);
        let mut authored = ofm_core::schedule::generate_league(
            "Authored Brazil Championship",
            2026,
            &["br-a".to_string(), "br-b".to_string()],
            Utc.with_ymd_and_hms(2026, 1, 28, 0, 0, 0).unwrap(),
        );
        authored.id = "authored-brasileirao".to_string();
        authored.country_id = Some("BR".to_string());
        authored.region_id = Some("south-america".to_string());
        authored.season_start_month = 1;
        authored.season_start_day = 28;
        game.competitions = vec![authored];

        let anchor = Utc.with_ymd_and_hms(2025, 12, 15, 0, 0, 0).unwrap();
        game.clock.start_date = anchor;
        game.clock.current_date = anchor;
        rebuild_competitions_for_management_date(&mut game, anchor);

        let competition = game
            .competitions
            .iter()
            .find(|competition| competition.id == "authored-brasileirao")
            .unwrap();
        assert_eq!(competition.season, 2026);
        assert!(competition
            .fixtures
            .iter()
            .any(|fixture| fixture.date == "2026-01-28"));
    }

    #[test]
    fn select_continental_entrants_caps_the_field() {
        let teams: Vec<domain::team::Team> = (0..10)
            .map(|index| {
                let mut team = domain::team::Team::new(
                    format!("eng-{index}"),
                    format!("Club {index}"),
                    format!("C{index}"),
                    "Country".to_string(),
                    "City".to_string(),
                    "Stadium".to_string(),
                    10_000,
                );
                team.football_nation = "GB".to_string();
                team.reputation = 1000 - index as u32;
                team
            })
            .collect();

        let entrants = select_continental_entrants(&teams, 8, 4);

        assert_eq!(entrants.len(), 4);
        assert_eq!(entrants[0], "eng-0", "strongest club is seeded first");
    }

    #[test]
    fn resolve_simulation_scope_auto_enables_required_regions_and_team_competitions() {
        let mut game = make_bootstrap_test_game();
        game.teams[0].football_nation = "BR".to_string();
        game.teams[1].football_nation = "GB".to_string();

        let mut domestic = League::new(
            "domestic-1".to_string(),
            "Brazil League".to_string(),
            2032,
            &["team1".to_string()],
        );
        domestic.region_id = Some("south-america".to_string());
        domestic.required_region_ids = vec!["south-america".to_string()];
        domestic.priority = 0;

        let mut continental = League::new(
            "continental-1".to_string(),
            "Continental Champions Cup".to_string(),
            2032,
            &["team1".to_string(), "team2".to_string()],
        );
        continental.scope = CompetitionScope::Continental;
        continental.required_region_ids = vec!["south-america".to_string(), "europe".to_string()];
        continental.priority = 1;

        game.competitions = vec![domestic.clone(), continental.clone()];

        let (active_regions, active_competitions) = resolve_simulation_scope(
            &game,
            "team1",
            Some(vec!["south-america".to_string()]),
            Some(vec![continental.id.clone()]),
        )
        .unwrap();

        assert_eq!(
            active_regions,
            vec!["europe".to_string(), "south-america".to_string()]
        );
        assert_eq!(
            active_competitions,
            vec![domestic.id.clone(), continental.id.clone()]
        );
    }

    #[test]
    fn resolve_simulation_scope_defaults_to_team_region_when_no_scope_is_provided() {
        let mut game = make_bootstrap_test_game();
        game.teams[0].football_nation = "BR".to_string();

        let mut domestic = League::new(
            "domestic-1".to_string(),
            "Brazil League".to_string(),
            2032,
            &["team1".to_string()],
        );
        domestic.region_id = Some("south-america".to_string());
        domestic.required_region_ids = vec!["south-america".to_string()];
        domestic.priority = 0;
        game.competitions = vec![domestic.clone()];

        let (active_regions, active_competitions) =
            resolve_simulation_scope(&game, "team1", None, None).unwrap();

        assert_eq!(active_regions, vec!["south-america".to_string()]);
        assert_eq!(active_competitions, vec![domestic.id.clone()]);
    }

    #[test]
    fn load_world_data_from_path_returns_read_file_key_when_missing() {
        let result =
            load_world_data_from_path("file:Z:/definitely-missing/openfootmanager-world.json");

        assert_eq!(result.unwrap_err(), "be.error.worldReadFileFailed");
    }

    fn sample_stats_state() -> domain::stats::StatsState {
        domain::stats::StatsState {
            player_matches: vec![PlayerMatchStatsRecord {
                fixture_id: "fixture-1".to_string(),
                season: 2031,
                matchday: 12,
                date: "2031-11-20".to_string(),
                competition: FixtureCompetition::League,
                player_id: "team1-player-0".to_string(),
                team_id: "team1".to_string(),
                opponent_team_id: "team2".to_string(),
                home_team_id: "team1".to_string(),
                away_team_id: "team2".to_string(),
                home_goals: 2,
                away_goals: 1,
                minutes_played: 90,
                goals: 1,
                assists: 0,
                shots: 4,
                shots_on_target: 2,
                passes_completed: 30,
                passes_attempted: 35,
                tackles_won: 1,
                interceptions: 1,
                fouls_committed: 0,
                yellow_cards: 0,
                red_cards: 0,
                rating: 7.5,
            }],
            team_matches: vec![TeamMatchStatsRecord {
                fixture_id: "fixture-1".to_string(),
                season: 2031,
                matchday: 12,
                date: "2031-11-20".to_string(),
                competition: FixtureCompetition::League,
                team_id: "team1".to_string(),
                opponent_team_id: "team2".to_string(),
                home_team_id: "team1".to_string(),
                away_team_id: "team2".to_string(),
                goals_for: 2,
                goals_against: 1,
                possession_pct: 53,
                shots: 11,
                shots_on_target: 6,
                passes_completed: 310,
                passes_attempted: 360,
                tackles_won: 15,
                interceptions: 9,
                fouls_committed: 7,
                yellow_cards: 1,
                red_cards: 0,
            }],
        }
    }

    fn make_imported_baseline_world_without_staff() -> WorldData {
        let teams = vec![
            domain::team::Team::new(
                "team1".to_string(),
                "Alpha FC".to_string(),
                "AFC".to_string(),
                "England".to_string(),
                "London".to_string(),
                "Alpha Park".to_string(),
                20_000,
            ),
            domain::team::Team::new(
                "team2".to_string(),
                "Beta FC".to_string(),
                "BFC".to_string(),
                "England".to_string(),
                "Manchester".to_string(),
                "Beta Park".to_string(),
                22_000,
            ),
        ];

        let mut players = Vec::new();
        for team in &teams {
            let make_player =
                |id: String, position: domain::player::Position, date_of_birth: &str| {
                    let mut player = domain::player::Player::new(
                        id.clone(),
                        format!("{id} Match"),
                        format!("{id} Full"),
                        date_of_birth.to_string(),
                        "England".to_string(),
                        position,
                        default_player_attributes(),
                    );
                    player.team_id = Some(team.id.clone());
                    player.ovr = 62;
                    player.potential = 68;
                    player
                };

            players.push(make_player(
                format!("{}-gk", team.id),
                domain::player::Position::Goalkeeper,
                "1998-01-01",
            ));
            players.push(make_player(
                format!("{}-def-youth", team.id),
                domain::player::Position::Defender,
                "2008-01-01",
            ));
            players.push(make_player(
                format!("{}-mid-youth", team.id),
                domain::player::Position::Midfielder,
                "2007-01-01",
            ));
            players.push(make_player(
                format!("{}-fwd-youth", team.id),
                domain::player::Position::Forward,
                "2006-01-01",
            ));
            for index in 0..8 {
                players.push(make_player(
                    format!("{}-senior-{index}", team.id),
                    domain::player::Position::Defender,
                    "1997-01-01",
                ));
            }
        }

        WorldData {
            name: "Imported Baseline".to_string(),
            description: "No staff import".to_string(),
            teams,
            players,
            staff: vec![],
            managers: vec![],
            league: None,
            news: vec![],
            stats: domain::stats::StatsState::default(),
            world_history: WorldHistoryArchive::default(),
            metadata: WorldDataMetadata::default(),
            ..Default::default()
        }
    }

    fn make_historical_snapshot_world() -> WorldData {
        let base_game = make_bootstrap_test_game();
        let mut league = League::new(
            "league-1".to_string(),
            "Premier Division".to_string(),
            2031,
            &["team1".to_string(), "team2".to_string()],
        );
        league.standings = vec![
            domain::league::StandingEntry {
                team_id: "team1".to_string(),
                played: 12,
                won: 7,
                drawn: 3,
                lost: 2,
                goals_for: 18,
                goals_against: 10,
                points: 24,
            },
            domain::league::StandingEntry {
                team_id: "team2".to_string(),
                played: 12,
                won: 5,
                drawn: 2,
                lost: 5,
                goals_for: 14,
                goals_against: 15,
                points: 17,
            },
        ];

        let mut incumbent = domain::manager::Manager::new(
            "mgr-incumbent".to_string(),
            "Jordan".to_string(),
            "Incumbent".to_string(),
            "1974-01-01".to_string(),
            "England".to_string(),
        );
        incumbent.hire("team1".to_string());

        let mut teams = base_game.teams.clone();
        teams[0].manager_id = Some(incumbent.id.clone());

        let mut archive = WorldHistoryArchive::default();
        archive.record_season_awards(HistoricalSeasonAwardsRecord {
            season: 2030,
            golden_boot: None,
            assist_king: None,
            player_of_year: None,
            clean_sheet_king: None,
            most_appearances: None,
            young_player: None,
            manager_of_season: None,
        });

        WorldData {
            name: "Historical Snapshot".to_string(),
            description: "Season already underway".to_string(),
            teams,
            players: base_game.players,
            staff: base_game.staff,
            managers: vec![incumbent],
            competitions: Vec::new(),
            competition_definitions: None,
            national_teams: Vec::new(),
            regions: Vec::new(),
            default_active_regions: Vec::new(),
            default_active_competitions: Vec::new(),
            league: Some(league),
            news: vec![NewsArticle::new(
                "news-1".to_string(),
                "Season underway".to_string(),
                "The campaign has begun.".to_string(),
                "World Feed".to_string(),
                "2031-11-20".to_string(),
                NewsCategory::StandingsUpdate,
            )],
            stats: sample_stats_state(),
            world_history: archive,
            metadata: WorldDataMetadata {
                format_version: 2,
                world_id: "historical-snapshot".to_string(),
                kind: WorldDataKind::HistoricalSnapshot,
                base_year: Some(2031),
                snapshot_date: Some("2031-11-20T00:00:00Z".to_string()),
            },
            extra_translations: std::collections::HashMap::new(),
            build_notices: Vec::new(),
        }
    }

    #[test]
    fn map_save_manager_lock_error_returns_backend_key_for_poisoned_mutex() {
        let mutex = Mutex::new(());
        let _ = std::panic::catch_unwind(|| {
            let _guard = mutex.lock().unwrap();
            panic!("poison save manager mutex for test");
        });

        let result = map_save_manager_lock_error(mutex.lock());

        assert_eq!(result.unwrap_err(), "be.error.saveManagerUnavailable");
    }

    #[test]
    fn normalize_startup_options_defaults_to_current_year_and_season_start() {
        let options = normalize_startup_options(None).unwrap();

        assert!(options.start_year >= 2020);
        assert_eq!(options.start_phase, StartPhase::SeasonStart);
        assert_eq!(
            options.history_depth_years,
            DEFAULT_GENERATED_HISTORY_DEPTH_YEARS
        );
    }

    #[test]
    fn normalize_startup_options_rejects_years_before_2020() {
        let result = normalize_startup_options(Some(RawStartupOptions {
            start_year: Some(2019),
            start_phase: Some("seasonStart".to_string()),
            history_depth_years: None,
        }));

        assert_eq!(result.unwrap_err(), "be.error.createManager.startYearMin");
    }

    #[test]
    fn normalize_startup_options_rejects_unknown_start_phase() {
        let result = normalize_startup_options(Some(RawStartupOptions {
            start_year: Some(2026),
            start_phase: Some("playoffs".to_string()),
            history_depth_years: None,
        }));

        assert_eq!(
            result.unwrap_err(),
            "be.error.createManager.invalidStartPhase"
        );
    }

    #[test]
    fn normalize_startup_options_rejects_history_depths_above_maximum() {
        let result = normalize_startup_options(Some(RawStartupOptions {
            start_year: Some(2026),
            start_phase: Some("seasonStart".to_string()),
            history_depth_years: Some(MAX_GENERATED_HISTORY_DEPTH_YEARS + 1),
        }));

        assert_eq!(
            result.unwrap_err(),
            "be.error.createManager.historyDepthMax"
        );
    }

    #[test]
    fn normalize_startup_options_accepts_custom_history_depth() {
        let options = normalize_startup_options(Some(RawStartupOptions {
            start_year: Some(2026),
            start_phase: Some("seasonStart".to_string()),
            history_depth_years: Some(24),
        }))
        .unwrap();

        assert_eq!(options.history_depth_years, 24);
    }

    #[test]
    fn start_date_for_year_uses_selected_july_first() {
        let start_date = start_date_for_year(2032).unwrap();

        assert_eq!(start_date.to_rfc3339(), "2032-07-01T00:00:00+00:00");
    }

    #[test]
    fn start_date_for_year_rejects_out_of_range_years() {
        let result = start_date_for_year(i32::MAX);

        assert_eq!(
            result.unwrap_err(),
            "be.error.createManager.invalidStartYear"
        );
    }

    #[test]
    fn current_date_for_midseason_phase_is_after_start_date() {
        let current_date = current_date_for_phase(2032, StartPhase::MidSeason).unwrap();

        assert_eq!(current_date.to_rfc3339(), "2032-10-29T00:00:00+00:00");
    }

    #[test]
    fn age_on_date_uses_selected_start_year() {
        let birth_date = chrono::NaiveDate::from_ymd_opt(2008, 1, 1).unwrap();
        let reference_date = current_date_for_phase(2038, StartPhase::SeasonStart)
            .unwrap()
            .date_naive();

        assert_eq!(age_on_date(birth_date, reference_date), 30);
    }

    #[test]
    fn age_on_date_changes_between_season_start_and_midseason() {
        let birth_date = chrono::NaiveDate::from_ymd_opt(2008, 8, 1).unwrap();
        let season_start = current_date_for_phase(2038, StartPhase::SeasonStart)
            .unwrap()
            .date_naive();
        let midseason = current_date_for_phase(2038, StartPhase::MidSeason)
            .unwrap()
            .date_naive();

        assert_eq!(age_on_date(birth_date, season_start), 29);
        assert_eq!(age_on_date(birth_date, midseason), 30);
    }

    #[test]
    fn age_on_date_uses_world_snapshot_date_over_startup_phase() {
        let startup_options = StartupOptions {
            start_year: 2032,
            start_phase: StartPhase::MidSeason,
            history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
        };
        let world = make_historical_snapshot_world();
        let reference_date = game_clock_for_world(&startup_options, &world.metadata)
            .unwrap()
            .current_date
            .date_naive();
        let birth_date = chrono::NaiveDate::from_ymd_opt(2001, 12, 15).unwrap();

        assert_eq!(reference_date.to_string(), "2031-11-20");
        assert_eq!(age_on_date(birth_date, reference_date), 29);
    }

    #[test]
    fn preseason_league_setup_uses_selected_start_year_for_context() {
        let clock = GameClock::new(start_date_for_year(2032).unwrap());
        let manager = domain::manager::Manager::new(
            "mgr1".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let teams = vec![
            domain::team::Team::new(
                "team1".to_string(),
                "Alpha FC".to_string(),
                "AFC".to_string(),
                "England".to_string(),
                "London".to_string(),
                "Alpha Park".to_string(),
                20_000,
            ),
            domain::team::Team::new(
                "team2".to_string(),
                "Beta FC".to_string(),
                "BFC".to_string(),
                "England".to_string(),
                "Manchester".to_string(),
                "Beta Park".to_string(),
                22_000,
            ),
        ];
        let mut game = Game::new(clock, manager, teams, vec![], vec![], vec![]);

        let season_start = preseason_season_start(&game.clock);
        let team_ids = game
            .teams
            .iter()
            .map(|team| team.id.clone())
            .collect::<Vec<_>>();
        game.league = Some(ofm_core::schedule::generate_league(
            "Premier Division",
            preseason_league_year(&game.clock),
            &team_ids,
            season_start,
        ));
        refresh_game_context(&mut game);

        assert_eq!(
            game.clock.start_date.to_rfc3339(),
            "2032-07-01T00:00:00+00:00"
        );
        assert_eq!(game.league.as_ref().map(|league| league.season), Some(2032));
        assert_eq!(
            game.season_context.season_start.as_deref(),
            Some("2032-07-31")
        );
        assert_eq!(game.season_context.days_until_season_start, Some(30));
    }

    #[test]
    fn apply_generated_past_history_populates_default_twelve_prior_seasons() {
        let clock = GameClock::new(start_date_for_year(2032).unwrap());
        let manager = domain::manager::Manager::new(
            "mgr-user".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let teams = vec![
            domain::team::Team::new(
                "team1".to_string(),
                "Alpha FC".to_string(),
                "AFC".to_string(),
                "England".to_string(),
                "London".to_string(),
                "Alpha Park".to_string(),
                20_000,
            ),
            domain::team::Team::new(
                "team2".to_string(),
                "Beta FC".to_string(),
                "BFC".to_string(),
                "England".to_string(),
                "Manchester".to_string(),
                "Beta Park".to_string(),
                22_000,
            ),
        ];
        let staff = vec![
            {
                let mut staff = domain::staff::Staff::new(
                    "staff1".to_string(),
                    "Pat".to_string(),
                    "Coach".to_string(),
                    "1978-01-01".to_string(),
                    domain::staff::StaffRole::AssistantManager,
                    domain::staff::StaffAttributes {
                        coaching: 70,
                        judging_ability: 65,
                        judging_potential: 64,
                        physiotherapy: 40,
                    },
                );
                staff.nationality = "England".to_string();
                staff.team_id = Some("team1".to_string());
                staff
            },
            {
                let mut staff = domain::staff::Staff::new(
                    "staff2".to_string(),
                    "Lee".to_string(),
                    "Coach".to_string(),
                    "1979-01-01".to_string(),
                    domain::staff::StaffRole::AssistantManager,
                    domain::staff::StaffAttributes {
                        coaching: 72,
                        judging_ability: 66,
                        judging_potential: 65,
                        physiotherapy: 39,
                    },
                );
                staff.nationality = "England".to_string();
                staff.team_id = Some("team2".to_string());
                staff
            },
        ];
        let players = vec![
            {
                let mut player = domain::player::Player::new(
                    "player1".to_string(),
                    "A. Keeper".to_string(),
                    "Alex Keeper".to_string(),
                    "1994-01-01".to_string(),
                    "England".to_string(),
                    domain::player::Position::Goalkeeper,
                    domain::player::PlayerAttributes {
                        pace: 48,
                        stamina: 62,
                        strength: 64,
                        agility: 66,
                        passing: 50,
                        shooting: 20,
                        tackling: 18,
                        dribbling: 32,
                        defending: 24,
                        positioning: 68,
                        vision: 48,
                        decisions: 63,
                        composure: 61,
                        aggression: 38,
                        teamwork: 64,
                        leadership: 58,
                        handling: 76,
                        reflexes: 77,
                        aerial: 72,
                    },
                );
                player.team_id = Some("team1".to_string());
                player.ovr = 68;
                player.potential = 73;
                player
            },
            {
                let mut player = domain::player::Player::new(
                    "player2".to_string(),
                    "A. Striker".to_string(),
                    "Alex Striker".to_string(),
                    "1996-01-01".to_string(),
                    "England".to_string(),
                    domain::player::Position::Striker,
                    domain::player::PlayerAttributes {
                        pace: 72,
                        stamina: 68,
                        strength: 70,
                        agility: 71,
                        passing: 60,
                        shooting: 79,
                        tackling: 34,
                        dribbling: 73,
                        defending: 28,
                        positioning: 74,
                        vision: 62,
                        decisions: 68,
                        composure: 69,
                        aggression: 52,
                        teamwork: 64,
                        leadership: 47,
                        handling: 18,
                        reflexes: 18,
                        aerial: 61,
                    },
                );
                player.team_id = Some("team1".to_string());
                player.ovr = 74;
                player.potential = 80;
                player
            },
            {
                let mut player = domain::player::Player::new(
                    "player3".to_string(),
                    "B. Keeper".to_string(),
                    "Ben Keeper".to_string(),
                    "1993-01-01".to_string(),
                    "England".to_string(),
                    domain::player::Position::Goalkeeper,
                    domain::player::PlayerAttributes {
                        pace: 47,
                        stamina: 61,
                        strength: 63,
                        agility: 65,
                        passing: 49,
                        shooting: 19,
                        tackling: 18,
                        dribbling: 30,
                        defending: 23,
                        positioning: 67,
                        vision: 47,
                        decisions: 62,
                        composure: 60,
                        aggression: 39,
                        teamwork: 63,
                        leadership: 57,
                        handling: 75,
                        reflexes: 76,
                        aerial: 71,
                    },
                );
                player.team_id = Some("team2".to_string());
                player.ovr = 67;
                player.potential = 72;
                player
            },
            {
                let mut player = domain::player::Player::new(
                    "player4".to_string(),
                    "B. Striker".to_string(),
                    "Ben Striker".to_string(),
                    "1995-01-01".to_string(),
                    "England".to_string(),
                    domain::player::Position::Striker,
                    domain::player::PlayerAttributes {
                        pace: 71,
                        stamina: 67,
                        strength: 69,
                        agility: 70,
                        passing: 59,
                        shooting: 78,
                        tackling: 33,
                        dribbling: 72,
                        defending: 27,
                        positioning: 73,
                        vision: 61,
                        decisions: 67,
                        composure: 68,
                        aggression: 51,
                        teamwork: 63,
                        leadership: 46,
                        handling: 18,
                        reflexes: 18,
                        aerial: 60,
                    },
                );
                player.team_id = Some("team2".to_string());
                player.ovr = 73;
                player.potential = 79;
                player
            },
        ];
        let mut game = Game::new(clock, manager, teams, players, staff, vec![]);

        apply_generated_past_history(
            &mut game,
            &StartupOptions {
                start_year: 2032,
                start_phase: StartPhase::SeasonStart,
                history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
            },
        );

        assert!(game.teams.iter().all(|team| team.history.len() == 12));
        assert_eq!(game.world_history.season_awards.len(), 12);
        assert!(game.players.iter().any(|player| player.career.len() == 12));
        assert!(game
            .managers
            .iter()
            .any(|manager| !manager.career_history.is_empty()));
    }

    #[test]
    fn historical_snapshot_startup_preserves_league_news_history_and_stats() {
        let manager = domain::manager::Manager::new(
            "mgr-user".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let startup_options = StartupOptions {
            start_year: 2032,
            start_phase: StartPhase::MidSeason,
            history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
        };
        let world = make_historical_snapshot_world();
        let clock = game_clock_for_world(&startup_options, &world.metadata).unwrap();

        let (game, stats_state) =
            build_game_from_world_data(clock, manager, &startup_options, world);

        assert_eq!(
            game.clock.start_date.to_rfc3339(),
            "2031-07-01T00:00:00+00:00"
        );
        assert_eq!(
            game.clock.current_date.to_rfc3339(),
            "2031-11-20T00:00:00+00:00"
        );
        assert_eq!(game.league.as_ref().map(|league| league.season), Some(2031));
        assert_eq!(game.news.len(), 1);
        assert_eq!(game.world_history.season_awards.len(), 1);
        assert_eq!(stats_state.team_matches.len(), 1);
        assert_eq!(stats_state.player_matches.len(), 1);
        assert!(game
            .managers
            .iter()
            .any(|manager| manager.id == "mgr-incumbent"));
    }

    #[test]
    fn imported_roster_baseline_bootstrap_backfills_staff_market_and_opening_youth() {
        let manager = domain::manager::Manager::new(
            "mgr-user".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let startup_options = StartupOptions {
            start_year: 2032,
            start_phase: StartPhase::SeasonStart,
            history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
        };
        let mut world = make_imported_baseline_world_without_staff();
        ofm_core::generator::normalize_imported_world_for_career_start(&mut world);
        let clock = game_clock_for_world(&startup_options, &world.metadata).unwrap();

        let (game, stats_state) =
            build_game_from_world_data(clock, manager, &startup_options, world);

        assert!(stats_state.team_matches.is_empty());
        assert_eq!(
            game.staff
                .iter()
                .filter(|staff_member| staff_member.team_id.is_none())
                .count(),
            12
        );
        for team_id in ["team1", "team2"] {
            for role in [
                domain::staff::StaffRole::AssistantManager,
                domain::staff::StaffRole::Coach,
                domain::staff::StaffRole::Scout,
                domain::staff::StaffRole::Physio,
            ] {
                let count = game
                    .staff
                    .iter()
                    .filter(|staff_member| {
                        staff_member.team_id.as_deref() == Some(team_id)
                            && staff_member.role == role
                    })
                    .count();
                assert_eq!(count, 1);
            }
            let youth_count = game
                .players
                .iter()
                .filter(|player| {
                    player.team_id.as_deref() == Some(team_id)
                        && player.squad_role == domain::player::SquadRole::Youth
                })
                .count();
            assert_eq!(youth_count, 3);
        }
        assert_eq!(
            game.available_staff_market_last_activity_date.as_deref(),
            Some("2032-07-01")
        );
    }

    #[test]
    fn imported_roster_baseline_bootstrap_allows_ai_manager_seeding_without_imported_staff() {
        let manager = domain::manager::Manager::new(
            "mgr-user".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let startup_options = StartupOptions {
            start_year: 2032,
            start_phase: StartPhase::SeasonStart,
            history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
        };
        let mut world = make_imported_baseline_world_without_staff();
        ofm_core::generator::normalize_imported_world_for_career_start(&mut world);
        let clock = game_clock_for_world(&startup_options, &world.metadata).unwrap();
        let (mut game, stats_state) =
            build_game_from_world_data(clock, manager, &startup_options, world);

        bootstrap_team_selection(&mut game, "team1", StartPhase::SeasonStart, stats_state).unwrap();

        assert_eq!(
            game.teams
                .iter()
                .find(|team| team.id == "team1")
                .and_then(|team| team.manager_id.as_deref()),
            Some("mgr-user")
        );
        assert!(game
            .teams
            .iter()
            .filter(|team| team.id != "team1")
            .all(|team| team.manager_id.is_some()));
    }

    #[test]
    fn bootstrap_team_selection_seeds_ai_loan_market() {
        let mut game = make_bootstrap_test_game();
        game.teams
            .iter_mut()
            .find(|team| team.id == "team2")
            .unwrap()
            .starting_xi_ids = (0..11)
            .map(|index| format!("team2-player-{index}"))
            .collect();

        for (id, date_of_birth) in [
            ("team2-loan-1", "2007-01-01"),
            ("team2-loan-2", "2006-01-01"),
            ("team2-loan-3", "2005-01-01"),
        ] {
            let mut player = domain::player::Player::new(
                id.to_string(),
                id.to_string(),
                id.to_string(),
                date_of_birth.to_string(),
                "England".to_string(),
                domain::player::Position::Midfielder,
                default_player_attributes(),
            );
            player.team_id = Some("team2".to_string());
            player.contract_end = Some("2035-06-30".to_string());
            game.players.push(player);
        }

        bootstrap_team_selection(
            &mut game,
            "team1",
            StartPhase::SeasonStart,
            domain::stats::StatsState::default(),
        )
        .unwrap();

        assert_eq!(
            game.players
                .iter()
                .filter(|player| {
                    player.team_id.as_deref() == Some("team2") && player.loan_listed
                })
                .count(),
            2
        );
        assert!(game
            .players
            .iter()
            .filter(|player| player.team_id.as_deref() == Some("team1"))
            .all(|player| !player.loan_listed));
    }

    #[test]
    fn imported_historical_snapshot_preserves_state_while_backfilling_staff() {
        let manager = domain::manager::Manager::new(
            "mgr-user".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let startup_options = StartupOptions {
            start_year: 2032,
            start_phase: StartPhase::MidSeason,
            history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
        };
        let mut world = make_historical_snapshot_world();
        world.staff.clear();
        let original_news_len = world.news.len();
        let original_season = world.league.as_ref().map(|league| league.season);
        let original_awards = world.world_history.season_awards.len();
        ofm_core::generator::normalize_imported_world_for_career_start(&mut world);
        let clock = game_clock_for_world(&startup_options, &world.metadata).unwrap();

        let (game, stats_state) =
            build_game_from_world_data(clock, manager, &startup_options, world);

        assert_eq!(
            game.league.as_ref().map(|league| league.season),
            original_season
        );
        assert_eq!(game.news.len(), original_news_len);
        assert_eq!(game.world_history.season_awards.len(), original_awards);
        assert_eq!(stats_state.team_matches.len(), 1);
        assert_eq!(
            game.staff
                .iter()
                .filter(|staff_member| staff_member.team_id.is_none())
                .count(),
            12
        );
        for team_id in ["team1", "team2"] {
            let has_assistant = game.staff.iter().any(|staff_member| {
                staff_member.team_id.as_deref() == Some(team_id)
                    && staff_member.role == domain::staff::StaffRole::AssistantManager
            });
            assert!(has_assistant);
        }
    }

    #[test]
    fn embedded_competition_definitions_replace_the_auto_built_competitions() {
        use ofm_core::generator::{
            CompetitionDefinition, CompetitionDefinitionFile, FormatDef, ParticipantSpec,
        };

        let manager = domain::manager::Manager::new(
            "mgr-user".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let startup_options = StartupOptions {
            start_year: 2032,
            start_phase: StartPhase::MidSeason,
            history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
        };
        let mut world = make_historical_snapshot_world();
        let team_ids: Vec<String> = world.teams.iter().map(|t| t.id.clone()).collect();
        assert!(team_ids.len() >= 2);
        world.competition_definitions = Some(CompetitionDefinitionFile {
            format_version: 1,
            competitions: vec![CompetitionDefinition {
                id: "custom-league".to_string(),
                name: "Custom League".to_string(),
                r#type: domain::league::CompetitionType::League,
                scope: domain::league::CompetitionScope::Domestic,
                region_id: None,
                country_id: None,
                required_region_ids: vec![],
                priority: 0,
                format: FormatDef {
                    kind: domain::league::CompetitionFormat::LeagueTable,
                    legs: None,
                    group_size: None,
                    qualifiers_per_group: None,
                    best_third_qualifiers: None,
                },
                participants: ParticipantSpec {
                    explicit: Some(team_ids.clone()),
                    selector: None,
                },
                berths: Vec::new(),
                season_start_month: None,
                season_start_day: None,
                name_key: None,
                logo: None,
            }],
        });
        let clock = game_clock_for_world(&startup_options, &world.metadata).unwrap();

        let (game, _stats) = build_game_from_world_data(clock, manager, &startup_options, world);

        let custom = game
            .competitions
            .iter()
            .find(|c| c.id == "custom-league")
            .expect("authored competition replaces the auto-built ones");
        assert_eq!(custom.participant_ids, team_ids);
        assert!(
            game.competitions.iter().all(|c| c.id == "custom-league"
                || c.kind == domain::league::CompetitionType::InternationalNation),
            "no auto-generated club competitions when definitions are supplied"
        );
    }

    #[test]
    fn bootstrap_team_selection_preserves_existing_snapshot_state() {
        let manager = domain::manager::Manager::new(
            "mgr-user".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let startup_options = StartupOptions {
            start_year: 2032,
            start_phase: StartPhase::MidSeason,
            history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
        };
        let world = make_historical_snapshot_world();
        let clock = game_clock_for_world(&startup_options, &world.metadata).unwrap();
        let (mut game, stats_state) =
            build_game_from_world_data(clock, manager, &startup_options, world);

        let updated_stats =
            bootstrap_team_selection(&mut game, "team1", StartPhase::MidSeason, stats_state)
                .unwrap();

        assert_eq!(game.league.as_ref().map(|league| league.season), Some(2031));
        assert_eq!(updated_stats.team_matches.len(), 1);
        assert_eq!(updated_stats.player_matches.len(), 1);
        assert_eq!(
            game.teams
                .iter()
                .find(|team| team.id == "team1")
                .and_then(|team| team.manager_id.as_deref()),
            Some("mgr-user")
        );
        assert!(game
            .news
            .iter()
            .any(|article| article.category == NewsCategory::ManagerialChange));
    }

    #[test]
    fn game_clock_for_world_rejects_out_of_range_snapshot_base_year() {
        let startup_options = StartupOptions {
            start_year: 2032,
            start_phase: StartPhase::MidSeason,
            history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
        };
        let mut world = make_historical_snapshot_world();
        world.metadata.base_year = Some(i32::MAX);

        let result = game_clock_for_world(&startup_options, &world.metadata);

        assert_eq!(
            result.unwrap_err(),
            "be.error.createManager.invalidStartYear"
        );
    }

    #[test]
    fn create_new_save_persists_stats_state_on_first_save() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let saves_dir = std::env::temp_dir().join(format!("ofm-game-command-tests-{}", unique));
        std::fs::create_dir_all(&saves_dir).unwrap();
        let mut save_manager = SaveManager::init(&saves_dir).unwrap();
        let game = make_bootstrap_test_game();
        let stats_state = sample_stats_state();

        let save_id =
            create_new_save(&mut save_manager, &game, &stats_state, "Stats Career").unwrap();
        let loaded_stats = save_manager.load_stats_state(&save_id).unwrap();

        assert_eq!(loaded_stats.team_matches.len(), 1);
        assert_eq!(loaded_stats.player_matches.len(), 1);
        assert_eq!(loaded_stats.team_matches[0].team_id, "team1");

        std::fs::remove_dir_all(&saves_dir).unwrap();
    }

    #[test]
    fn require_active_stats_state_returns_backend_key_when_missing() {
        let state = StateManager::new();

        let result = require_active_stats_state(&state);

        assert_eq!(result.unwrap_err(), "be.error.noActiveStatsSession");
    }

    #[test]
    fn require_active_stats_state_clones_active_stats() {
        let state = StateManager::new();
        let stats = sample_stats_state();
        state.set_stats_state(stats.clone());

        let result = require_active_stats_state(&state).unwrap();

        assert_eq!(result.team_matches.len(), stats.team_matches.len());
        assert_eq!(result.player_matches.len(), stats.player_matches.len());
    }

    #[test]
    fn bootstrap_team_selection_midseason_populates_half_season_state() {
        let mut game = make_bootstrap_test_game();

        let stats_state = bootstrap_team_selection(
            &mut game,
            "team1",
            StartPhase::MidSeason,
            domain::stats::StatsState::default(),
        )
        .unwrap();

        let league = game.league.as_ref().unwrap();
        let completed = league
            .fixtures
            .iter()
            .filter(|fixture| {
                fixture.counts_for_league_standings()
                    && fixture.status == domain::league::FixtureStatus::Completed
                    && (fixture.home_team_id == "team1" || fixture.away_team_id == "team1")
            })
            .count();
        let scheduled = league
            .fixtures
            .iter()
            .filter(|fixture| {
                fixture.counts_for_league_standings()
                    && (fixture.home_team_id == "team1" || fixture.away_team_id == "team1")
            })
            .count();
        let team_standing = league
            .standings
            .iter()
            .find(|entry| entry.team_id == "team1")
            .unwrap();

        assert_eq!(completed, scheduled / 2);
        assert!(!stats_state.team_matches.is_empty());
        assert!(!stats_state.player_matches.is_empty());
        assert_eq!(team_standing.played as usize, completed);
        assert!(game
            .news
            .iter()
            .any(|article| article.category == domain::news::NewsCategory::ManagerialChange));
        assert!(game.news.iter().any(|article| {
            matches!(
                article.category,
                domain::news::NewsCategory::MatchReport
                    | domain::news::NewsCategory::LeagueRoundup
                    | domain::news::NewsCategory::StandingsUpdate
            )
        }));
    }

    /// Regression test for issue #225: verifies that bootstrap_team_selection followed by
    /// upgrade_game_player_identities converts generic bucket positions
    /// (Defender/Midfielder/Forward) to granular positions (LeftBack/CentralMidfielder/etc.).
    /// select_team calls both in sequence; it cannot be called directly here because it
    /// requires Tauri App state, so this test exercises the same in-memory operations.
    #[test]
    fn bootstrap_and_upgrade_sets_granular_positions() {
        let startup_options = StartupOptions {
            start_year: 2032,
            start_phase: StartPhase::SeasonStart,
            history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
        };
        let mut world = make_imported_baseline_world_without_staff();
        ofm_core::generator::normalize_imported_world_for_career_start(&mut world);
        let clock = game_clock_for_world(&startup_options, &world.metadata).unwrap();
        let manager = domain::manager::Manager::new(
            "mgr-user".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let (mut game, stats_state) =
            build_game_from_world_data(clock, manager, &startup_options, world);

        // All generated players start with generic (legacy-bucket) positions
        let outfield_before: Vec<_> = game
            .players
            .iter()
            .filter(|p| p.position != domain::player::Position::Goalkeeper)
            .collect();
        assert!(
            outfield_before
                .iter()
                .all(|p| p.natural_position.is_legacy_bucket()),
            "generated players should all start with generic (legacy-bucket) natural_position"
        );

        bootstrap_team_selection(&mut game, "team1", StartPhase::SeasonStart, stats_state).unwrap();
        ofm_core::player_identity::upgrade_game_player_identities(&mut game);

        // After upgrade, outfield players on team1 should have granular natural_position
        let outfield_after: Vec<_> = game
            .players
            .iter()
            .filter(|p| {
                p.team_id.as_deref() == Some("team1")
                    && p.position != domain::player::Position::Goalkeeper
            })
            .collect();
        assert!(
            !outfield_after.is_empty(),
            "team1 should have outfield players"
        );
        assert!(
            outfield_after
                .iter()
                .all(|p| !p.natural_position.is_legacy_bucket()),
            "outfield players on the selected team should have granular natural_position after upgrade"
        );
    }

    #[test]
    fn brazil_state_region_covers_all_standard_br_cities() {
        // All cities from STANDARD_NATIONS BR entry must map to a region so that
        // state-series competitions are generated for every club location.
        let br_cities = [
            "São Paulo",
            "Rio",
            "Belo Horizonte",
            "Porto Alegre",
            "Salvador",
            "Recife",
            "Curitiba",
            "Fortaleza",
            "Goiânia",
            "Santos",
            "Campinas",
            "Belém",
            "Manaus",
            "Vitória",
            "Natal",
            "Florianópolis",
            "Cuiabá",
            "Maceió",
            "Bragantino",
            "Juiz de Fora",
        ];
        for city in br_cities {
            assert!(
                brazil_state_region(city).is_some(),
                "brazil_state_region returned None for BR city: {city}"
            );
        }
        assert_eq!(
            brazil_state_region("Vitória"),
            Some("southeast"),
            "Vitória (ES) belongs in the southeast region, not northeast"
        );
    }
}
