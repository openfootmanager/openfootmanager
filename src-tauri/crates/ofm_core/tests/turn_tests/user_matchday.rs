//! One call plays the user's fixture and finishes the day around it.
//!
//! `finish_live_match_day_with_capture` is only the tail of a live-match day. The sequence that
//! surrounds it — swap the competition in, create the session, sweep the rest of the round, apply
//! the user's result, restore the competition, run the tail — lived only in the Tauri application
//! layer, in two near-copies. A season harness driving the tail alone would leave the user's own
//! fixture `Scheduled`, so the sweep would play it through the batch engine: a path no real caller
//! takes, and exactly the flaw #618's review found in two hand-written tests.

use domain::league::{Fixture, FixtureCompetition, FixtureStatus, League, StandingEntry};
use ofm_core::game::Game;
use ofm_core::matchday;

use super::fixtures::{make_game_with_match, make_squad, make_team};

/// The user's club has a fixture still to play in `league1`, which has a *second* fixture on the
/// same date between two other clubs; `league2` is another country's division with its own fixture
/// that day.
///
/// The second fixture in the user's own league is what gives the start-of-day sweep something to
/// sweep. With only the user's fixture in it, the sweep is a no-op no matter what index it is told
/// to skip — so a world that small cannot tell the session path from the sweep, and every assertion
/// about "the user's match was played" passes either way.
pub(super) fn game_before_the_users_match() -> Game {
    let mut game = make_game_with_match();
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();

    let mut user_league = game.league.clone().expect("the helper builds a league");
    user_league.participant_ids = vec![
        "team1".to_string(),
        "team2".to_string(),
        "team5".to_string(),
        "team6".to_string(),
    ];
    user_league.fixtures.push(Fixture {
        id: "fix1b".to_string(),
        competition_id: "league1".to_string(),
        matchday: 1,
        date: today.clone(),
        home_team_id: "team5".to_string(),
        away_team_id: "team6".to_string(),
        competition: FixtureCompetition::League,
        status: FixtureStatus::Scheduled,
        result: None,
    });
    // `make_game_with_match` hand-builds the table for its two clubs, so the new pair need entries
    // of their own — `record_result` looks a club up by id and silently finds nothing otherwise.
    user_league
        .standings
        .push(StandingEntry::new("team5".to_string()));
    user_league
        .standings
        .push(StandingEntry::new("team6".to_string()));

    for (id, name, prefix) in [
        ("team3", "Third FC", "t3"),
        ("team4", "Fourth FC", "t4"),
        ("team5", "Fifth FC", "t5"),
        ("team6", "Sixth FC", "t6"),
    ] {
        game.teams.push(make_team(id, name));
        game.players.extend(make_squad(id, prefix));
    }

    let mut other = League::new(
        "league2".to_string(),
        "Other League".to_string(),
        1,
        &["team3".to_string(), "team4".to_string()],
    );
    other.fixtures.push(Fixture {
        id: "fix2".to_string(),
        competition_id: "league2".to_string(),
        matchday: 1,
        date: today,
        home_team_id: "team3".to_string(),
        away_team_id: "team4".to_string(),
        competition: FixtureCompetition::League,
        status: FixtureStatus::Scheduled,
        result: None,
    });

    game.competitions = vec![user_league, other];
    game.league = Some(game.competitions[0].clone());
    game
}

#[test]
fn playing_the_users_matchday_leaves_nothing_due_today_unplayed() {
    // The invariant the season harness asserts across a whole season, stated once here: after a day
    // has been played, no fixture dated that day is still waiting. It covers the user's own fixture
    // and every other competition's in one assertion, so a competition type added later is caught
    // without a new test.
    let mut game = game_before_the_users_match();
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();

    matchday::play_user_matchday_with_capture(&mut game, Some(0), 0, &mut |_| {})
        .expect("the user's matchday is played");

    // The shared rule, not a restatement of it: nothing dated today or earlier is left Scheduled.
    let unplayed = matchday::stranded_fixtures(
        &game,
        chrono::NaiveDate::parse_from_str(&today, "%Y-%m-%d").unwrap(),
    );

    assert!(
        unplayed.is_empty(),
        "every fixture due today should have been played: {unplayed:?}"
    );
}

