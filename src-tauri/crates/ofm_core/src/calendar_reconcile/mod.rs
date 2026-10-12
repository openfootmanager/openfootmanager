//! The calendar sweep: given every competition, say which unplayed fixtures must move so that no
//! club plays twice inside the rest window, and where they go.
//!
//! Fixtures are settled by layer, highest first (continental, then cups, then league, then
//! friendlies). A fixture keeps its date when that date is legal and otherwise moves forward, alone,
//! to the earliest legal date. Nothing already settled is ever evicted, so one move never causes
//! another, and the result is the same whatever order the competitions arrive in.

mod shadows;
#[cfg(test)]
mod tests;

use crate::calendar_booking::{Booking, Layer, Occupancy};
use crate::knockout_dates::SEARCH_CAP_DAYS;
use chrono::{Days, NaiveDate};
use domain::league::{Fixture, FixtureStatus, League, Postponement, RescheduleReason};
use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Move {
    pub competition_id: String,
    pub fixture_id: String,
    pub home_team_id: String,
    pub away_team_id: String,
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub reason: RescheduleReason,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Plan {
    pub moves: Vec<Move>,
    /// Friendlies that clash are dropped rather than moved: they are filler.
    pub dropped_friendlies: Vec<(String, String)>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ReconcileError {
    UnreadableDate {
        competition_id: String,
        fixture_id: String,
    },
    /// No legal date inside the search cap. By the rulings this cannot happen: it is a bug.
    SearchCapExceeded {
        competition_id: String,
        fixture_id: String,
    },
}

struct Item<'a> {
    competition: &'a League,
    fixture: &'a Fixture,
    date: NaiveDate,
    layer: Layer,
    movable: bool,
}

impl<'a> Item<'a> {
    fn key(&self) -> (&'a str, &'a str) {
        (&self.competition.id, &self.fixture.id)
    }
}

fn collect_items<'a>(
    competitions: &'a [League],
    today: NaiveDate,
) -> Result<Vec<Item<'a>>, ReconcileError> {
    let mut items = Vec::new();
    for competition in competitions {
        for fixture in &competition.fixtures {
            let layer = Layer::of(&fixture.competition);
            if layer == Layer::International {
                continue;
            }
            let date = NaiveDate::parse_from_str(&fixture.date, "%Y-%m-%d").map_err(|_| {
                ReconcileError::UnreadableDate {
                    competition_id: competition.id.clone(),
                    fixture_id: fixture.id.clone(),
                }
            })?;
            let movable = fixture.status == FixtureStatus::Scheduled && date > today;
            items.push(Item {
                competition,
                fixture,
                date,
                layer,
                movable,
            });
        }
    }
    items.sort_by(|a, b| {
        (
            a.movable,
            std::cmp::Reverse(a.layer),
            a.competition.priority,
            a.date,
            a.key(),
        )
            .cmp(&(
                b.movable,
                std::cmp::Reverse(b.layer),
                b.competition.priority,
                b.date,
                b.key(),
            ))
    });
    Ok(items)
}

/// Index of each knockout fixture's round within its competition.
fn round_index<'a>(competitions: &'a [League]) -> HashMap<(&'a str, &'a str), usize> {
    let mut rounds = HashMap::new();
    for competition in competitions {
        for (index, round) in competition.knockout_rounds.iter().enumerate() {
            for fixture_id in &round.fixture_ids {
                rounds.insert((competition.id.as_str(), fixture_id.as_str()), index);
            }
        }
    }
    rounds
}

struct Settling<'a> {
    occupancy: Occupancy<'a>,
    settled: HashMap<(&'a str, &'a str), NaiveDate>,
    rounds: HashMap<(&'a str, &'a str), usize>,
    blackout: &'a BTreeSet<NaiveDate>,
    today: NaiveDate,
}

