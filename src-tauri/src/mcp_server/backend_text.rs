//! English text for the translation keys the backend puts on messages and news.
//!
//! The backend writes most inbox messages and articles with empty plain text and a key
//! (`subject_key`, `body_key`, ...) plus the params the sentence needs; the player's screen turns
//! that into words in `src/utils/backendI18n.ts`. An agent reads the same inbox and news, so this
//! module does the same in English (the only language MCP output is written in).
//!
//! `backendI18n.ts` is the reference. `src/utils/backendTextGolden.json` is the list of cases both
//! resolvers are held to, so a change to one that the other does not follow fails CI. A news
//! article's structured params (roundups, standings, scorers, press quotes, deals) are expanded
//! into the lines of its body in [`news_params`]. What is deliberately not ported: a division's or
//! competition's display name and country names. They stay as the params the backend sent, and the
//! result says so in `details`.

mod news_params;

use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;

use chrono::{Datelike, NaiveDate};
use domain::message::{ActionOption, ActionType, InboxMessage, MessageAction};
use domain::news::NewsArticle;
use serde_json::Value;

/// The English locale file the player's screen reads, so the two cannot name a key differently.
const ENGLISH: &str = include_str!("../../../src/i18n/locales/en.json");

type Params = HashMap<String, String>;

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

/// The English sentence for a backend error `key` with its `params`, as the screen words it
/// (`resolveBackendError`): the locale file's text, params filled in, a param that is itself a key
/// in words. `None` when the locale file has no text for `key`.
pub fn resolve_error(key: &str, params: &HashMap<String, String>) -> Option<String> {
    template_of(Some(key)).map(|template| interpolate(template, &resolve_params(params), false))
}

pub fn resolve_message(message: &InboxMessage) -> ResolvedMessage {
    let params = resolve_params(&message.i18n_params);
    let details = structured_details(&message.i18n_params);
    ResolvedMessage {
        subject: resolve(message.subject_key.as_deref(), &message.subject, &params),
        body: resolve_body(
            message.body_key.as_deref(),
            &message.body,
            &params,
            &details,
        ),
        sender: resolve(message.sender_key.as_deref(), &message.sender, &params),
        sender_role: resolve(
            message.sender_role_key.as_deref(),
            &message.sender_role,
            &params,
        ),
        actions: message
            .actions
            .iter()
            .map(|action| resolve_action(action, &message.id, &params))
            .collect(),
        details,
    }
}

pub fn resolve_article(article: &NewsArticle) -> ResolvedArticle {
    // The screen expands the structured params first, then resolves every param value.
    let params = resolve_params(&news_params::expanded_params(article));
    let details = structured_details(&article.i18n_params);
    ResolvedArticle {
        headline: resolve(article.headline_key.as_deref(), &article.headline, &params),
        body: resolve_body(
            article.body_key.as_deref(),
            &article.body,
            &params,
            &details,
        ),
        source: resolve(article.source_key.as_deref(), &article.source, &params),
        details,
    }
}

fn resolve_action(action: &MessageAction, message_id: &str, params: &Params) -> ResolvedAction {
    let label_key = action
        .label_key
        .clone()
        .or_else(|| inferred_action_label_key(message_id, &action.id));
    let (kind, options) = match &action.action_type {
        ActionType::Acknowledge => ("acknowledge", Vec::new()),
        ActionType::NavigateTo { .. } => ("navigate", Vec::new()),
        ActionType::Dismiss => ("dismiss", Vec::new()),
        ActionType::ChooseOption { options } => (
            "choose_option",
            options
                .iter()
                .map(|option| resolve_option(option, message_id, params))
                .collect(),
        ),
    };
    ResolvedAction {
        id: action.id.clone(),
        label: resolve(label_key.as_deref(), &action.label, params),
        kind,
        resolved: action.resolved,
        options,
    }
}

