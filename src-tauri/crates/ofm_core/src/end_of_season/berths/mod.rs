//! End-of-season movement between competitions.
//!
//! Split by the question each part answers, because they had grown into one 2,369-line file:
//! [`pyramid`] moves clubs between adjacent rungs of a country's own ladder, [`domestic`] moves
//! them along declared berths between sibling competitions, and [`continental`] works out who
//! qualifies for a competition fed by berths.

mod continental;
mod domestic;
mod pyramid;

#[cfg(test)]
mod test_support;

/// Which of two tiers ranks first: the lower `priority` number, and on a tie the lower id. Never the
/// order the competitions happen to be listed in, which a save does not preserve (#635, #1381): this
/// is the one rule every "most prestigious tier" choice in the berth code goes through.
fn tier_rank(priority: u32, id: &str) -> (u32, &str) {
    (priority, id)
}

pub(super) use domestic::{
    apply_domestic_berth_promotion_relegation, resolve_domestic_berth_fields,
};
pub(super) use pyramid::apply_pyramid_promotion_relegation;

pub use continental::{
    berth_qualified_entrants, competition_has_incoming_berths, continental_qualified_entrants,
    resolve_continental_fields,
};
