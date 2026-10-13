//! How old someone is on a given day: the one rule every caller shares.

use chrono::{Datelike, NaiveDate};

/// The age the game assumes for a birth date it cannot read, so a damaged record reads as an
/// experienced player rather than a youth.
pub const UNKNOWN_BIRTH_AGE: i32 = 30;

/// Whole years between `born` and `on`: the birthday counts from its own day. Compared by
/// month and day, never by day-of-year, which is off by one after 28 February across a leap
/// year.
pub fn age_between(born: NaiveDate, on: NaiveDate) -> i32 {
    let had_birthday = (on.month(), on.day()) >= (born.month(), born.day());
    on.year() - born.year() - i32::from(!had_birthday)
}

/// The age on `on` of someone born on `date_of_birth` (`YYYY-MM-DD`), or `None` when the date
/// cannot be read.
pub fn age_on(on: NaiveDate, date_of_birth: &str) -> Option<i32> {
    let born = NaiveDate::parse_from_str(date_of_birth, "%Y-%m-%d").ok()?;
    Some(age_between(born, on))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    /// Given a birthday, when it is the day before, then he is still the younger age.
    #[test]
    fn the_day_before_a_birthday_he_is_still_the_younger_age() {
        assert_eq!(age_between(day(2000, 6, 15), day(2026, 6, 14)), 25);
    }

    /// Given a birthday, when it is the day itself, then he has turned.
    #[test]
    fn on_the_birthday_he_has_turned() {
        assert_eq!(age_between(day(2000, 6, 15), day(2026, 6, 15)), 26);
    }

    /// Given a birthday after 28 February in a leap year, when it is that day in a common year,
    /// then he has turned: day-of-year would say he has not (60 against 61).
    #[test]
    fn a_birthday_after_february_in_a_leap_year_is_counted_on_its_own_day() {
        assert_eq!(age_between(day(2000, 3, 1), day(2026, 3, 1)), 26);
        assert_eq!(age_between(day(2000, 12, 31), day(2026, 12, 31)), 26);
    }

    /// Given the reverse case, when a common-year birthday falls in a leap year, then he has not
    /// turned a day early.
    #[test]
    fn a_common_year_birthday_is_not_early_in_a_leap_year() {
        assert_eq!(age_between(day(1999, 3, 1), day(2024, 2, 29)), 24);
        assert_eq!(age_between(day(1999, 3, 1), day(2024, 3, 1)), 25);
    }

    /// Given someone born on 29 February, when it is 28 February or 1 March of a common year,
    /// then he turns on 1 March.
    #[test]
    fn a_leap_day_birthday_is_reached_on_the_first_of_march() {
        assert_eq!(age_between(day(2000, 2, 29), day(2026, 2, 28)), 25);
        assert_eq!(age_between(day(2000, 2, 29), day(2026, 3, 1)), 26);
    }

    /// Given a birth date as text, when it is read, then the age follows; unreadable text has none.
    #[test]
    fn a_birth_date_is_read_from_text_and_garbage_has_no_age() {
        assert_eq!(age_on(day(2026, 3, 1), "2000-03-01"), Some(26));
        assert_eq!(age_on(day(2026, 3, 1), "not a date"), None);
        assert_eq!(age_on(day(2026, 3, 1), ""), None);
    }
}
