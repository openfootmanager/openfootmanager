//! Repairing the calendar of a save when it is loaded: a save written before the calendar rules
//! (the rest between a club's matches, the international windows) may carry matches that clash.

use crate::calendar_notice;
use crate::calendar_reconcile;
use crate::game::Game;

/// Settles every unplayed clash in a loaded game, once, and tells the user about their club's
/// moved matches. Returns how many fixtures moved or were dropped, so the caller knows to save.
///
/// Everything in a loaded save was shown to the player, so every move is recorded as a
/// postponement. Played matches are history: they are never moved, and two played matches that
/// clash are left alone. The first day to be played is the game's date, so a match dated that
/// day may move. Idempotent: a settled calendar changes nothing and announces nothing.
pub fn repair_loaded_calendar(game: &mut Game) -> usize {
    let first_day_to_play = game.clock.current_date.date_naive();
    let blackout =
        crate::national_team::international_blackout_around(&game.competitions, first_day_to_play);
    let fixtures_before: usize = game.competitions.iter().map(|c| c.fixtures.len()).sum();

    let moved = calendar_reconcile::settle_calendar(
        &mut game.competitions,
        &blackout,
        first_day_to_play,
        |_| true,
    );

    let fixtures_after: usize = game.competitions.iter().map(|c| c.fixtures.len()).sum();
    let dropped = fixtures_before.saturating_sub(fixtures_after);
    let changed = moved.len() + dropped;
    if changed > 0 {
        game.sync_legacy_league();
        calendar_notice::announce_moved_matches(
            game,
            &format!("postponed_load_{first_day_to_play}"),
            &moved,
        );
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use chrono::{TimeZone, Utc};
    use domain::league::{Fixture, FixtureCompetition, FixtureStatus, League, MatchResult};
    use domain::manager::Manager;

    fn game_on(year: i32, month: u32, day: u32, user_club: &str) -> Game {
        let mut manager = Manager::new(
            "m".into(),
            "A".into(),
            "B".into(),
            "1980-01-01".into(),
            "ENG".into(),
        );
        manager.hire(user_club.to_string());
        let clock = GameClock::new(Utc.with_ymd_and_hms(year, month, day, 12, 0, 0).unwrap());
        Game::new(clock, manager, vec![], vec![], vec![], vec![])
    }

    fn fixture(competition: &str, id: &str, date: &str, home: &str, away: &str) -> Fixture {
        Fixture {
            id: id.into(),
            competition_id: competition.into(),
            date: date.into(),
            home_team_id: home.into(),
            away_team_id: away.into(),
            competition: if competition == "cup" {
                FixtureCompetition::Cup
            } else {
                FixtureCompetition::League
            },
            ..Default::default()
        }
    }

    fn played(mut fixture: Fixture) -> Fixture {
        fixture.status = FixtureStatus::Completed;
        fixture.result = Some(MatchResult::default());
        fixture
    }

    fn competition(id: &str, priority: u32, fixtures: Vec<Fixture>) -> League {
        let mut league = League::new(id.into(), id.into(), 2033, &[]);
        league.priority = priority;
        league.fixtures = fixtures;
        league
    }

    /// A save written by the old calendar: the user's club has a league match and a cup tie two
    /// days apart, and a second league match on the cup tie's day.
    fn clashing_save() -> Game {
        let mut game = game_on(2033, 3, 1, "a");
        game.competitions = vec![
            competition(
                "league",
                1,
                vec![
                    fixture("league", "l1", "2033-03-10", "a", "z"),
                    fixture("league", "l2", "2033-03-12", "a", "y"),
                ],
            ),
            competition("cup", 2, vec![fixture("cup", "t1", "2033-03-12", "a", "b")]),
        ];
        game
    }

    fn closest_unplayed_gap(game: &Game) -> i64 {
        let mut dates: Vec<chrono::NaiveDate> = game
            .competitions
            .iter()
            .flat_map(|c| &c.fixtures)
            .filter(|f| {
                f.status == FixtureStatus::Scheduled
                    && (f.home_team_id == "a" || f.away_team_id == "a")
            })
            .map(|f| chrono::NaiveDate::parse_from_str(&f.date, "%Y-%m-%d").unwrap())
            .collect();
        dates.sort();
        dates
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).num_days())
            .min()
            .unwrap_or(i64::MAX)
    }

    /// Given a save with unplayed matches inside the rest window, When it is repaired, Then none
    /// is left.
    #[test]
    fn repair_removes_every_unplayed_clash() {
        let mut game = clashing_save();
        assert!(closest_unplayed_gap(&game) < 3, "the premise: a clash");

        let changed = repair_loaded_calendar(&mut game);

        assert!(changed > 0);
        assert!(closest_unplayed_gap(&game) >= crate::calendar_booking::MIN_GAP_DAYS);
    }

    /// Given a repaired save, When it is repaired again, Then nothing changes and nobody is told
    /// twice.
    #[test]
    fn repair_is_idempotent() {
        let mut game = clashing_save();
        repair_loaded_calendar(&mut game);
        let once = serde_json::to_string(&game.competitions).unwrap();
        let messages = game.messages.len();

        let again = repair_loaded_calendar(&mut game);

        assert_eq!(again, 0);
        assert_eq!(serde_json::to_string(&game.competitions).unwrap(), once);
        assert_eq!(game.messages.len(), messages);
    }

    /// Given two matches already played inside the rest window, and a clash among unplayed ones,
    /// When the save is repaired, Then the played matches keep their dates: history is not ours.
    #[test]
    fn played_fixtures_are_never_touched() {
        let mut game = clashing_save();
        game.competitions[0].fixtures.push(played(fixture(
            "league",
            "old1",
            "2033-02-10",
            "a",
            "z",
        )));
        game.competitions[1]
            .fixtures
            .push(played(fixture("cup", "old2", "2033-02-11", "a", "b")));

        repair_loaded_calendar(&mut game);

        for (competition, id, date) in [
            ("league", "old1", "2033-02-10"),
            ("cup", "old2", "2033-02-11"),
        ] {
            let found = game
                .competitions
                .iter()
                .find(|c| c.id == competition)
                .and_then(|c| c.fixtures.iter().find(|f| f.id == id))
                .unwrap();
            assert_eq!(found.date, date, "{id} moved");
            assert!(found.postponement.is_none());
        }
    }

    /// Given a clash that moves the user's own match, When the save is repaired, Then the match is
    /// marked postponed from the date the player saw, and the user is told exactly once.
    #[test]
    fn a_repaired_published_fixture_is_marked_and_announced_once() {
        let mut game = clashing_save();

        repair_loaded_calendar(&mut game);
        repair_loaded_calendar(&mut game);

        let moved: Vec<&Fixture> = game
            .competitions
            .iter()
            .flat_map(|c| &c.fixtures)
            .filter(|f| f.postponement.is_some())
            .collect();
        assert!(!moved.is_empty(), "something moved, and was marked");
        for fixture in moved {
            assert_ne!(
                fixture.postponement.as_ref().unwrap().original_date,
                fixture.date
            );
        }
        let told = game
            .messages
            .iter()
            .filter(|m| m.subject_key.as_deref() == Some("be.msg.matchesMoved.subject"))
            .count();
        assert_eq!(told, 1);
    }

    /// Given a clash that only involves other clubs, When the save is repaired, Then the user is
    /// told nothing.
    #[test]
    fn a_repair_of_other_clubs_matches_tells_the_user_nothing() {
        let mut game = game_on(2033, 3, 1, "me");
        game.competitions = vec![
            competition(
                "league",
                1,
                vec![fixture("league", "l1", "2033-03-10", "a", "z")],
            ),
            competition("cup", 2, vec![fixture("cup", "t1", "2033-03-10", "a", "b")]),
        ];

        let changed = repair_loaded_calendar(&mut game);

        assert!(changed > 0);
        assert!(game.messages.is_empty());
    }

    /// Given a save with nothing to repair and a legacy mirror that differs from the competitions,
    /// When it is repaired, Then nothing is touched, the mirror included: a clean load is not a write.
    #[test]
    fn a_repair_with_nothing_to_do_leaves_the_game_alone() {
        let mut game = game_on(2033, 3, 1, "a");
        game.competitions = vec![competition(
            "league",
            1,
            vec![fixture("league", "l1", "2033-03-10", "a", "z")],
        )];
        let mut stale_mirror = game.competitions[0].clone();
        stale_mirror.name = "a mirror that is not synced".to_string();
        game.league = Some(stale_mirror);

        let changed = repair_loaded_calendar(&mut game);

        assert_eq!(changed, 0);
        assert_eq!(
            game.league.as_ref().unwrap().name,
            "a mirror that is not synced"
        );
    }

    /// Given a friendly that clashes with a league match, When the save is repaired, Then the
    /// friendly is dropped and counted: the caller must save.
    #[test]
    fn a_dropped_friendly_is_counted_so_the_repair_is_saved() {
        let mut game = game_on(2033, 3, 1, "a");
        let mut friendly = fixture("league", "f1", "2033-03-10", "a", "y");
        friendly.competition = FixtureCompetition::Friendly;
        game.competitions = vec![competition(
            "league",
            1,
            vec![fixture("league", "l1", "2033-03-10", "a", "z"), friendly],
        )];

        let changed = repair_loaded_calendar(&mut game);

        assert_eq!(changed, 1);
        assert!(game.competitions[0].fixtures.iter().all(|f| f.id != "f1"));
    }
}
