//! The World Cup: a quadrennial national-team tournament played in the summer
//! break (the real-world calendar: 2022, 2026, 2030, …). The field is filled
//! from the strongest national pools in the world; nations without enough
//! players get squads synthesised as free agents, so any world can stage it.

use std::collections::BTreeMap;

use chrono::{DateTime, Datelike, Utc};
use domain::league::{
    CompetitionFormat, CompetitionScope, CompetitionType, FixtureCompetition, FixtureStatus,
    GroupState, League, MatchResult, StandingEntry,
};
use domain::message::{InboxMessage, MessageCategory, MessagePriority};
use domain::national_team::NationalTeam;
use domain::news::{NewsArticle, NewsCategory};
use domain::world_history::{WorldCupChampionRecord, WorldCupHostRecord};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, RngExt, SeedableRng};

use crate::game::Game;
use crate::group_stage::GroupStageConfig;
use crate::nations;

/// A World Cup format preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldCupFormat {
    pub field: usize,
    pub qualifiers_per_group: u32,
    pub best_third_qualifiers: u32,
}

/// The 2026 format: 48 teams in 12 groups; top two per group plus the eight
/// best third-placed teams reach a round of 32.
pub const FORMAT_48: WorldCupFormat = WorldCupFormat {
    field: 48,
    qualifiers_per_group: 2,
    best_third_qualifiers: 8,
};

/// The 1998–2022 format: 32 teams in 8 groups; top two reach a round of 16.
pub const FORMAT_32: WorldCupFormat = WorldCupFormat {
    field: 32,
    qualifiers_per_group: 2,
    best_third_qualifiers: 0,
};

/// A compact format for small worlds: 16 teams in 4 groups; top two reach the
/// quarterfinals.
pub const FORMAT_16: WorldCupFormat = WorldCupFormat {
    field: 16,
    qualifiers_per_group: 2,
    best_third_qualifiers: 0,
};

/// Squads synthesised or topped up reach this size.
const TOPPED_UP_POOL: usize = 18;
/// Days between group matchdays — the tournament must fit the summer break.
const GROUP_MATCHDAY_GAP_DAYS: i64 = 2;
/// Days between knockout rounds, kept tight so the finals fit a real ~5-week
/// World Cup window.
const KNOCKOUT_GAP_DAYS: u32 = 3;

/// World Cups take place in the summers of 2022, 2026, 2030, …
pub fn is_world_cup_summer(year: i32) -> bool {
    year.rem_euclid(4) == 2
}

/// Whether a competition is a World Cup (a national-team tournament).
pub fn is_world_cup_competition(competition: &League) -> bool {
    competition.kind == CompetitionType::InternationalNation
        && competition.scope == CompetitionScope::International
}

/// Player OVRs per nation, strongest first (non-retired players only).
fn national_pools(game: &Game) -> BTreeMap<String, Vec<u8>> {
    let mut pools: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for player in game.players.iter().filter(|player| !player.retired) {
        let nation = if player.football_nation.is_empty() {
            player.nationality.clone()
        } else {
            player.football_nation.clone()
        };
        pools.entry(nation).or_default().push(player.ovr);
    }
    for ovrs in pools.values_mut() {
        ovrs.sort_unstable_by(|a, b| b.cmp(a));
    }
    pools
}

/// Average OVR of a pool's best XI.
fn pool_strength(ovrs: &[u8]) -> f64 {
    let xi: Vec<u8> = ovrs.iter().copied().take(11).collect();
    if xi.is_empty() {
        return 0.0;
    }
    xi.iter().map(|&ovr| ovr as u32).sum::<u32>() as f64 / xi.len() as f64
}

/// Pick the World Cup field, balanced across confederations like a real
/// tournament: each region gets a berth share (largest-remainder, via
/// [`berths_by_region`]) and contributes its strongest nations. A
/// confederation-balanced field keeps the FIFA draw's per-group caps
/// satisfiable and the bracket realistic.
fn select_field(game: &Game, format: &WorldCupFormat) -> Vec<String> {
    let pools = national_pools(game);

    // Candidate nations by region (catalog guarantees coverage in empty worlds).
    let mut candidate_codes: Vec<String> = pools.keys().cloned().collect();
    for nation in nations::NATION_CATALOG {
        candidate_codes.push(nation.code.to_string());
    }
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut by_region: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for code in candidate_codes {
        if !seen.insert(code.clone()) {
            continue;
        }
        by_region
            .entry(nations::region_for_code(&code).to_string())
            .or_default()
            .push(code);
    }
    let strength = |code: &str| pools.get(code).map(|ovrs| pool_strength(ovrs)).unwrap_or(0.0);
    for codes in by_region.values_mut() {
        codes.sort_by(|a, b| {
            strength(b)
                .partial_cmp(&strength(a))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.cmp(b))
        });
    }

    let counts: BTreeMap<String, usize> =
        by_region.iter().map(|(region, codes)| (region.clone(), codes.len())).collect();
    let berths = berths_by_region(format.field, &counts);

    let mut field: Vec<String> = Vec::new();
    for (region, codes) in &by_region {
        let take = berths.get(region).copied().unwrap_or(0);
        field.extend(codes.iter().take(take).cloned());
    }
    field.truncate(format.field);
    field
}

fn national_team_id(code: &str) -> String {
    format!("nt-{}", code.to_lowercase())
}

/// Real-world World Cup hosts (and the known near-future editions), so a
/// generated world honours actual history for these years and never reuses a
/// recent real host when awarding new ones.
const REAL_WORLD_HOSTS: &[(u32, &str)] = &[
    (1930, "UY"), (1934, "IT"), (1938, "FR"), (1950, "BR"), (1954, "CH"),
    (1958, "SE"), (1962, "CL"), (1966, "ENG"), (1970, "MX"), (1974, "DE"),
    (1978, "AR"), (1982, "ES"), (1986, "MX"), (1990, "IT"), (1994, "US"),
    (1998, "FR"), (2002, "JP"), (2006, "DE"), (2010, "ZA"), (2014, "BR"),
    (2018, "RU"), (2022, "QA"), (2026, "US"), (2030, "ES"), (2034, "SA"),
];

