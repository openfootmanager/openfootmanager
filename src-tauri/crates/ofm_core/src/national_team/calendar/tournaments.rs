//! National tournaments use the same nation/player resources as retained friendlies.

use super::immutable_bookings;
use crate::{calendar_reconcile, game::Game, knockout_dates};
use chrono::NaiveDate;
use domain::league::{CompetitionType, FixtureStatus, RescheduleReason};
use std::collections::{BTreeMap, BTreeSet};

/// A historical/scoped replay must not drag unrelated, earlier tournaments up to its own date.
/// Their original dates still participate in conflict resolution, even if that adapter won't play them.
pub(crate) fn settle_for_replay(game: &mut Game, day: NaiveDate) -> usize {
    let first = replay_first_day(game, day);
    settle_tournaments(game, first) + super::settle_friendlies(game, first)
}

fn replay_first_day(game: &Game, day: NaiveDate) -> NaiveDate {
    game.competitions
        .iter()
        .filter(|c| c.kind == CompetitionType::InternationalNation)
        .flat_map(|c| &c.fixtures)
        .chain(game.national_teams.iter().flat_map(|n| &n.fixtures))
        .filter(|f| f.status == FixtureStatus::Scheduled)
        .filter_map(|f| NaiveDate::parse_from_str(&f.date, "%Y-%m-%d").ok())
        .fold(day, NaiveDate::min)
}

/// Immutable matches keep their dates. Scheduled tournaments settle by the same numeric priority
/// as club competitions (smaller wins), before lower-priority friendlies and club fixtures.
pub(crate) fn settle_tournaments(game: &mut Game, first_day: NaiveDate) -> usize {
    settle_marking(game, first_day, |_| true)
}

/// Rollover retains publication history only for editions the player could already see.
pub(crate) fn settle_tournaments_with_history(
    game: &mut Game,
    first_day: NaiveDate,
    published: &BTreeSet<(String, String)>,
) -> usize {
    settle_marking(game, first_day, |m| {
        published.contains(&(m.competition_id.clone(), m.fixture_id.clone()))
    })
}

pub(crate) fn published_tournament_fixtures(game: &Game) -> BTreeSet<(String, String)> {
    game.competitions
        .iter()
        .filter(|c| c.kind == CompetitionType::InternationalNation)
        .flat_map(|c| c.fixtures.iter().map(|f| (c.id.clone(), f.id.clone())))
        .collect()
}

/// Newly drawn fixtures were never shown; existing rivals still keep their postponement record.
pub(crate) fn settle_drawn_tournaments(
    game: &mut Game,
    day: NaiveDate,
    published: &BTreeSet<(String, String)>,
) -> usize {
    let first = replay_first_day(game, day);
    settle_tournaments_with_history(game, first, published) + super::settle_friendlies(game, first)
}

fn settle_marking(
    game: &mut Game,
    first_day: NaiveDate,
    was_shown: impl FnMut(&calendar_reconcile::Move) -> bool,
) -> usize {
    let plan = tournament_plan(game, first_day);
    let moved = plan.moves.len();
    calendar_reconcile::apply_marking(&plan, &mut game.competitions, was_shown);
    moved
}

