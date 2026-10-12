//! Builders shared by `ofm_core`'s unit tests.
//!
//! `PlayerAttributes` has nineteen fields and no `Default` — deliberately, because an all-zero
//! player is a nonsense one and a test that builds it silently. So every test module that needs a
//! player has written its own `default_attrs()`, and there are thirteen of them in this crate,
//! each a copy of the same nineteen lines.
//!
//! This is that helper, once. New tests use it; the thirteen existing copies are reported under
//! epic #589 rather than migrated here, which would put a crate-wide sweep inside an unrelated fix.

use domain::player::PlayerAttributes;

/// Every attribute at `value`. Enough for any test that needs a player who exists and is not
/// remarkable — squad selection, strength averages, carry-back.
pub(crate) fn uniform_attributes(value: u8) -> PlayerAttributes {
    PlayerAttributes {
        pace: value,
        stamina: value,
        strength: value,
        agility: value,
        passing: value,
        shooting: value,
        tackling: value,
        dribbling: value,
        defending: value,
        positioning: value,
        vision: value,
        decisions: value,
        composure: value,
        aggression: value,
        teamwork: value,
        leadership: value,
        handling: value,
        reflexes: value,
        aerial: value,
    }
}
