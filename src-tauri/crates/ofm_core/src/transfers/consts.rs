//! Tuning constants and error keys for the transfer market.
//!
//! Kept together so the numbers that shape AI behaviour, and the translation
//! keys the UI resolves, can be read and adjusted without scrolling the logic.

pub(super) const TRANSFER_NEGOTIATION_STALE_DAYS: i64 = 14;
pub(super) const MAX_COMPLETED_AI_TRANSFERS_PER_DAY: usize = 2;
pub(super) const AWARD_LEADERBOARD_INTEREST_BONUS: i32 = 25;
/// Only one new club may open talks for a given user player on a single day,
/// so stars draw steady interest over the window instead of a same-day flood.
pub(super) const MAX_NEW_INCOMING_OFFERS_PER_USER_PLAYER_PER_DAY: usize = 1;
/// Ceiling on brand-new incoming offers across the whole user squad per day.
pub(super) const MAX_NEW_INCOMING_USER_OFFERS_PER_DAY: usize = 3;
/// A club won't pursue a player whose current club out-reputes it by more than
/// this margin — the player wouldn't realistically drop to a much smaller side.
pub(super) const MAX_BUYER_REPUTATION_DEFICIT: i32 = 150;
/// A club already this deep in a position group has no need to sign another
/// there, so it looks elsewhere.
pub(super) const POSITION_GROUP_SURPLUS_THRESHOLD: usize = 8;
pub(super) const ERR_TRANSFER_WINDOW_CLOSED: &str = "be.error.transfers.transferWindowClosed";
pub(super) const ERR_CANNOT_BID_ON_OWN_PLAYER: &str = "be.error.transfers.cannotBidOnOwnPlayer";
pub(super) const ERR_PLAYER_HAS_NO_TEAM: &str = "be.error.transfers.playerHasNoTeam";
pub(super) const ERR_INSUFFICIENT_FUNDS: &str = "be.error.transfers.insufficientFunds";
pub(super) const ERR_TRANSFER_BUDGET_TOO_LOW: &str = "be.error.transfers.transferBudgetTooLow";
pub(super) const ERR_PLAYER_NOT_OWNED_BY_USER: &str = "be.error.transfers.playerNotOwnedByUser";
pub(super) const ERR_OFFER_NOT_PENDING: &str = "be.error.transfers.offerNotPending";
pub(super) const ERR_COUNTER_OFFER_MUST_EXCEED_CURRENT: &str =
    "be.error.transfers.counterOfferMustExceedCurrentOffer";
pub(super) const ERR_LOAN_COUNTER_MUST_IMPROVE_TERMS: &str =
    "be.error.transfers.loanCounterMustImproveTerms";
pub(super) const ERR_PLAYER_NOT_LOAN_LISTED: &str = "be.error.transfers.playerNotLoanListed";
pub(super) const ERR_PLAYER_ALREADY_LOANED: &str = "be.error.transfers.playerAlreadyLoaned";
pub(super) const ERR_INVALID_LOAN_END_DATE: &str = "be.error.transfers.invalidLoanEndDate";
pub(super) const ERR_INVALID_LOAN_WAGE_CONTRIBUTION: &str =
    "be.error.transfers.invalidLoanWageContribution";
pub(super) const ERR_INVALID_LOAN_BUY_OPTION: &str = "be.error.transfers.invalidLoanBuyOption";
pub(super) const ERR_NO_LOAN_BUY_OPTION: &str = "be.error.transfers.noLoanBuyOption";
pub(super) const ERR_LOAN_BUY_OPTION_NOT_AVAILABLE: &str =
    "be.error.transfers.loanBuyOptionNotAvailable";
pub(super) const LOAN_DEVELOPMENT_REPORT_INTERVAL_DAYS: i64 = 30;
pub(super) const OPENING_LOAN_LISTINGS_PER_AI_TEAM: usize = 2;
pub(super) const MIN_OPENING_LOAN_CONTRACT_RUNWAY_DAYS: i64 = 90;
