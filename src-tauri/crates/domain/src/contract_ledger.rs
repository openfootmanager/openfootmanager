//! A player's contract history, kept in the movement ledger.
//!
//! `Player::movement_history` is the one record of a player's employment. An entry
//! that *establishes* a contract carries a [`ContractRecord`]; the player's current
//! contract is the latest such entry, unless a later release or retirement ended it.
//! Loans are movements that do not touch the contract: a loaned player is still on
//! his parent club's agreement.
//!
//! A player has no wage or contract-date fields. `wage()`, `contract_start()` and
//! `contract_end()` read the ledger, and the ledger writes `wage`, `contract_start`
//! and `contract_end` when it is serialized, so the wire and the saves keep the shape
//! they always had. Nothing but [`Player::record_movement`] adds to the ledger.
//! "Latest" is append order, never date order: a release and a new signing can share
//! a day.
//!
//! One thing sits beside the entries: a contract that is in force but not recorded
//! yet. World generation has no history to write, so it *stages* one; a save from
//! before the ledger has one in its old columns. Either is turned into the player's
//! first entry (`open_initial_contract`, `adopt_legacy_contract`), and the first
//! recorded contract replaces it.

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

/// A contract that is in force but has no entry yet.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct StagedContract {
    start: Option<String>,
    end: Option<String>,
    wage: u32,
}

/// What a saved or serialized player says about his contract and moves, under the keys
/// it has always used.
#[derive(Deserialize)]
struct LedgerKeys {
    #[serde(default)]
    movement_history: Vec<PlayerMovementEntry>,
    #[serde(default)]
    wage: u32,
    #[serde(default)]
    contract_start: Option<String>,
    #[serde(default)]
    contract_end: Option<String>,
}

/// A player's moves and contracts. Reads like the list of entries it holds; only
/// [`Player::record_movement`] can add to it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MovementLedger {
    entries: Vec<PlayerMovementEntry>,
    staged: Option<StagedContract>,
}

impl MovementLedger {
    /// Rebuild a ledger from a save: its entries, plus the contract columns the save
    /// has always had. The columns matter only when the entries hold no contract
    /// (a save from before the ledger): then they describe a contract that still has
    /// to be recorded. When the entries hold one, the entries are the truth.
    pub fn restore(
        entries: Vec<PlayerMovementEntry>,
        start: Option<String>,
        end: Option<String>,
        wage: u32,
    ) -> Self {
        let holds_a_contract = entries.iter().any(|entry| entry.contract.is_some());
        let staged = (!holds_a_contract && (end.is_some() || wage > 0)).then_some(StagedContract {
            start,
            end,
            wage,
        });
        Self { entries, staged }
    }

    /// The entry that makes the contract in force, if the ledger holds one: the
    /// latest with a contract block, unless a release or retirement came after it.
    fn recorded(&self) -> Option<&ContractRecord> {
        for entry in self.entries.iter().rev() {
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

    fn start(&self) -> Option<&str> {
        match &self.staged {
            Some(staged) => staged.start.as_deref(),
            None => self.recorded().and_then(|record| record.start.as_deref()),
        }
    }

    fn end(&self) -> Option<&str> {
        match &self.staged {
            Some(staged) => staged.end.as_deref(),
            None => self.recorded().and_then(|record| record.end.as_deref()),
        }
    }

    fn wage(&self) -> u32 {
        match &self.staged {
            Some(staged) => staged.wage,
            None => self.recorded().map_or(0, |record| record.weekly_wage),
        }
    }

    /// Change the staged contract, starting from whatever is in force.
    fn stage(&mut self, change: impl FnOnce(&mut StagedContract)) {
        let mut staged = self.staged.take().unwrap_or_else(|| StagedContract {
            start: self.start().map(str::to_string),
            end: self.end().map(str::to_string),
            wage: self.wage(),
        });
        change(&mut staged);
        self.staged = Some(staged);
    }
}

impl std::ops::Deref for MovementLedger {
    type Target = [PlayerMovementEntry];

    fn deref(&self) -> &Self::Target {
        &self.entries
    }
}

impl<'a> IntoIterator for &'a MovementLedger {
    type Item = &'a PlayerMovementEntry;
    type IntoIter = std::slice::Iter<'a, PlayerMovementEntry>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter()
    }
}

impl Serialize for MovementLedger {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(4))?;
        map.serialize_entry("movement_history", &self.entries)?;
        map.serialize_entry("wage", &self.wage())?;
        map.serialize_entry("contract_start", &self.start())?;
        map.serialize_entry("contract_end", &self.end())?;
        map.end()
    }
}