impl<'a> Settling<'a> {
    fn book(&mut self, item: &Item<'a>, date: NaiveDate) {
        let booking = Booking {
            competition_id: &item.competition.id,
            fixture_id: &item.fixture.id,
            layer: item.layer,
        };
        self.occupancy
            .book_firm(&item.fixture.home_team_id, booking, date);
        self.occupancy
            .book_firm(&item.fixture.away_team_id, booking, date);
        self.settled.insert(item.key(), date);
    }

    /// A knockout match cannot be played before the last match of the round that feeds it.
    fn not_before(&self, item: &Item<'a>) -> NaiveDate {
        let tomorrow = self
            .today
            .checked_add_days(Days::new(1))
            .unwrap_or(self.today);
        let Some(&round) = self.rounds.get(&item.key()) else {
            return tomorrow.max(item.date);
        };
        let feeding_last = item
            .competition
            .knockout_rounds
            .get(round.wrapping_sub(1))
            .into_iter()
            .flat_map(|previous| &previous.fixture_ids)
            .filter_map(|id| {
                self.settled
                    .get(&(item.competition.id.as_str(), id.as_str()))
            })
            .max();
        let after_feeding = feeding_last.and_then(|last| last.checked_add_days(Days::new(1)));
        after_feeding.map_or(tomorrow.max(item.date), |date| {
            date.max(tomorrow).max(item.date)
        })
    }

    /// Whether the fixture may stay on `date`: no blackout and no firm booking within the rest window.
    fn may_stay(&self, item: &Item<'a>, date: NaiveDate) -> bool {
        !self.blackout.contains(&date)
            && self.occupancy.is_free(&item.fixture.home_team_id, date)
            && self.occupancy.is_free(&item.fixture.away_team_id, date)
    }

    /// Whether the fixture may be moved onto `date`: as [`Self::may_stay`], and for league and
    /// friendly fixtures also clear of the dates a cup draw could still claim.
    fn may_move_to(&self, item: &Item<'a>, date: NaiveDate) -> bool {
        if item.layer > Layer::League {
            return self.may_stay(item, date);
        }
        !self.blackout.contains(&date)
            && self
                .occupancy
                .is_free_of_shadows(&item.fixture.home_team_id, date)
            && self
                .occupancy
                .is_free_of_shadows(&item.fixture.away_team_id, date)
    }

    fn reason_for_move(&self, item: &Item<'a>) -> RescheduleReason {
        if self.blackout.contains(&item.date) {
            return RescheduleReason::InternationalWindow;
        }
        let blocker = [&item.fixture.home_team_id, &item.fixture.away_team_id]
            .into_iter()
            .flat_map(|club| self.occupancy.firm_clashes(club, item.date))
            .map(|(_, booking)| booking.layer)
            .max()
            .unwrap_or(item.layer);
        match blocker {
            Layer::Continental => RescheduleReason::ContinentalTie,
            Layer::DomesticCup => RescheduleReason::CupTie,
            _ => RescheduleReason::LeagueClash,
        }
    }
}

