//! Calendar specification and edition provenance; renewal policy lives in `ofm_core`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum SeasonPhase {
    #[default]
    Annual,
    Opening,
    Closing,
}

/// Explicit division identity, independent of current participants or display names.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DivisionIdentity {
    #[serde(default)]
    pub family_id: String,
    #[serde(default)]
    pub tier: u32,
    #[serde(default)]
    pub phase: SeasonPhase,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CalendarDate {
    #[serde(default)]
    pub month: u8,
    #[serde(default)]
    pub day: u8,
}

/// Optional authored bounds. The opener remains League's existing month/day fields.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SeasonCalendar {
    #[serde(default)]
    pub division: Option<DivisionIdentity>,
    /// An end before the opener's month/day belongs to the following year.
    #[serde(default)]
    pub window_end: Option<CalendarDate>,
}

/// Interpretation of the existing `League::season`, without changing old counters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum EditionBasis {
    CalendarYear,
    LegacyOrdinal {
        #[serde(default)]
        first_season: u32,
        #[serde(default)]
        opener_year: i32,
    },
    #[default]
    Unresolved,
}

/// Durable definition identity, supported edition provenance and authored ordinary-table shape.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CalendarMetadata {
    #[serde(default)]
    pub definition_id: String,
    #[serde(default)]
    pub edition_basis: EditionBasis,
    /// Authored ordinary table legs; unknown on old saves that never stored the authoring format.
    #[serde(default)]
    pub league_legs: Option<u8>,
    /// Ordinary table cadence; cups retain their existing format-specific rules.
    #[serde(default)]
    pub matchday_gap_days: Option<u32>,
    #[serde(default)]
    pub season: SeasonCalendar,
}
