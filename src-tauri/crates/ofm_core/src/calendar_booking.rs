//! The one world-wide booking rule: two distinct fixtures of the same club must be at least
//! two dates apart, so a full rest day separates every pair of matches.

use chrono::{Days, NaiveDate};
use domain::league::League;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FixtureKey {
    pub competition_id: String,
    pub fixture_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Booking {
    pub fixture: FixtureKey,
    pub date: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BookingError {
    UnreadableDate(FixtureKey),
    /// One fixture identity listed with different participants or dates.
    InconsistentFixture(FixtureKey),
}

/// Every fixture a club already has, keyed by club and date.
#[derive(Debug, Default)]
pub struct BookingLedger {
    by_club: BTreeMap<String, BTreeMap<NaiveDate, BTreeSet<FixtureKey>>>,
}

impl BookingLedger {
    /// Reads every fixture of every competition. A fixture present in both a competition
    /// and its legacy mirror shares one key, so it is booked once; an unreadable date is an
    /// error rather than an empty booking.
    pub fn from_competitions(competitions: &[League]) -> Result<Self, BookingError> {
        let mut ledger = Self::default();
        let mut seen: BTreeMap<FixtureKey, (&str, &str, NaiveDate)> = BTreeMap::new();
        for fixture in competitions.iter().flat_map(|c| &c.fixtures) {
            let key = FixtureKey {
                competition_id: fixture.competition_id.clone(),
                fixture_id: fixture.id.clone(),
            };
            let date = NaiveDate::parse_from_str(&fixture.date, "%Y-%m-%d")
                .map_err(|_| BookingError::UnreadableDate(key.clone()))?;
            let listing = (
                fixture.home_team_id.as_str(),
                fixture.away_team_id.as_str(),
                date,
            );
            match seen.insert(key.clone(), listing) {
                Some(earlier) if earlier != listing => {
                    return Err(BookingError::InconsistentFixture(key));
                }
                Some(_) => continue,
                None => {}
            }
            ledger.book(&fixture.home_team_id, &key, date);
            ledger.book(&fixture.away_team_id, &key, date);
        }
        Ok(ledger)
    }

    fn book(&mut self, club_id: &str, key: &FixtureKey, date: NaiveDate) {
        self.by_club
            .entry(club_id.to_owned())
            .or_default()
            .entry(date)
            .or_default()
            .insert(key.clone());
    }

    fn nearby<'a>(
        &'a self,
        club_id: &str,
        date: NaiveDate,
    ) -> impl Iterator<Item = (NaiveDate, &'a FixtureKey)> + 'a {
        let from = date.checked_sub_days(Days::new(1)).unwrap_or(date);
        let to = date.checked_add_days(Days::new(1)).unwrap_or(date);
        self.by_club
            .get(club_id)
            .into_iter()
            .flat_map(move |days| days.range(from..=to))
            .flat_map(|(booked_on, keys)| keys.iter().map(move |key| (*booked_on, key)))
    }

    /// Bookings of `club_id` on `date - 1 ..= date + 1`; may include the queried fixture itself.
    pub fn clashes(&self, club_id: &str, date: NaiveDate) -> Vec<Booking> {
        self.nearby(club_id, date)
            .map(|(date, fixture)| Booking {
                fixture: fixture.clone(),
                date,
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposal {
    pub fixture: FixtureKey,
    pub home_team_id: String,
    pub away_team_id: String,
    pub date: NaiveDate,
}

/// A fixture on a date, borrowed, so legality can be probed without building a `Proposal`.
struct Placement<'a> {
    fixture: &'a FixtureKey,
    clubs: [&'a str; 2],
    date: NaiveDate,
}

impl Proposal {
    fn placement(&self) -> Placement<'_> {
        Placement {
            fixture: &self.fixture,
            clubs: [&self.home_team_id, &self.away_team_id],
            date: self.date,
        }
    }
}

impl FlexibleMatch {
    fn placement_on(&self, date: NaiveDate) -> Placement<'_> {
        Placement {
            fixture: &self.fixture,
            clubs: [&self.home_team_id, &self.away_team_id],
            date,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    pub club_id: String,
    pub proposed: FixtureKey,
    pub existing: Booking,
}

fn conflicts_with_ledger<'a>(
    ledger: &'a BookingLedger,
    placement: &'a Placement<'a>,
) -> impl Iterator<Item = Conflict> + 'a {
    placement.clubs.into_iter().flat_map(move |club_id| {
        ledger
            .nearby(club_id, placement.date)
            .filter(move |(_, booked)| *booked != placement.fixture)
            .map(move |(booked_on, booked)| Conflict {
                club_id: club_id.to_owned(),
                proposed: placement.fixture.clone(),
                existing: Booking {
                    fixture: booked.clone(),
                    date: booked_on,
                },
            })
    })
}

fn conflicts_between(earlier: &Placement<'_>, later: &Placement<'_>) -> Vec<Conflict> {
    if earlier.fixture == later.fixture || (earlier.date - later.date).num_days().abs() >= 2 {
        return Vec::new();
    }
    later
        .clubs
        .into_iter()
        .filter(|club_id| earlier.clubs.contains(club_id))
        .map(|club_id| Conflict {
            club_id: club_id.to_owned(),
            proposed: later.fixture.clone(),
            existing: Booking {
                fixture: earlier.fixture.clone(),
                date: earlier.date,
            },
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatchRejection {
    RestRule(Vec<Conflict>),
    /// The same fixture identity appears twice, whoever the participants are.
    RepeatedFixture(FixtureKey),
}

fn first_repeated_fixture<'a>(keys: impl Iterator<Item = &'a FixtureKey>) -> Option<FixtureKey> {
    let mut seen = BTreeSet::new();
    keys.into_iter().find(|key| !seen.insert(*key)).cloned()
}

/// Checks a whole batch against the ledger and against itself; nothing is published here,
/// so a rejected batch leaves no trace.
pub fn validate_batch(
    ledger: &BookingLedger,
    proposals: &[Proposal],
) -> Result<(), BatchRejection> {
    if let Some(key) = first_repeated_fixture(proposals.iter().map(|p| &p.fixture)) {
        return Err(BatchRejection::RepeatedFixture(key));
    }
    let placements: Vec<Placement<'_>> = proposals.iter().map(Proposal::placement).collect();
    let mut conflicts: Vec<Conflict> = placements
        .iter()
        .flat_map(|placement| conflicts_with_ledger(ledger, placement))
        .collect();
    for (index, later) in placements.iter().enumerate() {
        for earlier in &placements[..index] {
            conflicts.extend(conflicts_between(earlier, later));
        }
    }
    if conflicts.is_empty() {
        Ok(())
    } else {
        Err(BatchRejection::RestRule(conflicts))
    }
}

/// A matchday whose date is flexible inside its approved window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlexibleMatch {
    pub fixture: FixtureKey,
    pub home_team_id: String,
    pub away_team_id: String,
    pub earliest: NaiveDate,
    pub latest: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanFailure {
    InvalidBatch(BatchRejection),
    NoLegalDate(FixtureKey),
}

/// Places every flexible match on its earliest legal date, or fails without placing any.
/// Hard dates are validated first and never move. Matches are placed earliest deadline
/// first, ties broken by identity, so input order cannot change the plan.
pub fn plan_flexible(
    ledger: &BookingLedger,
    fixed: &[Proposal],
    flexible: &[FlexibleMatch],
) -> Result<Vec<Proposal>, PlanFailure> {
    validate_batch(ledger, fixed).map_err(PlanFailure::InvalidBatch)?;
    let all_keys = fixed
        .iter()
        .map(|p| &p.fixture)
        .chain(flexible.iter().map(|m| &m.fixture));
    if let Some(key) = first_repeated_fixture(all_keys) {
        return Err(PlanFailure::InvalidBatch(BatchRejection::RepeatedFixture(
            key,
        )));
    }
    let mut ordered: Vec<&FlexibleMatch> = flexible.iter().collect();
    ordered.sort_by(|a, b| {
        (a.latest, a.earliest, &a.fixture).cmp(&(b.latest, b.earliest, &b.fixture))
    });

    let mut placed: Vec<Proposal> = Vec::with_capacity(ordered.len());
    for candidate in ordered {
        let date = earliest_legal_date(ledger, fixed.iter().chain(&placed), candidate)
            .ok_or_else(|| PlanFailure::NoLegalDate(candidate.fixture.clone()))?;
        placed.push(Proposal {
            fixture: candidate.fixture.clone(),
            home_team_id: candidate.home_team_id.clone(),
            away_team_id: candidate.away_team_id.clone(),
            date,
        });
    }
    Ok(placed)
}

fn earliest_legal_date<'a>(
    ledger: &BookingLedger,
    committed: impl Iterator<Item = &'a Proposal> + Clone,
    candidate: &FlexibleMatch,
) -> Option<NaiveDate> {
    candidate
        .earliest
        .iter_days()
        .take_while(|date| *date <= candidate.latest)
        .find(|date| {
            let placement = candidate.placement_on(*date);
            conflicts_with_ledger(ledger, &placement).next().is_none()
                && committed
                    .clone()
                    .all(|other| conflicts_between(&other.placement(), &placement).is_empty())
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::league::{Fixture, FixtureCompetition, FixtureStatus};

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn key(competition: &str, fixture: &str) -> FixtureKey {
        FixtureKey {
            competition_id: competition.into(),
            fixture_id: fixture.into(),
        }
    }

    fn fixture(
        competition: &str,
        id: &str,
        home: &str,
        away: &str,
        date: &str,
        kind: FixtureCompetition,
    ) -> Fixture {
        Fixture {
            id: id.into(),
            competition_id: competition.into(),
            date: date.into(),
            home_team_id: home.into(),
            away_team_id: away.into(),
            competition: kind,
            ..Default::default()
        }
    }

    fn competition_with(id: &str, fixtures: Vec<Fixture>) -> League {
        let mut league = League::new(id.into(), id.into(), 2033, &[]);
        league.fixtures = fixtures;
        league
    }

    /// La Liga: madrid host a league match on Tuesday 2033-03-01 (a Tuesday).
    fn la_liga_tuesday() -> BookingLedger {
        BookingLedger::from_competitions(&[competition_with(
            "la-liga",
            vec![fixture(
                "la-liga",
                "l1",
                "madrid",
                "betis",
                "2033-03-01",
                FixtureCompetition::League,
            )],
        )])
        .unwrap()
    }

    fn rest_conflicts(result: Result<(), BatchRejection>) -> Vec<Conflict> {
        match result {
            Err(BatchRejection::RestRule(conflicts)) => conflicts,
            other => panic!("expected a rest-rule rejection, got {other:?}"),
        }
    }

    fn continental(id: &str, home: &str, away: &str, date: NaiveDate) -> Proposal {
        Proposal {
            fixture: key("champions", id),
            home_team_id: home.into(),
            away_team_id: away.into(),
            date,
        }
    }

    /// Given a league match on Tuesday, when a continental tie is proposed the same day, then
    /// the batch is rejected naming both fixtures.
    #[test]
    fn league_and_continental_same_day_plan_is_rejected() {
        let ledger = la_liga_tuesday();
        let conflicts = rest_conflicts(validate_batch(
            &ledger,
            &[continental("c1", "madrid", "inter", day(2033, 3, 1))],
        ));
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].club_id, "madrid");
        assert_eq!(conflicts[0].proposed, key("champions", "c1"));
        assert_eq!(conflicts[0].existing.fixture, key("la-liga", "l1"));
    }

    /// Given a league match on Tuesday, when a tie is proposed on Wednesday or Monday, then it is rejected.
    #[test]
    fn league_and_continental_consecutive_day_plan_is_rejected() {
        let ledger = la_liga_tuesday();
        for date in [day(2033, 3, 2), day(2033, 2, 28)] {
            assert!(
                validate_batch(&ledger, &[continental("c1", "madrid", "inter", date)]).is_err(),
                "{date} is next to the league match"
            );
        }
    }

    /// Given a league match on Tuesday, when a tie is proposed on Thursday or the Saturday before, then it is legal.
    #[test]
    fn one_full_rest_day_allows_a_cross_competition_match() {
        let ledger = la_liga_tuesday();
        for date in [day(2033, 3, 3), day(2033, 2, 27)] {
            assert_eq!(
                validate_batch(&ledger, &[continental("c1", "madrid", "inter", date)]),
                Ok(()),
                "{date} leaves a full rest day"
            );
        }
    }

    /// Given a booked club, when any kind of fixture is proposed beside it, then the competition kind never exempts it.
    #[test]
    fn rest_rule_does_not_depend_on_the_kind_of_competition() {
        for kind in [
            FixtureCompetition::League,
            FixtureCompetition::Cup,
            FixtureCompetition::ContinentalClub,
            FixtureCompetition::InternationalClub,
            FixtureCompetition::InternationalNation,
            FixtureCompetition::Friendly,
            FixtureCompetition::FriendlyCup,
            FixtureCompetition::PreseasonTournament,
        ] {
            let ledger = BookingLedger::from_competitions(&[competition_with(
                "brasileirao",
                vec![fixture(
                    "brasileirao",
                    "b1",
                    "flamengo",
                    "santos",
                    "2033-05-10",
                    kind.clone(),
                )],
            )])
            .unwrap();
            let libertadores = continental("lib1", "santos", "river", day(2033, 5, 11));
            assert!(
                validate_batch(&ledger, &[libertadores]).is_err(),
                "{kind:?}"
            );
        }
    }

    /// Given the home club free but the away club booked the day before or after, when a fixture is proposed, then it fails for the away club.
    #[test]
    fn both_participants_and_both_neighbours_are_checked() {
        let ledger = BookingLedger::from_competitions(&[competition_with(
            "cup",
            vec![
                fixture(
                    "cup",
                    "x1",
                    "betis",
                    "getafe",
                    "2033-03-09",
                    FixtureCompetition::Cup,
                ),
                fixture(
                    "cup",
                    "x2",
                    "betis",
                    "celta",
                    "2033-03-13",
                    FixtureCompetition::Cup,
                ),
            ],
        )])
        .unwrap();
        for date in [day(2033, 3, 10), day(2033, 3, 12)] {
            let conflicts = rest_conflicts(validate_batch(
                &ledger,
                &[continental("c1", "madrid", "betis", date)],
            ));
            assert_eq!(conflicts.len(), 1);
            assert_eq!(conflicts[0].club_id, "betis");
        }
    }

    /// Given a club booked on December 31, when January 1 and January 2 are proposed, then only January 2 is legal.
    #[test]
    fn rest_rule_crosses_december_and_january() {
        let ledger = BookingLedger::from_competitions(&[competition_with(
            "league",
            vec![fixture(
                "league",
                "n1",
                "ajax",
                "psv",
                "2033-12-31",
                FixtureCompetition::League,
            )],
        )])
        .unwrap();
        assert!(
            validate_batch(
                &ledger,
                &[continental("c1", "ajax", "inter", day(2034, 1, 1))]
            )
            .is_err()
        );
        assert_eq!(
            validate_batch(
                &ledger,
                &[continental("c2", "ajax", "inter", day(2034, 1, 2))]
            ),
            Ok(())
        );
    }

    /// Given one fixture in a competition and its legacy mirror plus a distinct same-opponent cup tie,
    /// when the ledger is built, then the mirror counts once and the cup tie counts separately.
    #[test]
    fn calendar_mirrors_do_not_create_or_hide_real_conflicts() {
        let league = fixture(
            "la-liga",
            "l1",
            "madrid",
            "betis",
            "2033-03-01",
            FixtureCompetition::League,
        );
        let cup = fixture(
            "copa",
            "k1",
            "madrid",
            "betis",
            "2033-03-01",
            FixtureCompetition::Cup,
        );
        let ledger = BookingLedger::from_competitions(&[
            competition_with("la-liga", vec![league.clone()]),
            competition_with("la-liga", vec![league]),
            competition_with("copa", vec![cup]),
        ])
        .unwrap();
        let clashes = ledger.clashes("madrid", day(2033, 3, 1));
        assert_eq!(clashes.len(), 2);
        let fixtures: BTreeSet<_> = clashes.into_iter().map(|b| b.fixture).collect();
        assert_eq!(
            fixtures,
            BTreeSet::from([key("la-liga", "l1"), key("copa", "k1")])
        );
    }

    /// Given the same fixtures in a different competition order, when the ledger is built, then it is identical.
    #[test]
    fn ledger_does_not_depend_on_competition_order() {
        let a = competition_with(
            "a",
            vec![fixture(
                "a",
                "1",
                "x",
                "y",
                "2033-03-01",
                FixtureCompetition::League,
            )],
        );
        let b = competition_with(
            "b",
            vec![fixture(
                "b",
                "2",
                "x",
                "z",
                "2033-03-09",
                FixtureCompetition::Cup,
            )],
        );
        let forward = BookingLedger::from_competitions(&[a.clone(), b.clone()]).unwrap();
        let reverse = BookingLedger::from_competitions(&[b, a]).unwrap();
        assert_eq!(
            format!("{:?}", forward.by_club),
            format!("{:?}", reverse.by_club)
        );
    }

    /// Given a fixture with an unreadable date, when the ledger is built, then it is unresolved, not an empty booking.
    #[test]
    fn unreadable_fixture_date_is_unresolved_not_empty() {
        let broken = competition_with(
            "league",
            vec![fixture(
                "league",
                "bad",
                "ajax",
                "psv",
                "next tuesday",
                FixtureCompetition::League,
            )],
        );
        assert_eq!(
            BookingLedger::from_competitions(&[broken]).unwrap_err(),
            BookingError::UnreadableDate(key("league", "bad"))
        );
    }

    /// Given completed and in-progress fixtures, when a neighbouring day is proposed, then both still reserve their dates.
    #[test]
    fn played_and_live_fixtures_keep_their_reservation() {
        for status in [FixtureStatus::Completed, FixtureStatus::InProgress] {
            let mut played = fixture(
                "league",
                "p1",
                "ajax",
                "psv",
                "2033-03-01",
                FixtureCompetition::League,
            );
            played.status = status.clone();
            let ledger =
                BookingLedger::from_competitions(&[competition_with("league", vec![played])])
                    .unwrap();
            assert!(
                validate_batch(
                    &ledger,
                    &[continental("c1", "ajax", "inter", day(2033, 3, 2))]
                )
                .is_err(),
                "{status:?}"
            );
        }
    }

    /// Given two proposals in one batch that share a club on consecutive days, when validated, then the batch is rejected.
    #[test]
    fn members_of_one_batch_conflict_with_each_other() {
        let batch = [
            continental("c1", "ajax", "inter", day(2033, 4, 5)),
            continental("c2", "ajax", "porto", day(2033, 4, 6)),
            continental("c3", "benfica", "porto", day(2033, 4, 9)),
        ];
        let conflicts = rest_conflicts(validate_batch(&BookingLedger::default(), &batch));
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].club_id, "ajax");
    }

    /// Given one fixture proposed twice in a batch, even on different dates and with different
    /// participants, when validated, then the batch is rejected as a repeated fixture.
    #[test]
    fn a_fixture_proposed_twice_in_one_batch_is_rejected() {
        for second in [
            ("ajax", "inter", day(2033, 4, 20)),
            ("psv", "porto", day(2033, 4, 5)),
        ] {
            let batch = [
                continental("c1", "ajax", "inter", day(2033, 4, 5)),
                continental("c1", second.0, second.1, second.2),
            ];
            assert_eq!(
                validate_batch(&BookingLedger::default(), &batch),
                Err(BatchRejection::RepeatedFixture(key("champions", "c1")))
            );
        }
    }

    /// Given a flexible match that repeats a fixed fixture's identity, when planned, then planning fails.
    #[test]
    fn planning_rejects_a_flexible_match_that_repeats_a_fixed_fixture() {
        let fixed = [Proposal {
            fixture: key("liga", "f1"),
            ..continental("f1", "a", "b", day(2033, 3, 1))
        }];
        let flexible = [flex("f1", "c", "d", day(2033, 3, 10), day(2033, 3, 12))];
        assert_eq!(
            plan_flexible(&BookingLedger::default(), &fixed, &flexible),
            Err(PlanFailure::InvalidBatch(BatchRejection::RepeatedFixture(
                key("liga", "f1")
            )))
        );
    }

    /// Given a competition and its mirror that list one fixture on different dates, when the
    /// ledger is built, then it fails instead of reserving both dates.
    #[test]
    fn mirrors_that_disagree_about_a_fixture_are_inconsistent() {
        let original = fixture(
            "la-liga",
            "l1",
            "madrid",
            "betis",
            "2033-03-01",
            FixtureCompetition::League,
        );
        let moved = Fixture {
            date: "2033-03-08".into(),
            ..original.clone()
        };
        assert_eq!(
            BookingLedger::from_competitions(&[
                competition_with("la-liga", vec![original]),
                competition_with("la-liga", vec![moved]),
            ])
            .unwrap_err(),
            BookingError::InconsistentFixture(key("la-liga", "l1"))
        );
    }

    /// Given a club forced into two hard openers on consecutive dates, when planning is attempted, then it fails and moves nothing.
    #[test]
    fn incompatible_fixed_openers_refuse_a_calendar_plan() {
        let fixed = [
            continental("a", "boca", "river", day(2034, 2, 1)),
            continental("b", "boca", "santos", day(2034, 2, 2)),
        ];
        let flexible = [flex("f1", "x", "y", day(2034, 3, 1), day(2034, 3, 5))];
        let failure = plan_flexible(&BookingLedger::default(), &fixed, &flexible).unwrap_err();
        assert!(
            matches!(failure, PlanFailure::InvalidBatch(BatchRejection::RestRule(c)) if c.len() == 1)
        );
    }

    fn flex(
        id: &str,
        home: &str,
        away: &str,
        earliest: NaiveDate,
        latest: NaiveDate,
    ) -> FlexibleMatch {
        FlexibleMatch {
            fixture: key("liga", id),
            home_team_id: home.into(),
            away_team_id: away.into(),
            earliest,
            latest,
        }
    }

    /// Given no commitments, when a flexible match is planned, then it takes the first day of its window.
    #[test]
    fn flexible_match_takes_the_earliest_legal_day_in_its_window() {
        let plan = plan_flexible(
            &BookingLedger::default(),
            &[],
            &[flex("f1", "a", "b", day(2033, 3, 1), day(2033, 3, 8))],
        )
        .unwrap();
        assert_eq!(plan[0].date, day(2033, 3, 1));
    }

    /// Given a commitment on the window's first days, when planned, then it lands two days clear of it.
    #[test]
    fn flexible_match_skips_the_rest_days_around_a_commitment() {
        let ledger = BookingLedger::from_competitions(&[competition_with(
            "copa",
            vec![fixture(
                "copa",
                "k1",
                "a",
                "z",
                "2033-03-01",
                FixtureCompetition::Cup,
            )],
        )])
        .unwrap();
        let plan = plan_flexible(
            &ledger,
            &[],
            &[flex("f1", "a", "b", day(2033, 3, 1), day(2033, 3, 8))],
        )
        .unwrap();
        assert_eq!(plan[0].date, day(2033, 3, 3));
    }

    /// Given a window with no legal day for one of two matches, when planned, then nothing is placed.
    #[test]
    fn impossible_capacity_rejects_the_whole_batch() {
        let flexible = [
            flex("f1", "a", "b", day(2033, 3, 1), day(2033, 3, 2)),
            flex("f2", "a", "c", day(2033, 3, 1), day(2033, 3, 2)),
        ];
        let failure = plan_flexible(&BookingLedger::default(), &[], &flexible).unwrap_err();
        assert!(matches!(failure, PlanFailure::NoLegalDate(_)));
    }

    /// Given the same flexible matches in any input order, when planned, then the plan is identical.
    #[test]
    fn plan_does_not_depend_on_input_order() {
        let matches = [
            flex("f1", "a", "b", day(2033, 3, 1), day(2033, 3, 9)),
            flex("f2", "a", "c", day(2033, 3, 1), day(2033, 3, 9)),
            flex("f3", "b", "c", day(2033, 3, 1), day(2033, 3, 9)),
        ];
        let mut reversed = matches.clone();
        reversed.reverse();
        let ledger = BookingLedger::default();
        assert_eq!(
            plan_flexible(&ledger, &[], &matches),
            plan_flexible(&ledger, &[], &reversed)
        );
    }

    /// Given a club whose next half opens July 1, when the previous half's last matchday is fitted,
    /// then it ends by June 29, and a window that only allows June 30 is refused.
    #[test]
    fn paired_half_openers_reserve_the_previous_half_rest_day() {
        let opener = [continental("clausura-1", "boca", "river", day(2033, 7, 1))];
        let roomy = [flex(
            "apertura-last",
            "boca",
            "santos",
            day(2033, 6, 20),
            day(2033, 6, 30),
        )];
        let plan = plan_flexible(&BookingLedger::default(), &opener, &roomy).unwrap();
        assert_eq!(plan[0].date, day(2033, 6, 20));

        let squeezed = [flex(
            "apertura-last",
            "boca",
            "santos",
            day(2033, 6, 30),
            day(2033, 6, 30),
        )];
        assert!(matches!(
            plan_flexible(&BookingLedger::default(), &opener, &squeezed),
            Err(PlanFailure::NoLegalDate(_))
        ));
    }

    /// Given a planned batch that fails, when the ledger is read afterwards, then it is untouched.
    #[test]
    fn a_failed_plan_leaves_the_ledger_untouched() {
        let ledger = la_liga_tuesday();
        let before = format!("{:?}", ledger.by_club);
        let flexible = [flex("f1", "madrid", "x", day(2033, 3, 1), day(2033, 3, 2))];
        assert!(plan_flexible(&ledger, &[], &flexible).is_err());
        assert_eq!(format!("{:?}", ledger.by_club), before);
    }
}