fn cup_dates_outside(
    settled: &HashMap<(&str, &str), NaiveDate>,
    items: &[Item<'_>],
    competition_id: &str,
) -> BTreeSet<NaiveDate> {
    items
        .iter()
        .filter(|item| item.layer >= Layer::DomesticCup && item.competition.id != competition_id)
        .filter_map(|item| settled.get(&item.key()).copied())
        .collect()
}

/// Plans every move. Pure: nothing is changed until [`apply`].
pub fn plan(
    competitions: &[League],
    blackout: &BTreeSet<NaiveDate>,
    today: NaiveDate,
) -> Result<Plan, ReconcileError> {
    let items = collect_items(competitions, today)?;
    let mut settling = Settling {
        occupancy: Occupancy::default(),
        settled: HashMap::new(),
        rounds: round_index(competitions),
        blackout,
        today,
    };
    let mut plan = Plan::default();
    let mut shadows_booked = false;
    for item in &items {
        if !shadows_booked && item.movable && item.layer <= Layer::League {
            shadows_booked = true;
            book_all_shadows(&mut settling, competitions, &items, blackout);
        }
        if !item.movable {
            settling.book(item, item.date);
            continue;
        }
        let lower_bound = settling.not_before(item);
        if lower_bound == item.date && settling.may_stay(item, item.date) {
            settling.book(item, item.date);
            continue;
        }
        if item.layer == Layer::Friendly {
            plan.dropped_friendlies
                .push((item.competition.id.clone(), item.fixture.id.clone()));
            continue;
        }
        let reason = settling.reason_for_move(item);
        let target = (0..=SEARCH_CAP_DAYS)
            .filter_map(|offset| lower_bound.checked_add_days(Days::new(offset)))
            .find(|date| settling.may_move_to(item, *date))
            .ok_or_else(|| ReconcileError::SearchCapExceeded {
                competition_id: item.competition.id.clone(),
                fixture_id: item.fixture.id.clone(),
            })?;
        settling.book(item, target);
        plan.moves.push(Move {
            competition_id: item.competition.id.clone(),
            fixture_id: item.fixture.id.clone(),
            home_team_id: item.fixture.home_team_id.clone(),
            away_team_id: item.fixture.away_team_id.clone(),
            from: item.date,
            to: target,
            reason,
        });
    }
    Ok(plan)
}

fn book_all_shadows<'a>(
    settling: &mut Settling<'a>,
    competitions: &'a [League],
    items: &[Item<'a>],
    blackout: &BTreeSet<NaiveDate>,
) {
    for cup in competitions {
        let layer = items
            .iter()
            .find(|item| item.competition.id == cup.id)
            .map_or(Layer::DomesticCup, |item| {
                item.layer.max(Layer::DomesticCup)
            });
        let others = cup_dates_outside(&settling.settled, items, &cup.id);
        let settled = settling.settled.clone();
        shadows::book_cup_shadows(
            &mut settling.occupancy,
            cup,
            &settled,
            blackout,
            &others,
            layer,
        );
    }
}

/// Writes a plan into the competitions. `mark_postponed` records the original date and reason on
/// each moved fixture (kept from the first move); generation passes false because those dates
/// were never shown to the player. Returns how many fixtures changed.
pub fn apply(plan: &Plan, competitions: &mut [League], mark_postponed: bool) -> usize {
    let mut changed = 0;
    for planned in &plan.moves {
        let Some(fixture) =
            find_fixture(competitions, &planned.competition_id, &planned.fixture_id)
        else {
            continue;
        };
        if mark_postponed && fixture.postponement.is_none() {
            fixture.postponement = Some(Postponement {
                original_date: planned.from.format("%Y-%m-%d").to_string(),
                reason: planned.reason,
            });
        }
        fixture.date = planned.to.format("%Y-%m-%d").to_string();
        changed += 1;
    }
    for (competition_id, fixture_id) in &plan.dropped_friendlies {
        if let Some(competition) = competitions.iter_mut().find(|c| &c.id == competition_id) {
            competition.fixtures.retain(|f| &f.id != fixture_id);
            changed += 1;
        }
    }
    changed
}

fn find_fixture<'a>(
    competitions: &'a mut [League],
    competition_id: &str,
    fixture_id: &str,
) -> Option<&'a mut Fixture> {
    competitions
        .iter_mut()
        .find(|competition| competition.id == competition_id)?
        .fixtures
        .iter_mut()
        .find(|fixture| fixture.id == fixture_id)
}

/// Plan and apply in one step; the common call. Returns the plan that was applied.
pub fn reconcile(
    competitions: &mut [League],
    blackout: &BTreeSet<NaiveDate>,
    today: NaiveDate,
    mark_postponed: bool,
) -> Result<Plan, ReconcileError> {
    let plan = plan(competitions, blackout, today)?;
    apply(&plan, competitions, mark_postponed);
    Ok(plan)
}
