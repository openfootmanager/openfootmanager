//! Deciding what competitions a generated world has, and when each one starts.
//!
//! This is the widest single piece of the game commands and it is kept apart
//! for that reason: one function builds every built-in competition as an
//! explicit `CompetitionDefinition`, so built-ins and imported definitions
//! then flow through the same resolver.

use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};

use domain::league::{CompetitionFormat, CompetitionScope, CompetitionType, FixtureCompetition};
use ofm_core::game::Game;

use super::super::{
    brazil_state_region, default_season_month_for_region, division_name, division_tier_name,
    division_tier_name_key, infer_region_id, infer_team_region_id, select_continental_entrants,
    split_into_divisions, CONTINENTAL_CHAMPIONS_CUP_ID, CONTINENTAL_QUALIFYING_POSITIONS,
    TOP_DIVISION_SIZE,
};

/// Days before a club's first competitive match that a Season-Start career
/// begins, so the player gets a pre-season (with friendlies) instead of being
/// dropped onto matchday one. Covers the four-friendly pre-season window
/// (earliest friendly is ~28 days out).
pub(super) const PRESEASON_ANCHOR_BUFFER_DAYS: i64 = 30;

/// When a player picks SeasonStart, anchor the clock a pre-season buffer before
/// the team's first competitive fixture so they begin in pre-season. Returns
/// `None` only when the club has no league. Northern (August) leagues resolve to
/// a date after the July game anchor, so the caller's `actual_start < now` guard
/// leaves them on the default start.
pub(in crate::commands::game) fn team_season_anchor(
    game: &Game,
    team_id: &str,
) -> Option<DateTime<Utc>> {
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

/// Build the generated world's competitions as `CompetitionDefinition`s with
/// explicit participant lists, paired with their staggered start dates. Built-in
/// competitions then flow through the same `build_explicit_competition` core as
/// imported definitions (see [`build_foundation_competitions`]).
///
/// `game_start` is the game anchor (July 1 in normal years; June 1 in World Cup
/// years so the WC opens in June). Each competition's start date is derived from
/// its region's default season month via
/// [`ofm_core::generator::start_date_at_game_open`].
pub(super) fn build_foundation_competition_plan(
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
