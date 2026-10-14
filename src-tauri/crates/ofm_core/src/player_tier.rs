//! The tier a rating puts a player in: the one definition of "elite" the game shares.
//!
//! A tier is read off a 1-99 rating, so it applies alike to a player's current `ovr` and to his
//! `potential` (the ceiling). The cut-offs sit on the measured shape of generated worlds
//! (ovr mean about 68, sd about 6.4): S is about mean + 2.1 sd, A about mean + 1.5 sd, which
//! makes S roughly 1 to 3% of senior players and A roughly 5 to 8%.

/// Lowest rating of an S-tier player.
pub const S_TIER_MIN_RATING: u8 = 81;
/// Lowest rating of an A-tier player.
pub const A_TIER_MIN_RATING: u8 = 77;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerTier {
    S,
    A,
    /// Everyone below A: the game names no lower tier.
    Rest,
}

pub fn player_tier(rating: u8) -> PlayerTier {
    if rating >= S_TIER_MIN_RATING {
        PlayerTier::S
    } else if rating >= A_TIER_MIN_RATING {
        PlayerTier::A
    } else {
        PlayerTier::Rest
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Given the lowest S rating, when it is tiered, then it is S; one less is A.
    #[test]
    fn the_lowest_s_rating_is_s_and_one_less_is_a() {
        assert_eq!(player_tier(S_TIER_MIN_RATING), PlayerTier::S);
        assert_eq!(player_tier(S_TIER_MIN_RATING - 1), PlayerTier::A);
    }

    /// Given the lowest A rating, when it is tiered, then it is A; one less is the rest.
    #[test]
    fn the_lowest_a_rating_is_a_and_one_less_is_the_rest() {
        assert_eq!(player_tier(A_TIER_MIN_RATING), PlayerTier::A);
        assert_eq!(player_tier(A_TIER_MIN_RATING - 1), PlayerTier::Rest);
    }

    /// Given the ends of the 1-99 scale, when they are tiered, then 99 is S and 0 is the rest.
    #[test]
    fn the_ends_of_the_scale_are_tiered() {
        assert_eq!(player_tier(99), PlayerTier::S);
        assert_eq!(player_tier(0), PlayerTier::Rest);
    }
}
