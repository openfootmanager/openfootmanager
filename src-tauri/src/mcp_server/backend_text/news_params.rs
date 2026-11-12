//! What the screen does to a news article's structured params before it resolves the text, in
//! English: `normalize*Params` in `src/utils/backendI18n.ts`, which this follows line by line and
//! `src/utils/backendTextGolden.json` holds it to.
//!
//! An article carries JSON params (`resultsData`, `standingsData`, `scorersData`, `quotesData`,
//! `dealsData`, `unbeatenTeamsData`) that the screen lays out into the lines of its body, and a few
//! plain params of its own (`results`, `scorers`, ...) that the backend leaves empty or only as an
//! English fallback. Without this the body's slots stay unfilled (a match report with no goals, a
//! press conference with no quote, a roundup that never names its biggest winner).
//!
//! Each expansion reads only the article's original params, none reads another's output, and one
//! whose JSON cannot be read adds nothing (the screen then shows the params as they came).

use domain::news::NewsArticle;
use serde_json::{Map, Number, Value};

use super::{money_param, names, resolve, resolve_params, Params};

const PRESEASON_NO_RESULTS: &str = "be.news.preseasonDigest.bodyNoResults";
const PRESEASON_WITH_RESULTS: &str = "be.news.preseasonDigest.bodyWithResults";
const ROUNDUP_BODY: &str = "be.news.roundup.body";
const STANDINGS_BODY: &str = "be.news.standings.body";
const TRANSFER_ROUNDUP_BODY: &str = "be.news.transferRoundup.body";
const PRESS_CONFERENCE: &str = "be.news.pressConference.";
const MATCH_REPORT_BODY: &str = "be.news.matchReport.body";
const MATCH_REPORT_COPIES: [&str; 3] = [
    "be.news.matchReport.reportFriendly.body",
    "be.news.matchReport.reportPreseason.body",
    "be.news.matchReport.reportCompetition.body",
];

/// The article's params with the lines its structured params stand for added.
pub(super) fn expanded_params(article: &NewsArticle) -> Params {
    let original = &article.i18n_params;
    let body_key = article.body_key.as_deref().unwrap_or_default();
    let headline_key = article.headline_key.as_deref().unwrap_or_default();
    let mut expanded = original.clone();
    for added in [
        names::season_awards_teams(body_key, original),
        names::report_competition(body_key, original),
        preseason_digest(body_key, original),
        roundup(body_key, original),
        standings(body_key, original),
        match_report(body_key, original),
        press_conference(headline_key, body_key, original),
        transfer_roundup(body_key, original),
    ]
    .into_iter()
    .flatten()
    {
        expanded.extend(added);
    }
    expanded
}

/// A param that is there and not empty (the screen's `if (params.name)`).
fn present<'a>(params: &'a Params, name: &str) -> Option<&'a String> {
    params.get(name).filter(|text| !text.is_empty())
}

fn items(data: &str) -> Option<Vec<Value>> {
    serde_json::from_str(data).ok()
}

/// JavaScript's `String(value ?? "")` for what the backend writes: a string as it is, a number
/// without a trailing `.0`, nothing as nothing.
fn text_of(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(text)) => text.clone(),
        Some(Value::Number(number)) => number_text(number),
        Some(other) => other.to_string(),
    }
}

fn number_text(number: &Number) -> String {
    match number.as_f64() {
        Some(value) if number.is_f64() && value.fract() == 0.0 && value.abs() < 1e15 => {
            format!("{}", value as i64)
        }
        _ => number.to_string(),
    }
}

/// The text of the first of `names` the item has (`a ?? b ?? ""`).
fn first_text(item: &Value, names: &[&str]) -> String {
    names
        .iter()
        .find_map(|name| item.get(name).filter(|value| !value.is_null()))
        .map(|value| text_of(Some(value)))
        .unwrap_or_default()
}

fn params_of<const N: usize>(pairs: [(&str, String); N]) -> Params {
    pairs
        .into_iter()
        .map(|(name, value)| (name.to_string(), value))
        .collect()
}

