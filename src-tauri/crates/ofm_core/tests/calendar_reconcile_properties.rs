//! Property tests for the calendar sweep: random worlds, checked against the rulings.
//!
//! Every world is a pure function of a seed and a few size parameters, so a failure reproduces
//! from the printed seed and proptest shrinks it towards the smallest world that still fails.

use chrono::{Days, NaiveDate, TimeZone, Utc};
use domain::league::{
    CompetitionScope, CompetitionType, Fixture, FixtureCompetition, FixtureStatus,
    KnockoutRoundState, League, MatchResult,
};
use ofm_core::calendar_booking::too_close;
use ofm_core::calendar_reconcile::{plan, reconcile};
use ofm_core::schedule::{
    advance_knockout_competition_round, generate_knockout_cup, generate_league,
};
use ofm_core::seed::rng_from_u64;
use proptest::prelude::*;
use rand::Rng;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone)]
struct Shape {
    seed: u64,
    clubs: usize,
    leagues: usize,
    cups: usize,
    windows: usize,
    progress: usize,
}

fn shapes() -> impl Strategy<Value = Shape> {
    (
        any::<u64>(),
        4usize..=120,
        1usize..=2,
        0usize..=3,
        0usize..=4,
        0usize..=3,
    )
        .prop_map(|(seed, clubs, leagues, cups, windows, progress)| Shape {
            seed,
            clubs,
            leagues,
            cups,
            windows,
            progress,
        })
}

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