#[test]
fn playing_the_users_matchday_reports_the_pre_round_table() {
    // The digest's baseline has to be the table *before* the round. Taken afterwards it reports
    // every delta as zero, which is the regression #618 shipped and then had to fix: the sequence
    // plays the round, so only a value captured at session creation can be a baseline.
    let mut game = game_before_the_users_match();

    let outcome = matchday::play_user_matchday_with_capture(&mut game, Some(0), 0, &mut |_| {})
        .expect("the user's matchday is played");

    let (matchday_number, previous) = outcome
        .league_round_context
        .expect("the user's league has a fixture today, so it has a round context");
    assert_eq!(matchday_number, 1, "the league's round due today");
    assert!(
        previous.iter().all(|entry| entry.played == 0),
        "the baseline is the table before the round, so nobody has played yet: {:?}",
        previous
            .iter()
            .map(|entry| (entry.team_id.as_str(), entry.played))
            .collect::<Vec<_>>()
    );

    // And the round really was played, so the baseline is not simply an empty world.
    let user_league = &game.competitions[0];
    assert!(
        user_league.standings.iter().any(|entry| entry.played > 0),
        "the league table has moved on from that baseline"
    );
}

#[test]
fn the_users_fixture_is_stored_from_the_session_and_the_sweep_plays_the_rest() {
    // Three mistakes this separates, none of which the other tests in this file can see.
    //
    // The user's stored result has to be the report this call hands back — the one the *session*
    // produced. Drop the `apply_match_report_with_capture` call and the day's tail sweeps the
    // fixture up instead, batch-playing it: still `Completed`, still 90-plus minutes, still
    // carrying stats, but a different run of a different engine, so a different scoreline and
    // different scorers.
    //
    // Every club playing exactly once catches the opposite mistake. Hand the start-of-day sweep
    // `None` instead of `Some(fixture_index)` and it plays the user's fixture too, then the report
    // is applied on top: team1 and team2 are recorded twice.
    //
    // And `fix1b` proves the sweep still does its job — that the user's fixture is skipped, not
    // the whole round.
    let mut game = game_before_the_users_match();

    let outcome = matchday::play_user_matchday_with_capture(&mut game, Some(0), 0, &mut |_| {})
        .expect("the user's matchday is played");

    let user_league = &game.competitions[0];
    let stored = user_league.fixtures[0]
        .result
        .as_ref()
        .expect("the user's fixture was played, so it carries a result");

    assert_eq!(
        (stored.home_goals, stored.away_goals),
        (outcome.report.home_goals, outcome.report.away_goals),
        "the stored scoreline is the session's own, not a second simulation of the same fixture"
    );

    // `GoalEvent` and `GoalDetail` do not derive `PartialEq`, so compare what identifies a goal.
    let reported = |side: engine::Side| -> Vec<(String, u8)> {
        outcome
            .report
            .goals
            .iter()
            .filter(|goal| goal.side == side)
            .map(|goal| (goal.scorer_id.clone(), goal.minute))
            .collect()
    };
    let stored_scorers = |scorers: &[domain::league::GoalEvent]| -> Vec<(String, u8)> {
        scorers
            .iter()
            .map(|goal| (goal.player_id.clone(), goal.minute))
            .collect()
    };
    assert_eq!(
        stored_scorers(&stored.home_scorers),
        reported(engine::Side::Home),
        "the home scorers stored are the ones the session's report named"
    );
    assert_eq!(
        stored_scorers(&stored.away_scorers),
        reported(engine::Side::Away),
        "the away scorers stored are the ones the session's report named"
    );

    assert!(
        user_league.standings.iter().all(|entry| entry.played == 1),
        "every club in the user's league played exactly once today: {:?}",
        user_league
            .standings
            .iter()
            .map(|entry| (entry.team_id.as_str(), entry.played))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        user_league.fixtures[1].status,
        FixtureStatus::Completed,
        "the sweep plays the rest of the user's own round, not only the other competitions"
    );
}

/// **Given** the user's fixture sits in a competition whose clubs have no players,
/// **when** the day is played, **then** the call fails *and* the legacy mirror still holds the
/// competition it held before.
///
/// The mirror has to be swapped in before the session is built, because that is where
/// `create_live_match` reads the fixture from. An empty squad is a handled `Err` by design — the
/// floor that keeps it from happening lives elsewhere, and this path must never panic — but the
/// early return used to leave the swap in place. This runs inside `update_game`, which mutates the
/// live game, so the player was left looking at another competition with no match to explain it.
#[test]
fn a_session_that_cannot_be_built_leaves_the_mirror_as_it_found_it() {
    let mut game = game_before_the_users_match();
    // League 2's clubs exist but have nobody on the books, so its fixture cannot field a side.
    game.players
        .retain(|player| player.team_id.as_deref() != Some("team3"));
    game.players
        .retain(|player| player.team_id.as_deref() != Some("team4"));
    let mirror_before = game
        .league
        .as_ref()
        .map(|league| league.id.clone())
        .expect("the user's league is in the mirror");

    let outcome = matchday::play_user_matchday_with_capture(&mut game, Some(1), 0, &mut |_| {});

    assert!(
        outcome.is_err(),
        "a side with nobody available cannot play, and that is an Err rather than a panic"
    );
    assert_eq!(
        game.league.as_ref().map(|league| league.id.as_str()),
        Some(mirror_before.as_str()),
        "the mirror is restored, so the game the player returns to is the one they left"
    );
}

