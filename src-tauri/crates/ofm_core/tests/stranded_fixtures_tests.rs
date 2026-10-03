//! The one rule for "nothing dated on or before this day is left Scheduled".

use chrono::{NaiveDate, TimeZone, Utc};
use domain::league::{Fixture, FixtureCompetition, FixtureStatus, League};
use domain::manager::Manager;
use domain::national_team::NationalTeam;
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::matchday::stranded_fixtures;

fn fixture(id: &str, date: &str, status: FixtureStatus) -> Fixture {
    Fixture {
        id: id.to_string(),
        competition_id: "c".to_string(),
        matchday: 1,
        date: date.to_string(),
        home_team_id: "a".to_string(),
        away_team_id: "b".to_string(),
        competition: FixtureCompetition::League,
        status,
        result: None,
    }
}

fn game_with(fixtures: Vec<Fixture>) -> Game {
    let manager = Manager::new(
        "m".to_string(),
        "A".to_string(),
        "B".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    let mut game = Game::new(
        GameClock::new(Utc.with_ymd_and_hms(2030, 9, 10, 0, 0, 0).unwrap()),
        manager,
        vec![],
        vec![],
        vec![],
        vec![],
    );
    let mut league = League::new("mine".to_string(), "Mine".to_string(), 2030, &[]);
    league.fixtures = fixtures;
    game.competitions = vec![league];
    game
}

fn day(date: &str) -> NaiveDate {
    NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap()
}

/// Given a game with one Scheduled fixture dated yesterday,
/// When the rule runs for today,
/// Then it is returned, with where it lives and when it was due.
#[test]
fn a_scheduled_fixture_in_the_past_is_stranded() {
    let game = game_with(vec![fixture("old", "2030-09-09", FixtureStatus::Scheduled)]);

    let stranded = stranded_fixtures(&game, day("2030-09-10"));

    assert_eq!(stranded.len(), 1);
    assert_eq!(stranded[0].fixture_id, "old");
    assert_eq!(stranded[0].owner, "mine");
    assert_eq!(stranded[0].date, "2030-09-09");
}

/// Given Completed fixtures and Scheduled fixtures dated tomorrow,
/// When the rule runs for today,
/// Then none are returned; and one dated today is, because the rule is "on or before".
#[test]
fn a_completed_or_future_fixture_is_not() {
    let game = game_with(vec![
        fixture("done", "2030-09-09", FixtureStatus::Completed),
        fixture("later", "2030-09-11", FixtureStatus::Scheduled),
        fixture("today", "2030-09-10", FixtureStatus::Scheduled),
    ]);

    let stranded = stranded_fixtures(&game, day("2030-09-10"));

    assert_eq!(
        stranded
            .iter()
            .map(|s| s.fixture_id.as_str())
            .collect::<Vec<_>>(),
        vec!["today"]
    );
}

/// Given a stranded fixture in a competition other than mine, and one on a national team,
/// When the rule runs,
/// Then both are returned (the rule is not scoped to the user's league).
#[test]
fn every_competition_is_checked() {
    let mut game = game_with(vec![]);
    let mut other = League::new("other".to_string(), "Other".to_string(), 2030, &[]);
    other.fixtures = vec![fixture("elsewhere", "2030-09-01", FixtureStatus::Scheduled)];
    game.competitions.push(other);
    let mut national =
        NationalTeam::new("nt-x".to_string(), "X".to_string(), "X".to_string(), None);
    national.fixtures = vec![fixture("window", "2030-09-02", FixtureStatus::Scheduled)];
    game.national_teams = vec![national];

    let mut found: Vec<(String, String)> = stranded_fixtures(&game, day("2030-09-10"))
        .into_iter()
        .map(|s| (s.owner, s.fixture_id))
        .collect();
    found.sort();

    assert_eq!(
        found,
        vec![
            ("nt-x".to_string(), "window".to_string()),
            ("other".to_string(), "elsewhere".to_string())
        ]
    );
}

/// Given a fixture whose date cannot be read,
/// When the rule runs,
/// Then it is not reported: repairing a fixture whose date is unknown is the worse mistake.
#[test]
fn a_fixture_with_an_unreadable_date_is_not_stranded() {
    let game = game_with(vec![fixture("odd", "someday", FixtureStatus::Scheduled)]);

    assert!(stranded_fixtures(&game, day("2030-09-10")).is_empty());
}
