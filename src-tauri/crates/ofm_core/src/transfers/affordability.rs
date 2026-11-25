//! Whether a club can pay a transfer fee: one rule, read by every place that asks.
//!
//! A fee has to fit both the club's cash and its transfer-budget envelope. The envelope scalar
//! is the season *limit*; settled spend this season is derived from the cash journal, and
//! reserved fees from pending-registration offers. The same question was answered four times,
//! with a slightly different shape each time (two errors, a bool, a cap); they all derive from
//! [`fee_block`] now.

use crate::game::Game;
use domain::finance::{CashKind, CashPost};
use domain::player::TransferOfferStatus;
use domain::team::Team;

use super::consts::{ERR_INSUFFICIENT_FUNDS, ERR_TRANSFER_BUDGET_TOO_LOW};

/// Which limit stops a fee. Cash is checked first, so a club short of both reports the cash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FeeBlock {
    InsufficientFunds,
    TransferBudgetTooLow,
    TransferBudgetCommitted,
}

impl FeeBlock {
    /// The translation key the player reads.
    pub(crate) fn error_key(self) -> &'static str {
        match self {
            Self::InsufficientFunds => ERR_INSUFFICIENT_FUNDS,
            Self::TransferBudgetTooLow => ERR_TRANSFER_BUDGET_TOO_LOW,
            Self::TransferBudgetCommitted => "be.error.transfers.transferBudgetCommitted",
        }
    }
}

/// `Ok` when `team` can pay `fee` out of both its cash and the unspent, unreserved envelope.
/// `transfer_budget` is the season limit. `spent` is net settled transfer-fee spend this season
/// (purchases positive, sales negative). `committed` is reserved pending-registration fees.
/// No clamping: a club with a negative unspent envelope cannot pay even a zero fee, as
/// registration has always decided.
pub(crate) fn fee_block(team: &Team, fee: u64, committed: i64, spent: i64) -> Result<(), FeeBlock> {
    // A fee beyond i64 is beyond any club's cash.
    let Ok(fee) = i64::try_from(fee) else {
        return Err(FeeBlock::InsufficientFunds);
    };
    if team.finance < fee {
        return Err(FeeBlock::InsufficientFunds);
    }
    let unspent = team.transfer_budget.saturating_sub(spent);
    if unspent < fee {
        return Err(FeeBlock::TransferBudgetTooLow);
    }
    if unspent.saturating_sub(committed) < fee {
        return Err(FeeBlock::TransferBudgetCommitted);
    }
    Ok(())
}

/// Whether `team` can pay `fee` out of cash and transfer budget: [`fee_block`] finds nothing in the way.
pub(crate) fn can_afford_fee(team: &Team, fee: u64, committed: i64, spent: i64) -> bool {
    fee_block(team, fee, committed, spent).is_ok()
}

/// The largest fee `team` could pay, for capping what a buyer will offer. Negative cash or
/// unspent envelope count as nothing to spend, so it agrees with [`fee_block`] for every fee
/// above zero.
pub(crate) fn transfer_fee_ceiling(team: &Team, committed: i64, spent: i64) -> u64 {
    (team
        .transfer_budget
        .saturating_sub(spent)
        .saturating_sub(committed)
        .max(0) as u64)
        .min(team.finance.max(0) as u64)
}

/// Net transfer-fee spend this season for `team_id`: purchases raise it, sales lower it.
/// Posts on or before [`Game::last_season_rollover_on`] belong to the previous season,
/// unless [`Game::last_season_rollover_journal_len`] is set: then posts at that index
/// and after belong to the new season, including settlements on the rollover day.
pub fn settled_transfer_spend(game: &Game, team_id: &str) -> i64 {
    settled_transfer_spend_in(
        game.cash_journal.iter(),
        team_id,
        game.last_season_rollover_on.as_deref(),
        game.last_season_rollover_journal_len
            .map(|len| len as usize),
    )
}

fn settled_transfer_spend_in<'a>(
    posts: impl IntoIterator<Item = &'a CashPost>,
    team_id: &str,
    last_rollover: Option<&str>,
    from_index: Option<usize>,
) -> i64 {
    posts
        .into_iter()
        .enumerate()
        .skip(from_index.unwrap_or(0))
        .filter(|(_, post)| post.club_id == team_id)
        .filter(|(_, post)| {
            matches!(
                post.kind,
                CashKind::TransferFeeOut | CashKind::TransferFeeIn
            )
        })
        .filter(|(index, post)| match from_index {
            Some(from) => *index >= from,
            None => match last_rollover {
                Some(day) => post.date.as_str() > day,
                None => true,
            },
        })
        .fold(0i64, |spent, (_, post)| spent.saturating_sub(post.amount))
}

