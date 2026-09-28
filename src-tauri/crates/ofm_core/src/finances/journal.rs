use std::collections::{HashMap, HashSet};

use crate::game::Game;
use chrono::NaiveDate;
use domain::finance::{CashKind, CashPost};

const ERR_TEAM_NOT_FOUND: &str = "be.error.managedTeamNotFound";
const ERR_OVERFLOW: &str = "be.error.finance.amountOverflow";
const ERR_OPENING_BALANCE: &str = "be.error.finance.openingBalanceReserved";

/// In-memory request. Dates become `YYYY-MM-DD` strings on the persisted post.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostRequest {
    pub club_id: String,
    pub amount: i64,
    pub kind: CashKind,
    pub date: NaiveDate,
}

impl PostRequest {
    pub fn new(club_id: impl Into<String>, amount: i64, kind: CashKind, date: NaiveDate) -> Self {
        Self {
            club_id: club_id.into(),
            amount,
            kind,
            date,
        }
    }
}

/// Post one movement. If this is the club's first journal row, an OpeningBalance
/// seed is prepended; the returned id is the movement, not the seed.
pub fn post(
    game: &mut Game,
    club_id: impl Into<String>,
    amount: i64,
    kind: CashKind,
    date: NaiveDate,
) -> Result<Option<String>, String> {
    let mut ids = post_all(game, &[PostRequest::new(club_id, amount, kind, date)])?;
    Ok(ids.pop())
}

/// Validate every club, then append all rows and update caches. Partial posts
/// are not visible: a missing seller leaves the buyer untouched.
pub fn post_all(game: &mut Game, reqs: &[PostRequest]) -> Result<Vec<String>, String> {
    let prepared = prepare_posts(game, reqs)?;
    if prepared.is_empty() {
        return Ok(Vec::new());
    }
    Ok(commit_posts(game, prepared))
}

/// Same checks as `post_all`, with no mutation. Callers that must not leave a
/// half-applied gameplay change (accepted offer, moved player) use this first.
pub fn validate_posts(game: &Game, reqs: &[PostRequest]) -> Result<(), String> {
    prepare_posts(game, reqs).map(|_| ())
}

struct PreparedPost {
    team_index: usize,
    post: CashPost,
}

fn prepare_posts(game: &Game, reqs: &[PostRequest]) -> Result<Vec<PreparedPost>, String> {
    let team_index: HashMap<&str, usize> = game
        .teams
        .iter()
        .enumerate()
        .map(|(index, team)| (team.id.as_str(), index))
        .collect();

    let mut seeded: HashSet<&str> = HashSet::new();
    let mut running: HashMap<&str, i64> = HashMap::new();
    let mut prepared = Vec::with_capacity(reqs.len());

    for req in reqs {
        if req.kind == CashKind::OpeningBalance {
            return Err(ERR_OPENING_BALANCE.to_string());
        }
        if req.amount == 0 {
            continue;
        }

        let &index = team_index
            .get(req.club_id.as_str())
            .ok_or_else(|| ERR_TEAM_NOT_FOUND.to_string())?;
        let team = &game.teams[index];

        if !game.cash_journal.contains_club(req.club_id.as_str())
            && !seeded.contains(req.club_id.as_str())
        {
            prepared.push(PreparedPost {
                team_index: index,
                post: build_post(&team.id, team.finance, CashKind::OpeningBalance, req.date),
            });
        }
        seeded.insert(req.club_id.as_str());

        let slot = running.entry(req.club_id.as_str()).or_insert(team.finance);
        *slot = slot
            .checked_add(req.amount)
            .ok_or_else(|| ERR_OVERFLOW.to_string())?;

        prepared.push(PreparedPost {
            team_index: index,
            post: build_post(&team.id, req.amount, req.kind, req.date),
        });
    }

    Ok(prepared)
}

