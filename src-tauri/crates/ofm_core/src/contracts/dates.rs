use chrono::{Datelike, NaiveDate};

/// New contracts end on 30 June of the signing year plus `years`.
///
/// January–June signings round their year anniversary forward to 30 June;
/// July–December signings round it back to 30 June of that same year. A signing
/// on 30 June keeps that day in the agreed future year. This preserves world
/// generation's year-based lengths and never ends a positive-length deal on its
/// signing day. Explicit dates in packages and existing saves are not rewritten.
/// Returns `None` for zero years or a date outside chrono's representable range.
pub(crate) fn contract_end_after(current_date: NaiveDate, years: u32) -> Option<NaiveDate> {
    let years = i32::try_from(years).ok().filter(|years| *years > 0)?;
    NaiveDate::from_ymd_opt(current_date.year().checked_add(years)?, 6, 30)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Given a zero-season deal, when an end is calculated, then no contract is offered.
    #[test]
    fn a_zero_year_contract_has_no_end() {
        assert_eq!(contract_end_after(NaiveDate::MIN, 0), None);
    }

    /// Given more years than fit in i32, when an end is calculated, then it is refused.
    #[test]
    fn an_unrepresentable_length_has_no_end() {
        assert_eq!(contract_end_after(NaiveDate::MIN, u32::MAX), None);
    }

    /// Given a signing in chrono's last year, when a year is added, then it is refused.
    #[test]
    fn a_contract_beyond_the_date_range_has_no_end() {
        assert_eq!(contract_end_after(NaiveDate::MAX, 1), None);
    }
}
