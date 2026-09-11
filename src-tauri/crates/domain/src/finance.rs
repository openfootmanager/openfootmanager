use serde::{Deserialize, Serialize};
use std::ops::Deref;
use std::sync::Arc;

use crate::team::FinancialTransactionKind;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CashPost {
    pub id: String,
    pub club_id: String,
    /// Positive is income, negative is expense. Integer euros.
    pub amount: i64,
    pub kind: CashKind,
    /// Game-clock date (`YYYY-MM-DD`), never wall-clock.
    pub date: String,
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

impl Deref for CashJournal {
    type Target = [CashPost];

    fn deref(&self) -> &[CashPost] {
        &self.posts
    }
}

impl CashJournal {
    pub fn from_vec(posts: Vec<CashPost>) -> Self {
        Self {
            posts: Arc::new(posts),
        }
    }

    pub fn shares_storage_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.posts, &other.posts)
    }

    pub fn contains_club(&self, club_id: &str) -> bool {
        self.iter().any(|post| post.club_id == club_id)
    }

    pub fn extend(&mut self, posts: impl IntoIterator<Item = CashPost>) {
        Arc::make_mut(&mut self.posts).extend(posts);
    }

    /// Overflow is a bug: `post_all` rejected it.
    pub fn cash_for(&self, club_id: &str) -> i64 {
        self.iter()
            .filter(|post| post.club_id == club_id)
            .try_fold(0i64, |acc, post| acc.checked_add(post.amount))
            .expect("journal overflow is a bug: post_all rejected it")
    }
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
