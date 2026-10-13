//! The daily calendar sweep: after a day's football, any fixture that now sits inside another
//! match's rest window, or on an international window, is moved to the earliest legal day. The
//! user hears about it once, in one message, when it touches their own club.

use std::collections::{BTreeSet, HashSet};

use chrono::{Datelike, NaiveDate};
use domain::message::{InboxMessage, MessageCategory, MessagePriority};
use log::error;

use crate::calendar_reconcile::{self, Move};
use crate::game::Game;
use crate::inbox;

/// Fixtures of competitions with a knockout bracket, as the day found them: a round drawn during
/// the day is new, so its dates were never shown and moving one is not a postponement.
pub(super) type ShownFixtures = HashSet<(String, String)>;

fn has_bracket(competition: &domain::league::League) -> bool {
    !competition.knockout_rounds.is_empty()
}

pub(super) fn shown_bracket_fixtures(game: &Game) -> ShownFixtures {
    game.competitions
        .iter()
        .filter(|competition| has_bracket(competition))
        .flat_map(|competition| {
            competition
                .fixtures
                .iter()
                .map(|fixture| (competition.id.clone(), fixture.id.clone()))
        })
        .collect()
}

/// Dates club football keeps clear of around `today`: the windows of the season that started the
/// year before and of the one starting this year, since a date belongs to either.
fn blackout_dates(game: &Game, today: NaiveDate) -> BTreeSet<NaiveDate> {
    let qualifying = game
        .competitions
        .iter()
        .any(crate::world_cup::is_world_cup_qualifying);
    [today.year() - 1, today.year()]
        .into_iter()
        .filter_map(|year| NaiveDate::from_ymd_opt(year, 7, 1))
        .filter_map(|start| start.and_hms_opt(0, 0, 0))
        .flat_map(|start| {
            crate::national_team::international_blackout_dates(
                chrono::DateTime::from_naive_utc_and_offset(start, chrono::Utc),
                qualifying,
            )
        })
        .collect()
}

/// Settles the world's calendar at the end of `today`, and tells the user about their own club's
/// moved matches. A calendar that cannot be settled is a defect: it is logged and left as it is.
pub(super) fn reconcile_calendar(game: &mut Game, today: &str, shown: &ShownFixtures) {
    let Ok(day) = NaiveDate::parse_from_str(today, "%Y-%m-%d") else {
        return;
    };
    let blackout = blackout_dates(game, day);
    let plan = match calendar_reconcile::plan(&game.competitions, &blackout, day) {
        Ok(plan) => plan,
        Err(failure) => {
            error!("[calendar] the {today} sweep could not settle the calendar: {failure:?}");
            return;
        }
    };
    if plan.moves.is_empty() && plan.dropped_friendlies.is_empty() {
        return;
    }

    let bracket_competitions: HashSet<&str> = game
        .competitions
        .iter()
        .filter(|competition| has_bracket(competition))
        .map(|competition| competition.id.as_str())
        .collect();
    let was_shown = |planned: &Move| {
        !bracket_competitions.contains(planned.competition_id.as_str())
            || shown.contains(&(planned.competition_id.clone(), planned.fixture_id.clone()))
    };
    let shown_moves: Vec<Move> = plan
        .moves
        .iter()
        .filter(|planned| was_shown(planned))
        .cloned()
        .collect();
    calendar_reconcile::apply_marking(&plan, &mut game.competitions, was_shown);
    game.sync_legacy_league();

    notify_user(game, today, &shown_moves);
}

fn notify_user(game: &mut Game, today: &str, moves: &[Move]) {
    let Some(user_team_id) = game.manager.team_id.clone() else {
        return;
    };
    let lines: Vec<String> = moves
        .iter()
        .filter(|planned| {
            planned.home_team_id == user_team_id || planned.away_team_id == user_team_id
        })
        .map(|planned| {
            format!(
                "{} – {}: {} → {}",
                game.team_name_or_id(&planned.home_team_id),
                game.team_name_or_id(&planned.away_team_id),
                planned.from.format("%Y-%m-%d"),
                planned.to.format("%Y-%m-%d"),
            )
        })
        .collect();
    if lines.is_empty() {
        return;
    }
    let key = format!("postponed_{today}");
    let date = game.clock.current_date.to_rfc3339();
    inbox::emit_once(game, &key, || fixtures_moved_message(&key, &lines, &date));
}

fn fixtures_moved_message(id: &str, lines: &[String], date: &str) -> InboxMessage {
    InboxMessage::new(
        id.to_string(),
        String::new(),
        String::new(),
        String::new(),
        date.to_string(),
    )
    .with_category(MessageCategory::LeagueInfo)
    .with_priority(MessagePriority::High)
    .with_sender_role("")
    .with_i18n(
        "be.msg.matchesMoved.subject",
        "be.msg.matchesMoved.body",
        std::collections::HashMap::from([("matches".to_string(), lines.join("\n"))]),
    )
    .with_sender_i18n("be.sender.assistantManager", "be.role.assistantManager")
}
