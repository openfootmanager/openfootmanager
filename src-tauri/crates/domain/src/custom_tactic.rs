use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::team::{MatchRoles, PlayerRole, TacticsPhaseSettings};

/// A tactic the player saved: a name, a formation, a play style and, since #1189, everything else
/// the tactic is (its instructions, the eleven and their roles, the set-piece takers). It is kept
/// with the career rather than the club, so it survives a change of club and travels with the save.
///
/// `formation` and `play_style` are the strings a team carries (`"4-4-2"`, `"HighPress"`); the
/// rules for which are valid live in `ofm_core::custom_tactics`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
pub struct CustomTactic {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub formation: String,
    pub play_style: String,
    /// The preset this one was started from, if any.
    #[serde(default)]
    pub source_preset_name: Option<String>,
    /// What the tactic holds beyond its shape and style. `None` for a tactic saved before tactics
    /// held it (#1189): applying one of those changes only its formation and play style.
    #[serde(default)]
    pub contents: Option<TacticContents>,
}

/// The rest of a saved tactic: what a manager sets on the Tactics screen besides the formation
/// and the style.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
pub struct TacticContents {
    /// Every team instruction, in and out of possession and in transition.
    #[serde(default)]
    pub instructions: TacticsPhaseSettings,
    /// The starting eleven, slot by slot in the formation's order, each with its role there.
    #[serde(default)]
    pub slots: Vec<SavedSlot>,
    /// The captain, vice-captain and set-piece takers.
    #[serde(default)]
    pub match_roles: MatchRoles,
}

/// One slot of a saved eleven: who plays there and the role he was given.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
pub struct SavedSlot {
    pub player_id: String,
    /// `None` is the position's standard role.
    #[serde(default)]
    pub role: Option<PlayerRole>,
}
