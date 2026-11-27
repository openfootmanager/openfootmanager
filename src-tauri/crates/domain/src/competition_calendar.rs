//! Calendar specification and edition provenance; renewal policy lives in `ofm_core`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Renamed in TypeScript only: `season::SeasonPhase` (pre-season, in season, post-season) has the
/// same Rust name, and the screens need both. The JSON is unchanged.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "CompetitionSeasonPhase")]
pub enum SeasonPhase {
    #[default]
    Annual,
    Opening,
    Closing,
}

impl SeasonPhase {
    /// The division phase a reader sees: no phase for a single annual table, otherwise the same
    /// spelling the calendar serializes (`"opening"` / `"closing"`). The stats screen, the team
    /// directory and the end-of-season notice all name the phase through here, so the spellings
    /// have one home rather than three hand-written matches.
    pub fn division_phase_name(self) -> Option<&'static str> {
        match self {
            Self::Annual => None,
            Self::Opening => Some("opening"),
            Self::Closing => Some("closing"),
        }
    }
}

/// Explicit division identity, independent of current participants or display names.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, TS)]
#[serde(rename_all = "camelCase")]
pub struct DivisionIdentity {
    #[serde(default)]
    pub family_id: String,
    #[serde(default)]
    pub tier: u32,
    #[serde(default)]
    pub phase: SeasonPhase,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, TS)]
pub struct CalendarDate {
    #[serde(default)]
    pub month: u8,
    #[serde(default)]
    pub day: u8,
}

/// Optional authored bounds. The opener remains League's existing month/day fields.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, TS)]
#[serde(rename_all = "camelCase")]
pub struct SeasonCalendar {
    #[serde(default)]
    pub division: Option<DivisionIdentity>,
    /// An end before the opener's month/day belongs to the following year.
    #[serde(default)]
    pub window_end: Option<CalendarDate>,
}

/// Interpretation of the existing `League::season`, without changing old counters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum EditionBasis {
    CalendarYear,
    LegacyOrdinal {
        /// The saved season counter at `opener_year`, refreshed when this edition is generated.
        #[serde(default, alias = "first_season")]
        season_at_opener: u32,
        #[serde(default)]
        opener_year: i32,
    },
    #[default]
    Unresolved,
}

/// Durable definition identity, supported edition provenance and authored ordinary-table shape.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, TS)]
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

#[cfg(test)]
mod tests {
    use super::{EditionBasis, SeasonPhase};
    use serde_json::json;

    /// Given each season phase
    /// When it is named for a division
    /// Then an annual table has no phase, and the split phases read exactly as the calendar
    ///      serializes them, so the two spellings cannot drift apart.
    #[test]
    fn a_division_phase_reads_as_its_serialized_spelling() {
        assert_eq!(SeasonPhase::Annual.division_phase_name(), None);
        for phase in [SeasonPhase::Opening, SeasonPhase::Closing] {
            let serialized: String =
                serde_json::from_value(serde_json::to_value(phase).expect("a phase serializes"))
                    .expect("a phase serializes to a string");
            assert_eq!(phase.division_phase_name(), Some(serialized.as_str()));
        }
    }

    /// Given tip JSON, when loaded and saved, then the old counter gets its precise name.
    #[test]
    fn legacy_ordinal_field_loads_and_writes_its_meaning() {
        let basis: EditionBasis = serde_json::from_value(json!({
            "kind": "legacyOrdinal", "first_season": 6, "opener_year": 2031
        }))
        .unwrap();
        assert_eq!(
            serde_json::to_value(basis).unwrap(),
            json!({
                "kind": "legacyOrdinal", "season_at_opener": 6, "opener_year": 2031
            })
        );
    }

    /// Given the renamed field, when loaded and saved, then its counter and defaults survive.
    #[test]
    fn season_at_opener_round_trips_without_losing_the_counter() {
        let raw = json!({"kind": "legacyOrdinal", "season_at_opener": 6, "opener_year": 2031});
        let basis: EditionBasis = serde_json::from_value(raw.clone()).unwrap();
        assert_eq!(serde_json::to_value(basis).unwrap(), raw);
        let missing: EditionBasis =
            serde_json::from_value(json!({"kind": "legacyOrdinal"})).unwrap();
        assert_eq!(
            serde_json::to_value(missing).unwrap(),
            json!({
                "kind": "legacyOrdinal", "season_at_opener": 0, "opener_year": 0
            })
        );
    }
    /// Given ambiguous or malformed counters, when loaded, then deserialization rejects them.
    #[test]
    fn conflicting_or_invalid_ordinal_counters_are_rejected() {
        for raw in [
            json!({"kind": "legacyOrdinal", "first_season": 6, "season_at_opener": 7, "opener_year": 2031}),
            json!({"kind": "legacyOrdinal", "season_at_opener": "six", "opener_year": 2031}),
        ] {
            assert!(serde_json::from_value::<EditionBasis>(raw).is_err());
        }
    }
}
