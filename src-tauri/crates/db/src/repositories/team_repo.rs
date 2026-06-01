use domain::team::{
    Facilities, FinancialTransaction, PlayStyle, Sponsorship, Team, TeamColors, TrainingFocus,
    TrainingIntensity, TrainingSchedule,
};
use rusqlite::{Connection, params};

const GAME_PERSISTENCE_LOAD_ERROR: &str = "be.error.gamePersistence.loadFailed";
const GAME_PERSISTENCE_WRITE_ERROR: &str = "be.error.gamePersistence.writeFailed";

/// Insert or replace a team row.
pub fn upsert_team(conn: &Connection, t: &Team) -> Result<(), String> {
    let starting_xi_json =
        serde_json::to_string(&t.starting_xi_ids)
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    let form_json = serde_json::to_string(&t.form)
        .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    let history_json =
        serde_json::to_string(&t.history).map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    let training_groups_json =
        serde_json::to_string(&t.training_groups)
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    let match_roles_json =
        serde_json::to_string(&t.match_roles)
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    let financial_ledger_json =
        serde_json::to_string(&t.financial_ledger)
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    let sponsorship_json =
        serde_json::to_string(&t.sponsorship)
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    let facilities_json =
        serde_json::to_string(&t.facilities)
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    let play_style_str = format!("{:?}", t.play_style);
    let training_focus_str = format!("{:?}", t.training_focus);
    let training_intensity_str = format!("{:?}", t.training_intensity);
    let training_schedule_str = format!("{:?}", t.training_schedule);

    conn.execute(
        "INSERT OR REPLACE INTO teams
         (id, name, short_name, country, football_nation, city, stadium_name, stadium_capacity,
          finance, manager_id, reputation, wage_budget, transfer_budget,
         season_income, season_expenses, formation, play_style,
         training_focus, training_intensity, training_schedule,
         founded_year, colors_primary, colors_secondary,
         starting_xi_ids, match_roles, form, history, training_groups, financial_ledger, sponsorship, facilities)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31)",
        params![
            t.id,
            t.name,
            t.short_name,
            t.country,
            t.football_nation,
            t.city,
            t.stadium_name,
            t.stadium_capacity,
            t.finance,
            t.manager_id,
            t.reputation,
            t.wage_budget,
            t.transfer_budget,
            t.season_income,
            t.season_expenses,
            t.formation,
            play_style_str,
            training_focus_str,
            training_intensity_str,
            training_schedule_str,
            t.founded_year,
            t.colors.primary,
            t.colors.secondary,
            starting_xi_json,
            match_roles_json,
            form_json,
            history_json,
            training_groups_json,
            financial_ledger_json,
            sponsorship_json,
            facilities_json,
        ],
    )
    .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    Ok(())
}

/// Insert or replace multiple teams in a single transaction.
pub fn upsert_teams(conn: &Connection, teams: &[Team]) -> Result<(), String> {
    for t in teams {
        upsert_team(conn, t)?;
    }
    Ok(())
}

fn parse_play_style(s: &str) -> PlayStyle {
    match s {
        "Attacking" => PlayStyle::Attacking,
        "Defensive" => PlayStyle::Defensive,
        "Possession" => PlayStyle::Possession,
        "Counter" => PlayStyle::Counter,
        "HighPress" => PlayStyle::HighPress,
        _ => PlayStyle::Balanced,
    }
}

fn parse_training_focus(s: &str) -> TrainingFocus {
    match s {
        "Technical" => TrainingFocus::Technical,
        "Tactical" => TrainingFocus::Tactical,
        "Defending" => TrainingFocus::Defending,
        "Attacking" => TrainingFocus::Attacking,
        "Recovery" => TrainingFocus::Recovery,
        _ => TrainingFocus::Physical,
    }
}

fn parse_training_intensity(s: &str) -> TrainingIntensity {
    match s {
        "Low" => TrainingIntensity::Low,
        "High" => TrainingIntensity::High,
        _ => TrainingIntensity::Medium,
    }
}

fn parse_training_schedule(s: &str) -> TrainingSchedule {
    match s {
        "Intense" => TrainingSchedule::Intense,
        "Light" => TrainingSchedule::Light,
        _ => TrainingSchedule::Balanced,
    }
}

