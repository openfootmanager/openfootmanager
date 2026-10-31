//! The one good-to-bad scale every rated value is painted on.
//!
//! The backend decides where a value sits on it, the way `player_tier` decides a tier, and the
//! screens map each step to its `tone-*` colour token. So no screen compares a number against a
//! threshold of its own, and a value reads the same wherever it appears.

use serde::Serialize;
use ts_rs::TS;

/// A step on the good-to-bad scale. Serialized lowercase, as the `tone-*` tokens are named.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum RatedTone {
    Top,
    Good,
    Fair,
    Low,
    Bad,
}