fn tournament_plan(game: &Game, first_day: NaiveDate) -> calendar_reconcile::Plan {
    let rounds = calendar_reconcile::round_index(&game.competitions);
    let mut fixtures: Vec<_> = game
        .competitions
        .iter()
        .filter(|c| c.kind == CompetitionType::InternationalNation)
        .flat_map(|c| {
            c.fixtures
                .iter()
                .filter(|f| f.status == FixtureStatus::Scheduled)
                .map(move |f| (c, f))
        })
        .collect();
    if fixtures.is_empty() {
        return calendar_reconcile::Plan::default();
    }
    fixtures.sort_by_key(|(c, f)| {
        (
            c.priority,
            rounds
                .get(&(c.id.as_str(), f.id.as_str()))
                .map_or(0, |round| round + 1),
            &f.date,
            &c.id,
            &f.id,
        )
    });
    let mut bookings = immutable_bookings(game, first_day);
    let mut settled = settled_history(game);
    let mut per_day = BTreeMap::new();
    for ((competition_id, _), date) in &settled {
        *per_day.entry((*competition_id, *date)).or_insert(0usize) += 1;
    }
    let mut plan = calendar_reconcile::Plan::default();
    for (competition, fixture) in fixtures {
        let Ok(from) = NaiveDate::parse_from_str(&fixture.date, "%Y-%m-%d") else {
            continue;
        };
        let earliest =
            knockout_dates::fixture_not_before(competition, &fixture.id, from, first_day, |id| {
                settled.get(&(competition.id.as_str(), id)).copied()
            });
        let blocked = bookings.blocked(&fixture.home_team_id, &fixture.away_team_id);
        let full_days = full_calendar_days(competition, &fixture.id, &per_day);
        let Some(to) = knockout_dates::knockout_round_date(earliest, None, 0, &full_days, &blocked)
        else {
            log::error!(
                "national tournament fixture {} has no legal calendar date",
                fixture.id
            );
            // Planning is atomic: never apply only a subset of a tournament's moves.
            return calendar_reconcile::Plan::default();
        };
        bookings.book(&fixture.home_team_id, &fixture.away_team_id, to);
        settled.insert((competition.id.as_str(), fixture.id.as_str()), to);
        *per_day.entry((competition.id.as_str(), to)).or_insert(0) += 1;
        if from != to {
            plan.moves.push(calendar_reconcile::Move {
                competition_id: competition.id.clone(),
                fixture_id: fixture.id.clone(),
                home_team_id: fixture.home_team_id.clone(),
                away_team_id: fixture.away_team_id.clone(),
                from,
                to,
                reason: RescheduleReason::InternationalWindow,
            });
        }
    }
    plan
}

// Completed feeders constrain a pre-seeded successor even outside the participant rest horizon.
fn settled_history(game: &Game) -> BTreeMap<(&str, &str), NaiveDate> {
    game.competitions
        .iter()
        .filter(|c| c.kind == CompetitionType::InternationalNation)
        .flat_map(|c| {
            c.fixtures
                .iter()
                .filter(|f| f.status != FixtureStatus::Scheduled)
                .filter_map(|f| {
                    NaiveDate::parse_from_str(&f.date, "%Y-%m-%d")
                        .ok()
                        .map(|date| ((c.id.as_str(), f.id.as_str()), date))
                })
        })
        .collect()
}

// Keep the existing draw's daily limits when an individual match moves, including World Cup TV days.
fn full_calendar_days(
    competition: &domain::league::League,
    fixture_id: &str,
    per_day: &BTreeMap<(&str, NaiveDate), usize>,
) -> BTreeSet<NaiveDate> {
    let limit = if competition.is_knockout_fixture(fixture_id) {
        Some(competition.rules.knockout_matches_per_day.max(1))
    } else if crate::world_cup::is_world_cup_finals(competition) {
        Some(crate::world_cup::FINALS_MATCHES_PER_DAY)
    } else {
        None
    };
    per_day
        .iter()
        .filter(|((id, _), count)| {
            *id == competition.id && limit.is_some_and(|limit| **count >= limit as usize)
        })
        .map(|((_, date), _)| *date)
        .collect()
}

/// Settle a newly generated season before reserving its national dates in the club calendar.
pub(crate) fn settle_retained_calendar(
    game: &mut Game,
    published: &std::collections::BTreeSet<(String, String)>,
) -> BTreeSet<NaiveDate> {
    let first_day = game.clock.current_date.date_naive();
    settle_tournaments_with_history(game, first_day, published);
    super::settle_friendlies(game, first_day);
    crate::national_team::international_blackout_around(game, first_day)
}