fn commit_posts(game: &mut Game, prepared: Vec<PreparedPost>) -> Vec<String> {
    let mut ids = Vec::with_capacity(prepared.len());
    let mut posts = Vec::with_capacity(prepared.len());
    let mut touched: HashSet<usize> = HashSet::new();

    for item in prepared {
        if item.post.kind != CashKind::OpeningBalance {
            let team = &mut game.teams[item.team_index];
            team.finance = team
                .finance
                .checked_add(item.post.amount)
                .expect("prepare_posts rejected overflow");
            if item.post.kind.counts_toward_season_totals() {
                if item.post.amount > 0 {
                    team.season_income = team.season_income.saturating_add(item.post.amount);
                } else if item.post.amount < 0 {
                    team.season_expenses = team
                        .season_expenses
                        .saturating_add(item.post.amount.saturating_neg());
                }
            }
        }
        touched.insert(item.team_index);

        log::trace!(
            "cash post id={} club={} kind={:?} amount={} date={}",
            item.post.id,
            item.post.club_id,
            item.post.kind,
            item.post.amount,
            item.post.date
        );
        ids.push(item.post.id.clone());
        posts.push(item.post);
    }

    log::debug!("cash journal committed {} posts", posts.len());
    game.cash_journal_dirty_ids.extend(ids.iter().cloned());
    game.cash_journal.extend(posts);
    debug_assert!(
        journal_matches_cash_for(game, &touched),
        "cash journal drifted from Team.finance"
    );
    ids
}

fn build_post(club_id: &str, amount: i64, kind: CashKind, date: NaiveDate) -> CashPost {
    CashPost {
        id: uuid::Uuid::new_v4().to_string(),
        club_id: club_id.to_string(),
        amount,
        kind,
        date: date.format("%Y-%m-%d").to_string(),
    }
}

/// Clubs with no journal rows yet are allowed to disagree with the cache;
/// the first `post` seeds an OpeningBalance so they line up afterwards.
pub fn journal_matches_cash(game: &Game) -> bool {
    game.teams.iter().all(|team| club_matches_cash(game, team))
}

fn journal_matches_cash_for(game: &Game, team_indices: &HashSet<usize>) -> bool {
    team_indices.iter().all(|&index| {
        game.teams
            .get(index)
            .is_some_and(|team| club_matches_cash(game, team))
    })
}

fn club_matches_cash(game: &Game, team: &domain::team::Team) -> bool {
    !game.cash_journal.contains_club(&team.id)
        || game.cash_journal.cash_for(&team.id) == team.finance
}

/// Credit cash as `OpeningBalance` and update `Team.finance`.
///
/// Public [`post`] rejects `OpeningBalance` so gameplay cannot mint it.
/// Save-format floors (weekly unit lock) use this instead.
pub fn credit_opening_cash(
    game: &mut Game,
    club_id: &str,
    amount: i64,
    date: NaiveDate,
) -> Result<(), String> {
    if amount == 0 {
        return Ok(());
    }
    if amount < 0 {
        return Err(ERR_OPENING_BALANCE.to_string());
    }

    let index = game
        .teams
        .iter()
        .position(|team| team.id == club_id)
        .ok_or_else(|| ERR_TEAM_NOT_FOUND.to_string())?;
    let current = game.teams[index].finance;
    let new_finance = current
        .checked_add(amount)
        .ok_or_else(|| ERR_OVERFLOW.to_string())?;

    let mut posts = Vec::with_capacity(2);
    if !game.cash_journal.contains_club(club_id) && current != 0 {
        posts.push(build_post(
            &game.teams[index].id,
            current,
            CashKind::OpeningBalance,
            date,
        ));
    }
    posts.push(build_post(
        &game.teams[index].id,
        amount,
        CashKind::OpeningBalance,
        date,
    ));

    game.teams[index].finance = new_finance;
    game.cash_journal_dirty_ids
        .extend(posts.iter().map(|post| post.id.clone()));
    game.cash_journal.extend(posts);
    debug_assert!(
        club_matches_cash(game, &game.teams[index]),
        "cash journal drifted from Team.finance"
    );
    Ok(())
}

