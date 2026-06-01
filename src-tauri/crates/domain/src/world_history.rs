use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct WorldHistoryArchive {
    #[serde(default)]
    pub rivalries: Vec<WorldRivalry>,
    #[serde(default)]
    pub season_awards: Vec<HistoricalSeasonAwardsRecord>,
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
        let Some(rivalry) = WorldRivalry::new(team_a_id.into(), team_b_id.into(), intensity, started_season) else {
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
    use super::{
        HistoricalPlayerAwardWinner, HistoricalSeasonAwardsRecord, WorldHistoryArchive,
    };

    #[test]
    fn world_history_archive_deserializes_missing_fields_to_empty_vectors() {
        let archive: WorldHistoryArchive = serde_json::from_str("{}").unwrap();

        assert!(archive.rivalries.is_empty());
        assert!(archive.season_awards.is_empty());
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