//! Availability of stored match ratings.

/// Zero denotes an unavailable rating until match ratings are implemented.
pub fn is_rated(rating: f32) -> bool {
    rating.is_finite() && rating > 0.0
}

#[cfg(test)]
mod tests {
    use super::is_rated;

    /// Given stored zero, when availability is checked, then it is unrated.
    #[test]
    fn zero_match_ratings_are_unrated() {
        assert!(!is_rated(0.0));
    }

    /// Given a positive finite match rating, when checked, then it remains available.
    #[test]
    fn positive_finite_match_ratings_are_available() {
        assert!(is_rated(6.8));
    }

    /// Given a malformed rating, when checked, then it is unavailable.
    #[test]
    fn negative_and_non_finite_match_ratings_are_unrated() {
        for value in [-1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(!is_rated(value));
        }
    }
}
