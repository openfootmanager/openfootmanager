use domain::player::{Footedness, Player, PlayerAttributes, PlayerMedia, Position, SquadRole};
use domain::team::TrainingFocus;
use rusqlite::{Connection, params};

const GAME_PERSISTENCE_LOAD_ERROR: &str = "be.error.gamePersistence.loadFailed";
const GAME_PERSISTENCE_WRITE_ERROR: &str = "be.error.gamePersistence.writeFailed";

/// Insert or replace a player row.
pub fn upsert_player(conn: &Connection, p: &Player) -> Result<(), String> {
    let attrs_json = serde_json::to_string(&p.attributes)
        .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    let injury_json = p
        .injury
        .as_ref()
        .map(|i| serde_json::to_string(i).unwrap_or_default());
    let traits_json =
        serde_json::to_string(&p.traits).map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    let stats_json =
        serde_json::to_string(&p.stats).map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    let career_json =
        serde_json::to_string(&p.career).map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    let movement_history_json = serde_json::to_string(&p.movement_history)
        .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    let offers_json = serde_json::to_string(&p.transfer_offers)
        .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    let loan_offers_json = serde_json::to_string(&p.loan_offers)
        .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    let active_loan_json = p
        .active_loan
        .as_ref()
        .map(|loan| serde_json::to_string(loan).unwrap_or_default());
    let morale_core_json = serde_json::to_string(&p.morale_core)
        .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    let media_json =
        serde_json::to_string(&p.media).map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    let position_str = format!("{:?}", p.position);
    let natural_position_str = format!("{:?}", p.natural_position);
    let alt_positions_json = serde_json::to_string(&p.alternate_positions)
        .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    let footedness_str = format!("{:?}", p.footedness);
    let training_focus_str: Option<String> = p.training_focus.as_ref().map(|f| format!("{:?}", f));

    conn.execute(
        "INSERT INTO players
         (id, match_name, full_name, date_of_birth, nationality, football_nation, birth_country, position,
          attributes, condition, morale, injury, team_id, retired, traits,
          contract_end, wage, market_value, stats, career,
          transfer_listed, loan_listed, transfer_offers, alternate_positions,
          natural_position, training_focus, morale_core, footedness, weak_foot, fitness, squad_role,
          ovr, potential, media_json, jersey_number, loan_offers, active_loan, movement_history)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33, ?34, ?35, ?36, ?37, ?38)
         ON CONFLICT(id) DO UPDATE SET
           match_name = excluded.match_name,
           full_name = excluded.full_name,
           date_of_birth = excluded.date_of_birth,
           nationality = excluded.nationality,
           football_nation = excluded.football_nation,
           birth_country = excluded.birth_country,
           position = excluded.position,
           attributes = excluded.attributes,
           condition = excluded.condition,
           morale = excluded.morale,
           injury = excluded.injury,
           team_id = excluded.team_id,
           retired = excluded.retired,
           traits = excluded.traits,
           contract_end = excluded.contract_end,
           wage = excluded.wage,
           market_value = excluded.market_value,
           stats = excluded.stats,
           career = excluded.career,
           transfer_listed = excluded.transfer_listed,
           loan_listed = excluded.loan_listed,
           transfer_offers = excluded.transfer_offers,
           alternate_positions = excluded.alternate_positions,
           natural_position = excluded.natural_position,
           training_focus = excluded.training_focus,
           morale_core = excluded.morale_core,
           footedness = excluded.footedness,
           weak_foot = excluded.weak_foot,
           fitness = excluded.fitness,
           squad_role = excluded.squad_role,
           ovr = excluded.ovr,
           potential = excluded.potential,
           media_json = excluded.media_json,
           jersey_number = excluded.jersey_number,
           loan_offers = excluded.loan_offers,
           active_loan = excluded.active_loan,
           movement_history = excluded.movement_history",
        params![
            p.id,
            p.match_name,
            p.full_name,
            p.date_of_birth,
            p.nationality,
            p.football_nation,
            p.birth_country,
            position_str,
            attrs_json,
            p.condition,
            p.morale,
            injury_json,
            p.team_id,
            p.retired as i32,
            traits_json,
            p.contract_end,
            p.wage,
            p.market_value as i64,
            stats_json,
            career_json,
            p.transfer_listed as i32,
            p.loan_listed as i32,
            offers_json,
            alt_positions_json,
            natural_position_str,
            training_focus_str,
            morale_core_json,
            footedness_str,
            p.weak_foot,
            p.fitness,
            format!("{:?}", p.squad_role),
            p.ovr as i64,
            p.potential as i64,
            media_json,
            p.jersey_number.map(|n| n as i64),
            loan_offers_json,
            active_loan_json,
            movement_history_json,
        ],
    )
    .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    Ok(())
}

