//! A division's or competition's display name, in English: `competitionDisplayName` in
//! `src/lib/competitionName.ts`, used by the screen's promotion/relegation and match-report
//! normalizers. A generated competition is stored under an English template name
//! ("Germany Second Division"); the notice or report carries its name key, country, season, tier
//! and phase, so the name reads as every other screen names it.

use domain::message::InboxMessage;
use domain::news::NewsArticle;

use super::{resolve, Params};

const TIER_DIVISION: &str = "tournaments.competitions.tierDivision";
const TIER_TEMPLATES: [&str; 4] = [
    "tournaments.competitions.league",
    "tournaments.competitions.firstDivision",
    "tournaments.competitions.secondDivision",
    "tournaments.competitions.tierDivision",
];

/// A promotion or relegation names the division the club moved to.
pub(super) fn apply_message(message: &InboxMessage, params: &mut Params) {
    let is_division_change = matches!(
        message.subject_key.as_deref(),
        Some("be.msg.promotion.subject" | "be.msg.relegation.subject")
    );
    if !is_division_change {
        return;
    }
    let Some(name_key) = params.get("divisionKey").filter(|key| !key.is_empty()) else {
        return;
    };

    let tier = positive_tier(params.get("divisionTier"));
    let season = params
        .get("divisionSeason")
        .and_then(|season| season.parse::<u32>().ok());
    let phase = params
        .get("divisionPhase")
        .and_then(|phase| phase_key(phase));
    let name = display_name(
        params.get("division").map(String::as_str),
        name_key,
        params.get("divisionCountry").map(String::as_str),
        season,
        tier,
        phase,
    );
    params.insert("division".to_string(), name);
}

/// A competition match report names the competition it was played in.
pub(super) fn apply_article(article: &NewsArticle, params: &mut Params) {
    if article.body_key.as_deref() != Some("be.news.matchReport.reportCompetition.body") {
        return;
    }
    let Some(name_key) = params.get("competitionKey").filter(|key| !key.is_empty()) else {
        return;
    };

    let season = params
        .get("competitionSeason")
        .and_then(|season| season.parse::<u32>().ok());
    let name = display_name(
        params.get("competition").map(String::as_str),
        name_key,
        params.get("competitionCountry").map(String::as_str),
        season,
        // The screen's report normalizer passes neither a tier nor a phase, so a generic tier
        // template keeps the stored name.
        None,
        None,
    );
    params.insert("competition".to_string(), name);
}

/// The name the screens show for `name_key`, or the stored `name` when it cannot be built.
fn display_name(
    name: Option<&str>,
    name_key: &str,
    country_id: Option<&str>,
    season: Option<u32>,
    tier: Option<u32>,
    phase: Option<&'static str>,
) -> String {
    let stored = name.unwrap_or_default();
    if name_key.is_empty() {
        return stored.to_string();
    }

    let country = country_id
        .map(|country_id| {
            let key = format!("nations.{}", country_id.to_lowercase());
            resolve(Some(&key), country_id, &Params::new())
        })
        .unwrap_or_default();

    // A tier template needs its tier; without one the stored name is all that can be said.
    if name_key == TIER_DIVISION && tier.is_none() {
        return stored.to_string();
    }

    let mut params = Params::new();
    if let Some(season) = season {
        params.insert("year".to_string(), season.to_string());
    }
    if let Some(tier) = tier {
        params.insert("tier".to_string(), tier.to_string());
    }
    // The screen always passes the country, empty when it has none, so a blank slot is not left.
    params.insert("country".to_string(), country);

    let localized = resolve(Some(name_key), stored, &params).trim().to_string();

    let phase = phase.filter(|_| TIER_TEMPLATES.contains(&name_key));
    match phase {
        Some("opening") => {
            let suffix = resolve(
                Some("tournaments.competitions.phaseOpening"),
                "",
                &Params::new(),
            );
            format!("{localized} {suffix}").trim().to_string()
        }
        Some("closing") => {
            let suffix = resolve(
                Some("tournaments.competitions.phaseClosing"),
                "",
                &Params::new(),
            );
            format!("{localized} {suffix}").trim().to_string()
        }
        _ => localized,
    }
}

fn positive_tier(value: Option<&String>) -> Option<u32> {
    value
        .and_then(|tier| tier.parse::<u32>().ok())
        .filter(|tier| *tier > 0)
}

/// The serialized season phase, as the backend writes it, mapped to its suffixed name.
fn phase_key(phase: &str) -> Option<&'static str> {
    match phase {
        "opening" => Some("opening"),
        "closing" => Some("closing"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn message(input: serde_json::Value) -> InboxMessage {
        let mut wire = json!({
            "id": "m", "subject": "", "body": "", "sender": "", "sender_role": "",
            "date": "2026-08-01", "read": false, "category": "System", "priority": "Normal",
            "actions": [], "context": {}
        });
        for (key, value) in input.as_object().expect("an object") {
            wire[key] = value.clone();
        }
        serde_json::from_value(wire).expect("a message")
    }

    /// Given a generated second-tier division with a phase
    /// When promotion params are localized
    /// Then the notice names it the way the screen does, phase included.
    #[test]
    fn a_promotion_names_its_second_tier_division_in_english() {
        let message = message(json!({
            "subject_key": "be.msg.promotion.subject",
            "body_key": "be.msg.promotion.body",
        }));
        let mut params: Params = [
            ("division".to_string(), "Germany Division 2".to_string()),
            ("divisionKey".to_string(), TIER_DIVISION.to_string()),
            ("divisionCountry".to_string(), "DE".to_string()),
            ("divisionSeason".to_string(), "2032".to_string()),
            ("divisionTier".to_string(), "2".to_string()),
            ("divisionPhase".to_string(), "opening".to_string()),
        ]
        .into_iter()
        .collect();

        apply_message(&message, &mut params);

        assert_eq!(
            params.get("division").map(String::as_str),
            Some("Germany Division 2 Opening tournament")
        );
    }

    /// Given a generic tier key with no tier to fill it
    /// When the division name is localized
    /// Then the stored name stands, as it does on the screen.
    #[test]
    fn a_generic_tier_without_a_tier_keeps_the_stored_name() {
        let message = message(json!({
            "subject_key": "be.msg.relegation.subject",
            "body_key": "be.msg.relegation.body",
        }));
        let mut params: Params = [
            ("division".to_string(), "Germany Division 3".to_string()),
            ("divisionKey".to_string(), TIER_DIVISION.to_string()),
            ("divisionCountry".to_string(), "DE".to_string()),
        ]
        .into_iter()
        .collect();

        apply_message(&message, &mut params);

        assert_eq!(
            params.get("division").map(String::as_str),
            Some("Germany Division 3")
        );
    }
}
