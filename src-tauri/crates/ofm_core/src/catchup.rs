//! Mid-season catch-up simulation.
//!
//! When a game starts and a competition's season began before the game's
//! anchor date (July 1), this module fills in the missing matchdays with
//! quick scoreline-only results so the player joins a living, in-progress
//! season rather than a blank table.

use crate::game::Game;
use chrono::{DateTime, NaiveDate, Utc};
use domain::league::{CompetitionType, FixtureStatus, League, MatchResult};
use domain::player::Player;
use rand::Rng;

const CATCHUP_XI: usize = 11;

/// Average OVR of a club's best XI, used as scoreline strength. Falls back to a
/// neutral rating when the club has no players on the books.
pub(crate) fn club_strength(players: &[Player], club_id: &str) -> f64 {
    let mut ovrs: Vec<u8> = players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some(club_id))
        .map(|p| p.ovr)
        .collect();
    if ovrs.is_empty() {
        return 50.0;
    }
    ovrs.sort_unstable_by(|a, b| b.cmp(a));
    let count = ovrs.len().min(CATCHUP_XI);
    let total: u32 = ovrs.iter().take(count).map(|&o| u32::from(o)).sum();
    total as f64 / count as f64
}

/// Settle one fixture of `competition` by scoreline alone: a score drawn from
/// the two clubs' strength, a shootout if a knockout tie is level, and the
/// result applied to the fixture, the table and any bracket. How the dormant
/// tier plays every fixture, and how an active one is settled when a side
/// cannot be fielded at all.
pub(crate) fn resolve_fixture_by_scoreline(
    players: &[Player],
    competition: &mut League,
    fixture_index: usize,
    rng: &mut impl rand::Rng,
) {
    resolve_fixture_with_strengths(
        competition,
        fixture_index,
        |team_id| club_strength(players, team_id),
        rng,
    );
}

/// [`resolve_fixture_by_scoreline`], with each club's strength read from
/// `strength_of` rather than recounted from the players: the one resolver,
/// for a caller that settles many fixtures and has the strengths to hand.
fn resolve_fixture_with_strengths(
    competition: &mut League,
    fixture_index: usize,
    strength_of: impl Fn(&str) -> f64,
    rng: &mut impl rand::Rng,
) {
    let Some(fixture) = competition.fixtures.get(fixture_index) else {
        return;
    };
    let (fixture_id, home_team_id, away_team_id) = (
        fixture.id.clone(),
        fixture.home_team_id.clone(),
        fixture.away_team_id.clone(),
    );
    let home_strength = strength_of(&home_team_id);
    let away_strength = strength_of(&away_team_id);
    let (home_goals, away_goals) =
        crate::national_team::simulate_scoreline(home_strength, away_strength, rng);
    // Level knockout ties are settled by a simulated shootout so the bracket
    // advances with a real winner instead of defaulting to home.
    let penalties = (home_goals == away_goals && competition.is_knockout_fixture(&fixture_id))
        .then(|| crate::national_team::simulate_shootout(home_strength, away_strength, rng));
    apply_simulated_result(
        competition,
        fixture_index,
        &home_team_id,
        &away_team_id,
        home_goals,
        away_goals,
        penalties,
    );
}

/// Apply a pre-computed scoreline to a fixture, updating standings and
/// advancing group/knockout state. The last step of the one scoreline resolver.
/// `penalties` carries a simulated shootout score for level knockout ties so
/// the round advances with a real winner instead of defaulting to home.
fn apply_simulated_result(
    competition: &mut League,
    fixture_index: usize,
    home_team_id: &str,
    away_team_id: &str,
    home_goals: u8,
    away_goals: u8,
    penalties: Option<(u8, u8)>,
) {
    let fixture = &mut competition.fixtures[fixture_index];
    fixture.status = FixtureStatus::Completed;
    let counts = fixture.counts_for_league_standings();
    fixture.result = Some(MatchResult {
        home_goals,
        away_goals,
        home_scorers: Vec::new(),
        away_scorers: Vec::new(),
        report: None,
        home_penalties: penalties.map(|(home, _)| home),
        away_penalties: penalties.map(|(_, away)| away),
    });
    if counts {
        if let Some(entry) = competition
            .standings
            .iter_mut()
            .find(|e| e.team_id == home_team_id)
        {
            entry.record_result(home_goals, away_goals);
        }
        if let Some(entry) = competition
            .standings
            .iter_mut()
            .find(|e| e.team_id == away_team_id)
        {
            entry.record_result(away_goals, home_goals);
        }
    }
    crate::group_stage::process_completed_fixture(competition, fixture_index);
    crate::schedule::advance_knockout_competition_round(competition);
}

