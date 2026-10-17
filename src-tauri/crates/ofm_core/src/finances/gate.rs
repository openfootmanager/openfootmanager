//! Gate income: every club is paid for each home fixture it completes, in any competition.
//!
//! A gate is keyed on its fixture, not on the week it falls in: the post's id is derived from the
//! fixture id, so the journal itself records which fixtures have been paid. A fixture is paid once
//! whatever day payday runs, across a rollover, and again only if its first post never landed.

use std::collections::HashSet;

use chrono::{Duration, NaiveDate};
use domain::finance::CashKind;
use domain::league::{Fixture, FixtureStatus};
use rand::RngExt;

use super::journal::{PostRequest, post_all};
use crate::game::Game;

const GATE_POST_PREFIX: &str = "gate-";

/// The legacy Monday paid the home fixtures of the six days before it and of the day itself. The
/// first settlement on a save that predates per-fixture gates reaches back the same distance, so
/// the week straddling the cutover is paid once and nothing before it is paid retroactively.
const LEGACY_GATE_WINDOW_DAYS: i64 = 6;

fn gate_post_id(fixture_id: &str) -> String {
    format!("{GATE_POST_PREFIX}{fixture_id}")
}

/// What `host_capacity` seats take for `fixture`. A pure function of the fixture and the seed, so
/// the finance snapshot's forecast and payday's post cannot disagree. `None` when it cannot be
/// priced (see [`gate_takings`]).
pub fn expected_gate(game: &Game, host_capacity: u32, fixture: &Fixture) -> Option<i64> {
    let mut rng = game.rng_for(&format!("finances/gate/{}", fixture.id), &fixture.date);
    let attendance_percent: i64 = rng.random_range(60..=92);
    let ticket_price: i64 = rng.random_range(15..=25);
    gate_takings(host_capacity, attendance_percent, ticket_price)
}

/// Capacity times the share of it that came times the price, in whole euros. `None` for a negative
/// price or a product past `i64`: such a gate is not posted at all, never posted clamped.
pub fn gate_takings(capacity: u32, attendance_percent: i64, ticket_price: i64) -> Option<i64> {
    if ticket_price < 0 {
        return None;
    }
    if capacity == 0 || attendance_percent <= 0 {
        return Some(0);
    }
    i64::from(capacity)
        .checked_mul(attendance_percent)
        .and_then(|crowd_percent| crowd_percent.checked_mul(ticket_price))
        .map(|takings| takings / 100)
}

/// The gates `team_id` is due over the next seven days, counting today, from the fixtures it has
/// not played yet.
pub fn expected_gate_next_week(game: &Game, team_id: &str, host_capacity: u32) -> i64 {
    let today = game.clock.current_date.date_naive();
    let week_end = today + Duration::days(7);
    home_fixtures(game, team_id)
        .filter(|fixture| fixture.status != FixtureStatus::Completed)
        .filter(|fixture| {
            fixture_date(fixture).is_some_and(|date| date >= today && date < week_end)
        })
        .filter_map(|fixture| expected_gate(game, host_capacity, fixture))
        .sum()
}

/// Pay every club for each home fixture it has completed and not yet been paid for.
///
/// Each fixture is its own journal batch: a gate that cannot be posted leaves the rest posted and
/// itself unpaid, so the next settlement tries it again.
pub fn settle_gate_income(game: &mut Game, today: NaiveDate) {
    let settled_from = settlement_start(game, today);
    let paid = paid_fixture_ids(game);
    let mut due = Vec::new();
    let mut seen = HashSet::new();
    for competition in game.competitions_in_play() {
        for fixture in &competition.fixtures {
            let is_due = fixture.status == FixtureStatus::Completed
                && fixture.result.is_some()
                && fixture_date(fixture).is_some_and(|date| date >= settled_from && date <= today)
                && !paid.contains(fixture.id.as_str())
                && seen.insert(fixture.id.clone());
            if is_due {
                due.push(fixture.clone());
            }
        }
    }

    for fixture in due {
        let Some(capacity) = game
            .teams
            .iter()
            .find(|team| team.id == fixture.home_team_id)
            .map(|team| team.stadium_capacity)
        else {
            continue;
        };
        let Some(takings) = expected_gate(game, capacity, &fixture) else {
            log::error!("gate for fixture {} cannot be priced", fixture.id);
            continue;
        };
        let request = PostRequest::new(&fixture.home_team_id, takings, CashKind::Matchday, today)
            .with_id(gate_post_id(&fixture.id));
        if let Err(err) = post_all(game, std::slice::from_ref(&request)) {
            log::error!("gate post for fixture {} failed: {err}", fixture.id);
        }
    }
}