/// Insert or replace multiple players.
pub fn upsert_players(conn: &Connection, players: &[Player]) -> Result<(), String> {
    for p in players {
        upsert_player(conn, p)?;
    }
    Ok(())
}

fn parse_position(s: &str) -> Position {
    match s {
        "Goalkeeper" => Position::Goalkeeper,
        "Defender" => Position::Defender,
        "Midfielder" => Position::Midfielder,
        "Forward" => Position::Forward,
        "RightBack" => Position::RightBack,
        "CenterBack" => Position::CenterBack,
        "LeftBack" => Position::LeftBack,
        "RightWingBack" => Position::RightWingBack,
        "LeftWingBack" => Position::LeftWingBack,
        "DefensiveMidfielder" => Position::DefensiveMidfielder,
        "CentralMidfielder" => Position::CentralMidfielder,
        "AttackingMidfielder" => Position::AttackingMidfielder,
        "RightMidfielder" => Position::RightMidfielder,
        "LeftMidfielder" => Position::LeftMidfielder,
        "RightWinger" => Position::RightWinger,
        "LeftWinger" => Position::LeftWinger,
        "Striker" => Position::Striker,
        _ => Position::Midfielder,
    }
}

fn parse_footedness(s: &str) -> Footedness {
    match s {
        "Left" => Footedness::Left,
        "Both" => Footedness::Both,
        _ => Footedness::Right,
    }
}

fn parse_squad_role(s: &str) -> SquadRole {
    match s {
        "Youth" => SquadRole::Youth,
        _ => SquadRole::Senior,
    }
}

fn parse_training_focus(s: &str) -> Option<TrainingFocus> {
    match s {
        "Physical" => Some(TrainingFocus::Physical),
        "Technical" => Some(TrainingFocus::Technical),
        "Tactical" => Some(TrainingFocus::Tactical),
        "Defending" => Some(TrainingFocus::Defending),
        "Attacking" => Some(TrainingFocus::Attacking),
        "Recovery" => Some(TrainingFocus::Recovery),
        _ => None,
    }
}

/// Load all players.
pub fn load_all_players(conn: &Connection) -> Result<Vec<Player>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, match_name, full_name, date_of_birth, nationality, football_nation, birth_country, position,
                    attributes, condition, morale, injury, team_id, retired, traits,
                    contract_end, wage, market_value, stats, career,
                    transfer_listed, loan_listed, transfer_offers, alternate_positions,
                    natural_position, training_focus, morale_core, footedness, weak_foot, fitness, squad_role,
                    ovr, potential, COALESCE(media_json, '{}'), jersey_number,
                    COALESCE(loan_offers, '[]'), active_loan,
                    COALESCE(movement_history, '[]')
             FROM players",
        )
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let rows = stmt
        .query_map([], row_to_player)
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let mut players = Vec::new();
    for row in rows {
        players.push(row.map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?);
    }
    Ok(players)
}

/// Load players by team id.
pub fn load_players_by_team(conn: &Connection, team_id: &str) -> Result<Vec<Player>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, match_name, full_name, date_of_birth, nationality, football_nation, birth_country, position,
                    attributes, condition, morale, injury, team_id, retired, traits,
                    contract_end, wage, market_value, stats, career,
                    transfer_listed, loan_listed, transfer_offers, alternate_positions,
                    natural_position, training_focus, morale_core, footedness, weak_foot, fitness, squad_role,
                    ovr, potential, COALESCE(media_json, '{}'), jersey_number,
                    COALESCE(loan_offers, '[]'), active_loan,
                    COALESCE(movement_history, '[]')
             FROM players WHERE team_id = ?1",
        )
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let rows = stmt
        .query_map(params![team_id], row_to_player)
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let mut players = Vec::new();
    for row in rows {
        players.push(row.map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?);
    }
    Ok(players)
}

