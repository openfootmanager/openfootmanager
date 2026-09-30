//! What the extra fields on an authored player mean.
//!
//! `package.rs` declares the fields; this says what they resolve to. The contract
//! interval is the part with rules worth having in one place, because two things
//! have to agree about it: generation, which turns it into a player, and
//! validation, which tells the author when it cannot be. Both call
//! [`resolve_authored_contract`], so what counts as a valid interval is decided
//! once.

use super::package::PlayerDef;
use crate::contracts::{MAX_CONTRACT_YEARS, parse_contract_date};
use chrono::{Months, NaiveDate};

/// The interval an author wrote, resolved against the year the career opens in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct AuthoredContract {
    /// `None` when the author gave none. It is given when the career opens, from
    /// the club's own season, not here: the opening date is not known yet.
    pub start: Option<NaiveDate>,
    /// `None` when the author wrote no end and no length, and generation's own
    /// roll should stand.
    pub end: Option<NaiveDate>,
}

/// Why an authored contract cannot be resolved. Each variant is something the
/// author can fix, and each has its own translated message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ContractError {
    /// Not a real `YYYY-MM-DD` date. Carries the JSON field it was in.
    InvalidDate(&'static str),
    /// Both `contractEnd` and `contractLength`, which say the same thing two ways.
    EndAndLength,
    /// A length outside `1..=MAX_CONTRACT_YEARS`.
    LengthOutOfRange,
    /// An end on or before the start.
    EndNotAfterStart,
}

/// A date exactly as it is stored: `YYYY-MM-DD` and nothing looser. The shared
/// parse accepts `2026-7-1`, which is fine for reading a save but would let an
/// author write a date the rest of the tooling does not produce.
fn parse_authored_date(field: &'static str, value: &str) -> Result<NaiveDate, ContractError> {
    if value.len() != 10 {
        return Err(ContractError::InvalidDate(field));
    }
    parse_contract_date(value).ok_or(ContractError::InvalidDate(field))
}

fn add_years(
    start: NaiveDate,
    years: u32,
    field: &'static str,
) -> Result<NaiveDate, ContractError> {
    start
        .checked_add_months(Months::new(years.saturating_mul(12)))
        .ok_or(ContractError::InvalidDate(field))
}