/// **Given** a game with two competitions, **when** the day is played for competition 99,
/// **then** it is refused.
///
/// `if let Some(...)` used to swallow this: no swap happened, and the day was played out of
/// whichever competition the mirror held, at `fixture_index`. Someone else's match, reported `Ok`.
#[test]
fn a_competition_index_that_names_no_competition_is_refused() {
    let mut game = game_before_the_users_match();

    let outcome = matchday::play_user_matchday_with_capture(&mut game, Some(99), 0, &mut |_| {});

    assert_eq!(
        outcome.err().as_deref(),
        Some("be.error.liveMatch.fixtureNotFound"),
        "an index naming no competition is refused, not quietly reinterpreted"
    );
    assert!(
        game.competitions
            .iter()
            .flat_map(|competition| competition.fixtures.iter())
            .all(|fixture| fixture.status == FixtureStatus::Scheduled),
        "and nothing was played on the way to finding out"
    );
}

/// **Given** a save written before `competitions` existed — the user's league is in the legacy
/// mirror only — **when** the day is played with no competition named, **then** the match is played
/// from the mirror.
///
/// `None` is what `scheduled_user_fixture_index` answers for such a save: there is no competition to
/// name, and the mirror already holds the fixture. This used to be an index one past the end, which
/// is why the out-of-range guard could not simply refuse everything beyond the list — the legacy
/// case and a genuinely wrong index were the same value. Now they are different types of answer,
/// and this test is what stops a future guard from refusing the legacy one.
#[test]
fn a_legacy_save_with_no_competition_named_plays_from_the_mirror() {
    let mut game = game_before_the_users_match();
    let mirror = game.competitions[0].clone();
    game.competitions = Vec::new();
    game.league = Some(mirror);

    let outcome = matchday::play_user_matchday_with_capture(&mut game, None, 0, &mut |_| {})
        .expect("with no competition named, the fixture the mirror holds is played");

    assert!(
        outcome.report.total_minutes >= 90,
        "a real match was played"
    );
    let played = game
        .league
        .as_ref()
        .expect("the mirror is still there")
        .fixtures[0]
        .clone();
    assert_eq!(
        played.status,
        FixtureStatus::Completed,
        "and the result stayed in the mirror, which is the only copy this save has"
    );
}

#[test]
fn playing_the_users_matchday_hands_back_the_report_and_the_stats() {
    let mut game = game_before_the_users_match();

    let mut captures = Vec::new();
    let outcome =
        matchday::play_user_matchday_with_capture(&mut game, Some(0), 0, &mut |capture| {
            captures.push(capture)
        })
        .expect("the user's matchday is played");

    assert!(
        outcome.report.total_minutes >= 90,
        "the user's own match was played to completion — 90 plus stoppage, not a default: {}",
        outcome.report.total_minutes
    );
    assert!(
        !outcome.report.player_stats.is_empty(),
        "the report carries per-player stats for the match that was played"
    );
    assert!(
        !captures.is_empty(),
        "the matches played handed their stats back rather than dropping them"
    );

    // Not merely "some capture arrived": the other fixtures on the day produce captures of their
    // own, so passing a no-op capture for the *user's* match still left this green. It has to name
    // the user's own fixture.
    let has_user_fixture_stats = captures.iter().any(|capture| {
        capture
            .team_matches
            .iter()
            .any(|record| record.fixture_id == "fix1")
            || capture
                .player_matches
                .iter()
                .any(|record| record.fixture_id == "fix1")
    });
    assert!(
        has_user_fixture_stats,
        "the user's own match is in the stats, not only the ones played around it: {:?}",
        captures
            .iter()
            .flat_map(|capture| capture.team_matches.iter())
            .map(|record| record.fixture_id.as_str())
            .collect::<Vec<_>>()
    );
}
