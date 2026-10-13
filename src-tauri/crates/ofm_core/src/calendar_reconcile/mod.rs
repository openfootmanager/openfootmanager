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

use crate::calendar_booking::{Booking, Layer, MIN_GAP_DAYS, Occupancy};
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
    /// Where the fixture sorts among its competition's: a knockout match is never earlier than the
    /// latest match of any round before it, so a feeding round authored after its successor
    /// still settles first.
    settle_on: NaiveDate,
}

impl<'a> Item<'a> {
    fn key(&self) -> (&'a str, &'a str) {
        (&self.competition.id, &self.fixture.id)
    }
}

fn collect_items<'a>(
    competitions: &'a [League],
    rounds: &HashMap<(&str, &str), usize>,
    today: NaiveDate,
) -> Result<Vec<Item<'a>>, ReconcileError> {
    let mut items = Vec::new();
    // History further back than the rest window cannot touch anything that may still move, which
    // is all dated after `today`; only a bracket's fixtures are kept, because a later round waits
    // for the last match of the round that feeds it.
    let horizon = today
        .checked_sub_days(Days::new(MIN_GAP_DAYS as u64))
        .unwrap_or(today);
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
            if !movable
                && date < horizon
                && !rounds.contains_key(&(competition.id.as_str(), fixture.id.as_str()))
            {
                continue;
            }
            items.push(Item {
                competition,
                fixture,
                date,
                layer,
                movable,
                settle_on: date,
            });
        }
    }
    let latest_per_round = latest_date_per_round(&items, rounds);
    for item in &mut items {
        if let Some(&round) = rounds.get(&item.key()) {
            let earlier_rounds = latest_per_round
                .get(item.competition.id.as_str())
                .map_or(&[][..], |dates| &dates[..round.min(dates.len())]);
            item.settle_on = earlier_rounds
                .iter()
                .flatten()
                .copied()
                .fold(item.date, NaiveDate::max);
        }
    }
    let round_of = |item: &Item<'_>| rounds.get(&item.key()).copied().unwrap_or(0);
    items.sort_by(|a, b| {
        (
            a.movable,
            std::cmp::Reverse(a.layer),
            a.competition.priority,
            a.settle_on,
            round_of(a),
            a.date,
            a.key(),
        )
            .cmp(&(
                b.movable,
                std::cmp::Reverse(b.layer),
                b.competition.priority,
                b.settle_on,
                round_of(b),
                b.date,
                b.key(),
            ))
    });
    Ok(items)
}

/// The latest authored date of each knockout round of each competition, by round index.
fn latest_date_per_round<'a>(
    items: &[Item<'a>],
    rounds: &HashMap<(&str, &str), usize>,
) -> HashMap<&'a str, Vec<Option<NaiveDate>>> {
    let mut latest: HashMap<&str, Vec<Option<NaiveDate>>> = HashMap::new();
    for item in items {
        let Some(&round) = rounds.get(&item.key()) else {
            continue;
        };
        let per_round = latest.entry(item.competition.id.as_str()).or_default();
        if per_round.len() <= round {
            per_round.resize(round + 1, None);
        }
        per_round[round] = per_round[round].max(Some(item.date));
    }
    latest
}

