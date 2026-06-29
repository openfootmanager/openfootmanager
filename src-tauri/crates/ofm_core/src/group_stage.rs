//! Group-and-knockout competitions: a Champions-League-style group stage whose
//! top finishers seed the knockout bracket.

use chrono::{DateTime, Duration, Utc};
use domain::league::{
    CompetitionFormat, CompetitionRules, CompetitionScope, CompetitionType, FixtureCompetition,
    FixtureStatus, GroupState, League, StandingEntry,
};
use uuid::Uuid;

/// Clubs per group; the last groups may run one short when the entrant count
/// doesn't divide evenly.
const GROUP_SIZE: usize = 4;

/// Shape of a group stage at creation time.
#[derive(Debug, Clone)]
pub struct GroupStageConfig {
    /// Round-robin legs within each group (2 = home and away).
    pub legs: u8,
    /// Days between group matchdays.
    pub matchday_gap_days: i64,
    /// Teams advancing from each group.
    pub qualifiers_per_group: u32,
    /// Additional best next-placed finishers across all groups that advance
    /// (the 2026 World Cup's "best thirds").
    pub best_third_qualifiers: u32,
    /// Days between knockout rounds once the bracket starts.
    pub knockout_round_gap_days: u32,
    /// When `Some(n)`, spread group-stage fixtures so at most `n` matches
    /// happen on any single calendar day. `None` keeps the default behaviour
    /// (all fixtures in a matchday share the same date).
    pub max_concurrent_matches_per_day: Option<usize>,
    /// Maximum fixtures scheduled on the same day within a single knockout
    /// round. Mirrors `CompetitionRules::knockout_matches_per_day`.
    pub knockout_matches_per_day: u32,
}

impl Default for GroupStageConfig {
    fn default() -> Self {
        Self {
            legs: 2,
            matchday_gap_days: 7,
            qualifiers_per_group: 2,
            best_third_qualifiers: 0,
            knockout_round_gap_days: 14,
            max_concurrent_matches_per_day: None,
            knockout_matches_per_day: 1,
        }
    }
}

fn fixture_competition_for(kind: &CompetitionType) -> FixtureCompetition {
    match kind {
        CompetitionType::ContinentalClub => FixtureCompetition::ContinentalClub,
        CompetitionType::InternationalClub => FixtureCompetition::InternationalClub,
        CompetitionType::InternationalNation => FixtureCompetition::InternationalNation,
        CompetitionType::FriendlyCup => FixtureCompetition::FriendlyCup,
        _ => FixtureCompetition::Cup,
    }
}

fn group_label(index: usize) -> String {
    char::from(b'A' + (index % 26) as u8).to_string()
}

/// Snake-seed `team_ids` (strongest first) into groups of ~[`GROUP_SIZE`], so
/// each group gets a comparable spread of strength.
fn seed_groups(competition_id: &str, team_ids: &[String]) -> Vec<GroupState> {
    let group_count = team_ids.len().div_ceil(GROUP_SIZE).max(1);
    let mut groups: Vec<GroupState> = (0..group_count)
        .map(|index| GroupState {
            id: format!("{competition_id}-group-{}", group_label(index)),
            name: group_label(index),
            team_ids: Vec::new(),
            standings: Vec::new(),
        })
        .collect();

    for (position, team_id) in team_ids.iter().enumerate() {
        let row = position / group_count;
        let column = position % group_count;
        let group = if row.is_multiple_of(2) {
            column
        } else {
            group_count - 1 - column
        };
        groups[group].team_ids.push(team_id.clone());
        groups[group]
            .standings
            .push(StandingEntry::new(team_id.clone()));
    }
    groups
}

/// Generate a group-and-knockout competition with the default club shape:
/// double round-robin groups, top two advancing.
pub fn generate_group_knockout_cup(
    name: &str,
    season: u32,
    team_ids: &[String],
    start_date: DateTime<Utc>,
    kind: CompetitionType,
    scope: CompetitionScope,
) -> League {
    generate_group_knockout_cup_with(
        name,
        season,
        team_ids,
        start_date,
        kind,
        scope,
        &GroupStageConfig::default(),
    )
}

