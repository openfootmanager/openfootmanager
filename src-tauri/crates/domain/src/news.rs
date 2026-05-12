use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NewsCategory {
    MatchReport,
    LeagueRoundup,
    StandingsUpdate,
    TransferRumour,
    TransferRoundup,
    InjuryNews,
    ManagerialChange,
    SeasonPreview,
    Editorial,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsArticle {
    pub id: String,
    pub headline: String,
    pub body: String,
    pub source: String,
    pub date: String,
    pub category: NewsCategory,
    /// IDs of teams referenced in the article
    pub team_ids: Vec<String>,
    /// IDs of players referenced in the article
    pub player_ids: Vec<String>,
    /// Optional match score context
    pub match_score: Option<NewsMatchScore>,
    pub read: bool,
    /// Optional i18n key for the headline (frontend resolves via t())
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headline_key: Option<String>,
    /// Optional i18n key for the body (frontend resolves via t())
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_key: Option<String>,
    /// Optional i18n key for the source (frontend resolves via t())
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_key: Option<String>,
    /// Interpolation parameters for the i18n keys
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub i18n_params: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsMatchScore {
    pub home_team_id: String,
    pub away_team_id: String,
    pub home_goals: u8,
    pub away_goals: u8,
}

impl NewsArticle {
    pub fn new(
        id: String,
        headline: String,
        body: String,
        source: String,
        date: String,
        category: NewsCategory,
    ) -> Self {
        Self {
            id,
            headline,
            body,
            source,
            date,
            category,
            team_ids: vec![],
            player_ids: vec![],
            match_score: None,
            read: false,
            headline_key: None,
            body_key: None,
            source_key: None,
            i18n_params: HashMap::new(),
        }
    }

    pub fn with_teams(mut self, ids: Vec<String>) -> Self {
        self.team_ids = ids;
        self
    }

    pub fn with_players(mut self, ids: Vec<String>) -> Self {
        self.player_ids = ids;
        self
    }

    pub fn with_score(mut self, score: NewsMatchScore) -> Self {
        self.match_score = Some(score);
        self
    }

    pub fn with_i18n(
        mut self,
        headline_key: &str,
        body_key: &str,
        source_key: &str,
        params: HashMap<String, String>,
    ) -> Self {
        self.headline_key = Some(headline_key.to_string());
        self.body_key = Some(body_key.to_string());
        self.source_key = Some(source_key.to_string());
        self.i18n_params = params;
        self
    }
}