/// The host nation code for a World Cup `year`: a host the game awarded, else
/// the real-world host where known.
fn host_for_year(game: &Game, year: i32) -> Option<String> {
    let year = year as u32;
    game.world_history
        .world_cup_host(year)
        .map(str::to_string)
        .or_else(|| {
            REAL_WORLD_HOSTS
                .iter()
                .find(|(host_year, _)| *host_year == year)
                .map(|(_, code)| code.to_string())
        })
}

/// Initial ranking points for a nation with the given squad strength: a neutral
/// base lifted by best-XI quality, so a fresh world's pots reflect strength
/// before any results have moved the ranking.
fn seed_points_for(strength: f64) -> f64 {
    1000.0 + strength * 10.0
}

/// Seed world-ranking points (from squad strength) for any field nation that
/// has not been ranked yet. Existing ranking points are left untouched so
/// accumulated results are preserved across tournaments.
pub(crate) fn seed_world_ranking(game: &mut Game, field: &[String]) {
    let pools = national_pools(game);
    for code in field {
        let strength = pools.get(code).map(|ovrs| pool_strength(ovrs)).unwrap_or(0.0);
        game.world_history.seed_ranking(code, seed_points_for(strength));
    }
}

/// Order nation `codes` by world ranking, strongest first, falling back to
/// squad strength for any nation not yet ranked.
pub(crate) fn ranked_field(game: &Game, codes: &[String]) -> Vec<String> {
    let pools = national_pools(game);
    let points_of = |code: &str| -> f64 {
        game.world_history.ranking_points(code).unwrap_or_else(|| {
            seed_points_for(pools.get(code).map(|ovrs| pool_strength(ovrs)).unwrap_or(0.0))
        })
    };
    let mut ordered: Vec<String> = codes.to_vec();
    ordered.sort_by(|a, b| {
        points_of(b)
            .partial_cmp(&points_of(a))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.cmp(b))
    });
    ordered
}

/// Confederation cap for one World Cup group: at most one team per confederation
/// except UEFA (`europe`), where up to two may share a group.
fn confederation_cap(region: &str) -> usize {
    if region == "europe" { 2 } else { 1 }
}

/// Whether `code` may be added to `group` without breaching the confederation
/// cap.
fn group_admits(group: &[String], code: &str) -> bool {
    let region = nations::region_for_code(code);
    let cap = confederation_cap(region);
    group
        .iter()
        .filter(|other| nations::region_for_code(other) == region)
        .count()
        < cap
}

/// Place each team of one pot into a distinct group (one per group) without
/// breaching the confederation cap, by backtracking. Returns whether it found a
/// full assignment.
fn place_pot(pot: &[String], used: &mut [bool], group_index: usize, groups: &mut [Vec<String>]) -> bool {
    if group_index == groups.len() {
        return true;
    }
    for (i, code) in pot.iter().enumerate() {
        if used[i] || !group_admits(&groups[group_index], code) {
            continue;
        }
        groups[group_index].push(code.clone());
        used[i] = true;
        if place_pot(pot, used, group_index + 1, groups) {
            return true;
        }
        used[i] = false;
        groups[group_index].pop();
    }
    false
}

/// Draw the field into groups of four following FIFA rules: pots seeded by world
/// ranking (the host forced into Pot 1), then a random draw keeping at most one
/// team per confederation in a group — except UEFA, where two may share one.
/// Returns the groups as nation codes.
fn draw_world_cup_groups(
    game: &Game,
    field_codes: &[String],
    host_code: Option<&str>,
    rng: &mut impl Rng,
) -> Vec<Vec<String>> {
    const GROUP_SIZE: usize = 4;
    let mut ranked = ranked_field(game, field_codes);
    // The host is seeded into Pot 1 regardless of its ranking.
    if let Some(host) = host_code
        && let Some(position) = ranked.iter().position(|code| code == host)
    {
        let host = ranked.remove(position);
        ranked.insert(0, host);
    }
    let group_count = (ranked.len() / GROUP_SIZE).max(1);
    let mut groups: Vec<Vec<String>> = vec![Vec::new(); group_count];

    for pot_index in 0..GROUP_SIZE {
        let start = pot_index * group_count;
        let end = ((pot_index + 1) * group_count).min(ranked.len());
        if start >= end {
            break;
        }
        let mut pot: Vec<String> = ranked[start..end].to_vec();
        let mut placed = false;
        // Reshuffle a few times until the confederation constraint is satisfiable.
        for _ in 0..64 {
            pot.shuffle(rng);
            let mut used = vec![false; pot.len()];
            let mut trial = groups.clone();
            if place_pot(&pot, &mut used, 0, &mut trial) {
                groups = trial;
                placed = true;
                break;
            }
        }
        if !placed {
            // Constraint unsatisfiable for this pot (degenerate field): fall back
            // to a plain one-per-group placement.
            for (offset, code) in pot.iter().enumerate() {
                groups[offset % group_count].push(code.clone());
            }
        }
    }
    groups
}

/// Top up thin national pools with generated free agents and (re)build the
/// national-team squads for every nation in the field.
fn prepare_national_squads(game: &mut Game, field: &[String]) {
    let pools = national_pools(game);
    for code in field {
        let have = pools.get(code).map(|ovrs| ovrs.len()).unwrap_or(0);
        for slot in have..TOPPED_UP_POOL {
            game.players
                .push(crate::generator::generate_national_team_player(code, slot));
        }
    }

    for code in field {
        let mut squad: Vec<(String, u8)> = game
            .players
            .iter()
            .filter(|player| !player.retired)
            .filter(|player| {
                let nation = if player.football_nation.is_empty() {
                    &player.nationality
                } else {
                    &player.football_nation
                };
                nation == code
            })
            .map(|player| (player.id.clone(), player.ovr))
            .collect();
        squad.sort_by(|a, b| b.1.cmp(&a.1));
        let squad_player_ids: Vec<String> =
            squad.into_iter().take(23).map(|(id, _)| id).collect();

        let nation_name_key = Some(format!("nations.{}", code.to_lowercase()));
        if let Some(team) = game
            .national_teams
            .iter_mut()
            .find(|team| &team.football_nation == code)
        {
            team.squad_player_ids = squad_player_ids;
            if team.name_key.is_none() {
                team.name_key = nation_name_key;
            }
        } else {
            let mut team = NationalTeam::new(
                national_team_id(code),
                nations::nation_display_name(code),
                code.clone(),
                nations::nation_by_code(code).map(|nation| nation.region_id.to_string()),
            );
            team.squad_player_ids = squad_player_ids;
            team.name_key = nation_name_key;
            game.national_teams.push(team);
        }
    }
}

