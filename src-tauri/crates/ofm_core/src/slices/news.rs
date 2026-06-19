use crate::game::Game;
use domain::news::NewsArticle;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Deserialize)]
pub struct NewsFeedQuery {}

#[derive(Debug, Serialize)]
pub struct NewsFeed {
    pub articles: Vec<NewsArticle>,
    /// Team names keyed by ID — only includes teams referenced in articles.
    pub team_names: BTreeMap<String, String>,
    /// Name of the manager's primary competition, used by AwardsCeremonyScreen.
    pub league_name: Option<String>,
}

pub fn query_news_feed(game: &Game, _query: &NewsFeedQuery) -> NewsFeed {
    let referenced_ids: BTreeSet<String> = game
        .news
        .iter()
        .flat_map(|a| {
            let mut ids = a.team_ids.clone();
            if let Some(score) = &a.match_score {
                ids.push(score.home_team_id.clone());
                ids.push(score.away_team_id.clone());
            }
            ids
        })
        .collect();

    let team_names: BTreeMap<String, String> = game
        .teams
        .iter()
        .filter(|t| referenced_ids.contains(&t.id))
        .map(|t| (t.id.clone(), t.name.clone()))
        .collect();

    let league_name = game
        .manager
        .team_id
        .as_deref()
        .and_then(|team_id| {
            game.competitions
                .iter()
                .find(|c| c.participant_ids.iter().any(|id| id == team_id))
        })
        .map(|c| c.name.clone());

    NewsFeed {
        articles: game.news.clone(),
        team_names,
        league_name,
    }
}
