//! A player's contract history, kept in the movement ledger.
//!
//! `Player::movement_history` is the one record of a player's employment. An entry
//! that *establishes* a contract carries a [`ContractRecord`]; the player's current
//! contract is the latest such entry, unless a later release or retirement ended it.
//! Loans are movements that do not touch the contract: a loaned player is still on
//! his parent club's agreement.
//!
//! `Player::wage`, `contract_start` and `contract_end` are that latest entry's terms,
//! kept in step by [`Player::record_movement`], which is the only thing that adds to
//! the ledger. "Latest" is append order, never date order: a release and a new
//! signing can share a day.

use crate::player::{Player, PlayerMovementEntry, PlayerMovementKind};
use serde::{Deserialize, Serialize};

/// How a contract came to exist. Mirrors the `ContractSource` in the contract spec
/// (#350), so the ledger can become that entity's history without a rewrite.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContractSource {
    /// In force when the career opened.
    Initial,
    /// Made from a save that predates the ledger: the old columns said there was a
    /// contract, so it is recorded once, with whatever start they knew.
    LegacyMigrated,
    Renewal,
    /// A signing from no club.
    FreeAgent,
    /// A move between clubs, permanent or by exercising a loan's buy option.
    Transfer,
}

/// Why a player left his club's books.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseReason {
    Expired,
    Terminated,
}

/// The terms of one agreement.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContractRecord {
    /// `None` is an honest unknown (a save from before starts were recorded), never
    /// "expired" and never a guessed date.
    #[serde(default)]
    pub start: Option<String>,
    #[serde(default)]
    pub end: Option<String>,
    /// Per week.
    pub weekly_wage: u32,
    pub source: ContractSource,
}

/// Why an entry was not added to the ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerError {
    /// A contract that ends on or before the day it starts.
    EndNotAfterStart,
}

impl Player {
    /// The one way an entry joins the ledger. It also brings `wage`,
    /// `contract_start` and `contract_end` into line with the current contract, so
    /// they cannot disagree with it.
    pub fn record_movement(&mut self, entry: PlayerMovementEntry) -> Result<(), LedgerError> {
        if let Some(record) = &entry.contract
            && let (Some(start), Some(end)) = (&record.start, &record.end)
            && end <= start
        {
            // `YYYY-MM-DD` orders the same as a date does, so no parse is needed.
            return Err(LedgerError::EndNotAfterStart);
        }
        let changes_contract = entry.contract.is_some()
            || matches!(
                entry.kind,
                PlayerMovementKind::Released | PlayerMovementKind::Retired
            );
        self.movement_history.push(entry);
        // Only an entry that can change the contract may move the flat fields: a loan
        // passing through must leave them as they are.
        if changes_contract {
            self.sync_contract_fields();
        }
        Ok(())
    }

    /// The contract in force: the latest entry that carries one, unless a release or
    /// retirement came after it. Entries for a loan carry none and are passed over.
    pub fn current_contract(&self) -> Option<&ContractRecord> {
        for entry in self.movement_history.iter().rev() {
            if matches!(
                entry.kind,
                PlayerMovementKind::Released | PlayerMovementKind::Retired
            ) {
                return None;
            }
            if let Some(record) = &entry.contract {
                return Some(record);
            }
        }
        None
    }

    pub fn contract_start(&self) -> Option<&str> {
        self.stored_contract_start.as_deref()
    }

    pub fn contract_end(&self) -> Option<&str> {
        self.stored_contract_end.as_deref()
    }

    pub fn wage(&self) -> u32 {
        self.stored_wage
    }

    /// Stage the contract world generation gave a player, before he has a history.
    /// It is the contract he opens a career on: `Player::open_initial_contract`
    /// turns it into his first ledger entry. Nothing else should call this once a
    /// player has a ledger; record a movement instead.
    pub fn stage_contract(&mut self, start: Option<String>, end: Option<String>, wage: u32) {
        self.stored_contract_start = start;
        self.stored_contract_end = end;
        self.stored_wage = wage;
    }

    fn sync_contract_fields(&mut self) {
        let (start, end, wage) = match self.current_contract() {
            Some(record) => (record.start.clone(), record.end.clone(), record.weekly_wage),
            None => (None, None, 0),
        };
        self.stored_contract_start = start;
        self.stored_contract_end = end;
        self.stored_wage = wage;
    }