/// Generate a group-and-knockout competition: snake-seeded groups playing a
/// round robin shaped by `config`; the knockout bracket is seeded later, once
/// every group fixture has been played.
pub fn generate_group_knockout_cup_with(
    name: &str,
    season: u32,
    team_ids: &[String],
    start_date: DateTime<Utc>,
    kind: CompetitionType,
    scope: CompetitionScope,
    config: &GroupStageConfig,
) -> League {
    let competition_id = Uuid::new_v4().to_string();
    let group_states = seed_groups(&competition_id, team_ids);
    let groups: Vec<Vec<String>> = group_states
        .into_iter()
        .map(|group| group.team_ids)
        .collect();
    build_group_cup(
        competition_id,
        name,
        season,
        team_ids,
        &groups,
        start_date,
        kind,
        scope,
        config,
    )
}

/// Generate a group-and-knockout competition from an explicit group assignment
/// (e.g. a World Cup draw), instead of snake-seeding. Each inner vector is one
/// group's team ids, in draw order.
pub fn generate_group_knockout_cup_with_groups(
    name: &str,
    season: u32,
    groups: &[Vec<String>],
    start_date: DateTime<Utc>,
    kind: CompetitionType,
    scope: CompetitionScope,
    config: &GroupStageConfig,
) -> League {
    let competition_id = Uuid::new_v4().to_string();
    let team_ids: Vec<String> = groups.iter().flatten().cloned().collect();
    build_group_cup(
        competition_id,
        name,
        season,
        &team_ids,
        groups,
        start_date,
        kind,
        scope,
        config,
    )
}

#[allow(clippy::too_many_arguments)]
fn build_group_cup(
    competition_id: String,
    name: &str,
    season: u32,
    team_ids: &[String],
    groups: &[Vec<String>],
    start_date: DateTime<Utc>,
    kind: CompetitionType,
    scope: CompetitionScope,
    config: &GroupStageConfig,
) -> League {
    let mut cup = League::new(competition_id.clone(), name.to_string(), season, team_ids);
    cup.kind = kind.clone();
    cup.scope = scope;
    cup.rules = CompetitionRules {
        format: CompetitionFormat::GroupAndKnockout,
        counts_in_season_flow: true,
        group_qualifiers_per_group: config.qualifiers_per_group,
        group_best_third_qualifiers: config.best_third_qualifiers,
        group_stage_legs: config.legs,
        group_matchday_gap_days: config.matchday_gap_days.max(1) as u32,
        knockout_round_gap_days: config.knockout_round_gap_days,
        knockout_matches_per_day: config.knockout_matches_per_day,
    };
    cup.standings.clear();
    cup.groups = groups
        .iter()
        .enumerate()
        .map(|(index, group_team_ids)| GroupState {
            id: format!("{competition_id}-group-{}", group_label(index)),
            name: group_label(index),
            team_ids: group_team_ids.clone(),
            standings: group_team_ids
                .iter()
                .map(|id| StandingEntry::new(id.clone()))
                .collect(),
        })
        .collect();

    let fixture_competition = fixture_competition_for(&kind);
    for group in &cup.groups {
        let fixtures = crate::schedule::build_round_robin_fixtures_with(
            &competition_id,
            &group.team_ids,
            start_date,
            fixture_competition.clone(),
            config.legs,
            config.matchday_gap_days,
        );
        cup.fixtures.extend(fixtures);
    }

    if let Some(max_per_day) = config.max_concurrent_matches_per_day {
        crate::schedule::spread_fixture_dates(&mut cup.fixtures, start_date, max_per_day);
    }

    cup
}

/// A group's table sorted by points, goal difference, then goals for.
pub fn sorted_group_standings(group: &GroupState) -> Vec<StandingEntry> {
    let mut sorted = group.standings.clone();
    sorted.sort_by(|a, b| {
        b.points
            .cmp(&a.points)
            .then(b.goal_difference().cmp(&a.goal_difference()))
            .then(b.goals_for.cmp(&a.goals_for))
    });
    sorted
}

fn is_knockout_fixture(league: &League, fixture_id: &str) -> bool {
    league
        .knockout_rounds
        .iter()
        .any(|round| round.fixture_ids.iter().any(|id| id == fixture_id))
}

