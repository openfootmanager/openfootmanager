//! Shadow bookings: dates a club could still be drawn on in a knockout it is alive in.

use crate::calendar_booking::{Booking, Layer, Occupancy};
use crate::knockout_dates::knockout_round_date;
use chrono::{Days, NaiveDate};
use domain::league::{CompetitionFormat, League};
use std::collections::{BTreeSet, HashMap};

/// Clubs that can still play in `cup`: every entrant before a bracket exists, then the clubs of
/// the latest incomplete round and its byes. A group stage has no shadows until its bracket exists.
pub(super) fn alive_clubs(cup: &League) -> Vec<&str> {
    match cup
        .knockout_rounds
        .iter()
        .rev()
        .find(|round| !round.completed)
    {
        Some(round) => {
            let mut clubs: Vec<&str> = cup
                .fixtures
                .iter()
                .filter(|fixture| round.fixture_ids.contains(&fixture.id))
                .flat_map(|fixture| [fixture.home_team_id.as_str(), fixture.away_team_id.as_str()])
                .chain(round.bye_team_ids.iter().map(String::as_str))
                .collect();
            clubs.sort_unstable();
            clubs.dedup();
            clubs
        }
        None if cup.knockout_rounds.is_empty()
            && cup.rules.format == CompetitionFormat::Knockout =>
        {
            cup.participant_ids.iter().map(String::as_str).collect()
        }
        None => Vec::new(),
    }
}

/// Books shadows for every round `cup` has not seeded yet, predicted one round after another
/// from the latest settled date of its current round.
pub(super) fn book_cup_shadows<'a>(
    occupancy: &mut Occupancy<'a>,
    cup: &'a League,
    settled: &HashMap<(&'a str, &'a str), NaiveDate>,
    blackout: &BTreeSet<NaiveDate>,
    other_cup_dates: &BTreeSet<NaiveDate>,
    layer: Layer,
) {
    let alive = alive_clubs(cup);
    if alive.len() < 2 {
        return;
    }
    let Some(mut previous_last) = cup
        .fixtures
        .iter()
        .filter_map(|fixture| {
            settled
                .get(&(cup.id.as_str(), fixture.id.as_str()))
                .copied()
        })
        .max()
        .or_else(|| NaiveDate::parse_from_str(&cup.fixtures.first()?.date, "%Y-%m-%d").ok())
    else {
        return;
    };
    let per_day = cup.rules.knockout_matches_per_day.max(1) as usize;
    let mut remaining = alive.len();
    loop {
        remaining = remaining.div_ceil(2);
        if remaining < 2 {
            return;
        }
        let ties = remaining / 2;
        let Some(first) = knockout_round_date(
            previous_last,
            Some(previous_last),
            cup.rules.knockout_round_gap_days,
            blackout,
            other_cup_dates,
        ) else {
            return;
        };
        let span = ties.div_ceil(per_day).max(1) as u64;
        for offset in 0..span {
            let Some(date) = first.checked_add_days(Days::new(offset)) else {
                continue;
            };
            for club in &alive {
                occupancy.book_shadow(
                    club,
                    Booking {
                        competition_id: &cup.id,
                        fixture_id: "",
                        layer,
                    },
                    date,
                );
            }
        }
        previous_last = first.checked_add_days(Days::new(span - 1)).unwrap_or(first);
    }
}
