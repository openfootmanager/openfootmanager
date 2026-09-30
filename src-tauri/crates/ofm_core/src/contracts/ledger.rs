//! Writing a contract into a player's history.
//!
//! Every route that makes, renews or ends a contract builds its entry here and
//! hands it to [`record_movement`], so there is one place that knows what a
//! contract entry looks like. The ledger itself, and the rule that the current
//! contract is its latest entry, live in `domain::contract_ledger`.

use super::{expected_contract_years, expected_wage};
use chrono::{Months, NaiveDate};
use domain::contract_ledger::{ContractRecord, ContractSource};
use domain::player::{Player, PlayerMovementEntry, PlayerMovementKind};
use domain::team::Team;

/// A stored date: `YYYY-MM-DD`.
pub(crate) fn iso(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

/// The terms of a contract signed `start` and running to `end`.
pub(crate) fn contract_record(
    start: NaiveDate,
    end: NaiveDate,
    weekly_wage: u32,
    source: ContractSource,
) -> ContractRecord {
    ContractRecord {
        start: Some(iso(start)),
        end: Some(iso(end)),
        weekly_wage,
        source,
    }
}

/// An entry for a contract made with `club`, on `date`.
pub(crate) fn contract_entry(
    kind: PlayerMovementKind,
    date: NaiveDate,
    club: &Team,
    record: ContractRecord,
) -> PlayerMovementEntry {
    PlayerMovementEntry {
        to_team_id: Some(club.id.clone()),
        to_team_name: Some(club.name.clone()),
        contract: Some(record),
        ..PlayerMovementEntry::new(iso(date), kind)
    }
}

/// Add an entry to the ledger. The one call every writer goes through.
///
/// The ledger refuses a contract that ends on or before the day it starts. Every
/// route here builds its end from its start, so that cannot happen from inside the
/// game; if it ever does, the contract is left as it was rather than corrupted, and
/// a debug build says so.
pub(crate) fn record_movement(player: &mut Player, entry: PlayerMovementEntry) {
    if let Err(problem) = player.record_movement(entry) {
        debug_assert!(false, "a contract entry was refused: {problem:?}");
    }
}

/// What a club offers a player it has just bought: its standard terms, the same
/// model a renewal uses. A bid that named a wage keeps it; a bid that named none
/// (every bid today) gets the wage the player would expect from that club.
///
/// `None` only when the end date cannot be calculated.
pub(crate) fn standard_contract_terms(
    player: &Player,
    buying_club: &Team,
    today: NaiveDate,
    offered_wage: u32,
) -> Option<(u32, NaiveDate)> {
    let wage = if offered_wage > 0 {
        offered_wage
    } else {
        expected_wage(player, buying_club, today)
    };
    let end = today.checked_add_months(Months::new(expected_contract_years(player, today) * 12))?;
    Some((wage, end))
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::player::{PlayerAttributes, Position};

    fn player(date_of_birth: &str) -> Player {
        let mut player = Player::new(
            "p".to_string(),
            "P".to_string(),
            "Player".to_string(),
            date_of_birth.to_string(),
            "England".to_string(),
            Position::Forward,
            PlayerAttributes {
                pace: 50,
                stamina: 50,
                strength: 50,
                agility: 50,
                passing: 50,
                shooting: 50,
                tackling: 50,
                dribbling: 50,
                defending: 50,
                positioning: 50,
                vision: 50,
                decisions: 50,
                composure: 50,
                aggression: 50,
                teamwork: 50,
                leadership: 50,
                handling: 20,
                reflexes: 20,
                aerial: 50,
            },
        );
        player.market_value = 1_000_000;
        player.morale = 70;
        player
    }

    fn club() -> Team {
        Team::new(
            "club".to_string(),
            "Club FC".to_string(),
            "CLB".to_string(),
            "England".to_string(),
            "City".to_string(),
            "Ground".to_string(),
            10_000,
        )
    }

    fn day(year: i32, month: u32, date: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, date).unwrap()
    }

    #[test]
    fn a_bid_that_named_a_wage_keeps_it() {
        let (wage, _) =
            standard_contract_terms(&player("2000-01-01"), &club(), day(2026, 8, 1), 12_345)
                .unwrap();
        assert_eq!(wage, 12_345);
    }

    #[test]
    fn a_bid_that_named_no_wage_gets_the_wage_the_player_expects() {
        let player = player("2000-01-01");
        let (wage, _) = standard_contract_terms(&player, &club(), day(2026, 8, 1), 0).unwrap();
        assert_eq!(wage, expected_wage(&player, &club(), day(2026, 8, 1)));
        assert!(wage > 0, "a zero offered wage must never become his pay");
    }

    #[test]
    fn the_length_follows_the_age_the_renewal_model_uses() {
        let today = day(2026, 8, 1);
        for (born, end) in [
            ("2000-01-01", day(2029, 8, 1)), // 26: three years
            ("1996-01-01", day(2028, 8, 1)), // 30: two years
            ("1990-01-01", day(2027, 8, 1)), // 36: one year
        ] {
            let (_, got) = standard_contract_terms(&player(born), &club(), today, 0).unwrap();
            assert_eq!(got, end, "born {born}");
        }
    }
}
