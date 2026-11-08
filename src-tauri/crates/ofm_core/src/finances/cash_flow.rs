//! A club's weekly cash flow, read from the cash journal: the one record of every movement of its
//! money (wages, gates, sponsorship, prize money, transfer and loan fees, campaigns, board support).
//! The Finances chart draws this; it sums nothing of its own (#1204).

use std::collections::BTreeMap;

use chrono::{Datelike, Duration, NaiveDate};
use domain::finance::CashKind;
use serde::Serialize;

use crate::game::Game;

/// The weeks the chart shows: the latest twelve in which the club's money moved.
pub const CASH_FLOW_WEEKS: usize = 12;

/// One week of a club's cash flow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WeeklyCashFlow {
    /// The Monday the week starts on, `YYYY-MM-DD`.
    pub week_start: String,
    /// Money in that week, in euros.
    pub income: i64,
    /// Money out that week, as a positive number of euros.
    pub expenses: i64,
}

/// The club's latest [`CASH_FLOW_WEEKS`] weeks of movement, oldest first, from the journal. The
/// opening balance is left out: it is where the money started, not a movement of it.
pub fn weekly_cash_flow(game: &Game, club_id: &str) -> Vec<WeeklyCashFlow> {
    let mut weeks: BTreeMap<NaiveDate, (i64, i64)> = BTreeMap::new();
    for post in game
        .cash_journal
        .iter()
        .filter(|post| post.club_id == club_id && post.kind != CashKind::OpeningBalance)
    {
        // Journal dates are written from the game clock as `YYYY-MM-DD`; one that isn't can't be
        // placed in a week, and leaving it out is all the chart can do with it.
        let Ok(date) = NaiveDate::parse_from_str(&post.date, "%Y-%m-%d") else {
            continue;
        };
        let monday = date - Duration::days(i64::from(date.weekday().num_days_from_monday()));
        let (income, expenses) = weeks.entry(monday).or_default();
        if post.amount >= 0 {
            *income = income.saturating_add(post.amount);
        } else {
            *expenses = expenses.saturating_add(post.amount.saturating_neg());
        }
    }
    let skip = weeks.len().saturating_sub(CASH_FLOW_WEEKS);
    weeks
        .into_iter()
        .skip(skip)
        .map(|(monday, (income, expenses))| WeeklyCashFlow {
            week_start: monday.format("%Y-%m-%d").to_string(),
            income,
            expenses,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use crate::finances::journal::{PostRequest, post_all};
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::team::Team;

    fn team(id: &str) -> Team {
        let mut team = Team::new(
            id.to_string(),
            id.to_string(),
            id[..3].to_string(),
            "England".to_string(),
            "Testville".to_string(),
            "Ground".to_string(),
            22_000,
        );
        team.finance = 1_000_000;
        team
    }

    /// A game on Monday 16 February 2026, managing "alpha", with "bravo" as another club, and
    /// one player at "alpha" on a wage.
    fn game() -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 2, 16, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "mgr".to_string(),
            "Alex".to_string(),
            "Boss".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("alpha".to_string());
        let mut player = domain::player::Player::new(
            "p1".to_string(),
            "P1".to_string(),
            "Player One".to_string(),
            "1998-01-01".to_string(),
            "England".to_string(),
            domain::player::Position::CentralMidfielder,
            crate::test_support::uniform_attributes(60),
        );
        player.team_id = Some("alpha".to_string());
        player.stage_wage(52_000);
        Game::new(
            clock,
            manager,
            vec![team("alpha"), team("bravo")],
            vec![player],
            vec![],
            vec![],
        )
    }

    fn day(d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 2, d).unwrap()
    }

    fn post(game: &mut Game, club: &str, amount: i64, kind: CashKind, date: NaiveDate) {
        post_all(game, &[PostRequest::new(club, amount, kind, date)]).expect("posted");
    }

    /// Given wages and a gate in the week of Monday 9 February, and sponsorship, a transfer fee
    /// received and more wages in the week of Monday 16 February
    /// When the club's cash flow is read
    /// Then there is one entry per week, oldest first, each splitting its money in and out: the
    ///      wages, gates, sponsorship and fees the chart used to leave out are all in it.
    #[test]
    fn each_week_sums_every_movement_the_journal_holds() {
        let mut game = game();
        post(&mut game, "alpha", -40_000, CashKind::PlayerWages, day(9));
        post(&mut game, "alpha", 25_000, CashKind::Matchday, day(14));
        post(&mut game, "alpha", 10_000, CashKind::Sponsorship, day(16));
        post(
            &mut game,
            "alpha",
            300_000,
            CashKind::TransferFeeIn,
            day(18),
        );
        post(&mut game, "alpha", -40_000, CashKind::PlayerWages, day(16));

        let flow = weekly_cash_flow(&game, "alpha");

        assert_eq!(
            flow,
            vec![
                WeeklyCashFlow {
                    week_start: "2026-02-09".into(),
                    income: 25_000,
                    expenses: 40_000
                },
                WeeklyCashFlow {
                    week_start: "2026-02-16".into(),
                    income: 310_000,
                    expenses: 40_000
                },
            ]
        );
    }

    /// Given one sponsorship payment to the club (its first post, which seeds the club's opening
    ///       balance in the journal the same week) and another club's wages
    /// When the club's cash flow is read
    /// Then only the sponsorship is in it: where the money started is not a movement, and another
    ///      club's money is not this club's.
    #[test]
    fn the_opening_balance_and_other_clubs_are_left_out() {
        let mut game = game();
        post(&mut game, "bravo", -90_000, CashKind::PlayerWages, day(16));
        post(&mut game, "alpha", 10_000, CashKind::Sponsorship, day(16));
        assert!(
            game.cash_journal
                .iter()
                .any(|post| post.club_id == "alpha" && post.kind == CashKind::OpeningBalance),
            "the first post seeds the club's opening balance"
        );

        let flow = weekly_cash_flow(&game, "alpha");

        assert_eq!(
            flow,
            vec![WeeklyCashFlow {
                week_start: "2026-02-16".into(),
                income: 10_000,
                expenses: 0
            }]
        );
    }

    /// Given wages posted every Monday for fifteen weeks
    /// When the club's cash flow is read
    /// Then it holds the latest twelve weeks, oldest first, ending on the latest.
    #[test]
    fn only_the_latest_twelve_weeks_are_kept() {
        let mut game = game();
        let first = NaiveDate::from_ymd_opt(2025, 11, 10).unwrap();
        for week in 0..15 {
            post(
                &mut game,
                "alpha",
                -1_000,
                CashKind::PlayerWages,
                first + Duration::weeks(week),
            );
        }

        let flow = weekly_cash_flow(&game, "alpha");

        assert_eq!(flow.len(), CASH_FLOW_WEEKS);
        assert_eq!(
            flow[0].week_start,
            (first + Duration::weeks(3)).format("%Y-%m-%d").to_string()
        );
        assert_eq!(
            flow.last().map(|week| week.week_start.clone()),
            Some((first + Duration::weeks(14)).format("%Y-%m-%d").to_string())
        );
    }

    /// Given a club on a Monday, when the week's finances are processed (its wages, upkeep and
    /// sponsorship posted, as the game does every Monday)
    /// When its cash flow is read
    /// Then this week's expenses are exactly what the season's expense figure (on the same screen)
    ///      went up by: the chart and the totals read the same journal.
    #[test]
    fn the_week_agrees_with_the_seasons_expenses_after_a_monday() {
        let mut game = game();
        let before = game.teams[0].season_expenses;

        crate::finances::process_weekly_finances(&mut game);

        let spent = game.teams[0].season_expenses - before;
        assert!(spent > 0, "a Monday should post some expense for the club");
        let flow = weekly_cash_flow(&game, "alpha");
        assert_eq!(flow.last().map(|week| week.expenses), Some(spent));
    }

    /// Given a club whose wages and a sponsorship payment have been posted
    /// When the Finances screen's snapshot is taken
    /// Then it carries the club's weekly cash flow, the same weeks the journal sums to: the chart
    ///      draws what the snapshot hands it.
    #[test]
    fn the_finance_snapshot_carries_the_weekly_cash_flow() {
        let mut game = game();
        post(&mut game, "alpha", -40_000, CashKind::PlayerWages, day(9));
        post(&mut game, "alpha", 10_000, CashKind::Sponsorship, day(16));

        let snapshot = crate::finances::team_finance_snapshot(&game, "alpha").expect("a snapshot");

        assert_eq!(snapshot.cash_flow.len(), 2);
        assert_eq!(snapshot.cash_flow, weekly_cash_flow(&game, "alpha"));
    }
}