    /// Give a player loaded from a save that predates the ledger one entry for the
    /// contract its old columns describe. Does nothing when the ledger already holds
    /// a contract, when he has no club, or when there is no contract to describe, so
    /// it is safe to call on every load.
    pub fn adopt_legacy_contract(&mut self) {
        // Dated by the start the old columns knew, and empty when they did not: an
        // unknown, not a made-up day.
        let start = self.stored_contract_start.clone();
        let _ = self.adopt_staged_contract(
            ContractSource::LegacyMigrated,
            start.clone().unwrap_or_default(),
            start,
        );
    }

    /// Record the contract a player opens a career on: the one world generation gave
    /// him (or his package authored), made into its first ledger entry on
    /// `opening_date`, with the `start` the opening rule settled on (`None` when it
    /// could not, an honest unknown). Does nothing for a player with no club or no
    /// contract, and nothing if the ledger already holds one.
    pub fn open_initial_contract(
        &mut self,
        opening_date: &str,
        start: Option<String>,
    ) -> Result<(), LedgerError> {
        self.adopt_staged_contract(ContractSource::Initial, opening_date.to_string(), start)
    }

    /// Turn the contract held in the flat fields into a ledger entry. The flat fields
    /// are where a contract waits before it has a history: world generation writes
    /// them, and so does a save from before the ledger.
    fn adopt_staged_contract(
        &mut self,
        source: ContractSource,
        date: String,
        start: Option<String>,
    ) -> Result<(), LedgerError> {
        if self
            .movement_history
            .iter()
            .any(|entry| entry.contract.is_some())
        {
            return Ok(());
        }
        if self.stored_contract_end.is_none() && self.stored_wage == 0 {
            return Ok(());
        }
        // A loaned player's contract is his parent's.
        let Some(club) = self
            .active_loan
            .as_ref()
            .map(|loan| loan.parent_team_id.clone())
            .or_else(|| self.team_id.clone())
        else {
            return Ok(());
        };
        let record = ContractRecord {
            start,
            end: self.stored_contract_end.clone(),
            weekly_wage: self.stored_wage,
            source,
        };
        // Appended last so it is the latest contract.
        self.record_movement(PlayerMovementEntry {
            to_team_id: Some(club),
            contract: Some(record),
            ..PlayerMovementEntry::new(date, PlayerMovementKind::InitialContract)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::{PlayerAttributes, Position};

    fn player() -> Player {
        Player::new(
            "p1".into(),
            "P One".into(),
            "Player One".into(),
            "1995-01-01".into(),
            "England".into(),
            Position::Forward,
            PlayerAttributes {
                pace: 60,
                stamina: 60,
                strength: 60,
                agility: 60,
                passing: 60,
                shooting: 60,
                tackling: 60,
                dribbling: 60,
                defending: 60,
                positioning: 60,
                vision: 60,
                decisions: 60,
                composure: 60,
                aggression: 60,
                teamwork: 60,
                leadership: 60,
                handling: 20,
                reflexes: 20,
                aerial: 50,
            },
        )
    }

    fn contract(
        start: Option<&str>,
        end: &str,
        wage: u32,
        source: ContractSource,
    ) -> ContractRecord {
        ContractRecord {
            start: start.map(str::to_string),
            end: Some(end.to_string()),
            weekly_wage: wage,
            source,
        }
    }

    fn entry(
        date: &str,
        kind: PlayerMovementKind,
        record: Option<ContractRecord>,
    ) -> PlayerMovementEntry {
        PlayerMovementEntry {
            contract: record,
            ..PlayerMovementEntry::new(date, kind)
        }
    }

    #[test]
    fn recording_a_contract_entry_sets_the_ledger_and_the_flat_fields_together() {
        let mut p = player();
        p.record_movement(entry(
            "2026-07-01",
            PlayerMovementKind::FreeAgentSigning,
            Some(contract(
                Some("2026-07-01"),
                "2029-06-30",
                4_000,
                ContractSource::FreeAgent,
            )),
        ))
        .expect("a valid contract is recorded");

        assert_eq!(p.movement_history.len(), 1);
        assert_eq!(p.stored_contract_start.as_deref(), Some("2026-07-01"));
        assert_eq!(p.stored_contract_end.as_deref(), Some("2029-06-30"));
        assert_eq!(p.stored_wage, 4_000);
    }

    #[test]
    fn the_accessors_return_the_latest_contract_entry() {
        let mut p = player();
        for (date, kind, record) in [
            (
                "2024-07-01",
                PlayerMovementKind::InitialContract,
                contract(
                    Some("2024-07-01"),
                    "2026-06-30",
                    1_000,
                    ContractSource::Initial,
                ),
            ),
            (
                "2026-01-10",
                PlayerMovementKind::Renewal,
                contract(
                    Some("2026-01-10"),
                    "2028-06-30",
                    2_000,
                    ContractSource::Renewal,
                ),
            ),
            (
                "2027-01-10",
                PlayerMovementKind::PermanentTransfer,
                contract(
                    Some("2027-01-10"),
                    "2030-06-30",
                    3_000,
                    ContractSource::Transfer,
                ),
            ),
        ] {
            p.record_movement(entry(date, kind, Some(record))).unwrap();
        }

        assert_eq!(p.contract_start(), Some("2027-01-10"));
        assert_eq!(p.contract_end(), Some("2030-06-30"));
        assert_eq!(p.wage(), 3_000);
        assert_eq!(p.movement_history.len(), 3, "earlier contracts are kept");
    }

    #[test]
    fn a_release_ends_the_contract() {
        let mut p = player();
        p.record_movement(entry(
            "2024-07-01",
            PlayerMovementKind::InitialContract,
            Some(contract(
                Some("2024-07-01"),
                "2026-06-30",
                1_000,
                ContractSource::Initial,
            )),
        ))
        .unwrap();
        p.record_movement(PlayerMovementEntry {
            release_reason: Some(ReleaseReason::Expired),
            ..PlayerMovementEntry::new("2026-06-30", PlayerMovementKind::Released)
        })
        .unwrap();

        assert_eq!(p.current_contract(), None);
        assert_eq!(p.contract_start(), None);
        assert_eq!(p.contract_end(), None);
        assert_eq!(p.wage(), 0);
        assert_eq!((p.stored_wage, p.stored_contract_end.as_deref()), (0, None));
    }

    #[test]
    fn a_retirement_ends_the_contract_and_the_wage() {
        let mut p = player();
        p.record_movement(entry(
            "2024-07-01",
            PlayerMovementKind::InitialContract,
            Some(contract(
                Some("2024-07-01"),
                "2027-06-30",
                9_000,
                ContractSource::Initial,
            )),
        ))
        .unwrap();
        p.record_movement(PlayerMovementEntry::new(
            "2026-06-30",
            PlayerMovementKind::Retired,
        ))
        .unwrap();

        assert_eq!(p.wage(), 0);
        assert_eq!(p.stored_wage, 0);
        assert_eq!(p.contract_end(), None);
    }

    #[test]
    fn a_loan_does_not_hide_the_parents_contract() {
        let mut p = player();
        p.record_movement(entry(
            "2024-07-01",
            PlayerMovementKind::InitialContract,
            Some(contract(
                Some("2024-07-01"),
                "2027-06-30",
                5_000,
                ContractSource::Initial,
            )),
        ))
        .unwrap();
        p.record_movement(PlayerMovementEntry::new(
            "2025-08-01",
            PlayerMovementKind::LoanStart,
        ))
        .unwrap();

        assert_eq!(p.wage(), 5_000);
        assert_eq!(p.contract_end(), Some("2027-06-30"));
        assert_eq!(
            p.stored_wage, 5_000,
            "a loan entry must not disturb the flat fields"
        );
    }

    #[test]
    fn same_day_events_keep_append_order() {
        let mut p = player();
        p.record_movement(entry(
            "2026-06-30",
            PlayerMovementKind::InitialContract,
            Some(contract(
                Some("2024-07-01"),
                "2026-06-30",
                1_000,
                ContractSource::Initial,
            )),
        ))
        .unwrap();
        p.record_movement(PlayerMovementEntry::new(
            "2026-06-30",
            PlayerMovementKind::Released,
        ))
        .unwrap();
        p.record_movement(entry(
            "2026-06-30",
            PlayerMovementKind::FreeAgentSigning,
            Some(contract(
                Some("2026-06-30"),
                "2029-06-30",
                2_500,
                ContractSource::FreeAgent,
            )),
        ))
        .unwrap();
        assert_eq!(
            p.wage(),
            2_500,
            "the signing was appended after the release"
        );

        p.record_movement(PlayerMovementEntry::new(
            "2026-06-30",
            PlayerMovementKind::Released,
        ))
        .unwrap();
        assert_eq!(p.wage(), 0, "a later release wins over a same-day signing");
    }

    #[test]
    fn a_contract_ending_on_or_before_its_start_is_refused_and_changes_nothing() {
        let mut p = player();
        for end in ["2026-07-01", "2026-06-30"] {
            let refused = p.record_movement(entry(
                "2026-07-01",
                PlayerMovementKind::Renewal,
                Some(contract(
                    Some("2026-07-01"),
                    end,
                    1_000,
                    ContractSource::Renewal,
                )),
            ));
            assert_eq!(refused, Err(LedgerError::EndNotAfterStart), "end {end}");
        }
        assert!(p.movement_history.is_empty());
        assert_eq!(p.stored_wage, 0);
    }

    #[test]
    fn an_unknown_start_is_recorded_as_unknown() {
        let mut p = player();
        p.record_movement(entry(
            "2026-07-01",
            PlayerMovementKind::InitialContract,
            Some(contract(None, "2028-06-30", 1_500, ContractSource::Initial)),
        ))
        .unwrap();

        assert_eq!(p.contract_start(), None);
        assert_eq!(p.stored_contract_start, None);
        assert_eq!(p.contract_end(), Some("2028-06-30"));
    }

    #[test]
    fn a_legacy_player_gets_one_entry_and_a_known_start_is_kept() {
        let mut p = player();
        p.team_id = Some("club-a".into());
        p.stored_contract_start = Some("2025-07-01".into());
        p.stored_contract_end = Some("2028-06-30".into());
        p.stored_wage = 7_000;

        p.adopt_legacy_contract();

        assert_eq!(p.movement_history.len(), 1);
        let only = &p.movement_history[0];
        assert_eq!(only.kind, PlayerMovementKind::InitialContract);
        assert_eq!(only.to_team_id.as_deref(), Some("club-a"));
        let record = only
            .contract
            .as_ref()
            .expect("the entry carries the contract");
        assert_eq!(record.source, ContractSource::LegacyMigrated);
        assert_eq!(record.start.as_deref(), Some("2025-07-01"));
        assert_eq!(record.end.as_deref(), Some("2028-06-30"));
        assert_eq!(record.weekly_wage, 7_000);
        assert_eq!((p.wage(), p.contract_end()), (7_000, Some("2028-06-30")));
    }

    #[test]
    fn a_legacy_start_that_was_never_known_stays_unknown() {
        let mut p = player();
        p.team_id = Some("club-a".into());
        p.stored_contract_end = Some("2028-06-30".into());
        p.stored_wage = 7_000;

        p.adopt_legacy_contract();

        assert_eq!(p.contract_start(), None, "no start is invented");
        assert_eq!(p.movement_history[0].contract.as_ref().unwrap().start, None);
    }

    #[test]
    fn a_legacy_contract_is_adopted_once() {
        let mut p = player();
        p.team_id = Some("club-a".into());
        p.stored_contract_end = Some("2028-06-30".into());
        p.stored_wage = 7_000;

        p.adopt_legacy_contract();
        p.adopt_legacy_contract();

        assert_eq!(p.movement_history.len(), 1);
    }

    #[test]
    fn a_player_with_no_contract_or_no_club_gets_no_legacy_entry() {
        let mut clubless = player();
        clubless.stored_contract_end = Some("2028-06-30".into());
        clubless.stored_wage = 7_000;
        clubless.adopt_legacy_contract();
        assert!(
            clubless.movement_history.is_empty(),
            "no club, no employer to record"
        );

        let mut uncontracted = player();
        uncontracted.team_id = Some("club-a".into());
        uncontracted.adopt_legacy_contract();
        assert!(uncontracted.movement_history.is_empty());
    }

    #[test]
    fn a_loaned_legacy_player_is_recorded_against_his_parent_club() {
        use crate::player::ActiveLoan;
        let mut p = player();
        p.team_id = Some("borrower".into());
        p.stored_contract_end = Some("2028-06-30".into());
        p.stored_wage = 7_000;
        p.active_loan = Some(ActiveLoan {
            parent_team_id: "parent".into(),
            loan_team_id: "borrower".into(),
            start_date: "2026-08-01".into(),
            end_date: "2027-05-31".into(),
            wage_contribution_pct: 50,
            buy_option_fee: None,
            loan_start_minutes: 0,
            loan_start_appearances: 0,
            development_reported_minutes: 0,
            development_reported_appearances: 0,
        });

        p.adopt_legacy_contract();

        assert_eq!(p.movement_history[0].to_team_id.as_deref(), Some("parent"));
    }

    #[test]
    fn an_entry_written_before_the_ledger_carried_contracts_still_reads() {
        let old = serde_json::json!({
            "date": "2025-08-01",
            "kind": "permanent_transfer",
            "from_team_id": "a",
            "to_team_id": "b",
            "fee": 1000
        });
        let parsed: PlayerMovementEntry = serde_json::from_value(old).unwrap();
        assert_eq!(parsed.contract, None);
        assert_eq!(parsed.release_reason, None);
    }

    /// The frontend and every saved game read the contract under its old names. The
    /// fields behind them are called `stored_*` in Rust; that must never reach the
    /// wire.
    #[test]
    fn the_wire_shape_of_a_players_contract_is_unchanged() {
        let mut p = player();
        p.record_movement(entry(
            "2026-07-01",
            PlayerMovementKind::FreeAgentSigning,
            Some(contract(
                Some("2026-07-01"),
                "2029-06-30",
                4_000,
                ContractSource::FreeAgent,
            )),
        ))
        .unwrap();

        let wire = serde_json::to_value(&p).unwrap();

        assert_eq!(wire["wage"], serde_json::json!(4_000));
        assert_eq!(wire["contract_start"], serde_json::json!("2026-07-01"));
        assert_eq!(wire["contract_end"], serde_json::json!("2029-06-30"));
        for key in [
            "stored_wage",
            "stored_contract_start",
            "stored_contract_end",
        ] {
            assert!(wire.get(key).is_none(), "{key} leaked onto the wire");
        }
        let back: Player = serde_json::from_value(wire).unwrap();
        assert_eq!(back.wage(), 4_000);
        assert_eq!(back.contract_end(), Some("2029-06-30"));
    }

    /// The contract a player carries is always the latest one in his history, however
    /// he got there. Every route is run here in turn and the two are compared after
    /// each step.
    #[test]
    fn the_contract_is_always_the_latest_ledger_entry_after_every_route() {
        let mut p = player();
        let steps: Vec<PlayerMovementEntry> = vec![
            entry(
                "2024-07-01",
                PlayerMovementKind::InitialContract,
                Some(contract(
                    Some("2024-07-01"),
                    "2026-06-30",
                    1_000,
                    ContractSource::Initial,
                )),
            ),
            PlayerMovementEntry::new("2025-08-01", PlayerMovementKind::LoanStart),
            PlayerMovementEntry::new("2026-01-01", PlayerMovementKind::LoanReturn),
            entry(
                "2026-01-10",
                PlayerMovementKind::Renewal,
                Some(contract(
                    Some("2026-01-10"),
                    "2028-06-30",
                    2_000,
                    ContractSource::Renewal,
                )),
            ),
            entry(
                "2027-01-10",
                PlayerMovementKind::PermanentTransfer,
                Some(contract(
                    Some("2027-01-10"),
                    "2030-06-30",
                    3_000,
                    ContractSource::Transfer,
                )),
            ),
            PlayerMovementEntry::new("2028-06-30", PlayerMovementKind::Released),
            entry(
                "2028-07-15",
                PlayerMovementKind::FreeAgentSigning,
                Some(contract(
                    Some("2028-07-15"),
                    "2030-06-30",
                    1_500,
                    ContractSource::FreeAgent,
                )),
            ),
            PlayerMovementEntry::new("2030-06-30", PlayerMovementKind::Retired),
        ];

        for step in steps {
            let kind = step.kind.clone();
            p.record_movement(step).unwrap();
            let latest = p.current_contract().cloned();
            assert_eq!(
                (p.wage(), p.contract_start(), p.contract_end()),
                (
                    latest.as_ref().map_or(0, |c| c.weekly_wage),
                    latest.as_ref().and_then(|c| c.start.as_deref()),
                    latest.as_ref().and_then(|c| c.end.as_deref()),
                ),
                "after {kind:?} the contract drifted from the ledger"
            );
        }
        assert_eq!(p.wage(), 0, "he retired");
    }

    #[test]
    fn the_first_event_on_an_old_save_starts_a_history_after_the_legacy_entry() {
        let mut p = player();
        p.team_id = Some("club-a".into());
        p.stored_contract_end = Some("2027-06-30".into());
        p.stored_wage = 5_000;
        p.adopt_legacy_contract();

        p.record_movement(entry(
            "2026-09-01",
            PlayerMovementKind::Renewal,
            Some(contract(
                Some("2026-09-01"),
                "2030-06-30",
                8_000,
                ContractSource::Renewal,
            )),
        ))
        .unwrap();

        assert_eq!(p.movement_history.len(), 2, "the legacy entry is kept");
        assert_eq!(
            p.movement_history[0].contract.as_ref().unwrap().source,
            ContractSource::LegacyMigrated
        );
        assert_eq!(p.wage(), 8_000);
        assert_eq!(p.contract_end(), Some("2030-06-30"));
    }
}