/// Net transfer-fee spend this season for every club, from one walk of the journal.
/// The market sweep reads this once and updates it after each completed deal.
pub(super) fn settled_transfer_spend_by_club(
    game: &Game,
) -> std::collections::HashMap<String, i64> {
    let from_index = game
        .last_season_rollover_journal_len
        .map(|len| len as usize);
    let last_rollover = game.last_season_rollover_on.as_deref();
    let mut totals = std::collections::HashMap::new();
    for (index, post) in game
        .cash_journal
        .iter()
        .enumerate()
        .skip(from_index.unwrap_or(0))
    {
        if !matches!(
            post.kind,
            CashKind::TransferFeeOut | CashKind::TransferFeeIn
        ) {
            continue;
        }
        let in_season = match from_index {
            Some(from) => index >= from,
            None => match last_rollover {
                Some(day) => post.date.as_str() > day,
                None => true,
            },
        };
        if !in_season {
            continue;
        }
        let spent = totals.entry(post.club_id.clone()).or_insert(0i64);
        *spent = spent.saturating_sub(post.amount);
    }
    totals
}

/// When the season cutoff advances but this club's limit was not renewed, fold
/// net settled spend into the stored limit so available room stays the same once
/// posts on or before the new cutoff drop out of spent.
pub(crate) fn fold_settled_spend_into_unrenewed_limits(
    game: &mut Game,
    renewed: &std::collections::HashSet<String>,
) {
    let ids: Vec<String> = game
        .teams
        .iter()
        .filter(|team| !renewed.contains(&team.id))
        .map(|team| team.id.clone())
        .collect();
    for id in ids {
        let spent = settled_transfer_spend(game, &id);
        if spent == 0 {
            continue;
        }
        if let Some(team) = game.teams.iter_mut().find(|team| team.id == id) {
            team.transfer_budget = team.transfer_budget.saturating_sub(spent);
        }
    }
}

/// Reconstruct the season limit from a save that stored remaining after settlement:
/// `remaining + spent_net = limit`. Gated by save format 10 so it cannot run twice.
pub fn lift_transfer_budgets_by_settled_spend(game: &mut Game) -> bool {
    let ids: Vec<String> = game.teams.iter().map(|team| team.id.clone()).collect();
    let mut changed = false;
    for id in ids {
        let spent = settled_transfer_spend(game, &id);
        if spent == 0 {
            continue;
        }
        if let Some(team) = game.teams.iter_mut().find(|team| team.id == id) {
            team.transfer_budget = team.transfer_budget.saturating_add(spent);
            changed = true;
        }
    }
    changed
}

/// Saved agreed offers are the reservation ledger, including those in older saves. Computing
/// from their status releases a cancelled or failed deal without a second mutable balance.
pub fn committed_transfer_fees(game: &Game, team_id: &str) -> i64 {
    committed_transfer_fees_by_club(game)
        .get(team_id)
        .copied()
        .unwrap_or(0)
}

/// The market reads this once: its pending approaches and immediate moves never alter reservations.
pub(super) fn committed_transfer_fees_by_club(
    game: &Game,
) -> std::collections::HashMap<String, i64> {
    let mut totals = std::collections::HashMap::new();
    for offer in game
        .players
        .iter()
        .flat_map(|player| &player.transfer_offers)
    {
        if offer.status == TransferOfferStatus::PendingRegistration {
            let total = totals.entry(offer.from_team_id.clone()).or_insert(0i64);
            *total = total.saturating_add(i64::try_from(offer.fee).unwrap_or(i64::MAX));
        }
    }
    totals
}

