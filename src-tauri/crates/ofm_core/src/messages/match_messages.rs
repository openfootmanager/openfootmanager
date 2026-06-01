use super::{action, params};
use domain::message::*;
use rand::RngExt;

pub fn pre_match_message(
    fixture_id: &str,
    opponent_name: &str,
    opponent_id: &str,
    is_home: bool,
    matchday: u32,
    match_date: &str,
    date: &str,
) -> InboxMessage {
    let mut rng = rand::rng();
    let idx = rng.random_range(0..2);
    let venue_short = if is_home { "H" } else { "A" };
    let body_key = format!(
        "be.msg.preMatch.body{}{}",
        idx,
        if is_home { "Home" } else { "Away" }
    );

    InboxMessage::new(
        format!("prematch_{}", fixture_id),
        String::new(),
        String::new(),
        String::new(),
        date.to_string(),
    )
    .with_category(MessageCategory::MatchPreview)
    .with_priority(MessagePriority::Normal)
    .with_sender_role("")
    .with_action(action(
        "set_tactics",
        "",
        "be.msg.preMatch.actionTactics",
        ActionType::NavigateTo {
            route: "/dashboard?tab=Tactics".to_string(),
        },
    ))
    .with_action(action(
        "view_opponent",
        "",
        "be.msg.preMatch.actionScout",
        ActionType::NavigateTo {
            route: format!("/team/{}", opponent_id),
        },
    ))
    .with_context(MessageContext {
        fixture_id: Some(fixture_id.to_string()),
        team_id: Some(opponent_id.to_string()),
        ..Default::default()
    })
    .with_i18n(
        "be.msg.preMatch.subject",
        &body_key,
        params(&[
            ("venue", venue_short),
            ("opponent", opponent_name),
            ("matchDate", match_date),
            ("matchday", &matchday.to_string()),
        ]),
    )
    .with_sender_i18n("be.sender.assistantManager", "be.role.assistantManager")
}

pub fn match_result_message(
    fixture_id: &str,
    home_name: &str,
    away_name: &str,
    home_goals: u8,
    away_goals: u8,
    home_team_id: &str,
    away_team_id: &str,
    user_team_id: &str,
    matchday: u32,
    date: &str,
) -> InboxMessage {
    let is_home = home_team_id == user_team_id;
    let user_goals = if is_home { home_goals } else { away_goals };
    let opp_goals = if is_home { away_goals } else { home_goals };

    let outcome = if user_goals > opp_goals {
        "Victory"
    } else if user_goals < opp_goals {
        "Defeat"
    } else {
        "Draw"
    };

    let mut rng = rand::rng();
    let body_key = format!(
        "be.msg.matchResult.body.{}{}",
        outcome.to_lowercase(),
        if outcome == "Draw" {
            String::new()
        } else {
            rng.random_range(0..2u8).to_string()
        }
    );

    InboxMessage::new(
        format!("result_{}", fixture_id),
        String::new(),
        String::new(),
        String::new(),
        date.to_string(),
    )
    .with_category(MessageCategory::MatchResult)
    .with_priority(if outcome == "Victory" {
        MessagePriority::Normal
    } else {
        MessagePriority::High
    })
    .with_sender_role("")
    .with_action(action(
        "view_standings",
        "",
        "be.msg.matchResult.actionStandings",
        ActionType::NavigateTo {
            route: "/dashboard?tab=Schedule".to_string(),
        },
    ))
    .with_context(MessageContext {
        fixture_id: Some(fixture_id.to_string()),
        match_result: Some(ContextMatchResult {
            home_team_id: home_team_id.to_string(),
            away_team_id: away_team_id.to_string(),
            home_goals,
            away_goals,
        }),
        ..Default::default()
    })
    .with_i18n(
        &format!("be.msg.matchResult.subject.{}", outcome.to_lowercase()),
        &body_key,
        {
            let mut p = params(&[
                ("home", home_name),
                ("away", away_name),
                ("homeGoals", &home_goals.to_string()),
                ("awayGoals", &away_goals.to_string()),
                ("matchday", &matchday.to_string()),
            ]);
            p.insert("outcome".to_string(), outcome.to_string());
            p
        },
    )
    .with_sender_i18n("be.sender.matchReporter", "be.role.pressOfficer")
}