/// Schedule a World Cup for the summer beginning at `kickoff` when the year is
/// a World Cup year and none is already running. Returns whether one was
/// scheduled.
pub fn schedule_world_cup_if_due(game: &mut Game, kickoff: DateTime<Utc>) -> bool {
    if !is_world_cup_summer(kickoff.year()) {
        return false;
    }
    if game.competitions.iter().any(is_world_cup_competition) {
        return false;
    }
    schedule_world_cup(game, kickoff, &FORMAT_48);
    true
}

/// Schedule a World Cup with an explicit format (48 by default; 32 and 16
/// reproduce the older tournaments). The field is chosen by ranking.
pub fn schedule_world_cup(game: &mut Game, kickoff: DateTime<Utc>, format: &WorldCupFormat) {
    schedule_world_cup_with_field(game, kickoff, format, None);
}

/// Schedule a World Cup, optionally with a pre-determined field (e.g. from
/// qualifying). When `predetermined_field` is `None` the field is chosen by
/// ranking the strongest nations.
pub fn schedule_world_cup_with_field(
    game: &mut Game,
    kickoff: DateTime<Utc>,
    format: &WorldCupFormat,
    predetermined_field: Option<Vec<String>>,
) {
    let year = kickoff.year();
    let host_code = host_for_year(game, year);
    let mut field = predetermined_field.unwrap_or_else(|| select_field(game, format));
    if field.len() < 4 {
        return;
    }
    // The host auto-qualifies: ensure it is in the field, displacing the weakest
    // entrant if the field is already full so the size stays constant.
    if let Some(host) = host_code.as_deref()
        && !field.iter().any(|code| code == host)
    {
        if field.len() >= format.field {
            field.pop();
        }
        field.push(host.to_string());
    }
    prepare_national_squads(game, &field);
    seed_world_ranking(game, &field);

    let id_for = |code: &str| -> String {
        game.national_teams
            .iter()
            .find(|team| team.football_nation == code)
            .map(|team| team.id.clone())
            .unwrap_or_else(|| national_team_id(code))
    };
    // FIFA draw: pots seeded by world ranking (host into Pot 1), one team per
    // confederation per group except UEFA (≤2). Deterministic per cup year.
    let mut draw_rng = StdRng::seed_from_u64(year as u64);
    let group_ids: Vec<Vec<String>> =
        draw_world_cup_groups(game, &field, host_code.as_deref(), &mut draw_rng)
            .iter()
            .map(|group| group.iter().map(|code| id_for(code)).collect())
            .collect();

    let mut cup = crate::group_stage::generate_group_knockout_cup_with_groups(
        &format!("World Cup {year}"),
        year as u32,
        &group_ids,
        kickoff,
        CompetitionType::InternationalNation,
        CompetitionScope::International,
        &GroupStageConfig {
            legs: 1,
            matchday_gap_days: GROUP_MATCHDAY_GAP_DAYS,
            qualifiers_per_group: format.qualifiers_per_group,
            best_third_qualifiers: format.best_third_qualifiers,
            knockout_round_gap_days: KNOCKOUT_GAP_DAYS,
            max_concurrent_matches_per_day: Some(4),
            knockout_matches_per_day: 4,
        },
    );
    // Sort after every club competition in browsing lists.
    cup.priority = 10_000;
    cup.name_key = Some("tournaments.competitions.worldCup".to_string());
    let cup_id = cup.id.clone();
    game.competitions.push(cup);
    if !game.active_competition_ids.is_empty() {
        game.active_competition_ids.push(cup_id);
    }

    // The whole world hears about a World Cup, participant or not.
    let kickoff_news_id = format!("world_cup_kickoff_{year}");
    if !game.news.iter().any(|article| article.id == kickoff_news_id) {
        let mut params = std::collections::HashMap::new();
        params.insert("year".to_string(), year.to_string());
        params.insert("nations".to_string(), field.len().to_string());
        game.news.push(
            NewsArticle::new(
                kickoff_news_id,
                String::new(),
                String::new(),
                String::new(),
                kickoff.format("%Y-%m-%d").to_string(),
                NewsCategory::Editorial,
            )
            .with_i18n(
                "be.news.worldCupKickoff.headline",
                "be.news.worldCupKickoff.body",
                "be.source.footballHerald",
                params,
            ),
        );
    }
}

// ---------------------------------------------------------------------------
// Qualifying
// ---------------------------------------------------------------------------

const QUALIFYING_COMPETITION_PREFIX: &str = "world-cup-qualifying-";
/// Maximum nations per qualifying group; a round robin of this size fits the
/// five international windows.
const QUALIFYING_GROUP_SIZE: usize = 6;

/// Whether a competition is a World Cup qualifying campaign.
pub fn is_world_cup_qualifying(competition: &League) -> bool {
    competition.id.starts_with(QUALIFYING_COMPETITION_PREFIX)
}

/// A season "leads into" a World Cup when the following summer is a cup summer.
pub fn season_leads_into_world_cup(season_start: DateTime<Utc>) -> bool {
    is_world_cup_summer(season_start.year() + 1)
}

fn national_team_id_for(game: &Game, code: &str) -> String {
    game.national_teams
        .iter()
        .find(|team| team.football_nation == code)
        .map(|team| team.id.clone())
        .unwrap_or_else(|| national_team_id(code))
}

pub(crate) fn nation_code_of_national_team(team_id: &str) -> String {
    team_id
        .strip_prefix("nt-")
        .map(|code| code.to_uppercase())
        .unwrap_or_else(|| team_id.to_string())
}

fn region_of_code(game: &Game, code: &str) -> String {
    game.region_for_country(code)
}

/// Candidate nations grouped by region: every world nation with players plus
/// the catalog nations, deduped.
fn qualifying_candidates_by_region(game: &Game) -> BTreeMap<String, Vec<String>> {
    let pools = national_pools(game);
    let mut codes: Vec<String> = pools.keys().cloned().collect();
    for nation in nations::NATION_CATALOG {
        codes.push(nation.code.to_string());
    }

    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut by_region: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for code in codes {
        if !seen.insert(code.clone()) {
            continue;
        }
        by_region
            .entry(region_of_code(game, &code))
            .or_default()
            .push(code);
    }
    by_region
}