pub fn available_transfer_budget(game: &Game, team: &Team) -> i64 {
    team.transfer_budget
        .saturating_sub(settled_transfer_spend(game, &team.id))
        .saturating_sub(committed_transfer_fees(game, &team.id))
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
        assert_eq!(fee_block(&club(1_000, 800), 800, 0, 0), Ok(()));
    }

    /// Given less cash than the fee, when it is checked, then cash is what blocks it.
    #[test]
    fn a_fee_beyond_cash_is_blocked_by_cash() {
        assert_eq!(
            fee_block(&club(500, 10_000), 800, 0, 0),
            Err(FeeBlock::InsufficientFunds)
        );
    }

    /// Given cash for the fee but too little budget, when it is checked, then the budget blocks it.
    #[test]
    fn a_fee_beyond_the_budget_is_blocked_by_the_budget() {
        assert_eq!(
            fee_block(&club(10_000, 500), 800, 0, 0),
            Err(FeeBlock::TransferBudgetTooLow)
        );
    }

    /// Given a club short of both, when a fee is checked, then cash is reported first.
    #[test]
    fn a_club_short_of_both_reports_cash() {
        assert_eq!(
            fee_block(&club(100, 100), 800, 0, 0),
            Err(FeeBlock::InsufficientFunds)
        );
    }

    /// Given a fee too large for any club, when it is checked, then it reads as short of cash
    /// rather than wrapping.
    #[test]
    fn a_fee_beyond_i64_is_insufficient_funds() {
        assert_eq!(
            fee_block(&club(i64::MAX, i64::MAX), u64::MAX, 0, 0),
            Err(FeeBlock::InsufficientFunds)
        );
    }

    /// Given a negative transfer budget, when a zero fee is checked, then it is still blocked: the
    /// registration rule never clamped negatives.
    #[test]
    fn a_negative_budget_blocks_even_a_zero_fee() {
        assert_eq!(
            fee_block(&club(1_000, -1), 0, 0, 0),
            Err(FeeBlock::TransferBudgetTooLow)
        );
        assert!(!can_afford_fee(&club(1_000, -1), 0, 0, 0));
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
        assert_eq!(
            FeeBlock::TransferBudgetCommitted.error_key(),
            "be.error.transfers.transferBudgetCommitted"
        );
    }

    /// Given a grid of cash, budget (including negatives), commitments, spent and fees, when the
    /// ceiling and the block are compared, then for every fee above zero `fee <= ceiling` exactly
    /// when nothing blocks it.
    #[test]
    fn the_ceiling_agrees_with_the_block_for_every_positive_fee() {
        let amounts = [-5_000, -1, 0, 1, 799, 800, 801, 5_000, 1_000_000];
        for finance in amounts {
            for budget in amounts {
                let team = club(finance, budget);
                for committed in [0, 1, 799, 5_000, 1_000_000] {
                    for spent in [-1_000, 0, 1, 300, 800, 5_000] {
                        let ceiling = transfer_fee_ceiling(&team, committed, spent);
                        for fee in [1u64, 2, 799, 800, 801, 5_000, 999_999, 1_000_000, 2_000_000] {
                            assert_eq!(
                                fee <= ceiling,
                                fee_block(&team, fee, committed, spent).is_ok(),
                                "finance {finance}, budget {budget}, committed {committed}, spent {spent}, fee {fee}"
                            );
                        }
                    }
                }
            }
        }
    }

    /// Given a season limit that still covers the fee on paper, when settled spend has already
    /// eaten the room, then the budget is too low — reserved money is not what blocks it.
    #[test]
    fn settled_spend_blocks_a_fee_the_gross_limit_would_cover() {
        assert_eq!(
            fee_block(&club(10_000, 800), 800, 0, 300),
            Err(FeeBlock::TransferBudgetTooLow)
        );
        assert_eq!(transfer_fee_ceiling(&club(10_000, 800), 0, 300), 500);
    }

    /// Given unspent room that covers the fee and a reservation that does not, when the fee is
    /// checked, then the reservation is what blocks it.
    #[test]
    fn a_reservation_blocks_after_spend_has_been_taken() {
        assert_eq!(
            fee_block(&club(10_000, 1_000), 400, 400, 300),
            Err(FeeBlock::TransferBudgetCommitted)
        );
        assert_eq!(fee_block(&club(10_000, 1_000), 300, 400, 300), Ok(()));
    }

    /// Given fees for two clubs in one journal
    /// When spend is aggregated once
    /// Then each club's total matches walking that club alone
    #[test]
    fn one_journal_walk_totals_spend_per_club() {
        let posts = vec![
            fee_post("club", -800, CashKind::TransferFeeOut, "2026-08-01"),
            fee_post("other", -200, CashKind::TransferFeeOut, "2026-08-01"),
            fee_post("club", 100, CashKind::TransferFeeIn, "2026-08-10"),
        ];
        let game = game_with_remaining(5_000, posts);
        let totals = settled_transfer_spend_by_club(&game);
        assert_eq!(totals.get("club").copied(), Some(700));
        assert_eq!(totals.get("other").copied(), Some(200));
        assert_eq!(settled_transfer_spend(&game, "club"), 700);
        assert_eq!(settled_transfer_spend(&game, "other"), 200);
    }

    /// Given sale proceeds in the journal, when spent is derived, then available room rises by
    /// the fee without the limit changing.
    #[test]
    fn sale_proceeds_reduce_net_spend() {
        let posts = [
            fee_post("club", -800, CashKind::TransferFeeOut, "2026-08-01"),
            fee_post("club", 500, CashKind::TransferFeeIn, "2026-08-10"),
        ];
        assert_eq!(settled_transfer_spend_in(&posts, "club", None, None), 300);
    }

    /// Given a previous-season fee on the rollover day and a later one, when spent is derived,
    /// then only this season's post counts.
    #[test]
    fn spend_after_rollover_ignores_the_previous_season() {
        let posts = [
            fee_post("club", -800, CashKind::TransferFeeOut, "2026-05-30"),
            fee_post("club", -200, CashKind::TransferFeeOut, "2026-08-01"),
            fee_post("other", -1_000, CashKind::TransferFeeOut, "2026-08-01"),
        ];
        assert_eq!(
            settled_transfer_spend_in(&posts, "club", Some("2026-05-30"), None),
            200
        );
    }

    fn fee_post(club: &str, amount: i64, kind: CashKind, date: &str) -> CashPost {
        CashPost {
            id: format!("{club}-{date}-{amount}"),
            club_id: club.to_string(),
            amount,
            kind,
            date: date.to_string(),
        }
    }

    fn game_with_remaining(remaining: i64, posts: Vec<CashPost>) -> Game {
        use crate::clock::GameClock;
        use chrono::{TimeZone, Utc};
        use domain::finance::CashJournal;
        use domain::manager::Manager;

        let team = club(5_000_000, remaining);
        let mut game = Game::new(
            GameClock::new(Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap()),
            Manager::new(
                "mgr".into(),
                "Test".into(),
                "Manager".into(),
                "1980-01-01".into(),
                "England".into(),
            ),
            vec![team],
            vec![],
            vec![],
            vec![],
        );
        game.cash_journal = CashJournal::from_vec(posts);
        game
    }

    /// Given a save that stored remaining after a 1.5m purchase from a 3m limit
    /// When the limit is reconstructed
    /// Then transfer_budget is the original 3m and available room is the remaining 1.5m
    #[test]
    fn lifting_remaining_restores_the_season_limit() {
        let mut game = game_with_remaining(
            1_500_000,
            vec![fee_post(
                "club",
                -1_500_000,
                CashKind::TransferFeeOut,
                "2026-08-01",
            )],
        );
        assert!(lift_transfer_budgets_by_settled_spend(&mut game));
        assert_eq!(game.teams[0].transfer_budget, 3_000_000);
        assert_eq!(available_transfer_budget(&game, &game.teams[0]), 1_500_000);
    }

    /// Given a reconstructed limit
    /// When lift runs again
    /// Then spent is added a second time: save format 10 exists so this cannot run twice
    #[test]
    fn lifting_twice_would_double_count_spend() {
        let mut game = game_with_remaining(
            1_500_000,
            vec![fee_post(
                "club",
                -1_500_000,
                CashKind::TransferFeeOut,
                "2026-08-01",
            )],
        );
        assert!(lift_transfer_budgets_by_settled_spend(&mut game));
        assert!(lift_transfer_budgets_by_settled_spend(&mut game));
        assert_eq!(game.teams[0].transfer_budget, 4_500_000);
    }

    /// Given a club whose limit was not renewed and a 400k purchase against a 500k limit
    /// When spend is folded in and the cutoff then advances past that purchase
    /// Then available room stays 100k
    #[test]
    fn folding_spend_into_the_limit_preserves_available_room_after_the_cutoff_advances() {
        let mut game = game_with_remaining(
            500_000,
            vec![fee_post(
                "club",
                -400_000,
                CashKind::TransferFeeOut,
                "2026-04-01",
            )],
        );
        fold_settled_spend_into_unrenewed_limits(&mut game, &std::collections::HashSet::new());
        game.last_season_rollover_on = Some("2026-05-20".to_string());
        assert_eq!(game.teams[0].transfer_budget, 100_000);
        assert_eq!(available_transfer_budget(&game, &game.teams[0]), 100_000);
    }

    /// Given a new season whose cutoff is today, a same-day fee already in the old prefix, and a cursor at that length
    /// When an 80k fee posts on that same calendar day
    /// Then spent is 80k (the prefix fee is excluded) and available room is 20k of a 100k limit
    #[test]
    fn a_settlement_on_the_rollover_day_after_the_cutoff_counts_in_the_new_season() {
        let mut game = game_with_remaining(
            100_000,
            vec![fee_post(
                "club",
                -50_000,
                CashKind::TransferFeeOut,
                "2026-05-20",
            )],
        );
        game.last_season_rollover_on = Some("2026-05-20".to_string());
        game.last_season_rollover_journal_len = Some(game.cash_journal.len() as u32);
        game.cash_journal.extend([fee_post(
            "club",
            -80_000,
            CashKind::TransferFeeOut,
            "2026-05-20",
        )]);
        assert_eq!(settled_transfer_spend(&game, "club"), 80_000);
        assert_eq!(available_transfer_budget(&game, &game.teams[0]), 20_000);
    }
}