/// The first date a completed fixture can be paid for. Fixed the first time gates are settled and
/// saved with the game, so a fixture played before per-fixture gates existed is never paid.
fn settlement_start(game: &mut Game, today: NaiveDate) -> NaiveDate {
    let stored = game
        .gate_settled_from
        .as_deref()
        .and_then(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d").ok());
    if let Some(date) = stored {
        return date;
    }
    let start = today - Duration::days(LEGACY_GATE_WINDOW_DAYS);
    game.gate_settled_from = Some(start.format("%Y-%m-%d").to_string());
    start
}

fn paid_fixture_ids(game: &Game) -> HashSet<&str> {
    game.cash_journal
        .iter()
        .filter(|post| post.kind == CashKind::Matchday)
        .filter_map(|post| post.id.strip_prefix(GATE_POST_PREFIX))
        .collect()
}

fn home_fixtures<'a>(game: &'a Game, team_id: &'a str) -> impl Iterator<Item = &'a Fixture> {
    game.competitions_in_play()
        .iter()
        .flat_map(|competition| competition.fixtures.iter())
        .filter(move |fixture| fixture.home_team_id == team_id)
}

fn fixture_date(fixture: &Fixture) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(&fixture.date, "%Y-%m-%d").ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use chrono::{TimeZone, Utc};
    use domain::league::{FixtureCompetition, League, MatchResult, Postponement, RescheduleReason};
    use domain::manager::Manager;
    use domain::team::Team;

    const CAPACITY: u32 = 30_000;

    fn team(id: &str) -> Team {
        let mut team = Team::new(
            id.to_string(),
            id.to_string(),
            id[..3].to_string(),
            "England".to_string(),
            "Testville".to_string(),
            format!("{id} Ground"),
            CAPACITY,
        );
        team.finance = 1_000_000;
        team
    }

    fn game_on(year: i32, month: u32, day: u32) -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(year, month, day, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "mgr-user".to_string(),
            "Alex".to_string(),
            "Boss".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("user".to_string());
        let teams = ["user", "rival", "hosts", "guests"].map(team).to_vec();
        Game::new(clock, manager, teams, vec![], vec![], vec![])
    }

    /// 2025-06-16 is a Monday.
    fn monday() -> Game {
        game_on(2025, 6, 16)
    }

    fn played(id: &str, date: &str, home: &str, away: &str) -> Fixture {
        Fixture {
            id: id.to_string(),
            date: date.to_string(),
            home_team_id: home.to_string(),
            away_team_id: away.to_string(),
            status: FixtureStatus::Completed,
            result: Some(MatchResult::default()),
            ..Default::default()
        }
    }

    fn scheduled(id: &str, date: &str, home: &str, away: &str) -> Fixture {
        Fixture {
            status: FixtureStatus::Scheduled,
            result: None,
            ..played(id, date, home, away)
        }
    }

    fn competition(id: &str, fixtures: Vec<Fixture>) -> League {
        League {
            id: id.to_string(),
            fixtures,
            ..Default::default()
        }
    }

    fn user_league() -> League {
        competition(
            "user-league",
            vec![played("u1", "2025-06-14", "user", "rival")],
        )
    }

    fn gates_of<'a>(game: &'a Game, club: &str) -> Vec<&'a domain::finance::CashPost> {
        game.cash_journal
            .iter()
            .filter(|post| post.club_id == club && post.kind == CashKind::Matchday)
            .collect()
    }

    fn settle_today(game: &mut Game) {
        let today = game.clock.current_date.date_naive();
        settle_gate_income(game, today);
    }

    fn gate_for(game: &Game, fixture_id: &str) -> i64 {
        let fixture = game
            .competitions_in_play()
            .iter()
            .flat_map(|competition| competition.fixtures.iter())
            .find(|fixture| fixture.id == fixture_id)
            .unwrap();
        expected_gate(game, CAPACITY, fixture).unwrap()
    }

    /// Given a club outside the user's league with a completed home league fixture on Saturday
    ///       and a completed home cup fixture on Wednesday,
    /// When payday runs,
    /// Then it receives exactly two `Matchday` posts, each the expected gate of its fixture.
    #[test]
    fn a_club_outside_the_users_league_is_paid_for_every_home_fixture_in_any_competition() {
        let mut game = monday();
        let mut cup = competition("cup", vec![played("c1", "2025-06-11", "hosts", "guests")]);
        cup.fixtures[0].competition = FixtureCompetition::Cup;
        game.competitions = vec![
            user_league(),
            competition(
                "other-league",
                vec![played("l1", "2025-06-14", "hosts", "guests")],
            ),
            cup,
        ];
        game.sync_legacy_league();

        crate::finances::process_weekly_finances(&mut game);

        let mut paid: Vec<i64> = gates_of(&game, "hosts")
            .iter()
            .map(|post| post.amount)
            .collect();
        let mut expected = vec![gate_for(&game, "l1"), gate_for(&game, "c1")];
        paid.sort_unstable();
        expected.sort_unstable();
        assert_eq!(paid, expected);
        assert!(expected.iter().all(|gate| *gate > 0));
    }

    /// Given an away win, a home fixture postponed to Wednesday and a completed home fixture,
    /// When payday runs, and the postponed fixture is played and the next payday runs,
    /// Then only the completed home fixture pays the first time, and the postponed one pays once
    ///      after it is played.
    #[test]
    fn away_and_postponed_fixtures_pay_nothing_until_a_home_fixture_is_played() {
        let mut game = monday();
        let mut postponed = scheduled("moved", "2025-06-18", "hosts", "rival");
        postponed.postponement = Some(Postponement {
            original_date: "2025-06-14".to_string(),
            reason: RescheduleReason::CupTie,
        });
        game.competitions = vec![competition(
            "league",
            vec![
                played("away", "2025-06-13", "guests", "hosts"),
                postponed,
                played("home", "2025-06-15", "hosts", "guests"),
            ],
        )];

        settle_today(&mut game);
        let first: Vec<i64> = gates_of(&game, "hosts")
            .iter()
            .map(|post| post.amount)
            .collect();
        assert_eq!(first, vec![gate_for(&game, "home")]);

        let moved = &mut game.competitions[0].fixtures[1];
        moved.status = FixtureStatus::Completed;
        moved.result = Some(MatchResult::default());
        let next_monday = NaiveDate::from_ymd_opt(2025, 6, 23).unwrap();
        settle_gate_income(&mut game, next_monday);
        settle_gate_income(&mut game, next_monday);

        assert_eq!(gates_of(&game, "hosts").len(), 2);
        assert_eq!(gates_of(&game, "hosts")[1].amount, gate_for(&game, "moved"));
    }

    /// Given home fixtures on the last day of one season and the first day of the next,
    /// When gates are settled at the rollover and again on the Monday after it,
    /// Then each fixture is paid exactly once.
    #[test]
    fn a_fixture_is_paid_once_across_the_rollover() {
        let mut game = game_on(2025, 6, 15);
        game.competitions = vec![competition(
            "league",
            vec![played("last", "2025-06-15", "hosts", "guests")],
        )];
        settle_today(&mut game);

        game.competitions = vec![competition(
            "league",
            vec![
                played("last", "2025-06-15", "hosts", "guests"),
                played("first", "2025-06-16", "hosts", "guests"),
            ],
        )];
        let monday = NaiveDate::from_ymd_opt(2025, 6, 16).unwrap();
        settle_gate_income(&mut game, monday);
        settle_gate_income(&mut game, monday);

        let ids: Vec<&str> = gates_of(&game, "hosts")
            .iter()
            .map(|post| post.id.as_str())
            .collect();
        assert_eq!(ids, vec!["gate-last", "gate-first"]);
    }

    /// Given an Apertura and a Clausura that both list the same club,
    /// When each of its home fixtures is completed and payday runs,
    /// Then each pays once.
    #[test]
    fn each_half_of_a_split_season_pays_its_own_home_fixtures() {
        let mut game = monday();
        game.competitions = vec![
            competition(
                "apertura",
                vec![played("a1", "2025-06-12", "hosts", "guests")],
            ),
            competition(
                "clausura",
                vec![played("c1", "2025-06-15", "hosts", "rival")],
            ),
        ];

        settle_today(&mut game);
        settle_today(&mut game);

        let ids: Vec<&str> = gates_of(&game, "hosts")
            .iter()
            .map(|post| post.id.as_str())
            .collect();
        assert_eq!(ids, vec!["gate-a1", "gate-c1"]);
    }

    /// Given an empty stadium or nobody coming, Then the gate is 0; given a negative price or a
    /// product past `i64`, Then the gate is refused rather than posted.
    #[test]
    fn the_gate_is_zero_for_an_empty_ground_and_refused_when_it_cannot_be_priced() {
        assert_eq!(gate_takings(0, 80, 20), Some(0));
        assert_eq!(gate_takings(30_000, 0, 20), Some(0));
        assert_eq!(gate_takings(30_000, 80, 20), Some(480_000));
        assert_eq!(gate_takings(30_000, 80, -1), None);
        assert_eq!(gate_takings(u32::MAX, i64::MAX / 2, 20), None);
    }

    /// Given one home fixture in the next seven days,
    /// When the snapshot is read before it is played and payday runs after,
    /// Then the snapshot's expected gate equals the posted amount to the euro.
    #[test]
    fn the_snapshot_forecasts_exactly_what_payday_posts() {
        let mut game = monday();
        game.competitions = vec![competition(
            "league",
            vec![scheduled("next", "2025-06-21", "user", "rival")],
        )];
        game.sync_legacy_league();
        let snapshot = crate::finances::team_finance_snapshot(&game, "user").unwrap();
        let forecast = snapshot.weekly_recurring_income - snapshot.weekly_sponsor_income;

        let fixture = &mut game.competitions[0].fixtures[0];
        fixture.status = FixtureStatus::Completed;
        fixture.result = Some(MatchResult::default());
        game.clock = GameClock::new(Utc.with_ymd_and_hms(2025, 6, 23, 12, 0, 0).unwrap());
        settle_today(&mut game);

        assert!(forecast > 0);
        assert_eq!(gates_of(&game, "user")[0].amount, forecast);
    }

    /// Given the same state twice, When payday runs on each, Then every gate is identical.
    #[test]
    fn the_same_state_pays_the_same_gates() {
        let run = || {
            let mut game = monday();
            game.competitions = vec![competition(
                "league",
                vec![
                    played("g1", "2025-06-12", "hosts", "guests"),
                    played("g2", "2025-06-15", "user", "rival"),
                ],
            )];
            settle_today(&mut game);
            game.cash_journal
                .iter()
                .filter(|post| post.kind == CashKind::Matchday)
                .map(|post| (post.id.clone(), post.amount))
                .collect::<Vec<_>>()
        };

        assert_eq!(run(), run());
        assert_eq!(run().len(), 2);
    }

    /// Given a completed home fixture whose post cannot be committed (the club's cash would
    /// overflow),
    /// When payday runs, the club is repaired and payday runs again,
    /// Then the failed attempt did not mark the fixture paid, and it is paid once.
    #[test]
    fn a_gate_that_failed_to_post_is_paid_by_the_next_settlement() {
        let mut game = monday();
        game.competitions = vec![competition(
            "league",
            vec![played("f1", "2025-06-14", "hosts", "guests")],
        )];
        let hosts = game
            .teams
            .iter()
            .position(|team| team.id == "hosts")
            .unwrap();
        game.teams[hosts].finance = i64::MAX - 1;

        settle_today(&mut game);
        assert!(gates_of(&game, "hosts").is_empty());

        game.teams[hosts].finance = 0;
        settle_today(&mut game);
        settle_today(&mut game);

        assert_eq!(gates_of(&game, "hosts").len(), 1);
    }

    /// Given a save from before per-fixture gates, with home fixtures played three weeks ago and
    ///       on Saturday,
    /// When it is opened and the next two paydays run,
    /// Then Saturday's fixture is paid, the old one never is, and the cutover date is kept.
    #[test]
    fn a_save_from_before_per_fixture_gates_is_not_paid_retroactively() {
        let mut game = monday();
        assert_eq!(game.gate_settled_from, None);
        game.competitions = vec![competition(
            "league",
            vec![
                played("old", "2025-05-24", "hosts", "guests"),
                played("recent", "2025-06-14", "hosts", "guests"),
            ],
        )];

        settle_today(&mut game);
        settle_gate_income(&mut game, NaiveDate::from_ymd_opt(2025, 6, 23).unwrap());

        let ids: Vec<&str> = gates_of(&game, "hosts")
            .iter()
            .map(|post| post.id.as_str())
            .collect();
        assert_eq!(ids, vec!["gate-recent"]);
        assert_eq!(game.gate_settled_from.as_deref(), Some("2025-06-10"));
    }
}