/// Simulate all fixtures in `competition` whose date is before `cutoff`,
/// filling in random scorelines and updating standings. Called once at
/// new-game creation for leagues that started before the game's anchor date.
///
/// Returns how many fixtures it resolved. Note the due list is computed **once**, before any of it
/// is played, while resolving a knockout round *seeds the next one* — dated later, but often still
/// before `cutoff`. So one call advances a bracket by exactly one round. A caller that needs the
/// whole bracket resolved must call again while the count is non-zero; see
/// [`repair_stranded_fixtures`], which does.
pub fn simulate_past_fixtures(
    competition: &mut League,
    players: &[Player],
    cutoff: DateTime<Utc>,
    rng: &mut impl Rng,
) -> usize {
    let cutoff_date = cutoff.date_naive();

    // Precompute strength once per participant to avoid O(fixtures × players).
    let strengths: std::collections::HashMap<String, f64> = competition
        .participant_ids
        .iter()
        .map(|id| (id.clone(), club_strength(players, id)))
        .collect();

    let due: Vec<usize> = competition
        .fixtures
        .iter()
        .enumerate()
        .filter(|(_, f)| {
            f.status == FixtureStatus::Scheduled
                && NaiveDate::parse_from_str(&f.date, "%Y-%m-%d")
                    .map(|d| d < cutoff_date)
                    .unwrap_or(false)
        })
        .map(|(i, _)| i)
        .collect();

    let resolved = due.len();
    for idx in due {
        resolve_fixture_with_strengths(
            competition,
            idx,
            |team_id| strengths.get(team_id).copied().unwrap_or(50.0),
            rng,
        );
    }

    resolved
}

/// Resolve fixtures left `Scheduled` with a date that has already passed.
///
/// Called on every save load. Distinct from [`simulate_past_fixtures`], which runs once at career
/// creation to fill in a season that began before the game's anchor date — this is a repair for
/// fixtures that *should* have been played during the career and were not.
///
/// It exists because #608 skipped whole days: on any day the player watched their own match, every
/// other competition's fixtures due that day were never simulated. Nothing picks them up afterwards,
/// because every "due today" check matches on `date == today` — so the table stays permanently
/// short, a knockout bracket stalls with no winner, and `is_season_complete` does not even notice,
/// since it only asks about fixtures still to come.
///
/// Deliberately unconditional rather than gated on a save-format version. This is a data-integrity
/// invariant — no fixture stays `Scheduled` in the past — not a one-time migration, so it also heals
/// any future regression of the same class. It is idempotent: once repaired there is nothing to
/// find, and the count it returns is zero, so a load does not keep rewriting the save.
///
/// `InProgress` is left alone: a save taken mid-match may have a resumable session, and resolving it
/// by scoreline would discard it.
///
/// A stranded knockout round seeds the next one, so the repair iterates. The cap is a backstop
/// against a fixture that cannot be resolved at all — the loop normally ends because a pass moved
/// nothing. Sixty-four rounds is past any real bracket (a 64-team cup is six).
const MAX_REPAIR_PASSES: usize = 64;

/// The last day that counts as "already passed" when `today` is the clock's day.
fn yesterday(today: NaiveDate) -> NaiveDate {
    today.pred_opt().unwrap_or(today)
}

/// National-team football, which the club match engine never simulates — the same test
/// `turn::competition_indices_due_today` applies when it decides what a day's club sweep may touch.
fn is_national_team_competition(competition: &League) -> bool {
    competition.kind == CompetitionType::InternationalNation
}

fn stranded_in(competition: &League, today: NaiveDate) -> usize {
    competition
        .fixtures
        .iter()
        .filter(|fixture| crate::matchday::is_stranded(fixture, yesterday(today)))
        .count()
}

/// Every fixture the repair is responsible for: the competitions, **and** the window fixtures that
/// live on `game.national_teams` rather than in `competitions` at all. Leaving the latter out of the
/// count is how they stayed stranded — the early return saw nothing to do.
fn count_stranded(game: &Game, today: NaiveDate) -> usize {
    crate::matchday::stranded_fixtures(game, yesterday(today)).len()
}

/// The past dates on which a national-team fixture is still `Scheduled`, oldest first.
///
/// The national-team processors select by `date == today`, so replaying them means handing each
/// stranded date back in turn. Oldest first because a group stage decides who is in the knockout
/// round that follows it.
fn stranded_international_dates(game: &Game, today: NaiveDate) -> Vec<String> {
    let mut dates: Vec<String> = game
        .competitions
        .iter()
        .filter(|competition| is_national_team_competition(competition))
        .flat_map(|competition| competition.fixtures.iter())
        .chain(
            game.national_teams
                .iter()
                .flat_map(|team| team.fixtures.iter()),
        )
        .filter(|fixture| crate::matchday::is_stranded(fixture, yesterday(today)))
        .map(|fixture| fixture.date.clone())
        .collect();
    dates.sort();
    dates.dedup();
    dates
}