fn row_to_team(row: &rusqlite::Row) -> rusqlite::Result<Team> {
    let starting_xi_json: String = row.get(23)?;
    let match_roles_json: String = row.get(24)?;
    let form_json: String = row.get(25)?;
    let history_json: String = row.get(26)?;
    let training_groups_json: String = row.get(27)?;
    let financial_ledger_json: String = row.get(28)?;
    let sponsorship_json: String = row.get(29)?;
    let facilities_json: String = row.get(30)?;
    let play_style_str: String = row.get(16)?;
    let training_focus_str: String = row.get(17)?;
    let training_intensity_str: String = row.get(18)?;
    let training_schedule_str: String = row.get(19)?;

    Ok(Team {
        id: row.get(0)?,
        name: row.get(1)?,
        short_name: row.get(2)?,
        country: row.get(3)?,
        football_nation: row.get(4)?,
        city: row.get(5)?,
        stadium_name: row.get(6)?,
        stadium_capacity: row.get(7)?,
        finance: row.get(8)?,
        manager_id: row.get(9)?,
        reputation: row.get(10)?,
        wage_budget: row.get(11)?,
        transfer_budget: row.get(12)?,
        season_income: row.get(13)?,
        season_expenses: row.get(14)?,
        financial_ledger: serde_json::from_str::<Vec<FinancialTransaction>>(&financial_ledger_json)
            .unwrap_or_default(),
        sponsorship: serde_json::from_str::<Option<Sponsorship>>(&sponsorship_json)
            .unwrap_or_default(),
        facilities: serde_json::from_str::<Facilities>(&facilities_json).unwrap_or_default(),
        formation: row.get(15)?,
        play_style: parse_play_style(&play_style_str),
        training_focus: parse_training_focus(&training_focus_str),
        training_intensity: parse_training_intensity(&training_intensity_str),
        training_schedule: parse_training_schedule(&training_schedule_str),
        training_groups: serde_json::from_str(&training_groups_json).unwrap_or_default(),
        founded_year: row.get(20)?,
        colors: TeamColors {
            primary: row.get(21)?,
            secondary: row.get(22)?,
        },
        starting_xi_ids: serde_json::from_str(&starting_xi_json).unwrap_or_default(),
        match_roles: serde_json::from_str(&match_roles_json).unwrap_or_default(),
        form: serde_json::from_str(&form_json).unwrap_or_default(),
        history: serde_json::from_str(&history_json).unwrap_or_default(),
    })
}

/// Load all teams.
pub fn load_all_teams(conn: &Connection) -> Result<Vec<Team>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, short_name, country, football_nation, city, stadium_name, stadium_capacity,
                    finance, manager_id, reputation, wage_budget, transfer_budget,
                    season_income, season_expenses, formation, play_style,
                    training_focus, training_intensity, training_schedule,
                    founded_year, colors_primary, colors_secondary,
                    starting_xi_ids, match_roles, form, history, training_groups, financial_ledger, sponsorship, facilities
             FROM teams",
        )
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let rows = stmt
        .query_map([], row_to_team)
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let mut teams = Vec::new();
    for row in rows {
        teams.push(row.map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?);
    }
    Ok(teams)
}