/// Resolve what an author wrote about a player's contract.
///
/// - An end date is kept exactly. It is correct for the period it was written for
///   and nowhere else, which is the author's choice to make.
/// - A length counts whole years from the start when there is one, and otherwise
///   from the year the career opens in, ending on 30 June as every generated
///   contract does. That is what lets one package be played in any era.
/// - A start on its own is half an interval. Rather than invent an end, the length
///   generation would have rolled (`rolled_years`) is counted from it.
/// - Nothing at all is `Ok(AuthoredContract::default())`: generation decides.
pub(super) fn resolve_authored_contract(
    def: &PlayerDef,
    opening_year: u32,
    rolled_years: u32,
) -> Result<AuthoredContract, ContractError> {
    let start = def
        .contract_start
        .as_deref()
        .map(|value| parse_authored_date("contractStart", value))
        .transpose()?;
    let end = def
        .contract_end
        .as_deref()
        .map(|value| parse_authored_date("contractEnd", value))
        .transpose()?;

    if end.is_some() && def.contract_length.is_some() {
        return Err(ContractError::EndAndLength);
    }

    let end = match (end, def.contract_length, start) {
        (Some(end), _, _) => Some(end),
        (None, Some(length), start) => {
            if !(1..=MAX_CONTRACT_YEARS).contains(&length) {
                return Err(ContractError::LengthOutOfRange);
            }
            Some(match start {
                Some(start) => add_years(start, length, "contractStart")?,
                None => NaiveDate::from_ymd_opt(opening_year.saturating_add(length) as i32, 6, 30)
                    .ok_or(ContractError::InvalidDate("contractLength"))?,
            })
        }
        (None, None, Some(start)) => Some(add_years(start, rolled_years, "contractStart")?),
        (None, None, None) => None,
    };

    if let (Some(start), Some(end)) = (start, end)
        && end <= start
    {
        return Err(ContractError::EndNotAfterStart);
    }

    Ok(AuthoredContract { start, end })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def(extra: serde_json::Value) -> PlayerDef {
        let mut base = serde_json::json!({ "id": "p", "name": "P" });
        base.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        serde_json::from_value(base).expect("the fixture deserializes")
    }

    fn day(year: i32, month: u32, date: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, date).unwrap()
    }

    fn resolve(extra: serde_json::Value) -> Result<AuthoredContract, ContractError> {
        resolve_authored_contract(&def(extra), 2026, 3)
    }

    #[test]
    fn nothing_authored_leaves_the_roll_to_generation() {
        assert_eq!(
            resolve(serde_json::json!({})),
            Ok(AuthoredContract::default())
        );
    }

    #[test]
    fn an_end_date_is_kept_exactly() {
        let resolved = resolve(serde_json::json!({ "contractEnd": "2031-03-15" })).unwrap();
        assert_eq!(resolved.end, Some(day(2031, 3, 15)));
        assert_eq!(resolved.start, None);
    }

    #[test]
    fn a_length_with_no_start_ends_on_30_june_of_the_opening_year_plus_length() {
        let resolved = resolve(serde_json::json!({ "contractLength": 3 })).unwrap();
        assert_eq!(resolved.end, Some(day(2029, 6, 30)));
        assert_eq!(resolved.start, None);
    }

    #[test]
    fn a_length_with_a_start_counts_whole_years_from_it() {
        let resolved =
            resolve(serde_json::json!({ "contractStart": "2024-01-15", "contractLength": 2 }))
                .unwrap();
        assert_eq!(resolved.start, Some(day(2024, 1, 15)));
        assert_eq!(resolved.end, Some(day(2026, 1, 15)));
    }

    /// 29 February plus a year has no 29 February to land on. chrono clamps to the
    /// 28th, which is the only sensible answer and worth pinning.
    #[test]
    fn a_leap_day_start_lands_on_the_28th_a_year_later() {
        let resolved =
            resolve(serde_json::json!({ "contractStart": "2024-02-29", "contractLength": 1 }))
                .unwrap();
        assert_eq!(resolved.end, Some(day(2025, 2, 28)));
    }

    #[test]
    fn a_start_alone_rolls_the_generated_length_from_it() {
        // `resolve` passes a rolled length of 3.
        let resolved = resolve(serde_json::json!({ "contractStart": "2024-01-15" })).unwrap();
        assert_eq!(resolved.start, Some(day(2024, 1, 15)));
        assert_eq!(resolved.end, Some(day(2027, 1, 15)));
    }

    #[test]
    fn an_end_and_a_length_together_are_refused() {
        let resolved =
            resolve(serde_json::json!({ "contractEnd": "2030-06-30", "contractLength": 2 }));
        assert_eq!(resolved, Err(ContractError::EndAndLength));
    }

    #[test]
    fn a_length_is_held_to_the_same_limit_as_negotiation() {
        for length in [0, MAX_CONTRACT_YEARS + 1, 100] {
            assert_eq!(
                resolve(serde_json::json!({ "contractLength": length })),
                Err(ContractError::LengthOutOfRange),
                "length {length}"
            );
        }
        for length in 1..=MAX_CONTRACT_YEARS {
            assert!(
                resolve(serde_json::json!({ "contractLength": length })).is_ok(),
                "length {length}"
            );
        }
    }

    #[test]
    fn an_end_on_or_before_the_start_is_refused() {
        for end in ["2025-01-01", "2026-01-01"] {
            let resolved =
                resolve(serde_json::json!({ "contractStart": "2026-01-01", "contractEnd": end }));
            assert_eq!(resolved, Err(ContractError::EndNotAfterStart), "end {end}");
        }
    }

    #[test]
    fn a_date_that_is_not_yyyy_mm_dd_is_refused_and_named() {
        for bad in ["2026-13-45", "2026-7-1", "tomorrow", "", "26-01-01"] {
            assert_eq!(
                resolve(serde_json::json!({ "contractEnd": bad })),
                Err(ContractError::InvalidDate("contractEnd")),
                "contractEnd {bad:?}"
            );
            assert_eq!(
                resolve(serde_json::json!({ "contractStart": bad })),
                Err(ContractError::InvalidDate("contractStart")),
                "contractStart {bad:?}"
            );
        }
    }
}
