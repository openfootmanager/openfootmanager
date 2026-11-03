//! Availability of stored match ratings, and how they feed a season average.

use domain::player::PlayerSeasonStats;

/// Zero is the engine's mark for a player it did not rate: unused, or on the
/// pitch for fewer than `engine::rating::MIN_RATED_MINUTES`.
pub fn is_rated(rating: f32) -> bool {
    rating.is_finite() && rating > 0.0
}

/// Averages over rated matches only, so appearances that were never rated
/// (saves from before ratings, short cameos) cannot dilute the mean.
pub fn record_rating(stats: &mut PlayerSeasonStats, rating: f32) {
    if !is_rated(rating) {
        return;
    }
    let rated = stats.rated_appearances as f32;
    stats.avg_rating = (stats.avg_rating * rated + rating) / (rated + 1.0);
    stats.rated_appearances += 1;
}

#[cfg(test)]
mod tests {
    use super::{is_rated, record_rating};
    use domain::player::PlayerSeasonStats;

    /// Given no rated match yet, when one is recorded, then it is the average.
    #[test]
    fn the_first_rated_match_is_the_average() {
        let mut stats = PlayerSeasonStats::default();
        record_rating(&mut stats, 7.2);
        assert_eq!(stats.avg_rating, 7.2);
        assert_eq!(stats.rated_appearances, 1);
    }

    /// Given two rated matches, when a third is recorded, then the average is their mean.
    #[test]
    fn later_rated_matches_form_a_running_mean() {
        let mut stats = PlayerSeasonStats::default();
        for rating in [6.0, 8.0, 7.0] {
            record_rating(&mut stats, rating);
        }
        assert!((stats.avg_rating - 7.0).abs() < 1e-5);
        assert_eq!(stats.rated_appearances, 3);
    }

    /// Given appearances that were never rated, when the first rating arrives, then
    /// those appearances do not drag the average down.
    #[test]
    fn unrated_history_does_not_dilute_the_average() {
        let mut stats = PlayerSeasonStats {
            appearances: 30,
            ..Default::default()
        };
        record_rating(&mut stats, 7.0);
        record_rating(&mut stats, 6.0);
        assert!((stats.avg_rating - 6.5).abs() < 1e-5);
    }

    /// Given an unrated match, when recorded, then the average and its count are untouched.
    #[test]
    fn an_unrated_match_changes_nothing() {
        let mut stats = PlayerSeasonStats::default();
        record_rating(&mut stats, 6.8);
        record_rating(&mut stats, 0.0);
        record_rating(&mut stats, f32::NAN);
        assert_eq!(stats.avg_rating, 6.8);
        assert_eq!(stats.rated_appearances, 1);
    }

    /// Given a save from before ratings, when the stats load, then no rating exists yet.
    #[test]
    fn stats_saved_before_ratings_load_as_unrated() {
        let stats: PlayerSeasonStats =
            serde_json::from_str(r#"{"appearances":12,"avg_rating":0.0}"#).unwrap();
        assert_eq!(stats.rated_appearances, 0);
        assert!(!is_rated(stats.avg_rating));
    }

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
