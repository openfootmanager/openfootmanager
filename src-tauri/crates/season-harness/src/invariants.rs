//! The rules a run is held to.
//!
//! Each rule is a small function in a table: [`EVERY_DAY`] rules are checked
//! after every simulated day, [`AT_ROLLOVER`] rules once each season is rolled
//! over, comparing what was true just before with what is true just after.
//! Adding a rule is adding an entry, not editing the observer.
//!
//! These rules are outcome-independent by design. They hold for any season
//! however the matches fell, so a change to how the match engine plays can
//! never break them; only a change to how the *world* behaves can.
//!
//! What is deliberately not here: how many players a squad holds, how old they
//! are, how much money clubs have. Those drift for reasons that are known and
//! filed, and a threshold on them would either fail immediately or be tuned
//! until it passed. They belong to a report of how the numbers move, not to a
//! gate.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use chrono::NaiveDate;

use domain::league::{CompetitionFormat, CompetitionType, FixtureCompetition, FixtureStatus};
use domain::player::PlayerSeasonStats;
use ofm_core::end_of_season::{EndOfSeasonSummary, has_full_schedule, is_regenerated_at_rollover};
use ofm_core::game::Game;
use ofm_core::nations::is_split_season_country;
use ofm_core::world::ladder::{division_sizes, domestic_tables_by_country, ladder_violations};
use ofm_core::world_cup::{is_world_cup_competition, is_world_cup_qualifying};

use crate::driver::SeasonObserver;

/// One rule broken at one moment of a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// The harness's season count, 1-based; 0 is the world as it was built.
    pub season: u32,
    /// The day within the season, or `None` at a rollover or before the run.
    pub day: Option<u32>,
    pub rule: &'static str,
    pub message: String,
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.day {
            Some(day) => write!(
                f,
                "[season {} day {day}] {}: {}",
                self.season, self.rule, self.message
            ),
            None => write!(
                f,
                "[season {}] {}: {}",
                self.season, self.rule, self.message
            ),
        }
    }
}

type DayRule = fn(&Game) -> Vec<String>;
type RolloverRule = fn(&Before, &Game) -> Vec<String>;

const EVERY_DAY: &[(&str, DayRule)] = &[
    ("world-not-emptied", world_not_emptied),
    ("referential-integrity", referential_integrity),
    ("finances-in-range", finances_in_range),
    ("no-stranded-fixture", no_stranded_fixture),
    (
        "completed-fixtures-have-results",
        completed_fixtures_have_results,
    ),
    (
        "completed-club-fixtures-have-reports",
        completed_club_fixtures_have_reports,
    ),
    ("standings-are-consistent", standings_are_consistent),
    (CLUB_RESTS_BETWEEN_MATCHES, club_rests_between_matches),
];

const AT_ROLLOVER: &[(&str, RolloverRule)] = &[
    ("ladder-holds", ladder_holds),
    ("competitions-survive", competitions_survive),
    (
        "completed-competitions-advance-one-season",
        completed_competitions_advance_one_season,
    ),
    ("regenerated-tables-are-fresh", regenerated_tables_are_fresh),
    ("season-stats-are-reset", season_stats_are_reset),
    ("finished-ladders-move-clubs", finished_ladders_move_clubs),
];

/// Violations kept per rule: enough to name what is wrong, not enough for one systemic failure
/// to bury a report under thousands of copies of itself. Per rule, so a rule that fails
/// everywhere cannot hide another.
const MAX_RECORDED: usize = 200;

/// The invariants observer: attach it to a run, then read [`Self::violations`].
pub struct Invariants {
    authored_sizes: BTreeMap<String, usize>,
    before: Option<Before>,
    violations: Vec<Violation>,
    seen: BTreeSet<(&'static str, String)>,
    dropped: BTreeMap<&'static str, usize>,
}

impl Invariants {
    /// Takes the authored division sizes from `game` as it is now, and checks
    /// the world exactly as built.
    pub fn new(game: &Game) -> Self {
        let mut invariants = Self {
            authored_sizes: division_sizes(game),
            before: None,
            violations: Vec::new(),
            seen: BTreeSet::new(),
            dropped: BTreeMap::new(),
        };
        invariants.check_every_day(game, 0, None);
        let sizes = invariants.authored_sizes.clone();
        for message in ladder_violations(game, &sizes) {
            invariants.record(0, None, "ladder-holds", message, false);
        }
        invariants
    }

    pub fn violations(&self) -> &[Violation] {
        &self.violations
    }

    pub fn is_clean(&self) -> bool {
        self.report().is_empty()
    }

    /// Every violation, one per line, for a failed assertion to print.
    pub fn report(&self) -> String {
        self.report_where(|_| true)
    }

    /// The report of one rule alone: empty when that rule held.
    pub fn report_of(&self, rule: &str) -> String {
        self.report_where(|recorded| recorded == rule)
    }

    /// The report of every rule but one: empty when they all held.
    pub fn report_except(&self, rule: &str) -> String {
        self.report_where(|recorded| recorded != rule)
    }

    fn report_where(&self, wanted: impl Fn(&str) -> bool) -> String {
        let mut lines: Vec<String> = self
            .violations
            .iter()
            .filter(|violation| wanted(violation.rule))
            .map(Violation::to_string)
            .collect();
        for (rule, count) in self.dropped.iter().filter(|(rule, _)| wanted(rule)) {
            lines.push(format!("... and {count} more {rule} not recorded"));
        }
        lines.join("\n")
    }

