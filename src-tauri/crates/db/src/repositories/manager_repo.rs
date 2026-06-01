use domain::manager::{Manager, ManagerCareerEntry, ManagerCareerStats};
use rusqlite::{Connection, params};

const GAME_PERSISTENCE_LOAD_ERROR: &str = "be.error.gamePersistence.loadFailed";
const GAME_PERSISTENCE_WRITE_ERROR: &str = "be.error.gamePersistence.writeFailed";

/// Insert or replace a manager row.
pub fn upsert_manager(conn: &Connection, m: &Manager) -> Result<(), String> {
    let career_stats_json = serde_json::to_string(&m.career_stats)
        .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    let career_history_json = serde_json::to_string(&m.career_history)
        .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;

    conn.execute(
        "INSERT OR REPLACE INTO managers
         (id, first_name, last_name, date_of_birth, nationality, football_nation, birth_country, reputation, satisfaction, fan_approval, team_id, warning_stage, career_stats, career_history)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            m.id,
            m.first_name,
            m.last_name,
            m.date_of_birth,
            m.nationality,
            m.football_nation,
            m.birth_country,
            m.reputation,
            m.satisfaction,
            m.fan_approval,
            m.team_id,
            m.warning_stage,
            career_stats_json,
            career_history_json,
        ],
    )
    .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    Ok(())
}

/// Load a manager by id.
pub fn load_manager(conn: &Connection, id: &str) -> Result<Option<Manager>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, first_name, last_name, date_of_birth, nationality, football_nation, birth_country, reputation, satisfaction, fan_approval, team_id, warning_stage, career_stats, career_history
             FROM managers WHERE id = ?1",
        )
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let mut rows = stmt
        .query_map(params![id], |row| {
            let career_stats_json: String = row.get(12)?;
            let career_history_json: String = row.get(13)?;
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, u32>(7)?,
                row.get::<_, u8>(8)?,
                row.get::<_, u8>(9)?,
                row.get::<_, Option<String>>(10)?,
                row.get::<_, u8>(11)?,
                career_stats_json,
                career_history_json,
            ))
        })
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    match rows.next() {
        Some(Ok((
            id,
            first_name,
            last_name,
            dob,
            nationality,
            football_nation,
            birth_country,
            reputation,
            satisfaction,
            fan_approval,
            team_id,
            warning_stage,
            stats_json,
            history_json,
        ))) => {
            let career_stats: ManagerCareerStats = serde_json::from_str(&stats_json)
                .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;
            let career_history: Vec<ManagerCareerEntry> = serde_json::from_str(&history_json)
                .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

            Ok(Some(Manager {
                id,
                first_name,
                last_name,
                date_of_birth: dob,
                nationality,
                football_nation,
                birth_country,
                reputation,
                satisfaction,
                fan_approval,
                team_id,
                warning_stage,
                career_stats,
                career_history,
            }))
        }
        Some(Err(_)) => Err(GAME_PERSISTENCE_LOAD_ERROR.to_string()),
        None => Ok(None),
    }
}

