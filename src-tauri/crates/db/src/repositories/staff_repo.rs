use domain::staff::{CoachingSpecialization, Staff, StaffAttributes, StaffRole};
use rusqlite::{Connection, params};

const GAME_PERSISTENCE_LOAD_ERROR: &str = "be.error.gamePersistence.loadFailed";
const GAME_PERSISTENCE_WRITE_ERROR: &str = "be.error.gamePersistence.writeFailed";

/// Insert or replace a staff row.
pub fn upsert_staff(conn: &Connection, s: &Staff) -> Result<(), String> {
    let attrs_json =
        serde_json::to_string(&s.attributes).map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    let role_str = format!("{:?}", s.role);
    let spec_str = s.specialization.as_ref().map(|sp| format!("{:?}", sp));

    conn.execute(
        "INSERT OR REPLACE INTO staff
         (id, first_name, last_name, date_of_birth, nationality, football_nation, birth_country, role,
          attributes, team_id, specialization, wage, contract_end)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            s.id,
            s.first_name,
            s.last_name,
            s.date_of_birth,
            s.nationality,
            s.football_nation,
            s.birth_country,
            role_str,
            attrs_json,
            s.team_id,
            spec_str,
            s.wage,
            s.contract_end,
        ],
    )
    .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    Ok(())
}

/// Insert or replace multiple staff members.
pub fn upsert_staff_list(conn: &Connection, staff: &[Staff]) -> Result<(), String> {
    for s in staff {
        upsert_staff(conn, s)?;
    }
    Ok(())
}

fn parse_role(s: &str) -> StaffRole {
    match s {
        "AssistantManager" => StaffRole::AssistantManager,
        "Coach" => StaffRole::Coach,
        "Scout" => StaffRole::Scout,
        "Physio" => StaffRole::Physio,
        _ => StaffRole::Coach,
    }
}

fn parse_specialization(s: &str) -> Option<CoachingSpecialization> {
    match s {
        "Fitness" => Some(CoachingSpecialization::Fitness),
        "Technique" => Some(CoachingSpecialization::Technique),
        "Tactics" => Some(CoachingSpecialization::Tactics),
        "Defending" => Some(CoachingSpecialization::Defending),
        "Attacking" => Some(CoachingSpecialization::Attacking),
        "GoalKeeping" => Some(CoachingSpecialization::GoalKeeping),
        "Youth" => Some(CoachingSpecialization::Youth),
        _ => None,
    }
}

/// Load all staff.
pub fn load_all_staff(conn: &Connection) -> Result<Vec<Staff>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, first_name, last_name, date_of_birth, nationality, football_nation, birth_country, role,
                    attributes, team_id, specialization, wage, contract_end
             FROM staff",
        )
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let rows = stmt
        .query_map([], row_to_staff)
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let mut staff = Vec::new();
    for row in rows {
        staff.push(row.map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?);
    }
    Ok(staff)
}

fn row_to_staff(row: &rusqlite::Row) -> rusqlite::Result<Staff> {
    let role_str: String = row.get(7)?;
    let attrs_json: String = row.get(8)?;
    let spec_str: Option<String> = row.get(10)?;

    Ok(Staff {
        id: row.get(0)?,
        first_name: row.get(1)?,
        last_name: row.get(2)?,
        date_of_birth: row.get(3)?,
        nationality: row.get(4)?,
        football_nation: row.get(5)?,
        birth_country: row.get(6)?,
        role: parse_role(&role_str),
        attributes: serde_json::from_str(&attrs_json).unwrap_or(StaffAttributes {
            coaching: 50,
            judging_ability: 50,
            judging_potential: 50,
            physiotherapy: 50,
        }),
        team_id: row.get(9)?,
        specialization: spec_str.and_then(|s| parse_specialization(&s)),
        wage: row.get(11)?,
        contract_end: row.get(12)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_database::GameDatabase;
    use rusqlite::Connection;

    fn test_db() -> GameDatabase {
        GameDatabase::open_in_memory().unwrap()
    }

    fn sample_staff(id: &str, role: StaffRole) -> Staff {
        let mut s = Staff::new(
            id.to_string(),
            "Alice".to_string(),
            "Coach".to_string(),
            "1980-05-10".to_string(),
            role,
            StaffAttributes {
                coaching: 75,
                judging_ability: 60,
                judging_potential: 55,
                physiotherapy: 40,
            },
        );
        s.nationality = "GB".to_string();
        s.team_id = Some("team-001".to_string());
        s.wage = 3000;
        s
    }

    #[test]
    fn test_upsert_and_load_staff() {
        let db = test_db();
        let mut staff = sample_staff("staff-001", StaffRole::Coach);
        staff.nationality = "Scottish".to_string();
        staff.football_nation = "SCO".to_string();
        staff.birth_country = Some("SCO".to_string());

        upsert_staff(db.conn(), &staff).unwrap();
        let all = load_all_staff(db.conn()).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "staff-001");
        assert_eq!(all[0].role, StaffRole::Coach);
        assert_eq!(all[0].attributes.coaching, 75);
        assert_eq!(all[0].wage, 3000);
        assert_eq!(all[0].football_nation, "SCO");
        assert_eq!(all[0].birth_country, Some("SCO".to_string()));
    }

    #[test]
    fn test_upsert_staff_list() {
        let db = test_db();
        let list = vec![
            sample_staff("s-001", StaffRole::Coach),
            sample_staff("s-002", StaffRole::Scout),
            sample_staff("s-003", StaffRole::Physio),
            sample_staff("s-004", StaffRole::AssistantManager),
        ];

        upsert_staff_list(db.conn(), &list).unwrap();
        let all = load_all_staff(db.conn()).unwrap();
        assert_eq!(all.len(), 4);
    }

    #[test]
    fn test_upsert_staff_returns_backend_key_when_schema_is_missing() {
        let conn = Connection::open_in_memory().unwrap();
        let staff = sample_staff("staff-001", StaffRole::Coach);

        let result = upsert_staff(&conn, &staff);

        assert_eq!(result.unwrap_err(), GAME_PERSISTENCE_WRITE_ERROR);
    }

    #[test]
    fn test_load_staff_returns_backend_key_when_schema_is_missing() {
        let conn = Connection::open_in_memory().unwrap();

        let result = load_all_staff(&conn);

        assert_eq!(result.unwrap_err(), GAME_PERSISTENCE_LOAD_ERROR);
    }

    #[test]
    fn test_staff_specialization_roundtrip() {
        let db = test_db();
        let mut staff = sample_staff("s-001", StaffRole::Coach);
        staff.specialization = Some(CoachingSpecialization::Attacking);

        upsert_staff(db.conn(), &staff).unwrap();
        let all = load_all_staff(db.conn()).unwrap();
        assert_eq!(
            all[0].specialization,
            Some(CoachingSpecialization::Attacking)
        );
    }

    #[test]
    fn test_staff_no_specialization() {
        let db = test_db();
        let staff = sample_staff("s-001", StaffRole::Physio);

        upsert_staff(db.conn(), &staff).unwrap();
        let all = load_all_staff(db.conn()).unwrap();
        assert!(all[0].specialization.is_none());
    }
}
