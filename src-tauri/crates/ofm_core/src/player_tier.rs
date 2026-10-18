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

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum PlayerTier {
    S,
    A,
    /// Everyone below A: the game names no lower tier.
    Rest,
}

/// The tier a 1-99 rating falls in: `S` from `S_TIER_MIN_RATING` (81) up, `A` from
/// `A_TIER_MIN_RATING` (77) to one below that, and `Rest` for everything lower. Works on `ovr` and
/// on `potential` alike.
pub fn player_tier(rating: u8) -> PlayerTier {
    if rating >= S_TIER_MIN_RATING {
        PlayerTier::S
    } else if rating >= A_TIER_MIN_RATING {
        PlayerTier::A
    } else {
        PlayerTier::Rest
    }
}

/// The tier of every rating from 0 to 99, indexed by rating. The screens colour a rating by
/// looking it up here, so they never hold a cut-off of their own.
pub fn rating_tiers() -> Vec<PlayerTier> {
    (0..=99).map(player_tier).collect()
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

    /// Given the table of every rating's tier,
    /// When it is read at each side of both cut-offs,
    /// Then it covers 0 to 99, and the tier changes exactly at 77 and at 81.
    #[test]
    fn the_rating_table_changes_tier_exactly_at_the_cut_offs() {
        let table = rating_tiers();

        assert_eq!(table.len(), 100);
        assert_eq!(table[76], PlayerTier::Rest);
        assert_eq!(table[77], PlayerTier::A);
        assert_eq!(table[80], PlayerTier::A);
        assert_eq!(table[81], PlayerTier::S);
        assert_eq!(table[99], PlayerTier::S);
        assert_eq!(table[0], PlayerTier::Rest);
    }

    /// Given the ends of the 1-99 scale, when they are tiered, then 99 is S and 0 is the rest.
    #[test]
    fn the_ends_of_the_scale_are_tiered() {
        assert_eq!(player_tier(99), PlayerTier::S);
        assert_eq!(player_tier(0), PlayerTier::Rest);
    }
}
