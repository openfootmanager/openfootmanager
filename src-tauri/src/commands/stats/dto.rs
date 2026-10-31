use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlayerAdvancedMetricDto {
    pub total: u32,
    pub per90: Option<f32>,
    pub percentile: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlayerAdvancedPassMetricDto {
    pub completed: u32,
    pub attempted: u32,
    pub accuracy: Option<f32>,
    pub percentile: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlayerStatsOverviewMetricsDto {
    pub shots: PlayerAdvancedMetricDto,
    pub shots_on_target: PlayerAdvancedMetricDto,
    pub passes: PlayerAdvancedPassMetricDto,
    pub tackles_won: PlayerAdvancedMetricDto,
    pub interceptions: PlayerAdvancedMetricDto,
    pub fouls_committed: PlayerAdvancedMetricDto,
}

/// The league and position a player's percentiles are measured within.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PeerGroupDto {
    pub competition_id: String,
    pub competition_name: String,
    pub competition_name_key: Option<String>,
    pub country_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub division_tier: Option<u32>,
    /// The natural position the peers share, as `Position`'s name (e.g. `Striker`).
    pub position: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlayerStatsOverviewDto {
    pub percentile_eligible: bool,
    /// Who the percentiles compare the player with; `None` when the player has no league.
    pub peer_group: Option<PeerGroupDto>,
    pub season_rating_rated: bool,
    pub metrics: PlayerStatsOverviewMetricsDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayerMatchHistoryEntryDto {
    pub fixture_id: String,
    pub date: String,
    pub competition: String,
    pub matchday: u32,
    pub opponent_team_id: String,
    pub opponent_name: String,
    pub team_goals: u8,
    pub opponent_goals: u8,
    pub minutes_played: u8,
    pub goals: u8,
    pub assists: u8,
    pub shots: u8,
    pub shots_on_target: u8,
    pub passes_completed: u8,
    pub passes_attempted: u8,
    pub tackles_won: u8,
    pub interceptions: u8,
    pub fouls_committed: u8,
    pub yellow_cards: u8,
    pub red_cards: u8,
    pub rating: f32,
    pub rated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TeamMatchHistoryEntryDto {
    pub fixture_id: String,
    pub date: String,
    pub competition: String,
    pub matchday: u32,
    pub opponent_team_id: String,
    pub opponent_name: String,
    pub goals_for: u8,
    pub goals_against: u8,
    pub possession_pct: u8,
    pub shots: u16,
    pub shots_on_target: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TeamAdvancedMetricDto {
    pub total: u32,
    pub per_match: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TeamAdvancedPassMetricDto {
    pub completed: u32,
    pub attempted: u32,
    pub accuracy: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TeamStatsOverviewMetricsDto {
    pub shots: TeamAdvancedMetricDto,
    pub shots_on_target: TeamAdvancedMetricDto,
    pub passes: TeamAdvancedPassMetricDto,
    pub tackles_won: TeamAdvancedMetricDto,
    pub interceptions: TeamAdvancedMetricDto,
    pub fouls_committed: TeamAdvancedMetricDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TeamStatsOverviewDto {
    pub matches_played: u32,
    pub goals_for: u32,
    pub goals_against: u32,
    pub goal_difference: i32,
    pub possession_average: Option<f32>,
    pub metrics: TeamStatsOverviewMetricsDto,
}

#[cfg(test)]
mod tests {
    use super::PeerGroupDto;
    use serde_json::json;

    #[test]
    fn peer_group_serializes_division_metadata_for_frontend_display() {
        let peer_group = PeerGroupDto {
            competition_id: "arg-tier-3".to_string(),
            competition_name: "Argentina League".to_string(),
            competition_name_key: Some("tournaments.competitions.tierDivision".to_string()),
            country_id: Some("ARG".to_string()),
            division_tier: Some(3),
            position: "Striker".to_string(),
        };

        assert_eq!(
            serde_json::to_value(peer_group).unwrap(),
            json!({
                "competitionId": "arg-tier-3",
                "competitionName": "Argentina League",
                "competitionNameKey": "tournaments.competitions.tierDivision",
                "countryId": "ARG",
                "divisionTier": 3,
                "position": "Striker"
            })
        );
    }
}