fn resolve_option(option: &ActionOption, message_id: &str, params: &Params) -> ResolvedOption {
    let inferred = inferred_option_base_key(message_id, &option.id);
    let label_key = option
        .label_key
        .clone()
        .or_else(|| inferred.as_ref().map(|base| format!("{base}.label")));
    let description_key = option
        .description_key
        .clone()
        .or_else(|| inferred.as_ref().map(|base| format!("{base}.description")));
    ResolvedOption {
        id: option.id.clone(),
        label: resolve(label_key.as_deref(), &option.label, params),
        description: resolve(description_key.as_deref(), &option.description, params),
    }
}

// ─── Player events saved before their actions carried keys ─────────────────

/// The group a legacy player-event message belongs to, from its id's prefix
/// (`backendI18nPlayerEvents.ts`).
fn player_event_group(message_id: &str) -> Option<&'static str> {
    [
        ("morale_talk_", "moraleCrisis"),
        ("bench_complaint_", "benchComplaint"),
        ("happy_player_", "happyPlayer"),
        ("contract_concern_", "contractConcern"),
    ]
    .into_iter()
    .find_map(|(prefix, group)| message_id.starts_with(prefix).then_some(group))
}

fn inferred_action_label_key(message_id: &str, action_id: &str) -> Option<String> {
    (player_event_group(message_id).is_some() && action_id == "respond")
        .then(|| "be.msg.playerEvent.respond".to_string())
}

fn inferred_option_base_key(message_id: &str, option_id: &str) -> Option<String> {
    let group = player_event_group(message_id)?;
    let option = match option_id {
        "encourage" => "encourage",
        "promise_time" => "promiseTime",
        "work_harder" => "workHarder",
        "explain" => "explain",
        "promise_chance" => "promiseChance",
        "prove_yourself" => "proveYourself",
        "praise_back" => "praiseBack",
        "stay_professional" => "stayProfessional",
        "higher_expectations" => "higherExpectations",
        "reassure" => "reassure",
        "noncommittal" => "noncommittal",
        "no_renewal" => "noRenewal",
        _ => return None,
    };
    Some(format!("be.msg.playerEvent.options.{group}.{option}"))
}

// ─── Keys, templates and params ─────────────────────────────────────────────

fn english() -> &'static HashMap<String, String> {
    static TABLE: OnceLock<HashMap<String, String>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = HashMap::new();
        if let Ok(root) = serde_json::from_str::<Value>(ENGLISH) {
            flatten("", &root, &mut table);
        }
        table
    })
}

/// The locale file's nested objects as dotted keys, the way i18next addresses them.
fn flatten(prefix: &str, node: &Value, table: &mut HashMap<String, String>) {
    match node {
        Value::String(text) => {
            table.insert(prefix.to_string(), text.clone());
        }
        Value::Object(children) => {
            for (name, child) in children {
                let path = if prefix.is_empty() {
                    name.clone()
                } else {
                    format!("{prefix}.{name}")
                };
                flatten(&path, child, table);
            }
        }
        _ => {}
    }
}

/// The text of `key` with `params` filled in, or `fallback` when there is no key or no text for
/// it. A placeholder the params do not supply is left as written, as i18next leaves it.
fn resolve(key: Option<&str>, fallback: &str, params: &Params) -> String {
    match template_of(key) {
        Some(template) => interpolate(template, params, false),
        None => fallback.to_string(),
    }
}

/// A body: as [`resolve`], except that when the message or article carries structured `details`
/// (the lists the screen would lay out itself), the template's own slots the params do not fill
/// are dropped, since the details hold what they stood for. Only the template's: what a value
/// contains, braces included, is never read.
fn resolve_body(
    key: Option<&str>,
    fallback: &str,
    params: &Params,
    details: &BTreeMap<String, Value>,
) -> String {
    match template_of(key) {
        Some(template) => interpolate(template, params, !details.is_empty()),
        None => fallback.to_string(),
    }
}

fn template_of(key: Option<&str>) -> Option<&'static String> {
    key.filter(|key| !key.is_empty())
        .and_then(|key| english().get(key))
}

