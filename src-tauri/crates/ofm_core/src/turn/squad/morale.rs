//! What a player's mood does to his match rating in a club match. Domain morale (0 to 100, 50 the
//! middle) becomes the factor `engine::PlayerData::morale_factor` that the engine applies to his
//! rating, in both engines, wherever it applies his familiarity.

/// The most a player's mood moves his match rating either way: at morale 100, or at 0. Judged,
/// not measured: the literature on morale and performance is mostly self-report, so the size is
/// kept small (a role moves a duel by 3 to 10%; a half-time talk, at most 12 morale points, moves
/// a rating by at most 0.72%). The engine keeps its own, wider bound (`MORALE_FACTOR_RANGE`).
const EXTREME_SHARE: f64 = 0.03;

/// The factor on a rating at `morale` (0 to 100): 1.0 at 50, rising to `1 + EXTREME_SHARE` at 100
/// and falling to `1 - EXTREME_SHARE` at 0, in a straight line. The same function builds the
/// factor at kick-off and refits it after the half-time talk.
pub(crate) fn morale_factor(morale: u8) -> f64 {
    1.0 + EXTREME_SHARE * (f64::from(morale.min(100)) - 50.0) / 50.0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Given a player at the middle of the scale
    /// When his factor is built
    /// Then it is exactly neutral, so a world at morale 50 plays as it did before morale counted
    #[test]
    fn a_player_at_morale_fifty_is_exactly_neutral() {
        assert_eq!(morale_factor(50), 1.0);
    }

    /// Given the two ends of the scale
    /// When their factors are built
    /// Then they are the 3% either way the size was approved at, and the middle of each half is half
    #[test]
    fn the_ends_of_the_scale_are_three_percent_either_way() {
        assert!((morale_factor(100) - 1.03).abs() < 1e-12);
        assert!((morale_factor(0) - 0.97).abs() < 1e-12);
        assert!((morale_factor(75) - 1.015).abs() < 1e-12);
        assert!((morale_factor(25) - 0.985).abs() < 1e-12);
    }

    /// Given morale that is out of range (it should not be, but a save can hold anything)
    /// When the factor is built
    /// Then it is the factor of the top of the scale, not beyond it
    #[test]
    fn morale_above_the_scale_counts_as_the_top_of_it() {
        assert_eq!(morale_factor(255), morale_factor(100));
    }

    /// Given a better mood
    /// When the factors are compared
    /// Then the happier player's is higher at every step
    #[test]
    fn a_happier_player_always_has_the_higher_factor() {
        for morale in 0..100u8 {
            assert!(morale_factor(morale) < morale_factor(morale + 1));
        }
    }
}
