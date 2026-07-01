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

/// The `YYYY-MM-DD` day an article is dated on. Article dates come in two
/// shapes — a bare `YYYY-MM-DD` and an RFC3339 timestamp
/// (`YYYY-MM-DDThh:mm:ss+00:00`) — so compare on the day prefix.
pub fn article_day(date: &str) -> &str {
    date.get(..10).unwrap_or(date)
}

/// Whether an article dated `date` is visible at game-date `today` (formatted
/// `%Y-%m-%d`). Future-dated articles (e.g. the World Cup kickoff, dated at
/// kickoff) stay hidden until their day, so they can't sit permanently atop the
/// feed "every day" until they arrive. Comparing on the day prefix avoids an
/// RFC3339 timestamp sorting *after* the bare day and hiding same-day news.
pub fn article_is_visible(date: &str, today: &str) -> bool {
    article_day(date) <= today
}

pub fn query_news_feed(game: &Game, _query: &NewsFeedQuery) -> NewsFeed {
    // Only surface news that has already happened.
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let articles: Vec<NewsArticle> = game
        .news
        .iter()
        .filter(|article| article_is_visible(&article.date, &today))
        .cloned()
        .collect();

    let referenced_ids: BTreeSet<String> = articles
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
        articles,
        team_names,
        league_name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::manager::Manager;
    use domain::news::NewsCategory;

    fn game_on(date: &str) -> Game {
        let start: chrono::DateTime<chrono::Utc> =
            format!("{date}T00:00:00Z").parse().expect("valid date");
        let clock = crate::clock::GameClock::new(start);
        let manager = Manager::new(
            "mgr".to_string(),
            "Alex".to_string(),
            "Boss".to_string(),
            "1980-01-01".to_string(),
            "EN".to_string(),
        );
        Game::new(clock, manager, vec![], vec![], vec![], vec![])
    }

    fn article(id: &str, date: &str) -> NewsArticle {
        NewsArticle::new(
            id.to_string(),
            "Headline".to_string(),
            "Body".to_string(),
            "Source".to_string(),
            date.to_string(),
            NewsCategory::Editorial,
        )
    }

    #[test]
    fn news_feed_hides_future_dated_articles_until_their_date() {
        let mut game = game_on("2026-02-15");
        game.news = vec![
            article("past", "2026-02-01"),
            article("today", "2026-02-15"),
            article("future", "2026-06-03"),
        ];

        let feed = query_news_feed(&game, &NewsFeedQuery {});
        let ids: Vec<&str> = feed.articles.iter().map(|a| a.id.as_str()).collect();

        assert!(ids.contains(&"past") && ids.contains(&"today"));
        assert!(
            !ids.contains(&"future"),
            "a future-dated article must not appear before its date"
        );

        // Once the clock reaches the article's date, it surfaces.
        let later = game_on("2026-06-03");
        let mut game = later;
        game.news = vec![article("future", "2026-06-03")];
        let feed = query_news_feed(&game, &NewsFeedQuery {});
        assert_eq!(feed.articles.len(), 1);
    }

    #[test]
    fn news_feed_shows_same_day_rfc3339_articles() {
        // Many articles (weekly digests, injuries, transfer rumours) store an
        // RFC3339 timestamp rather than a bare date. The day-of-publication
        // feed must still show them — a naive string compare would hide them
        // because the timestamp sorts after the bare `YYYY-MM-DD` of "today".
        let mut game = game_on("2026-02-15");
        game.news = vec![
            article("digest", "2026-02-15T00:00:00+00:00"),
            article("future", "2026-06-03T12:00:00+00:00"),
        ];

        let feed = query_news_feed(&game, &NewsFeedQuery {});
        let ids: Vec<&str> = feed.articles.iter().map(|a| a.id.as_str()).collect();

        assert!(
            ids.contains(&"digest"),
            "a same-day RFC3339 article must appear on its publication day"
        );
        assert!(
            !ids.contains(&"future"),
            "a future-dated RFC3339 article must still be hidden until its date"
        );
    }
}