/// `{{name}}` replaced by its param. One pass: a param's own text is never read as a template. A
/// slot no param fills stays as written, or is dropped when `drop_unfilled`.
fn interpolate(template: &str, params: &Params, drop_unfilled: bool) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find("{{") {
        out.push_str(&rest[..open]);
        let after = &rest[open + 2..];
        let Some(close) = after.find("}}") else {
            out.push_str(&rest[open..]);
            return out;
        };
        match params.get(after[..close].trim()) {
            Some(value) => out.push_str(value),
            None if drop_unfilled => {}
            None => out.push_str(&rest[open..open + 2 + close + 2]),
        }
        rest = &after[close + 2..];
    }
    out.push_str(rest);
    out
}

/// Whether a param value is a key we have text for; a name or a money figure is not.
fn is_translation_key(value: &str) -> bool {
    value.contains('.') && english().contains_key(value)
}

/// Params as the sentence wants them (`resolveParamValues` in `backendI18n.ts`): the ordinal of a
/// finishing position, an old param under its current name, a key as its text, a money figure
/// compact, a day as a date.
fn resolve_params(params: &Params) -> Params {
    let mut resolved = params.clone();
    if !resolved.contains_key("ordinal") {
        let position = resolved
            .get("position")
            .filter(|position| !position.is_empty() && position.bytes().all(|b| b.is_ascii_digit()))
            .and_then(|position| position.parse::<u64>().ok());
        if let Some(position) = position {
            resolved.insert("ordinal".to_string(), format_ordinal(position));
        }
    }
    if let Some(wage_bill) = resolved.get("annualWages").cloned() {
        // `annualWages` held the weekly wage bill all along.
        resolved
            .entry("weeklyWageBill".to_string())
            .or_insert(wage_bill);
    }
    for (key, value) in resolved.clone() {
        let text = if is_translation_key(&value) {
            resolve(Some(value.as_str()), &value, &Params::new())
        } else {
            date_param(&key, &money_param(&key, &value))
        };
        resolved.insert(key, text);
    }
    resolved
}

/// English ordinals: 1st, 2nd, 3rd, and th for the rest, with 11 to 13 th as well.
fn format_ordinal(n: u64) -> String {
    let form = match (n % 10, n % 100) {
        (1, rest) if rest != 11 => "one",
        (2, rest) if rest != 12 => "two",
        (3, rest) if rest != 13 => "few",
        _ => "other",
    };
    let params = Params::from([("n".to_string(), n.to_string())]);
    let key = format!("common.ordinal.{form}");
    resolve(Some(key.as_str()), &n.to_string(), &params)
}

const MONEY_PARAMS: [&str; 13] = [
    "amount",
    "budget",
    "buyOption",
    "campaignCost",
    "cost",
    "fee",
    "grossRevenue",
    "netIncome",
    "transferBudgetReduction",
    "wage",
    "wageBudget",
    "weeklyWageBill",
    "weeklyWages",
];

/// A day the backend sends as `2026-08-01`, as English reads it: `1 August 2026` (day-month-year,
/// as `getLocale` writes English).
fn date_param(key: &str, value: &str) -> String {
    if key != "start" && key != "end" {
        return value.to_string();
    }
    let day = value.get(..10).unwrap_or(value);
    match NaiveDate::parse_from_str(day, "%Y-%m-%d") {
        Ok(date) if day.len() == 10 => {
            format!(
                "{} {} {}",
                date.day(),
                month_name(date.month()),
                date.year()
            )
        }
        _ => value.to_string(),
    }
}

fn month_name(month: u32) -> &'static str {
    const MONTHS: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    MONTHS[(month as usize).saturating_sub(1) % 12]
}

fn money_param(key: &str, value: &str) -> String {
    if !MONEY_PARAMS.contains(&key) {
        return value.to_string();
    }
    match parse_money(value) {
        Some((amount, already_shortened)) => format_precise_compact(amount, already_shortened),
        None => value.to_string(),
    }
}

