//! English text for the translation keys the backend puts on messages and news.
//!
//! The backend writes most inbox messages and articles with empty plain text and a key
//! (`subject_key`, `body_key`, ...) plus the params the sentence needs; the player's screen turns
//! that into words in `src/utils/backendI18n.ts`. An agent reads the same inbox and news, so this
//! module does the same in English (the only language MCP output is written in).
//!
//! `backendI18n.ts` is the reference. `src/utils/backendTextGolden.json` is the list of cases both
//! resolvers are held to, so a change to one that the other does not follow fails CI. What is
//! deliberately not ported: the params the frontend expands out of structured JSON (roundups,
//! standings, scorers, press quotes), a division's or competition's display name, and country
//! names. They stay as the params the backend sent, and the result says so in `details`.

use std::collections::BTreeMap;

use domain::message::{ActionType, InboxMessage};
use domain::news::NewsArticle;
use serde_json::Value;

/// A message as an agent reads it.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedMessage {
    pub subject: String,
    pub body: String,
    pub sender: String,
    pub sender_role: String,
    pub actions: Vec<ResolvedAction>,
    pub details: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedAction {
    pub id: String,
    pub label: String,
    pub kind: &'static str,
    pub resolved: bool,
    pub options: Vec<ResolvedOption>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedOption {
    pub id: String,
    pub label: String,
    pub description: String,
}

/// A news article as an agent reads it.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedArticle {
    pub headline: String,
    pub body: String,
    pub source: String,
    pub details: BTreeMap<String, Value>,
}

pub fn resolve_message(message: &InboxMessage) -> ResolvedMessage {
    ResolvedMessage {
        subject: message.subject.clone(),
        body: message.body.clone(),
        sender: message.sender.clone(),
        sender_role: message.sender_role.clone(),
        actions: message
            .actions
            .iter()
            .map(|action| ResolvedAction {
                id: action.id.clone(),
                label: action.label.clone(),
                kind: match &action.action_type {
                    ActionType::Acknowledge => "acknowledge",
                    ActionType::NavigateTo { .. } => "navigate",
                    ActionType::ChooseOption { .. } => "choose_option",
                    ActionType::Dismiss => "dismiss",
                },
                resolved: action.resolved,
                options: match &action.action_type {
                    ActionType::ChooseOption { options } => options
                        .iter()
                        .map(|option| ResolvedOption {
                            id: option.id.clone(),
                            label: option.label.clone(),
                            description: option.description.clone(),
                        })
                        .collect(),
                    _ => Vec::new(),
                },
            })
            .collect(),
        details: BTreeMap::new(),
    }
}

pub fn resolve_article(article: &NewsArticle) -> ResolvedArticle {
    ResolvedArticle {
        headline: article.headline.clone(),
        body: article.body.clone(),
        source: article.source.clone(),
        details: BTreeMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The cases `backendI18n.ts` wrote, each with the text it made of them.
    const GOLDEN: &str = include_str!("../../../src/utils/backendTextGolden.json");

    /// A message as the wire carries it, with the fields a case leaves out filled in.
    fn message_from(input: &Value) -> InboxMessage {
        let mut wire = json!({
            "id": "m", "subject": "", "body": "", "sender": "", "sender_role": "",
            "date": "2026-08-01", "read": false, "category": "System", "priority": "Normal",
            "actions": [], "context": {}
        });
        merge(&mut wire, input);
        serde_json::from_value(wire).expect("a golden message is a message")
    }

    fn article_from(input: &Value) -> NewsArticle {
        let mut wire = json!({
            "id": "a", "headline": "", "body": "", "source": "", "date": "2026-08-01",
            "category": "Editorial", "team_ids": [], "player_ids": [], "match_score": null,
            "read": false
        });
        merge(&mut wire, input);
        serde_json::from_value(wire).expect("a golden article is an article")
    }

    fn merge(base: &mut Value, over: &Value) {
        if let (Some(base), Some(over)) = (base.as_object_mut(), over.as_object()) {
            for (key, value) in over {
                base.insert(key.clone(), value.clone());
            }
        }
    }

    fn golden_expected_of_message(resolved: &ResolvedMessage) -> Value {
        json!({
            "subject": resolved.subject,
            "body": resolved.body,
            "sender": resolved.sender,
            "sender_role": resolved.sender_role,
            "actions": resolved.actions.iter().map(|action| json!({
                "id": action.id,
                "label": action.label,
                "options": action.options.iter().map(|option| json!({
                    "id": option.id,
                    "label": option.label,
                    "description": option.description,
                })).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
        })
    }

    /// Given the cases `backendI18n.ts` resolved into `backendTextGolden.json`
    /// When this module resolves each input
    /// Then the text is the same, key for key: the two resolvers do not drift apart. A mismatch
    ///      here means either this module changed or the frontend did and the file was
    ///      regenerated (`OFM_UPDATE_BACKEND_TEXT_GOLDEN=1`) without this module following.
    #[test]
    fn the_rust_resolver_writes_the_text_the_frontend_resolver_wrote() {
        let golden: Value = serde_json::from_str(GOLDEN).expect("the golden file is JSON");
        let cases = golden["cases"].as_array().expect("a list of cases");
        assert!(cases.len() >= 10, "the golden file lists its cases");

        let mut mismatches = Vec::new();
        for case in cases {
            let name = case["name"].as_str().unwrap_or("?");
            let actual = match case["kind"].as_str() {
                Some("article") => {
                    let resolved = resolve_article(&article_from(&case["input"]));
                    json!({
                        "headline": resolved.headline,
                        "body": resolved.body,
                        "source": resolved.source,
                    })
                }
                _ => golden_expected_of_message(&resolve_message(&message_from(&case["input"]))),
            };
            if actual != case["expected"] {
                mismatches.push(format!(
                    "{name}\n  rust:     {actual}\n  frontend: {}",
                    case["expected"]
                ));
            }
        }
        assert!(
            mismatches.is_empty(),
            "the Rust resolver and the frontend resolver disagree on {} case(s):\n{}",
            mismatches.len(),
            mismatches.join("\n")
        );
    }
}
