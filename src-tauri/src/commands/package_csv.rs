//! CSV export for world-package entities.
//!
//! Authoring a realistic database means entering hundreds of teams and players,
//! which is what spreadsheets are for. These commands write the editor's current
//! teams and players out as CSV so they can be bulk-edited elsewhere.
//!
//! # Column contract
//!
//! The header row *is* the contract — there is no version number. A future
//! importer matches columns by header name, so column order does not matter,
//! unknown columns can be ignored, and a missing column means "not provided"
//! (the engine generates that field) rather than an error. That keeps files
//! written by an older or newer editor readable both ways, which a version
//! number alone would not.
//!
//! An empty cell always means "omitted", never "zero" or "empty string" — it is
//! how a package says *let the engine decide*, matching how the editor's forms
//! already treat blank fields.

use std::path::Path;

use domain::player::PlayerAttributes;
use ofm_core::generator::{PlayerDef, TeamDef};

const TEAM_HEADERS: &[&str] = &[
    "id",
    "name",
    "shortName",
    "city",
    "country",
    "playStyle",
    "stadiumName",
    "primaryColor",
    "secondaryColor",
    "reputationMin",
    "reputationMax",
    "financeMin",
    "financeMax",
    "kitPattern",
    "logo",
];

const PLAYER_IDENTITY_HEADERS: &[&str] = &[
    "id",
    "firstName",
    "lastName",
    "name",
    "club",
    "nationality",
    "position",
    "dateOfBirth",
    "age",
    "footedness",
    "youth",
    "photo",
    "overall",
];

/// The 19 attribute columns, in the order [`PlayerAttributes`] declares them:
/// physical, technical, mental, then goalkeeping.
const PLAYER_ATTRIBUTE_HEADERS: &[&str] = &[
    "pace",
    "stamina",
    "strength",
    "agility",
    "passing",
    "shooting",
    "tackling",
    "dribbling",
    "defending",
    "positioning",
    "vision",
    "decisions",
    "composure",
    "aggression",
    "teamwork",
    "leadership",
    "handling",
    "reflexes",
    "aerial",
];

fn attribute_values(attributes: &PlayerAttributes) -> Vec<String> {
    vec![
        attributes.pace.to_string(),
        attributes.stamina.to_string(),
        attributes.strength.to_string(),
        attributes.agility.to_string(),
        attributes.passing.to_string(),
        attributes.shooting.to_string(),
        attributes.tackling.to_string(),
        attributes.dribbling.to_string(),
        attributes.defending.to_string(),
        attributes.positioning.to_string(),
        attributes.vision.to_string(),
        attributes.decisions.to_string(),
        attributes.composure.to_string(),
        attributes.aggression.to_string(),
        attributes.teamwork.to_string(),
        attributes.leadership.to_string(),
        attributes.handling.to_string(),
        attributes.reflexes.to_string(),
        attributes.aerial.to_string(),
    ]
}

fn optional<T: ToString>(value: &Option<T>) -> String {
    value.as_ref().map(ToString::to_string).unwrap_or_default()
}

/// Render an enum the way serde writes it into the package JSON, so the column
/// holds exactly the token a reader will deserialize.
///
/// `format!("{value:?}")` produces the same string today, but only because no
/// variant carries a `#[serde(rename)]`. Going through serde means adding one
/// later can't silently desynchronise the CSV from the JSON.
fn serde_token<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|json| json.as_str().map(str::to_string))
        .unwrap_or_default()
}

fn team_row(team: &TeamDef) -> Vec<String> {
    // A range is written as two columns so it can be sorted and filtered in a
    // spreadsheet; absent ranges leave both blank rather than inventing bounds.
    let (reputation_min, reputation_max) = team
        .reputation_range
        .map(|range| (range[0].to_string(), range[1].to_string()))
        .unwrap_or_default();
    let (finance_min, finance_max) = team
        .finance_range
        .map(|range| (range[0].to_string(), range[1].to_string()))
        .unwrap_or_default();

    vec![
        team.id.clone(),
        team.name.clone(),
        team.short_name.clone(),
        team.city.clone(),
        team.country.clone(),
        team.play_style.clone(),
        team.stadium_name.clone(),
        team.colors.primary.clone(),
        team.colors.secondary.clone(),
        reputation_min,
        reputation_max,
        finance_min,
        finance_max,
        optional(&team.kit_pattern),
        optional(&team.logo),
    ]
}

