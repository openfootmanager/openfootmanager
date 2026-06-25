use domain::league::Fixture;
use domain::national_team::NationalTeam;
use rusqlite::{Connection, params};

const GAME_PERSISTENCE_LOAD_ERROR: &str = "be.error.gamePersistence.loadFailed";
const GAME_PERSISTENCE_WRITE_ERROR: &str = "be.error.gamePersistence.writeFailed";

pub fn replace_national_teams(
    conn: &Connection,
    national_teams: &[NationalTeam],
) -> Result<(), String> {
    conn.execute("DELETE FROM national_teams", [])
        .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;

    for team in national_teams {
        let squad_player_ids_json = serde_json::to_string(&team.squad_player_ids)
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
        let fixtures_json = serde_json::to_string(&team.fixtures)
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;

        conn.execute(
            "INSERT INTO national_teams (id, name, football_nation, region_id, squad_player_ids_json, manager_name, reputation, fixtures_json, name_key)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                team.id,
                team.name,
                team.football_nation,
                team.region_id,
                squad_player_ids_json,
                team.manager_name,
                team.reputation,
                fixtures_json,
                team.name_key,
            ],
        )
        .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    }

    Ok(())
}

pub fn load_national_teams(conn: &Connection) -> Result<Vec<NationalTeam>, String> {
    let mut stmt = match conn.prepare(
        "SELECT id, name, football_nation, region_id, squad_player_ids_json, manager_name, reputation, fixtures_json, name_key
         FROM national_teams
         ORDER BY name ASC",
    ) {
        Ok(stmt) => stmt,
        Err(_) => return Ok(Vec::new()),
    };

    let rows = stmt
        .query_map([], |row| {
            let squad_player_ids_json: String = row.get(4)?;
            let fixtures_json: String = row.get(7)?;
            Ok(NationalTeam {
                id: row.get(0)?,
                name: row.get(1)?,
                football_nation: row.get(2)?,
                region_id: row.get(3)?,
                squad_player_ids: serde_json::from_str(&squad_player_ids_json).unwrap_or_default(),
                manager_name: row.get(5)?,
                reputation: row.get::<_, i64>(6).unwrap_or(500) as u32,
                fixtures: serde_json::from_str::<Vec<Fixture>>(&fixtures_json).unwrap_or_default(),
                name_key: row.get(8)?,
            })
        })
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let mut national_teams = Vec::new();
    for row in rows {
        national_teams.push(row.map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?);
    }
    Ok(national_teams)
}