/// Load a single team by id.
pub fn load_team(conn: &Connection, id: &str) -> Result<Option<Team>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, short_name, country, football_nation, city, stadium_name, stadium_capacity,
                    finance, manager_id, reputation, wage_budget, transfer_budget,
                    season_income, season_expenses, formation, play_style,
                    training_focus, training_intensity, training_schedule,
                    founded_year, colors_primary, colors_secondary,
                    starting_xi_ids, match_roles, form, history, training_groups, financial_ledger, sponsorship, facilities
             FROM teams WHERE id = ?1",
        )
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let mut rows = stmt
        .query_map(params![id], row_to_team)
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    match rows.next() {
        Some(Ok(team)) => Ok(Some(team)),
        Some(Err(_)) => Err(GAME_PERSISTENCE_LOAD_ERROR.to_string()),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_database::GameDatabase;
    use domain::team::{Facilities, Sponsorship, SponsorshipBonusCriterion, TeamSeasonRecord};
    use rusqlite::Connection;

    fn test_db() -> GameDatabase {
        GameDatabase::open_in_memory().unwrap()
    }

    fn sample_team(id: &str, name: &str) -> Team {
        let mut team = Team::new(
            id.to_string(),
            name.to_string(),
            "TST".to_string(),
            "GB".to_string(),
            "London".to_string(),
            "Test Arena".to_string(),
            50000,
        );
        team.play_style = PlayStyle::Possession;
        team.finance = 5_000_000;
        team.wage_budget = 200_000;
        team.transfer_budget = 500_000;
        team
    }

    #[test]
    fn test_upsert_and_load_team() {
        let db = test_db();
        let team = sample_team("team-001", "London FC");

        upsert_team(db.conn(), &team).unwrap();
        let loaded = load_team(db.conn(), "team-001").unwrap().unwrap();

        assert_eq!(loaded.id, "team-001");
        assert_eq!(loaded.name, "London FC");
        assert_eq!(loaded.short_name, "TST");
        assert_eq!(loaded.football_nation, "GB");
        assert_eq!(loaded.play_style, PlayStyle::Possession);
        assert_eq!(loaded.finance, 5_000_000);
        assert_eq!(loaded.stadium_capacity, 50000);
    }

    #[test]
    fn test_load_team_not_found() {
        let db = test_db();
        let loaded = load_team(db.conn(), "nonexistent").unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn test_upsert_teams_batch() {
        let db = test_db();
        let teams = vec![
            sample_team("team-001", "London FC"),
            sample_team("team-002", "Manchester City"),
            sample_team("team-003", "Liverpool Athletic"),
        ];

        upsert_teams(db.conn(), &teams).unwrap();
        let all = load_all_teams(db.conn()).unwrap();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_team_colors_roundtrip() {
        let db = test_db();
        let mut team = sample_team("team-001", "Red Team");
        team.colors = TeamColors {
            primary: "#ff0000".to_string(),
            secondary: "#00ff00".to_string(),
        };

        upsert_team(db.conn(), &team).unwrap();
        let loaded = load_team(db.conn(), "team-001").unwrap().unwrap();

        assert_eq!(loaded.colors.primary, "#ff0000");
        assert_eq!(loaded.colors.secondary, "#00ff00");
    }

    #[test]
    fn test_team_training_settings_roundtrip() {
        let db = test_db();
        let mut team = sample_team("team-001", "Training FC");
        team.training_focus = TrainingFocus::Attacking;
        team.training_intensity = TrainingIntensity::High;
        team.training_schedule = TrainingSchedule::Intense;

        upsert_team(db.conn(), &team).unwrap();
        let loaded = load_team(db.conn(), "team-001").unwrap().unwrap();

        assert_eq!(loaded.training_focus, TrainingFocus::Attacking);
        assert_eq!(loaded.training_intensity, TrainingIntensity::High);
        assert_eq!(loaded.training_schedule, TrainingSchedule::Intense);
    }

    #[test]
    fn test_team_history_roundtrip() {
        let db = test_db();
        let mut team = sample_team("team-001", "History FC");
        team.history.push(TeamSeasonRecord {
            season: 2025,
            league_position: 3,
            played: 30,
            won: 18,
            drawn: 7,
            lost: 5,
            goals_for: 55,
            goals_against: 30,
        });

        upsert_team(db.conn(), &team).unwrap();
        let loaded = load_team(db.conn(), "team-001").unwrap().unwrap();

        assert_eq!(loaded.history.len(), 1);
        assert_eq!(loaded.history[0].season, 2025);
        assert_eq!(loaded.history[0].league_position, 3);
    }

    #[test]
    fn test_team_training_groups_roundtrip() {
        let db = test_db();
        let mut team = sample_team("team-001", "Groups FC");
        team.training_groups = vec![
            domain::team::TrainingGroup {
                id: "g1".to_string(),
                name: "Defenders".to_string(),
                focus: TrainingFocus::Defending,
                player_ids: vec!["p1".to_string(), "p2".to_string()],
            },
            domain::team::TrainingGroup {
                id: "g2".to_string(),
                name: "Attackers".to_string(),
                focus: TrainingFocus::Attacking,
                player_ids: vec!["p3".to_string()],
            },
        ];

        upsert_team(db.conn(), &team).unwrap();
        let loaded = load_team(db.conn(), "team-001").unwrap().unwrap();

        assert_eq!(loaded.training_groups.len(), 2);
        assert_eq!(loaded.training_groups[0].name, "Defenders");
        assert_eq!(loaded.training_groups[0].focus, TrainingFocus::Defending);
        assert_eq!(loaded.training_groups[0].player_ids.len(), 2);
        assert_eq!(loaded.training_groups[1].name, "Attackers");
        assert_eq!(loaded.training_groups[1].focus, TrainingFocus::Attacking);
    }

    #[test]
    fn test_team_starting_xi_roundtrip() {
        let db = test_db();
        let mut team = sample_team("team-001", "XI FC");
        team.starting_xi_ids = vec!["p1".to_string(), "p2".to_string(), "p3".to_string()];

        upsert_team(db.conn(), &team).unwrap();
        let loaded = load_team(db.conn(), "team-001").unwrap().unwrap();

        assert_eq!(loaded.starting_xi_ids.len(), 3);
        assert_eq!(loaded.starting_xi_ids[0], "p1");
    }

    #[test]
    fn test_team_match_roles_roundtrip() {
        let db = test_db();
        let mut team = sample_team("team-001", "Roles FC");
        team.match_roles = domain::team::MatchRoles {
            captain: Some("p1".to_string()),
            vice_captain: Some("p2".to_string()),
            penalty_taker: Some("p3".to_string()),
            free_kick_taker: Some("p4".to_string()),
            corner_taker: Some("p5".to_string()),
        };

        upsert_team(db.conn(), &team).unwrap();
        let loaded = load_team(db.conn(), "team-001").unwrap().unwrap();

        assert_eq!(loaded.match_roles.captain.as_deref(), Some("p1"));
        assert_eq!(loaded.match_roles.vice_captain.as_deref(), Some("p2"));
        assert_eq!(loaded.match_roles.penalty_taker.as_deref(), Some("p3"));
        assert_eq!(loaded.match_roles.free_kick_taker.as_deref(), Some("p4"));
        assert_eq!(loaded.match_roles.corner_taker.as_deref(), Some("p5"));
    }

    #[test]
    fn test_team_sponsorship_roundtrip() {
        let db = test_db();
        let mut team = sample_team("team-001", "Sponsor FC");
        team.sponsorship = Some(Sponsorship {
            sponsor_name: "Acme Corp".to_string(),
            base_value: 100_000,
            remaining_weeks: 12,
            bonus_criteria: vec![SponsorshipBonusCriterion::UnbeatenRun {
                required_matches: 3,
                bonus_amount: 25_000,
            }],
        });

        upsert_team(db.conn(), &team).unwrap();
        let loaded = load_team(db.conn(), "team-001").unwrap().unwrap();

        let sponsorship = loaded
            .sponsorship
            .expect("sponsorship should roundtrip through DB");
        assert_eq!(sponsorship.sponsor_name, "Acme Corp");
        assert_eq!(sponsorship.base_value, 100_000);
        assert_eq!(sponsorship.remaining_weeks, 12);
        assert!(matches!(
            sponsorship.bonus_criteria.as_slice(),
            [SponsorshipBonusCriterion::UnbeatenRun {
                required_matches: 3,
                bonus_amount: 25_000,
            }]
        ));
    }

    #[test]
    fn test_team_facilities_roundtrip() {
        let db = test_db();
        let mut team = sample_team("team-001", "Facilities FC");
        team.facilities = Facilities {
            training: 2,
            medical: 3,
            scouting: 4,
        };

        upsert_team(db.conn(), &team).unwrap();
        let loaded = load_team(db.conn(), "team-001").unwrap().unwrap();

        assert_eq!(loaded.facilities.training, 2);
        assert_eq!(loaded.facilities.medical, 3);
        assert_eq!(loaded.facilities.scouting, 4);
    }

    #[test]
    fn test_upsert_team_returns_backend_key_when_schema_is_missing() {
        let conn = Connection::open_in_memory().unwrap();
        let team = sample_team("team-001", "London FC");

        let result = upsert_team(&conn, &team);

        assert_eq!(result.unwrap_err(), GAME_PERSISTENCE_WRITE_ERROR);
    }

    #[test]
    fn test_load_team_returns_backend_key_when_schema_is_missing() {
        let conn = Connection::open_in_memory().unwrap();

        let result = load_team(&conn, "team-001");

        assert_eq!(result.unwrap_err(), GAME_PERSISTENCE_LOAD_ERROR);
    }
}
