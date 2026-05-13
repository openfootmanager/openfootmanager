use domain::news::{NewsArticle, NewsCategory};
use rusqlite::{Connection, params};

/// Insert or replace a news article row.
pub fn upsert_news(conn: &Connection, article: &NewsArticle) -> Result<(), String> {
    let team_ids_json =
        serde_json::to_string(&article.team_ids).map_err(|e| format!("JSON error: {}", e))?;
    let player_ids_json =
        serde_json::to_string(&article.player_ids).map_err(|e| format!("JSON error: {}", e))?;
    let match_score_json = article
        .match_score
        .as_ref()
        .map(|s| serde_json::to_string(s).unwrap_or_default());
    let category_str = format!("{:?}", article.category);

    let i18n = serde_json::json!({
        "headline_key": article.headline_key,
        "body_key": article.body_key,
        "source_key": article.source_key,
        "i18n_params": article.i18n_params,
    });
    let i18n_json = serde_json::to_string(&i18n).map_err(|e| format!("JSON error: {}", e))?;

    conn.execute(
        "INSERT OR REPLACE INTO news
         (id, headline, body, source, date, category, team_ids, player_ids, match_score, read, i18n)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            article.id,
            article.headline,
            article.body,
            article.source,
            article.date,
            category_str,
            team_ids_json,
            player_ids_json,
            match_score_json,
            article.read as i32,
            i18n_json,
        ],
    )
    .map_err(|e| format!("Failed to upsert news: {}", e))?;
    Ok(())
}

/// Insert or replace multiple news articles.
pub fn upsert_news_list(conn: &Connection, articles: &[NewsArticle]) -> Result<(), String> {
    for a in articles {
        upsert_news(conn, a)?;
    }
    Ok(())
}

fn parse_news_category(s: &str) -> NewsCategory {
    match s {
        "MatchReport" => NewsCategory::MatchReport,
        "LeagueRoundup" => NewsCategory::LeagueRoundup,
        "StandingsUpdate" => NewsCategory::StandingsUpdate,
        "TransferRumour" => NewsCategory::TransferRumour,
        "TransferRoundup" => NewsCategory::TransferRoundup,
        "InjuryNews" => NewsCategory::InjuryNews,
        "ManagerialChange" => NewsCategory::ManagerialChange,
        "SeasonPreview" => NewsCategory::SeasonPreview,
        _ => NewsCategory::Editorial,
    }
}

/// Load all news articles ordered by date descending.
pub fn load_all_news(conn: &Connection) -> Result<Vec<NewsArticle>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, headline, body, source, date, category, team_ids, player_ids, match_score, read, i18n
             FROM news ORDER BY date DESC",
        )
        .map_err(|e| format!("Failed to prepare news query: {}", e))?;

    let rows = stmt
        .query_map([], row_to_news)
        .map_err(|e| format!("Failed to query news: {}", e))?;

    let mut articles = Vec::new();
    for row in rows {
        articles.push(row.map_err(|e| format!("Failed to read news: {}", e))?);
    }
    Ok(articles)
}

fn row_to_news(row: &rusqlite::Row) -> rusqlite::Result<NewsArticle> {
    let category_str: String = row.get(5)?;
    let team_ids_json: String = row.get(6)?;
    let player_ids_json: String = row.get(7)?;
    let match_score_json: Option<String> = row.get(8)?;
    let read_int: i32 = row.get(9)?;
    let i18n_json: String = row.get(10)?;

    let i18n: serde_json::Value = serde_json::from_str(&i18n_json).unwrap_or_default();

    Ok(NewsArticle {
        id: row.get(0)?,
        headline: row.get(1)?,
        body: row.get(2)?,
        source: row.get(3)?,
        date: row.get(4)?,
        category: parse_news_category(&category_str),
        team_ids: serde_json::from_str(&team_ids_json).unwrap_or_default(),
        player_ids: serde_json::from_str(&player_ids_json).unwrap_or_default(),
        match_score: match_score_json.and_then(|j| serde_json::from_str(&j).ok()),
        read: read_int != 0,
        headline_key: i18n
            .get("headline_key")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        body_key: i18n
            .get("body_key")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        source_key: i18n
            .get("source_key")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        i18n_params: i18n
            .get("i18n_params")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_database::GameDatabase;
    use domain::news::NewsMatchScore;

    fn test_db() -> GameDatabase {
        GameDatabase::open_in_memory().unwrap()
    }

    fn sample_article(id: &str) -> NewsArticle {
        NewsArticle::new(
            id.to_string(),
            "Match Report".to_string(),
            "London FC beat Manchester City 2-1.".to_string(),
            "OFM News".to_string(),
            "2026-08-15".to_string(),
            NewsCategory::MatchReport,
        )
    }

    #[test]
    fn test_upsert_and_load_news() {
        let db = test_db();
        let article = sample_article("news-001");

        upsert_news(db.conn(), &article).unwrap();
        let all = load_all_news(db.conn()).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "news-001");
        assert_eq!(all[0].headline, "Match Report");
        assert_eq!(all[0].category, NewsCategory::MatchReport);
    }

    #[test]
    fn test_news_with_match_score() {
        let db = test_db();
        let article = sample_article("news-001").with_score(NewsMatchScore {
            home_team_id: "team-001".to_string(),
            away_team_id: "team-002".to_string(),
            home_goals: 2,
            away_goals: 1,
        });

        upsert_news(db.conn(), &article).unwrap();
        let all = load_all_news(db.conn()).unwrap();
        let score = all[0].match_score.as_ref().unwrap();
        assert_eq!(score.home_goals, 2);
        assert_eq!(score.away_goals, 1);
    }

    #[test]
    fn test_news_batch() {
        let db = test_db();
        let articles = vec![sample_article("news-001"), sample_article("news-002")];

        upsert_news_list(db.conn(), &articles).unwrap();
        let all = load_all_news(db.conn()).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_transfer_roundup_category_roundtrips() {
        let db = test_db();
        let article = NewsArticle::new(
            "news-transfer-roundup".to_string(),
            "Transfer Roundup".to_string(),
            "Big moves landed this week.".to_string(),
            "Transfer Intelligence".to_string(),
            "2026-08-15".to_string(),
            NewsCategory::TransferRoundup,
        );

        upsert_news(db.conn(), &article).unwrap();
        let all = load_all_news(db.conn()).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].category, NewsCategory::TransferRoundup);
    }
}