fn results_lines(data: &str, line_key: &str) -> Option<String> {
    let results = items(data)?;
    let lines: Vec<String> = results
        .iter()
        .map(|result| {
            let home = text_of(result.get("home"));
            let away = text_of(result.get("away"));
            let home_goals = first_text(result, &["home_goals", "homeGoals"]);
            let away_goals = first_text(result, &["away_goals", "awayGoals"]);
            let fallback = format!("  {home} {home_goals} - {away_goals} {away}");
            resolve(
                Some(line_key),
                &fallback,
                &params_of([
                    ("home", home),
                    ("homeGoals", home_goals),
                    ("away", away),
                    ("awayGoals", away_goals),
                ]),
            )
        })
        .collect();
    Some(lines.join("\n"))
}

fn preseason_digest(body_key: &str, params: &Params) -> Option<Params> {
    if body_key != PRESEASON_NO_RESULTS && body_key != PRESEASON_WITH_RESULTS {
        return None;
    }
    let mut added = Params::new();
    if let Some(data) = present(params, "resultsData") {
        added.insert(
            "results".to_string(),
            results_lines(data, "be.news.preseasonDigest.resultLine")?,
        );
    }
    if let Some(data) = present(params, "unbeatenTeamsData") {
        added.insert("unbeatenLine".to_string(), unbeaten_line(data)?);
    }
    Some(added)
}

/// The preseason digest's sentence about the clubs still unbeaten: none, one, two, or a list.
fn unbeaten_line(data: &str) -> Option<String> {
    let teams: Vec<String> = items(data)?
        .iter()
        .map(|team| text_of(Some(team)))
        .collect();
    Some(match teams.as_slice() {
        [] => String::new(),
        [team] => resolve(
            Some("be.news.preseasonDigest.unbeatenLine.one"),
            &format!("\n\n{team} remain unbeaten in preseason."),
            &params_of([("team", team.clone())]),
        ),
        [first, second] => resolve(
            Some("be.news.preseasonDigest.unbeatenLine.two"),
            &format!("\n\n{first} and {second} remain unbeaten in preseason."),
            &params_of([("first", first.clone()), ("second", second.clone())]),
        ),
        _ => {
            let list = english_list(&teams);
            resolve(
                Some("be.news.preseasonDigest.unbeatenLine.multiple"),
                &format!("\n\n{list} remain unbeaten in preseason."),
                &params_of([("teams", list.clone()), ("count", teams.len().to_string())]),
            )
        }
    })
}

/// `Intl.ListFormat("en", { style: "long", type: "conjunction" })`: "A, B, and C".
fn english_list(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [only] => only.clone(),
        [first, second] => format!("{first} and {second}"),
        [init @ .., last] => format!("{}, and {last}", init.join(", ")),
    }
}

fn roundup(body_key: &str, params: &Params) -> Option<Params> {
    if body_key != ROUNDUP_BODY {
        return None;
    }
    let mut added = Params::new();
    if let Some(data) = present(params, "resultsData") {
        added.insert(
            "results".to_string(),
            results_lines(data, "be.news.roundup.resultLine")?,
        );
    }
    // The screen tests the trimmed name and passes the name as it came.
    let line = match params.get("biggestWinner") {
        Some(name) if !name.trim().is_empty() => resolve(
            Some("be.news.roundup.biggestWinnerLine"),
            &format!(" {name} recorded the biggest win of the day."),
            &params_of([("biggestWinner", name.clone())]),
        ),
        _ => String::new(),
    };
    added.insert("biggestWinnerLine".to_string(), line);
    Some(added)
}

