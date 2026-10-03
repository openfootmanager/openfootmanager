//! The wording of a news article or inbox message is decided by what it is about, so a
//! replayed day writes the same story.
//!
//! Each builder is called, with the same arguments, for thirty different days: a builder
//! that still drew from the operating system would disagree with itself on some of them,
//! and one that ignored what it was given would use one phrasing for all thirty.

use domain::league::FixtureCompetition;
use ofm_core::{messages, news};

fn days() -> Vec<String> {
    (1..=30).map(|day| format!("2032-09-{day:02}")).collect()
}

/// `build(date)` twice for each day is the same, and across the days it is not always the
/// same.
fn assert_replays_but_varies<T: serde::Serialize>(what: &str, build: impl Fn(&str) -> T) {
    // Through `Value`, whose maps are sorted: the params are a `HashMap`, whose order in
    // a string differs between two equal values.
    let json = |date: &str| serde_json::to_value(build(date)).unwrap().to_string();
    for date in days() {
        assert_eq!(json(&date), json(&date), "{what} on {date}");
    }
    let distinct: std::collections::BTreeSet<String> =
        days().iter().map(|date| json(date)).collect();
    // Dates are in the output, so they always differ; what must differ beyond that is
    // the wording, which is checked by stripping the date.
    let wordings: std::collections::BTreeSet<String> = days()
        .iter()
        .map(|date| json(date).replace(date.as_str(), ""))
        .collect();
    assert_eq!(distinct.len(), days().len(), "{what} lost its dates");
    assert!(wordings.len() > 1, "{what} used one wording for every day");
}

#[test]
fn a_league_roundup_is_worded_the_same_way_each_time() {
    let results = vec![
        ("Alpha".to_string(), 3, "Beta".to_string(), 0),
        ("Gamma".to_string(), 1, "Delta".to_string(), 1),
    ];
    assert_replays_but_varies("league roundup", |date| {
        news::league_roundup_article("eng-d1", 4, &results, date)
    });
}

#[test]
fn a_standings_update_is_worded_the_same_way_each_time() {
    let table = vec![("Alpha".to_string(), 12, 7), ("Beta".to_string(), 10, 3)];
    assert_replays_but_varies("standings update", |date| {
        news::standings_update_article("eng-d1", 4, &table, date)
    });
}

#[test]
fn a_season_preview_names_the_same_contenders_each_time() {
    let teams: Vec<String> = (0..8).map(|n| format!("Club {n}")).collect();
    assert_replays_but_varies("season preview", |date| {
        news::season_preview_article(&teams, date)
    });
}

#[test]
fn a_transfer_rumour_is_worded_the_same_way_each_time() {
    assert_replays_but_varies("transfer rumour", |date| {
        news::transfer_rumour_gossip_article(
            &format!("rumour_p1_{date}"),
            "p1",
            "A. Player",
            "t1",
            "Alpha FC",
            date,
        )
    });
}

#[test]
fn an_injury_report_is_worded_the_same_way_each_time() {
    assert_replays_but_varies("injury report", |date| {
        news::injury_news_article(
            &format!("injury_p1_{date}"),
            "p1",
            "A. Player",
            "t1",
            "Alpha FC",
            20,
            date,
        )
    });
}

#[test]
fn a_match_report_is_worded_the_same_way_each_time() {
    let scorers = vec![("A. Player".to_string(), 12)];
    assert_replays_but_varies("match report", |date| {
        news::match_report_article(
            &format!("fixture-{date}"),
            "Alpha FC",
            "Beta FC",
            2,
            0,
            "t1",
            "t2",
            FixtureCompetition::League,
            3,
            &scorers,
            &[],
            date,
        )
    });
}

#[test]
fn a_pre_match_message_is_worded_the_same_way_each_time() {
    assert_replays_but_varies("pre-match message", |date| {
        messages::pre_match_message(
            &format!("fixture-{date}"),
            "Beta FC",
            "t2",
            true,
            3,
            date,
            date,
        )
    });
}

#[test]
fn a_match_result_message_is_worded_the_same_way_each_time() {
    assert_replays_but_varies("match result message", |date| {
        messages::match_result_message(
            &format!("fixture-{date}"),
            "Alpha FC",
            "Beta FC",
            2,
            0,
            "t1",
            "t2",
            "t1",
            3,
            date,
        )
    });
}