/// Load all managers.
pub fn load_all_managers(conn: &Connection) -> Result<Vec<Manager>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, first_name, last_name, date_of_birth, nationality, football_nation, birth_country, reputation, satisfaction, fan_approval, team_id, warning_stage, career_stats, career_history
             FROM managers",
        )
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, u32>(7)?,
                row.get::<_, u8>(8)?,
                row.get::<_, u8>(9)?,
                row.get::<_, Option<String>>(10)?,
                row.get::<_, u8>(11)?,
                row.get::<_, String>(12)?,
                row.get::<_, String>(13)?,
            ))
        })
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let mut managers = Vec::new();
    for row in rows {
        let (
            id,
            first_name,
            last_name,
            dob,
            nationality,
            football_nation,
            birth_country,
            reputation,
            satisfaction,
            fan_approval,
            team_id,
            warning_stage,
            stats_json,
            history_json,
        ) = row.map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;
        let career_stats: ManagerCareerStats = serde_json::from_str(&stats_json)
            .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;
        let career_history: Vec<ManagerCareerEntry> = serde_json::from_str(&history_json)
            .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;
        managers.push(Manager {
            id,
            first_name,
            last_name,
            date_of_birth: dob,
            nationality,
            football_nation,
            birth_country,
            reputation,
            satisfaction,
            fan_approval,
            team_id,
            warning_stage,
            career_stats,
            career_history,
        });
    }
    Ok(managers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_database::GameDatabase;
    use rusqlite::Connection;

    fn test_db() -> GameDatabase {
        GameDatabase::open_in_memory().unwrap()
    }

    fn sample_manager() -> Manager {
        Manager::new(
            "mgr_user".to_string(),
            "John".to_string(),
            "Smith".to_string(),
            "1990-01-15".to_string(),
            "British".to_string(),
        )
    }

    #[test]
    fn test_upsert_and_load_manager() {
        let db = test_db();
        let mut mgr = sample_manager();
        mgr.hire("team-001".to_string());
        mgr.reputation = 750;

        upsert_manager(db.conn(), &mgr).unwrap();
        let loaded = load_manager(db.conn(), "mgr_user").unwrap().unwrap();

        assert_eq!(loaded.id, "mgr_user");
        assert_eq!(loaded.first_name, "John");
        assert_eq!(loaded.last_name, "Smith");
        assert_eq!(loaded.team_id, Some("team-001".to_string()));
        assert_eq!(loaded.reputation, 750);
        assert_eq!(loaded.satisfaction, 100);
        assert_eq!(loaded.fan_approval, 50);
        assert_eq!(loaded.football_nation, "GB");
        assert_eq!(loaded.birth_country, None);
    }

    #[test]
    fn test_load_manager_not_found() {
        let db = test_db();
        let loaded = load_manager(db.conn(), "nonexistent").unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn test_upsert_overwrites_manager() {
        let db = test_db();
        let mut mgr = sample_manager();
        upsert_manager(db.conn(), &mgr).unwrap();

        mgr.reputation = 999;
        mgr.satisfaction = 50;
        upsert_manager(db.conn(), &mgr).unwrap();

        let loaded = load_manager(db.conn(), "mgr_user").unwrap().unwrap();
        assert_eq!(loaded.reputation, 999);
        assert_eq!(loaded.satisfaction, 50);
    }

    #[test]
    fn test_load_all_managers() {
        let db = test_db();
        let mgr1 = sample_manager();
        let mut mgr2 = Manager::new(
            "mgr_ai".to_string(),
            "Jane".to_string(),
            "Doe".to_string(),
            "1985-06-20".to_string(),
            "German".to_string(),
        );
        mgr2.hire("team-002".to_string());

        upsert_manager(db.conn(), &mgr1).unwrap();
        upsert_manager(db.conn(), &mgr2).unwrap();

        let all = load_all_managers(db.conn()).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_career_stats_roundtrip() {
        let db = test_db();
        let mut mgr = sample_manager();
        mgr.career_stats.matches_managed = 42;
        mgr.career_stats.wins = 20;
        mgr.career_stats.trophies = 1;

        upsert_manager(db.conn(), &mgr).unwrap();
        let loaded = load_manager(db.conn(), "mgr_user").unwrap().unwrap();

        assert_eq!(loaded.career_stats.matches_managed, 42);
        assert_eq!(loaded.career_stats.wins, 20);
        assert_eq!(loaded.career_stats.trophies, 1);
    }

    #[test]
    fn test_upsert_manager_returns_backend_key_when_schema_is_missing() {
        let conn = Connection::open_in_memory().unwrap();
        let manager = sample_manager();

        let result = upsert_manager(&conn, &manager);

        assert_eq!(result.unwrap_err(), GAME_PERSISTENCE_WRITE_ERROR);
    }

    #[test]
    fn test_load_manager_returns_backend_key_when_schema_is_missing() {
        let conn = Connection::open_in_memory().unwrap();

        let result = load_manager(&conn, "mgr_user");

        assert_eq!(result.unwrap_err(), GAME_PERSISTENCE_LOAD_ERROR);
    }
}
