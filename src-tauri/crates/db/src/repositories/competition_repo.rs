use domain::league::{
    CompetitionRules, CompetitionScope, CompetitionState, CompetitionType, Fixture, GroupState,
    KnockoutRoundState, StandingEntry,
};
use rusqlite::{Connection, params};

const GAME_PERSISTENCE_LOAD_ERROR: &str = "be.error.gamePersistence.loadFailed";
const GAME_PERSISTENCE_WRITE_ERROR: &str = "be.error.gamePersistence.writeFailed";

fn competition_type_to_string(kind: &CompetitionType) -> &'static str {
    match kind {
        CompetitionType::League => "League",
        CompetitionType::Cup => "Cup",
        CompetitionType::ContinentalClub => "ContinentalClub",
        CompetitionType::InternationalClub => "InternationalClub",
        CompetitionType::InternationalNation => "InternationalNation",
        CompetitionType::FriendlyCup => "FriendlyCup",
    }
}

fn competition_scope_to_string(scope: &CompetitionScope) -> &'static str {
    match scope {
        CompetitionScope::Domestic => "Domestic",
        CompetitionScope::Regional => "Regional",
        CompetitionScope::Continental => "Continental",
        CompetitionScope::International => "International",
    }
}

fn parse_competition_type(value: &str) -> CompetitionType {
    match value {
        "Cup" => CompetitionType::Cup,
        "ContinentalClub" => CompetitionType::ContinentalClub,
        "InternationalClub" => CompetitionType::InternationalClub,
        "InternationalNation" => CompetitionType::InternationalNation,
        "FriendlyCup" => CompetitionType::FriendlyCup,
        _ => CompetitionType::League,
    }
}

fn parse_competition_scope(value: &str) -> CompetitionScope {
    match value {
        "Regional" => CompetitionScope::Regional,
        "Continental" => CompetitionScope::Continental,
        "International" => CompetitionScope::International,
        _ => CompetitionScope::Domestic,
    }
}

pub fn replace_competitions(conn: &Connection, competitions: &[CompetitionState]) -> Result<(), String> {
    conn.execute("DELETE FROM competitions", [])
        .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;

    for competition in competitions {
        let required_region_ids_json = serde_json::to_string(&competition.required_region_ids)
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
        let participant_ids_json = serde_json::to_string(&competition.participant_ids)
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
        let rules_json = serde_json::to_string(&competition.rules)
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
        let fixtures_json = serde_json::to_string(&competition.fixtures)
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
        let standings_json = serde_json::to_string(&competition.standings)
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
        let groups_json = serde_json::to_string(&competition.groups)
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
        let knockout_rounds_json = serde_json::to_string(&competition.knockout_rounds)
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
        let transfer_log_json = serde_json::to_string(&competition.transfer_log)
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
        let transfer_rumours_json = serde_json::to_string(&competition.transfer_rumours)
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
        let berths_json = serde_json::to_string(&competition.berths)
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;

        conn.execute(
            "INSERT INTO competitions (id, name, kind, scope, season, region_id, country_id, required_region_ids_json, participant_ids_json, rules_json, fixtures_json, standings_json, groups_json, knockout_rounds_json, transfer_log_json, transfer_rumours_json, priority, berths_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            params![
                competition.id,
                competition.name,
                competition_type_to_string(&competition.kind),
                competition_scope_to_string(&competition.scope),
                competition.season,
                competition.region_id,
                competition.country_id,
                required_region_ids_json,
                participant_ids_json,
                rules_json,
                fixtures_json,
                standings_json,
                groups_json,
                knockout_rounds_json,
                transfer_log_json,
                transfer_rumours_json,
                competition.priority,
                berths_json,
            ],
        )
        .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    }

    Ok(())
}

pub fn load_competitions(conn: &Connection) -> Result<Vec<CompetitionState>, String> {
    let mut stmt = match conn.prepare(
        "SELECT id, name, kind, scope, season, region_id, country_id, required_region_ids_json, participant_ids_json, rules_json, fixtures_json, standings_json, groups_json, knockout_rounds_json, transfer_log_json, transfer_rumours_json, priority, berths_json
         FROM competitions
         ORDER BY priority ASC, season DESC, name ASC",
    ) {
        Ok(stmt) => stmt,
        Err(_) => return Ok(Vec::new()),
    };

    let rows = stmt
        .query_map([], |row| {
            let required_region_ids_json: String = row.get(7)?;
            let participant_ids_json: String = row.get(8)?;
            let rules_json: String = row.get(9)?;
            let fixtures_json: String = row.get(10)?;
            let standings_json: String = row.get(11)?;
            let groups_json: String = row.get(12)?;
            let knockout_rounds_json: String = row.get(13)?;
            let transfer_log_json: String = row.get(14)?;
            let transfer_rumours_json: String = row.get(15)?;
            let berths_json: String = row.get(17)?;

            Ok(CompetitionState {
                id: row.get(0)?,
                name: row.get(1)?,
                kind: parse_competition_type(&row.get::<_, String>(2)?),
                scope: parse_competition_scope(&row.get::<_, String>(3)?),
                season: row.get(4)?,
                region_id: row.get(5)?,
                country_id: row.get(6)?,
                required_region_ids: serde_json::from_str(&required_region_ids_json)
                    .unwrap_or_default(),
                participant_ids: serde_json::from_str(&participant_ids_json).unwrap_or_default(),
                rules: serde_json::from_str::<CompetitionRules>(&rules_json).unwrap_or_default(),
                fixtures: serde_json::from_str::<Vec<Fixture>>(&fixtures_json).unwrap_or_default(),
                standings: serde_json::from_str::<Vec<StandingEntry>>(&standings_json)
                    .unwrap_or_default(),
                groups: serde_json::from_str::<Vec<GroupState>>(&groups_json).unwrap_or_default(),
                knockout_rounds: serde_json::from_str::<Vec<KnockoutRoundState>>(
                    &knockout_rounds_json,
                )
                .unwrap_or_default(),
                transfer_log: serde_json::from_str(&transfer_log_json).unwrap_or_default(),
                transfer_rumours: serde_json::from_str(&transfer_rumours_json).unwrap_or_default(),
                priority: row.get::<_, i64>(16).unwrap_or_default() as u32,
                berths: serde_json::from_str(&berths_json).unwrap_or_default(),
            })
        })
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let mut competitions = Vec::new();
    for row in rows {
        competitions.push(row.map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?);
    }
    Ok(competitions)
}