    fn check_every_day(&mut self, game: &Game, season: u32, day: Option<u32>) {
        for (rule, check) in EVERY_DAY {
            for message in check(game) {
                // The same fault is on every day after the first, so a daily
                // rule is recorded once, on the day it first appears.
                self.record(season, day, rule, message, true);
            }
        }
    }

    fn record(
        &mut self,
        season: u32,
        day: Option<u32>,
        rule: &'static str,
        message: String,
        dedupe: bool,
    ) {
        if dedupe && !self.seen.insert((rule, message.clone())) {
            return;
        }
        let recorded = self.violations.iter().filter(|v| v.rule == rule).count();
        if recorded >= MAX_RECORDED {
            *self.dropped.entry(rule).or_default() += 1;
            return;
        }
        self.violations.push(Violation {
            season,
            day,
            rule,
            message,
        });
    }
}

impl SeasonObserver for Invariants {
    fn on_day_end(&mut self, game: &Game, season: u32, day: u32) {
        self.check_every_day(game, season, Some(day));
    }

    fn on_before_rollover(&mut self, game: &Game, _season: u32) {
        self.before = Some(Before::capture(game, &self.authored_sizes));
    }

    fn on_season_end(&mut self, game: &Game, _summary: &EndOfSeasonSummary, season: u32) {
        self.check_every_day(game, season, None);
        let Some(before) = self.before.take() else {
            return;
        };
        for (rule, check) in AT_ROLLOVER {
            for message in check(&before, game) {
                self.record(season, None, rule, message, false);
            }
        }
    }
}

// ── What is compared across a rollover ──────────────────────────────────────

/// The parts of the world a rollover replaces, read just before it runs.
struct Before {
    authored_sizes: BTreeMap<String, usize>,
    competitions: BTreeMap<String, CompetitionBefore>,
    /// Which table each club sat in, for countries that run a ladder.
    membership: BTreeMap<String, String>,
    /// Countries whose every domestic table had finished, so the ladder had
    /// everything it needs to move clubs.
    finished_countries: BTreeSet<String>,
}

struct CompetitionBefore {
    season: u32,
    complete: bool,
    world_cup: bool,
}

impl Before {
    fn capture(game: &Game, authored_sizes: &BTreeMap<String, usize>) -> Self {
        let competitions = game
            .competitions
            .iter()
            .map(|competition| {
                (
                    competition.id.clone(),
                    CompetitionBefore {
                        season: competition.season,
                        complete: is_regenerated_at_rollover(competition),
                        world_cup: is_world_cup_competition(competition)
                            || is_world_cup_qualifying(competition),
                    },
                )
            })
            .collect();

        let mut membership = BTreeMap::new();
        let mut finished_countries = BTreeSet::new();
        for (country, tables) in domestic_tables_by_country(game) {
            // A split-season country plays one division twice, so nobody moves
            // between its halves; a one-table country has no ladder to climb.
            if is_split_season_country(country) || tables.len() < 2 {
                continue;
            }
            if tables.iter().all(|table| is_regenerated_at_rollover(table)) {
                finished_countries.insert(country.to_string());
            }
            for table in tables {
                for club in &table.participant_ids {
                    membership.insert(club.clone(), table.id.clone());
                }
            }
        }

        Self {
            authored_sizes: authored_sizes.clone(),
            competitions,
            membership,
            finished_countries,
        }
    }
}

// ── Every day ───────────────────────────────────────────────────────────────

/// Given the world at any moment of a run,
/// When it has no teams, or no players,
/// Then that is reported.
fn world_not_emptied(game: &Game) -> Vec<String> {
    let mut faults = Vec::new();
    if game.teams.is_empty() {
        faults.push("no teams remain".to_string());
    }
    if game.players.is_empty() {
        faults.push("no players remain".to_string());
    }
    faults
}

/// Given any player, staff member or standing,
/// When it names a team that does not exist,
/// Then that is reported, with the id.
fn referential_integrity(game: &Game) -> Vec<String> {
    let teams: BTreeSet<&str> = game.teams.iter().map(|team| team.id.as_str()).collect();
    let mut faults = Vec::new();
    for player in &game.players {
        if let Some(team_id) = player.team_id.as_deref()
            && !teams.contains(team_id)
        {
            faults.push(format!(
                "player {} references unknown team {team_id}",
                player.id
            ));
        }
    }
    for member in &game.staff {
        if let Some(team_id) = member.team_id.as_deref()
            && !teams.contains(team_id)
        {
            faults.push(format!(
                "staff {} references unknown team {team_id}",
                member.id
            ));
        }
    }
    faults
}

/// Given every club's finances,
/// When a balance is within a factor of two of overflowing,
/// Then that is reported.
///
/// Guards against wraparound and overflow, not against a club being poor.
fn finances_in_range(game: &Game) -> Vec<String> {
    let limit = i64::MAX.unsigned_abs() / 2;
    game.teams
        .iter()
        .filter(|team| team.finance.unsigned_abs() >= limit)
        .map(|team| format!("team {} finance out of range: {}", team.id, team.finance))
        .collect()
}

/// Given a day has been played,
/// When a fixture dated before today is still Scheduled,
/// Then that is reported — the live day once skipped every other competition's fixtures (#608), and nothing else noticed.
///
/// Every fixture due since the career began has been played, in every
/// competition and for every national team.
///
/// Only `Scheduled` counts. An `InProgress` fixture belongs to a live-match
/// session that has not finished, and the harness does not start one yet.
///
/// A fixture is only ever due on the exact date, so one that is skipped is
/// skipped for good — and a season can complete with fixtures never played,
/// because completeness only looks forward. Nothing else notices.
///
///
/// Counted from the world's creation too: a fixture dated before the career began that the world
/// builder left unplayed is as stranded as one the turn skipped (#652, #653).
fn no_stranded_fixture(game: &Game) -> Vec<String> {
    let Some(yesterday) = game.clock.current_date.date_naive().pred_opt() else {
        return Vec::new();
    };
    ofm_core::matchday::stranded_fixtures(game, yesterday)
        .into_iter()
        .map(|fixture| {
            format!(
                "{} fixture {} was due {} and is still Scheduled",
                fixture.owner, fixture.fixture_id, fixture.date
            )
        })
        .collect()
}

/// The name `club_rests_between_matches` reports under.
pub const CLUB_RESTS_BETWEEN_MATCHES: &str = "club-rests-between-matches";

/// The maintainer's ruling of 12 Oct 2026: a club's competitive matches are at least this many
/// days apart, so Saturday then Tuesday is legal and Saturday then Monday is a clash.
///
/// Held here rather than read from `ofm_core::calendar_booking`: that module books on a
/// two-date rule and exposes no constant, and the calendar rework is rewriting it. Point this at
/// the game's own constant once it has one.
pub const MIN_DAYS_BETWEEN_MATCHES: i64 = 3;

/// Given the clubs' competitions as they stand,
/// When a club has two competitive fixtures less than [`MIN_DAYS_BETWEEN_MATCHES`] apart,
/// Then that is reported, naming the club, both days and both competitions.
///
/// Friendlies are left out: the calendar drops a friendly that clashes. Read from the career's
/// first day on, like `no_stranded_fixture`, since a world built part-way through a season
/// carries fixtures from before the career that it never played.
fn club_rests_between_matches(game: &Game) -> Vec<String> {
    clashes_since(game, game.clock.start_date.date_naive())
}

/// The clashes among fixtures dated `since` or later.
fn clashes_since(game: &Game, since: NaiveDate) -> Vec<String> {
    let mut matches_by_club: BTreeMap<&str, Vec<(NaiveDate, &str)>> = BTreeMap::new();
    for competition in &game.competitions {
        for fixture in &competition.fixtures {
            let competitive = matches!(
                fixture.competition,
                FixtureCompetition::League
                    | FixtureCompetition::Cup
                    | FixtureCompetition::ContinentalClub
                    | FixtureCompetition::InternationalClub
            );
            let Ok(date) = NaiveDate::parse_from_str(&fixture.date, "%Y-%m-%d") else {
                continue;
            };
            if !competitive || date < since {
                continue;
            }
            for club in [&fixture.home_team_id, &fixture.away_team_id] {
                matches_by_club
                    .entry(club.as_str())
                    .or_default()
                    .push((date, competition.id.as_str()));
            }
        }
    }
    let mut faults = Vec::new();
    for (club, mut matches) in matches_by_club {
        matches.sort_unstable();
        for pair in matches.windows(2) {
            let ((first_day, first), (second_day, second)) = (pair[0], pair[1]);
            let days_apart = (second_day - first_day).num_days();
            if days_apart < MIN_DAYS_BETWEEN_MATCHES {
                faults.push(format!(
                    "{club} plays {first} on {first_day} and {second} on {second_day}, \
                     {days_apart} days apart; at least {MIN_DAYS_BETWEEN_MATCHES} are needed"
                ));
            }
        }
    }
    faults
}

/// Given any club fixture marked Completed on or after the career's first day,
/// When it has no match report,
/// Then that is reported: a match the engine played leaves a report, and one settled by scoreline
///      does not.
///
/// The allowlist is the dormant tier: a competition outside the game's active scope is settled by
/// scoreline today and has no reports. Removing the dormant tier empties the allowlist, at which
/// point this rule holds for every club fixture. National-team tournaments are not club fixtures.
fn completed_club_fixtures_have_reports(game: &Game) -> Vec<String> {
    let began = game.clock.start_date.format("%Y-%m-%d").to_string();
    game.competitions
        .iter()
        .filter(|competition| competition.kind != CompetitionType::InternationalNation)
        .filter(|competition| game.competition_in_active_scope(competition))
        .flat_map(|competition| {
            competition
                .fixtures
                .iter()
                .map(move |fixture| (competition.id.as_str(), fixture))
        })
        .filter(|(_, fixture)| {
            fixture.status == FixtureStatus::Completed
                && fixture.date.as_str() >= began.as_str()
                && fixture
                    .result
                    .as_ref()
                    .is_some_and(|result| result.report.is_none())
        })
        .map(|(owner, fixture)| {
            format!(
                "{owner} fixture {} on {} is Completed with a result but no match report",
                fixture.id, fixture.date
            )
        })
        .collect()
}

/// Given any fixture marked Completed,
/// When it has no result,
/// Then that is reported.
fn completed_fixtures_have_results(game: &Game) -> Vec<String> {
    game.competitions
        .iter()
        .flat_map(|competition| {
            competition
                .fixtures
                .iter()
                .map(move |f| (competition.id.as_str(), f))
        })
        .filter(|(_, fixture)| {
            fixture.status == FixtureStatus::Completed && fixture.result.is_none()
        })
        .map(|(owner, fixture)| {
            format!("{owner} fixture {} is Completed with no result", fixture.id)
        })
        .collect()
}

/// Given any league table,
/// When a row's played or points do not follow from its won, drawn and lost, or goals for and against differ, or a club has two rows or is unknown,
/// Then that is reported.
///
/// A league table adds up. Only league tables: a knockout tie can end level and
/// go to a shootout, which breaks both totals without anything being wrong.
fn standings_are_consistent(game: &Game) -> Vec<String> {
    let teams: BTreeSet<&str> = game.teams.iter().map(|team| team.id.as_str()).collect();
    let mut faults = Vec::new();
    for table in game
        .competitions
        .iter()
        .filter(|competition| competition.rules.format == CompetitionFormat::LeagueTable)
    {
        let mut seen = BTreeSet::new();
        for row in &table.standings {
            if !teams.contains(row.team_id.as_str()) {
                faults.push(format!(
                    "{} has a standing for unknown team {}",
                    table.id, row.team_id
                ));
            }
            if !seen.insert(row.team_id.as_str()) {
                faults.push(format!(
                    "{} has two standings for {}",
                    table.id, row.team_id
                ));
            }
            if row.played != row.won + row.drawn + row.lost {
                faults.push(format!(
                    "{}: {} played != won + drawn + lost",
                    table.id, row.team_id
                ));
            }
            if row.points != row.won * 3 + row.drawn {
                faults.push(format!(
                    "{}: {} points != 3 * won + drawn",
                    table.id, row.team_id
                ));
            }
        }
        let played: u32 = table.standings.iter().map(|row| row.played).sum();
        if !played.is_multiple_of(2) {
            faults.push(format!("{}: total games played is odd", table.id));
        }
        let scored: u32 = table.standings.iter().map(|row| row.goals_for).sum();
        let conceded: u32 = table.standings.iter().map(|row| row.goals_against).sum();
        if scored != conceded {
            faults.push(format!(
                "{}: goals for {scored} != goals against {conceded}",
                table.id
            ));
        }
    }
    faults
}

// ── At each rollover ────────────────────────────────────────────────────────

/// Given a country with a pyramid, before and after a rollover,
/// When a division's size changed, a club is in two leagues, or a country lost its league,
/// Then that is reported.
fn ladder_holds(before: &Before, after: &Game) -> Vec<String> {
    ladder_violations(after, &before.authored_sizes)
}

/// Given the competitions before a rollover,
/// When one is missing afterwards (the World Cup and its qualifying excepted),
/// Then that is reported.
///
/// The World Cup and its qualifying are retired and rebuilt on their own
/// schedule, so they may legitimately go; nothing else may.
fn competitions_survive(before: &Before, after: &Game) -> Vec<String> {
    before
        .competitions
        .iter()
        .filter(|(_, was)| !was.world_cup)
        .filter(|(id, _)| {
            !after
                .competitions
                .iter()
                .any(|competition| &competition.id == *id)
        })
        .map(|(id, _)| format!("{id} disappeared at the rollover"))
        .collect()
}

/// Given a competition the rollover judged complete,
/// When its season is not exactly one more afterwards,
/// Then that is reported (#654).
///
/// A competition the rollover regenerates moves on by exactly one season. Only
/// those the rollover itself judged complete: a league still mid-season on a
/// hemisphere calendar is left as it is, by design.
fn completed_competitions_advance_one_season(before: &Before, after: &Game) -> Vec<String> {
    after
        .competitions
        .iter()
        .filter_map(|competition| {
            let was = before.competitions.get(&competition.id)?;
            (was.complete && !was.world_cup && competition.season != was.season + 1).then(|| {
                format!(
                    "{} was complete in season {} and is now season {}",
                    competition.id, was.season, competition.season
                )
            })
        })
        .collect()
}

/// Given a league table the rollover regenerated,
/// When it does not start from nothing, or does not hold a full double round robin,
/// Then that is reported.
///
/// A league table the rollover regenerated starts from nothing and holds a full
/// double round robin.
fn regenerated_tables_are_fresh(before: &Before, after: &Game) -> Vec<String> {
    let mut faults = Vec::new();
    for table in after
        .competitions
        .iter()
        .filter(|competition| competition.rules.format == CompetitionFormat::LeagueTable)
    {
        let Some(was) = before.competitions.get(&table.id) else {
            continue;
        };
        if table.season <= was.season {
            continue;
        }
        if table.standings.len() != table.participant_ids.len() {
            faults.push(format!(
                "{} has {} standings for {} participants",
                table.id,
                table.standings.len(),
                table.participant_ids.len()
            ));
        }
        if table.standings.iter().any(|row| {
            row.played != 0 || row.points != 0 || row.goals_for != 0 || row.goals_against != 0
        }) {
            faults.push(format!(
                "{} starts its new season with a played standing",
                table.id
            ));
        }
        if !has_full_schedule(table) {
            faults.push(format!(
                "{} has no full double round robin scheduled",
                table.id
            ));
        }
    }
    faults
}

/// Given every player after a rollover,
/// When any season statistic is still carried over,
/// Then that is reported.
fn season_stats_are_reset(_before: &Before, after: &Game) -> Vec<String> {
    let stale: Vec<&str> = after
        .players
        .iter()
        .filter(|player| player.stats != PlayerSeasonStats::default())
        .map(|player| player.id.as_str())
        .collect();
    match stale.first() {
        Some(first) => vec![format!(
            "{} players kept last season's stats, first {first}",
            stale.len()
        )],
        None => Vec::new(),
    }
}

/// Given a country whose every division finished,
/// When no club changed division at the rollover,
/// Then that is reported — promotion and relegation once never ran in any generated world (#555).
///
/// A country whose every division finished moves at least one club between
/// divisions. Promotion and relegation that never run is a bug that leaves the
/// world looking healthy, and it shipped once.
fn finished_ladders_move_clubs(before: &Before, after: &Game) -> Vec<String> {
    let tables = domestic_tables_by_country(after);
    before
        .finished_countries
        .iter()
        .filter_map(|country| {
            let tables = tables.get(country.as_str())?;
            let moved = tables.iter().any(|table| {
                table
                    .participant_ids
                    .iter()
                    .any(|club| before.membership.get(club) != Some(&table.id))
            });
            (!moved)
                .then(|| format!("{country}'s divisions all finished but no club changed division"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    //! Every rule is shown to fail. A rule that has never been seen to fire is
    //! not a guard, it is a comment: these corrupt a real generated world in the
    //! one way each rule exists to catch, and check that it notices, and that it
    //! stays quiet on the world as built.

    use std::sync::OnceLock;

    use super::*;
    use crate::worlds::WorldSpec;

    fn world() -> Game {
        static WORLD: OnceLock<Game> = OnceLock::new();
        WORLD
            .get_or_init(|| WorldSpec::gate(1).build().expect("the world builds"))
            .clone()
    }

    fn before(game: &Game) -> Before {
        Before::capture(game, &division_sizes(game))
    }

    fn competition<'a>(game: &'a mut Game, id: &str) -> &'a mut domain::league::League {
        game.competitions
            .iter_mut()
            .find(|competition| competition.id == id)
            .unwrap_or_else(|| panic!("{id} should exist"))
    }

    fn mentions(faults: &[String], needle: &str) -> bool {
        faults.iter().any(|fault| fault.contains(needle))
    }

    #[test]
    fn every_daily_rule_is_quiet_on_the_world_as_built() {
        let game = world();
        for (rule, check) in EVERY_DAY {
            // The built world books clubs inside the rest (#553). The rule's own tests below
            // prove it looks; the gate holds the world to it.
            if *rule == CLUB_RESTS_BETWEEN_MATCHES {
                assert!(
                    !check(&game).is_empty(),
                    "#553 is fixed: delete this exemption (the built world has no clash)"
                );
                continue;
            }
            assert_eq!(
                check(&game),
                Vec::<String>::new(),
                "{rule} fired on a clean world"
            );
        }
        assert_eq!(
            ladder_violations(&game, &division_sizes(&game)),
            Vec::<String>::new()
        );
    }

    #[test]
    fn an_emptied_world_is_noticed() {
        let mut game = world();
        game.players.clear();
        assert!(mentions(&world_not_emptied(&game), "no players remain"));

        let mut game = world();
        game.teams.clear();
        assert!(mentions(&world_not_emptied(&game), "no teams remain"));
    }

    #[test]
    fn a_player_on_a_team_that_does_not_exist_is_noticed() {
        let mut game = world();
        game.players[0].team_id = Some("ghost".to_string());
        assert!(mentions(
            &referential_integrity(&game),
            "unknown team ghost"
        ));
    }

    #[test]
    fn a_finance_at_the_bottom_of_the_range_is_noticed_not_a_panic() {
        let mut game = world();
        game.teams[0].finance = i64::MIN;
        assert!(mentions(&finances_in_range(&game), "out of range"));
    }

    #[test]
    fn a_finance_that_has_wrapped_is_noticed() {
        let mut game = world();
        game.teams[0].finance = i64::MAX;
        assert!(mentions(&finances_in_range(&game), "out of range"));
    }

    #[test]
    fn a_fixture_left_behind_by_the_calendar_is_noticed() {
        let mut game = world();
        game.clock.advance_days(10);
        let faults = no_stranded_fixture(&game);
        assert!(mentions(&faults, "still Scheduled"), "{faults:?}");
    }

    /// Given one league, and a club in it,
    /// When a cup fixture is put on a day that club already plays in the league,
    /// Then the club is reported as playing twice that day — and is not before.
    ///
    /// The world is cut down to a single competition first so the control is
    /// real: the built world has double bookings of its own (#553), and a rule
    /// that "passes" there could not be told from one that does not look.
    #[test]
    fn a_club_booked_twice_on_one_day_is_noticed() {
        let mut game = world();
        game.competitions
            .retain(|competition| competition.id == "eng-d1");
        game.competitions[0]
            .fixtures
            .retain(|fixture| fixture.competition == FixtureCompetition::League);
        assert!(
            club_rests_between_matches(&game).is_empty(),
            "a single round-robin never books a club twice inside the rest"
        );

        let league_fixture = game.competitions[0].fixtures[0].clone();
        let mut clash = league_fixture.clone();
        clash.id = "a-cup-tie-on-the-same-day".to_string();
        clash.competition = FixtureCompetition::Cup;
        game.competitions[0].fixtures.push(clash);

        let faults = club_rests_between_matches(&game);
        assert!(
            mentions(
                &faults,
                &format!(
                    "{} plays eng-d1 on {}",
                    league_fixture.home_team_id, league_fixture.date
                )
            ),
            "{faults:?}"
        );
    }

    /// The rule reads from the career's first day on. This asserts that fixtures from before it do
    /// clash in the built world, so the day they stop, it fails and says to delete the exemption.
    #[test]
    fn the_built_world_clashes_before_the_career_began_too() {
        let game = world();
        let in_career = club_rests_between_matches(&game).len();
        let ever = clashes_since(&game, NaiveDate::MIN).len();
        assert!(
            ever > in_career,
            "#553 is fixed before the career too: delete the pre-career exemption in club_rests_between_matches"
        );
    }

    /// Fixtures dated before the career began are the world builder's, not the
    /// turn's, and the rule reads from the first day on (see its doc).
    #[test]
    fn a_clash_dated_before_the_career_began_is_not_this_rules_business() {
        let mut game = world();
        game.competitions
            .retain(|competition| competition.id == "eng-d1");
        game.competitions[0]
            .fixtures
            .retain(|fixture| fixture.competition == FixtureCompetition::League);
        let mut clash = game.competitions[0].fixtures[0].clone();
        clash.id = "a-cup-tie-from-before-the-career".to_string();
        clash.competition = FixtureCompetition::Cup;
        game.competitions[0].fixtures[0].date = "2033-01-01".to_string();
        clash.date = "2033-01-01".to_string();
        game.competitions[0].fixtures.push(clash);

        assert!(club_rests_between_matches(&game).is_empty());
    }

    /// Given a club's league fixture and a cup tie for the same club,
    /// When the cup tie is `MIN_DAYS_BETWEEN_MATCHES - 1` days later, then it is a clash;
    ///      when it is `MIN_DAYS_BETWEEN_MATCHES` days later, it is not.
    #[test]
    fn two_matches_are_a_clash_until_the_minimum_rest_has_passed() {
        let tie_after = |days: i64| {
            let mut game = world();
            game.competitions
                .retain(|competition| competition.id == "eng-d1");
            game.competitions[0]
                .fixtures
                .retain(|fixture| fixture.competition == FixtureCompetition::League);
            let league_fixture = game.competitions[0].fixtures[0].clone();
            let day = NaiveDate::parse_from_str(&league_fixture.date, "%Y-%m-%d").unwrap();
            let mut tie = league_fixture;
            tie.id = "a-cup-tie".to_string();
            tie.competition = FixtureCompetition::Cup;
            tie.date = (day + chrono::Days::new(days.unsigned_abs()))
                .format("%Y-%m-%d")
                .to_string();
            game.competitions[0].fixtures.push(tie);
            club_rests_between_matches(&game)
        };

        let too_soon = tie_after(MIN_DAYS_BETWEEN_MATCHES - 1);
        assert!(mentions(&too_soon, "2 days apart"), "{too_soon:?}");
        assert!(tie_after(MIN_DAYS_BETWEEN_MATCHES).is_empty());
    }

    /// A friendly on a league day is how pre-season and the international
    /// breaks work, and is not what this rule is about.
    #[test]
    fn a_friendly_on_a_league_day_is_not_a_double_booking() {
        let mut game = world();
        game.competitions
            .retain(|competition| competition.id == "eng-d1");
        game.competitions[0]
            .fixtures
            .retain(|fixture| fixture.competition == FixtureCompetition::League);
        let mut friendly = game.competitions[0].fixtures[0].clone();
        friendly.id = "a-friendly".to_string();
        friendly.competition = FixtureCompetition::Friendly;
        game.competitions[0].fixtures.push(friendly);

        assert!(club_rests_between_matches(&game).is_empty());
    }

    #[test]
    fn a_completed_fixture_without_a_result_is_noticed() {
        let mut game = world();
        competition(&mut game, "eng-d1").fixtures[0].status = FixtureStatus::Completed;
        assert!(mentions(
            &completed_fixtures_have_results(&game),
            "no result"
        ));
    }

    #[test]
    fn a_standing_that_does_not_add_up_is_noticed() {
        let mut game = world();
        competition(&mut game, "eng-d1").standings[0].played = 3;
        let faults = standings_are_consistent(&game);
        assert!(
            mentions(&faults, "played != won + drawn + lost"),
            "{faults:?}"
        );
        assert!(mentions(&faults, "total games played is odd"), "{faults:?}");
    }

    #[test]
    fn a_country_that_loses_its_league_is_noticed_at_the_rollover() {
        let game = world();
        let was = before(&game);
        let mut after = game.clone();
        after
            .competitions
            .retain(|competition| competition.country_id.as_deref() != Some("PT"));
        assert!(mentions(
            &ladder_holds(&was, &after),
            "PT has clubs but no league table"
        ));
    }

    #[test]
    fn a_competition_that_vanishes_at_the_rollover_is_noticed() {
        let game = world();
        let was = before(&game);
        let mut after = game.clone();
        after
            .competitions
            .retain(|competition| competition.id != "eng-cup");
        assert!(mentions(
            &competitions_survive(&was, &after),
            "eng-cup disappeared"
        ));
    }

    #[test]
    fn a_completed_competition_must_advance_by_exactly_one_season() {
        let game = world();
        let was = before(&game);

        // Left where it was, and jumped past a season: both wrong.
        assert!(mentions(
            &completed_competitions_advance_one_season(&was, &game),
            "eng-cup was complete in season 2033 and is now season 2033"
        ));
        let mut skipped = game.clone();
        competition(&mut skipped, "eng-cup").season += 2;
        assert!(mentions(
            &completed_competitions_advance_one_season(&was, &skipped),
            "eng-cup was complete in season 2033 and is now season 2035"
        ));

        // One season on is right.
        let mut advanced = game.clone();
        competition(&mut advanced, "eng-cup").season += 1;
        assert!(!mentions(
            &completed_competitions_advance_one_season(&was, &advanced),
            "eng-cup"
        ));
    }

    #[test]
    fn a_league_still_mid_season_is_exempt_from_advancing() {
        // A league the rollover judged unfinished is left as it is, by design, so
        // it must not be held to advancing. Mark one as started and unfinished.
        let mut game = world();
        let table = competition(&mut game, "eng-d1");
        table.fixtures[0].status = FixtureStatus::Completed;
        assert!(
            !is_regenerated_at_rollover(table),
            "one played, the rest still to come"
        );
        let was = before(&game);
        let faults = completed_competitions_advance_one_season(&was, &game);
        assert!(!mentions(&faults, "eng-d1"), "{faults:?}");
        assert!(
            mentions(&faults, "eng-cup"),
            "the same world must still flag a competition that did complete, or this proves nothing: {faults:?}"
        );
    }

    #[test]
    fn a_regenerated_table_that_starts_with_played_games_is_noticed() {
        let game = world();
        let was = before(&game);
        let mut after = game.clone();
        let table = competition(&mut after, "eng-d1");
        table.season += 1;
        table.standings[0].played = 1;
        let faults = regenerated_tables_are_fresh(&was, &after);
        assert!(
            mentions(&faults, "starts its new season with a played standing"),
            "{faults:?}"
        );
    }

    #[test]
    fn stats_carried_over_the_rollover_are_noticed() {
        let game = world();
        let was = before(&game);
        let mut after = game.clone();
        after.players[0].stats.goals = 4;
        assert!(mentions(
            &season_stats_are_reset(&was, &after),
            "kept last season's stats"
        ));
    }

    #[test]
    fn a_finished_ladder_that_moves_nobody_is_noticed() {
        let game = world();
        let mut was = before(&game);
        was.finished_countries.insert("ENG".to_string());

        assert!(mentions(
            &finished_ladders_move_clubs(&was, &game),
            "ENG's divisions all finished but no club changed division"
        ));

        // Swap a club between the two divisions: now somebody moved.
        let mut moved = game.clone();
        let promoted = competition(&mut moved, "eng-d2").participant_ids[0].clone();
        let demoted = competition(&mut moved, "eng-d1").participant_ids[0].clone();
        competition(&mut moved, "eng-d2").participant_ids[0] = demoted;
        competition(&mut moved, "eng-d1").participant_ids[0] = promoted;
        assert!(finished_ladders_move_clubs(&was, &moved).is_empty());
    }

    #[test]
    fn a_standing_whose_points_do_not_follow_its_results_is_noticed() {
        let mut game = world();
        competition(&mut game, "eng-d1").standings[0].points = 7;
        let faults = standings_are_consistent(&game);
        assert!(mentions(&faults, "points != 3 * won + drawn"), "{faults:?}");
    }

    #[test]
    fn a_standing_for_a_team_that_does_not_exist_is_noticed() {
        let mut game = world();
        competition(&mut game, "eng-d1").standings[0].team_id = "ghost".to_string();
        let faults = standings_are_consistent(&game);
        assert!(
            mentions(&faults, "standing for unknown team ghost"),
            "{faults:?}"
        );
    }

    #[test]
    fn a_regenerated_table_with_a_standing_missing_is_noticed() {
        let game = world();
        let was = before(&game);
        let mut after = game.clone();
        let table = competition(&mut after, "eng-d1");
        table.season += 1;
        table.standings.pop();
        let faults = regenerated_tables_are_fresh(&was, &after);
        assert!(mentions(&faults, "standings for"), "{faults:?}");
    }

    #[test]
    fn two_standings_for_one_team_are_noticed() {
        let mut game = world();
        let table = competition(&mut game, "eng-d1");
        let again = table.standings[0].clone();
        table.standings.push(again);
        let faults = standings_are_consistent(&game);
        assert!(mentions(&faults, "has two standings for"), "{faults:?}");
    }

    #[test]
    fn goals_that_are_scored_but_never_conceded_are_noticed() {
        let mut game = world();
        competition(&mut game, "eng-d1").standings[0].goals_for = 3;
        let faults = standings_are_consistent(&game);
        assert!(
            mentions(&faults, "goals for 3 != goals against 0"),
            "{faults:?}"
        );
    }

    #[test]
    fn a_staff_member_on_a_team_that_does_not_exist_is_noticed() {
        let mut game = world();
        game.staff[0].team_id = Some("ghost".to_string());
        assert!(mentions(&referential_integrity(&game), "staff"));
        assert!(mentions(
            &referential_integrity(&game),
            "unknown team ghost"
        ));
    }

    #[test]
    fn the_world_cup_may_be_retired_at_the_rollover_but_nothing_else_may() {
        let game = world();
        let was = before(&game);
        let world_cup: Vec<String> = game
            .competitions
            .iter()
            .filter(|competition| {
                is_world_cup_competition(competition) || is_world_cup_qualifying(competition)
            })
            .map(|competition| competition.id.clone())
            .collect();
        assert!(
            !world_cup.is_empty(),
            "a 2033 start has World Cup qualifying under way, so there is one to retire"
        );

        let mut after = game.clone();
        after.competitions.retain(|competition| {
            !world_cup.contains(&competition.id) && competition.id != "eng-cup"
        });

        let faults = competitions_survive(&was, &after);
        assert!(mentions(&faults, "eng-cup disappeared"), "{faults:?}");
        for id in &world_cup {
            assert!(
                !mentions(&faults, id),
                "{id} may legitimately go: {faults:?}"
            );
        }
    }

    #[test]
    fn a_regenerated_table_missing_part_of_its_schedule_is_noticed() {
        let game = world();
        let was = before(&game);
        let mut after = game.clone();
        let table = competition(&mut after, "eng-d1");
        table.season += 1;
        table.fixtures.pop();
        let faults = regenerated_tables_are_fresh(&was, &after);
        assert!(
            mentions(&faults, "no full double round robin"),
            "{faults:?}"
        );
    }

    #[test]
    fn only_a_country_with_a_ladder_whose_every_division_finished_counts_as_finished() {
        let mut game = world();
        assert!(
            before(&game).finished_countries.is_empty(),
            "nothing has been played yet"
        );

        for id in ["eng-d1", "eng-d2"] {
            for fixture in &mut competition(&mut game, id).fixtures {
                fixture.status = FixtureStatus::Completed;
            }
        }
        // Portugal has one table and Argentina one division played twice: neither
        // has a ladder to climb, finished or not.
        for id in ["pt-d1", "ar-d1-apertura", "ar-d1-clausura"] {
            for fixture in &mut competition(&mut game, id).fixtures {
                fixture.status = FixtureStatus::Completed;
            }
        }

        let finished = before(&game).finished_countries;
        assert_eq!(
            finished.iter().map(String::as_str).collect::<Vec<_>>(),
            ["ENG"]
        );
    }

    #[test]
    fn a_daily_fault_is_recorded_once_however_long_it_lasts() {
        let mut game = world();
        game.players[0].team_id = Some("ghost".to_string());
        let mut invariants = Invariants::new(&game);
        let recorded = invariants.violations().len();
        assert!(recorded > 0, "the corrupted world starts with the fault");

        for day in 1..=5 {
            invariants.on_day_end(&game, 1, day);
        }

        assert_eq!(invariants.violations().len(), recorded);
    }

    /// Given one rule that fails more often than the cap and another that fails once,
    /// When both are recorded,
    /// Then the flood is cut at the cap with a count of the rest, and the other rule is still kept.
    #[test]
    fn a_rule_that_fails_everywhere_is_capped_without_hiding_another() {
        let mut invariants = Invariants::new(&world());

        for n in 0..MAX_RECORDED + 3 {
            invariants.record(1, None, "flood", format!("fault {n}"), false);
        }
        invariants.record(1, None, "quiet", "one fault".to_string(), false);

        let flood = invariants.report_of("flood");
        assert_eq!(flood.lines().count(), MAX_RECORDED + 1);
        assert!(
            flood.ends_with("... and 3 more flood not recorded"),
            "{flood}"
        );
        assert!(invariants.report_of("quiet").contains("one fault"));
    }

    fn an_empty_report() -> domain::league::CompactMatchReport {
        let side = domain::league::CompactTeamMatchStats {
            possession_pct: 50,
            shots: 0,
            shots_on_target: 0,
            fouls: 0,
            corners: 0,
            yellow_cards: 0,
            red_cards: 0,
        };
        domain::league::CompactMatchReport {
            total_minutes: 90,
            home_stats: side.clone(),
            away_stats: side,
            events: Vec::new(),
        }
    }

    /// Completes the first scheduled club fixture on `day` with a result and no report, and
    /// returns the competition and fixture it was.
    fn complete_a_club_fixture_without_report(game: &mut Game, day: &str) -> (String, String) {
        let competition = game
            .competitions
            .iter_mut()
            .filter(|competition| competition.kind != CompetitionType::InternationalNation)
            .find(|competition| {
                competition
                    .fixtures
                    .iter()
                    .any(|fixture| fixture.status == FixtureStatus::Scheduled)
            })
            .expect("the world has a club fixture to play");
        let fixture = competition
            .fixtures
            .iter_mut()
            .find(|fixture| fixture.status == FixtureStatus::Scheduled)
            .expect("checked above");
        fixture.status = FixtureStatus::Completed;
        fixture.date = day.to_string();
        fixture.result = Some(domain::league::MatchResult {
            home_goals: 1,
            away_goals: 0,
            ..Default::default()
        });
        (competition.id.clone(), fixture.id.clone())
    }

    /// Given a club fixture played during the career with a result and no report,
    /// When the rule reads the world,
    /// Then the fixture is named; with a report it is not.
    #[test]
    fn a_played_club_fixture_without_a_report_is_noticed() {
        let mut game = world();
        // Every competition active, so the fixture cannot be on the dormant allowlist.
        game.active_competition_ids.clear();
        game.active_region_ids.clear();
        let day = game.clock.current_date.format("%Y-%m-%d").to_string();
        let (owner, fixture_id) = complete_a_club_fixture_without_report(&mut game, &day);

        let faults = completed_club_fixtures_have_reports(&game);
        assert!(
            mentions(&faults, &format!("{owner} fixture {fixture_id}")),
            "{faults:?}"
        );

        let fixture = game
            .competitions
            .iter_mut()
            .filter(|competition| competition.id == owner)
            .flat_map(|competition| competition.fixtures.iter_mut())
            .find(|fixture| fixture.id == fixture_id)
            .unwrap();
        fixture.result.as_mut().unwrap().report = Some(an_empty_report());
        assert!(completed_club_fixtures_have_reports(&game).is_empty());
    }

    /// Given a fixture completed without a report in a competition outside the active scope,
    /// When the rule reads the world,
    /// Then it is allowed, as the dormant tier settles by scoreline; with no scope it is not.
    #[test]
    fn a_dormant_competition_is_on_the_allowlist() {
        let mut game = world();
        let day = game.clock.current_date.format("%Y-%m-%d").to_string();
        complete_a_club_fixture_without_report(&mut game, &day);
        game.active_competition_ids = vec!["some-other-competition".to_string()];
        assert!(completed_club_fixtures_have_reports(&game).is_empty());

        game.active_competition_ids.clear();
        game.active_region_ids.clear();
        assert!(!completed_club_fixtures_have_reports(&game).is_empty());
    }

    /// Given a fixture completed without a report before the career began,
    /// When the rule reads the world,
    /// Then it is not this rule's business (the world builder's catch-up settles those).
    #[test]
    fn a_fixture_from_before_the_career_is_not_this_rules_business() {
        let mut game = world();
        complete_a_club_fixture_without_report(&mut game, "2000-01-01");
        assert!(completed_club_fixtures_have_reports(&game).is_empty());
    }
}
