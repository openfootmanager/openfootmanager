//! National tournaments use the same nation/player resources as retained friendlies.

use super::immutable_bookings;
use crate::{calendar_reconcile, game::Game, knockout_dates};
use chrono::NaiveDate;
use domain::league::{CompetitionType, FixtureStatus, RescheduleReason};
use std::collections::{BTreeMap, BTreeSet};

/// A historical/scoped replay must not drag unrelated, earlier tournaments up to its own date.
/// Their original dates still participate in conflict resolution, even if that adapter won't play them.
pub(crate) fn settle_for_replay(game: &mut Game, day: NaiveDate) -> usize {
    let first = game
        .competitions
        .iter()
        .filter(|c| c.kind == CompetitionType::InternationalNation)
        .flat_map(|c| &c.fixtures)
        .chain(game.national_teams.iter().flat_map(|n| &n.fixtures))
        .filter(|f| f.status == FixtureStatus::Scheduled)
        .filter_map(|f| NaiveDate::parse_from_str(&f.date, "%Y-%m-%d").ok())
        .fold(day, NaiveDate::min);
    settle_tournaments(game, first)
}

/// Immutable matches keep their dates. Scheduled tournaments settle by the same numeric priority
/// as club competitions (smaller wins), before lower-priority friendlies and club fixtures.
pub(crate) fn settle_tournaments(game: &mut Game, first_day: NaiveDate) -> usize {
    let plan = tournament_plan(game, first_day);
    let moved = plan.moves.len();
    calendar_reconcile::apply(&plan, &mut game.competitions, true);
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
        let Some(to) =
            knockout_dates::knockout_round_date(earliest, None, 0, &BTreeSet::new(), &blocked)
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