fn player_row(player: &PlayerDef) -> Vec<String> {
    let mut row = vec![
        player.id.clone(),
        player.first_name.clone(),
        player.last_name.clone(),
        player.name.clone(),
        player.club.clone(),
        player.nationality.clone(),
        serde_token(&player.position),
        optional(&player.date_of_birth),
        optional(&player.age),
        optional(&player.footedness),
        // Only written when set, so the common non-youth case stays blank rather
        // than filling a column with "false".
        if player.youth { "true".to_string() } else { String::new() },
        optional(&player.photo),
        optional(&player.overall),
    ];

    // A player carries either a single `overall` or an explicit attribute block.
    // Both are written when both are set; on import the attribute block wins,
    // matching `generate_player_from_def`, which only derives attributes from
    // `overall` when no block is present.
    match &player.attributes {
        Some(attributes) => row.extend(attribute_values(attributes)),
        None => row.extend(std::iter::repeat_n(String::new(), PLAYER_ATTRIBUTE_HEADERS.len())),
    }

    row
}

fn write_csv(output: &str, headers: &[&str], rows: Vec<Vec<String>>) -> Result<(), String> {
    let mut writer =
        csv::Writer::from_path(Path::new(output)).map_err(|_| "be.error.csv.writeFailed".to_string())?;
    writer
        .write_record(headers)
        .map_err(|_| "be.error.csv.writeFailed".to_string())?;
    for row in rows {
        writer
            .write_record(&row)
            .map_err(|_| "be.error.csv.writeFailed".to_string())?;
    }
    writer
        .flush()
        .map_err(|_| "be.error.csv.writeFailed".to_string())
}

/// Write the package's teams to `output` as CSV.
#[tauri::command]
pub fn export_teams_csv(teams: Vec<TeamDef>, output: String) -> Result<(), String> {
    write_csv(&output, TEAM_HEADERS, teams.iter().map(team_row).collect())
}