/// Index of each knockout fixture's round within its competition.
fn round_index(competitions: &[League]) -> HashMap<(&str, &str), usize> {
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

    /// A knockout match cannot be played before the last match of the round that feeds it, plus
    /// the cup's round gap.
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
        // The same wait `knockout_round_date` leaves between rounds (at least a day, so a round
        // is never on the day of its feeders), or a moved feeder would pull its successor
        // closer than the cup's own rule when the two are dated separately.
        let wait = u64::from(item.competition.rules.knockout_round_gap_days.max(1));
        let after_feeding = feeding_last.and_then(|last| last.checked_add_days(Days::new(wait)));
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

/// The dates of other cups' matches that involve one of `clubs`: the rest window is a per-club
/// rule, so a cup none of these clubs plays in cannot block them.
fn cup_dates_outside(
    settled: &HashMap<(&str, &str), NaiveDate>,
    items: &[Item<'_>],
    competition_id: &str,
    clubs: &[&str],
) -> BTreeSet<NaiveDate> {
    items
        .iter()
        .filter(|item| item.layer >= Layer::DomesticCup && item.competition.id != competition_id)
        .filter(|item| {
            clubs.contains(&item.fixture.home_team_id.as_str())
                || clubs.contains(&item.fixture.away_team_id.as_str())
        })
        .filter_map(|item| settled.get(&item.key()).copied())
        .collect()
}

/// Plans every move. Pure: nothing is changed until [`apply`].
pub fn plan(
    competitions: &[League],
    blackout: &BTreeSet<NaiveDate>,
    today: NaiveDate,
) -> Result<Plan, ReconcileError> {
    let rounds = round_index(competitions);
    let items = collect_items(competitions, &rounds, today)?;
    let mut settling = Settling {
        occupancy: Occupancy::default(),
        settled: HashMap::new(),
        rounds,
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
        // A friendly outside a knockout bracket yields by being dropped. A tie of a friendly cup or a preseason
        // tournament is part of a bracket whose next round names its winner, so it moves like
        // any cup tie.
        if item.layer == Layer::Friendly && !settling.rounds.contains_key(&item.key()) {
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

/// Only a knockout, or a cup whose bracket already exists, can have shadows.
fn has_knockout_bracket(competition: &League) -> bool {
    !competition.knockout_rounds.is_empty()
        || competition.rules.format == domain::league::CompetitionFormat::Knockout
}

fn book_all_shadows<'a>(
    settling: &mut Settling<'a>,
    competitions: &'a [League],
    items: &[Item<'a>],
    blackout: &BTreeSet<NaiveDate>,
) {
    for cup in competitions.iter().filter(|c| has_knockout_bracket(c)) {
        let layer = items
            .iter()
            .find(|item| item.competition.id == cup.id)
            .map_or(Layer::DomesticCup, |item| {
                item.layer.max(Layer::DomesticCup)
            });
        let others = cup_dates_outside(
            &settling.settled,
            items,
            &cup.id,
            &shadows::alive_clubs(cup),
        );
        shadows::book_cup_shadows(
            &mut settling.occupancy,
            cup,
            &settling.settled,
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
    apply_marking(plan, competitions, |_| mark_postponed)
}

/// As [`apply`], deciding per move whether the date it leaves had been shown to the player.
pub fn apply_marking(
    plan: &Plan,
    competitions: &mut [League],
    mut mark_postponed: impl FnMut(&Move) -> bool,
) -> usize {
    let mut changed = 0;
    for planned in &plan.moves {
        let marks = mark_postponed(planned);
        let Some(fixture) =
            find_fixture(competitions, &planned.competition_id, &planned.fixture_id)
        else {
            continue;
        };
        if marks && fixture.postponement.is_none() {
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

/// Settles the calendar of a world that was just generated: every clash is resolved by the same
/// rules as the daily sweep, but nothing is recorded as postponed because none of these dates was
/// ever published to the player. `current_date` is the first day still to be played, so a match
/// dated that day may move.
///
/// A calendar that cannot be settled is a defect, never a rejection: it is logged loudly and the
/// competitions are left as generated.
pub fn settle_generated_calendar(
    competitions: &mut [League],
    blackout: &BTreeSet<NaiveDate>,
    current_date: NaiveDate,
) {
    settle_calendar(competitions, blackout, current_date, |_| false);
}

/// As [`settle_generated_calendar`], for a calendar part of which the player has already seen:
/// `was_shown` says which moves leave a date they were shown, and those are recorded as
/// postponed. Returns exactly those moves, for the caller to announce.
pub fn settle_calendar(
    competitions: &mut [League],
    blackout: &BTreeSet<NaiveDate>,
    current_date: NaiveDate,
    mut was_shown: impl FnMut(&Move) -> bool,
) -> Vec<Move> {
    let yesterday = current_date.pred_opt().unwrap_or(current_date);
    let plan = match plan(competitions, blackout, yesterday) {
        Ok(plan) => plan,
        Err(failure) => {
            log::error!("generated calendar could not be settled: {failure:?}");
            return Vec::new();
        }
    };
    let shown: Vec<Move> = plan
        .moves
        .iter()
        .filter(|planned| was_shown(planned))
        .cloned()
        .collect();
    apply_marking(&plan, competitions, |planned| shown.contains(planned));
    shown
}
