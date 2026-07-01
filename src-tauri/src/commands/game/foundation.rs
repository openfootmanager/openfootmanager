//! World-construction: national-team rosters, continental-cup seeding, and the
//! competition-foundation builders that lay out a generated world's league
//! pyramid and cup calendar. Faithful extraction from the original game.rs;
//! behaviour is locked by the foundation tests in `game::tests`.

use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use domain::league::{
    CompetitionFormat, CompetitionScope, CompetitionType, FixtureCompetition, League,
};
use domain::national_team::NationalTeam;
use ofm_core::game::Game;

use super::regions::{
    brazil_state_region, default_season_month_for_region, division_name, division_tier_name,
    division_tier_name_key, infer_region_id, infer_team_region_id, split_into_divisions,
};
use super::{preseason_league_year, preseason_season_start};

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
pub(crate) fn select_continental_entrants(
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
pub(crate) fn team_season_anchor(game: &Game, team_id: &str) -> Option<DateTime<Utc>> {
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

pub(crate) fn build_foundation_competitions(game: &Game) -> Vec<League> {
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

pub(crate) fn rebuild_competitions_for_management_date(
    game: &mut Game,
    management_date: DateTime<Utc>,
) {
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

pub(crate) fn ensure_multi_competition_foundations(game: &mut Game) {
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
pub(crate) fn ensure_international_windows(game: &mut Game) {
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
    if needs_fixtures && !qualifying_running {
        // A career starting the season before a World Cup opens with the
        // qualifying campaign; any other season opens with friendlies.
        if leads_into_world_cup {
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
    let reserved_dates = if leads_into_world_cup || qualifying_running {
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