/// Allocate `field_size` World Cup berths across regions, proportional to how
/// many nations each region has (largest-remainder method), at least one berth
/// per region with a nation, capped at each region's nation count.
pub fn berths_by_region(
    field_size: usize,
    nations_by_region: &BTreeMap<String, usize>,
) -> BTreeMap<String, usize> {
    let regions: Vec<(String, usize)> = nations_by_region
        .iter()
        .filter(|(_, count)| **count > 0)
        .map(|(region, count)| (region.clone(), *count))
        .collect();
    let total: usize = regions.iter().map(|(_, count)| count).sum();
    if total == 0 || field_size == 0 {
        return BTreeMap::new();
    }

    let mut berths: BTreeMap<String, usize> = BTreeMap::new();

    // More regions than berths: one berth each to the largest regions.
    if field_size <= regions.len() {
        let mut sorted = regions;
        sorted.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        for (region, _) in sorted.into_iter().take(field_size) {
            berths.insert(region, 1);
        }
        return berths;
    }

    let mut allocated = 0usize;
    let mut remainders: Vec<(String, f64)> = Vec::new();
    for (region, count) in &regions {
        let exact = field_size as f64 * *count as f64 / total as f64;
        let base = (exact.floor() as usize).max(1).min(*count);
        berths.insert(region.clone(), base);
        allocated += base;
        remainders.push((region.clone(), exact - exact.floor()));
    }
    remainders.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    while allocated > field_size {
        if let Some((region, _)) = remainders
            .iter()
            .rev()
            .find(|(r, _)| berths.get(r).copied().unwrap_or(0) > 1)
        {
            *berths.get_mut(region).unwrap() -= 1;
            allocated -= 1;
        } else {
            break;
        }
    }
    let mut index = 0usize;
    let guard = remainders.len() * (field_size + 1);
    while allocated < field_size && index < guard {
        let (region, _) = &remainders[index % remainders.len()];
        let cap = *nations_by_region.get(region).unwrap_or(&0);
        if berths.get(region).copied().unwrap_or(0) < cap {
            *berths.entry(region.clone()).or_insert(0) += 1;
            allocated += 1;
        }
        index += 1;
    }
    berths
}

/// Spread qualifying fixtures so each matchday's matches fan out across its
/// international window's [`INTERNATIONAL_WINDOW_SPAN_DAYS`]-day block (matchday
/// N opens on `window_dates[N-1]`) rather than all landing on the opening date.
/// The per-day count is sized so every match still fits inside the block.
fn spread_qualifying_over_windows(fixtures: &mut [domain::league::Fixture], window_dates: &[String]) {
    use std::collections::BTreeMap;
    if window_dates.is_empty() {
        return;
    }
    let span = crate::national_team::INTERNATIONAL_WINDOW_SPAN_DAYS;

    let mut per_matchday: BTreeMap<u32, usize> = BTreeMap::new();
    for fixture in fixtures.iter() {
        *per_matchday.entry(fixture.matchday).or_default() += 1;
    }

    // Deterministic order so day assignment is stable across runs.
    fixtures.sort_by(|a, b| a.matchday.cmp(&b.matchday).then(a.id.cmp(&b.id)));

    let mut seen: BTreeMap<u32, i64> = BTreeMap::new();
    for fixture in fixtures.iter_mut() {
        let window = (fixture.matchday as usize).saturating_sub(1).min(window_dates.len() - 1);
        let Some(base) = chrono::NaiveDate::parse_from_str(&window_dates[window], "%Y-%m-%d").ok()
        else {
            continue;
        };
        let count = per_matchday.get(&fixture.matchday).copied().unwrap_or(1) as i64;
        let per_day = ((count + span - 1) / span).max(1);
        let index = seen.entry(fixture.matchday).or_default();
        let offset = (*index / per_day).min(span - 1);
        *index += 1;
        fixture.date = (base + chrono::Duration::days(offset))
            .format("%Y-%m-%d")
            .to_string();
    }
}

fn standing_order(a: &StandingEntry, b: &StandingEntry) -> std::cmp::Ordering {
    b.points
        .cmp(&a.points)
        .then(b.goal_difference().cmp(&a.goal_difference()))
        .then(b.goals_for.cmp(&a.goals_for))
}