fn text(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

/// Lets club-0 win every tie it plays, so one club can win through to every final.
fn finish_current_round(cup: &mut League) {
    let Some(round) = cup
        .knockout_rounds
        .iter()
        .find(|round| !round.completed)
        .cloned()
    else {
        return;
    };
    for fixture in cup
        .fixtures
        .iter_mut()
        .filter(|fixture| round.fixture_ids.contains(&fixture.id))
    {
        let home_wins = fixture.home_team_id <= fixture.away_team_id;
        fixture.status = FixtureStatus::Completed;
        fixture.result = Some(MatchResult {
            home_goals: u8::from(home_wins),
            away_goals: u8::from(!home_wins),
            ..Default::default()
        });
    }
    advance_knockout_competition_round(cup);
}

fn subset(rng: &mut impl Rng, clubs: &[String], size: usize) -> Vec<String> {
    let mut picked = clubs.to_vec();
    for index in 0..size.min(picked.len()) {
        let swap = rng.random_range(index..picked.len());
        picked.swap(index, swap);
    }
    picked.truncate(size);
    picked
}

struct World {
    competitions: Vec<League>,
    blackout: BTreeSet<NaiveDate>,
    today: NaiveDate,
}

fn world(shape: &Shape) -> World {
    let mut rng = rng_from_u64(shape.seed);
    let clubs: Vec<String> = (0..shape.clubs).map(|i| format!("club-{i:03}")).collect();
    let mut competitions = Vec::new();
    for index in 0..shape.leagues {
        let size = rng.random_range(4..=shape.clubs.min(40)).max(2);
        let members = subset(&mut rng, &clubs, size);
        let start = Utc
            .with_ymd_and_hms(2033, 8, rng.random_range(1..=20), 0, 0, 0)
            .unwrap();
        let mut league = generate_league(&format!("League {index}"), 2033, &members, start);
        league.priority = index as u32;
        competitions.push(league);
    }
    let mut continental_entry: Option<(Vec<String>, chrono::DateTime<Utc>)> = None;
    for index in 0..shape.cups {
        let size = rng.random_range(4..=shape.clubs.min(32)).max(4);
        let members = subset(&mut rng, &clubs, size);
        let start = Utc
            .with_ymd_and_hms(
                2033,
                rng.random_range(8..=11),
                rng.random_range(1..=28),
                0,
                0,
                0,
            )
            .unwrap();
        // The second cup shares the continental cup's clubs and starts within days of it, so
        // that a domestic tie and a continental tie of one club regularly land in one rest window.
        let (members, start) = match (index, &continental_entry) {
            (1, Some((first_members, first_start))) => (
                first_members.clone(),
                *first_start + Days::new(rng.random_range(0..=3)),
            ),
            _ => (members, start),
        };
        if index == 0 {
            continental_entry = Some((members.clone(), start));
        }
        let mut cup = generate_knockout_cup(
            &format!("Cup {index}"),
            2033,
            &members,
            start,
            CompetitionType::Cup,
            CompetitionScope::Domestic,
        );
        cup.priority = 10 + index as u32;
        if index == 0 {
            cup.fixtures
                .iter_mut()
                .for_each(|f| f.competition = FixtureCompetition::ContinentalClub);
        }
        for _ in 0..rng.random_range(0..=shape.progress) {
            finish_current_round(&mut cup);
        }
        competitions.push(cup);
    }
    let mut blackout = BTreeSet::new();
    for _ in 0..shape.windows {
        let first = date(2033, 8, 1) + Days::new(rng.random_range(0..200));
        for offset in 0..rng.random_range(2..=4u64) {
            blackout.insert(first + Days::new(offset));
        }
    }
    let today = date(2033, 8, 1) + Days::new(rng.random_range(0..120));
    competitions.push(friendlies(&mut rng, &clubs));
    competitions.push(national_team_matches(&blackout));
    let played = text(today);
    for fixture in competitions.iter_mut().flat_map(|c| c.fixtures.iter_mut()) {
        if fixture.date <= played && fixture.status == FixtureStatus::Scheduled {
            fixture.status = FixtureStatus::Completed;
            fixture.result = Some(MatchResult::default());
        }
    }
    World {
        competitions,
        blackout,
        today,
    }
}

/// Filler matches between random clubs; they are the only fixtures a clash may delete.
fn friendlies(rng: &mut impl Rng, clubs: &[String]) -> League {
    let mut league = League::new("Friendlies".into(), "Friendlies".into(), 2033, &[]);
    league.priority = 100;
    for index in 0..clubs.len() {
        let pair = subset(rng, clubs, 2);
        league.fixtures.push(Fixture {
            id: format!("friendly-{index}"),
            competition_id: league.id.clone(),
            matchday: 1,
            date: text(date(2033, 8, 1) + Days::new(rng.random_range(0..200))),
            home_team_id: pair[0].clone(),
            away_team_id: pair[1].clone(),
            competition: FixtureCompetition::Friendly,
            ..Default::default()
        });
    }
    league
}

/// National-team matches played inside the international windows: no club is involved, and they
/// must never be moved or counted as a club's match.
fn national_team_matches(blackout: &BTreeSet<NaiveDate>) -> League {
    let mut league = League::new("Nations".into(), "Nations".into(), 2033, &[]);
    league.priority = 0;
    for (index, day) in blackout.iter().enumerate() {
        league.fixtures.push(Fixture {
            id: format!("nation-{index}"),
            competition_id: league.id.clone(),
            matchday: 1,
            date: text(*day),
            home_team_id: format!("nation-{}", index % 4),
            away_team_id: format!("nation-{}", (index + 1) % 4),
            competition: FixtureCompetition::InternationalNation,
            ..Default::default()
        });
    }
    league
}

fn parsed(fixture: &Fixture) -> NaiveDate {
    NaiveDate::parse_from_str(&fixture.date, "%Y-%m-%d").unwrap_or_else(|_| {
        panic!(
            "fixture {} has an invalid date {:?}",
            fixture.id, fixture.date
        )
    })
}

/// Fixtures that could not have moved: played, live, or dated today or earlier.
fn immovable_ids(competitions: &[League], today: NaiveDate) -> BTreeSet<(String, String)> {
    competitions
        .iter()
        .flat_map(|c| c.fixtures.iter().map(move |f| (c, f)))
        .filter(|(_, f)| f.status != FixtureStatus::Scheduled || parsed(f) <= today)
        .map(|(c, f)| (c.id.clone(), f.id.clone()))
        .collect()
}

fn dates_by_key(competitions: &[League]) -> BTreeMap<(String, String), String> {
    competitions
        .iter()
        .flat_map(|c| {
            c.fixtures
                .iter()
                .map(move |f| ((c.id.clone(), f.id.clone()), f.date.clone()))
        })
        .collect()
}

/// Pairs of one club's fixtures closer than the rest window where at least one could move.
fn clashes_involving_movable(competitions: &[League], today: NaiveDate) -> Vec<String> {
    let immovable = immovable_ids(competitions, today);
    let mut by_club: BTreeMap<&str, Vec<(NaiveDate, bool, &str)>> = BTreeMap::new();
    for competition in competitions {
        for fixture in &competition.fixtures {
            if fixture.competition == FixtureCompetition::InternationalNation {
                continue;
            }
            let fixed = immovable.contains(&(competition.id.clone(), fixture.id.clone()));
            for club in [&fixture.home_team_id, &fixture.away_team_id] {
                by_club
                    .entry(club)
                    .or_default()
                    .push((parsed(fixture), fixed, &fixture.id));
            }
        }
    }
    let mut found = Vec::new();
    for (club, mut rows) in by_club {
        rows.sort();
        for (i, a) in rows.iter().enumerate() {
            for b in rows[i + 1..].iter().take_while(|b| too_close(a.0, b.0)) {
                if !(a.1 && b.1) {
                    found.push(format!("{club}: {} and {}", a.2, b.2));
                }
            }
        }
    }
    found
}

/// Whether some club has a continental and a domestic cup match closer than the rest window.
fn shares_a_rest_window(competitions: &[League]) -> bool {
    let mut by_club: BTreeMap<&str, Vec<(NaiveDate, bool)>> = BTreeMap::new();
    for fixture in competitions.iter().flat_map(|c| &c.fixtures) {
        let continental = match fixture.competition {
            FixtureCompetition::ContinentalClub => true,
            FixtureCompetition::Cup => false,
            _ => continue,
        };
        for club in [&fixture.home_team_id, &fixture.away_team_id] {
            by_club
                .entry(club)
                .or_default()
                .push((parsed(fixture), continental));
        }
    }
    by_club.values().any(|rows| {
        rows.iter().any(|(a, a_continental)| {
            rows.iter()
                .any(|(b, b_continental)| a_continental != b_continental && too_close(*a, *b))
        })
    })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 40, failure_persistence: None, ..ProptestConfig::default() })]

    /// Given any generated world, when the calendar is reconciled, then every rulings-level invariant holds.
    #[test]
    fn the_sweep_keeps_every_invariant(shape in shapes()) {
        let World { mut competitions, blackout, today } = world(&shape);
        let immovable = immovable_ids(&competitions, today);
        let before = dates_by_key(&competitions);

        let planned = plan(&competitions, &blackout, today).expect("the search cap is never reached");
        let applied = reconcile(&mut competitions, &blackout, today, true).expect("the search cap is never reached");
        prop_assert_eq!(&planned, &applied, "a plan and a reconcile of the same world agree");

        // 1. no two fixtures of a club are closer than the minimum rest (history excepted)
        let clashes = clashes_involving_movable(&competitions, today);
        prop_assert!(clashes.is_empty(), "clashes: {:?}", clashes);

        // 2. every fixture has a valid date (parsed panics otherwise)
        for fixture in competitions.iter().flat_map(|c| &c.fixtures) { parsed(fixture); }

        // 3. played, live and same-day fixtures never move
        let after = dates_by_key(&competitions);
        for key in &immovable {
            prop_assert_eq!(before.get(key), after.get(key), "immovable {:?} moved", key);
        }

        // 3b. national-team matches are the calendar's fixed points, and only friendlies are dropped
        for key in before.keys().filter(|(competition, _)| competition == "Nations") {
            prop_assert_eq!(before.get(key), after.get(key), "national-team match {:?} moved", key);
        }
        for (competition, fixture) in &applied.dropped_friendlies {
            prop_assert_eq!(competition, "Friendlies", "{} was dropped but is not a friendly", fixture);
        }

        // 4. nothing is lost except friendlies, and moves only go forward
        prop_assert_eq!(before.len() - applied.dropped_friendlies.len(), after.len());
        for planned_move in &applied.moves {
            prop_assert!(planned_move.to > planned_move.from, "{:?} did not move forward", planned_move);
        }

        // 5. knockout rounds follow every round that feeds them
        for cup in &competitions {
            let round_dates: Vec<Option<(NaiveDate, NaiveDate)>> = cup
                .knockout_rounds
                .iter()
                .map(|round| {
                    let dates: Vec<NaiveDate> = cup
                        .fixtures
                        .iter()
                        .filter(|f| round.fixture_ids.contains(&f.id))
                        .map(parsed)
                        .collect();
                    Some((*dates.iter().min()?, *dates.iter().max()?))
                })
                .collect();
            for pair in round_dates.windows(2) {
                if let (Some(earlier), Some(later)) = (pair[0], pair[1]) {
                    prop_assert!(later.0 > earlier.1, "a round starts before the round that feeds it ends");
                }
            }
        }

        // 6. idempotent: a second sweep moves nothing
        let again = reconcile(&mut competitions, &blackout, today, true).expect("no cap");
        prop_assert!(again.moves.is_empty() && again.dropped_friendlies.is_empty(), "second sweep moved {:?}", again);
    }

    /// Given the same world in any order, when reconciled, then the result is identical.
    #[test]
    fn the_result_does_not_depend_on_order(shape in shapes()) {
        let World { competitions, blackout, today } = world(&shape);
        let mut forward = competitions.clone();
        let mut reversed = competitions;
        reversed.reverse();
        reconcile(&mut forward, &blackout, today, true).unwrap();
        reconcile(&mut reversed, &blackout, today, true).unwrap();
        reversed.reverse();
        prop_assert_eq!(serde_json::to_value(&forward).unwrap(), serde_json::to_value(&reversed).unwrap());
    }
}

