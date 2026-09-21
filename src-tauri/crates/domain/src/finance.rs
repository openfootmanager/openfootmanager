use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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
    pub fn counts_toward_season_totals(self) -> bool {
        !matches!(
            self,
            Self::OpeningBalance | Self::TransferFeeIn | Self::TransferFeeOut | Self::LoanFee
        )
    }

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
    clubs: Arc<HashSet<String>>,
}

impl Deref for CashJournal {
    type Target = [CashPost];

    fn deref(&self) -> &[CashPost] {
        &self.posts
    }
}

impl CashJournal {
    pub fn from_vec(posts: Vec<CashPost>) -> Self {
        let clubs = posts.iter().map(|post| post.club_id.clone()).collect();
        Self {
            posts: Arc::new(posts),
            clubs: Arc::new(clubs),
        }
    }

    pub fn shares_storage_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.posts, &other.posts)
    }

    pub fn contains_club(&self, club_id: &str) -> bool {
        self.clubs.contains(club_id)
    }

    pub fn extend(&mut self, posts: impl IntoIterator<Item = CashPost>) {
        let extra: Vec<CashPost> = posts.into_iter().collect();
        if extra.is_empty() {
            return;
        }
        let clubs = Arc::make_mut(&mut self.clubs);
        for post in &extra {
            clubs.insert(post.club_id.clone());
        }
        Arc::make_mut(&mut self.posts).extend(extra);
    }

    /// Sum of posts for one club. Folded in `i128` so prefix order cannot panic.
    pub fn cash_for(&self, club_id: &str) -> i64 {
        let sum = self
            .iter()
            .filter(|post| post.club_id == club_id)
            .fold(0i128, |acc, post| acc + i128::from(post.amount));
        i64::try_from(sum).expect("journal overflow is a bug: post_all rejected it")
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

    fn post(id: &str, amount: i64) -> CashPost {
        CashPost {
            id: id.to_string(),
            club_id: "alpha".to_string(),
            amount,
            kind: CashKind::Other,
            date: "2026-02-16".to_string(),
        }
    }

    #[test]
    fn cash_for_does_not_depend_on_prefix_order() {
        let credits_first =
            CashJournal::from_vec(vec![post("1", i64::MAX), post("2", 1), post("3", -1)]);
        let debit_first =
            CashJournal::from_vec(vec![post("1", i64::MAX), post("3", -1), post("2", 1)]);
        assert_eq!(credits_first.cash_for("alpha"), i64::MAX);
        assert_eq!(debit_first.cash_for("alpha"), i64::MAX);
    }
}
