use domain::player::{Footedness, Player, PlayerAttributes, Position, SquadRole};
use domain::team::TrainingFocus;
use rusqlite::{Connection, params};

/// Insert or replace a player row.
pub fn upsert_player(conn: &Connection, p: &Player) -> Result<(), String> {
    let attrs_json =
        serde_json::to_string(&p.attributes).map_err(|e| format!("JSON error: {}", e))?;
    let injury_json = p
        .injury
        .as_ref()
        .map(|i| serde_json::to_string(i).unwrap_or_default());
    let traits_json = serde_json::to_string(&p.traits).map_err(|e| format!("JSON error: {}", e))?;
    let stats_json = serde_json::to_string(&p.stats).map_err(|e| format!("JSON error: {}", e))?;
    let career_json = serde_json::to_string(&p.career).map_err(|e| format!("JSON error: {}", e))?;
    let offers_json =
        serde_json::to_string(&p.transfer_offers).map_err(|e| format!("JSON error: {}", e))?;
    let morale_core_json =
        serde_json::to_string(&p.morale_core).map_err(|e| format!("JSON error: {}", e))?;
    let position_str = format!("{:?}", p.position);
    let natural_position_str = format!("{:?}", p.natural_position);
    let alt_positions_json =
        serde_json::to_string(&p.alternate_positions).map_err(|e| format!("JSON error: {}", e))?;
    let footedness_str = format!("{:?}", p.footedness);
    let training_focus_str: Option<String> = p.training_focus.as_ref().map(|f| format!("{:?}", f));

    conn.execute(
        "INSERT OR REPLACE INTO players
         (id, match_name, full_name, date_of_birth, nationality, football_nation, birth_country, position,
          attributes, condition, morale, injury, team_id, traits,
          contract_end, wage, market_value, stats, career,
          transfer_listed, loan_listed, transfer_offers, alternate_positions,
          natural_position, training_focus, morale_core, footedness, weak_foot, fitness, squad_role,
          ovr, potential)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32)",
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
        ],
    )
    .map_err(|e| format!("Failed to upsert player: {}", e))?;
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
                    attributes, condition, morale, injury, team_id, traits,
                    contract_end, wage, market_value, stats, career,
                    transfer_listed, loan_listed, transfer_offers, alternate_positions,
                    natural_position, training_focus, morale_core, footedness, weak_foot, fitness, squad_role,
                    ovr, potential
             FROM players",
        )
        .map_err(|e| format!("Failed to prepare players query: {}", e))?;

    let rows = stmt
        .query_map([], row_to_player)
        .map_err(|e| format!("Failed to query players: {}", e))?;

    let mut players = Vec::new();
    for row in rows {
        players.push(row.map_err(|e| format!("Failed to read player row: {}", e))?);
    }
    Ok(players)
}

/// Load players by team id.
pub fn load_players_by_team(conn: &Connection, team_id: &str) -> Result<Vec<Player>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, match_name, full_name, date_of_birth, nationality, football_nation, birth_country, position,
                    attributes, condition, morale, injury, team_id, traits,
                    contract_end, wage, market_value, stats, career,
                    transfer_listed, loan_listed, transfer_offers, alternate_positions,
                    natural_position, training_focus, morale_core, footedness, weak_foot, fitness, squad_role,
                    ovr, potential
             FROM players WHERE team_id = ?1",
        )
        .map_err(|e| format!("Failed to prepare players query: {}", e))?;

    let rows = stmt
        .query_map(params![team_id], row_to_player)
        .map_err(|e| format!("Failed to query players: {}", e))?;

    let mut players = Vec::new();
    for row in rows {
        players.push(row.map_err(|e| format!("Failed to read player row: {}", e))?);
    }
    Ok(players)
}

fn row_to_player(row: &rusqlite::Row) -> rusqlite::Result<Player> {
    let position_str: String = row.get(7)?;
    let attrs_json: String = row.get(8)?;
    let injury_json: Option<String> = row.get(11)?;
    let traits_json: String = row.get(13)?;
    let stats_json: String = row.get(17)?;
    let career_json: String = row.get(18)?;
    let offers_json: String = row.get(21)?;
    let alt_positions_json: String = row.get(22)?;
    let natural_position_str: String = row.get(23)?;
    let training_focus_str: Option<String> = row.get(24)?;
    let morale_core_json: String = row.get(25)?;
    let footedness_str: String = row.get(26)?;
    let weak_foot: u8 = row.get(27)?;
    let fitness: u8 = row.get(28).unwrap_or(75); // default 75 for saves before V13
    let squad_role_str: String = row.get(29).unwrap_or_else(|_| "Senior".to_string());
    let ovr: u8 = row.get::<_, i64>(30).unwrap_or(0) as u8; // default 0 for saves before V20
    let potential: u8 = row.get::<_, i64>(31).unwrap_or(0) as u8; // default 0 for saves before V20
    let transfer_listed_int: i32 = row.get(19)?;
    let loan_listed_int: i32 = row.get(20)?;
    let market_value_i64: i64 = row.get(16)?;

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
        squad_role: parse_squad_role(&squad_role_str),
        traits: serde_json::from_str(&traits_json).unwrap_or_default(),
        ovr,
        potential,
        contract_end: row.get(14)?,
        wage: row.get(15)?,
        market_value: market_value_i64 as u64,
        stats: serde_json::from_str(&stats_json).unwrap_or_default(),
        career: serde_json::from_str(&career_json).unwrap_or_default(),
        training_focus: training_focus_str.and_then(|s| parse_training_focus(&s)),
        transfer_listed: transfer_listed_int != 0,
        loan_listed: loan_listed_int != 0,
        transfer_offers: serde_json::from_str(&offers_json).unwrap_or_default(),
        morale_core: serde_json::from_str(&morale_core_json).unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_database::GameDatabase;
    use domain::player::{Injury, PlayerIssue, PlayerIssueCategory, PlayerMoraleCore};

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

    #[test]
    fn test_load_players_by_team() {
        let db = test_db();
        let players = vec![
            sample_player("p-001", Some("team-001")),
            sample_player("p-002", Some("team-001")),
            sample_player("p-003", Some("team-002")),
        ];
        upsert_players(db.conn(), &players).unwrap();

        let team1 = load_players_by_team(db.conn(), "team-001").unwrap();
        assert_eq!(team1.len(), 2);

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
}
