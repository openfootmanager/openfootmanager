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
        let upper = ranked_participants(&divisions[i]);
        let lower = ranked_participants(&divisions[i + 1]);
        if upper.is_empty() || lower.is_empty() {
            continue;
        }
        // A club already promoted out of this division cannot also be
        // relegated out of it. A table shorter than the two swap groups ranks
        // the same club as both its best and its worst, and it would then
        // arrive in the division above *and* the one below — six clubs holding
        // seven registrations.
        let rising: HashSet<&str> = if i >= 1 {
            promoted_at[i - 1].iter().map(String::as_str).collect()
        } else {
            HashSet::new()
        };
        let falling: Vec<&String> = upper
            .iter()
            .filter(|club| !rising.contains(club.as_str()))
            .collect();
        if falling.is_empty() {
            continue;
        }

        // Never move more clubs than either division has finishers to move.
        // The count is derived from the rosters, so a division whose table is
        // short would otherwise relegate its whole field and take up more than
        // it sent down, leaving the two divisions permanently different sizes.
        let count = relegation_count(
            divisions[i].participant_ids.len(),
            divisions[i + 1].participant_ids.len(),
        )
        .min(falling.len())
        .min(lower.len());
        relegated_at[i] = falling
            .iter()
            .rev()
            .take(count)
            .map(|club| (*club).clone())
            .collect();
        promoted_at[i] = lower.iter().take(count).cloned().collect();
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

/// The division's finishing order, restricted to the clubs still registered
/// with it.
///
/// A standing is the record of a season played; `participant_ids` is who is
/// registered now. Production keeps them in step — `regenerate_league_for_season`
/// rebuilds the table from the roster — but they are separate fields, and
/// promoting from the table while relegating from the roster lets a row that
/// outlived its registration put a club into a division it never played in.
/// The berth pass already filters to registered clubs before it moves anyone;
/// this is the same rule on the other route.
fn ranked_participants(division: &League) -> Vec<String> {
    let registered: HashSet<&str> = division
        .participant_ids
        .iter()
        .map(String::as_str)
        .collect();
    division
        .sorted_standings()
        .into_iter()
        .map(|entry| entry.team_id)
        .filter(|club| registered.contains(club.as_str()))
        .collect()
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

    /// A middle tier whose table ranks fewer clubs than the two swap groups
    /// names the same club as both its champion and its worst finisher. It was
    /// then promoted into the division above and relegated into the one below
    /// at the same time, so six clubs ended up holding seven registrations.
    #[test]
    fn a_short_middle_table_never_sends_one_club_both_ways() {
        let mut top = division("top", 0, &[("t1", 30), ("t2", 20)]);
        let mut middle = division("middle", 1, &[("m1", 30), ("m2", 20)]);
        let mut bottom = division("bottom", 2, &[("b1", 30), ("b2", 20)]);
        top.participant_ids = vec!["t1".to_string(), "t2".to_string()];
        middle.participant_ids = vec!["m1".to_string(), "m2".to_string()];
        bottom.participant_ids = vec!["b1".to_string(), "b2".to_string()];
        // Only one club of the middle tier finished the season on the table,
        // so it is simultaneously the best and the worst.
        middle.standings.truncate(1);

        let mut divisions = vec![top, middle, bottom];
        apply_promotion_relegation(&mut divisions);

        let registrations: Vec<&String> = divisions
            .iter()
            .flat_map(|division| division.participant_ids.iter())
            .collect();
        let unique: HashSet<&String> = registrations.iter().copied().collect();
        assert_eq!(
            registrations.len(),
            unique.len(),
            "a club holds two registrations: {:?}",
            divisions
                .iter()
                .map(|division| division.participant_ids.clone())
                .collect::<Vec<_>>()
        );
        assert_eq!(registrations.len(), 6, "six clubs, six registrations");
        for division in &divisions {
            assert_eq!(division.participant_ids.len(), 2, "sizes must hold");
        }
    }

    /// A standings row that outlived its registration must not promote a club
    /// into a division it never played in — the tier above would gain a club
    /// from nowhere and the two divisions would end up different sizes.
    #[test]
    fn a_standing_without_a_registration_moves_nobody() {
        let mut top = division("top", 0, &[("t1", 30), ("t2", 20)]);
        let mut bottom = division("bottom", 1, &[("b1", 30), ("b2", 20)]);
        // `ghost` finished second in the lower division but is no longer on its
        // books: a save edited by hand, or a roster rewritten under a table.
        bottom
            .standings
            .push(StandingEntry::new("ghost".to_string()));
        bottom.standings[2].points = 99;
        top.participant_ids = vec!["t1".to_string(), "t2".to_string()];
        bottom.participant_ids = vec!["b1".to_string(), "b2".to_string()];

        let mut divisions = vec![top, bottom];
        apply_promotion_relegation(&mut divisions);

        assert!(
            !divisions[0].participant_ids.contains(&"ghost".to_string()),
            "an unregistered club cannot be promoted: {:?}",
            divisions[0].participant_ids
        );
        assert_eq!(divisions[0].participant_ids.len(), 2, "sizes must hold");
        assert_eq!(divisions[1].participant_ids.len(), 2, "sizes must hold");
    }

    /// A short table must not relegate the whole division. The swap count comes
    /// from the rosters, so without a cap a division with three finishers and
    /// twenty clubs sent all three down and took four up.
    #[test]
    fn a_short_table_never_moves_more_clubs_than_it_ranks() {
        let clubs: Vec<(String, u32)> = (1..=20)
            .map(|index| (format!("t{index}"), (30 - index) as u32))
            .collect();
        let rows: Vec<(&str, u32)> = clubs
            .iter()
            .map(|(id, points)| (id.as_str(), *points))
            .collect();
        let mut top = division("top", 0, &rows);
        let mut bottom = division(
            "bottom",
            1,
            &rows
                .iter()
                .map(|(id, points)| (*id, *points))
                .collect::<Vec<_>>(),
        );
        // Rename the lower division's clubs so the two are disjoint, then keep
        // only three finishers on its table.
        bottom.participant_ids = (1..=20).map(|index| format!("b{index}")).collect();
        bottom.standings = (1..=3)
            .map(|index| {
                let mut entry = StandingEntry::new(format!("b{index}"));
                entry.points = (30 - index) as u32;
                entry
            })
            .collect();
        top.participant_ids = (1..=20).map(|index| format!("t{index}")).collect();

        let mut divisions = vec![top, bottom];
        apply_promotion_relegation(&mut divisions);

        assert_eq!(
            divisions[0].participant_ids.len(),
            20,
            "top division changed size: {:?}",
            divisions[0].participant_ids
        );
        assert_eq!(
            divisions[1].participant_ids.len(),
            20,
            "lower division changed size: {:?}",
            divisions[1].participant_ids
        );
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