fn standings(body_key: &str, params: &Params) -> Option<Params> {
    if body_key != STANDINGS_BODY {
        return None;
    }
    let entries = items(present(params, "standingsData")?)?;
    let lines: Vec<String> = entries
        .iter()
        .map(|entry| {
            let rank = text_of(entry.get("rank"));
            let team = text_of(entry.get("team"));
            let points = text_of(entry.get("points"));
            let goal_difference = first_text(entry, &["goal_difference", "goalDifference"]);
            let fallback = format!("  {rank}. {team} — {points} pts (GD: {goal_difference})");
            resolve(
                Some("be.news.standings.entry"),
                &fallback,
                &params_of([
                    ("rank", rank),
                    ("team", team),
                    ("points", points),
                    ("goalDifference", goal_difference),
                ]),
            )
        })
        .collect();
    Some(params_of([("standings", lines.join("\n"))]))
}

fn match_report(body_key: &str, params: &Params) -> Option<Params> {
    let is_match_report =
        body_key.starts_with(MATCH_REPORT_BODY) || MATCH_REPORT_COPIES.contains(&body_key);
    if !is_match_report {
        return None;
    }
    let scorers = items(present(params, "scorersData")?)?;
    if scorers.is_empty() {
        return Some(params_of([
            ("scorers", String::new()),
            ("scorersSection", String::new()),
        ]));
    }
    let goals: Vec<String> = scorers
        .iter()
        .map(|scorer| {
            let player = text_of(scorer.get("player"));
            let minute = text_of(scorer.get("minute"));
            let team = text_of(scorer.get("team"));
            let fallback = format!("{player} ({minute}', {team})");
            resolve(
                Some("be.news.matchReport.scorer"),
                &fallback,
                &params_of([("player", player), ("minute", minute), ("team", team)]),
            )
        })
        .collect();
    let text = goals.join(", ");
    let section = resolve(
        Some("be.news.matchReport.scorersSection"),
        &format!("\n\nGoals: {text}"),
        &params_of([("scorers", text.clone())]),
    );
    Some(params_of([("scorers", text), ("scorersSection", section)]))
}

fn press_conference(headline_key: &str, body_key: &str, params: &Params) -> Option<Params> {
    if !headline_key.starts_with(PRESS_CONFERENCE) && !body_key.starts_with(PRESS_CONFERENCE) {
        return None;
    }
    let quotes = items(present(params, "quotesData")?)?;
    let said: Vec<String> = quotes
        .iter()
        .map(|quote| {
            let key = quote.get("key").and_then(Value::as_str);
            let fallback = quote.get("fallback").and_then(Value::as_str).unwrap_or("");
            let quote_params = quote
                .get("params")
                .and_then(Value::as_object)
                .map(string_params)
                .unwrap_or_default();
            resolve(key, fallback, &resolve_params(&quote_params))
        })
        .filter(|quote| !quote.is_empty())
        .collect();
    let first = said.first()?.clone();
    let bulleted: Vec<String> = said.iter().map(|quote| format!("• \"{quote}\"")).collect();
    Some(params_of([
        ("quote", first),
        ("quotes", bulleted.join("\n")),
    ]))
}

fn string_params(object: &Map<String, Value>) -> Params {
    object
        .iter()
        .map(|(name, value)| (name.clone(), text_of(Some(value))))
        .collect()
}

fn transfer_roundup(body_key: &str, params: &Params) -> Option<Params> {
    if body_key != TRANSFER_ROUNDUP_BODY {
        return None;
    }
    let deals = items(present(params, "dealsData")?)?;
    let lines: Vec<String> = deals
        .iter()
        .map(|deal| {
            let mut fields = deal.as_object().map(string_params).unwrap_or_default();
            // The screen writes the fee compact, as it writes every money param of a message.
            let fee = money_param("fee", &text_of(deal.get("fee")));
            fields.insert("fee".to_string(), fee.clone());
            let fallback = format!(
                "  {}: {} -> {} ({})",
                text_of(deal.get("player")),
                text_of(deal.get("fromTeam")),
                text_of(deal.get("toTeam")),
                fee
            );
            resolve(Some("be.news.transferRoundup.dealLine"), &fallback, &fields)
        })
        .collect();
    Some(params_of([("deals", lines.join("\n"))]))
}
