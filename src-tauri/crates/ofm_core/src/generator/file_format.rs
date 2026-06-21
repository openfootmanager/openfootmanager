//! Format-agnostic deserialisation for definition and data files.
//!
//! A definition may be authored as JSON or YAML — YAML is friendlier to write
//! by hand — and the loader accepts either: by file extension when a path is
//! known, or by trying JSON first and falling back to YAML for raw text.

use serde::de::DeserializeOwned;
use std::path::Path;

/// Parse definition text that may be JSON or YAML. JSON is attempted first (for
/// precise errors and speed); YAML, a JSON superset, is the fallback. The error
/// string is a debug detail for logs — callers map it to a localized key.
pub fn parse_definition_str<T: DeserializeOwned>(text: &str) -> Result<T, String> {
    match serde_json::from_str::<T>(text) {
        Ok(value) => Ok(value),
        Err(json_error) => serde_yaml::from_str::<T>(text).map_err(|yaml_error| {
            format!("not valid JSON ({json_error}) or YAML ({yaml_error})")
        }),
    }
}

/// Load a definition file, honouring a `.json`/`.yaml`/`.yml` extension and
/// sniffing the content otherwise. Returns `None` on any read or parse error.
pub fn load_definition_file<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let text = std::fs::read_to_string(path).ok()?;
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("yaml") | Some("yml") => serde_yaml::from_str(&text).ok(),
        Some("json") => serde_json::from_str(&text).ok(),
        _ => parse_definition_str(&text).ok(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::CompetitionDefinitionFile;

    const JSON: &str = r#"{
        "formatVersion": 1,
        "competitions": [
            {
                "id": "tr-1",
                "name": "Super Lig",
                "type": "League",
                "scope": "Domestic",
                "format": { "kind": "LeagueTable" },
                "participants": { "selector": { "kind": "allInCountry", "country": "TR" } }
            }
        ]
    }"#;

    const YAML: &str = "
formatVersion: 1
competitions:
  - id: tr-1
    name: Super Lig
    type: League
    scope: Domestic
    format:
      kind: LeagueTable
    participants:
      selector:
        kind: allInCountry
        country: TR
";

    #[test]
    fn parses_equivalent_json_and_yaml_into_the_same_definition() {
        let from_json: CompetitionDefinitionFile = parse_definition_str(JSON).unwrap();
        let from_yaml: CompetitionDefinitionFile = parse_definition_str(YAML).unwrap();

        assert_eq!(from_json.competitions.len(), 1);
        assert_eq!(from_yaml.competitions.len(), 1);
        assert_eq!(from_json.competitions[0].id, from_yaml.competitions[0].id);
        assert_eq!(from_yaml.competitions[0].name, "Super Lig");
        assert_eq!(
            from_yaml.competitions[0]
                .participants
                .selector
                .as_ref()
                .unwrap()
                .country
                .as_deref(),
            Some("TR")
        );
    }

    #[test]
    fn rejects_text_that_is_neither_json_nor_yaml() {
        let result: Result<CompetitionDefinitionFile, _> =
            parse_definition_str("competitions: [ this: is: broken");
        assert!(result.is_err());
    }

    #[test]
    fn load_definition_file_honours_the_extension() {
        let dir = std::env::temp_dir().join(format!("ofm-fmt-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();

        let yaml_path = dir.join("defs.yaml");
        std::fs::write(&yaml_path, YAML).unwrap();
        let json_path = dir.join("defs.json");
        std::fs::write(&json_path, JSON).unwrap();

        let from_yaml: CompetitionDefinitionFile = load_definition_file(&yaml_path).unwrap();
        let from_json: CompetitionDefinitionFile = load_definition_file(&json_path).unwrap();
        assert_eq!(from_yaml.competitions[0].id, "tr-1");
        assert_eq!(from_json.competitions[0].id, "tr-1");

        std::fs::remove_dir_all(&dir).ok();
    }
}