/// Schedule World Cup qualifying for `wc_year` across the season's international
/// windows: per-region groups playing a single round robin, one matchday per
/// window. National squads are prepared for every candidate nation.
pub fn schedule_world_cup_qualifying(
    game: &mut Game,
    wc_year: i32,
    window_dates: &[String],
) {
    if window_dates.is_empty() {
        return;
    }
    let candidates = qualifying_candidates_by_region(game);
    let all_codes: Vec<String> = candidates.values().flatten().cloned().collect();
    if all_codes.len() < 4 {
        return;
    }
    prepare_national_squads(game, &all_codes);

    let competition_id = format!("{QUALIFYING_COMPETITION_PREFIX}{wc_year}");
    let mut competition = League::new(
        competition_id.clone(),
        format!("World Cup Qualifying {wc_year}"),
        wc_year as u32,
        &[],
    );
    competition.kind = CompetitionType::InternationalNation;
    competition.scope = CompetitionScope::International;
    competition.rules.format = CompetitionFormat::LeagueTable;
    competition.standings.clear();
    competition.priority = 9_000;
    competition.name_key = Some("tournaments.competitions.worldCupQualifying".to_string());

    let base_date = chrono::NaiveDate::parse_from_str(&window_dates[0], "%Y-%m-%d")
        .ok()
        .and_then(|date| date.and_hms_opt(0, 0, 0))
        .map(|naive| DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
        .unwrap_or_else(Utc::now);

    let mut participant_ids: Vec<String> = Vec::new();
    for (region, codes) in &candidates {
        let group_count = codes.len().div_ceil(QUALIFYING_GROUP_SIZE).max(1);
        let mut groups: Vec<Vec<String>> = vec![Vec::new(); group_count];
        for (index, code) in codes.iter().enumerate() {
            groups[index % group_count].push(code.clone());
        }

        for (group_index, group_codes) in groups.iter().enumerate() {
            if group_codes.len() < 2 {
                continue;
            }
            let team_ids: Vec<String> = group_codes
                .iter()
                .map(|code| national_team_id_for(game, code))
                .collect();
            participant_ids.extend(team_ids.iter().cloned());

            let fixtures = crate::schedule::build_round_robin_fixtures_with(
                &competition_id,
                &team_ids,
                base_date,
                FixtureCompetition::InternationalNation,
                1,
                3,
            );
            competition.fixtures.extend(fixtures);
            competition.groups.push(GroupState {
                id: format!("{competition_id}-{region}-{group_index}"),
                name: format!("{region} {}", group_index + 1),
                team_ids: team_ids.clone(),
                standings: team_ids
                    .iter()
                    .map(|id| StandingEntry::new(id.clone()))
                    .collect(),
            });
        }
    }

    if competition.groups.is_empty() {
        return;
    }
    // Every group plays the same matchday in a window; spread those matches
    // across the window's multi-day block so no single calendar day is swamped.
    spread_qualifying_over_windows(&mut competition.fixtures, window_dates);
    competition.participant_ids = participant_ids;
    let competition_id = competition.id.clone();
    game.competitions.push(competition);
    if !game.active_competition_ids.is_empty() {
        game.active_competition_ids.push(competition_id);
    }

    // News: the qualifying campaign gets under way.
    let news_id = format!("world_cup_qualifying_{wc_year}");
    if !game.news.iter().any(|article| article.id == news_id) {
        let mut params = std::collections::HashMap::new();
        params.insert("year".to_string(), wc_year.to_string());
        params.insert("nations".to_string(), all_codes.len().to_string());
        game.news.push(
            NewsArticle::new(
                news_id,
                String::new(),
                String::new(),
                String::new(),
                window_dates[0].clone(),
                NewsCategory::Editorial,
            )
            .with_i18n(
                "be.news.worldCupQualifying.headline",
                "be.news.worldCupQualifying.body",
                "be.source.footballHerald",
                params,
            ),
        );
    }
}

/// The qualified field (nation codes) from a completed qualifying campaign:
/// each region's berths go to its best group finishers (winners first, then
/// runners-up, ranked across the region's groups). Returns `None` when there is
/// no qualifying campaign to read.
pub fn qualified_field_from_game(game: &Game, field_size: usize) -> Option<Vec<String>> {
    let competition = game
        .competitions
        .iter()
        .find(|competition| is_world_cup_qualifying(competition))?;

    let mut groups_by_region: BTreeMap<String, Vec<Vec<StandingEntry>>> = BTreeMap::new();
    let mut nations_by_region: BTreeMap<String, usize> = BTreeMap::new();
    for group in &competition.groups {
        let region = group
            .team_ids
            .first()
            .map(|id| region_of_code(game, &nation_code_of_national_team(id)))
            .unwrap_or_else(|| "europe".to_string());
        let sorted = crate::group_stage::sorted_group_standings(group);
        *nations_by_region.entry(region.clone()).or_insert(0) += sorted.len();
        groups_by_region.entry(region).or_default().push(sorted);
    }

    let berths = berths_by_region(field_size, &nations_by_region);
    let mut field: Vec<String> = Vec::new();
    for (region, groups) in &groups_by_region {
        let want = berths.get(region).copied().unwrap_or(0);
        let max_rank = groups.iter().map(|group| group.len()).max().unwrap_or(0);
        let mut qualified = 0usize;
        'ranks: for rank in 0..max_rank {
            let mut at_rank: Vec<&StandingEntry> =
                groups.iter().filter_map(|group| group.get(rank)).collect();
            at_rank.sort_by(|a, b| standing_order(a, b));
            for entry in at_rank {
                if qualified >= want {
                    break 'ranks;
                }
                field.push(nation_code_of_national_team(&entry.team_id));
                qualified += 1;
            }
        }
    }
    Some(field)
}

/// Simulate every World Cup fixture due on `today` with the national-team
/// engine (carry-back included), progressing groups and knockout rounds, and
/// announcing the champion when the final is decided. Returns the number of
/// fixtures simulated.
pub fn process_world_cup_fixtures_due(game: &mut Game, today: &str, rng: &mut impl Rng) -> usize {
    let competition_indices: Vec<usize> = game
        .competitions
        .iter()
        .enumerate()
        .filter(|(_, competition)| is_world_cup_competition(competition))
        .map(|(index, _)| index)
        .collect();

    let mut simulated = 0;
    for competition_index in competition_indices {
        let due: Vec<usize> = game.competitions[competition_index]
            .fixtures
            .iter()
            .enumerate()
            .filter(|(_, fixture)| {
                fixture.date == today && fixture.status == FixtureStatus::Scheduled
            })
            .map(|(fixture_index, _)| fixture_index)
            .collect();

        for fixture_index in due {
            let (home_id, away_id, fixture_id) = {
                let fixture = &game.competitions[competition_index].fixtures[fixture_index];
                (
                    fixture.home_team_id.clone(),
                    fixture.away_team_id.clone(),
                    fixture.id.clone(),
                )
            };
            // Knockout ties must produce a winner: extra time, then penalties.
            let is_knockout = game.competitions[competition_index]
                .knockout_rounds
                .iter()
                .any(|round| round.fixture_ids.contains(&fixture_id));
            let (
                home_goals,
                away_goals,
                home_scorers,
                away_scorers,
                home_penalties,
                away_penalties,
            ) = if is_knockout {
                crate::national_team::play_national_knockout_match(game, &home_id, &away_id, rng)
            } else {
                let (home_goals, away_goals, home_scorers, away_scorers) =
                    crate::national_team::play_national_match(game, &home_id, &away_id, rng);
                (home_goals, away_goals, home_scorers, away_scorers, None, None)
            };

            let competition = &mut game.competitions[competition_index];
            let fixture = &mut competition.fixtures[fixture_index];
            fixture.status = FixtureStatus::Completed;
            fixture.result = Some(MatchResult {
                home_goals,
                away_goals,
                home_scorers,
                away_scorers,
                report: None,
                home_penalties,
                away_penalties,
            });
            crate::group_stage::process_completed_fixture(competition, fixture_index);
            crate::schedule::advance_knockout_competition_round(competition);
            game.world_history.apply_national_result(
                &nation_code_of_national_team(&home_id),
                &nation_code_of_national_team(&away_id),
                home_goals,
                away_goals,
                home_penalties.is_some(),
            );
            simulated += 1;
        }

        announce_champion_if_decided(game, competition_index, today, rng);
    }
    simulated
}