/// A knockout cup whose rounds are authored in any order of dates: a feeding round may be dated
/// after the round it feeds, as in a corrupted or hand-authored save.
fn cup_with_shuffled_rounds(seed: u64, rounds: usize) -> League {
    let mut rng = rng_from_u64(seed);
    let mut cup = League::new("Shuffled".into(), "Shuffled".into(), 2033, &[]);
    cup.rules.format = domain::league::CompetitionFormat::Knockout;
    for round in 0..rounds {
        let ties = 1usize << (rounds - 1 - round);
        let mut state = KnockoutRoundState {
            id: format!("round-{round}"),
            ..Default::default()
        };
        for tie in 0..ties {
            let id = format!("r{round}-t{tie}");
            state.fixture_ids.push(id.clone());
            cup.fixtures.push(Fixture {
                id,
                competition_id: cup.id.clone(),
                matchday: 1,
                date: text(date(2033, 9, 1) + Days::new(rng.random_range(0..30))),
                home_team_id: format!("r{round}-t{tie}-home"),
                away_team_id: format!("r{round}-t{tie}-away"),
                competition: FixtureCompetition::Cup,
                ..Default::default()
            });
        }
        cup.knockout_rounds.push(state);
    }
    cup
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 200, failure_persistence: None, ..ProptestConfig::default() })]

    /// Given a cup whose rounds are dated in any order, when reconciled, then every round starts
    /// after the round that feeds it has ended.
    #[test]
    fn rounds_authored_out_of_order_are_put_back_in_order(seed in any::<u64>(), rounds in 2usize..=4) {
        let mut world = vec![cup_with_shuffled_rounds(seed, rounds)];
        reconcile(&mut world, &BTreeSet::new(), date(2033, 8, 1), true).unwrap();
        let cup = &world[0];
        let span = |round: &KnockoutRoundState| {
            let dates: Vec<NaiveDate> = cup
                .fixtures
                .iter()
                .filter(|f| round.fixture_ids.contains(&f.id))
                .map(parsed)
                .collect();
            (*dates.iter().min().unwrap(), *dates.iter().max().unwrap())
        };
        for pair in cup.knockout_rounds.windows(2) {
            prop_assert!(span(&pair[1]).0 > span(&pair[0]).1, "a round starts before its feeders end");
        }
    }
}