/// A whole amount in euros, and whether the text had already been shortened (an old saved message
/// reads `€1.25M`). Only the euro and a bare number are read; another symbol is left as it is.
fn parse_money(text: &str) -> Option<(i64, bool)> {
    let text = text.trim();
    let (negative, text) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let text = text.strip_prefix('€').unwrap_or(text);
    let (number, unit) = match text.chars().last()? {
        'K' | 'k' => (&text[..text.len() - 1], 1_000.0),
        'M' | 'm' => (&text[..text.len() - 1], 1_000_000.0),
        _ => (text, 1.0),
    };
    let mut chars = number.chars();
    let first = chars.next()?;
    let well_formed = first.is_ascii_digit()
        && chars.all(|c| c.is_ascii_digit() || c == ',' || c == '.')
        && number.matches('.').count() <= 1
        && !number.ends_with('.');
    if !well_formed {
        return None;
    }
    let magnitude = number.replace(',', "").parse::<f64>().ok()? * unit;
    if !magnitude.is_finite() || magnitude > i64::MAX as f64 {
        return None;
    }
    let amount = magnitude.round() as i64;
    Some((if negative { -amount } else { amount }, unit > 1.0))
}

/// Compact when three digits say it all (`€1.25M`, `€1.7K`), exact when they would not (`€800,001`),
/// which is how the player's inbox shows a figure read in passing that must never be a rounded
/// one. `already_rounded` is for an amount that is itself a shortened figure: it is rounded to
/// three digits, never made to look exact.
fn format_precise_compact(amount: i64, already_rounded: bool) -> String {
    let sign = if amount < 0 { "-" } else { "" };
    let magnitude = amount.unsigned_abs();
    let rounded = round_to_three_digits(magnitude);
    if rounded != magnitude && !already_rounded {
        return format!("{sign}€{}", grouped(magnitude));
    }
    format!("{sign}€{}", compact(rounded))
}

/// `magnitude` to three significant digits, halves rounded up.
fn round_to_three_digits(magnitude: u64) -> u64 {
    let mut step = 1u64;
    while magnitude / step >= 1_000 {
        step = step.saturating_mul(10);
    }
    if step == 1 {
        return magnitude;
    }
    (magnitude + step / 2) / step * step
}

/// `1250000` as `1.25M`; a figure of three digits or fewer as it is.
fn compact(magnitude: u64) -> String {
    const UNITS: [(u64, &str); 4] = [
        (1_000_000_000_000, "T"),
        (1_000_000_000, "B"),
        (1_000_000, "M"),
        (1_000, "K"),
    ];
    let Some((unit, suffix)) = UNITS.into_iter().find(|(unit, _)| magnitude >= *unit) else {
        return magnitude.to_string();
    };
    let whole = magnitude / unit;
    let width = unit.ilog10() as usize;
    let fraction = format!("{:0width$}", magnitude % unit);
    let fraction = fraction.trim_end_matches('0');
    if fraction.is_empty() {
        format!("{whole}{suffix}")
    } else {
        format!("{whole}.{fraction}{suffix}")
    }
}

fn grouped(magnitude: u64) -> String {
    let digits = magnitude.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

// ─── What the sentence cannot say ───────────────────────────────────────────

/// The params the backend sends as structured JSON (`resultsData`, `standingsData`, ...), which
/// the screen lays out itself and this module does not. Handed over whole, so the agent reads
/// them as data and the sentence does not carry raw JSON.
fn structured_details(params: &Params) -> BTreeMap<String, Value> {
    params
        .iter()
        .filter(|(name, _)| name.ends_with("Data"))
        .map(|(name, text)| {
            let value =
                serde_json::from_str::<Value>(text).unwrap_or_else(|_| Value::String(text.clone()));
            (name.clone(), value)
        })
        .collect()
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
    ///      regenerated (delete the file and run `backendTextGolden.test.ts`) without this module
    ///      following.
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