/// The tournament winner, once the final has been played.
pub fn world_cup_champion(competition: &League) -> Option<String> {
    let last_round = competition.knockout_rounds.last()?;
    if !last_round.completed || last_round.fixture_ids.len() != 1 {
        return None;
    }
    let final_fixture = competition
        .fixtures
        .iter()
        .find(|fixture| last_round.fixture_ids.contains(&fixture.id))?;
    let result = final_fixture.result.as_ref()?;
    Some(if result.advancing_is_home() {
        final_fixture.home_team_id.clone()
    } else {
        final_fixture.away_team_id.clone()
    })
}

fn announce_champion_if_decided(
    game: &mut Game,
    competition_index: usize,
    today: &str,
    rng: &mut impl Rng,
) {
    let competition = &game.competitions[competition_index];
    let Some(champion_id) = world_cup_champion(competition) else {
        return;
    };
    let year = competition.season;
    let msg_id = format!("world_cup_champion_{year}");
    if game.messages.iter().any(|message| message.id == msg_id) {
        return;
    }

    let (nation, nation_code) = game
        .national_teams
        .iter()
        .find(|team| team.id == champion_id)
        .map(|team| (team.name.clone(), team.football_nation.clone()))
        .unwrap_or((champion_id.clone(), String::new()));

    // The game world's highest honour goes into the hall of fame.
    game.world_history
        .record_world_cup_champion(WorldCupChampionRecord {
            year,
            nation_code,
            nation_name: nation.clone(),
        });

    let mut params = std::collections::HashMap::new();
    params.insert("nation".to_string(), nation);
    params.insert("year".to_string(), year.to_string());

    let message = InboxMessage::new(
        msg_id,
        String::new(),
        String::new(),
        String::new(),
        today.to_string(),
    )
    .with_category(MessageCategory::LeagueInfo)
    .with_priority(MessagePriority::High)
    .with_sender_role("")
    .with_i18n(
        "be.msg.worldCupChampion.subject",
        "be.msg.worldCupChampion.body",
        params.clone(),
    )
    .with_sender_i18n("be.sender.intlLiaison", "be.role.intlLiaison");
    game.messages.push(message);

    // Front-page news for everyone, participant or not.
    let news_id = format!("world_cup_champion_news_{year}");
    if !game.news.iter().any(|article| article.id == news_id) {
        game.news.push(
            NewsArticle::new(
                news_id,
                String::new(),
                String::new(),
                String::new(),
                today.to_string(),
                NewsCategory::Editorial,
            )
            .with_i18n(
                "be.news.worldCupChampion.headline",
                "be.news.worldCupChampion.body",
                "be.source.footballHerald",
                params,
            ),
        );
    }

    // With the champion crowned, the bid race for a future edition is decided.
    award_next_world_cup_host(game, year, today, rng);
}

