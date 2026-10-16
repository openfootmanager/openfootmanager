//! Whether a club can pay a transfer fee: one rule, read by every place that asks.
//!
//! A fee has to fit both the club's cash and its transfer-budget envelope. The same question was
//! answered four times, with a slightly different shape each time (two errors, a bool, a cap);
//! they all derive from [`fee_block`] now.

use domain::team::Team;

use super::consts::{ERR_INSUFFICIENT_FUNDS, ERR_TRANSFER_BUDGET_TOO_LOW};

/// Which limit stops a fee. Cash is checked first, so a club short of both reports the cash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FeeBlock {
    InsufficientFunds,
    TransferBudgetTooLow,
}

impl FeeBlock {
    /// The translation key the player reads.
    pub(crate) fn error_key(self) -> &'static str {
        match self {
            Self::InsufficientFunds => ERR_INSUFFICIENT_FUNDS,
            Self::TransferBudgetTooLow => ERR_TRANSFER_BUDGET_TOO_LOW,
        }
    }
}

/// `Ok` when `team` can pay `fee` out of both its cash and its transfer budget. No clamping: a
/// club with a negative budget cannot pay even a zero fee, as registration has always decided.
pub(crate) fn fee_block(team: &Team, fee: u64) -> Result<(), FeeBlock> {
    // A fee beyond i64 is beyond any club's cash.
    let Ok(fee) = i64::try_from(fee) else {
        return Err(FeeBlock::InsufficientFunds);
    };
    if team.finance < fee {
        return Err(FeeBlock::InsufficientFunds);
    }
    if team.transfer_budget < fee {
        return Err(FeeBlock::TransferBudgetTooLow);
    }
    Ok(())
}

/// Whether `team` can pay `fee` out of cash and transfer budget: [`fee_block`] finds nothing in the way.
pub(crate) fn can_afford_fee(team: &Team, fee: u64) -> bool {
    fee_block(team, fee).is_ok()
}

/// The largest fee `team` could pay, for capping what a buyer will offer. Negative cash or budget
/// count as nothing to spend, so it agrees with [`fee_block`] for every fee above zero.
pub(crate) fn transfer_fee_ceiling(team: &Team) -> u64 {
    (team.transfer_budget.max(0) as u64).min(team.finance.max(0) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn club(finance: i64, transfer_budget: i64) -> Team {
        let mut team = Team::new(
            "club".to_string(),
            "Club".to_string(),
            "CLB".to_string(),
            "England".to_string(),
            "Town".to_string(),
            "Ground".to_string(),
            10_000,
        );
        team.finance = finance;
        team.transfer_budget = transfer_budget;
        team
    }

    /// Given cash and budget that both cover a fee, when it is checked, then nothing blocks it.
    #[test]
    fn a_fee_within_cash_and_budget_is_not_blocked() {
        assert_eq!(fee_block(&club(1_000, 800), 800), Ok(()));
    }

    /// Given less cash than the fee, when it is checked, then cash is what blocks it.
    #[test]
    fn a_fee_beyond_cash_is_blocked_by_cash() {
        assert_eq!(
            fee_block(&club(500, 10_000), 800),
            Err(FeeBlock::InsufficientFunds)
        );
    }

    /// Given cash for the fee but too little budget, when it is checked, then the budget blocks it.
    #[test]
    fn a_fee_beyond_the_budget_is_blocked_by_the_budget() {
        assert_eq!(
            fee_block(&club(10_000, 500), 800),
            Err(FeeBlock::TransferBudgetTooLow)
        );
    }

    /// Given a club short of both, when a fee is checked, then cash is reported first.
    #[test]
    fn a_club_short_of_both_reports_cash() {
        assert_eq!(
            fee_block(&club(100, 100), 800),
            Err(FeeBlock::InsufficientFunds)
        );
    }

    /// Given a fee too large for any club, when it is checked, then it reads as short of cash
    /// rather than wrapping.
    #[test]
    fn a_fee_beyond_i64_is_insufficient_funds() {
        assert_eq!(
            fee_block(&club(i64::MAX, i64::MAX), u64::MAX),
            Err(FeeBlock::InsufficientFunds)
        );
    }

    /// Given a negative transfer budget, when a zero fee is checked, then it is still blocked: the
    /// registration rule never clamped negatives.
    #[test]
    fn a_negative_budget_blocks_even_a_zero_fee() {
        assert_eq!(
            fee_block(&club(1_000, -1), 0),
            Err(FeeBlock::TransferBudgetTooLow)
        );
        assert!(!can_afford_fee(&club(1_000, -1), 0));
    }

    /// Given each block, when its error key is read, then it is the key the bid screen has always
    /// shown.
    #[test]
    fn each_block_reports_its_translation_key() {
        assert_eq!(
            FeeBlock::InsufficientFunds.error_key(),
            "be.error.transfers.insufficientFunds"
        );
        assert_eq!(
            FeeBlock::TransferBudgetTooLow.error_key(),
            "be.error.transfers.transferBudgetTooLow"
        );
    }

    /// Given a grid of cash, budget (including negatives) and fees, when the ceiling and the block
    /// are compared, then for every fee above zero `fee <= ceiling` exactly when nothing blocks it.
    #[test]
    fn the_ceiling_agrees_with_the_block_for_every_positive_fee() {
        let amounts = [-5_000, -1, 0, 1, 799, 800, 801, 5_000, 1_000_000];
        for finance in amounts {
            for budget in amounts {
                let team = club(finance, budget);
                let ceiling = transfer_fee_ceiling(&team);
                for fee in [1u64, 2, 799, 800, 801, 5_000, 999_999, 1_000_000, 2_000_000] {
                    assert_eq!(
                        fee <= ceiling,
                        fee_block(&team, fee).is_ok(),
                        "finance {finance}, budget {budget}, fee {fee}"
                    );
                }
            }
        }
    }
}
