//! What a player did at each club during the season in progress.
//!
//! His season stats are one total for the player. When he changes club mid-season the total
//! carries on, so the rollover would credit all of it to whichever club holds him then. A stint
//! is closed at every move, from what he had done by then, and the rollover credits the club he
//! is with only what he did since he arrived.

use serde::{Deserialize, Serialize};

use crate::player::{CareerEntry, PlayerSeasonStats};

/// Appearances, goals and assists: the three figures a career row carries.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct StintTally {
    pub appearances: u32,
    pub goals: u32,
    pub assists: u32,
}

/// One club's share of the season in progress, kept when he left it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ClosedStint {
    /// Empty for a spell at no club.
    pub team_id: String,
    /// Empty for a spell at no club: the screens word it in the player's language.
    pub team_name: String,
    pub tally: StintTally,
}

/// The season in progress, split by club. Empty for a save written before the split existed, which
/// credits the whole season to the club that holds him at the rollover, as it always did.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SeasonStints {
    /// His season totals when he arrived at the club he is with now. Zero at the start of a season.
    pub stats_at_arrival: StintTally,
    /// The clubs he has left since the season began, oldest first.
    pub closed: Vec<ClosedStint>,
}

impl StintTally {
    fn of(stats: &PlayerSeasonStats) -> Self {
        Self {
            appearances: stats.appearances,
            goals: stats.goals,
            assists: stats.assists,
        }
    }

    /// What `self` adds to `earlier`; a total that fell below it (the stats were reset some other
    /// way) counts as nothing, never wraps.
    fn since(self, earlier: Self) -> Self {
        Self {
            appearances: self.appearances.saturating_sub(earlier.appearances),
            goals: self.goals.saturating_sub(earlier.goals),
            assists: self.assists.saturating_sub(earlier.assists),
        }
    }
}

impl SeasonStints {
    /// He leaves `team_id`: what he did there since he arrived is kept, and the next club's
    /// stint starts from his totals now. A spell with no appearance is not kept.
    pub(crate) fn close(&mut self, stats: &PlayerSeasonStats, team_id: &str, team_name: &str) {
        let now = StintTally::of(stats);
        let tally = now.since(self.stats_at_arrival);
        if tally.appearances > 0 {
            self.closed.push(ClosedStint {
                team_id: team_id.to_string(),
                team_name: team_name.to_string(),
                tally,
            });
        }
        self.stats_at_arrival = now;
    }

    /// The season's career rows: the clubs he left, then the club he is with now for what he did
    /// since he arrived. A club he never appeared for has no row.
    pub fn career_entries(
        &self,
        season: u32,
        stats: &PlayerSeasonStats,
        current_team_id: &str,
        current_team_name: &str,
    ) -> Vec<CareerEntry> {
        let current = StintTally::of(stats).since(self.stats_at_arrival);
        let current_stint = (current.appearances > 0).then(|| ClosedStint {
            team_id: current_team_id.to_string(),
            team_name: current_team_name.to_string(),
            tally: current,
        });
        self.closed
            .iter()
            .cloned()
            .chain(current_stint)
            .map(|stint| CareerEntry {
                season,
                team_id: stint.team_id,
                team_name: stint.team_name,
                appearances: stint.tally.appearances,
                goals: stint.tally.goals,
                assists: stint.tally.assists,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats(appearances: u32, goals: u32, assists: u32) -> PlayerSeasonStats {
        PlayerSeasonStats {
            appearances,
            goals,
            assists,
            ..PlayerSeasonStats::default()
        }
    }

    fn row(entry: &CareerEntry) -> (&str, &str, u32, u32, u32) {
        (
            entry.team_id.as_str(),
            entry.team_name.as_str(),
            entry.appearances,
            entry.goals,
            entry.assists,
        )
    }

    /// Given a player with 10 appearances, 4 goals and 2 assists who leaves his club mid-season
    /// When the season's rows are read with 16 appearances, 6 goals and 3 assists by the end
    /// Then the club he left has the first 10, and the club he is with the six since.
    #[test]
    fn a_move_splits_the_season_between_the_two_clubs() {
        let mut stints = SeasonStints::default();
        stints.close(&stats(10, 4, 2), "seller", "Seller FC");

        let rows = stints.career_entries(2026, &stats(16, 6, 3), "buyer", "Buyer FC");

        assert_eq!(
            rows.iter().map(row).collect::<Vec<_>>(),
            vec![
                ("seller", "Seller FC", 10, 4, 2),
                ("buyer", "Buyer FC", 6, 2, 1)
            ]
        );
        assert!(rows.iter().all(|entry| entry.season == 2026));
    }

    /// Given a player who has played nothing at a club when he leaves it (or was a free agent)
    /// When the season's rows are read
    /// Then that club has no row, and the club he is with has the whole season.
    #[test]
    fn a_spell_without_an_appearance_has_no_row() {
        let mut stints = SeasonStints::default();
        stints.close(&stats(0, 0, 0), "bench", "Bench FC");

        let rows = stints.career_entries(2026, &stats(5, 1, 0), "buyer", "Buyer FC");

        assert_eq!(
            rows.iter().map(row).collect::<Vec<_>>(),
            vec![("buyer", "Buyer FC", 5, 1, 0)]
        );
    }

    /// Given a player who left his club and has not played since
    /// When the season's rows are read
    /// Then only the club he left has a row: nothing is credited to the club that holds him now.
    #[test]
    fn a_player_who_has_not_played_since_arriving_has_no_current_row() {
        let mut stints = SeasonStints::default();
        stints.close(&stats(12, 3, 1), "seller", "Seller FC");

        let rows = stints.career_entries(2026, &stats(12, 3, 1), "", "");

        assert_eq!(
            rows.iter().map(row).collect::<Vec<_>>(),
            vec![("seller", "Seller FC", 12, 3, 1)]
        );
    }

    /// Given a save written before stints existed (no baseline, no closed stints)
    /// When the season's rows are read
    /// Then the whole season goes to the club that holds him, as it did.
    #[test]
    fn a_save_with_no_stints_credits_the_whole_season_to_the_current_club() {
        let stints: SeasonStints = serde_json::from_str("{}").unwrap();

        let rows = stints.career_entries(2026, &stats(30, 20, 10), "club", "Club FC");

        assert_eq!(
            rows.iter().map(row).collect::<Vec<_>>(),
            vec![("club", "Club FC", 30, 20, 10)]
        );
    }

    /// Given totals that sit below the baseline (stats were reset some other way)
    /// When the season's rows are read
    /// Then nothing underflows: the current club's tally is zero.
    #[test]
    fn a_total_below_the_baseline_does_not_underflow() {
        let mut stints = SeasonStints::default();
        stints.close(&stats(10, 4, 2), "seller", "Seller FC");

        let rows = stints.career_entries(2026, &stats(3, 1, 0), "buyer", "Buyer FC");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].team_id, "seller");
    }
}
