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

use domain::league::{CompetitionFormat, FixtureCompetition, FixtureStatus};
use domain::player::PlayerSeasonStats;
use ofm_core::end_of_season::{EndOfSeasonSummary, has_full_schedule, is_competition_complete};
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
    ("standings-are-consistent", standings_are_consistent),
    ("club-plays-once-a-day", club_plays_once_a_day),
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

/// Enough to name what is wrong; not enough for one systemic failure to bury a
/// report under thousands of copies of itself.
const MAX_RECORDED: usize = 200;

/// The invariants observer: attach it to a run, then read [`Self::violations`].
pub struct Invariants {
    authored_sizes: BTreeMap<String, usize>,
    before: Option<Before>,
    violations: Vec<Violation>,
    seen: BTreeSet<(&'static str, String)>,
    dropped: usize,
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
            dropped: 0,
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
        self.violations.is_empty() && self.dropped == 0
    }

    /// Every violation, one per line, for a failed assertion to print.
    pub fn report(&self) -> String {
        let mut lines: Vec<String> = self.violations.iter().map(Violation::to_string).collect();
        if self.dropped > 0 {
            lines.push(format!("... and {} more not recorded", self.dropped));
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
        if self.violations.len() >= MAX_RECORDED {
            self.dropped += 1;
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
                        complete: is_competition_complete(competition),
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
            if tables.iter().all(|table| is_competition_complete(table)) {
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
    let limit = i64::MAX / 2;
    game.teams
        .iter()
        .filter(|team| team.finance.abs() >= limit)
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
/// Counted from the day the career began, not from the start of time: the world
/// builder's own catch-up leaves fixtures dated before the clock unplayed (the
/// later rounds of a cup that ended months earlier, and the pre-season
/// friendlies of a league that started before the game did). That is a defect of
/// world creation, not of the turn, and folding it in here would make every run
/// fail before a day was played and hide what this rule is for.
fn no_stranded_fixture(game: &Game) -> Vec<String> {
    let began = game.clock.start_date.format("%Y-%m-%d").to_string();
    let Some(yesterday) = game.clock.current_date.pred_opt() else {
        return Vec::new();
    };
    ofm_core::matchday::stranded_fixtures(game, yesterday)
        .into_iter()
        .filter(|fixture| fixture.date.as_str() >= began.as_str())
        .map(|fixture| {
            format!(
                "{} fixture {} was due {} and is still Scheduled",
                fixture.owner, fixture.fixture_id, fixture.date
            )
        })
        .collect()
}

/// Given the clubs' competitions as they stand,
/// When any club is down to play two competitive fixtures on one day,
/// Then that is reported, naming the club, the day and both competitions.
///
/// A player cannot play two matches at once, so one of the two is being played by
/// a squad that is not the one the table says played it. Friendlies are left out:
/// they fill the days competitions leave free, and the international windows.
///
/// Read from the career's first day on, like `no_stranded_fixture`, since a
/// world that was built part-way through a season carries fixtures from before
/// the career that it never played.
fn club_plays_once_a_day(game: &Game) -> Vec<String> {
    let began = game.clock.start_date.format("%Y-%m-%d").to_string();
    let mut booked: BTreeMap<(&str, &str), Vec<&str>> = BTreeMap::new();
    for competition in &game.competitions {
        for fixture in &competition.fixtures {
            let competitive = matches!(
                fixture.competition,
                FixtureCompetition::League
                    | FixtureCompetition::Cup
                    | FixtureCompetition::ContinentalClub
                    | FixtureCompetition::InternationalClub
            );
            if !competitive || fixture.date.as_str() < began.as_str() {
                continue;
            }
            for club in [&fixture.home_team_id, &fixture.away_team_id] {
                booked
                    .entry((club.as_str(), fixture.date.as_str()))
                    .or_default()
                    .push(competition.id.as_str());
            }
        }
    }
    booked
        .into_iter()
        .filter(|(_, competitions)| competitions.len() > 1)
        .map(|((club, date), competitions)| {
            format!(
                "{club} plays {} competitive fixtures on {date}: {}",
                competitions.len(),
                competitions.join(", ")
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
            // The built world books clubs twice in a day (#553). The rule's own
            // tests below prove it looks; the gate is what holds the world to it.
            if *rule == "club-plays-once-a-day" {
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

    /// The exemption is real and has a cost, so it is pinned rather than
    /// assumed: the world builder really does leave fixtures from before the
    /// career unplayed (#652, #653), and the rule must not count them.
    ///
    /// When those are fixed this fails on its first assertion. That is the
    /// signal to delete the exemption in `no_stranded_fixture` — so that the
    /// rule guards world creation too — and then this test.
    #[test]
    fn a_fixture_dated_before_the_career_began_is_not_this_rules_business() {
        let game = world();
        let began = game.clock.start_date.format("%Y-%m-%d").to_string();
        let carried_in = game
            .competitions
            .iter()
            .flat_map(|competition| competition.fixtures.iter())
            .filter(|fixture| {
                fixture.status == FixtureStatus::Scheduled && fixture.date.as_str() < began.as_str()
            })
            .count();

        assert!(
            carried_in > 0,
            "the built world no longer carries unplayed fixtures from before the career: \
             remove the exemption in no_stranded_fixture and this test"
        );
        assert!(no_stranded_fixture(&game).is_empty());
    }

    /// Given one league, and a club in it,
    /// When a cup fixture is put on a day that club already plays in the league,
    /// Then the club is reported as playing twice that day — and is not before.
    ///
    /// The world is cut down to a single competition first so the control is
    /// real: the built world has double bookings of its own (#553), and a rule
    /// that "passes" there could not be told from one that does not look.
    #[test]
    fn a_club_booked_for_two_competitive_fixtures_on_one_day_is_noticed() {
        let mut game = world();
        game.competitions
            .retain(|competition| competition.id == "eng-d1");
        game.competitions[0]
            .fixtures
            .retain(|fixture| fixture.competition == FixtureCompetition::League);
        assert!(
            club_plays_once_a_day(&game).is_empty(),
            "a single round-robin never books a club twice on a day"
        );

        let league_fixture = game.competitions[0].fixtures[0].clone();
        let mut clash = league_fixture.clone();
        clash.id = "a-cup-tie-on-the-same-day".to_string();
        clash.competition = FixtureCompetition::Cup;
        game.competitions[0].fixtures.push(clash);

        let faults = club_plays_once_a_day(&game);
        assert!(
            mentions(
                &faults,
                &format!(
                    "{} plays 2 competitive fixtures on {}",
                    league_fixture.home_team_id, league_fixture.date
                )
            ),
            "{faults:?}"
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

        assert!(club_plays_once_a_day(&game).is_empty());
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

        assert!(club_plays_once_a_day(&game).is_empty());
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
            !is_competition_complete(table),
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
}