impl<'de> Deserialize<'de> for MovementLedger {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let keys = LedgerKeys::deserialize(deserializer)?;
        Ok(Self::restore(
            keys.movement_history,
            keys.contract_start,
            keys.contract_end,
            keys.wage,
        ))
    }
}

impl Player {
    /// The one way an entry joins the ledger. A contract it makes replaces any staged
    /// one, and a release or retirement ends it.
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
        self.movement_history.entries.push(entry);
        // Only an entry that can change the contract may replace a staged one: a loan
        // passing through must leave it as it is.
        if changes_contract {
            self.movement_history.staged = None;
        }
        Ok(())
    }

    /// The contract the ledger records as in force: the latest entry that carries
    /// one, unless a release or retirement came after it. A contract that is only
    /// staged is not in the ledger yet, so it is not here; `wage()` and the two dates
    /// do include it.
    pub fn current_contract(&self) -> Option<&ContractRecord> {
        self.movement_history.recorded()
    }

    /// The club whose contract he is on: his parent club while he is out on loan, his
    /// own club otherwise. `team_id` is where he plays, which on loan is the borrower.
    /// The one place this rule is written.
    pub fn contract_club_id(&self) -> Option<&str> {
        self.active_loan
            .as_ref()
            .map(|loan| loan.parent_team_id.as_str())
            .or(self.team_id.as_deref())
    }

    pub fn contract_start(&self) -> Option<&str> {
        self.movement_history.start()
    }

    pub fn contract_end(&self) -> Option<&str> {
        self.movement_history.end()
    }

    /// Weekly wage.
    pub fn wage(&self) -> u32 {
        self.movement_history.wage()
    }

    /// Stage the contract world generation gave a player, before he has a history.
    /// Opening a career turns it into his first ledger entry
    /// (`open_initial_contract`). Nothing else should call this once a player has a
    /// ledger; record a movement instead.
    pub fn stage_contract(&mut self, start: Option<String>, end: Option<String>, wage: u32) {
        self.movement_history
            .stage(|staged| *staged = StagedContract { start, end, wage });
    }

    pub fn stage_wage(&mut self, wage: u32) {
        self.movement_history.stage(|staged| staged.wage = wage);
    }

    pub fn stage_contract_start(&mut self, start: Option<String>) {
        self.movement_history.stage(|staged| staged.start = start);
    }

    pub fn stage_contract_end(&mut self, end: Option<String>) {
        self.movement_history.stage(|staged| staged.end = end);
    }

    /// Give a player loaded from a save that predates the ledger one entry for the
    /// contract its old columns describe. Does nothing when there is no such
    /// contract, when he has no club, or when the ledger already holds a contract, so
    /// it is safe to call on every load.
    pub fn adopt_legacy_contract(&mut self) {
        // Dated by the start the old columns knew, and empty when they did not: an
        // unknown, not a made-up day.
        let date = self
            .movement_history
            .staged
            .as_ref()
            .and_then(|staged| staged.start.clone())
            .unwrap_or_default();
        let _ = self.adopt_staged_contract(ContractSource::LegacyMigrated, date, None);
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
        self.adopt_staged_contract(
            ContractSource::Initial,
            opening_date.to_string(),
            Some(start),
        )
    }

    /// Turn the staged contract into a ledger entry. `start_override` replaces the
    /// staged start when given (opening settles it); `None` keeps the staged one.
    fn adopt_staged_contract(
        &mut self,
        source: ContractSource,
        date: String,
        start_override: Option<Option<String>>,
    ) -> Result<(), LedgerError> {
        let Some(staged) = self.movement_history.staged.clone() else {
            return Ok(());
        };
        if self
            .movement_history
            .entries
            .iter()
            .any(|entry| entry.contract.is_some())
        {
            self.movement_history.staged = None;
            return Ok(());
        }
        if staged.end.is_none() && staged.wage == 0 {
            self.movement_history.staged = None;
            return Ok(());
        }
        let Some(club) = self.contract_club_id().map(str::to_string) else {
            return Ok(());
        };
        let record = ContractRecord {
            start: start_override.unwrap_or(staged.start),
            end: staged.end,
            weekly_wage: staged.wage,
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
        assert_eq!(p.contract_start(), Some("2026-07-01"));
        assert_eq!(p.contract_end(), Some("2029-06-30"));
        assert_eq!(p.wage(), 4_000);
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
        assert_eq!((p.wage(), p.contract_end()), (0, None));
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
        assert_eq!(p.wage(), 0);
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
            p.wage(),
            5_000,
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
        assert_eq!(p.wage(), 0);
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
        assert_eq!(p.contract_end(), Some("2028-06-30"));
    }

    #[test]
    fn a_legacy_player_gets_one_entry_and_a_known_start_is_kept() {
        let mut p = player();
        p.team_id = Some("club-a".into());
        p.stage_contract_start(Some("2025-07-01".into()));
        p.stage_contract_end(Some("2028-06-30".into()));
        p.stage_wage(7_000);

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
        p.stage_contract_end(Some("2028-06-30".into()));
        p.stage_wage(7_000);

        p.adopt_legacy_contract();

        assert_eq!(p.contract_start(), None, "no start is invented");
        assert_eq!(p.movement_history[0].contract.as_ref().unwrap().start, None);
    }

    #[test]
    fn a_legacy_contract_is_adopted_once() {
        let mut p = player();
        p.team_id = Some("club-a".into());
        p.stage_contract_end(Some("2028-06-30".into()));
        p.stage_wage(7_000);

        p.adopt_legacy_contract();
        p.adopt_legacy_contract();

        assert_eq!(p.movement_history.len(), 1);
    }

    #[test]
    fn a_player_with_no_contract_or_no_club_gets_no_legacy_entry() {
        let mut clubless = player();
        clubless.stage_contract_end(Some("2028-06-30".into()));
        clubless.stage_wage(7_000);
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
        p.stage_contract_end(Some("2028-06-30".into()));
        p.stage_wage(7_000);
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
    /// fields they used to live in no longer exist; the ledger writes them itself, and that must never change the
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
        p.stage_contract_end(Some("2027-06-30".into()));
        p.stage_wage(5_000);
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

    #[test]
    fn a_staged_contract_is_what_the_accessors_and_the_wire_report() {
        let mut p = player();
        p.stage_contract(Some("2026-07-01".into()), Some("2029-06-30".into()), 4_500);

        assert_eq!(p.wage(), 4_500);
        assert_eq!(p.contract_start(), Some("2026-07-01"));
        assert_eq!(p.contract_end(), Some("2029-06-30"));
        assert!(p.current_contract().is_none(), "nothing is recorded yet");
        let wire = serde_json::to_value(&p).unwrap();
        assert_eq!(wire["wage"], serde_json::json!(4_500));
        assert_eq!(wire["contract_end"], serde_json::json!("2029-06-30"));
    }

    #[test]
    fn staging_one_part_keeps_the_rest() {
        let mut p = player();
        p.stage_contract(Some("2026-07-01".into()), Some("2029-06-30".into()), 4_500);

        p.stage_contract_end(Some("2030-06-30".into()));

        assert_eq!(p.contract_end(), Some("2030-06-30"));
        assert_eq!(p.contract_start(), Some("2026-07-01"));
        assert_eq!(p.wage(), 4_500);
    }

    #[test]
    fn recording_a_contract_replaces_a_staged_one() {
        let mut p = player();
        p.stage_contract(None, Some("2026-06-30".into()), 1_000);

        p.record_movement(entry(
            "2026-07-01",
            PlayerMovementKind::FreeAgentSigning,
            Some(contract(
                Some("2026-07-01"),
                "2029-06-30",
                6_000,
                ContractSource::FreeAgent,
            )),
        ))
        .unwrap();

        assert_eq!(p.wage(), 6_000);
        assert_eq!(p.contract_end(), Some("2029-06-30"));
    }

    #[test]
    fn a_release_clears_a_staged_contract() {
        let mut p = player();
        p.stage_contract(None, Some("2026-06-30".into()), 1_000);

        p.record_movement(PlayerMovementEntry::new(
            "2026-06-30",
            PlayerMovementKind::Released,
        ))
        .unwrap();

        assert_eq!(p.wage(), 0);
        assert_eq!(p.contract_end(), None);
    }

    #[test]
    fn a_loan_leaves_a_staged_contract_alone() {
        let mut p = player();
        p.stage_contract(None, Some("2027-06-30".into()), 2_000);

        p.record_movement(PlayerMovementEntry::new(
            "2026-08-01",
            PlayerMovementKind::LoanStart,
        ))
        .unwrap();

        assert_eq!(p.wage(), 2_000);
    }

    #[test]
    fn the_old_columns_are_ignored_once_the_entries_hold_a_contract() {
        let entries = vec![entry(
            "2026-01-10",
            PlayerMovementKind::Renewal,
            Some(contract(
                Some("2026-01-10"),
                "2029-06-30",
                9_000,
                ContractSource::Renewal,
            )),
        )];

        let ledger = MovementLedger::restore(
            entries,
            Some("2019-07-01".into()),
            Some("2020-06-30".into()),
            1_000,
        );

        let mut p = player();
        p.movement_history = ledger;
        assert_eq!(
            p.wage(),
            9_000,
            "the entries are the truth, not the stale columns"
        );
        assert_eq!(p.contract_end(), Some("2029-06-30"));
    }

    #[test]
    fn a_players_old_json_keys_become_a_contract_to_adopt() {
        let mut wire = serde_json::to_value(player()).unwrap();
        wire["team_id"] = serde_json::json!("club-a");
        wire["wage"] = serde_json::json!(7_000);
        wire["contract_end"] = serde_json::json!("2028-06-30");
        wire["contract_start"] = serde_json::Value::Null;
        wire.as_object_mut().unwrap().remove("movement_history");

        let mut p: Player = serde_json::from_value(wire).unwrap();

        assert_eq!(p.wage(), 7_000, "visible straight away");
        p.adopt_legacy_contract();
        assert_eq!(p.movement_history.len(), 1);
        assert_eq!(
            p.movement_history[0].contract.as_ref().unwrap().source,
            ContractSource::LegacyMigrated
        );
        assert_eq!(p.contract_start(), None);
    }

    #[test]
    fn the_contract_club_is_his_club_unless_he_is_on_loan() {
        use crate::player::ActiveLoan;
        let mut p = player();
        assert_eq!(p.contract_club_id(), None, "no club, no contract club");

        p.team_id = Some("club-a".into());
        assert_eq!(p.contract_club_id(), Some("club-a"));

        p.team_id = Some("borrower".into());
        p.active_loan = Some(ActiveLoan {
            parent_team_id: "club-a".into(),
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
        assert_eq!(
            p.contract_club_id(),
            Some("club-a"),
            "the contract stays with the parent while he is out on loan"
        );
    }
}