/// Given a spread of seeds, when worlds are generated, then most start with clashes and the sweep
/// moves fixtures: the properties above are not passing on empty worlds.
#[test]
fn the_generator_produces_clashes_for_the_sweep_to_resolve() {
    let mut dirty = 0;
    let mut moved = 0;
    let mut dropped = 0;
    let mut continental_overlaps = 0;
    let mut total_fixtures = 0;
    for seed in 0..40u64 {
        let shape = Shape {
            seed,
            clubs: 30,
            leagues: 2,
            cups: 3,
            windows: 3,
            progress: 2,
        };
        let World {
            mut competitions,
            blackout,
            today,
        } = world(&shape);
        total_fixtures += competitions.iter().map(|c| c.fixtures.len()).sum::<usize>();
        if !clashes_involving_movable(&competitions, today).is_empty() {
            dirty += 1;
        }
        if shares_a_rest_window(&competitions) {
            continental_overlaps += 1;
        }
        let applied = reconcile(&mut competitions, &blackout, today, true).unwrap();
        moved += applied.moves.len();
        dropped += applied.dropped_friendlies.len();
    }
    assert!(
        dropped > 0,
        "no friendly was dropped, so \"nothing is lost but friendlies\" is untested"
    );
    assert!(
        continental_overlaps >= 20,
        "only {continental_overlaps} of 40 worlds put a continental and a domestic cup tie of one club in one rest window"
    );
    assert!(
        total_fixtures > 10_000,
        "worlds are too small to be meaningful: {total_fixtures}"
    );
    assert!(
        dirty >= 30,
        "only {dirty} of 40 generated worlds contained a clash"
    );
    assert!(
        moved > 500,
        "the sweep only moved {moved} fixtures across 40 worlds"
    );
}