/// Record a completed group fixture in its group's table. For a
/// group-and-knockout competition, once the whole group stage is played out the
/// knockout bracket is seeded with each group's top finishers (group winners
/// first, so any byes favour them). For a plain grouped competition (e.g. World
/// Cup qualifying) only the table is updated. A no-op for competitions without
/// groups and for knockout-round fixtures.
pub fn process_completed_fixture(league: &mut League, fixture_index: usize) {
    if league.groups.is_empty() {
        return;
    }
    let Some(fixture) = league.fixtures.get(fixture_index) else {
        return;
    };
    if is_knockout_fixture(league, &fixture.id) {
        return;
    }
    let Some(result) = fixture.result.clone() else {
        return;
    };
    let home_team_id = fixture.home_team_id.clone();
    let away_team_id = fixture.away_team_id.clone();

    if let Some(group) = league
        .groups
        .iter_mut()
        .find(|group| group.team_ids.contains(&home_team_id))
    {
        if let Some(entry) = group
            .standings
            .iter_mut()
            .find(|entry| entry.team_id == home_team_id)
        {
            entry.record_result(result.home_goals, result.away_goals);
        }
        if let Some(entry) = group
            .standings
            .iter_mut()
            .find(|entry| entry.team_id == away_team_id)
        {
            entry.record_result(result.away_goals, result.home_goals);
        }
    }

    if league.rules.format == CompetitionFormat::GroupAndKnockout {
        maybe_seed_knockout_from_groups(league);
    }
}

