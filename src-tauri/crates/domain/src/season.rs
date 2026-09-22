use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum SeasonPhase {
    #[default]
    Preseason,
    InSeason,
    PostSeason,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum TransferWindowStatus {
    #[default]
    Closed,
    Open,
    DeadlineDay,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct TransferWindowContext {
    pub status: TransferWindowStatus,
    pub opens_on: Option<String>,
    pub closes_on: Option<String>,
    pub days_until_opens: Option<i64>,
    pub days_remaining: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct SeasonContext {
    pub phase: SeasonPhase,
    pub season_start: Option<String>,
    pub season_end: Option<String>,
    pub days_until_season_start: Option<i64>,
    pub transfer_window: TransferWindowContext,
    /// Whether the rollover to next season is available *now*.
    ///
    /// This is the same answer `advance_to_next_season` enforces before it will
    /// do anything, so the screen that offers the rollover and the command that
    /// performs it can never disagree. `phase` is not a substitute: it is
    /// derived from the user's primary league alone, while this accounts for
    /// every division the rollover waits on.
    pub season_complete: bool,
}
