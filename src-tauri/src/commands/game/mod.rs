use log::info;
use std::sync::Arc;
use tauri::{Manager as TauriManager, State};

use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};

use db::{save_index::SaveEntry, save_manager::SaveManager};
use domain::league::{
    CompetitionFormat, CompetitionScope, CompetitionType, FixtureCompetition, League,
};
use domain::manager::Manager;
use domain::stats::StatsState;
use ofm_core::game::Game;
use ofm_core::state::StateManager;

use crate::commands::util::persist_active_game;
use crate::SaveManagerState;

mod helpers;
mod startup;
mod validation;
mod world_build;
mod world_load;

// Private globs, so a submodule's items need only `pub(super)` to be reachable
// from here and from the tests below. The names the rest of the crate calls are
// re-exported explicitly, and they are the only promise this module makes.
use helpers::*;
pub(crate) use helpers::{default_save_name, first_package_error_message};
use startup::*;
pub(crate) use startup::{start_phase_for_game, StartPhase};
use world_build::*;
use world_load::*;
// Public, unlike the others: these are `#[tauri::command]`s and the types in
// their signatures. `commands/mod.rs` re-exports them again for the invoke
// handler in `lib.rs`, so the path must stay `crate::commands::<name>`.
pub use validation::*;

/// Build the generated world's competitions as `CompetitionDefinition`s with
/// explicit participant lists, paired with their staggered start dates. Built-in
/// competitions then flow through the same `build_explicit_competition` core as
/// imported definitions (see [`build_foundation_competitions`]).
///
/// `game_start` is the game anchor (July 1 in normal years; June 1 in World Cup
/// years so the WC opens in June). Each competition's start date is derived from
/// its region's default season month via
/// [`ofm_core::generator::start_date_at_game_open`].
/// Days before a club's first competitive match that a Season-Start career
/// begins, so the player gets a pre-season (with friendlies) instead of being
/// dropped onto matchday one. Covers the four-friendly pre-season window
/// (earliest friendly is ~28 days out).
const PRESEASON_ANCHOR_BUFFER_DAYS: i64 = 30;

