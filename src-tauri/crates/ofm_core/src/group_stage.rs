//! Group-and-knockout competitions: a Champions-League-style group stage whose
//! top finishers seed the knockout bracket.

use chrono::{DateTime, Duration, Utc};
use domain::league::{
    CompetitionFormat, CompetitionRules, CompetitionScope, CompetitionType, FixtureCompetition,
    FixtureStatus, GroupState, League, StandingEntry,
};

/// Shape of a group stage at creation time.
#[derive(Debug, Clone)]
pub struct GroupStageConfig {
    /// Maximum clubs per snake-seeded group.
    pub group_size: u32,
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
        let rules = CompetitionRules::default();
        Self {
            group_size: rules.group_size,
            legs: rules.group_stage_legs,
            matchday_gap_days: i64::from(rules.group_matchday_gap_days),
            qualifiers_per_group: rules.group_qualifiers_per_group,
            best_third_qualifiers: rules.group_best_third_qualifiers,
            knockout_round_gap_days: rules.knockout_round_gap_days,
            max_concurrent_matches_per_day: None,
            knockout_matches_per_day: rules.knockout_matches_per_day,
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
    // Continue after Z so small authored groups keep distinct ids in large fields.
    let mut number = index + 1;
    let mut letters = Vec::new();
    while number > 0 {
        number -= 1;
        letters.push(char::from(b'A' + (number % 26) as u8));
        number /= 26;
    }
    letters.into_iter().rev().collect()
}

/// Number of balanced groups needed for a field with this maximum group size.
pub(crate) fn group_count(entrants: usize, group_size: usize) -> usize {
    // Use the supported configuration minimum for malformed persisted sizes.
    let group_size = group_size.max(domain::league::MIN_GROUP_SIZE as usize);
    entrants.div_ceil(group_size).max(1)
}

/// Snake-seed `team_ids` (strongest first) into balanced groups up to `group_size`, so
/// each group gets a comparable spread of strength.
fn seed_groups(competition_id: &str, team_ids: &[String], group_size: u32) -> Vec<GroupState> {
    let group_count = group_count(team_ids.len(), group_size as usize);
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
    let season_key = season.to_string();
    let start_key = start_date.to_rfc3339();
    let mut id_parts = vec!["group-cup", name, &season_key, &start_key];
    id_parts.extend(team_ids.iter().map(String::as_str));
    let competition_id = crate::seed::derived_id(&id_parts);
    let group_states = seed_groups(&competition_id, team_ids, config.group_size);
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
    let team_ids: Vec<String> = groups.iter().flatten().cloned().collect();
    let season_key = season.to_string();
    let start_key = start_date.to_rfc3339();
    let group_sizes: Vec<String> = groups.iter().map(|group| group.len().to_string()).collect();
    let mut id_parts = vec!["group-cup-with-groups", name, &season_key, &start_key];
    // Length-prefixed entrants preserve ids containing separators; group sizes also
    // preserve the draw's nested boundaries, including empty groups.
    for (group, size) in groups.iter().zip(&group_sizes) {
        id_parts.push(size.as_str());
        id_parts.extend(group.iter().map(String::as_str));
    }
    let competition_id = crate::seed::derived_id(&id_parts);
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
        group_size: config.group_size,
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

    let qualifiers = knockout_bracket_order(league);
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

/// The existing field/seeding rule, shared with strict archive verification.
pub(crate) fn knockout_qualifiers(league: &League) -> Vec<String> {
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
    qualifiers
}

/// The group-stage qualifiers in first-round bracket order, as [`crate::schedule::seed_knockout_round`]
/// reads them: the byes first (the strongest group winners), then the ties two entrants at a time,
/// in the order they sit in the bracket. A round's winners meet their neighbour next.
///
/// - A group winner plays the lowest-placed qualifier from another group, so group winners meet
///   runners-up (and best thirds), not each other: A1 v B2 and B1 v A2 for a plain four-team
///   knockout, and the lowest qualifiers available when best thirds are in the field.
/// - Two teams of one group never meet in this round while another opponent exists.
/// - The ties are placed so a group's teams sit in opposite halves of the bracket where they can:
///   A1 and A2 can only meet in the final.
///
/// The set of entrants is exactly [`knockout_qualifiers`]'s; only their order differs, so a bracket
/// drawn the old way (winner against winner) still verifies in the strict archive check, which
/// reads the entrants as a set.
pub(crate) fn knockout_bracket_order(league: &League) -> Vec<String> {
    let ranked = knockout_qualifiers(league);
    let Some(byes) = crate::schedule::knockout_bye_count(ranked.len()) else {
        return ranked;
    };
    let group_of = |team: &str| {
        league
            .groups
            .iter()
            .position(|group| group.team_ids.iter().any(|id| id == team))
    };
    let (bye_teams, playing) = ranked.split_at(byes);
    let paired = if byes == 0 && is_two_per_group_without_thirds(league) {
        paired_groups_ties(league)
    } else {
        Vec::new()
    };
    let ties = if paired.len() * 2 == playing.len() {
        paired
    } else {
        lowest_available_ties(playing, group_of)
    };
    let ties = arrange_halves(ties, &group_of);
    bye_teams
        .iter()
        .cloned()
        .chain(ties.into_iter().flat_map(|(home, away)| [home, away]))
        .collect()
}

type Tie = (String, String);

/// A tie with its index in the order it was drawn.
type NumberedTie = (usize, Tie);

fn is_two_per_group_without_thirds(league: &League) -> bool {
    league.rules.group_qualifiers_per_group == 2
        && league.rules.group_best_third_qualifiers == 0
        && league.groups.len() >= 2
        && league.groups.len().is_multiple_of(2)
}

/// A1 v B2, B1 v A2, C1 v D2, D1 v C2, ...: each winner against the runner-up of the group it is
/// paired with, the groups paired in the order they were drawn.
fn paired_groups_ties(league: &League) -> Vec<Tie> {
    let placed = |group: &GroupState, place: usize| {
        sorted_group_standings(group)
            .into_iter()
            .nth(place)
            .map(|entry| entry.team_id)
    };
    let mut ties = Vec::new();
    for pair in league.groups.chunks(2) {
        let [first, second] = pair else { continue };
        for (winner_group, runner_up_group) in [(first, second), (second, first)] {
            if let (Some(winner), Some(runner_up)) =
                (placed(winner_group, 0), placed(runner_up_group, 1))
            {
                ties.push((winner, runner_up));
            }
        }
    }
    ties
}

/// The best remaining qualifier plays the lowest-placed remaining qualifier of another group (of
/// its own group only when no other is left). `ranked` is best first.
fn lowest_available_ties(ranked: &[String], group_of: impl Fn(&str) -> Option<usize>) -> Vec<Tie> {
    let mut pool: Vec<String> = ranked.to_vec();
    let mut ties = Vec::new();
    while !pool.is_empty() {
        let best = pool.remove(0);
        if pool.is_empty() {
            break;
        }
        let best_group = group_of(best.as_str());
        let opponent = pool
            .iter()
            .rposition(|team| group_of(team.as_str()) != best_group)
            .unwrap_or(pool.len() - 1);
        ties.push((best, pool.remove(opponent)));
    }
    ties
}

/// The ties in bracket order: split into two halves with as few same-group pairs inside a half as
/// possible, then each half the same way. Searches every split for fields up to
/// [`MAX_ARRANGED_TIES`] ties (a 64-team round) and keeps the order given beyond that. The first
/// split with the fewest clashes wins, so the result is the same on every run.
fn arrange_halves(ties: Vec<Tie>, group_of: &impl Fn(&str) -> Option<usize>) -> Vec<Tie> {
    const MAX_ARRANGED_TIES: usize = 20;
    if ties.len() <= 2 || ties.len() > MAX_ARRANGED_TIES {
        return ties;
    }
    let half = ties.len() / 2;
    let clashes = |members: &[&Tie]| {
        let mut per_group = std::collections::BTreeMap::<usize, usize>::new();
        for tie in members {
            for team in [&tie.0, &tie.1] {
                if let Some(group) = group_of(team.as_str()) {
                    *per_group.entry(group).or_default() += 1;
                }
            }
        }
        per_group
            .values()
            .map(|count| count * (count - 1) / 2)
            .sum::<usize>()
    };
    let mut best: Option<(usize, u32)> = None;
    for mask in 0..(1u32 << ties.len()) {
        // The first tie stays in the first half: swapping the halves is the same bracket.
        if mask & 1 == 0 || mask.count_ones() as usize != half {
            continue;
        }
        let side = |first: bool| -> Vec<&Tie> {
            ties.iter()
                .enumerate()
                .filter(|(index, _)| (mask & (1u32 << *index) != 0) == first)
                .map(|(_, tie)| tie)
                .collect()
        };
        let cost = clashes(&side(true)) + clashes(&side(false));
        if best.is_none_or(|(lowest, _)| cost < lowest) {
            best = Some((cost, mask));
        }
    }
    let Some((_, mask)) = best else {
        return ties;
    };
    let (first, second): (Vec<NumberedTie>, Vec<NumberedTie>) = ties
        .into_iter()
        .enumerate()
        .partition(|(index, _)| mask & (1u32 << *index) != 0);
    let strip = |side: Vec<NumberedTie>| side.into_iter().map(|(_, tie)| tie).collect::<Vec<_>>();
    let mut arranged = arrange_halves(strip(first), group_of);
    arranged.extend(arrange_halves(strip(second), group_of));
    arranged
}

/// Reset a group-and-knockout competition for a new season in place: fresh
/// snake-seeded groups from `participant_ids`, no fixtures played, no bracket.
pub fn regenerate_for_season(league: &mut League, season: u32, start_date: DateTime<Utc>) {
    league.season = season;
    crate::calendar_identity::record_regenerated_calendar(league, start_date);
    league.fixtures.clear();
    league.standings.clear();
    league.knockout_rounds.clear();
    league.groups = seed_groups(&league.id, &league.participant_ids, league.rules.group_size);

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

    /// Given a group cup of each competition kind, from World Cup finals to a friendly tournament
    /// When its group stage is drawn
    /// Then every group fixture carries its own competition's tag: a World Cup match is an
    ///      international fixture, not a domestic cup tie the club calendar would move around
    #[test]
    fn group_stage_fixtures_carry_their_competitions_fixture_tag() {
        let tags: Vec<(CompetitionType, Vec<FixtureCompetition>)> = [
            (
                CompetitionType::ContinentalClub,
                CompetitionScope::Continental,
            ),
            (
                CompetitionType::InternationalClub,
                CompetitionScope::International,
            ),
            (
                CompetitionType::InternationalNation,
                CompetitionScope::International,
            ),
            (
                CompetitionType::FriendlyCup,
                CompetitionScope::International,
            ),
            (CompetitionType::Cup, CompetitionScope::Domestic),
        ]
        .into_iter()
        .map(|(kind, scope)| {
            let cup = generate_group_knockout_cup(
                "Tagged Cup",
                2026,
                &clubs(8),
                start(),
                kind.clone(),
                scope,
            );
            let mut seen: Vec<FixtureCompetition> = cup
                .fixtures
                .iter()
                .map(|fixture| fixture.competition.clone())
                .collect();
            seen.dedup();
            (kind, seen)
        })
        .collect();

        assert_eq!(
            tags,
            vec![
                (
                    CompetitionType::ContinentalClub,
                    vec![FixtureCompetition::ContinentalClub]
                ),
                (
                    CompetitionType::InternationalClub,
                    vec![FixtureCompetition::InternationalClub]
                ),
                (
                    CompetitionType::InternationalNation,
                    vec![FixtureCompetition::InternationalNation]
                ),
                (
                    CompetitionType::FriendlyCup,
                    vec![FixtureCompetition::FriendlyCup]
                ),
                (CompetitionType::Cup, vec![FixtureCompetition::Cup]),
            ]
        );
    }

    /// Given distinct entrant lists whose comma-joined text is identical,
    /// When snake-seeded group cups are generated for the same season and date,
    /// Then their competition and fixture ids remain distinct.
    #[test]
    fn seeded_group_cups_preserve_entrant_boundaries_inside_ids() {
        let first = ["a,b", "c", "d", "e"].map(String::from);
        let second = ["a", "b,c", "d", "e"].map(String::from);
        let cup = |teams: &[String]| {
            generate_group_knockout_cup(
                "Test Cup",
                2026,
                teams,
                start(),
                CompetitionType::Cup,
                CompetitionScope::Domestic,
            )
        };
        let first = cup(&first);
        let second = cup(&second);

        assert_ne!(first.id, second.id);
        assert!(
            first
                .fixtures
                .iter()
                .all(|fixture| second.fixtures.iter().all(|other| fixture.id != other.id))
        );
    }

    /// Given explicit groups with distinct comma-containing entrant ids,
    /// When group cups are generated for the same season and date,
    /// Then their competition and fixture ids remain distinct.
    #[test]
    fn explicit_group_cups_preserve_entrant_boundaries_inside_ids() {
        let first = vec![vec!["a,b".into(), "c".into(), "d".into(), "e".into()]];
        let second = vec![vec!["a".into(), "b,c".into(), "d".into(), "e".into()]];
        let cup = |groups: &[Vec<String>]| {
            generate_group_knockout_cup_with_groups(
                "Test Cup",
                2026,
                groups,
                start(),
                CompetitionType::Cup,
                CompetitionScope::Domestic,
                &GroupStageConfig::default(),
            )
        };
        let first = cup(&first);
        let second = cup(&second);

        assert_ne!(first.id, second.id);
        assert!(
            first
                .fixtures
                .iter()
                .all(|fixture| second.fixtures.iter().all(|other| fixture.id != other.id))
        );
    }

    /// Given the same flattened entrants but different explicit group boundaries,
    /// When group cups are generated with a common a/b group,
    /// Then their competition ids and the common group's fixture ids remain distinct.
    #[test]
    fn explicit_group_cups_preserve_group_boundaries() {
        let first = vec![
            vec!["a".into(), "b".into()],
            vec!["c".into(), "d".into()],
            vec!["e".into(), "f".into()],
        ];
        let second = vec![
            vec!["a".into(), "b".into()],
            vec!["c".into(), "d".into(), "e".into(), "f".into()],
        ];
        let cup = |groups: &[Vec<String>]| {
            generate_group_knockout_cup_with_groups(
                "Test Cup",
                2026,
                groups,
                start(),
                CompetitionType::Cup,
                CompetitionScope::Domestic,
                &GroupStageConfig::default(),
            )
        };
        let first = cup(&first);
        let second = cup(&second);
        let tie = |cup: &League| {
            cup.fixtures
                .iter()
                .find(|f| f.home_team_id == "a" && f.away_team_id == "b")
                .unwrap()
                .id
                .clone()
        };

        assert_ne!(tie(&first), tie(&second));
        assert_ne!(first.id, second.id);
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
    fn regeneration_handles_saved_group_sizes_below_minimum() {
        for size in [0, 1] {
            let mut cup = make_cup(8);
            let mut saved_rules = serde_json::to_value(&cup.rules).unwrap();
            saved_rules["group_size"] = serde_json::json!(size);
            cup.rules = serde_json::from_value(saved_rules).unwrap();

            regenerate_for_season(&mut cup, 2032, start());

            assert_eq!(cup.groups.len(), 4, "saved group size {size}");
            assert!(cup.groups.iter().all(|group| group.team_ids.len() == 2));
            assert_eq!(cup.fixtures.len(), 8);
        }
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

    /// Plays every group fixture as a home win, so each group ranks in its drawn order, and
    /// returns the cup with its first knockout round seeded.
    fn after_the_group_stage(mut cup: League) -> League {
        for index in 0..cup.fixtures.len() {
            if cup.fixtures[index].status == FixtureStatus::Scheduled
                && !is_knockout_fixture(&cup, &cup.fixtures[index].id.clone())
            {
                complete_fixture(&mut cup, index, 1, 0);
            }
        }
        cup
    }

    /// The first knockout round's ties in bracket order.
    fn first_round_ties(cup: &League) -> Vec<(String, String)> {
        cup.knockout_rounds[0]
            .fixture_ids
            .iter()
            .map(|id| {
                let fixture = cup.fixtures.iter().find(|f| &f.id == id).unwrap();
                (fixture.home_team_id.clone(), fixture.away_team_id.clone())
            })
            .collect()
    }

    /// Where a team finished in its group, 1 for the winner.
    fn place_of(cup: &League, team: &str) -> usize {
        let group = cup
            .groups
            .iter()
            .find(|group| group.team_ids.iter().any(|id| id == team))
            .unwrap();
        sorted_group_standings(group)
            .iter()
            .position(|entry| entry.team_id == team)
            .unwrap()
            + 1
    }

    fn group_index_of(cup: &League, team: &str) -> usize {
        cup.groups
            .iter()
            .position(|group| group.team_ids.iter().any(|id| id == team))
            .unwrap()
    }

    /// The team that finished `place` in group `group`.
    fn finisher(cup: &League, group: usize, place: usize) -> String {
        sorted_group_standings(&cup.groups[group])[place - 1]
            .team_id
            .clone()
    }

    /// Given two groups of four that each send two teams through,
    /// When the knockout is drawn,
    /// Then each group winner plays the other group's runner-up: A1 v B2 and B1 v A2.
    #[test]
    fn two_groups_cross_over_winner_against_the_other_runner_up() {
        let cup = after_the_group_stage(make_cup(8));

        assert_eq!(
            first_round_ties(&cup),
            vec![
                (finisher(&cup, 0, 1), finisher(&cup, 1, 2)),
                (finisher(&cup, 1, 1), finisher(&cup, 0, 2)),
            ]
        );
    }

    /// Given four groups that each send two teams through,
    /// When the knockout is drawn,
    /// Then the winners meet runners-up of the paired group, nobody meets a team of their own
    ///      group, and a group's two teams sit in opposite halves of the bracket.
    #[test]
    fn four_groups_pair_winners_with_runners_up_in_opposite_halves() {
        let cup = after_the_group_stage(make_cup(16));
        let ties = first_round_ties(&cup);

        assert_eq!(
            ties,
            vec![
                (finisher(&cup, 0, 1), finisher(&cup, 1, 2)),
                (finisher(&cup, 2, 1), finisher(&cup, 3, 2)),
                (finisher(&cup, 1, 1), finisher(&cup, 0, 2)),
                (finisher(&cup, 3, 1), finisher(&cup, 2, 2)),
            ]
        );
        for group in 0..4 {
            let tie_of = |team: String| {
                ties.iter()
                    .position(|(home, away)| *home == team || *away == team)
                    .unwrap()
            };
            let (winner, runner_up) = (
                tie_of(finisher(&cup, group, 1)),
                tie_of(finisher(&cup, group, 2)),
            );
            assert!(
                (winner < 2) != (runner_up < 2),
                "group {group}: ties {winner} and {runner_up} are in the same half"
            );
        }
    }

    /// Given six groups that each send two teams through, and the four best third-placed teams,
    /// When the knockout is drawn,
    /// Then every group winner plays one of the six lowest qualifiers, and no tie pairs two teams
    ///      of one group.
    #[test]
    fn with_best_thirds_the_winners_meet_the_lowest_qualifiers() {
        let cup = after_the_group_stage(generate_group_knockout_cup_with(
            "World Cup",
            2026,
            &clubs(24),
            start(),
            CompetitionType::InternationalNation,
            CompetitionScope::International,
            &GroupStageConfig {
                legs: 1,
                matchday_gap_days: 3,
                qualifiers_per_group: 2,
                best_third_qualifiers: 4,
                ..Default::default()
            },
        ));
        let ranked = knockout_qualifiers(&cup);
        let lowest_six: Vec<&String> = ranked[ranked.len() - 6..].iter().collect();

        let ties = first_round_ties(&cup);

        assert_eq!(ties.len(), 8);
        for (home, away) in &ties {
            assert_ne!(
                group_index_of(&cup, home),
                group_index_of(&cup, away),
                "{home} v {away} are from one group"
            );
        }
        for group in 0..6 {
            let winner = finisher(&cup, group, 1);
            let tie = ties
                .iter()
                .find(|(home, away)| *home == winner || *away == winner)
                .unwrap();
            let opponent = if tie.0 == winner { &tie.1 } else { &tie.0 };
            assert!(
                lowest_six.contains(&opponent),
                "{winner} (group {group}) plays {opponent}, who is not among the six lowest qualifiers"
            );
            assert!(place_of(&cup, opponent) >= 2);
        }
    }

    /// Given three groups that each send two teams through (six qualifiers, so two byes),
    /// When the knockout is drawn,
    /// Then the byes go to the two strongest group winners, and the other four play off with no
    ///      tie inside one group.
    #[test]
    fn byes_go_to_the_strongest_winners_and_no_tie_is_inside_a_group() {
        let cup = after_the_group_stage(make_cup(12));
        let ranked = knockout_qualifiers(&cup);

        let round = &cup.knockout_rounds[0];
        assert_eq!(round.bye_team_ids, ranked[..2].to_vec());
        for team in &round.bye_team_ids {
            assert_eq!(place_of(&cup, team), 1, "{team} has a bye but did not win");
        }
        let ties = first_round_ties(&cup);
        assert_eq!(ties.len(), 2);
        for (home, away) in &ties {
            assert_ne!(group_index_of(&cup, home), group_index_of(&cup, away));
        }
        // The one winner without a bye plays the lowest-placed qualifier of another group.
        let winner = ranked[2].clone();
        let lowest_other = ranked
            .iter()
            .rev()
            .find(|team| group_index_of(&cup, team) != group_index_of(&cup, &winner))
            .unwrap();
        let tie = ties
            .iter()
            .find(|(home, away)| *home == winner || *away == winner)
            .unwrap();
        let opponent = if tie.0 == winner { &tie.1 } else { &tie.0 };
        assert_eq!(opponent, lowest_other);
    }

    /// Plays every group fixture with a pseudo-random score drawn from `seed`, so group tables
    /// are not simply ordered by the draw.
    fn play_group_stage_at_random(cup: &mut League, seed: u64) {
        let mut state = seed.wrapping_add(17);
        let mut next = move || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (state >> 33) as u8 % 5
        };
        for index in 0..cup.fixtures.len() {
            let id = cup.fixtures[index].id.clone();
            if cup.fixtures[index].status == FixtureStatus::Scheduled
                && !is_knockout_fixture(cup, &id)
            {
                let (home, away) = (next(), next());
                complete_fixture(cup, index, home, away);
            }
        }
    }

    /// Given group-and-knockout fields of every shape a package can describe (2 to 8 groups of 3
    ///       to 5, one to three qualifiers per group, no best thirds to as many as groups), with
    ///       six pseudo-random sets of group results each,
    /// When the group stage ends and the first knockout round is drawn,
    /// Then no tie pairs two teams of one group whenever a draw without such a tie exists: the
    ///      teams in play are an even number and no group supplies more than half of them.
    #[test]
    fn no_first_round_tie_is_inside_a_group_when_a_clean_draw_exists() {
        let mut checked = 0;
        for group_count in 2usize..=8 {
            for group_size in 3usize..=5 {
                for qualifiers in 1u32..=3 {
                    if qualifiers as usize >= group_size {
                        continue;
                    }
                    for best_thirds in [0, 1, group_count as u32 / 2, group_count as u32] {
                        for seed in 0..6u64 {
                            let mut cup = generate_group_knockout_cup_with(
                                "Cup",
                                2026,
                                &clubs(group_count * group_size),
                                start(),
                                CompetitionType::Cup,
                                CompetitionScope::Domestic,
                                &GroupStageConfig {
                                    group_size: group_size as u32,
                                    legs: 1,
                                    matchday_gap_days: 3,
                                    qualifiers_per_group: qualifiers,
                                    best_third_qualifiers: best_thirds,
                                    ..Default::default()
                                },
                            );
                            play_group_stage_at_random(&mut cup, seed);
                            let Some(round) = cup.knockout_rounds.first() else {
                                continue;
                            };
                            let playing: Vec<String> = first_round_ties(&cup)
                                .into_iter()
                                .flat_map(|(home, away)| [home, away])
                                .collect();
                            let mut per_group = std::collections::BTreeMap::<usize, usize>::new();
                            for team in &playing {
                                *per_group.entry(group_index_of(&cup, team)).or_default() += 1;
                            }
                            let largest = per_group.values().copied().max().unwrap_or(0);
                            if largest * 2 > playing.len() {
                                continue;
                            }
                            checked += 1;
                            for (home, away) in first_round_ties(&cup) {
                                assert_ne!(
                                    group_index_of(&cup, &home),
                                    group_index_of(&cup, &away),
                                    "{home} v {away}: groups={group_count} size={group_size} \
                                 qualifiers={qualifiers} thirds={best_thirds} seed={seed} ({} byes)",
                                    round.bye_team_ids.len()
                                );
                            }
                        }
                    }
                }
            }
        }
        assert!(checked > 100, "only {checked} field shapes were checked");
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
        let config = GroupStageConfig {
            legs: 1,
            matchday_gap_days: 7,
            knockout_round_gap_days: 7,
            ..Default::default()
        };
        let mut cup = generate_group_knockout_cup_with(
            "State Series",
            2026,
            &clubs(8),
            start(),
            CompetitionType::Cup,
            CompetitionScope::Regional,
            &config,
        );
        regenerate_for_season(&mut cup, 2027, start());
        assert_eq!(cup.rules.group_stage_legs, 1);
        assert_eq!(cup.rules.group_matchday_gap_days, 7);
        assert_eq!(cup.fixtures.len(), 12);
    }
}
