use domain::news::{NewsArticle, NewsCategory};
use rusqlite::{Connection, params};

const GAME_PERSISTENCE_LOAD_ERROR: &str = "be.error.gamePersistence.loadFailed";
const GAME_PERSISTENCE_WRITE_ERROR: &str = "be.error.gamePersistence.writeFailed";

/// Insert or replace a news article row.
pub fn upsert_news(conn: &Connection, article: &NewsArticle) -> Result<(), String> {
    let team_ids_json = serde_json::to_string(&article.team_ids)
        .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    let player_ids_json = serde_json::to_string(&article.player_ids)
        .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
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
    let i18n_json =
        serde_json::to_string(&i18n).map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;

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
    .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    Ok(())
}

/// Insert or replace multiple news articles.
pub fn upsert_news_list(conn: &Connection, articles: &[NewsArticle]) -> Result<(), String> {
    for a in articles {
        upsert_news(conn, a)?;
    }
    Ok(())
}

/// Brings the stored feed in line with `articles`, writing only the difference.
///
/// A save re-writes an existing database, so upserting alone cannot express a removal: the
/// season rollover clears `game.news`, and with upsert-only writes last season's coverage came
/// back on the next load (and let a stale `roundup_md1` suppress the new season's first roundup).
///
/// An article is written once and then only ever marked read, and the season rollover retires
/// the whole feed, so the difference is small: new ids are inserted, a changed read flag is
/// updated, and ids the feed no longer holds are deleted. The stored feed ends up exactly
/// `articles`, each id once. Runs on the caller's connection, which `save_game` wraps in a transaction,
/// so an interrupted save cannot leave the feed half-written.
pub fn sync_news_list(conn: &Connection, articles: &[NewsArticle]) -> Result<(), String> {
    let mut stored = stored_read_flags(conn)?;
    let wanted: std::collections::HashSet<&str> =
        articles.iter().map(|article| article.id.as_str()).collect();
    for retired in stored.keys().filter(|id| !wanted.contains(id.as_str())) {
        conn.execute("DELETE FROM news WHERE id = ?1", params![retired])
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    }
    for article in articles {
        match stored.get(&article.id) {
            Some(read) if *read == article.read => {}
            Some(_) => {
                conn.execute(
                    "UPDATE news SET read = ?2 WHERE id = ?1",
                    params![article.id, article.read as i32],
                )
                .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
            }
            None => upsert_news(conn, article)?,
        }
        stored.insert(article.id.clone(), article.read);
    }
    Ok(())
}

