//! Shared contract negotiation logic used by both contract renewal and transfer personal terms.
//!
//! This module extracts the core wage and contract length expectation logic
//! that is reused across different negotiation contexts (renewals, free agent signings, transfers).

use chrono::{Datelike, Months, NaiveDate};
use domain::player::Player;
use domain::team::Team;

/// Constants for wage calculation
pub const MARKET_VALUE_TO_WAGE_RATIO: u64 = 200;
pub const MINIMUM_DEFAULT_WAGE: u64 = 500;
pub const MAX_CONTRACT_YEARS: u32 = 5;

/// Calculate the expected wage a player would demand based on their profile and team context.
///
/// This considers:
/// - Current wage (as baseline)
/// - Age (younger players demand more, older players less)
/// - Morale (low morale = higher demands)
/// - Player importance (higher market value = higher multiplier)
/// - Team reputation (low reputation clubs pay premium)
/// - Contract time remaining (expiring soon = higher demands)
pub fn expected_wage(player: &Player, team: &Team, current_date: NaiveDate) -> u32 {
    let mut wage = reference_player_wage(player) as f32;
    let age = player_age_on(current_date, &player.date_of_birth);
    let remaining_days = remaining_contract_days(player, current_date);

    // Age adjustments
    if age <= 27 {
        wage *= 1.05;
    } else if age >= 32 {
        wage *= 0.95;
    }

    // Morale adjustment
    if player.morale <= 50 {
        wage *= 1.10;
    }

    // Importance multiplier based on market value
    wage *= importance_wage_multiplier(player);

    // Team reputation adjustment
    if team.reputation < 40 {
        wage *= 1.05;
    }

    // Contract expiry pressure
    if remaining_days <= 180 {
        wage *= 1.10;
    } else if remaining_days <= 365 {
        wage *= 1.05;
    }

    let rounded = round_up_to_nearest_thousand(wage.ceil() as u32);
    rounded.max(reference_player_wage(player))
}

/// Get the reference wage for a player - their current wage or derived from market value.
pub fn reference_player_wage(player: &Player) -> u32 {
    if player.wage > 0 {
        return player.wage;
    }

    let derived_wage = (player.market_value / MARKET_VALUE_TO_WAGE_RATIO).max(MINIMUM_DEFAULT_WAGE);

    round_up_to_nearest_thousand(derived_wage.min(u32::MAX as u64) as u32)
}

/// Calculate the wage multiplier based on player importance (market value).
fn importance_wage_multiplier(player: &Player) -> f32 {
    if player.market_value >= 2_000_000 {
        return 1.18;
    }

    if player.market_value >= 750_000 {
        return 1.10;
    }

    if player.market_value <= 150_000 {
        return 0.95;
    }

    1.0
}

/// Calculate the expected contract length in years based on player age.
pub fn expected_contract_years(player: &Player, current_date: NaiveDate) -> u32 {
    let age = player_age_on(current_date, &player.date_of_birth);

    if age <= 28 {
        return 3;
    }

    if age <= 32 {
        return 2;
    }

    1
}

/// Calculate the minimum acceptable wage (85% of current wage).
pub fn minimum_acceptable_wage(current_wage: u32) -> u32 {
    ((current_wage as f32) * 0.85).floor() as u32
}

/// Check if a wage offer is insulting (below 65% of the anchor wage).
/// The anchor wage is the maximum of reference wage and expected wage.
pub fn is_insulting_wage_offer(reference_wage: u32, expected_wage: u32, offered_wage: u32) -> bool {
    let anchor_wage = reference_wage.max(expected_wage);
    let insulting_floor = ((anchor_wage as f32) * 0.65).floor() as u32;

    offered_wage < insulting_floor
}

/// Calculate the player's age on a given date.
pub fn player_age_on(current_date: NaiveDate, date_of_birth: &str) -> i32 {
    let Ok(dob) = NaiveDate::parse_from_str(date_of_birth, "%Y-%m-%d") else {
        return 30;
    };

    let mut age = current_date.year() - dob.year();
    if current_date.ordinal() < dob.ordinal() {
        age -= 1;
    }
    age
}

/// Calculate remaining days on the player's current contract.
pub fn remaining_contract_days(player: &Player, current_date: NaiveDate) -> i64 {
    contract_days_remaining(player.contract_end.as_deref(), current_date)
        .unwrap_or(0)
        .max(0)
}

/// Round up a value to the nearest thousand.
pub fn round_up_to_nearest_thousand(value: u32) -> u32 {
    if value == 0 {
        return 0;
    }

    value.div_ceil(1000) * 1000
}

/// Calculate days remaining on a contract.
pub fn contract_days_remaining(contract_end: Option<&str>, current_date: NaiveDate) -> Option<i64> {
    let contract_end = contract_end?;
    let contract_end_date = NaiveDate::parse_from_str(contract_end, "%Y-%m-%d").ok()?;
    Some((contract_end_date - current_date).num_days())
}

/// Calculate the contract end date given a start date and number of years.
pub fn calculate_contract_end_date(start_date: NaiveDate, years: u32) -> Option<NaiveDate> {
    start_date.checked_add_months(Months::new(years * 12))
}

/// Validate that contract years are within acceptable bounds.
pub fn validate_contract_years(years: u32) -> bool {
    years > 0 && years <= MAX_CONTRACT_YEARS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimum_acceptable_wage() {
        assert_eq!(minimum_acceptable_wage(10000), 8500);
        assert_eq!(minimum_acceptable_wage(5000), 4250);
    }

    #[test]
    fn test_is_insulting_wage_offer() {
        // Offer below 65% of max(reference, expected) is insulting
        assert!(is_insulting_wage_offer(10000, 10000, 6000)); // 6000 < 6500
        assert!(!is_insulting_wage_offer(10000, 10000, 6500)); // 6500 == 6500
        assert!(!is_insulting_wage_offer(10000, 10000, 7000)); // 7000 > 6500
    }

    #[test]
    fn test_round_up_to_nearest_thousand() {
        assert_eq!(round_up_to_nearest_thousand(1500), 2000);
        assert_eq!(round_up_to_nearest_thousand(1000), 1000);
        assert_eq!(round_up_to_nearest_thousand(500), 1000);
        assert_eq!(round_up_to_nearest_thousand(0), 0);
    }

    #[test]
    fn test_player_age_on() {
        let current_date = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        
        // Before birthday this year
        assert_eq!(player_age_on(current_date, "1995-07-01"), 29);
        // After birthday this year
        assert_eq!(player_age_on(current_date, "1995-01-01"), 30);
    }
}