/// After a World Cup ends, award the next edition whose host is not already
/// known (real-world hosts stand for their years). A handful of strong nations
/// that have not hosted recently form a shortlist; one is chosen at random.
/// Both the bid race and the award make the news, and the host is stored so it
/// auto-qualifies and is seeded into Pot 1 of that tournament's draw.
fn award_next_world_cup_host(game: &mut Game, played_year: u32, today: &str, rng: &mut impl Rng) {
    let next_year = played_year + 4;
    if host_for_year(game, next_year as i32).is_some() {
        return;
    }
    // Hosts used within the last ~24 years are off the table.
    let mut recent: std::collections::HashSet<String> = std::collections::HashSet::new();
    for year in next_year.saturating_sub(24)..next_year {
        if let Some(code) = host_for_year(game, year as i32) {
            recent.insert(code);
        }
    }
    let candidates: Vec<String> = nations::NATION_CATALOG
        .iter()
        .map(|nation| nation.code.to_string())
        .filter(|code| !recent.contains(code))
        .collect();
    let shortlist = ranked_field(game, &candidates);
    if shortlist.is_empty() {
        return;
    }
    let shortlist_len = shortlist.len().min(4);
    let chosen = shortlist[rng.random_range(0..shortlist_len)].clone();
    let nation_name = nations::nation_display_name(&chosen);

    game.world_history.record_world_cup_host(WorldCupHostRecord {
        year: next_year,
        nation_code: chosen,
        nation_name: nation_name.clone(),
    });

    let bid_id = format!("world_cup_host_bid_{next_year}");
    if !game.news.iter().any(|article| article.id == bid_id) {
        let mut params = std::collections::HashMap::new();
        params.insert("count".to_string(), shortlist_len.to_string());
        params.insert("year".to_string(), next_year.to_string());
        game.news.push(
            NewsArticle::new(
                bid_id,
                String::new(),
                String::new(),
                String::new(),
                today.to_string(),
                NewsCategory::Editorial,
            )
            .with_i18n(
                "be.news.worldCupHostBid.headline",
                "be.news.worldCupHostBid.body",
                "be.source.footballHerald",
                params,
            ),
        );
    }

    let host_id = format!("world_cup_host_{next_year}");
    if !game.news.iter().any(|article| article.id == host_id) {
        let mut params = std::collections::HashMap::new();
        params.insert("nation".to_string(), nation_name);
        params.insert("year".to_string(), next_year.to_string());
        game.news.push(
            NewsArticle::new(
                host_id,
                String::new(),
                String::new(),
                String::new(),
                today.to_string(),
                NewsCategory::Editorial,
            )
            .with_i18n(
                "be.news.worldCupHostChosen.headline",
                "be.news.worldCupHostChosen.body",
                "be.source.footballHerald",
                params,
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use chrono::TimeZone;
    use domain::manager::Manager;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn kickoff(year: i32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(year, 6, 10, 0, 0, 0).unwrap()
    }

    fn empty_game() -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap());
        let manager = Manager::new(
            "mgr".to_string(),
            "Alex".to_string(),
            "Boss".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        Game::new(clock, manager, vec![], vec![], vec![], vec![])
    }

    #[test]
    fn berths_sum_to_the_field_and_respect_caps() {
        let mut counts = std::collections::BTreeMap::new();
        counts.insert("europe".to_string(), 26usize);
        counts.insert("asia".to_string(), 10usize);
        counts.insert("oceania".to_string(), 2usize);

        let berths = berths_by_region(16, &counts);

        assert_eq!(berths.values().sum::<usize>(), 16);
        assert!((1..=2).contains(&berths["oceania"]), "small region keeps a berth but is capped");
        assert!(berths["europe"] > berths["asia"], "berths scale with region size");
    }

    #[test]
    fn qualifying_feeds_a_world_cup_field() {
        use chrono::TimeZone;
        let mut game = empty_game();
        let windows = crate::national_team::international_window_dates(
            Utc.with_ymd_and_hms(2025, 8, 1, 0, 0, 0).unwrap(),
        );

        schedule_world_cup_qualifying(&mut game, 2026, &windows);

        // Matches spread across each window's multi-day block.
        let window_block = crate::national_team::international_window_span_dates(&windows);
        {
            let qualifying = game
                .competitions
                .iter()
                .find(|c| is_world_cup_qualifying(c))
                .expect("qualifying is scheduled");
            assert!(!qualifying.groups.is_empty());
            assert!(qualifying.fixtures.iter().all(|f| {
                window_block.contains(&f.date)
                    && f.competition == FixtureCompetition::InternationalNation
            }));
        }
        assert!(
            game.news.iter().any(|a| a.id == "world_cup_qualifying_2026"),
            "the qualifying campaign makes the news"
        );

        // Play every qualifying matchday across the spread window blocks.
        let mut rng = StdRng::seed_from_u64(11);
        for date in &window_block {
            process_world_cup_fixtures_due(&mut game, date, &mut rng);
        }
        // Group tables recorded results.
        let played = game
            .competitions
            .iter()
            .find(|c| is_world_cup_qualifying(c))
            .unwrap()
            .groups
            .iter()
            .any(|g| g.standings.iter().any(|s| s.played > 0));
        assert!(played, "qualifying group tables update as matches are played");

        let field = qualified_field_from_game(&game, FORMAT_16.field).expect("a field");
        assert_eq!(field.len(), FORMAT_16.field);
        let distinct: std::collections::HashSet<&String> = field.iter().collect();
        assert_eq!(distinct.len(), field.len(), "qualified nations are distinct");
    }

    /// Max number of fixtures that fall on any single calendar date.
    fn max_fixtures_per_day(competition: &League) -> usize {
        let mut per_day: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for fixture in &competition.fixtures {
            *per_day.entry(fixture.date.as_str()).or_default() += 1;
        }
        per_day.values().copied().max().unwrap_or(0)
    }

    #[test]
    fn world_cup_finals_never_overload_a_calendar_day() {
        // FORMAT_48 finals: groups + knockouts must never overload a calendar day.
        let mut game = empty_game();
        schedule_world_cup(&mut game, kickoff(2026), &FORMAT_48);
        let mut rng = StdRng::seed_from_u64(99);
        for _ in 0..400 {
            let next = game
                .competitions
                .iter()
                .filter(|c| is_world_cup_competition(c))
                .flat_map(|c| c.fixtures.iter())
                .filter(|f| f.status == FixtureStatus::Scheduled)
                .map(|f| f.date.clone())
                .min();
            let Some(date) = next else { break };
            process_world_cup_fixtures_due(&mut game, &date, &mut rng);
        }
        let finals = game
            .competitions
            .iter()
            .find(|c| is_world_cup_competition(c))
            .unwrap();
        assert!(
            max_fixtures_per_day(finals) <= 4,
            "finals should never exceed 4 matches on one day, saw {}",
            max_fixtures_per_day(finals)
        );

        // The whole tournament should fit a real World Cup window (~5-6 weeks).
        let dates: Vec<chrono::NaiveDate> = finals
            .fixtures
            .iter()
            .filter_map(|f| chrono::NaiveDate::parse_from_str(&f.date, "%Y-%m-%d").ok())
            .collect();
        let span_days =
            (*dates.iter().max().unwrap() - *dates.iter().min().unwrap()).num_days();
        assert!(
            span_days <= 42,
            "finals should fit a real World Cup window, spanned {span_days} days"
        );
    }

    #[test]
    fn qualifying_spreads_matches_across_window_blocks() {
        // Every region's groups play the same matchday in a window; without
        // spreading, a full catalog world piles ~28 matches onto one date.
        let mut game = empty_game();
        let windows = crate::national_team::international_window_dates(
            Utc.with_ymd_and_hms(2025, 8, 1, 0, 0, 0).unwrap(),
        );
        schedule_world_cup_qualifying(&mut game, 2026, &windows);
        let qualifying = game
            .competitions
            .iter()
            .find(|c| is_world_cup_qualifying(c))
            .unwrap();

        // No date holds an unrealistic pile of matches any more.
        assert!(
            max_fixtures_per_day(qualifying) <= 8,
            "qualifying should be spread under 8 matches/day, saw {}",
            max_fixtures_per_day(qualifying)
        );

        // Every match still falls inside a reserved international-window block.
        let block: std::collections::HashSet<String> = crate::national_team::
            international_window_span_dates(&windows)
            .into_iter()
            .collect();
        assert!(
            qualifying.fixtures.iter().all(|f| block.contains(&f.date)),
            "qualifying matches must stay inside the window span blocks"
        );
    }

    #[test]
    fn draw_keeps_at_most_one_confederation_per_group_except_uefa() {
        let mut game = empty_game();
        schedule_world_cup(&mut game, kickoff(2026), &FORMAT_48);
        let cup = game
            .competitions
            .iter()
            .find(|c| is_world_cup_competition(c))
            .unwrap();
        assert_eq!(cup.groups.len(), 12);
        for group in &cup.groups {
            let mut by_region: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();
            for team_id in &group.team_ids {
                let region =
                    nations::region_for_code(&nation_code_of_national_team(team_id)).to_string();
                *by_region.entry(region).or_default() += 1;
            }
            for (region, count) in by_region {
                let cap = if region == "europe" { 2 } else { 1 };
                assert!(
                    count <= cap,
                    "group {} holds {count} teams from {region}",
                    group.name
                );
            }
        }
    }

    #[test]
    fn world_cup_awards_a_future_host_with_news() {
        let mut game = empty_game();
        // 2038 is a cup year; its successor 2042 has no real-world host, so the
        // game awards one when the 2038 final is decided.
        schedule_world_cup(&mut game, kickoff(2038), &FORMAT_16);
        let mut rng = StdRng::seed_from_u64(3);
        for _ in 0..200 {
            let next = game
                .competitions
                .iter()
                .filter(|c| is_world_cup_competition(c))
                .flat_map(|c| c.fixtures.iter())
                .filter(|f| f.status == FixtureStatus::Scheduled)
                .map(|f| f.date.clone())
                .min();
            let Some(date) = next else { break };
            process_world_cup_fixtures_due(&mut game, &date, &mut rng);
        }
        assert!(
            game.world_history.world_cup_host(2042).is_some(),
            "a host should be awarded for the 2042 edition"
        );
        assert!(game.news.iter().any(|a| a.id == "world_cup_host_2042"));
        assert!(game.news.iter().any(|a| a.id == "world_cup_host_bid_2042"));
    }

    #[test]
    fn world_cup_summers_follow_the_real_calendar() {
        assert!(is_world_cup_summer(2022));
        assert!(is_world_cup_summer(2026));
        assert!(is_world_cup_summer(2030));
        assert!(!is_world_cup_summer(2024));
        assert!(!is_world_cup_summer(2025));
    }

    #[test]
    fn schedules_a_full_field_by_synthesising_missing_nations() {
        let mut game = empty_game();
        let players_before = game.players.len();

        schedule_world_cup(&mut game, kickoff(2026), &FORMAT_16);

        let cup = game
            .competitions
            .iter()
            .find(|c| is_world_cup_competition(c))
            .expect("a World Cup must be scheduled");
        assert_eq!(cup.participant_ids.len(), 16);
        assert_eq!(cup.groups.len(), 4);
        assert_eq!(cup.name, "World Cup 2026");
        // An empty world needed everything generated: 16 squads of free agents.
        assert_eq!(game.players.len(), players_before + 16 * 18);
        assert!(game.players.iter().all(|p| p.team_id.is_none()));
        // Every participant has a real national squad.
        for participant in &cup.participant_ids {
            let team = game
                .national_teams
                .iter()
                .find(|t| &t.id == participant)
                .expect("participant national team exists");
            assert!(team.squad_player_ids.len() >= 11);
        }
        // The whole world reads about the kickoff.
        let kickoff_news = game
            .news
            .iter()
            .find(|article| article.id == "world_cup_kickoff_2026")
            .expect("the tournament makes front-page news");
        assert_eq!(
            kickoff_news.headline_key.as_deref(),
            Some("be.news.worldCupKickoff.headline")
        );
        assert_eq!(kickoff_news.i18n_params.get("nations"), Some(&"16".to_string()));
    }

    #[test]
    fn the_host_auto_qualifies_into_the_field() {
        let mut game = empty_game();
        // Award an obscure, weak nation the 2030 hosting rights — one that would
        // never reach the finals on strength alone.
        game.world_history.record_world_cup_host(WorldCupHostRecord {
            year: 2030,
            nation_code: "AND".to_string(),
            nation_name: "Andorra".to_string(),
        });

        schedule_world_cup(&mut game, kickoff(2030), &FORMAT_16);

        let cup = game
            .competitions
            .iter()
            .find(|c| is_world_cup_competition(c))
            .expect("a World Cup must be scheduled");
        let host_qualified = cup.participant_ids.iter().any(|id| {
            game.national_teams
                .iter()
                .any(|team| &team.id == id && team.football_nation == "AND")
        });
        assert!(host_qualified, "the host auto-qualifies into the finals field");
        assert_eq!(cup.participant_ids.len(), 16, "the field size is preserved");
    }

    #[test]
    fn is_due_respects_the_calendar_and_never_doubles_up() {
        let mut game = empty_game();

        assert!(!schedule_world_cup_if_due(&mut game, kickoff(2025)));
        assert!(game.competitions.is_empty());

        assert!(schedule_world_cup_if_due(&mut game, kickoff(2026)));
        assert_eq!(
            game.competitions
                .iter()
                .filter(|c| is_world_cup_competition(c))
                .count(),
            1
        );

        assert!(
            !schedule_world_cup_if_due(&mut game, kickoff(2026)),
            "a running World Cup must not be scheduled twice"
        );
    }

    #[test]
    fn the_tournament_plays_to_a_champion_with_carry_back() {
        let mut game = empty_game();
        schedule_world_cup(&mut game, kickoff(2026), &FORMAT_16);
        let mut rng = StdRng::seed_from_u64(7);

        // Play every date in order until no fixtures remain scheduled.
        for _ in 0..200 {
            let next_date = game
                .competitions
                .iter()
                .filter(|c| is_world_cup_competition(c))
                .flat_map(|c| c.fixtures.iter())
                .filter(|f| f.status == FixtureStatus::Scheduled)
                .map(|f| f.date.clone())
                .min();
            let Some(date) = next_date else {
                break;
            };
            assert!(process_world_cup_fixtures_due(&mut game, &date, &mut rng) > 0);
        }

        let cup = game
            .competitions
            .iter()
            .find(|c| is_world_cup_competition(c))
            .unwrap();
        assert!(cup.knockout_rounds.iter().all(|round| round.completed));
        assert_eq!(cup.knockout_rounds.last().unwrap().fixture_ids.len(), 1);

        let champion = world_cup_champion(cup).expect("the final decides a champion");
        assert!(champion.starts_with("nt-"));
        let message = game
            .messages
            .iter()
            .find(|m| m.id == "world_cup_champion_2026")
            .expect("the champion is announced");
        assert_eq!(
            message.subject_key.as_deref(),
            Some("be.msg.worldCupChampion.subject")
        );
        assert!(message.i18n_params.contains_key("nation"));

        // Carry-back reached the squads: tournament players show fatigue.
        assert!(game.players.iter().any(|p| p.condition < 100));

        // The triumph is front-page news and enters the hall of fame.
        let news = game
            .news
            .iter()
            .find(|article| article.id == "world_cup_champion_news_2026")
            .expect("the champion makes front-page news");
        assert_eq!(
            news.headline_key.as_deref(),
            Some("be.news.worldCupChampion.headline")
        );
        let record = game
            .world_history
            .world_cup_champions
            .first()
            .expect("the champion is recorded for the hall of fame");
        assert_eq!(record.year, 2026);
        assert!(!record.nation_name.is_empty());
    }
}