fn stored_read_flags(conn: &Connection) -> Result<std::collections::HashMap<String, bool>, String> {
    let mut stmt = conn
        .prepare("SELECT id, read FROM news")
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;
    stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)? != 0))
    })
    .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?
    .map(|row| row.map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string()))
    .collect()
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
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let rows = stmt
        .query_map([], row_to_news)
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let mut articles = Vec::new();
    for row in rows {
        articles.push(row.map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?);
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
    use rusqlite::Connection;

    /// Pins the stored name of every news category.
    /// The case list also generates an exhaustive match for new variants.
    #[test]
    fn news_categories_are_stored_by_name() {
        use NewsCategory as C;
        crate::stored_text::assert_stored_as(
            &crate::stored_text::stored_text_cases!(
                C::MatchReport; [
                    ("MatchReport", C::MatchReport),
                    ("LeagueRoundup", C::LeagueRoundup),
                    ("StandingsUpdate", C::StandingsUpdate),
                    ("TransferRumour", C::TransferRumour),
                    ("TransferRoundup", C::TransferRoundup),
                    ("InjuryNews", C::InjuryNews),
                    ("ManagerialChange", C::ManagerialChange),
                    ("SeasonPreview", C::SeasonPreview),
                    ("Editorial", C::Editorial),
                ]
            ),
            |value| format!("{value:?}"),
            parse_news_category,
        );
    }

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

    #[test]
    fn test_upsert_news_returns_backend_key_when_schema_is_missing() {
        let conn = Connection::open_in_memory().unwrap();
        let article = sample_article("news-001");

        let result = upsert_news(&conn, &article);

        assert_eq!(result.unwrap_err(), GAME_PERSISTENCE_WRITE_ERROR);
    }

    #[test]
    fn test_load_news_returns_backend_key_when_schema_is_missing() {
        let conn = Connection::open_in_memory().unwrap();

        let result = load_all_news(&conn);

        assert_eq!(result.unwrap_err(), GAME_PERSISTENCE_LOAD_ERROR);
    }

    fn rows_written_by(conn: &Connection, save: impl FnOnce()) -> u64 {
        let before = conn.total_changes();
        save();
        conn.total_changes() - before
    }

    fn feed_of(count: usize) -> Vec<NewsArticle> {
        (0..count)
            .map(|n| sample_article(&format!("news-{n:04}")))
            .collect()
    }

    /// Given a feed already saved, when the same feed is saved again, then nothing is written.
    #[test]
    fn saving_an_unchanged_feed_writes_nothing() {
        let db = test_db();
        let feed = feed_of(50);
        sync_news_list(db.conn(), &feed).unwrap();

        let written = rows_written_by(db.conn(), || sync_news_list(db.conn(), &feed).unwrap());

        assert_eq!(written, 0);
    }

    /// Given a saved feed and one new article, when it is saved, then one row is written.
    #[test]
    fn a_new_article_writes_one_row() {
        let db = test_db();
        let mut feed = feed_of(50);
        sync_news_list(db.conn(), &feed).unwrap();
        feed.push(sample_article("news-fresh"));

        let written = rows_written_by(db.conn(), || sync_news_list(db.conn(), &feed).unwrap());

        assert_eq!(written, 1);
        assert_eq!(load_all_news(db.conn()).unwrap().len(), 51);
    }

    /// Given a saved feed and one article read since, when it is saved, then that article's
    /// read flag is updated and nothing else is written.
    #[test]
    fn reading_an_article_writes_only_that_flag() {
        let db = test_db();
        let mut feed = feed_of(20);
        sync_news_list(db.conn(), &feed).unwrap();
        feed[7].read = true;

        let written = rows_written_by(db.conn(), || sync_news_list(db.conn(), &feed).unwrap());

        assert_eq!(written, 1);
        let stored = load_all_news(db.conn()).unwrap();
        assert_eq!(stored.iter().filter(|a| a.read).count(), 1);
        assert!(stored.iter().find(|a| a.id == "news-0007").unwrap().read);
    }

    /// Given a saved feed that the season rollover then clears, when it is saved, then the
    /// stored feed is empty: last season's coverage does not come back on the next load.
    #[test]
    fn a_cleared_feed_empties_the_stored_one() {
        let db = test_db();
        sync_news_list(db.conn(), &feed_of(30)).unwrap();

        sync_news_list(db.conn(), &[]).unwrap();

        assert!(load_all_news(db.conn()).unwrap().is_empty());
    }

    /// Given a saved feed of which some articles are retired and a few added, when it is saved,
    /// then the file holds exactly the new feed, each article once.
    #[test]
    fn retired_articles_leave_and_new_ones_arrive() {
        let db = test_db();
        sync_news_list(db.conn(), &feed_of(10)).unwrap();
        let mut next = feed_of(10).split_off(6);
        next.push(sample_article("news-new-a"));
        next.push(sample_article("news-new-b"));

        sync_news_list(db.conn(), &next).unwrap();

        let mut ids: Vec<_> = load_all_news(db.conn())
            .unwrap()
            .into_iter()
            .map(|a| a.id)
            .collect();
        ids.sort();
        let mut expected: Vec<_> = next.iter().map(|a| a.id.clone()).collect();
        expected.sort();
        assert_eq!(ids, expected);
    }
}