/// One repair pass. Returns how many fixtures it resolved; zero means nothing moved.
fn repair_one_pass(game: &mut Game, today: NaiveDate, pass: usize) -> usize {
    let mut resolved = 0;

    // Club football: a scoreline sweep per competition, skipping the ones with nothing stranded so
    // an ordinary load does not pay to rank every squad in the world.
    //
    // Taken out so the scoreline model can read `game.players` while the competitions are mutated.
    let mut competitions = std::mem::take(&mut game.competitions);
    for competition in competitions.iter_mut().filter(|competition| {
        !is_national_team_competition(competition) && stranded_in(competition, today) > 0
    }) {
        // This competition's own stream for this pass: a second pass over the same competition
        // is a second roll, not the first one again.
        let mut rng = crate::seed::rng_for_seed(
            game.seed,
            &format!("catchup/{}/pass{pass}", competition.id),
            &today.to_string(),
        );
        resolved += simulate_past_fixtures(
            competition,
            &game.players,
            game.clock.current_date,
            &mut rng,
        );
    }
    game.competitions = competitions;

    // National-team football, replayed date by date through the code that owns it. A nation has no
    // club players, so `club_strength` reads 50.0 for every side — routing a stranded World Cup tie
    // or a window friendly through the club sweep makes it a coin flip between two nobodies. These
    // processors use the called-up squads, apply carry-back to the club players who travelled, and
    // move the world ranking, exactly as they would have on the day.
    //
    // Replaying a past date also produces the news that date would have produced, a crowned
    // champion included. That is correct — the player learns what happened — but it does mean a load
    // can deliver months-old international news.
    let mut rng = game.rng_for(
        &format!("catchup/international/pass{pass}"),
        &today.to_string(),
    );
    for date in stranded_international_dates(game, today) {
        resolved += crate::national_team::process_national_team_fixtures_due(game, &date);
        resolved += crate::world_cup::process_world_cup_fixtures_due(game, &date, &mut rng);
    }

    // A national-team competition the processors above cannot reach at all:
    // `process_world_cup_fixtures_due` serves international *scope* only, and a package may author a
    // national-team competition at continental scope (`competition.scope` comes straight from the
    // definition file). For those, the scoreline sweep is worse football than a squad-strength
    // result but better than a fixture stranded for good — which is the bug, not a lesser form of it.
    //
    // Excluding what those processors *do* serve is not a nicety. The dates were collected before
    // this pass played anything, so a round the national path has just seeded — the final of a
    // bracket, the knockouts after a group stage — is stranded-in-the-past and not yet on the list.
    // Without this filter the fallback would resolve it here, by coin flip between two sides scored
    // at 50.0 apiece, before the next pass could hand it to the engine that owns it.
    let mut competitions = std::mem::take(&mut game.competitions);
    for competition in competitions.iter_mut().filter(|competition| {
        is_national_team_competition(competition)
            && !crate::world_cup::is_world_cup_competition(competition)
            && stranded_in(competition, today) > 0
    }) {
        let mut rng = crate::seed::rng_for_seed(
            game.seed,
            &format!("catchup/{}/pass{pass}", competition.id),
            &today.to_string(),
        );
        resolved += simulate_past_fixtures(
            competition,
            &game.players,
            game.clock.current_date,
            &mut rng,
        );
    }
    game.competitions = competitions;

    resolved
}

/// Returns how many fixtures were repaired, for the caller to log and to decide whether to resave.
pub fn repair_stranded_fixtures(game: &mut Game) -> usize {
    let today = game.clock.current_date.date_naive();

    // Cheap enough to run on every load: it reads fixture status and date, nothing else. Worth
    // keeping as a guard because a pass is not cheap — it ranks every participant's squad.
    if count_stranded(game, today) == 0 {
        return 0;
    }

    let mut repaired = 0;
    for pass in 0..MAX_REPAIR_PASSES {
        let resolved = repair_one_pass(game, today, pass);
        if resolved == 0 {
            // Nothing moved, so nothing will. Stop instead of spinning: an unplayable fixture stays
            // as it is rather than costing the load a bounded-but-pointless sixty-four passes.
            break;
        }
        repaired += resolved;
    }

    if repaired > 0 {
        // The repair rewrote `competitions`, and `game.league` is a *mirror* of the user's — the one
        // the home dashboard reads for the next match and the league table. Left un-synced it shows
        // the player a fixture that has just been played as still to come, above a table that
        // predates it. `read_game` syncs the mirror before this runs, which is exactly why it has to
        // be synced again after.
        //
        // Guarded on `repaired > 0` because syncing is not free (it clones a competition) and a load
        // that found nothing stranded has not moved anything for the mirror to fall behind.
        game.sync_legacy_league();
    }

    repaired
}

