use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct WorldHistoryArchive {
    #[serde(default)]
    pub rivalries: Vec<WorldRivalry>,
    #[serde(default)]
    pub season_awards: Vec<HistoricalSeasonAwardsRecord>,
    #[serde(default)]
    pub world_cup_champions: Vec<WorldCupChampionRecord>,
    /// FIFA-style national-team ranking points, updated after every
    /// international match. Drives World Cup draw pots and qualifying seeding.
    #[serde(default)]
    pub national_team_ranking: Vec<NationRankingRecord>,
    /// Confirmed World Cup hosts by year (real history plus those the game has
    /// awarded). The host auto-qualifies and is seeded into Pot 1.
    #[serde(default)]
    pub world_cup_hosts: Vec<WorldCupHostRecord>,
}

/// The host nation awarded a future World Cup.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorldCupHostRecord {
    pub year: u32,
    pub nation_code: String,
    pub nation_name: String,
}

/// One nation's place in the world ranking.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NationRankingRecord {
    pub nation_code: String,
    pub points: f64,
}

/// A nation crowned at a World Cup — the game world's highest honour.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorldCupChampionRecord {
    pub year: u32,
    pub nation_code: String,
    pub nation_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorldRivalry {
    pub team_a_id: String,
    pub team_b_id: String,
    pub intensity: u8,
    #[serde(default)]
    pub started_season: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HistoricalSeasonAwardsRecord {
    pub season: u32,
    #[serde(default)]
    pub golden_boot: Option<HistoricalPlayerAwardWinner>,
    #[serde(default)]
    pub assist_king: Option<HistoricalPlayerAwardWinner>,
    #[serde(default)]
    pub player_of_year: Option<HistoricalPlayerAwardWinner>,
    #[serde(default)]
    pub clean_sheet_king: Option<HistoricalPlayerAwardWinner>,
    #[serde(default)]
    pub most_appearances: Option<HistoricalPlayerAwardWinner>,
    #[serde(default)]
    pub young_player: Option<HistoricalPlayerAwardWinner>,
    #[serde(default)]
    pub manager_of_season: Option<HistoricalManagerAwardWinner>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HistoricalPlayerAwardWinner {
    pub player_id: String,
    pub player_name: String,
    pub team_id: String,
    pub team_name: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HistoricalManagerAwardWinner {
    pub manager_id: String,
    pub manager_name: String,
    pub team_id: String,
    pub team_name: String,
    pub value: f64,
    pub win_rate: f64,
}

impl WorldHistoryArchive {
    pub fn upsert_rivalry(
        &mut self,
        team_a_id: impl Into<String>,
        team_b_id: impl Into<String>,
        intensity: u8,
        started_season: Option<u32>,
    ) {
        let Some(rivalry) = WorldRivalry::new(
            team_a_id.into(),
            team_b_id.into(),
            intensity,
            started_season,
        ) else {
            return;
        };

        if let Some(existing) = self.rivalries.iter_mut().find(|existing| {
            existing.team_a_id == rivalry.team_a_id && existing.team_b_id == rivalry.team_b_id
        }) {
            *existing = rivalry;
        } else {
            self.rivalries.push(rivalry);
        }

        self.rivalries.sort_by(|left, right| {
            left.team_a_id
                .cmp(&right.team_a_id)
                .then_with(|| left.team_b_id.cmp(&right.team_b_id))
        });
    }

    pub fn record_season_awards(&mut self, record: HistoricalSeasonAwardsRecord) {
        if let Some(existing) = self
            .season_awards
            .iter_mut()
            .find(|existing| existing.season == record.season)
        {
            *existing = record;
        } else {
            self.season_awards.push(record);
        }

        self.season_awards
            .sort_by(|left, right| right.season.cmp(&left.season));
    }

    pub fn record_world_cup_champion(&mut self, record: WorldCupChampionRecord) {
        if let Some(existing) = self
            .world_cup_champions
            .iter_mut()
            .find(|existing| existing.year == record.year)
        {
            *existing = record;
        } else {
            self.world_cup_champions.push(record);
        }

        self.world_cup_champions
            .sort_by(|left, right| right.year.cmp(&left.year));
    }

    /// Current ranking points for a nation, if it has been ranked.
    pub fn ranking_points(&self, nation_code: &str) -> Option<f64> {
        self.national_team_ranking
            .iter()
            .find(|record| record.nation_code == nation_code)
            .map(|record| record.points)
    }

    /// Set a nation's ranking points (insert or replace).
    pub fn set_ranking_points(&mut self, nation_code: &str, points: f64) {
        if let Some(record) = self
            .national_team_ranking
            .iter_mut()
            .find(|record| record.nation_code == nation_code)
        {
            record.points = points;
        } else {
            self.national_team_ranking.push(NationRankingRecord {
                nation_code: nation_code.to_string(),
                points,
            });
        }
    }

    /// Seed a nation's ranking only if it has none yet (e.g. from squad
    /// strength when a World Cup is first scheduled).
    pub fn seed_ranking(&mut self, nation_code: &str, points: f64) {
        if self.ranking_points(nation_code).is_none() {
            self.set_ranking_points(nation_code, points);
        }
    }

    /// Update both nations' ranking points from one international result with a
    /// zero-sum Elo step. A knockout decided on penalties counts as a draw for
    /// ranking purposes (as in the real FIFA ranking). Unranked nations start
    /// from a neutral base.
    pub fn apply_national_result(
        &mut self,
        home_code: &str,
        away_code: &str,
        home_goals: u8,
        away_goals: u8,
        decided_on_penalties: bool,
    ) {
        const BASE: f64 = 1000.0;
        const K: f64 = 30.0;
        if home_code.is_empty() || away_code.is_empty() || home_code == away_code {
            return;
        }
        let home_points = self.ranking_points(home_code).unwrap_or(BASE);
        let away_points = self.ranking_points(away_code).unwrap_or(BASE);
        let expected_home = 1.0 / (1.0 + 10_f64.powf((away_points - home_points) / 400.0));
        let actual_home = if decided_on_penalties || home_goals == away_goals {
            0.5
        } else if home_goals > away_goals {
            1.0
        } else {
            0.0
        };
        let margin = 1.0 + ((f64::from(home_goals) - f64::from(away_goals)).abs() - 1.0).max(0.0) * 0.2;
        let delta = K * margin * (actual_home - expected_home);
        self.set_ranking_points(home_code, home_points + delta);
        self.set_ranking_points(away_code, away_points - delta);
    }

    /// Record (or replace) the host awarded a World Cup year.
    pub fn record_world_cup_host(&mut self, record: WorldCupHostRecord) {
        if let Some(existing) = self
            .world_cup_hosts
            .iter_mut()
            .find(|existing| existing.year == record.year)
        {
            *existing = record;
        } else {
            self.world_cup_hosts.push(record);
        }
        self.world_cup_hosts
            .sort_by(|left, right| left.year.cmp(&right.year));
    }

    /// The confirmed host nation code for a World Cup year, if any.
    pub fn world_cup_host(&self, year: u32) -> Option<&str> {
        self.world_cup_hosts
            .iter()
            .find(|record| record.year == year)
            .map(|record| record.nation_code.as_str())
    }

    /// Nation codes ordered by ranking points, strongest first.
    pub fn ranked_nation_codes(&self) -> Vec<String> {
        let mut ranked = self.national_team_ranking.clone();
        ranked.sort_by(|left, right| {
            right
                .points
                .partial_cmp(&left.points)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.nation_code.cmp(&right.nation_code))
        });
        ranked
            .into_iter()
            .map(|record| record.nation_code)
            .collect()
    }
}

impl WorldRivalry {
    pub fn new(
        team_a_id: String,
        team_b_id: String,
        intensity: u8,
        started_season: Option<u32>,
    ) -> Option<Self> {
        if team_a_id.is_empty() || team_b_id.is_empty() || team_a_id == team_b_id {
            return None;
        }

        let (team_a_id, team_b_id) = if team_a_id <= team_b_id {
            (team_a_id, team_b_id)
        } else {
            (team_b_id, team_a_id)
        };

        Some(Self {
            team_a_id,
            team_b_id,
            intensity: intensity.min(100),
            started_season,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{HistoricalPlayerAwardWinner, HistoricalSeasonAwardsRecord, WorldHistoryArchive};

    #[test]
    fn world_history_archive_deserializes_missing_fields_to_empty_vectors() {
        let archive: WorldHistoryArchive = serde_json::from_str("{}").unwrap();

        assert!(archive.rivalries.is_empty());
        assert!(archive.season_awards.is_empty());
    }

    #[test]
    fn ranking_rewards_winners_and_seeds_only_when_absent() {
        let mut archive = WorldHistoryArchive::default();
        archive.seed_ranking("BR", 1500.0);
        archive.seed_ranking("BR", 9999.0); // already ranked → no-op
        assert_eq!(archive.ranking_points("BR"), Some(1500.0));

        archive.seed_ranking("AR", 1500.0);
        archive.apply_national_result("BR", "AR", 2, 0, false);
        assert!(archive.ranking_points("BR").unwrap() > 1500.0);
        assert!(archive.ranking_points("AR").unwrap() < 1500.0);
        // Elo is zero-sum.
        let total = archive.ranking_points("BR").unwrap() + archive.ranking_points("AR").unwrap();
        assert!((total - 3000.0).abs() < 1e-6);

        assert_eq!(
            archive.ranked_nation_codes().first().map(String::as_str),
            Some("BR")
        );
    }

    #[test]
    fn penalty_win_counts_as_a_draw_for_ranking() {
        let mut archive = WorldHistoryArchive::default();
        archive.seed_ranking("BR", 1500.0);
        archive.seed_ranking("AR", 1500.0);
        // Equal points + a shootout (treated as a draw) → no movement.
        archive.apply_national_result("BR", "AR", 1, 1, true);
        assert_eq!(archive.ranking_points("BR"), Some(1500.0));
        assert_eq!(archive.ranking_points("AR"), Some(1500.0));
    }

    #[test]
    fn upsert_rivalry_normalizes_pair_and_replaces_existing_entry() {
        let mut archive = WorldHistoryArchive::default();
        archive.upsert_rivalry("team-b", "team-a", 81, Some(2024));
        archive.upsert_rivalry("team-a", "team-b", 95, Some(2026));

        assert_eq!(archive.rivalries.len(), 1);
        assert_eq!(archive.rivalries[0].team_a_id, "team-a");
        assert_eq!(archive.rivalries[0].team_b_id, "team-b");
        assert_eq!(archive.rivalries[0].intensity, 95);
        assert_eq!(archive.rivalries[0].started_season, Some(2026));
    }

    #[test]
    fn record_season_awards_replaces_existing_season_and_keeps_latest_first() {
        let mut archive = WorldHistoryArchive::default();
        archive.record_season_awards(HistoricalSeasonAwardsRecord {
            season: 2025,
            golden_boot: Some(HistoricalPlayerAwardWinner {
                player_id: "player-1".to_string(),
                player_name: "First Winner".to_string(),
                team_id: "team-1".to_string(),
                team_name: "Alpha FC".to_string(),
                value: 20.0,
            }),
            assist_king: None,
            player_of_year: None,
            clean_sheet_king: None,
            most_appearances: None,
            young_player: None,
            manager_of_season: None,
        });
        archive.record_season_awards(HistoricalSeasonAwardsRecord {
            season: 2026,
            golden_boot: None,
            assist_king: None,
            player_of_year: None,
            clean_sheet_king: None,
            most_appearances: None,
            young_player: None,
            manager_of_season: None,
        });
        archive.record_season_awards(HistoricalSeasonAwardsRecord {
            season: 2025,
            golden_boot: Some(HistoricalPlayerAwardWinner {
                player_id: "player-2".to_string(),
                player_name: "Replacement Winner".to_string(),
                team_id: "team-2".to_string(),
                team_name: "Beta FC".to_string(),
                value: 24.0,
            }),
            assist_king: None,
            player_of_year: None,
            clean_sheet_king: None,
            most_appearances: None,
            young_player: None,
            manager_of_season: None,
        });

        assert_eq!(archive.season_awards.len(), 2);
        assert_eq!(archive.season_awards[0].season, 2026);
        assert_eq!(archive.season_awards[1].season, 2025);
        assert_eq!(
            archive.season_awards[1]
                .golden_boot
                .as_ref()
                .map(|winner| winner.player_name.as_str()),
            Some("Replacement Winner")
        );
    }
}
