//! The daily calendar sweep: after a day's football, any fixture that now sits inside another
//! match's rest window, or on an international window, is moved to the earliest legal day. The
//! user hears about it once, in one message, when it touches their own club.

use std::collections::{BTreeSet, HashSet};

use crate::calendar_notice;
use crate::calendar_reconcile;
use crate::game::Game;
use chrono::{Datelike, NaiveDate};

/// Fixtures of competitions with a knockout bracket, as the day found them: a round drawn during
/// the day is new, so its dates were never shown and moving one is not a postponement.
pub(super) type ShownFixtures = HashSet<(String, String)>;

fn has_bracket(competition: &domain::league::League) -> bool {
    !competition.knockout_rounds.is_empty()
}

pub(super) fn shown_bracket_fixtures(game: &Game) -> ShownFixtures {
    game.competitions
        .iter()
        .filter(|competition| has_bracket(competition))
        .flat_map(|competition| {
            competition
                .fixtures
                .iter()
                .map(|fixture| (competition.id.clone(), fixture.id.clone()))
        })
        .collect()
}

/// Dates club football keeps clear of around `today`: the windows of the season that started the
/// year before and of the one starting this year, since a date belongs to either.
fn blackout_dates(game: &Game, today: NaiveDate) -> BTreeSet<NaiveDate> {
    let qualifying = game
        .competitions
        .iter()
        .any(crate::world_cup::is_world_cup_qualifying);
    [today.year() - 1, today.year()]
        .into_iter()
        .filter_map(|year| NaiveDate::from_ymd_opt(year, 7, 1))
        .filter_map(|start| start.and_hms_opt(0, 0, 0))
        .flat_map(|start| {
            crate::national_team::international_blackout_dates(
                chrono::DateTime::from_naive_utc_and_offset(start, chrono::Utc),
                qualifying,
            )
        })
        .collect()
}

/// The id and date of every fixture, played or not, and the dates the calendar must keep clear
/// of: a result changes neither, and neither does anything else a day does to a fixture. Only a
/// fixture added or moved, or a window that has come into force, can create a clash, so a day that
/// left all of this alone needs no sweep. That is nearly every day, and settling the whole
/// calendar costs milliseconds.
fn calendar_fingerprint(game: &Game, blackout: &BTreeSet<NaiveDate>) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for competition in &game.competitions {
        competition.id.hash(&mut hasher);
        for fixture in &competition.fixtures {
            fixture.id.hash(&mut hasher);
            fixture.date.hash(&mut hasher);
        }
    }
    blackout.hash(&mut hasher);
    hasher.finish()
}

/// Settles the world's calendar at the end of `today`, and tells the user about their own club's
/// moved matches. A calendar that cannot be settled is a defect: it is logged and left as it is.
/// Returns whether it ran: it is skipped when nothing it looks at has changed since it last did.
pub(super) fn reconcile_calendar(game: &mut Game, today: &str, shown: &ShownFixtures) -> bool {
    let Ok(day) = NaiveDate::parse_from_str(today, "%Y-%m-%d") else {
        return false;
    };
    let blackout = blackout_dates(game, day);
    let before = calendar_fingerprint(game, &blackout);
    if game.calendar_fingerprint == Some(before) {
        return false;
    }
    let moved_after_being_shown = settle_calendar_today(game, day, &blackout, shown);
    let after = calendar_fingerprint(game, &blackout);
    if after != before {
        // Something moved or was dropped, announced or not: the mirror the dashboard reads has to
        // follow. A sweep that changed nothing leaves it alone.
        game.sync_legacy_league();
    }
    calendar_notice::announce_moved_matches(
        game,
        &format!("postponed_{today}"),
        &moved_after_being_shown,
    );
    game.calendar_fingerprint = Some(after);
    true
}