/// Import `financial_ledger` rows and an OpeningBalance so `sum(journal) == finance`.
///
/// Call from `save_manager::load_game` when the loaded journal is empty.
/// Returns whether any posts were added (caller should resave).
pub fn backfill_opening_balances(game: &mut Game) -> bool {
    if !game.cash_journal.is_empty() || game.teams.is_empty() {
        return false;
    }

    let date = game.clock.current_date.date_naive();
    let mut batch = Vec::new();

    for team in &game.teams {
        match reconstructable_opening(team.finance, &team.financial_ledger) {
            Some(opening) => {
                batch.push(build_post(
                    &team.id,
                    opening,
                    CashKind::OpeningBalance,
                    date,
                ));
                for entry in &team.financial_ledger {
                    let parsed = NaiveDate::parse_from_str(&entry.date, "%Y-%m-%d").unwrap_or(date);
                    batch.push(build_post(
                        &team.id,
                        entry.amount,
                        CashKind::from_legacy_ledger(entry.kind),
                        parsed,
                    ));
                }
            }
            None => {
                batch.push(build_post(
                    &team.id,
                    team.finance,
                    CashKind::OpeningBalance,
                    date,
                ));
            }
        }
    }

    let ids: Vec<String> = batch.iter().map(|post| post.id.clone()).collect();
    game.cash_journal.extend(batch);
    game.cash_journal_dirty_ids.extend(ids);
    true
}

