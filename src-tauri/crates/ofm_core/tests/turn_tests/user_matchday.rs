//! One call plays the user's fixture and finishes the day around it.
//!
//! `finish_live_match_day_with_capture` is only the tail of a live-match day. The sequence that
//! surrounds it — swap the competition in, create the session, sweep the rest of the round, apply
//! the user's result, restore the competition, run the tail — lived only in the Tauri application
//! layer, in two near-copies. A season harness driving the tail alone would leave the user's own
//! fixture `Scheduled`, so the sweep would play it through the batch engine: a path no real caller
//! takes, and exactly the flaw #618's review found in two hand-written tests.

use domain::league::{Fixture, FixtureCompetition, FixtureStatus, League};
use ofm_core::game::Game;
use ofm_core::matchday;

use super::fixtures::{make_game_with_match, make_squad, make_team};

/// The user's club has a fixture still to play in `league1`; `league2` is another country's
/// division with its own fixture on the same date.
fn game_before_the_users_match() -> Game {
    let mut game = make_game_with_match();
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();

    let mut user_league = game.league.clone().expect("the helper builds a league");
    user_league.participant_ids = vec!["team1".to_string(), "team2".to_string()];

    game.teams.push(make_team("team3", "Third FC"));
    game.teams.push(make_team("team4", "Fourth FC"));
    game.players.extend(make_squad("team3", "t3"));
    game.players.extend(make_squad("team4", "t4"));

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

    matchday::play_user_matchday_with_capture(&mut game, 0, 0, &mut |_| {})
        .expect("the user's matchday is played");

    let unplayed: Vec<(String, String)> = game
        .competitions
        .iter()
        .flat_map(|competition| {
            competition
                .fixtures
                .iter()
                .filter(|fixture| {
                    fixture.date == today && fixture.status == FixtureStatus::Scheduled
                })
                .map(|fixture| (competition.id.clone(), fixture.id.clone()))
        })
        .collect();

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

    let outcome = matchday::play_user_matchday_with_capture(&mut game, 0, 0, &mut |_| {})
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
fn playing_the_users_matchday_hands_back_the_report_and_the_stats() {
    let mut game = game_before_the_users_match();

    let mut captures = Vec::new();
    let outcome = matchday::play_user_matchday_with_capture(&mut game, 0, 0, &mut |capture| {
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
}