/// Write the package's players to `output` as CSV. Youth and first-team players
/// share one file, told apart by the `youth` column.
#[tauri::command]
pub fn export_players_csv(players: Vec<PlayerDef>, output: String) -> Result<(), String> {
    let headers: Vec<&str> = PLAYER_IDENTITY_HEADERS
        .iter()
        .chain(PLAYER_ATTRIBUTE_HEADERS)
        .copied()
        .collect();
    write_csv(&output, &headers, players.iter().map(player_row).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn team_def(json: serde_json::Value) -> TeamDef {
        serde_json::from_value(json).expect("team def should deserialize")
    }

    fn player_def(json: serde_json::Value) -> PlayerDef {
        serde_json::from_value(json).expect("player def should deserialize")
    }

    fn base_team(name: &str) -> serde_json::Value {
        serde_json::json!({
            "id": "fc-test",
            "name": name,
            "shortName": "TST",
            "city": "Testville",
            "country": "ENG",
            "colors": { "primary": "#ff0000", "secondary": "#ffffff" }
        })
    }

    #[test]
    fn team_row_splits_ranges_into_two_columns() {
        let mut json = base_team("FC Test");
        json["reputationRange"] = serde_json::json!([100, 200]);
        json["financeRange"] = serde_json::json!([1000, 2000]);
        let row = team_row(&team_def(json));

        assert_eq!(row.len(), TEAM_HEADERS.len());
        assert_eq!(row[9], "100");
        assert_eq!(row[10], "200");
        assert_eq!(row[11], "1000");
        assert_eq!(row[12], "2000");
    }

    #[test]
    fn absent_ranges_leave_both_bounds_blank() {
        let row = team_row(&team_def(base_team("FC Test")));
        // Blank means "omitted" — inventing a bound here would silently pin a
        // reputation the author never chose.
        assert_eq!(row[9], "");
        assert_eq!(row[10], "");
        assert_eq!(row[11], "");
        assert_eq!(row[12], "");
    }

    #[test]
    fn a_player_without_an_attribute_block_leaves_those_columns_blank() {
        let player = player_def(serde_json::json!({
            "id": "p1", "firstName": "Test", "lastName": "Player",
            "club": "fc-test", "nationality": "ENG", "overall": 70
        }));
        let row = player_row(&player);

        assert_eq!(row.len(), PLAYER_IDENTITY_HEADERS.len() + PLAYER_ATTRIBUTE_HEADERS.len());
        assert_eq!(row[12], "70", "overall is written");
        assert!(
            row[PLAYER_IDENTITY_HEADERS.len()..].iter().all(String::is_empty),
            "attribute columns stay blank so import falls back to overall"
        );
    }

    #[test]
    fn an_authored_attribute_block_fills_every_attribute_column() {
        let mut attributes = serde_json::Map::new();
        for (index, key) in PLAYER_ATTRIBUTE_HEADERS.iter().enumerate() {
            attributes.insert((*key).to_string(), serde_json::json!(index + 40));
        }
        let player = player_def(serde_json::json!({
            "id": "p1", "firstName": "Test", "lastName": "Player",
            "club": "fc-test", "nationality": "ENG",
            "attributes": serde_json::Value::Object(attributes)
        }));
        let row = player_row(&player);

        let written = &row[PLAYER_IDENTITY_HEADERS.len()..];
        assert_eq!(written.len(), PLAYER_ATTRIBUTE_HEADERS.len());
        for (index, value) in written.iter().enumerate() {
            assert_eq!(value, &(index + 40).to_string(), "column {index} out of order");
        }
    }

    #[test]
    fn youth_is_blank_unless_the_player_is_a_youth_player() {
        let senior = player_def(serde_json::json!({ "id": "p1", "club": "fc-test" }));
        assert_eq!(player_row(&senior)[10], "");

        let youth = player_def(serde_json::json!({ "id": "p2", "club": "fc-test", "youth": true }));
        assert_eq!(player_row(&youth)[10], "true");
    }

    #[test]
    fn position_is_written_the_way_serde_reads_it() {
        // An importer deserializes this column with serde, so the exported token
        // must be serde's spelling rather than the Debug one that merely
        // coincides with it while no variant is renamed.
        use domain::player::Position;

        for position in [
            Position::Goalkeeper,
            Position::CentralMidfielder,
            Position::Striker,
        ] {
            let expected = serde_json::to_value(&position)
                .expect("position serializes")
                .as_str()
                .expect("position is a string")
                .to_string();
            let player = player_def(serde_json::json!({
                "id": "p1", "club": "fc-test", "position": expected
            }));
            assert_eq!(player_row(&player)[6], expected);
        }
    }

    #[test]
    fn commas_and_quotes_in_names_survive_a_round_trip() {
        // The whole reason for using a real CSV writer: a club whose name holds a
        // comma or a quote must not shift or corrupt the surrounding columns.
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("teams.csv");
        let output = path.to_string_lossy().to_string();

        // Both escaping cases in one name: a comma (which would shift columns)
        // and a double quote (which the writer must double, not drop).
        let teams = vec![team_def(base_team(r#"Preston "North End", Lancashire"#))];
        export_teams_csv(teams, output.clone()).expect("export should succeed");

        let mut reader = csv::Reader::from_path(&path).expect("written file should parse");
        assert_eq!(reader.headers().expect("headers"), TEAM_HEADERS);
        let record = reader.records().next().expect("one row").expect("valid row");
        assert_eq!(&record[1], r#"Preston "North End", Lancashire"#);
        assert_eq!(&record[2], "TST", "the comma must not shift later columns");
    }
}
