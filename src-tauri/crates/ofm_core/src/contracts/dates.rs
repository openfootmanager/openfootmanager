use chrono::{Datelike, NaiveDate};

use super::{MAX_CONTRACT_YEARS, parse_contract_date};

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

/// A missing old end imposes no constraint; an unreadable stored date cannot
/// safely be replaced by an agreement whose ordering cannot be checked.
pub(crate) fn extends_contract(current_end: Option<&str>, proposed_end: NaiveDate) -> bool {
    current_end.is_none_or(|end| {
        parse_contract_date(end).is_some_and(|current_end| proposed_end > current_end)
    })
}

/// The shortest permitted term whose June end is strictly after the current end.
pub(crate) fn minimum_renewal_years(
    current_date: NaiveDate,
    current_end: Option<&str>,
) -> Option<u32> {
    (1..=MAX_CONTRACT_YEARS).find(|&years| {
        contract_end_after(current_date, years)
            .is_some_and(|end| extends_contract(current_end, end))
    })
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
