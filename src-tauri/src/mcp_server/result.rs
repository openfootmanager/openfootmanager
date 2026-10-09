//! How a tool call fails. One value, [`Failure`], produces both the readable
//! text and the `structuredContent`, so the two cannot drift apart.

use rmcp::model::{CallToolResult, ContentBlock};
use serde_json::{json, Map, Value};

use crate::mcp_server::formatting::translate_error;

const MISSING_PARAMETER_KEY: &str = "be.error.mcp.missingParameter";
const INVALID_PARAMETER_KEY: &str = "be.error.mcp.invalidParameter";

pub struct Failure {
    key: Option<String>,
    params: Map<String, Value>,
    message: String,
}

impl Failure {
    /// A backend error as the application layer reports it: a `be.error.*` key,
    /// optionally followed by `?name=value&...`. Anything else is prose private to
    /// the MCP layer, which has no key to offer.
    pub fn from_backend_error(error: &str) -> Self {
        let (key, query) = error.split_once('?').unwrap_or((error, ""));
        if !key.starts_with("be.") {
            return Self {
                key: None,
                params: Map::new(),
                message: error.to_string(),
            };
        }
        Self {
            key: Some(key.to_string()),
            params: query_params(query),
            message: translate_error(key),
        }
    }

    pub fn missing_parameter(name: &str) -> Self {
        Self::about_parameter(
            MISSING_PARAMETER_KEY,
            name,
            format!("Missing required parameter: {name}"),
        )
    }

    pub fn invalid_parameter(name: &str, problem: &str) -> Self {
        Self::about_parameter(
            INVALID_PARAMETER_KEY,
            name,
            format!("Parameter {name} {problem}"),
        )
    }

    fn about_parameter(key: &str, name: &str, message: String) -> Self {
        let mut params = Map::new();
        params.insert("parameter".to_string(), json!(name));
        Self {
            key: Some(key.to_string()),
            params,
            message,
        }
    }

    pub fn into_result(self) -> CallToolResult {
        let structured = json!({
            "error": { "key": self.key, "params": self.params, "message": self.message }
        });
        let mut result = CallToolResult::structured_error(structured);
        result.content = vec![ContentBlock::text(self.message)];
        result
    }
}

fn query_params(query: &str) -> Map<String, Value> {
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .map(|(name, value)| (name.to_string(), json!(percent_decoded(value))))
        .collect()
}

/// Inverse of the encoding `first_package_error_message` applies to its parameters.
fn percent_decoded(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let escaped = (bytes[i] == b'%')
            .then(|| {
                value
                    .get(i + 1..i + 3)
                    .and_then(|hex| u8::from_str_radix(hex, 16).ok())
            })
            .flatten();
        match escaped {
            Some(byte) => {
                decoded.push(byte);
                i += 3;
            }
            None => {
                decoded.push(bytes[i]);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn structured(failure: Failure) -> (Value, String, Option<bool>) {
        let result = failure.into_result();
        let text = result.content[0]
            .as_text()
            .expect("text block")
            .text
            .clone();
        (
            result.structured_content.expect("structured"),
            text,
            result.is_error,
        )
    }

    /// Given a refusal carrying a backend key
    /// When it becomes a tool result
    /// Then it is an error, the structure holds the key, and the text is the readable form.
    #[test]
    fn an_error_names_its_key() {
        let (data, text, is_error) =
            structured(Failure::from_backend_error("be.error.noActiveGameSession"));

        assert_eq!(is_error, Some(true));
        assert_eq!(data["error"]["key"], "be.error.noActiveGameSession");
        assert_eq!(text, "No active game session. Start or load a game first.");
    }

    /// Given a key with query parameters
    /// When it becomes a tool result
    /// Then the key and the decoded parameters are separate fields.
    #[test]
    fn a_keys_parameters_become_fields() {
        let (data, _, _) = structured(Failure::from_backend_error(
            "be.error.package.unknownCountry?country=C%C3%B4te%20d%27Ivoire&n=2",
        ));

        assert_eq!(data["error"]["key"], "be.error.package.unknownCountry");
        assert_eq!(data["error"]["params"]["country"], "Côte d'Ivoire");
        assert_eq!(data["error"]["params"]["n"], "2");
    }

    /// Given a failure private to the MCP layer, in prose
    /// When it becomes a tool result
    /// Then the key is null and the prose is the message.
    #[test]
    fn private_prose_has_no_key() {
        let (data, text, _) = structured(Failure::from_backend_error("Unknown position: XX"));

        assert!(data["error"]["key"].is_null());
        assert_eq!(text, "Unknown position: XX");
    }

    /// Given a call missing a required parameter
    /// When it is refused
    /// Then the error names the parameter as data.
    #[test]
    fn a_missing_parameter_is_named() {
        let (data, text, is_error) = structured(Failure::missing_parameter("team_id"));

        assert_eq!(is_error, Some(true));
        assert_eq!(data["error"]["key"], "be.error.mcp.missingParameter");
        assert_eq!(data["error"]["params"]["parameter"], "team_id");
        assert_eq!(text, "Missing required parameter: team_id");
    }
}