fn reconstructable_opening(
    finance: i64,
    ledger: &[domain::team::FinancialTransaction],
) -> Option<i64> {
    let imported_sum = ledger
        .iter()
        .try_fold(0i64, |acc, entry| acc.checked_add(entry.amount))?;
    let opening = finance.checked_sub(imported_sum)?;
    let mut running = opening;
    for entry in ledger {
        running = running.checked_add(entry.amount)?;
    }
    debug_assert_eq!(running, finance);
    Some(opening)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::team::{FinancialTransaction, FinancialTransactionKind, Team};

    fn make_team(id: &str, finance: i64) -> Team {
        let mut team = Team::new(
            id.to_string(),
            format!("{id} FC"),
            id[..3.min(id.len())].to_string(),
            "England".to_string(),
            "Testville".to_string(),
            "Ground".to_string(),
            20_000,
        );
        team.finance = finance;
        team
    }

    fn make_game(teams: Vec<Team>) -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 2, 16, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "mgr".to_string(),
            "Alex".to_string(),
            "Boss".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire(teams[0].id.clone());
        Game::new(clock, manager, teams, vec![], vec![], vec![])
    }

    fn monday() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 2, 16).unwrap()
    }

    #[test]
    fn post_seeds_opening_balance_then_applies_the_movement() {
        let mut game = make_game(vec![make_team("alpha", 1_000_000)]);

        let movement_id = post(&mut game, "alpha", -1_000, CashKind::PlayerWages, monday())
            .unwrap()
            .expect("movement");

        assert_eq!(game.teams[0].finance, 999_000);
        assert_eq!(game.cash_journal.len(), 2);
        assert_eq!(game.cash_journal[0].kind, CashKind::OpeningBalance);
        assert_eq!(game.cash_journal[0].amount, 1_000_000);
        assert_eq!(game.cash_journal[1].kind, CashKind::PlayerWages);
        assert_eq!(game.cash_journal[1].amount, -1_000);
        assert_eq!(game.cash_journal[1].id, movement_id);
        assert_eq!(game.cash_journal.cash_for("alpha"), 999_000);
        assert_eq!(game.teams[0].season_expenses, 1_000);
        assert_eq!(game.cash_journal_dirty_ids.len(), 2);
        assert!(journal_matches_cash(&game));
    }

    #[test]
    fn opening_balance_does_not_inflate_season_totals() {
        let mut game = make_game(vec![make_team("alpha", 500_000)]);
        post(&mut game, "alpha", 10_000, CashKind::Matchday, monday()).unwrap();
        assert_eq!(game.teams[0].season_income, 10_000);
        assert_eq!(game.teams[0].season_expenses, 0);
    }

    #[test]
    fn transfer_fees_do_not_change_season_totals() {
        let mut game = make_game(vec![
            make_team("buyer", 2_000_000),
            make_team("seller", 1_000_000),
        ]);
        post_all(
            &mut game,
            &[
                PostRequest::new("buyer", -750_000, CashKind::TransferFeeOut, monday()),
                PostRequest::new("seller", 750_000, CashKind::TransferFeeIn, monday()),
            ],
        )
        .unwrap();

        assert_eq!(game.teams[0].finance, 1_250_000);
        assert_eq!(game.teams[1].finance, 1_750_000);
        assert_eq!(game.teams[0].season_income, 0);
        assert_eq!(game.teams[0].season_expenses, 0);
        assert_eq!(game.teams[1].season_income, 0);
        assert_eq!(game.teams[1].season_expenses, 0);
    }

    #[test]
    fn post_all_is_atomic_when_a_club_is_missing() {
        let mut game = make_game(vec![make_team("alpha", 1_000_000)]);
        let err = post_all(
            &mut game,
            &[
                PostRequest::new("alpha", -50_000, CashKind::TransferFeeOut, monday()),
                PostRequest::new("beta", 50_000, CashKind::TransferFeeIn, monday()),
            ],
        )
        .unwrap_err();

        assert_eq!(err, ERR_TEAM_NOT_FOUND);
        assert_eq!(game.teams[0].finance, 1_000_000);
        assert!(game.cash_journal.is_empty());
        assert!(game.cash_journal_dirty_ids.is_empty());
    }

    #[test]
    fn zero_amount_is_skipped() {
        let mut game = make_game(vec![make_team("alpha", 10)]);
        let id = post(&mut game, "alpha", 0, CashKind::PlayerWages, monday()).unwrap();
        assert!(id.is_none());
        assert!(game.cash_journal.is_empty());
    }

    #[test]
    fn cloning_a_game_does_not_clone_journal_storage() {
        let mut game = make_game(vec![make_team("alpha", 100)]);
        post(&mut game, "alpha", -1, CashKind::StaffWages, monday()).unwrap();
        let clone = game.clone();
        assert!(game.cash_journal.shares_storage_with(&clone.cash_journal));
    }

    #[test]
    fn negative_cash_is_allowed() {
        let mut game = make_game(vec![make_team("alpha", 10)]);
        post(&mut game, "alpha", -50, CashKind::Facilities, monday()).unwrap();
        assert_eq!(game.teams[0].finance, -40);
        assert_eq!(game.cash_journal.cash_for("alpha"), -40);
    }

    #[test]
    fn explicit_opening_balance_is_rejected() {
        let mut game = make_game(vec![make_team("alpha", 100)]);
        let err = post(&mut game, "alpha", 100, CashKind::OpeningBalance, monday()).unwrap_err();
        assert_eq!(err, ERR_OPENING_BALANCE);
        assert_eq!(game.teams[0].finance, 100);
        assert!(game.cash_journal.is_empty());
    }

    #[test]
    fn min_expense_does_not_overflow_season_totals() {
        let mut game = make_game(vec![make_team("alpha", 0)]);
        post(&mut game, "alpha", i64::MIN, CashKind::Facilities, monday()).unwrap();
        assert_eq!(game.teams[0].finance, i64::MIN);
        assert_eq!(game.teams[0].season_expenses, i64::MAX);
        assert_eq!(game.cash_journal.cash_for("alpha"), i64::MIN);
    }

    #[test]
    fn overflow_is_rejected_atomically() {
        let mut game = make_game(vec![make_team("alpha", i64::MAX)]);
        let err = post(&mut game, "alpha", 1, CashKind::Matchday, monday()).unwrap_err();
        assert_eq!(err, ERR_OVERFLOW);
        assert_eq!(game.teams[0].finance, i64::MAX);
        assert!(game.cash_journal.is_empty());
        assert!(game.cash_journal_dirty_ids.is_empty());
    }

    #[test]
    fn a_second_post_does_not_reseed_opening_balance() {
        let mut game = make_game(vec![make_team("alpha", 100)]);
        post(&mut game, "alpha", -10, CashKind::PlayerWages, monday()).unwrap();
        post(&mut game, "alpha", 5, CashKind::Matchday, monday()).unwrap();
        let openings = game
            .cash_journal
            .iter()
            .filter(|post| post.kind == CashKind::OpeningBalance)
            .count();
        assert_eq!(openings, 1);
        assert_eq!(game.cash_journal.len(), 3);
        assert_eq!(game.cash_journal.cash_for("alpha"), 95);
    }

    #[test]
    fn backfill_imports_ledger_and_balances_to_finance() {
        let mut team = make_team("alpha", 1_000_000);
        team.financial_ledger.push(FinancialTransaction {
            date: "2026-01-01".to_string(),
            description: "prize".to_string(),
            amount: 5_000_000,
            kind: FinancialTransactionKind::PrizeMoney,
        });
        team.finance = 1_000_000;
        let mut game = make_game(vec![team]);

        assert!(backfill_opening_balances(&mut game));
        assert_eq!(game.cash_journal.len(), 2);
        assert_eq!(game.cash_journal.cash_for("alpha"), 1_000_000);
        let opening = game
            .cash_journal
            .iter()
            .find(|post| post.kind == CashKind::OpeningBalance)
            .unwrap();
        assert_eq!(opening.amount, -4_000_000);
        assert!(!backfill_opening_balances(&mut game));
    }

    #[test]
    fn backfill_skips_unrepresentable_ledger_history() {
        let mut team = make_team("alpha", i64::MIN);
        team.financial_ledger.push(FinancialTransaction {
            date: "2026-01-01".to_string(),
            description: "prize".to_string(),
            amount: i64::MAX,
            kind: FinancialTransactionKind::PrizeMoney,
        });
        let mut game = make_game(vec![team]);

        assert!(backfill_opening_balances(&mut game));
        assert_eq!(game.cash_journal.len(), 1);
        assert_eq!(game.cash_journal[0].kind, CashKind::OpeningBalance);
        assert_eq!(game.cash_journal.cash_for("alpha"), i64::MIN);
        assert_eq!(game.teams[0].finance, i64::MIN);
    }

    #[test]
    fn backfill_skips_history_when_a_prefix_overflows() {
        let mut team = make_team("alpha", i64::MAX);
        team.financial_ledger.push(FinancialTransaction {
            date: "2026-01-01".to_string(),
            description: "in".to_string(),
            amount: 1,
            kind: FinancialTransactionKind::PrizeMoney,
        });
        team.financial_ledger.push(FinancialTransaction {
            date: "2026-01-02".to_string(),
            description: "out".to_string(),
            amount: -1,
            kind: FinancialTransactionKind::ContractTermination,
        });
        let mut game = make_game(vec![team]);

        assert!(backfill_opening_balances(&mut game));
        assert_eq!(game.cash_journal.len(), 1);
        assert_eq!(game.cash_journal[0].kind, CashKind::OpeningBalance);
        assert_eq!(game.cash_journal[0].amount, i64::MAX);
        assert_eq!(game.cash_journal.cash_for("alpha"), i64::MAX);
    }

    #[test]
    fn credit_opening_cash_raises_finance_and_keeps_the_journal_in_sync() {
        let mut game = make_game(vec![make_team("alpha", 1_000)]);

        credit_opening_cash(&mut game, "alpha", 15_000, monday()).unwrap();

        assert_eq!(game.teams[0].finance, 16_000);
        assert_eq!(game.cash_journal.cash_for("alpha"), 16_000);
        assert_eq!(game.teams[0].season_income, 0);
        assert!(journal_matches_cash(&game));
    }
}