/// Settles the calendar for the days from tomorrow on; returns the moves of fixtures the player
/// had been shown.
fn settle_calendar_today(
    game: &mut Game,
    day: NaiveDate,
    blackout: &BTreeSet<NaiveDate>,
    shown: &ShownFixtures,
) -> Vec<calendar_reconcile::Move> {
    let Some(tomorrow) = day.succ_opt() else {
        return Vec::new();
    };
    let bracket_competitions: HashSet<String> = game
        .competitions
        .iter()
        .filter(|competition| has_bracket(competition))
        .map(|competition| competition.id.clone())
        .collect();
    calendar_reconcile::settle_calendar(&mut game.competitions, blackout, tomorrow, |planned| {
        !bracket_competitions.contains(&planned.competition_id)
            || shown.contains(&(planned.competition_id.clone(), planned.fixture_id.clone()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use chrono::{TimeZone, Utc};
    use domain::league::{
        CompetitionFormat, Fixture, FixtureCompetition, KnockoutRoundState, League,
    };
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

    fn league(fixtures: Vec<Fixture>) -> League {
        let mut league = League::new("league".into(), "League".into(), 2033, &[]);
        league.priority = 1;
        league.fixtures = fixtures;
        league
    }

    fn cup(rounds: Vec<Vec<Fixture>>) -> League {
        let mut cup = League::new("cup".into(), "Cup".into(), 2033, &[]);
        cup.priority = 2;
        cup.rules.format = CompetitionFormat::Knockout;
        cup.rules.knockout_round_gap_days = 3;
        cup.knockout_rounds = rounds
            .iter()
            .enumerate()
            .map(|(index, fixtures)| KnockoutRoundState {
                id: format!("cup-round-{index}"),
                fixture_ids: fixtures.iter().map(|f| f.id.clone()).collect(),
                ..Default::default()
            })
            .collect();
        cup.fixtures = rounds.into_iter().flatten().collect();
        cup
    }

    fn date_of(game: &Game, competition: &str, id: &str) -> String {
        game.competitions
            .iter()
            .find(|c| c.id == competition)
            .and_then(|c| c.fixtures.iter().find(|f| f.id == id))
            .map(|f| f.date.clone())
            .unwrap_or_else(|| panic!("{competition}/{id} is missing"))
    }

    fn moved_message_count(game: &Game) -> usize {
        game.messages
            .iter()
            .filter(|m| m.subject_key.as_deref() == Some("be.msg.matchesMoved.subject"))
            .count()
    }

    /// Given a cup tie drawn into a round the player has already been shown, that clashes with
    /// a league match of the user's club,
    /// When the day's sweep runs,
    /// Then the league match moves (marked and announced), and the cup tie keeps its date.
    #[test]
    fn a_cup_draw_displaces_the_league_match_not_the_cup_tie() {
        let mut game = game_on(2033, 3, 1, "a");
        game.competitions = vec![
            league(vec![fixture("league", "l1", "2033-03-10", "a", "z")]),
            cup(vec![vec![fixture("cup", "t1", "2033-03-10", "a", "b")]]),
        ];
        let shown = shown_bracket_fixtures(&game);

        reconcile_calendar(&mut game, "2033-03-01", &shown);

        assert_eq!(date_of(&game, "cup", "t1"), "2033-03-10");
        assert_ne!(date_of(&game, "league", "l1"), "2033-03-10");
        let moved = &game.competitions[0].fixtures[0];
        assert_eq!(
            moved.postponement.as_ref().unwrap().original_date,
            "2033-03-10"
        );
        assert_eq!(moved_message_count(&game), 1);
    }

    /// Given a round drawn today, whose tie clashes with an international window,
    /// When the sweep runs, Then the tie moves, but its first date was never shown: nothing is
    /// recorded as postponed and the user is not told.
    #[test]
    fn a_tie_moved_the_day_it_is_drawn_is_not_marked_postponed() {
        let mut game = game_on(2033, 9, 1, "a");
        game.competitions = vec![cup(vec![vec![fixture(
            "cup",
            "t1",
            "2033-09-09",
            "a",
            "b",
        )]])];
        // The sweep's snapshot predates the draw: nothing was shown.
        let shown = ShownFixtures::new();

        reconcile_calendar(&mut game, "2033-09-01", &shown);

        assert_ne!(date_of(&game, "cup", "t1"), "2033-09-09");
        assert!(game.competitions[0].fixtures[0].postponement.is_none());
        assert_eq!(moved_message_count(&game), 0);
    }

    /// Given a tie of a round that was already shown, moved by a window,
    /// When the sweep runs, Then it is marked postponed with the window as the reason and the
    /// user, whose club plays it, hears once.
    #[test]
    fn a_shown_tie_moved_off_a_window_is_marked_and_announced() {
        let mut game = game_on(2033, 9, 1, "a");
        game.competitions = vec![cup(vec![vec![fixture(
            "cup",
            "t1",
            "2033-09-09",
            "a",
            "b",
        )]])];
        let shown = shown_bracket_fixtures(&game);

        reconcile_calendar(&mut game, "2033-09-01", &shown);

        let tie = &game.competitions[0].fixtures[0];
        assert_ne!(tie.date, "2033-09-09");
        assert_eq!(
            tie.postponement.as_ref().unwrap().reason,
            domain::league::RescheduleReason::InternationalWindow
        );
        assert_eq!(moved_message_count(&game), 1);
    }

    /// Given a round whose feeder ties were moved later, and a successor round drawn today at the
    /// round gap after the feeders' old dates,
    /// When the sweep runs, Then the successor waits for the feeders' new last date plus the gap.
    #[test]
    fn a_round_drawn_after_moved_feeders_waits_for_them() {
        let mut game = game_on(2033, 9, 1, "z");
        game.competitions = vec![cup(vec![
            vec![fixture("cup", "r1", "2033-09-09", "a", "b")],
            vec![fixture("cup", "r2", "2033-09-12", "c", "d")],
        ])];
        let shown = ShownFixtures::new();

        reconcile_calendar(&mut game, "2033-09-01", &shown);

        let feeder = date_of(&game, "cup", "r1");
        let successor = date_of(&game, "cup", "r2");
        assert_ne!(feeder, "2033-09-09", "the feeder was on the window");
        let days = |s: &str| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap();
        assert!(days(&successor) >= days(&feeder) + chrono::Duration::days(3));
    }

    /// Given a calendar with no clash, When the sweep runs, Then nothing changes and nobody is told.
    #[test]
    fn the_sweep_changes_nothing_on_a_clean_calendar() {
        let mut game = game_on(2033, 3, 1, "a");
        game.competitions = vec![
            league(vec![fixture("league", "l1", "2033-03-10", "a", "z")]),
            cup(vec![vec![fixture("cup", "t1", "2033-03-20", "a", "b")]]),
        ];
        let before = serde_json::to_string(&game.competitions).unwrap();

        let shown = shown_bracket_fixtures(&game);
        reconcile_calendar(&mut game, "2033-03-01", &shown);

        assert_eq!(serde_json::to_string(&game.competitions).unwrap(), before);
        assert_eq!(moved_message_count(&game), 0);
    }

    /// Given a clash dated tomorrow, When the day is processed (batch path) or finished around the
    /// player's own match (live path), Then both run the sweep and the clash is resolved.
    #[test]
    fn both_ways_of_ending_a_day_run_the_sweep() {
        let clash = || {
            let mut game = game_on(2033, 3, 1, "a");
            game.competitions = vec![
                league(vec![fixture("league", "l1", "2033-03-02", "a", "z")]),
                cup(vec![vec![fixture("cup", "t1", "2033-03-02", "a", "b")]]),
            ];
            game
        };

        let mut batch = clash();
        super::super::process_day(&mut batch);
        assert_ne!(date_of(&batch, "league", "l1"), "2033-03-02", "process_day");

        let mut live = clash();
        super::super::finish_live_match_day(&mut live);
        assert_ne!(
            date_of(&live, "league", "l1"),
            "2033-03-02",
            "finish_live_match_day"
        );
    }

    /// Given a dormant competition's draw that clashes with an active league match, When the
    /// day's sweep runs, Then the league match moves, whatever the scope.
    #[test]
    fn a_dormant_cup_draw_displaces_an_active_league_fixture() {
        let mut game = game_on(2033, 3, 1, "a");
        game.competitions = vec![
            league(vec![fixture("league", "l1", "2033-03-10", "a", "z")]),
            cup(vec![vec![fixture("cup", "t1", "2033-03-10", "a", "b")]]),
        ];
        game.active_competition_ids = vec!["league".to_string()];

        let shown = shown_bracket_fixtures(&game);
        reconcile_calendar(&mut game, "2033-03-01", &shown);

        assert_ne!(date_of(&game, "league", "l1"), "2033-03-10");
        assert_eq!(date_of(&game, "cup", "t1"), "2033-03-10");
    }

    /// Given a calendar the sweep has settled and no fixture added or moved since,
    /// When the next day's sweep is asked for, Then it does no work; once a round is drawn it does.
    #[test]
    fn the_sweep_runs_only_when_the_calendar_changed() {
        let mut game = game_on(2033, 3, 1, "a");
        game.competitions = vec![
            league(vec![fixture("league", "l1", "2033-03-10", "a", "z")]),
            cup(vec![vec![fixture("cup", "t1", "2033-03-10", "a", "b")]]),
        ];
        let shown = shown_bracket_fixtures(&game);

        assert!(
            reconcile_calendar(&mut game, "2033-03-01", &shown),
            "first sweep"
        );
        assert!(
            !reconcile_calendar(&mut game, "2033-03-02", &shown),
            "nothing changed"
        );

        game.competitions[1]
            .fixtures
            .push(fixture("cup", "t2", "2033-03-20", "c", "d"));
        assert!(
            reconcile_calendar(&mut game, "2033-03-03", &shown),
            "a draw was made"
        );

        // A result changes neither the id nor the date of a fixture: no sweep.
        game.competitions[1].fixtures[0].status = domain::league::FixtureStatus::Completed;
        game.competitions[1].fixtures[0].result = Some(domain::league::MatchResult::default());
        assert!(
            !reconcile_calendar(&mut game, "2033-03-04", &shown),
            "only a result"
        );

        // Anything that moves a fixture, whatever moved it, is seen.
        game.competitions[0].fixtures[0].date = "2033-03-21".to_string();
        assert!(
            reconcile_calendar(&mut game, "2033-03-05", &shown),
            "a date changed"
        );

        // A rollover replaces a competition's fixtures with new ones.
        game.competitions[0].fixtures.clear();
        game.competitions[0].fixtures.push(fixture(
            "league",
            "next-season",
            "2033-08-20",
            "a",
            "z",
        ));
        assert!(
            reconcile_calendar(&mut game, "2033-03-06", &shown),
            "regenerated"
        );
    }

    /// Given a tie drawn today that the sweep moves, and a user whose dashboard mirrors that cup,
    /// When the sweep runs, Then the mirror has the new date, though nobody is told.
    #[test]
    fn a_move_nobody_is_told_about_still_reaches_the_mirror() {
        let mut game = game_on(2033, 9, 1, "z");
        game.competitions = vec![cup(vec![vec![fixture(
            "cup",
            "t1",
            "2033-09-09",
            "a",
            "b",
        )]])];
        game.league = Some(game.competitions[0].clone());

        reconcile_calendar(&mut game, "2033-09-01", &ShownFixtures::new());

        let mirrored = &game.league.as_ref().unwrap().fixtures[0].date;
        assert_ne!(mirrored, "2033-09-09");
        assert_eq!(mirrored, &game.competitions[0].fixtures[0].date);
    }

    /// Given a calendar nobody has touched, When a new year brings new windows into force, Then
    /// the sweep runs again, because what it keeps clear of has changed.
    #[test]
    fn a_window_coming_into_force_is_a_change() {
        let mut game = game_on(2033, 12, 30, "a");
        game.competitions = vec![league(vec![fixture(
            "league",
            "l1",
            "2034-03-10",
            "a",
            "z",
        )])];
        let shown = ShownFixtures::new();

        assert!(
            reconcile_calendar(&mut game, "2033-12-30", &shown),
            "first sweep"
        );
        assert!(
            !reconcile_calendar(&mut game, "2033-12-31", &shown),
            "same windows"
        );
        assert!(
            reconcile_calendar(&mut game, "2034-01-02", &shown),
            "a new year of windows"
        );
    }

    /// Given a league match on a window that belongs to the season that began the year before
    /// (24 March), and a day in January,
    /// When the sweep runs, Then the match moves off the window.
    #[test]
    fn the_windows_of_the_season_that_began_last_year_are_kept_clear() {
        let mut game = game_on(2034, 1, 10, "z");
        game.competitions = vec![league(vec![fixture(
            "league",
            "l1",
            "2034-03-24",
            "a",
            "b",
        )])];

        reconcile_calendar(&mut game, "2034-01-10", &ShownFixtures::new());

        assert_ne!(date_of(&game, "league", "l1"), "2034-03-24");
    }

    /// Given a league match the sweep moves that was not in the day's snapshot of bracket fixtures,
    /// When the sweep runs, Then it is still recorded as postponed and announced: only a round
    /// drawn the same day is unshown, and a league's dates are always shown.
    #[test]
    fn a_league_match_is_always_a_shown_date() {
        let mut game = game_on(2033, 3, 1, "a");
        game.competitions = vec![
            league(vec![fixture("league", "l1", "2033-03-10", "a", "z")]),
            cup(vec![vec![fixture("cup", "t1", "2033-03-10", "a", "b")]]),
        ];

        reconcile_calendar(&mut game, "2033-03-01", &ShownFixtures::new());

        assert!(game.competitions[0].fixtures[0].postponement.is_some());
        assert_eq!(moved_message_count(&game), 1);
    }

    /// Given a sweep that has run, When the calendar changes (a draw), Then the sweep runs
    /// again: the cache key is the calendar, not merely "has run once".
    #[test]
    fn a_changed_calendar_is_never_mistaken_for_the_one_already_swept() {
        let mut game = game_on(2033, 3, 1, "a");
        game.competitions = vec![league(vec![fixture(
            "league",
            "l1",
            "2033-03-10",
            "a",
            "z",
        )])];
        let shown = ShownFixtures::new();
        assert!(reconcile_calendar(&mut game, "2033-03-01", &shown));

        game.competitions.push(cup(vec![vec![fixture(
            "cup",
            "t1",
            "2033-03-10",
            "a",
            "b",
        )]]));

        assert!(reconcile_calendar(&mut game, "2033-03-02", &shown));
        assert_ne!(date_of(&game, "league", "l1"), "2033-03-10");
    }
}
