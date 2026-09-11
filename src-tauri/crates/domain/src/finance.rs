use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::team::FinancialTransactionKind;

/// A single cash movement. The journal is append-only; v1 never deletes rows.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CashPost {
    pub id: String,
    pub club_id: String,
    /// Signed integer euros: positive is income, negative is expense.
    pub amount: i64,
    pub kind: CashKind,
    /// Game-clock date (`YYYY-MM-DD`), never wall-clock.
    pub date: String,
    /// Copied from `Team.envelope_generation` at post time.
    #[serde(default)]
    pub envelope_generation: u32,
    #[serde(default)]
    pub meta: CashPostMeta,
    /// Reserved for a later compensating-post API. v1 never sets this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reverses_id: Option<String>,
}

/// Translation-key metadata for a cash post. Never English prose.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct CashPostMeta {
    pub counterparty_club_id: Option<String>,
    pub player_id: Option<String>,
    pub staff_id: Option<String>,
    pub fixture_id: Option<String>,
    pub offer_id: Option<String>,
    pub reservation_id: Option<String>,
    pub description_key: Option<String>,
    pub description_params: HashMap<String, String>,
}

/// Chart of accounts. Additive — never `#[serde(other)]`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CashKind {
    OpeningBalance,
    PlayerWages,
    StaffWages,
    Matchday,
    Sponsorship,
    CommercialCampaign,
    PrizeMoney,
    TransferFeeOut,
    TransferFeeIn,
    LoanFee,
    ContractTermination,
    BoardSupport,
    Facilities,
    Upkeep,
    Other,
}

impl CashKind {
    pub fn from_legacy_ledger(kind: FinancialTransactionKind) -> Self {
        match kind {
            FinancialTransactionKind::PrizeMoney => Self::PrizeMoney,
            FinancialTransactionKind::ContractTermination => Self::ContractTermination,
            FinancialTransactionKind::BoardSupport => Self::BoardSupport,
            FinancialTransactionKind::CommercialCampaign => Self::CommercialCampaign,
        }
    }
}

/// Append-only cash journal. `Clone` is a pointer bump; writers copy-on-write.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CashJournal {
    posts: Arc<Vec<CashPost>>,
}

impl CashJournal {
    pub fn from_vec(posts: Vec<CashPost>) -> Self {
        Self {
            posts: Arc::new(posts),
        }
    }

    pub fn as_slice(&self) -> &[CashPost] {
        &self.posts
    }

    pub fn iter(&self) -> std::slice::Iter<'_, CashPost> {
        self.posts.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.posts.is_empty()
    }

    pub fn len(&self) -> usize {
        self.posts.len()
    }

    pub fn shares_storage_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.posts, &other.posts)
    }

    pub fn contains_club(&self, club_id: &str) -> bool {
        self.posts.iter().any(|post| post.club_id == club_id)
    }

    pub fn extend(&mut self, posts: impl IntoIterator<Item = CashPost>) {
        Arc::make_mut(&mut self.posts).extend(posts);
    }

    /// Sum of posts for one club. Overflow is a bug: `post_all` rejected it.
    pub fn cash_for(&self, club_id: &str) -> i64 {
        self.posts
            .iter()
            .filter(|post| post.club_id == club_id)
            .try_fold(0i64, |acc, post| acc.checked_add(post.amount))
            .expect("journal overflow is a bug: post_all rejected it")
    }
}

/// Transfer reservations with the same cheap-clone contract as [`CashJournal`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TransferReservationBook {
    rows: Arc<Vec<TransferReservation>>,
}

impl TransferReservationBook {
    pub fn from_vec(rows: Vec<TransferReservation>) -> Self {
        Self {
            rows: Arc::new(rows),
        }
    }

    pub fn as_slice(&self) -> &[TransferReservation] {
        &self.rows
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn shares_storage_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.rows, &other.rows)
    }

    pub fn extend(&mut self, rows: impl IntoIterator<Item = TransferReservation>) {
        Arc::make_mut(&mut self.rows).extend(rows);
    }
}

/// A transfer-fee (or fee-less loan registration) reservation.
///
/// Agreement creates these; settlement posts cash. Status changes in place.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransferReservation {
    pub id: String,
    pub club_id: String,
    pub player_id: String,
    pub offer_id: String,
    pub amount: i64,
    pub created_on: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settles_on: Option<String>,
    pub kind: ReservationKind,
    pub status: ReservationStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReservationKind {
    TransferFee,
    LoanRegistration,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReservationStatus {
    Active,
    Settled,
    Released,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cash_kind_serializes_as_snake_case() {
        let json = serde_json::to_string(&CashKind::TransferFeeOut).unwrap();
        assert_eq!(json, "\"transfer_fee_out\"");
        let back: CashKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, CashKind::TransferFeeOut);
    }

    #[test]
    fn unknown_kind_fails_to_deserialize() {
        let err = serde_json::from_str::<CashKind>("\"not_a_kind\"").unwrap_err();
        assert!(err.to_string().contains("unknown variant"));
    }
}
