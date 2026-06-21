//! Promotion and relegation between the divisions of a domestic pyramid.
//!
//! A pyramid is an ordered set of league competitions (highest tier first).
//! After a season, the bottom clubs of each division swap with the top clubs of
//! the division below it. Only `participant_ids` are updated here; fixtures are
//! regenerated separately for the new season.

use std::collections::HashSet;

use domain::league::League;

/// Number of clubs that swap between two adjacent divisions, scaled to the
/// smaller division (roughly one slot per five clubs, at least one). A 20-club
/// division yields four; smaller leagues move fewer.
pub fn relegation_count(top_size: usize, bottom_size: usize) -> usize {
    (top_size.min(bottom_size) / 5).max(1)
}

/// Apply promotion/relegation across `divisions`, ordered highest tier first.
/// Swaps are computed from each division's final standings before any club
/// moves, so multi-tier pyramids resolve consistently.
pub fn apply_promotion_relegation(divisions: &mut [League]) {
    let tiers = divisions.len();
    if tiers < 2 {
        return;
    }

    // For each boundary i / i+1, the clubs leaving downward and upward.
    let mut relegated_at: Vec<Vec<String>> = vec![Vec::new(); tiers];
    let mut promoted_at: Vec<Vec<String>> = vec![Vec::new(); tiers];

    for i in 0..tiers - 1 {
        let upper = divisions[i].sorted_standings();
        let lower = divisions[i + 1].sorted_standings();
        if upper.is_empty() || lower.is_empty() {
            continue;
        }
        let count = relegation_count(
            divisions[i].participant_ids.len(),
            divisions[i + 1].participant_ids.len(),
        );
        relegated_at[i] = upper
            .iter()
            .rev()
            .take(count)
            .map(|entry| entry.team_id.clone())
            .collect();
        promoted_at[i] = lower
            .iter()
            .take(count)
            .map(|entry| entry.team_id.clone())
            .collect();
    }

    for i in 0..tiers {
        let mut leaving: HashSet<&String> = HashSet::new();
        leaving.extend(relegated_at[i].iter()); // relegated to the division below
        if i >= 1 {
            leaving.extend(promoted_at[i - 1].iter()); // promoted to the division above
        }

        let mut new_participants: Vec<String> = divisions[i]
            .participant_ids
            .iter()
            .filter(|id| !leaving.contains(id))
            .cloned()
            .collect();

        if i >= 1 {
            new_participants.extend(relegated_at[i - 1].iter().cloned()); // arrivals from above
        }
        if i < tiers - 1 {
            new_participants.extend(promoted_at[i].iter().cloned()); // arrivals from below
        }

        divisions[i].participant_ids = new_participants;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::league::{League, StandingEntry};

    fn division(id: &str, priority: u32, standings: &[(&str, u32)]) -> League {
        let team_ids: Vec<String> = standings.iter().map(|(id, _)| id.to_string()).collect();
        let mut league = League::new(id.to_string(), id.to_string(), 2026, &team_ids);
        league.priority = priority;
        league.standings = standings
            .iter()
            .map(|(team, points)| {
                let mut entry = StandingEntry::new(team.to_string());
                entry.points = *points;
                entry
            })
            .collect();
        league
    }

    #[test]
    fn relegation_count_scales_with_division_size() {
        assert_eq!(relegation_count(20, 20), 4);
        assert_eq!(relegation_count(10, 10), 2);
        assert_eq!(relegation_count(8, 8), 1);
        assert_eq!(relegation_count(4, 4), 1);
        // Bound by the smaller division.
        assert_eq!(relegation_count(20, 6), 1);
    }

    #[test]
    fn apply_promotion_relegation_swaps_bottom_and_top_clubs() {
        // Top division: t1 best ... t6 worst. Second division: s1 best ... s6 worst.
        let mut divisions = vec![
            division(
                "top",
                0,
                &[
                    ("t1", 60),
                    ("t2", 50),
                    ("t3", 40),
                    ("t4", 30),
                    ("t5", 20),
                    ("t6", 10),
                ],
            ),
            division(
                "second",
                1,
                &[
                    ("s1", 60),
                    ("s2", 50),
                    ("s3", 40),
                    ("s4", 30),
                    ("s5", 20),
                    ("s6", 10),
                ],
            ),
        ];

        // 6-club divisions -> one up / one down.
        apply_promotion_relegation(&mut divisions);

        let top: HashSet<&String> = divisions[0].participant_ids.iter().collect();
        let second: HashSet<&String> = divisions[1].participant_ids.iter().collect();

        assert!(!top.contains(&"t6".to_string()), "worst top club relegated");
        assert!(top.contains(&"s1".to_string()), "best second club promoted");
        assert!(second.contains(&"t6".to_string()));
        assert!(!second.contains(&"s1".to_string()));
        // Sizes preserved.
        assert_eq!(divisions[0].participant_ids.len(), 6);
        assert_eq!(divisions[1].participant_ids.len(), 6);
    }

    #[test]
    fn apply_promotion_relegation_is_noop_for_single_division() {
        let mut divisions = vec![division("only", 0, &[("a", 10), ("b", 5)])];
        apply_promotion_relegation(&mut divisions);
        assert_eq!(
            divisions[0].participant_ids,
            vec!["a".to_string(), "b".to_string()]
        );
    }

    #[test]
    fn apply_promotion_relegation_skips_boundary_without_standings() {
        let mut top = division("top", 0, &[("t1", 30), ("t2", 10)]);
        top.standings.clear(); // never simulated
        let mut divisions = vec![top, division("second", 1, &[("s1", 30), ("s2", 10)])];

        apply_promotion_relegation(&mut divisions);

        // No standings to rank the top division, so nothing moves.
        assert_eq!(
            divisions[0].participant_ids,
            vec!["t1".to_string(), "t2".to_string()]
        );
        assert_eq!(
            divisions[1].participant_ids,
            vec!["s1".to_string(), "s2".to_string()]
        );
    }
}
