//! The one rule for when a knockout round is played.

use crate::calendar_booking::too_close;
use chrono::{Days, NaiveDate};
use std::collections::BTreeSet;

/// How far past its earliest date a round is searched for: bounded only so that a defect
/// cannot hang a load; by the rulings it must never be reached.
pub const SEARCH_CAP_DAYS: u64 = 730;

/// The first date a knockout round may be played: no earlier than the cup's own grid date and
/// the previous round's actual last match plus the round gap, not on a blackout date and not
/// within the rest window of `other_cup_dates`: the dates other cups play the clubs of this
/// round, which is for the caller to choose — unrelated cups must not be in it.
pub fn knockout_round_date(
    grid_date: NaiveDate,
    previous_round_last: Option<NaiveDate>,
    round_gap_days: u32,
    blackout: &BTreeSet<NaiveDate>,
    other_cup_dates: &BTreeSet<NaiveDate>,
) -> Option<NaiveDate> {
    let after_previous = previous_round_last
        .and_then(|last| last.checked_add_days(Days::new(u64::from(round_gap_days))));
    let earliest = after_previous.map_or(grid_date, |date| date.max(grid_date));
    (0..=SEARCH_CAP_DAYS)
        .filter_map(|offset| earliest.checked_add_days(Days::new(offset)))
        .find(|date| {
            !blackout.contains(date)
                && !other_cup_dates.iter().any(|other| too_close(*date, *other))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn none() -> BTreeSet<NaiveDate> {
        BTreeSet::new()
    }

    /// Given a previous round that finished late, when the next round is dated, then it follows the
    /// actual last match by the gap, not the grid date.
    #[test]
    fn a_round_follows_the_actual_end_of_the_previous_one() {
        let date = knockout_round_date(
            day(2033, 10, 1),
            Some(day(2033, 10, 20)),
            14,
            &none(),
            &none(),
        );
        assert_eq!(date, Some(day(2033, 11, 3)));
    }

    /// Given a previous round that finished early, when the next round is dated, then the grid date holds.
    #[test]
    fn the_grid_date_holds_when_the_previous_round_was_early() {
        let date = knockout_round_date(
            day(2033, 12, 1),
            Some(day(2033, 10, 20)),
            14,
            &none(),
            &none(),
        );
        assert_eq!(date, Some(day(2033, 12, 1)));
    }

    /// Given a blackout over the first days, when a round is dated, then it lands on the first day after.
    #[test]
    fn a_round_moves_off_a_blackout() {
        let blackout: BTreeSet<_> = (1..=4).map(|d| day(2033, 11, d)).collect();
        let date = knockout_round_date(day(2033, 11, 1), None, 14, &blackout, &none());
        assert_eq!(date, Some(day(2033, 11, 5)));
    }

    /// Given another cup's round nearby, when a round is dated, then it keeps the rest window from it.
    #[test]
    fn a_round_keeps_the_rest_window_from_another_cups_round() {
        let others = BTreeSet::from([day(2033, 11, 1)]);
        let date = knockout_round_date(day(2033, 11, 1), None, 14, &none(), &others);
        assert_eq!(date, Some(day(2033, 11, 4)));
    }

    /// Given a calendar blocked past the search cap, when a round is dated, then there is no date: a bug, never a spill.
    #[test]
    fn a_blocked_calendar_returns_none_instead_of_hanging() {
        let blackout: BTreeSet<_> = (0..=SEARCH_CAP_DAYS)
            .map(|d| day(2033, 1, 1) + Days::new(d))
            .collect();
        assert_eq!(
            knockout_round_date(day(2033, 1, 1), None, 0, &blackout, &none()),
            None
        );
    }
}
