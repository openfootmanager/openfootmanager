//! Tunable thresholds and translation-key error constants shared across the
//! contract clusters. Extracted verbatim from the original contracts.rs.

pub(crate) const RENEWAL_SESSION_STALE_DAYS: i64 = 14;
pub(crate) const INSULTING_RENEWAL_BLOCK_DAYS: u64 = 30;
pub(crate) const MAX_CONTRACT_YEARS: u32 = 5;
pub(crate) const MARKET_VALUE_TO_WAGE_RATIO: u64 = 200;
pub(crate) const MINIMUM_DEFAULT_WAGE: u64 = 500;
pub(crate) const ERR_NO_TEAM_ASSIGNED: &str = "be.error.noTeamAssigned";
pub(crate) const ERR_MANAGED_TEAM_NOT_FOUND: &str = "be.error.managedTeamNotFound";
pub(crate) const ERR_PLAYER_NOT_FOUND: &str = "be.error.playerNotFound";
pub(crate) const ERR_PLAYER_NOT_OWNED_BY_CLUB: &str = "be.error.contracts.playerNotOwnedByClub";
pub(crate) const ERR_PLAYER_NOT_FREE_AGENT: &str = "be.error.contracts.playerNotFreeAgent";
pub(crate) const ERR_UNABLE_TO_CALCULATE_CONTRACT_END_DATE: &str =
    "be.error.contracts.unableToCalculateContractEndDate";
pub(crate) const ERR_PLAYER_HAS_NO_ACTIVE_CONTRACT: &str =
    "be.error.contracts.playerHasNoActiveContract";
pub(crate) const ERR_PLAYER_ON_ACTIVE_LOAN: &str = "be.error.contracts.playerOnActiveLoan";
pub(crate) const ERR_TERMINATION_WOULD_LEAVE_MATCHDAY_SQUAD_SHORT: &str =
    "be.error.contracts.terminationWouldLeaveMatchdaySquadShort";