/// When a player picks SeasonStart, anchor the clock a pre-season buffer before
/// the team's first competitive fixture so they begin in pre-season. Returns
/// `None` only when the club has no league. Northern (August) leagues resolve to
/// a date after the July game anchor, so the caller's `actual_start < now` guard
/// leaves them on the default start.
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
    // Anchor a pre-season buffer before the club's first competitive fixture so
    // every calendar (South America in March, Asia in February, Oceania in
    // October, …) starts the player in pre-season — with the generated
    // friendlies still in the future and playable — rather than dropping them
    // onto matchday one. Northern (August) leagues land their buffered date
    // after the July game anchor, so the caller's guard leaves them untouched.
    competition
        .fixtures
        .iter()
        .filter(|fixture| fixture.competition != FixtureCompetition::Friendly)
        .filter(|fixture| fixture.home_team_id == team_id || fixture.away_team_id == team_id)
        .filter_map(|fixture| chrono::NaiveDate::parse_from_str(&fixture.date, "%Y-%m-%d").ok())
        .min()
        .and_then(|date| date.and_hms_opt(0, 0, 0))
        .map(|date| {
            DateTime::<Utc>::from_naive_utc_and_offset(date, Utc)
                - Duration::days(PRESEASON_ANCHOR_BUFFER_DAYS)
        })
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
        // International tournaments (the World Cup and its qualifying) own a fixed
        // calendar tied to the cup year, not the club's hemisphere. Re-anchoring
        // them against a club's season start would corrupt their dates (and
        // orphan a future-dated kickoff), so leave them untouched.
        if ofm_core::world_cup::is_world_cup_competition(competition)
            || ofm_core::world_cup::is_world_cup_qualifying(competition)
        {
            continue;
        }
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
    let leads_into_world_cup =
        ofm_core::world_cup::season_leads_into_world_cup(preseason_season_start(&game.clock));
    let starts_qualifying = ofm_core::world_cup::season_starts_world_cup_qualifying(
        preseason_season_start(&game.clock),
    );
    if needs_fixtures && !qualifying_running {
        // A career starting two seasons before a World Cup opens with the full
        // home-and-away qualifying campaign; one starting the season before
        // squeezes in a compressed campaign; any other season opens with
        // friendlies.
        if starts_qualifying {
            ofm_core::world_cup::schedule_world_cup_qualifying(
                game,
                preseason_season_start(&game.clock).year() + 2,
                &window_dates,
            );
        } else if leads_into_world_cup {
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

    // Qualifying spreads each window's matches across a multi-day block, so club
    // fixtures must keep clear of the whole span rather than just the openers.
    let reserved_dates = if leads_into_world_cup || starts_qualifying || qualifying_running {
        ofm_core::national_team::international_window_span_dates(&window_dates)
    } else {
        window_dates.clone()
    };
    for competition in &mut game.competitions {
        // The World Cup and its qualifying own the reserved window — they are the
        // reason it is reserved — so shifting them off it would move the fixtures
        // we just scheduled there. Only club competitions step aside.
        if ofm_core::world_cup::is_world_cup_competition(competition)
            || ofm_core::world_cup::is_world_cup_qualifying(competition)
        {
            continue;
        }
        ofm_core::schedule::shift_fixtures_off_reserved_dates(competition, &reserved_dates);
    }
    ofm_core::schedule::append_south_american_preseason_friendlies(
        &mut game.competitions,
        &reserved_dates,
    );
    ofm_core::schedule::append_other_preseason_friendlies(&mut game.competitions, &reserved_dates);
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
    let mut league = ofm_core::schedule::generate_league(
        DEFAULT_LEAGUE_NAME,
        preseason_league_year(&game.clock),
        &team_ids,
        season_start,
    );
    league.name_key = Some(DEFAULT_LEAGUE_NAME_KEY.to_string());
    let friendlies = ofm_core::schedule::generate_preseason_friendlies(&team_ids, season_start, 4);
    ofm_core::schedule::append_fixtures(&mut league, friendlies);
    game.league = Some(league);
    ofm_core::season_context::refresh_game_context(game);

    let date_str = game.clock.current_date.to_rfc3339();
    let welcome_msg = ofm_core::messages::welcome_message(&team_name, team_id, &date_str);
    game.messages.push(welcome_msg);

    // Both params are resolved frontend-side: the league name is a translation
    // key, and the ISO date is formatted in the player's locale.
    let season_msg = ofm_core::messages::season_schedule_message(
        DEFAULT_LEAGUE_NAME_KEY,
        &season_start.format(ISO_DATE_FORMAT).to_string(),
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
    let team_ids: Vec<String> = game.teams.iter().map(|t| t.id.clone()).collect();
    let mut league = ofm_core::schedule::generate_league(
        DEFAULT_LEAGUE_NAME,
        preseason_league_year(&game.clock),
        &team_ids,
        season_start,
    );
    league.name_key = Some(DEFAULT_LEAGUE_NAME_KEY.to_string());
    game.league = Some(league);
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
    let (mut world, package_lockfile) =
        if let Some(ids) = package_ids.as_deref().filter(|ids| !ids.is_empty()) {
            let packages_dir = app_handle
                .path()
                .app_data_dir()
                .map_err(|e| e.to_string())?
                .join("packages");
            // The career start year wins over the package's declared `baseYear`:
            // squads must be aged against the clock the player actually gets.
            let opening_year = u32::try_from(startup_options.start_year).ok();
            let asset_root = crate::commands::world::package_assets_dir(&app_handle).ok();
            load_world_data_from_package_ids(
                &packages_dir,
                ids,
                opening_year,
                asset_root.as_deref(),
                &definition_sources(&app_handle),
            )?
        } else {
            (
                load_world_data(world_source.as_deref(), &definition_sources(&app_handle))?,
                vec![],
            )
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
        // Staff generated to fill an imported world are aged against the year the
        // career actually opens in, so a historical world does not get a backroom
        // team born decades after the season it models.
        let opening_year = u32::try_from(clock.start_date.year())
            .unwrap_or_else(|_| ofm_core::generator::default_opening_year());
        ofm_core::generator::normalize_imported_world_for_career_start(&mut world, opening_year);
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
    let mut sm = map_save_manager_lock_error(sm_state.0.lock())?;
    persist_active_game(&state, &mut sm)
}

/// Save the current game and clear the active session so the player returns to the main menu.
#[tauri::command]
pub async fn exit_to_menu(
    state: State<'_, Arc<StateManager>>,
    sm_state: State<'_, Arc<SaveManagerState>>,
) -> Result<(), String> {
    info!("[cmd] exit_to_menu");
    if state.get_save_id().is_some_and(|id| !id.is_empty()) {
        let mut sm = map_save_manager_lock_error(sm_state.0.lock())?;
        persist_active_game(&state, &mut sm)?;
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
    let bootstrap_opening_year = normalize_startup_options(None)
        .ok()
        .and_then(|options| game_clock_for_world(&options, &world.metadata).ok())
        .and_then(|clock| u32::try_from(clock.start_date.year()).ok())
        .unwrap_or_else(ofm_core::generator::default_opening_year);
    ofm_core::generator::normalize_imported_world_for_career_start(
        &mut world,
        bootstrap_opening_year,
    );

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

/// World-building fixtures shared by the tests of several clusters in this
/// module. Kept apart so each peel of `game.rs` can take its own tests with it
/// without duplicating a fixture or reaching into a sibling.
#[cfg(test)]
mod testkit;

#[cfg(test)]
mod tests {
    use super::testkit::*;
    use super::{
        bootstrap_team_selection, build_foundation_competitions, build_game_from_world_data,
        create_new_save, ensure_international_windows, ensure_multi_competition_foundations,
        game_clock_for_world, rebuild_competitions_for_management_date, resolve_simulation_scope,
        start_date_for_year, StartPhase, StartupOptions, DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
    };
    use chrono::{TimeZone, Utc};
    use db::save_manager::SaveManager;
    use domain::{
        league::{
            CompetitionFormat, CompetitionScope, CompetitionType, FixtureCompetition, League,
        },
        manager::Manager,
        news::NewsCategory,
    };
    use ofm_core::{clock::GameClock, game::Game};
    use std::collections::{BTreeMap, BTreeSet};

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
    fn rebuilding_competitions_leaves_the_world_cup_schedule_intact() {
        use ofm_core::world_cup::is_world_cup_competition;
        // A 2026 World Cup summer career, staged at the June anchor.
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 7, 1, 12, 0, 0).unwrap());
        let mut game = Game::new(
            clock,
            manager_for("team-1"),
            vec![nation_team("team-1", "ES", 500)],
            vec![],
            vec![],
            vec![],
        );
        game.active_competition_ids = vec!["dummy".to_string()];
        ensure_international_windows(&mut game);

        // Capture the World Cup's id and fixture dates before any re-anchoring.
        let (wc_id, before): (String, Vec<String>) = {
            let world_cup = game
                .competitions
                .iter()
                .find(|competition| is_world_cup_competition(competition))
                .expect("the World Cup is staged");
            (
                world_cup.id.clone(),
                world_cup
                    .fixtures
                    .iter()
                    .map(|fixture| fixture.date.clone())
                    .collect(),
            )
        };
        assert!(
            !before.is_empty(),
            "the staged World Cup has fixtures to protect"
        );

        // Re-anchor competitions to a February management date — the Argentina
        // mid-season scenario that previously orphaned the cup's June schedule.
        let management_date = Utc.with_ymd_and_hms(2026, 2, 1, 12, 0, 0).unwrap();
        rebuild_competitions_for_management_date(&mut game, management_date);

        let world_cup = game
            .competitions
            .iter()
            .find(|competition| competition.id == wc_id)
            .expect("the World Cup survives the re-anchor");
        let after: Vec<String> = world_cup
            .fixtures
            .iter()
            .map(|fixture| fixture.date.clone())
            .collect();
        assert_eq!(
            before, after,
            "the World Cup keeps its June schedule through a February re-anchor"
        );
        assert!(
            after
                .iter()
                .all(|date| date.starts_with("2026-06") || date.starts_with("2026-07")),
            "World Cup fixtures stay in the cup window, not pulled back to February"
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

    /// The regression test for #555, at the shape the game actually ships.
    ///
    /// Every earlier promotion test built its divisions by hand, and that is
    /// exactly how the bug survived: hand-built divisions carry no berths, so
    /// none of them ever entered the code path that a real world takes. This
    /// one runs `ensure_multi_competition_foundations` — the same competition
    /// plan `create_game` builds — through three consecutive rollovers and
    /// checks the invariant that matters: every club is registered with exactly
    /// one of its country's league tables, the divisions keep their sizes, and
    /// each top flight actually turns over.
    ///
    /// The nations are chosen to cover the shapes that broke: two-tier pyramids
    /// (ENG, ES, BR), a split-season country whose two halves share one roster
    /// (AR), a single-tier country (PT), Brazil's regional state cups, and a
    /// continental cup every first division berths into — the shared target
    /// that detached every ladder in the world.
    fn production_world(start_year: i32, user_team: &str) -> Game {
        let mut teams = Vec::new();
        for (nation, count) in [("ENG", 40), ("ES", 40), ("BR", 40), ("AR", 20), ("PT", 20)] {
            for index in 0..count {
                teams.push(nation_team(
                    &format!("{}-{index:02}", nation.to_lowercase()),
                    nation,
                    1000 - index as u32,
                ));
            }
        }
        let clock = GameClock::new(start_date_for_year(start_year).expect("valid start year"));
        let mut game = Game::new(clock, manager_for(user_team), teams, vec![], vec![], vec![]);
        ensure_multi_competition_foundations(&mut game);
        game.sync_legacy_league();
        game
    }

    /// (nation, club id) for every club in the world.
    fn team_list(game: &Game) -> Vec<(String, String)> {
        game.teams
            .iter()
            .map(|team| (team.football_nation.clone(), team.id.clone()))
            .collect()
    }

    fn roster(competition: &League) -> BTreeSet<String> {
        competition.participant_ids.iter().cloned().collect()
    }

    /// Every club in exactly one of its country's league tables, every division
    /// still its authored size, and a split-season country's two halves still
    /// running over the same clubs.
    fn assert_one_league_per_club(game: &Game, sizes: &BTreeMap<String, usize>, label: &str) {
        let mut clubs_by_nation: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for team in &team_list(game) {
            clubs_by_nation
                .entry(team.0.clone())
                .or_default()
                .insert(team.1.clone());
        }
        let mut by_country: BTreeMap<String, Vec<&League>> = BTreeMap::new();
        for competition in &game.competitions {
            if competition.rules.format != CompetitionFormat::LeagueTable
                || competition.kind != CompetitionType::League
                || competition.scope != CompetitionScope::Domestic
            {
                continue;
            }
            if let Some(country) = &competition.country_id {
                by_country
                    .entry(country.clone())
                    .or_default()
                    .push(competition);
            }
        }
        // Iterating only the countries that still have competitions means a
        // country whose leagues vanished is never checked — the test passed
        // with both Argentine leagues deleted after every rollover. Compare the
        // key sets first, so a disappearing country is itself a failure.
        let represented: BTreeSet<&String> = by_country.keys().collect();
        let expected: BTreeSet<&String> = clubs_by_nation.keys().collect();
        assert_eq!(
            represented, expected,
            "{label}: every nation must still have at least one league table"
        );

        for (country, leagues) in by_country {
            let mut union: BTreeSet<String> = BTreeSet::new();
            let split_season = ofm_core::nations::is_split_season_country(&country);
            for league in &leagues {
                let clubs = roster(league);
                assert_eq!(
                    clubs.len(),
                    league.participant_ids.len(),
                    "{label}: {} lists a club twice: {:?}",
                    league.id,
                    league.participant_ids
                );
                assert_eq!(
                    league.participant_ids.len(),
                    sizes[&league.id],
                    "{label}: {} changed size",
                    league.id
                );
                if !split_season {
                    assert!(
                        union.is_disjoint(&clubs),
                        "{label}: {} shares clubs with another {country} league",
                        league.id
                    );
                }
                union.extend(clubs);
            }
            if split_season {
                // Both halves are the same division played twice, so they must
                // hold identical rosters rather than disjoint ones.
                let first = roster(leagues[0]);
                for league in &leagues[1..] {
                    assert_eq!(
                        roster(league),
                        first,
                        "{label}: {country} halves drifted apart at {}",
                        league.id
                    );
                }
            }
            assert_eq!(
                union, clubs_by_nation[&country],
                "{label}: every {country} club must be in exactly one league table"
            );
        }
    }

    #[test]
    fn a_generated_world_promotes_and_relegates_for_three_seasons_running() {
        let mut game = production_world(2035, "eng-00");
        let (regions, competitions) =
            resolve_simulation_scope(&game, "eng-00", None, None).expect("a valid scope");
        game.active_region_ids = regions;
        game.active_competition_ids = competitions;
        let sizes: BTreeMap<String, usize> = game
            .competitions
            .iter()
            .map(|competition| (competition.id.clone(), competition.participant_ids.len()))
            .collect();
        let mut previous: BTreeMap<String, BTreeSet<String>> = game
            .competitions
            .iter()
            .filter(|competition| ["eng-d1", "es-d1"].contains(&competition.id.as_str()))
            .map(|competition| (competition.id.clone(), roster(competition)))
            .collect();

        for rollover in 1..=3 {
            let far_future = Utc.with_ymd_and_hms(2100, 1, 1, 0, 0, 0).unwrap();
            let players = game.players.clone();
            for competition in game.competitions.iter_mut() {
                ofm_core::catchup::simulate_past_fixtures(competition, &players, far_future);
            }
            let last_match_day = game
                .competitions
                .iter()
                .find(|competition| {
                    competition.rules.format == CompetitionFormat::LeagueTable
                        && competition.participant_ids.iter().any(|id| id == "eng-00")
                })
                .expect("the user's division")
                .fixtures
                .iter()
                .filter(|fixture| fixture.counts_for_league_standings())
                .filter_map(|fixture| {
                    chrono::NaiveDate::parse_from_str(&fixture.date, "%Y-%m-%d").ok()
                })
                .max()
                .expect("a played season");
            game.clock.current_date = Utc.from_utc_datetime(
                &last_match_day
                    .and_hms_opt(0, 0, 0)
                    .expect("midnight is a valid time"),
            ) + chrono::Duration::days(1);

            ofm_core::end_of_season::process_end_of_season(&mut game);

            let label = format!("rollover {rollover}");
            assert_one_league_per_club(&game, &sizes, &label);

            // Every domestic division that existed at kickoff must still exist.
            // Size and disjointness assertions say nothing about a league that
            // is simply gone.
            for id in sizes.keys() {
                assert!(
                    game.competitions.iter().any(|c| c.id == *id),
                    "{label}: competition {id} disappeared"
                );
            }

            // The user's own division has to stay in simulation scope, or the
            // day loop cannot see their fixtures and runs their match against
            // whichever competition sorts first.
            let user_division = game
                .competitions
                .iter()
                .find(|c| {
                    c.rules.format == CompetitionFormat::LeagueTable
                        && c.participant_ids.iter().any(|id| id == "eng-00")
                })
                .expect("the user is registered somewhere");
            assert!(
                game.active_competition_ids.contains(&user_division.id),
                "{label}: the user's division {} is out of scope",
                user_division.id
            );

            // The point of #555: a top flight that never changes hands is the
            // bug, not a stable league.
            for id in ["eng-d1", "es-d1"] {
                let now = roster(
                    game.competitions
                        .iter()
                        .find(|competition| competition.id == id)
                        .expect(id),
                );
                assert_ne!(
                    previous.get(id),
                    Some(&now),
                    "{label}: {id} promoted and relegated nobody"
                );
                previous.insert(id.to_string(), now);
            }
        }
    }

    /// Brazil is the one shipped nation whose divisions run on different
    /// calendars — Série A opens 28 January, Série B on 21 March — so at the
    /// first division's last matchday the second still has eight rounds to
    /// play. The rollover used to fire anyway, the ladder skipped the whole
    /// country for want of a finished lower tier, and Série B was never
    /// regenerated: promotion happened every *other* season, and Série B
    /// skipped a calendar year each time it did.
    ///
    /// A Brazilian career is the whole point of this test. An English one
    /// cannot see the bug, because both English divisions finish together.
    #[test]
    fn a_brazilian_career_promotes_and_relegates_every_season() {
        let mut game = production_world(2035, "br-00");
        let (regions, competitions) =
            resolve_simulation_scope(&game, "br-00", None, None).expect("a valid scope");
        game.active_region_ids = regions;
        game.active_competition_ids = competitions;

        for rollover in 1..=3 {
            let user_last = game
                .competitions
                .iter()
                .find(|competition| {
                    competition.rules.format == CompetitionFormat::LeagueTable
                        && competition.participant_ids.iter().any(|id| id == "br-00")
                })
                .expect("the user's division")
                .fixtures
                .iter()
                .filter(|fixture| fixture.counts_for_league_standings())
                .filter_map(|fixture| {
                    chrono::NaiveDate::parse_from_str(&fixture.date, "%Y-%m-%d").ok()
                })
                .max()
                .expect("a scheduled season");

            // Advance in steps the way a player does, until the game itself
            // says the season is over. If the gate let the rollover through
            // while Série B was mid-season, this would stop at the first step.
            let mut cutoff = Utc.from_utc_datetime(
                &user_last
                    .and_hms_opt(0, 0, 0)
                    .expect("midnight is a valid time"),
            ) + chrono::Duration::days(1);
            for _ in 0..40 {
                let players = game.players.clone();
                for competition in game.competitions.iter_mut() {
                    ofm_core::catchup::simulate_past_fixtures(competition, &players, cutoff);
                }
                game.clock.current_date = cutoff;
                if ofm_core::end_of_season::is_season_complete(&game) {
                    break;
                }
                cutoff += chrono::Duration::days(14);
            }
            assert!(
                ofm_core::end_of_season::is_season_complete(&game),
                "rollover {rollover}: the Brazilian season never completed"
            );

            let roster_before = by_competition_id(&game, "br-d1").participant_ids.clone();
            let second_tier_season_before = by_competition_id(&game, "br-d2").season;

            ofm_core::end_of_season::process_end_of_season(&mut game);

            assert_ne!(
                by_competition_id(&game, "br-d1").participant_ids,
                roster_before,
                "rollover {rollover}: Série A promoted and relegated nobody"
            );
            assert_eq!(
                by_competition_id(&game, "br-d2").season,
                second_tier_season_before + 1,
                "rollover {rollover}: Série B must advance exactly one season, not skip a year"
            );
        }
    }

    fn by_competition_id<'a>(game: &'a Game, id: &str) -> &'a League {
        game.competitions
            .iter()
            .find(|competition| competition.id == id)
            .unwrap_or_else(|| panic!("{id} should exist"))
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

    #[test]
    #[ignore = "perf harness; run: cargo test -p openfootmanager perf_baseline -- --ignored --nocapture"]
    fn perf_baseline() {
        use std::time::Instant;

        let t = Instant::now();
        let world = ofm_core::generator::generate_world_data(
            &ofm_core::generator::DefinitionSources::embedded_only(),
        );
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
    fn season_start_anchor_buffers_preseason_for_non_august_calendars() {
        // An Asian (February) club: the Season-Start clock should land a
        // pre-season buffer before the first competitive fixture — not on
        // matchday one — so the player gets a pre-season with playable
        // friendlies. (Regression for Asian/Oceanian leagues, which used to
        // re-anchor straight onto their opener.)
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap());
        let manager = Manager::new(
            "mgr".to_string(),
            "Alex".to_string(),
            "Boss".to_string(),
            "1980-01-01".to_string(),
            "Japan".to_string(),
        );
        let mut team = domain::team::Team::new(
            "jp-1".to_string(),
            "Tokyo FC".to_string(),
            "TFC".to_string(),
            "JP".to_string(),
            "Tokyo".to_string(),
            "Stadium".to_string(),
            10_000,
        );
        team.football_nation = "JP".to_string();

        let mut game = Game::new(clock, manager, vec![team], vec![], vec![], vec![]);
        let mut league = League::new(
            "jp-league".to_string(),
            "JP League".to_string(),
            2026,
            &["jp-1".to_string()],
        );
        league.region_id = Some("asia".to_string());
        league.fixtures.push(domain::league::Fixture {
            id: "f1".to_string(),
            competition_id: "jp-league".to_string(),
            matchday: 1,
            date: "2026-02-07".to_string(),
            home_team_id: "jp-1".to_string(),
            away_team_id: "jp-2".to_string(),
            competition: FixtureCompetition::League,
            status: domain::league::FixtureStatus::Scheduled,
            result: None,
        });
        game.competitions = vec![league];

        let anchor =
            super::team_season_anchor(&game, "jp-1").expect("a non-August league re-anchors");
        // The first competitive fixture (Feb 7) minus the 30-day pre-season buffer.
        assert_eq!(anchor, Utc.with_ymd_and_hms(2026, 1, 8, 0, 0, 0).unwrap());
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
        ofm_core::generator::normalize_imported_world_for_career_start(
            &mut world,
            startup_options.start_year as u32,
        );
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
        ofm_core::generator::normalize_imported_world_for_career_start(
            &mut world,
            startup_options.start_year as u32,
        );
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
        ofm_core::generator::normalize_imported_world_for_career_start(
            &mut world,
            startup_options.start_year as u32,
        );
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
        ofm_core::generator::normalize_imported_world_for_career_start(
            &mut world,
            startup_options.start_year as u32,
        );
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
}