#[cfg(test)]
mod tests {
    use super::apply_simulated_result;
    use domain::league::{
        CompetitionFormat, CompetitionRules, Fixture, FixtureCompetition, FixtureStatus,
        KnockoutRoundState, League,
    };

    fn make_knockout_cup() -> League {
        League {
            id: "cup".to_string(),
            name: "Cup".to_string(),
            season: 2030,
            rules: CompetitionRules {
                format: CompetitionFormat::Knockout,
                ..CompetitionRules::default()
            },
            fixtures: vec![Fixture {
                id: "fix-1".to_string(),
                competition_id: "cup".to_string(),
                matchday: 1,
                date: "2030-08-10".to_string(),
                home_team_id: "home".to_string(),
                away_team_id: "away".to_string(),
                competition: FixtureCompetition::Cup,
                status: FixtureStatus::Scheduled,
                result: None,
            }],
            knockout_rounds: vec![KnockoutRoundState {
                id: "round-1".to_string(),
                name: "Final".to_string(),
                fixture_ids: vec!["fix-1".to_string()],
                bye_team_ids: Vec::new(),
                completed: false,
            }],
            ..League::default()
        }
    }

    // Regression: level knockout results used to persist with no shootout
    // score, so the away side could never advance from a simulated draw.
    #[test]
    fn simulated_shootout_score_persists_and_decides_the_tie() {
        let mut cup = make_knockout_cup();
        apply_simulated_result(&mut cup, 0, "home", "away", 1, 1, Some((3, 4)));

        let result = cup.fixtures[0].result.as_ref().unwrap();
        assert_eq!(result.home_penalties, Some(3));
        assert_eq!(result.away_penalties, Some(4));
        assert!(!result.advancing_is_home());
        assert!(cup.knockout_rounds[0].completed);
    }

    #[test]
    fn decisive_result_carries_no_shootout() {
        let mut cup = make_knockout_cup();
        apply_simulated_result(&mut cup, 0, "home", "away", 2, 0, None);

        let result = cup.fixtures[0].result.as_ref().unwrap();
        assert_eq!(result.home_penalties, None);
        assert_eq!(result.away_penalties, None);
        assert!(result.advancing_is_home());
    }
    // ---------------------------------------------------------------------
    // Repairing fixtures that were stranded in the past (#608's aftermath).
    // ---------------------------------------------------------------------

    use crate::clock::GameClock;
    use crate::game::Game;
    use chrono::{TimeZone, Utc};
    use domain::league::StandingEntry;
    use domain::manager::Manager;