fn row_to_player(row: &rusqlite::Row) -> rusqlite::Result<Player> {
    let position_str: String = row.get(7)?;
    let attrs_json: String = row.get(8)?;
    let injury_json: Option<String> = row.get(11)?;
    let retired: i32 = row.get(13).unwrap_or(0);
    let traits_json: String = row.get(14)?;
    let stats_json: String = row.get(18)?;
    let career_json: String = row.get(19)?;
    let offers_json: String = row.get(22)?;
    let alt_positions_json: String = row.get(23)?;
    let natural_position_str: String = row.get(24)?;
    let training_focus_str: Option<String> = row.get(25)?;
    let morale_core_json: String = row.get(26)?;
    let footedness_str: String = row.get(27)?;
    let weak_foot: u8 = row.get(28)?;
    let fitness: u8 = row.get(29).unwrap_or(75); // default 75 for saves before V13
    let squad_role_str: String = row.get(30).unwrap_or_else(|_| "Senior".to_string());
    let ovr: u8 = row.get::<_, i64>(31).unwrap_or(0) as u8; // default 0 for saves before V20
    let potential: u8 = row.get::<_, i64>(32).unwrap_or(0) as u8; // default 0 for saves before V20
    let media_json: String = row.get(33).unwrap_or_else(|_| "{}".to_string());
    let jersey_number: Option<u8> = match row.get::<_, Option<i64>>(34)? {
        Some(n) if (1..=99).contains(&n) => Some(n as u8),
        Some(_) => {
            return Err(rusqlite::Error::FromSqlConversionFailure(
                34,
                rusqlite::types::Type::Integer,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "invalid players.jersey_number: expected NULL or 1..=99",
                )),
            ));
        }
        None => None,
    };
    let loan_offers_json: String = row.get(35).unwrap_or_else(|_| "[]".to_string());
    let active_loan_json: Option<String> = row.get(36).unwrap_or(None);
    let movement_history_json: String = row.get(37).unwrap_or_else(|_| "[]".to_string());
    let transfer_listed_int: i32 = row.get(20)?;
    let loan_listed_int: i32 = row.get(21)?;
    let market_value_i64: i64 = row.get(17)?;

    let position = parse_position(&position_str);
    let natural_position = if natural_position_str.is_empty() {
        position.clone()
    } else {
        parse_position(&natural_position_str)
    };

    Ok(Player {
        id: row.get(0)?,
        match_name: row.get(1)?,
        full_name: row.get(2)?,
        date_of_birth: row.get(3)?,
        nationality: row.get(4)?,
        football_nation: row.get(5)?,
        birth_country: row.get(6)?,
        media: serde_json::from_str(&media_json).unwrap_or_else(|_| PlayerMedia::default()),
        position,
        natural_position,
        alternate_positions: serde_json::from_str(&alt_positions_json).unwrap_or_default(),
        footedness: parse_footedness(&footedness_str),
        weak_foot,
        attributes: serde_json::from_str(&attrs_json).unwrap_or(PlayerAttributes {
            pace: 50,
            stamina: 50,
            strength: 50,
            agility: 50,
            passing: 50,
            shooting: 50,
            tackling: 50,
            dribbling: 50,
            defending: 50,
            positioning: 50,
            vision: 50,
            decisions: 50,
            composure: 50,
            aggression: 50,
            teamwork: 50,
            leadership: 50,
            handling: 50,
            reflexes: 50,
            aerial: 50,
        }),
        condition: row.get(9)?,
        morale: row.get(10)?,
        fitness,
        injury: injury_json.and_then(|j| serde_json::from_str(&j).ok()),
        team_id: row.get(12)?,
        retired: retired != 0,
        squad_role: parse_squad_role(&squad_role_str),
        traits: serde_json::from_str(&traits_json).unwrap_or_default(),
        ovr,
        potential,
        contract_end: row.get(15)?,
        wage: row.get(16)?,
        market_value: market_value_i64 as u64,
        stats: serde_json::from_str(&stats_json).unwrap_or_default(),
        career: serde_json::from_str(&career_json).unwrap_or_default(),
        movement_history: serde_json::from_str(&movement_history_json).unwrap_or_default(),
        training_focus: training_focus_str.and_then(|s| parse_training_focus(&s)),
        transfer_listed: transfer_listed_int != 0,
        loan_listed: loan_listed_int != 0,
        transfer_offers: serde_json::from_str(&offers_json).unwrap_or_default(),
        loan_offers: serde_json::from_str(&loan_offers_json).unwrap_or_default(),
        active_loan: active_loan_json.and_then(|json| serde_json::from_str(&json).ok()),
        morale_core: serde_json::from_str(&morale_core_json).unwrap_or_default(),
        jersey_number,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_database::GameDatabase;
    use domain::player::{
        ActiveLoan, Injury, LoanOffer, LoanOfferStatus, PlayerIssue, PlayerIssueCategory,
        PlayerMoraleCore, PlayerMovementEntry, PlayerMovementKind,
    };
    use rusqlite::Connection;

    fn test_db() -> GameDatabase {
        GameDatabase::open_in_memory().unwrap()
    }

    fn sample_player(id: &str, team_id: Option<&str>) -> Player {
        let mut p = Player::new(
            id.to_string(),
            "J. Smith".to_string(),
            "John Smith".to_string(),
            "2000-01-15".to_string(),
            "GB".to_string(),
            Position::Midfielder,
            PlayerAttributes {
                pace: 70,
                stamina: 75,
                strength: 65,
                agility: 72,
                passing: 80,
                shooting: 60,
                tackling: 55,
                dribbling: 68,
                defending: 50,
                positioning: 65,
                vision: 78,
                decisions: 70,
                composure: 60,
                aggression: 55,
                teamwork: 80,
                leadership: 45,
                handling: 20,
                reflexes: 25,
                aerial: 40,
            },
        );
        p.team_id = team_id.map(|s| s.to_string());
        p.wage = 5000;
        p.market_value = 500_000;
        p
    }

    #[test]
    fn test_upsert_and_load_player() {
        let db = test_db();
        let player = sample_player("p-001", Some("team-001"));

        upsert_player(db.conn(), &player).unwrap();
        let all = load_all_players(db.conn()).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "p-001");
        assert_eq!(all[0].full_name, "John Smith");
        assert_eq!(all[0].position, Position::Midfielder);
        assert_eq!(all[0].team_id, Some("team-001".to_string()));
        assert_eq!(all[0].wage, 5000);
        assert_eq!(all[0].market_value, 500_000);
        assert_eq!(all[0].football_nation, "GB");
        assert_eq!(all[0].birth_country, None);
    }

    // Pins the contract of the `INSERT ... ON CONFLICT(id) DO UPDATE SET ...`
    // upsert: every column listed in the SET clause must actually overwrite
    // the previously stored value on a second save with the same id. If a
    // future field is added to `Player` and the SET clause is not updated to
    // match, this test starts failing because the stored row keeps stale data
    // on subsequent saves.
    #[test]
    fn test_upsert_player_with_same_id_overwrites_all_fields() {
        let db = test_db();
        let mut player = sample_player("p-mutating", Some("team-1"));
        player.jersey_number = Some(10);
        player.condition = 80;
        player.morale = 70;
        upsert_player(db.conn(), &player).unwrap();

        player.team_id = Some("team-2".to_string());
        player.jersey_number = Some(7);
        player.condition = 50;
        player.morale = 40;
        player.wage = 12_000;
        player.market_value = 1_200_000;
        player.transfer_listed = true;
        player.loan_listed = true;
        player.position = Position::Striker;
        player.natural_position = Position::Forward;
        player.ovr = 88;
        player.potential = 95;
        player.contract_end = Some("2030-06-30".to_string());
        upsert_player(db.conn(), &player).unwrap();

        let all = load_all_players(db.conn()).unwrap();
        assert_eq!(all.len(), 1, "same id must update in place, not duplicate");
        let stored = &all[0];
        assert_eq!(stored.team_id.as_deref(), Some("team-2"));
        assert_eq!(stored.jersey_number, Some(7));
        assert_eq!(stored.condition, 50);
        assert_eq!(stored.morale, 40);
        assert_eq!(stored.wage, 12_000);
        assert_eq!(stored.market_value, 1_200_000);
        assert!(stored.transfer_listed);
        assert!(stored.loan_listed);
        assert_eq!(stored.position, Position::Striker);
        assert_eq!(stored.natural_position, Position::Forward);
        assert_eq!(stored.ovr, 88);
        assert_eq!(stored.potential, 95);
        assert_eq!(stored.contract_end.as_deref(), Some("2030-06-30"));
    }

    #[test]
    fn test_player_football_identity_roundtrip() {
        let db = test_db();
        let mut player = sample_player("p-eng", Some("team-001"));
        player.nationality = "English".to_string();
        player.football_nation = "ENG".to_string();
        player.birth_country = Some("ENG".to_string());

        upsert_player(db.conn(), &player).unwrap();
        let loaded = load_all_players(db.conn()).unwrap();

        assert_eq!(loaded[0].football_nation, "ENG");
        assert_eq!(loaded[0].birth_country, Some("ENG".to_string()));
    }

    #[test]
    fn test_player_media_face_roundtrip() {
        let db = test_db();
        let mut player = sample_player("p-media", Some("team-001"));
        player.media.face = Some("/assets/worlds/test-world/players/p-media.png".to_string());

        upsert_player(db.conn(), &player).unwrap();
        let loaded = load_all_players(db.conn()).unwrap();
        let stored = loaded
            .iter()
            .find(|candidate| candidate.id == "p-media")
            .expect("stored player should exist");

        assert_eq!(
            stored.media.face.as_deref(),
            Some("/assets/worlds/test-world/players/p-media.png")
        );
    }

    #[test]
    fn test_player_squad_role_roundtrip() {
        let db = test_db();
        let mut player = sample_player("p-youth", Some("team-001"));
        player.squad_role = SquadRole::Youth;

        upsert_player(db.conn(), &player).unwrap();
        let loaded = load_all_players(db.conn()).unwrap();

        assert_eq!(loaded[0].squad_role, SquadRole::Youth);
    }

    #[test]
    fn test_player_ovr_potential_roundtrip() {
        let db = test_db();
        let mut player = sample_player("p-rated", Some("team-001"));
        player.ovr = 78;
        player.potential = 85;

        upsert_player(db.conn(), &player).unwrap();
        let loaded = load_all_players(db.conn()).unwrap();
        let stored = loaded
            .iter()
            .find(|candidate| candidate.id == "p-rated")
            .expect("stored player should exist");

        assert_eq!(stored.ovr, 78);
        assert_eq!(stored.potential, 85);
    }

    #[test]
    fn test_player_loan_state_roundtrip() {
        let db = test_db();
        let mut player = sample_player("p-loan", Some("team-loan"));
        player.loan_offers.push(LoanOffer {
            id: "loan-offer-1".to_string(),
            from_team_id: "team-loan".to_string(),
            parent_team_id: "team-parent".to_string(),
            start_date: "2026-08-01".to_string(),
            end_date: "2027-01-01".to_string(),
            wage_contribution_pct: 75,
            buy_option_fee: Some(1_250_000),
            last_manager_wage_contribution_pct: None,
            last_manager_end_date: None,
            last_manager_buy_option_fee: None,
            negotiation_round: 1,
            suggested_wage_contribution_pct: None,
            suggested_end_date: None,
            suggested_buy_option_fee: None,
            status: LoanOfferStatus::Accepted,
            date: "2026-08-01".to_string(),
        });
        player.active_loan = Some(ActiveLoan {
            parent_team_id: "team-parent".to_string(),
            loan_team_id: "team-loan".to_string(),
            start_date: "2026-08-01".to_string(),
            end_date: "2027-01-01".to_string(),
            wage_contribution_pct: 75,
            buy_option_fee: Some(1_250_000),
            loan_start_minutes: 120,
            loan_start_appearances: 2,
            development_reported_minutes: 360,
            development_reported_appearances: 4,
        });

        upsert_player(db.conn(), &player).unwrap();
        let loaded = load_all_players(db.conn()).unwrap();
        let stored = loaded
            .iter()
            .find(|candidate| candidate.id == "p-loan")
            .expect("stored player should exist");

        assert_eq!(stored.loan_offers.len(), 1);
        assert_eq!(stored.loan_offers[0].status, LoanOfferStatus::Accepted);
        assert_eq!(stored.loan_offers[0].buy_option_fee, Some(1_250_000));
        assert_eq!(
            stored
                .active_loan
                .as_ref()
                .map(|loan| loan.parent_team_id.as_str()),
            Some("team-parent")
        );
        assert_eq!(
            stored
                .active_loan
                .as_ref()
                .and_then(|loan| loan.buy_option_fee),
            Some(1_250_000)
        );
        assert_eq!(
            stored
                .active_loan
                .as_ref()
                .map(|loan| loan.loan_start_minutes),
            Some(120)
        );
        assert_eq!(
            stored
                .active_loan
                .as_ref()
                .map(|loan| loan.loan_start_appearances),
            Some(2)
        );
        assert_eq!(
            stored
                .active_loan
                .as_ref()
                .map(|loan| loan.development_reported_minutes),
            Some(360)
        );
        assert_eq!(
            stored
                .active_loan
                .as_ref()
                .map(|loan| loan.development_reported_appearances),
            Some(4)
        );
    }

    #[test]
    fn test_player_movement_history_roundtrip() {
        let db = test_db();
        let mut player = sample_player("p-move", Some("team-002"));
        player.movement_history.push(PlayerMovementEntry {
            date: "2026-08-01".to_string(),
            kind: PlayerMovementKind::PermanentTransfer,
            from_team_id: Some("team-001".to_string()),
            from_team_name: Some("Alpha FC".to_string()),
            to_team_id: Some("team-002".to_string()),
            to_team_name: Some("Beta FC".to_string()),
            fee: Some(1_250_000),
            loan_end_date: None,
        });

        upsert_player(db.conn(), &player).unwrap();
        let loaded = load_all_players(db.conn()).unwrap();
        let stored = loaded
            .iter()
            .find(|candidate| candidate.id == "p-move")
            .expect("stored player should exist");

        assert_eq!(stored.movement_history.len(), 1);
        assert_eq!(
            stored.movement_history[0].kind,
            PlayerMovementKind::PermanentTransfer
        );
        assert_eq!(
            stored.movement_history[0].from_team_name.as_deref(),
            Some("Alpha FC")
        );
        assert_eq!(stored.movement_history[0].fee, Some(1_250_000));
    }

    #[test]
    fn test_upsert_players_batch() {
        let db = test_db();
        let players = vec![
            sample_player("p-001", Some("team-001")),
            sample_player("p-002", Some("team-001")),
            sample_player("p-003", Some("team-002")),
        ];

        upsert_players(db.conn(), &players).unwrap();
        let all = load_all_players(db.conn()).unwrap();
        assert_eq!(all.len(), 3);
    }

    // Reproduces the squad-eating bug: when a player arrives at a team and is
    // assigned a jersey number that's already taken, the previous holder must
    // not silently disappear. Persistence should refuse the second write loudly,
    // not delete the first row to make room.
    #[test]
    fn test_assigning_taken_jersey_must_not_delete_previous_holder() {
        let db = test_db();

        let mut original = sample_player("p-original", Some("team-dortmund"));
        original.jersey_number = Some(6);
        upsert_player(db.conn(), &original).unwrap();

        let mut newcomer = sample_player("p-newcomer", Some("team-dortmund"));
        newcomer.jersey_number = Some(6);
        let result = upsert_player(db.conn(), &newcomer);

        assert!(
            result.is_err(),
            "second save into an occupied (team, jersey) slot must fail loudly"
        );

        let all = load_all_players(db.conn()).unwrap();
        let ids: std::collections::HashSet<&str> =
            all.iter().map(|player| player.id.as_str()).collect();
        assert!(
            ids.contains("p-original"),
            "original #6 wearer must still exist in the DB"
        );
    }

    #[test]
    fn test_load_players_by_team() {
        let db = test_db();
        let mut media_player = sample_player("p-001", Some("team-001"));
        media_player.media.face = Some("/assets/worlds/test-world/players/p-001.png".to_string());
        let players = vec![
            media_player,
            sample_player("p-002", Some("team-001")),
            sample_player("p-003", Some("team-002")),
        ];
        upsert_players(db.conn(), &players).unwrap();

        let team1 = load_players_by_team(db.conn(), "team-001").unwrap();
        assert_eq!(team1.len(), 2);
        assert_eq!(
            team1
                .iter()
                .find(|player| player.id == "p-001")
                .unwrap()
                .media
                .face
                .as_deref(),
            Some("/assets/worlds/test-world/players/p-001.png")
        );

        let team2 = load_players_by_team(db.conn(), "team-002").unwrap();
        assert_eq!(team2.len(), 1);
    }

    #[test]
    fn test_player_alternate_positions_roundtrip() {
        let db = test_db();
        let mut player = sample_player("p-001", Some("team-001"));
        player.alternate_positions = vec![Position::DefensiveMidfielder, Position::Striker];

        upsert_player(db.conn(), &player).unwrap();
        let loaded = load_all_players(db.conn()).unwrap();

        assert_eq!(loaded[0].alternate_positions.len(), 2);
        assert_eq!(
            loaded[0].alternate_positions[0],
            Position::DefensiveMidfielder
        );
        assert_eq!(loaded[0].alternate_positions[1], Position::Striker);
    }

    #[test]
    fn test_player_empty_alternate_positions_roundtrip() {
        let db = test_db();
        let player = sample_player("p-001", None);

        upsert_player(db.conn(), &player).unwrap();
        let loaded = load_all_players(db.conn()).unwrap();

        assert!(loaded[0].alternate_positions.is_empty());
    }

    #[test]
    fn test_player_attributes_roundtrip() {
        let db = test_db();
        let player = sample_player("p-001", None);

        upsert_player(db.conn(), &player).unwrap();
        let loaded = load_all_players(db.conn()).unwrap();

        assert_eq!(loaded[0].attributes.pace, 70);
        assert_eq!(loaded[0].attributes.passing, 80);
        assert_eq!(loaded[0].attributes.vision, 78);
    }

    #[test]
    fn test_player_injury_roundtrip() {
        let db = test_db();
        let mut player = sample_player("p-001", None);
        player.injury = Some(Injury {
            name: "Hamstring".to_string(),
            days_remaining: 14,
        });

        upsert_player(db.conn(), &player).unwrap();
        let loaded = load_all_players(db.conn()).unwrap();

        assert!(loaded[0].injury.is_some());
        let injury = loaded[0].injury.as_ref().unwrap();
        assert_eq!(injury.name, "Hamstring");
        assert_eq!(injury.days_remaining, 14);
    }

    #[test]
    fn test_player_no_injury_roundtrip() {
        let db = test_db();
        let player = sample_player("p-001", None);

        upsert_player(db.conn(), &player).unwrap();
        let loaded = load_all_players(db.conn()).unwrap();

        assert!(loaded[0].injury.is_none());
    }

    #[test]
    fn test_player_transfer_flags_roundtrip() {
        let db = test_db();
        let mut player = sample_player("p-001", None);
        player.transfer_listed = true;
        player.loan_listed = true;

        upsert_player(db.conn(), &player).unwrap();
        let loaded = load_all_players(db.conn()).unwrap();

        assert!(loaded[0].transfer_listed);
        assert!(loaded[0].loan_listed);
    }

    #[test]
    fn test_player_stats_roundtrip() {
        let db = test_db();
        let mut player = sample_player("p-001", None);
        player.stats.appearances = 20;
        player.stats.goals = 5;
        player.stats.assists = 8;
        player.stats.shots = 42;
        player.stats.shots_on_target = 21;
        player.stats.passes_completed = 510;
        player.stats.passes_attempted = 612;
        player.stats.tackles_won = 33;
        player.stats.interceptions = 19;
        player.stats.fouls_committed = 14;

        upsert_player(db.conn(), &player).unwrap();
        let loaded = load_all_players(db.conn()).unwrap();

        assert_eq!(loaded[0].stats.appearances, 20);
        assert_eq!(loaded[0].stats.goals, 5);
        assert_eq!(loaded[0].stats.assists, 8);
        assert_eq!(loaded[0].stats.shots, 42);
        assert_eq!(loaded[0].stats.shots_on_target, 21);
        assert_eq!(loaded[0].stats.passes_completed, 510);
        assert_eq!(loaded[0].stats.passes_attempted, 612);
        assert_eq!(loaded[0].stats.tackles_won, 33);
        assert_eq!(loaded[0].stats.interceptions, 19);
        assert_eq!(loaded[0].stats.fouls_committed, 14);
    }

    #[test]
    fn test_legacy_player_stats_defaults_new_fields() {
        let db = test_db();
        let player = sample_player("p-legacy", None);

        upsert_player(db.conn(), &player).unwrap();
        db.conn()
            .execute(
                "UPDATE players SET stats = ?1 WHERE id = ?2",
                params![
                    r#"{"appearances":12,"goals":4,"assists":6,"minutes_played":900}"#,
                    "p-legacy"
                ],
            )
            .unwrap();

        let loaded = load_all_players(db.conn()).unwrap();
        let loaded_player = loaded
            .iter()
            .find(|candidate| candidate.id == "p-legacy")
            .unwrap();

        assert_eq!(loaded_player.stats.appearances, 12);
        assert_eq!(loaded_player.stats.goals, 4);
        assert_eq!(loaded_player.stats.assists, 6);
        assert_eq!(loaded_player.stats.minutes_played, 900);
        assert_eq!(loaded_player.stats.shots, 0);
        assert_eq!(loaded_player.stats.shots_on_target, 0);
        assert_eq!(loaded_player.stats.passes_completed, 0);
        assert_eq!(loaded_player.stats.passes_attempted, 0);
        assert_eq!(loaded_player.stats.tackles_won, 0);
        assert_eq!(loaded_player.stats.interceptions, 0);
        assert_eq!(loaded_player.stats.fouls_committed, 0);
    }

    #[test]
    fn test_player_morale_core_roundtrip() {
        let db = test_db();
        let mut player = sample_player("p-001", Some("team-001"));
        player.morale_core = PlayerMoraleCore {
            manager_trust: 63,
            unresolved_issue: Some(PlayerIssue {
                category: PlayerIssueCategory::PlayingTime,
                severity: 55,
            }),
            recent_treatment: None,
            pending_promise: None,
            talk_cooldown_until: None,
            renewal_state: None,
        };

        upsert_player(db.conn(), &player).unwrap();
        let loaded = load_all_players(db.conn()).unwrap();

        assert_eq!(loaded[0].morale_core.manager_trust, 63);
        assert_eq!(
            loaded[0]
                .morale_core
                .unresolved_issue
                .as_ref()
                .map(|issue| &issue.category),
            Some(&PlayerIssueCategory::PlayingTime)
        );
    }

    #[test]
    fn test_player_granular_identity_roundtrip() {
        let db = test_db();
        let mut player = sample_player("p-identity", Some("team-001"));
        player.natural_position = Position::LeftBack;
        player.alternate_positions = vec![Position::LeftWingBack, Position::CenterBack];
        player.footedness = Footedness::Left;
        player.weak_foot = 3;

        upsert_player(db.conn(), &player).unwrap();
        let loaded = load_all_players(db.conn()).unwrap();

        assert_eq!(loaded[0].natural_position, Position::LeftBack);
        assert_eq!(
            loaded[0].alternate_positions,
            vec![Position::LeftWingBack, Position::CenterBack]
        );
        assert_eq!(loaded[0].footedness, Footedness::Left);
        assert_eq!(loaded[0].weak_foot, 3);
    }

    #[test]
    fn test_player_fitness_roundtrip() {
        let db = test_db();
        let mut player = sample_player("p-001", None);
        player.fitness = 88;

        upsert_player(db.conn(), &player).unwrap();
        let loaded = load_all_players(db.conn()).unwrap();

        assert_eq!(
            loaded[0].fitness, 88,
            "Fitness should round-trip through DB"
        );
    }

    #[test]
    fn test_player_fitness_default_on_new() {
        let db = test_db();
        let player = sample_player("p-001", None);
        assert_eq!(
            player.fitness, 75,
            "New player should start with fitness=75"
        );

        upsert_player(db.conn(), &player).unwrap();
        let loaded = load_all_players(db.conn()).unwrap();
        assert_eq!(loaded[0].fitness, 75);
    }

    #[test]
    fn test_player_retired_roundtrip() {
        let db = test_db();
        let mut player = sample_player("p-retired", None);
        player.retired = true;

        upsert_player(db.conn(), &player).unwrap();
        let loaded = load_all_players(db.conn()).unwrap();
        let stored = loaded
            .iter()
            .find(|candidate| candidate.id == "p-retired")
            .expect("stored player should exist");

        assert!(stored.retired);
    }

    #[test]
    fn test_upsert_player_returns_backend_key_when_schema_is_missing() {
        let conn = Connection::open_in_memory().unwrap();
        let player = sample_player("p-001", None);

        let result = upsert_player(&conn, &player);

        assert_eq!(result.unwrap_err(), GAME_PERSISTENCE_WRITE_ERROR);
    }

    #[test]
    fn test_load_players_returns_backend_key_when_schema_is_missing() {
        let conn = Connection::open_in_memory().unwrap();

        let result = load_all_players(&conn);

        assert_eq!(result.unwrap_err(), GAME_PERSISTENCE_LOAD_ERROR);
    }
}