fn maybe_seed_knockout_from_groups(league: &mut League) {
    if !league.knockout_rounds.is_empty() {
        return;
    }
    let group_stage_complete = league
        .fixtures
        .iter()
        .filter(|fixture| !is_knockout_fixture(league, &fixture.id))
        .all(|fixture| fixture.status == FixtureStatus::Completed);
    if !group_stage_complete {
        return;
    }

    // Group winners (ranked among themselves), then runners-up, and so on, so
    // the strongest group performances receive any knockout byes. The next
    // placed finishers across all groups can also qualify ("best thirds").
    let per_group = (league.rules.group_qualifiers_per_group.max(1)) as usize;
    let best_remainders = league.rules.group_best_third_qualifiers as usize;

    let mut qualifiers_by_rank: Vec<Vec<StandingEntry>> = vec![Vec::new(); per_group];
    let mut remainder_pool: Vec<StandingEntry> = Vec::new();
    for group in &league.groups {
        for (rank, entry) in sorted_group_standings(group)
            .into_iter()
            .take(per_group + 1)
            .enumerate()
        {
            if rank < per_group {
                qualifiers_by_rank[rank].push(entry);
            } else {
                remainder_pool.push(entry);
            }
        }
    }

    let rank_order = |a: &StandingEntry, b: &StandingEntry| {
        b.points
            .cmp(&a.points)
            .then(b.goal_difference().cmp(&a.goal_difference()))
            .then(b.goals_for.cmp(&a.goals_for))
    };
    let mut qualifiers: Vec<String> = Vec::new();
    for mut rank_entries in qualifiers_by_rank {
        rank_entries.sort_by(rank_order);
        qualifiers.extend(rank_entries.into_iter().map(|entry| entry.team_id));
    }
    if best_remainders > 0 {
        remainder_pool.sort_by(rank_order);
        qualifiers.extend(
            remainder_pool
                .into_iter()
                .take(best_remainders)
                .map(|entry| entry.team_id),
        );
    }
    if qualifiers.len() < 2 {
        return;
    }

    let last_group_date = league
        .fixtures
        .iter()
        .map(|fixture| fixture.date.as_str())
        .max()
        .unwrap_or("2026-01-01");
    let knockout_start = chrono::NaiveDate::parse_from_str(last_group_date, "%Y-%m-%d")
        .ok()
        .and_then(|date| date.and_hms_opt(0, 0, 0))
        .map(|naive| DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
        .unwrap_or_else(Utc::now)
        + Duration::days(league.rules.knockout_round_gap_days as i64);

    crate::schedule::seed_knockout_round(
        league,
        &qualifiers,
        knockout_start,
        fixture_competition_for(&league.kind.clone()),
    );
}

/// Reset a group-and-knockout competition for a new season in place: fresh
/// snake-seeded groups from `participant_ids`, no fixtures played, no bracket.
pub fn regenerate_for_season(league: &mut League, season: u32, start_date: DateTime<Utc>) {
    league.season = season;
    league.fixtures.clear();
    league.standings.clear();
    league.knockout_rounds.clear();
    league.groups = seed_groups(&league.id, &league.participant_ids);

    let fixture_competition = fixture_competition_for(&league.kind.clone());
    let competition_id = league.id.clone();
    for group in league.groups.clone() {
        let fixtures = crate::schedule::build_round_robin_fixtures_with(
            &competition_id,
            &group.team_ids,
            start_date,
            fixture_competition.clone(),
            league.rules.group_stage_legs.max(1),
            i64::from(league.rules.group_matchday_gap_days.max(1)),
        );
        league.fixtures.extend(fixtures);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use domain::league::MatchResult;

    fn clubs(count: usize) -> Vec<String> {
        (0..count).map(|i| format!("club-{i}")).collect()
    }

    fn start() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 15, 0, 0, 0).unwrap()
    }

    fn make_cup(entrants: usize) -> League {
        generate_group_knockout_cup(
            "Continental Champions Cup",
            2026,
            &clubs(entrants),
            start(),
            CompetitionType::ContinentalClub,
            CompetitionScope::Continental,
        )
    }

    fn complete_fixture(league: &mut League, index: usize, home_goals: u8, away_goals: u8) {
        league.fixtures[index].status = FixtureStatus::Completed;
        league.fixtures[index].result = Some(MatchResult {
            home_goals,
            away_goals,
            home_scorers: vec![],
            away_scorers: vec![],
            report: None,
            home_penalties: None,
            away_penalties: None,
        });
        process_completed_fixture(league, index);
    }

    #[test]
    fn generates_snake_seeded_groups_with_round_robin_fixtures() {
        let cup = make_cup(8);

        assert_eq!(cup.rules.format, CompetitionFormat::GroupAndKnockout);
        assert_eq!(cup.groups.len(), 2);
        assert_eq!(cup.groups[0].name, "A");
        assert_eq!(cup.groups[1].name, "B");
        // Snake seeding: A gets seeds 0,3,4,7; B gets 1,2,5,6.
        assert_eq!(
            cup.groups[0].team_ids,
            vec!["club-0", "club-3", "club-4", "club-7"]
        );
        assert_eq!(
            cup.groups[1].team_ids,
            vec!["club-1", "club-2", "club-5", "club-6"]
        );
        // 4-club double round robin = 12 fixtures per group.
        assert_eq!(cup.fixtures.len(), 24);
        assert!(
            cup.fixtures
                .iter()
                .all(|f| f.competition == FixtureCompetition::ContinentalClub)
        );
        assert!(cup.knockout_rounds.is_empty());
        assert!(cup.standings.is_empty());
    }

    #[test]
    fn group_results_update_only_the_owning_group_table() {
        let mut cup = make_cup(8);
        // Find a Group A fixture.
        let index = cup
            .fixtures
            .iter()
            .position(|f| cup.groups[0].team_ids.contains(&f.home_team_id))
            .unwrap();
        let home = cup.fixtures[index].home_team_id.clone();

        complete_fixture(&mut cup, index, 2, 0);

        let group_a_entry = cup.groups[0]
            .standings
            .iter()
            .find(|entry| entry.team_id == home)
            .unwrap();
        assert_eq!(group_a_entry.points, 3);
        assert!(
            cup.groups[1]
                .standings
                .iter()
                .all(|entry| entry.played == 0),
            "the other group is untouched"
        );
        assert!(
            cup.knockout_rounds.is_empty(),
            "groups are not finished yet"
        );
    }

    #[test]
    fn finished_group_stage_seeds_the_knockout_with_top_two_per_group() {
        let mut cup = make_cup(8);
        // Home win everywhere: group standings then rank by seeding order.
        for index in 0..cup.fixtures.len() {
            if cup.fixtures[index].status == FixtureStatus::Scheduled
                && !is_knockout_fixture(&cup, &cup.fixtures[index].id.clone())
            {
                complete_fixture(&mut cup, index, 1, 0);
            }
        }

        assert_eq!(
            cup.knockout_rounds.len(),
            1,
            "bracket seeded once groups end"
        );
        let round = &cup.knockout_rounds[0];
        assert_eq!(round.name, "Semifinal");
        assert_eq!(round.fixture_ids.len(), 2);
        assert!(round.bye_team_ids.is_empty());

        // Four qualifiers: two per group.
        let knockout_fixtures: Vec<_> = cup
            .fixtures
            .iter()
            .filter(|f| round.fixture_ids.contains(&f.id))
            .collect();
        let mut qualifiers: Vec<&str> = knockout_fixtures
            .iter()
            .flat_map(|f| [f.home_team_id.as_str(), f.away_team_id.as_str()])
            .collect();
        qualifiers.sort();
        let from_a = qualifiers
            .iter()
            .filter(|id| cup.groups[0].team_ids.iter().any(|t| t == *id))
            .count();
        let from_b = qualifiers
            .iter()
            .filter(|id| cup.groups[1].team_ids.iter().any(|t| t == *id))
            .count();
        assert_eq!((from_a, from_b), (2, 2));
    }

    #[test]
    fn single_leg_groups_with_best_thirds_qualify_a_2026_style_field() {
        // 12 entrants -> 3 single-leg groups; top 2 per group plus the 2 best
        // third-placed teams -> 8 knockout qualifiers (a scaled 2026 format).
        let mut cup = generate_group_knockout_cup_with(
            "World Cup",
            2026,
            &clubs(12),
            start(),
            CompetitionType::InternationalNation,
            CompetitionScope::International,
            &GroupStageConfig {
                legs: 1,
                matchday_gap_days: 3,
                qualifiers_per_group: 2,
                best_third_qualifiers: 2,
                ..Default::default()
            },
        );

        assert_eq!(cup.groups.len(), 3);
        // 4-club single round robin = 6 fixtures per group.
        assert_eq!(cup.fixtures.len(), 18);
        let max_matchday = cup.fixtures.iter().map(|f| f.matchday).max().unwrap();
        assert_eq!(max_matchday, 3, "single leg plays three group matchdays");

        for index in 0..cup.fixtures.len() {
            if cup.fixtures[index].status == FixtureStatus::Scheduled {
                complete_fixture(&mut cup, index, 1, 0);
            }
        }

        let round = &cup.knockout_rounds[0];
        assert_eq!(round.name, "Quarterfinal");
        assert_eq!(round.fixture_ids.len(), 4, "8 qualifiers pair into 4 ties");
        assert!(round.bye_team_ids.is_empty());

        // Exactly two third-placed teams advanced.
        let knockout_team_ids: Vec<String> = cup
            .fixtures
            .iter()
            .filter(|f| round.fixture_ids.contains(&f.id))
            .flat_map(|f| [f.home_team_id.clone(), f.away_team_id.clone()])
            .collect();
        let third_placed_qualifiers = cup
            .groups
            .iter()
            .map(|group| sorted_group_standings(group)[2].team_id.clone())
            .filter(|team_id| knockout_team_ids.contains(team_id))
            .count();
        assert_eq!(third_placed_qualifiers, 2);
    }

    #[test]
    fn regenerate_for_season_resets_groups_and_fixtures() {
        let mut cup = make_cup(8);
        for index in 0..cup.fixtures.len() {
            if cup.fixtures[index].status == FixtureStatus::Scheduled {
                complete_fixture(&mut cup, index, 1, 0);
            }
        }
        assert!(!cup.knockout_rounds.is_empty());

        regenerate_for_season(&mut cup, 2027, start());

        assert_eq!(cup.season, 2027);
        assert!(cup.knockout_rounds.is_empty());
        assert_eq!(cup.groups.len(), 2);
        assert_eq!(cup.fixtures.len(), 24);
        assert!(
            cup.fixtures
                .iter()
                .all(|f| f.status == FixtureStatus::Scheduled)
        );
        assert!(
            cup.groups
                .iter()
                .all(|group| group.standings.iter().all(|entry| entry.played == 0))
        );
    }

    #[test]
    fn regeneration_preserves_single_leg_weekly_group_schedule() {
        let config = GroupStageConfig { legs: 1, matchday_gap_days: 7, knockout_round_gap_days: 7, ..Default::default() };
        let mut cup = generate_group_knockout_cup_with("State Series", 2026, &clubs(8), start(), CompetitionType::Cup, CompetitionScope::Regional, &config);
        regenerate_for_season(&mut cup, 2027, start());
        assert_eq!(cup.rules.group_stage_legs, 1);
        assert_eq!(cup.rules.group_matchday_gap_days, 7);
        assert_eq!(cup.fixtures.len(), 12);
    }
}