    fn game_on(date: &str) -> Game {
        let parts: Vec<u32> = date.split('-').map(|p| p.parse().unwrap()).collect();
        let clock = GameClock::new(
            Utc.with_ymd_and_hms(parts[0] as i32, parts[1], parts[2], 12, 0, 0)
                .unwrap(),
        );
        let manager = Manager::new(
            "mgr".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        Game::new(clock, manager, vec![], vec![], vec![], vec![])
    }

    fn league_with_fixture_on(date: &str) -> League {
        let mut league = League::new(
            "league1".to_string(),
            "Test League".to_string(),
            1,
            &["home".to_string(), "away".to_string()],
        );
        league.standings = vec![
            StandingEntry::new("home".to_string()),
            StandingEntry::new("away".to_string()),
        ];
        league.fixtures = vec![Fixture {
            id: "fix-1".to_string(),
            competition_id: "league1".to_string(),
            matchday: 1,
            date: date.to_string(),
            home_team_id: "home".to_string(),
            away_team_id: "away".to_string(),
            competition: FixtureCompetition::League,
            status: FixtureStatus::Scheduled,
            result: None,
        }];
        league
    }

    fn scorelines_after_repairing_twenty_stranded_fixtures(seed: u64) -> Vec<String> {
        let mut game = game_on("2030-09-01");
        game.seed = seed;
        let mut league = league_with_fixture_on("2030-08-10");
        let template = league.fixtures[0].clone();
        league.fixtures = (1..=20)
            .map(|n| Fixture {
                id: format!("fix-{n}"),
                matchday: n,
                date: format!("2030-08-{:02}", n),
                ..template.clone()
            })
            .collect();
        game.competitions = vec![league];

        super::repair_stranded_fixtures(&mut game);

        game.competitions[0]
            .fixtures
            .iter()
            .map(|fixture| {
                let result = fixture.result.as_ref().expect("every fixture was repaired");
                format!("{}-{}", result.home_goals, result.away_goals)
            })
            .collect()
    }

    /// Given a save with twenty stranded fixtures,
    /// When they are repaired twice from the same seed,
    /// Then each is settled with the same score both times — a save reloaded twice must not
    ///      rewrite its own past differently.
    #[test]
    fn stranded_fixtures_are_settled_the_same_way_from_the_same_seed() {
        for seed in 0..10 {
            assert_eq!(
                scorelines_after_repairing_twenty_stranded_fixtures(seed),
                scorelines_after_repairing_twenty_stranded_fixtures(seed),
                "seed {seed}"
            );
        }
    }

    /// Given two games that differ only in their seed,
    /// When the same stranded fixtures are repaired in each,
    /// Then they are not settled alike — a competition caught up to a day is not one fixed
    ///      set of results for every world that ever has it.
    #[test]
    fn the_seed_decides_how_stranded_fixtures_end() {
        let runs: std::collections::BTreeSet<Vec<String>> = (0..10)
            .map(scorelines_after_repairing_twenty_stranded_fixtures)
            .collect();

        assert!(runs.len() > 1, "ten seeds all settled the fixtures alike");
    }

    /// The control: the repair is drawing, not stamping one score on everything.
    #[test]
    fn repaired_fixtures_do_not_all_end_alike() {
        let scores: std::collections::BTreeSet<String> =
            scorelines_after_repairing_twenty_stranded_fixtures(1)
                .into_iter()
                .collect();

        assert!(scores.len() > 1);
    }

    #[test]
    fn repairs_a_fixture_left_scheduled_in_the_past() {
        // #608 left fixtures `Scheduled` with a date that had passed. Nothing can pick them up
        // again — every due-today check matches on `date == today` — so the table is permanently
        // short and a knockout bracket permanently stalled.
        let mut game = game_on("2030-09-01");
        game.competitions = vec![league_with_fixture_on("2030-08-10")];

        let repaired = super::repair_stranded_fixtures(&mut game);

        assert_eq!(repaired, 1, "the stranded fixture is counted");
        assert_eq!(
            game.competitions[0].fixtures[0].status,
            FixtureStatus::Completed
        );
        assert!(game.competitions[0].fixtures[0].result.is_some());
        assert!(
            game.competitions[0]
                .standings
                .iter()
                .all(|entry| entry.played == 1),
            "the table records the match that was missing from it"
        );
    }

    /// **Given** a save whose user league has a fixture stranded in the past, and whose legacy
    /// mirror holds the pre-repair copy of it, **when** the fixtures are repaired, **then** the
    /// mirror shows the repaired fixture too.
    ///
    /// `game.league` is a mirror of the user's competition and the home dashboard reads it for the
    /// next match and the league position. The repair rewrites `competitions`; `read_game` syncs the
    /// mirror *before* that happens, so without a sync afterwards the dashboard shows a fixture that
    /// has just been played as still to come, above a table that predates it.
    #[test]
    fn repairing_competitions_keeps_the_legacy_mirror_in_step() {
        let mut game = game_on("2030-09-01");
        let mut manager = Manager::new(
            "mgr".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("home".to_string());
        game.manager = manager;

        let stranded = league_with_fixture_on("2030-08-10");
        game.competitions = vec![stranded.clone()];
        // As `read_game` leaves it: a mirror of the competition, synced before the repair runs.
        game.league = Some(stranded);

        assert_eq!(super::repair_stranded_fixtures(&mut game), 1);

        let mirror = game.league.as_ref().expect("the mirror is still there");
        assert_eq!(
            mirror.fixtures[0].status,
            FixtureStatus::Completed,
            "the dashboard reads the mirror, so it must show the repaired fixture"
        );
        assert!(
            mirror.standings.iter().all(|entry| entry.played == 1),
            "and the table behind it, not the one from before the repair"
        );
    }

    #[test]
    fn leaves_today_and_the_future_alone() {
        // The repair must not eat the season. Only a date that has *passed* is unreachable; today's
        // fixtures are about to be played by the day's own code, and later ones are not due yet.
        let mut game = game_on("2030-09-01");
        game.competitions = vec![
            league_with_fixture_on("2030-09-01"),
            league_with_fixture_on("2030-09-08"),
        ];

        let repaired = super::repair_stranded_fixtures(&mut game);

        assert_eq!(repaired, 0, "nothing was stranded");
        for competition in &game.competitions {
            assert_eq!(
                competition.fixtures[0].status,
                FixtureStatus::Scheduled,
                "a fixture due today or later is not the repair's business"
            );
        }
    }

    #[test]
    fn is_idempotent_so_every_load_does_not_re_resave() {
        // It runs on every load, not behind a save-format gate, so the second run has to be a
        // no-op — otherwise every load reports a change and rewrites the save.
        let mut game = game_on("2030-09-01");
        game.competitions = vec![league_with_fixture_on("2030-08-10")];

        assert_eq!(super::repair_stranded_fixtures(&mut game), 1);
        let after_first = game.competitions[0].fixtures[0]
            .result
            .as_ref()
            .map(|result| (result.home_goals, result.away_goals));

        assert_eq!(
            super::repair_stranded_fixtures(&mut game),
            0,
            "a second load finds nothing left to repair"
        );
        assert_eq!(
            game.competitions[0].fixtures[0]
                .result
                .as_ref()
                .map(|result| (result.home_goals, result.away_goals)),
            after_first,
            "and does not re-roll the result it already recorded"
        );
    }

    #[test]
    fn leaves_an_in_progress_fixture_alone() {
        // A save taken mid-match has an `InProgress` fixture whose live session may be resumable.
        // Resolving it by scoreline would throw that away.
        let mut game = game_on("2030-09-01");
        let mut league = league_with_fixture_on("2030-08-10");
        league.fixtures[0].status = FixtureStatus::InProgress;
        game.competitions = vec![league];

        assert_eq!(super::repair_stranded_fixtures(&mut game), 0);
        assert_eq!(
            game.competitions[0].fixtures[0].status,
            FixtureStatus::InProgress
        );
    }

    // -----------------------------------------------------------------------
    // A bracket cascades: repairing one round seeds the next, also in the past.
    // -----------------------------------------------------------------------

    fn eight_team_cup_starting_on(date: &str) -> League {
        let clubs: Vec<String> = (1..=8).map(|number| format!("club{number}")).collect();
        let mut cup = League::new("cup".to_string(), "National Cup".to_string(), 2030, &clubs);
        cup.rules.format = CompetitionFormat::Knockout;
        for (index, pair) in clubs.chunks(2).enumerate() {
            cup.fixtures.push(Fixture {
                id: format!("r1-{index}"),
                competition_id: "cup".to_string(),
                matchday: 1,
                date: date.to_string(),
                home_team_id: pair[0].clone(),
                away_team_id: pair[1].clone(),
                competition: FixtureCompetition::Cup,
                status: FixtureStatus::Scheduled,
                result: None,
            });
        }
        cup.knockout_rounds = vec![KnockoutRoundState {
            id: "round-1".to_string(),
            name: "Quarter-finals".to_string(),
            fixture_ids: cup.fixtures.iter().map(|f| f.id.clone()).collect(),
            bye_team_ids: Vec::new(),
            completed: false,
        }];
        cup
    }

    #[test]
    fn repairs_a_whole_stranded_bracket_and_not_just_its_first_round() {
        // Resolving a knockout round *seeds the next one* — dated later by the round gap, but for a
        // bracket deep in the past still before today. `simulate_past_fixtures` builds its due list
        // once, before playing any of it, so one sweep advances the bracket by exactly one round.
        //
        // An eight-team cup therefore repaired 4 fixtures on one load, 2 on the next and 1 on the
        // one after: three loads to reach a champion, and a bracket that stalls for good if the
        // player never loads again. The invariant is per load, so the repair iterates.
        let mut game = game_on("2030-12-01");
        game.competitions = vec![eight_team_cup_starting_on("2030-08-10")];

        let repaired = super::repair_stranded_fixtures(&mut game);

        let cup = &game.competitions[0];
        assert_eq!(
            repaired,
            7,
            "four quarter-finals, two semi-finals and a final, in one call: {:?}",
            cup.fixtures
                .iter()
                .map(|fixture| (fixture.id.as_str(), &fixture.status))
                .collect::<Vec<_>>()
        );
        assert!(
            cup.fixtures
                .iter()
                .all(|fixture| fixture.status == FixtureStatus::Completed),
            "including the rounds the repair itself brought into being"
        );
        assert!(
            cup.knockout_rounds.iter().all(|round| round.completed),
            "the bracket reached a champion rather than stalling part-way"
        );
    }

    // -----------------------------------------------------------------------
    // National-team football is not club football (the user ruling on #642).
    // -----------------------------------------------------------------------

    use crate::test_support::uniform_attributes;
    use domain::league::{CompetitionScope, CompetitionType};
    use domain::national_team::NationalTeam;
    use domain::player::{Player, Position};

    fn international_fixture(id: &str, date: &str, home: &str, away: &str) -> Fixture {
        Fixture {
            id: id.to_string(),
            competition_id: "wc-2030".to_string(),
            matchday: 1,
            date: date.to_string(),
            home_team_id: home.to_string(),
            away_team_id: away.to_string(),
            competition: FixtureCompetition::InternationalNation,
            status: FixtureStatus::Scheduled,
            result: None,
        }
    }

    /// Two national sides with real squads, and no clubs at all — which is the point. A nation has
    /// no players *on its books*, so `club_strength` falls through to its neutral 50.0 for both
    /// sides and the tie becomes a coin flip between two blanks.
    fn add_national_squad(game: &mut Game, team_id: &str, nation: &str, prefix: &str, ovr: u8) {
        let mut squad = Vec::new();
        for index in 0..11 {
            let position = if index == 0 {
                Position::Goalkeeper
            } else if index < 5 {
                Position::Defender
            } else if index < 9 {
                Position::Midfielder
            } else {
                Position::Forward
            };
            let mut player = Player::new(
                format!("{prefix}-{index}"),
                format!("{prefix} {index}"),
                format!("{prefix} {index}"),
                "1998-01-01".to_string(),
                nation.to_string(),
                position,
                uniform_attributes(60),
            );
            player.ovr = ovr;
            squad.push(player);
        }
        let mut team = NationalTeam::new(
            team_id.to_string(),
            nation.to_string(),
            nation.to_string(),
            None,
        );
        team.squad_player_ids = squad.iter().map(|player| player.id.clone()).collect();
        game.players.extend(squad);
        game.national_teams.push(team);
    }

    fn game_with_two_national_squads(today: &str) -> Game {
        let mut game = game_on(today);
        add_national_squad(&mut game, "nt-eng", "England", "eng", 80);
        add_national_squad(&mut game, "nt-bra", "Brazil", "bra", 70);
        game
    }

    #[test]
    fn repairs_a_stranded_international_tie_through_the_national_team_path() {
        // The club sweep would mark this `Completed` too — with an empty scorer list, no carry-back
        // to the players who travelled, and no movement in the world ranking, because it resolves
        // the tie between two sides it scores at 50.0 apiece. Routing it through the national-team
        // processors is what the user ruled for, so assert the things only that path does.
        let mut game = game_with_two_national_squads("2030-12-01");
        let mut competition = League::new(
            "wc-2030".to_string(),
            "World Cup 2030".to_string(),
            2030,
            &["nt-eng".to_string(), "nt-bra".to_string()],
        );
        competition.kind = CompetitionType::InternationalNation;
        competition.scope = CompetitionScope::International;
        competition.fixtures = vec![international_fixture(
            "wc-g1",
            "2030-06-12",
            "nt-eng",
            "nt-bra",
        )];
        game.competitions = vec![competition];

        let repaired = super::repair_stranded_fixtures(&mut game);

        assert_eq!(repaired, 1);
        let fixture = &game.competitions[0].fixtures[0];
        assert_eq!(fixture.status, FixtureStatus::Completed);
        let result = fixture.result.as_ref().expect("the tie was resolved");

        // `apply_simulated_result` — the club sweep's applier — stores an empty scorer list. The
        // national path names who scored, from the called-up squad.
        let goals = usize::from(result.home_goals) + usize::from(result.away_goals);
        assert_eq!(
            result.home_scorers.len() + result.away_scorers.len(),
            goals,
            "every goal in an international is credited to a squad member"
        );
        for scorer in result.home_scorers.iter().chain(result.away_scorers.iter()) {
            assert!(
                scorer.player_id.starts_with("eng-") || scorer.player_id.starts_with("bra-"),
                "a scorer who was never called up: {}",
                scorer.player_id
            );
        }

        // And the world ranking moved, which only the national path does — it holds whether or not
        // anybody scored, so it is what pins the path when the tie finishes 0-0.
        let ranked = game.world_history.ranked_nation_codes();
        assert!(
            ranked.contains(&"ENG".to_string()) && ranked.contains(&"BRA".to_string()),
            "both nations are in the world ranking after playing: {ranked:?}"
        );
    }

    #[test]
    fn a_cascaded_international_round_still_goes_through_the_national_team_path() {
        // The two fixes meet here, and the first version got it wrong.
        //
        // The stranded dates are collected before the pass plays anything, so the final that playing
        // the semi-finals *seeds* is stranded-in-the-past and not on that list. The fallback sweep
        // for package-authored competitions then matched the World Cup as well, and resolved the
        // final by coin flip between two sides scored at 50.0 — inside the same pass, before the
        // next one could hand it to the national-team engine.
        //
        // A single-fixture international cannot see this, and a club cup cannot either. It takes a
        // bracket that cascades *and* is national.
        let mut game = game_with_two_national_squads("2030-12-01");
        add_national_squad(&mut game, "nt-fra", "France", "fra", 75);
        add_national_squad(&mut game, "nt-arg", "Argentina", "arg", 78);

        let mut competition = League::new(
            "wc-2030".to_string(),
            "World Cup 2030".to_string(),
            2030,
            &[
                "nt-eng".to_string(),
                "nt-bra".to_string(),
                "nt-fra".to_string(),
                "nt-arg".to_string(),
            ],
        );
        competition.kind = CompetitionType::InternationalNation;
        competition.scope = CompetitionScope::International;
        competition.rules.format = CompetitionFormat::Knockout;
        competition.fixtures = vec![
            international_fixture("sf-1", "2030-06-12", "nt-eng", "nt-bra"),
            international_fixture("sf-2", "2030-06-12", "nt-fra", "nt-arg"),
        ];
        competition.knockout_rounds = vec![KnockoutRoundState {
            id: "semis".to_string(),
            name: "Semi-finals".to_string(),
            fixture_ids: vec!["sf-1".to_string(), "sf-2".to_string()],
            bye_team_ids: Vec::new(),
            completed: false,
        }];
        game.competitions = vec![competition];

        let repaired = super::repair_stranded_fixtures(&mut game);

        assert_eq!(repaired, 3, "two semi-finals and the final they produced");
        let cup = &game.competitions[0];
        assert!(
            cup.fixtures
                .iter()
                .all(|fixture| fixture.status == FixtureStatus::Completed),
            "including the final, which did not exist when the pass began"
        );

        // The discriminator, and the reason it is this one: a champion is recorded only by
        // `process_world_cup_fixtures_due`. The scoreline sweep completes the final and marks the
        // round done, so fixture status cannot tell the two paths apart — and scorers cannot either,
        // because a 0-0 final decided on penalties has none on either path.
        let champions = &game.world_history.world_cup_champions;
        assert_eq!(
            champions.len(),
            1,
            "the final was played by the engine that crowns a champion, not by the fallback sweep"
        );
        assert_eq!(champions[0].year, 2030);
    }

    #[test]
    fn repairs_a_stranded_window_friendly_on_a_national_team() {
        // These fixtures do not live in `competitions` at all — they hang off `game.national_teams`,
        // which is why #608 stranded them and why a repair that only walks `competitions` cannot see
        // them. The count has to include them too, or the early return skips the whole job.
        let mut game = game_with_two_national_squads("2030-12-01");
        game.national_teams[0].fixtures = vec![international_fixture(
            "friendly-1",
            "2030-09-05",
            "nt-eng",
            "nt-bra",
        )];

        let repaired = super::repair_stranded_fixtures(&mut game);

        assert_eq!(repaired, 1, "the window friendly is found and played");
        let fixture = &game.national_teams[0].fixtures[0];
        assert_eq!(fixture.status, FixtureStatus::Completed);
        assert!(
            fixture.result.is_some(),
            "and it carries a result, not just a status"
        );
    }

    fn friendly_score_after_repair(seed: u64) -> String {
        let mut game = game_with_two_national_squads("2030-12-01");
        game.seed = seed;
        game.national_teams[0].fixtures = vec![international_fixture(
            "friendly-1",
            "2030-09-05",
            "nt-eng",
            "nt-bra",
        )];
        super::repair_stranded_fixtures(&mut game);
        let result = game.national_teams[0].fixtures[0]
            .result
            .as_ref()
            .expect("the friendly was played");
        format!("{}-{}", result.home_goals, result.away_goals)
    }

    /// Given a stranded national-team friendly,
    /// When it is repaired twice from the same seed,
    /// Then it ends the same way both times, and the seed is what decides how.
    #[test]
    fn a_stranded_friendly_is_settled_the_same_way_from_the_same_seed() {
        for seed in 0..20 {
            assert_eq!(
                friendly_score_after_repair(seed),
                friendly_score_after_repair(seed),
                "seed {seed}"
            );
        }
        let scores: std::collections::BTreeSet<String> =
            (0..40).map(friendly_score_after_repair).collect();
        assert!(scores.len() > 1, "forty seeds all settled it alike");
    }
}
