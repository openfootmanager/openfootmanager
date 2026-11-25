//! Deciding what competitions a generated world has, and when each one starts.
//!
//! This is the widest single piece of the game commands and it is kept apart
//! for that reason: one function builds every built-in competition as an
//! explicit `CompetitionDefinition`, so built-ins and imported definitions
//! then flow through the same resolver.

use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};

use crate::game::Game;
use crate::generator::{CompetitionDefinition, FormatDef, ParticipantSpec};
use domain::competition_calendar::{CalendarDate, DivisionIdentity, SeasonCalendar, SeasonPhase};
use domain::league::{
    Berth, BerthRule, CompetitionFormat, CompetitionScope, CompetitionType, FixtureCompetition,
};
use std::collections::BTreeMap;

use super::*;

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
pub fn team_season_anchor(game: &Game, team_id: &str) -> Option<DateTime<Utc>> {
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

type PlannedCompetition = (CompetitionDefinition, DateTime<Utc>);

struct CountryPlanContext {
    country: String,
    country_slug: String,
    country_label: String,
    region_id: String,
    league_month: u8,
}

impl CountryPlanContext {
    fn new(country: &str) -> Self {
        let region_id = infer_region_id(country);
        Self {
            country: country.to_string(),
            country_slug: country.to_lowercase(),
            country_label: crate::nations::nation_display_name(country),
            league_month: if country == "BR" {
                1
            } else {
                default_season_month_for_region(&region_id)
            },
            region_id,
        }
    }
}

fn league_format(kind: CompetitionFormat) -> FormatDef {
    FormatDef {
        kind,
        legs: None,
        group_size: None,
        qualifiers_per_group: None,
        best_third_qualifiers: None,
    }
}

fn continental_berth(rule: BerthRule) -> Berth {
    Berth {
        target: CONTINENTAL_CHAMPIONS_CUP_ID.to_string(),
        rule,
        fallback_to: None,
    }
}

/// Build the generated world's competitions with explicit participant lists and staggered starts.
/// Built-in competitions then use the same resolver as imported definitions.
pub(super) fn build_foundation_competition_plan(
    game: &Game,
    game_start: DateTime<Utc>,
    division_size: usize,
) -> Vec<PlannedCompetition> {
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

    let mut planned = Vec::new();
    let mut priority = 0u32;
    for (country, mut team_ids) in teams_by_country {
        if team_ids.len() < 2 {
            continue;
        }
        team_ids.sort_by(|left, right| {
            reputation
                .get(right.as_str())
                .cmp(&reputation.get(left.as_str()))
                .then_with(|| left.cmp(right))
        });
        let context = CountryPlanContext::new(&country);
        planned.extend(plan_country_competitions(
            game,
            game_start,
            division_size,
            &context,
            &team_ids,
            &mut priority,
        ));
    }
    if let Some(continental) = plan_continental_competition(game, game_start, priority) {
        planned.push(continental);
    }
    planned
}

fn plan_country_competitions(
    game: &Game,
    game_start: DateTime<Utc>,
    division_size: usize,
    context: &CountryPlanContext,
    team_ids: &[String],
    priority: &mut u32,
) -> Vec<PlannedCompetition> {
    let divisions = split_into_divisions(team_ids, division_size);
    let mut planned = if crate::nations::is_split_season_country(&context.country) {
        plan_split_season_leagues(game_start, context, &divisions, priority)
    } else {
        plan_annual_leagues(game_start, context, &divisions, priority)
    };
    planned.push(plan_national_cup(game_start, context, team_ids, priority));
    if context.country == "BR" {
        planned.extend(plan_brazil_state_series(
            game, game_start, context, team_ids, priority,
        ));
    }
    planned
}

fn plan_split_season_leagues(
    game_start: DateTime<Utc>,
    context: &CountryPlanContext,
    divisions: &[Vec<String>],
    priority: &mut u32,
) -> Vec<PlannedCompetition> {
    let (opening_start, _) = crate::generator::start_date_at_game_open(game_start, 2, 1);
    let (closing_start, _) = crate::generator::start_date_at_game_open(game_start, 7, 1);
    let division_count = divisions.len();
    let mut planned = Vec::with_capacity(division_count * 2);

    for (tier, division_ids) in divisions.iter().enumerate() {
        let closing_berths = if tier == 0 {
            vec![continental_berth(BerthRule::PositionRange {
                from: 1,
                to: CONTINENTAL_QUALIFYING_POSITIONS,
            })]
        } else {
            Vec::new()
        };
        let opening = split_season_definition(
            context,
            division_ids,
            tier,
            division_count,
            2,
            Vec::new(),
            *priority,
        );
        planned.push((opening, opening_start));
        *priority += 1;
        let closing = split_season_definition(
            context,
            division_ids,
            tier,
            division_count,
            7,
            closing_berths,
            *priority,
        );
        planned.push((closing, closing_start));
        *priority += 1;
    }
    planned
}

fn split_season_definition(
    context: &CountryPlanContext,
    division_ids: &[String],
    tier: usize,
    division_count: usize,
    month: u8,
    berths: Vec<Berth>,
    priority: u32,
) -> CompetitionDefinition {
    let opening = month == 2;
    let phase = if opening {
        SeasonPhase::Opening
    } else {
        SeasonPhase::Closing
    };
    let phase_name = if opening { "Apertura" } else { "Clausura" };
    let phase_id = if opening { "apertura" } else { "clausura" };
    CompetitionDefinition {
        id: format!("{}-d{}-{phase_id}", context.country_slug, tier + 1),
        name: format!(
            "{} {} {phase_name}",
            context.country_label,
            division_tier_name(tier, division_count)
        ),
        r#type: CompetitionType::League,
        scope: CompetitionScope::Domestic,
        region_id: Some(context.region_id.clone()),
        country_id: Some(context.country.clone()),
        required_region_ids: vec![context.region_id.clone()],
        priority,
        format: league_format(CompetitionFormat::LeagueTable),
        participants: ParticipantSpec {
            explicit: Some(division_ids.to_vec()),
            selector: None,
        },
        berths,
        season_start_month: Some(month),
        season_start_day: Some(1),
        name_key: Some(division_tier_name_key(tier, division_count).to_string()),
        logo: None,
        calendar: Some(SeasonCalendar {
            division: Some(DivisionIdentity {
                family_id: format!("{}-league", context.country_slug),
                tier: (tier + 1) as u32,
                phase,
            }),
            window_end: Some(if opening {
                CalendarDate { month: 6, day: 30 }
            } else {
                CalendarDate { month: 1, day: 31 }
            }),
        }),
    }
}

fn plan_annual_leagues(
    game_start: DateTime<Utc>,
    context: &CountryPlanContext,
    divisions: &[Vec<String>],
    priority: &mut u32,
) -> Vec<PlannedCompetition> {
    let (league_start, _) = crate::generator::start_date_at_game_open(
        game_start,
        context.league_month,
        if context.country == "BR" { 28 } else { 1 },
    );
    let division_count = divisions.len();
    let mut planned = Vec::with_capacity(division_count);
    for (tier, division_ids) in divisions.iter().enumerate() {
        let actual_start = if context.country == "BR" && tier > 0 {
            crate::generator::start_date_at_game_open(game_start, 3, 21).0
        } else {
            league_start
        };
        let berths = if tier == 0 {
            vec![continental_berth(BerthRule::PositionRange {
                from: 1,
                to: CONTINENTAL_QUALIFYING_POSITIONS,
            })]
        } else {
            Vec::new()
        };
        let definition = annual_league_definition(
            context,
            division_ids,
            tier,
            division_count,
            actual_start,
            berths,
            *priority,
        );
        planned.push((definition, actual_start));
        *priority += 1;
    }
    planned
}

fn annual_league_definition(
    context: &CountryPlanContext,
    division_ids: &[String],
    tier: usize,
    division_count: usize,
    actual_start: DateTime<Utc>,
    berths: Vec<Berth>,
    priority: u32,
) -> CompetitionDefinition {
    CompetitionDefinition {
        id: format!("{}-d{}", context.country_slug, tier + 1),
        name: division_name(&context.country_label, tier, division_count),
        r#type: CompetitionType::League,
        scope: CompetitionScope::Domestic,
        region_id: Some(context.region_id.clone()),
        country_id: Some(context.country.clone()),
        required_region_ids: vec![context.region_id.clone()],
        priority,
        format: league_format(CompetitionFormat::LeagueTable),
        participants: ParticipantSpec {
            explicit: Some(division_ids.to_vec()),
            selector: None,
        },
        berths,
        season_start_month: Some(if context.country == "BR" && tier > 0 {
            actual_start.month() as u8
        } else {
            context.league_month
        }),
        season_start_day: Some(if context.country == "BR" {
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
        calendar: Some(SeasonCalendar {
            division: Some(DivisionIdentity {
                family_id: format!("{}-league", context.country_slug),
                tier: (tier + 1) as u32,
                phase: SeasonPhase::Annual,
            }),
            window_end: None,
        }),
    }
}

fn plan_national_cup(
    game_start: DateTime<Utc>,
    context: &CountryPlanContext,
    team_ids: &[String],
    priority: &mut u32,
) -> PlannedCompetition {
    let cup_month = if crate::nations::is_split_season_country(&context.country) {
        2
    } else {
        context.league_month
    };
    let (actual_start, _) = crate::generator::start_date_at_game_open(game_start, cup_month, 1);
    let cup_start = actual_start + Duration::days(35);
    let definition = CompetitionDefinition {
        id: format!("{}-cup", context.country_slug),
        name: format!("{} Cup", context.country_label),
        r#type: CompetitionType::Cup,
        scope: CompetitionScope::Domestic,
        region_id: Some(context.region_id.clone()),
        country_id: Some(context.country.clone()),
        required_region_ids: vec![context.region_id.clone()],
        priority: *priority,
        format: league_format(CompetitionFormat::Knockout),
        participants: ParticipantSpec {
            explicit: Some(team_ids.to_vec()),
            selector: None,
        },
        berths: vec![continental_berth(BerthRule::CupWinner)],
        season_start_month: Some(cup_start.month() as u8),
        season_start_day: Some(cup_start.day() as u8),
        name_key: Some("tournaments.competitions.nationalCup".to_string()),
        logo: None,
        calendar: None,
    };
    *priority += 1;
    (definition, cup_start)
}

fn plan_brazil_state_series(
    game: &Game,
    game_start: DateTime<Utc>,
    context: &CountryPlanContext,
    team_ids: &[String],
    priority: &mut u32,
) -> Vec<PlannedCompetition> {
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
    for team_id in team_ids {
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
    let state_start = crate::generator::start_date_at_game_open(game_start, 1, 11).0;
    let mut planned = Vec::new();
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
                region_id: Some(context.region_id.clone()),
                country_id: Some(context.country.clone()),
                required_region_ids: vec![context.region_id.clone()],
                priority: *priority,
                format: FormatDef {
                    kind: CompetitionFormat::GroupAndKnockout,
                    legs: Some(1),
                    group_size: Some(domain::league::DEFAULT_GROUP_SIZE),
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
                calendar: None,
            },
            state_start,
        ));
        *priority += 1;
    }
    planned
}

fn plan_continental_competition(
    game: &Game,
    game_start: DateTime<Utc>,
    priority: u32,
) -> Option<PlannedCompetition> {
    let team_ids = select_continental_entrants(&game.teams, 2, 16);
    if team_ids.len() < 4 {
        return None;
    }
    let mut feeder_regions: Vec<String> = game
        .teams
        .iter()
        .filter(|team| team_ids.contains(&team.id))
        .map(infer_team_region_id)
        .collect();
    feeder_regions.sort();
    feeder_regions.dedup();
    let format_kind = if team_ids.len() >= 8 {
        CompetitionFormat::GroupAndKnockout
    } else {
        CompetitionFormat::Knockout
    };
    let (start, _) = crate::generator::start_date_at_game_open(game_start, 10, 1);
    Some((
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
            format: league_format(format_kind),
            participants: ParticipantSpec {
                explicit: Some(team_ids),
                selector: None,
            },
            berths: Vec::new(),
            season_start_month: Some(10),
            season_start_day: Some(1),
            logo: None,
            calendar: None,
        },
        start,
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use crate::world::test_fixtures::{manager_for, nation_team};
    use crate::world::{TOP_DIVISION_SIZE, start_date_for_year};

    fn league_sizes(club_count: usize, division_size: usize) -> Vec<usize> {
        let teams = (0..club_count)
            .map(|index| nation_team(&format!("eng-{index:02}"), "ENG", 1000 - index as u32))
            .collect();
        let start = start_date_for_year(2032).expect("valid start year");
        let game = Game::new(
            GameClock::new(start),
            manager_for("eng-00"),
            teams,
            vec![],
            vec![],
            vec![],
        );
        build_foundation_competition_plan(&game, start, division_size)
            .into_iter()
            .filter(|(def, _)| {
                def.country_id.as_deref() == Some("ENG") && def.r#type == CompetitionType::League
            })
            .map(|(def, _)| {
                def.participants
                    .explicit
                    .expect("generated league has participants")
                    .len()
            })
            .collect()
    }

    #[test]
    fn three_clubs_at_size_two_stay_in_one_playable_league() {
        assert_eq!(league_sizes(3, 2), vec![3]);
    }

    #[test]
    fn default_size_keeps_the_original_forty_five_club_layout() {
        assert_eq!(league_sizes(45, TOP_DIVISION_SIZE), vec![20, 25]);
    }

    #[test]
    fn default_size_keeps_the_original_fifty_club_layout() {
        assert_eq!(league_sizes(50, TOP_DIVISION_SIZE), vec![20, 20, 10]);
    }

    /// Given 60 clubs that form three default-size divisions, when the plan is built, then every
    /// division has its own display name and localization key.
    #[test]
    fn a_three_tier_country_has_distinct_names_and_translation_keys() {
        let teams = (0..60)
            .map(|index| nation_team(&format!("eng-{index:02}"), "ENG", 1000 - index))
            .collect();
        let start = start_date_for_year(2032).expect("valid start year");
        let game = Game::new(
            GameClock::new(start),
            manager_for("eng-00"),
            teams,
            vec![],
            vec![],
            vec![],
        );

        let leagues: Vec<_> = build_foundation_competition_plan(&game, start, TOP_DIVISION_SIZE)
            .into_iter()
            .filter(|(definition, _)| {
                definition.country_id.as_deref() == Some("ENG")
                    && definition.r#type == CompetitionType::League
            })
            .map(|(definition, _)| definition)
            .collect();

        assert_eq!(leagues.len(), 3);
        assert_eq!(
            leagues
                .iter()
                .map(|league| league.name.as_str())
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            3,
            "each tier needs a distinct display name"
        );
        assert_eq!(
            leagues
                .iter()
                .map(|league| league.name_key.as_deref().expect("league name key"))
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            3,
            "each tier needs a distinct localization key"
        );
        assert_eq!(
            leagues
                .iter()
                .map(|league| league.name_key.as_deref().expect("league name key"))
                .collect::<Vec<_>>(),
            vec![
                "tournaments.competitions.firstDivision",
                "tournaments.competitions.secondDivision",
                "tournaments.competitions.tierDivision"
            ]
        );
        assert_eq!(
            leagues
                .iter()
                .map(|league| league.name.as_str())
                .collect::<Vec<_>>(),
            vec![
                "England First Division",
                "England Second Division",
                "England Division 3"
            ]
        );
    }

    /// Given 80 clubs that form four divisions, when the plan is built, then the fourth tier gets
    /// its ordinal display name and the generic tier localization key.
    #[test]
    fn a_deeper_tier_keeps_its_ordinal_name_and_generic_key() {
        let teams = (0..80)
            .map(|index| nation_team(&format!("eng-{index:02}"), "ENG", 1000 - index))
            .collect();
        let start = start_date_for_year(2032).expect("valid start year");
        let game = Game::new(
            GameClock::new(start),
            manager_for("eng-00"),
            teams,
            vec![],
            vec![],
            vec![],
        );

        let leagues: Vec<_> = build_foundation_competition_plan(&game, start, TOP_DIVISION_SIZE)
            .into_iter()
            .filter(|(definition, _)| {
                definition.country_id.as_deref() == Some("ENG")
                    && definition.r#type == CompetitionType::League
            })
            .map(|(definition, _)| definition)
            .collect();

        assert_eq!(leagues.len(), 4);
        assert_eq!(leagues[3].name, "England Division 4");
        assert_eq!(
            leagues[3].name_key.as_deref(),
            Some("tournaments.competitions.tierDivision")
        );
    }

    /// Given 18 clubs and a caller-selected size of six, when the plan is built, then it keeps
    /// all three requested tiers instead of widening them into two.
    #[test]
    fn custom_three_tier_pyramid_has_distinct_names_and_translation_keys() {
        let teams = (0..18)
            .map(|index| nation_team(&format!("eng-{index:02}"), "ENG", 1000 - index))
            .collect();
        let start = start_date_for_year(2032).expect("valid start year");
        let game = Game::new(
            GameClock::new(start),
            manager_for("eng-00"),
            teams,
            vec![],
            vec![],
            vec![],
        );

        let leagues: Vec<_> = build_foundation_competition_plan(&game, start, 6)
            .into_iter()
            .filter(|(definition, _)| {
                definition.country_id.as_deref() == Some("ENG")
                    && definition.r#type == CompetitionType::League
            })
            .map(|(definition, _)| definition)
            .collect();

        assert_eq!(leagues.len(), 3);
        assert_eq!(
            leagues
                .iter()
                .map(|league| league.name.as_str())
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            3,
            "each tier needs a distinct display name"
        );
        assert_eq!(
            leagues
                .iter()
                .map(|league| league.name_key.as_deref().expect("league name key"))
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            3,
            "each tier needs a distinct localization key"
        );
        assert_eq!(
            leagues
                .iter()
                .map(|league| league.name.as_str())
                .collect::<Vec<_>>(),
            vec![
                "England First Division",
                "England Second Division",
                "England Division 3"
            ]
        );
    }

    /// Given a split-season country with two tiers, when both calendar halves are planned, then
    /// each half keeps its phase in the stored name and uses the localized key for its tier.
    #[test]
    fn generated_calendar_halves_declare_shared_family_and_distinct_windows() {
        let teams = (0..40)
            .map(|i| nation_team(&format!("ar-{i}"), "AR", 1000 - i))
            .collect();
        let start = start_date_for_year(2033).unwrap();
        let game = Game::new(
            GameClock::new(start),
            manager_for("ar-0"),
            teams,
            vec![],
            vec![],
            vec![],
        );
        let plan = build_foundation_competition_plan(&game, start, 20);
        assert_eq!(
            plan.iter()
                .filter(|(d, _)| d.country_id.as_deref() == Some("AR")
                    && d.r#type == CompetitionType::League)
                .count(),
            4
        );
        let mut phase_names_by_tier =
            std::collections::BTreeMap::<usize, std::collections::BTreeMap<&str, &str>>::new();
        for (definition, _) in plan.iter().filter(|(d, _)| {
            d.country_id.as_deref() == Some("AR") && d.r#type == CompetitionType::League
        }) {
            let value = serde_json::to_value(definition).unwrap();
            let phase = if definition.id.ends_with("-apertura") {
                "opening"
            } else {
                "closing"
            };
            let tier = if definition.id.contains("d1") { 1 } else { 2 };
            let expected_key = match tier {
                1 => "tournaments.competitions.firstDivision",
                2 => "tournaments.competitions.secondDivision",
                _ => unreachable!("the split-season fixture has two tiers"),
            };
            assert_eq!(definition.name_key.as_deref(), Some(expected_key));
            assert!(
                phase_names_by_tier
                    .entry(tier)
                    .or_default()
                    .insert(phase, &definition.name)
                    .is_none()
            );
            assert_eq!(value["calendar"]["division"]["familyId"], "ar-league");
            assert_eq!(value["calendar"]["division"]["phase"], phase);
            assert_eq!(value["calendar"]["division"]["tier"], tier);
            assert_eq!(
                value["calendar"]["windowEnd"],
                if phase == "opening" {
                    serde_json::json!({"month":6,"day":30})
                } else {
                    serde_json::json!({"month":1,"day":31})
                }
            );
            assert_eq!(definition.season_start_day, Some(1));
            assert_eq!(
                definition.season_start_month,
                Some(if phase == "opening" { 2 } else { 7 })
            );
        }
        assert_eq!(phase_names_by_tier.len(), 2);
        for (tier, phase_names) in phase_names_by_tier {
            let opening = phase_names.get("opening").expect("opening half");
            let closing = phase_names.get("closing").expect("closing half");
            assert_ne!(
                opening, closing,
                "tier {tier} must keep its phase in the name"
            );
        }
    }
    #[test]
    fn generated_annual_calendar_declares_tiers_without_split_phase() {
        let teams = (0..40)
            .map(|i| nation_team(&format!("eng-{i}"), "ENG", 1000 - i))
            .collect();
        let start = start_date_for_year(2033).unwrap();
        let game = Game::new(
            GameClock::new(start),
            manager_for("eng-0"),
            teams,
            vec![],
            vec![],
            vec![],
        );
        let plan = build_foundation_competition_plan(&game, start, 20);
        let leagues: Vec<_> = plan
            .iter()
            .filter(|(definition, _)| {
                definition.country_id.as_deref() == Some("ENG")
                    && definition.r#type == CompetitionType::League
            })
            .collect();
        assert_eq!(leagues.len(), 2);
        for (definition, _) in leagues {
            let value = serde_json::to_value(definition).unwrap();
            assert_eq!(value["calendar"]["division"]["familyId"], "eng-league");
            assert_eq!(
                value["calendar"]["division"]["tier"],
                if definition.id.ends_with("d1") { 1 } else { 2 }
            );
            assert_eq!(value["calendar"]["division"]["phase"], "annual");
            assert!(value["calendar"]["windowEnd"].is_null());
            assert_eq!(
                (definition.season_start_month, definition.season_start_day),
                (Some(8), Some(1))
            );
        }
    }
}
