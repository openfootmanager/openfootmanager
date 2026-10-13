//! What it costs to play every match in the world with the match engine (#804, epic #803).
//!
//! Measurement only: nothing here changes how the game plays. Every test is `#[ignore]`d and
//! prints its numbers, so it runs outside the PR gate, in release mode like the shipped game:
//!
//! `cargo test --release -p season-harness --test measure_every_match -- --ignored --nocapture`
//!
//! The shipped world is 440 clubs. A season is measured twice, once as a new career plays it
//! (only the player's competitions go through the engine; the rest are settled by scoreline) and
//! once with every competition active, so the difference is the price of making every match real.

use std::time::{Duration, Instant};

use domain::league::{CompetitionType, FixtureStatus};
use ofm_core::end_of_season::EndOfSeasonSummary;
use ofm_core::game::Game;
use ofm_core::live_match_manager::play_unwatched_fixture;
use ofm_core::turn::apply_match_report;
use season_harness::driver::{RunOptions, SeasonObserver, run_seasons};
use season_harness::worlds::{Scope, WorldSpec};

const SAMPLED_FIXTURES_PER_COMPETITION: usize = 4;
const HISTORY_SEASONS: &[u32] = &[1, 5, 12];

fn club_competitions(game: &Game) -> impl Iterator<Item = &domain::league::League> {
    game.competitions
        .iter()
        .filter(|competition| competition.kind != CompetitionType::InternationalNation)
}

/// The path a day takes to play a club fixture: `play_unwatched_fixture` (which includes the
/// kick-off gate) then `apply_match_report`, with the competition in the legacy slot as the day's
/// sweep puts it there. Only the two calls are timed, not the move into the slot.
fn time_one_match(game: &mut Game, competition_index: usize, fixture_index: usize) -> Duration {
    game.league = Some(std::mem::take(&mut game.competitions[competition_index]));
    let started = Instant::now();
    let played = play_unwatched_fixture(game, fixture_index).expect("both sides can be fielded");
    apply_match_report(
        game,
        fixture_index,
        &played.home_team_id,
        &played.away_team_id,
        &played.report,
    );
    let elapsed = started.elapsed();
    game.competitions[competition_index] = game.league.take().expect("the slot is filled");
    game.sync_legacy_league();
    elapsed
}

fn percentile(sorted: &[Duration], fraction: f64) -> Duration {
    sorted[((sorted.len() - 1) as f64 * fraction).round() as usize]
}

/// Given the shipped world at the start of a season,
/// When a sample of its club fixtures is played one at a time through the production path,
/// Then the per-match cost is reported (mean, median, 95th percentile, slowest).
#[test]
#[ignore = "measurement, run by hand in release mode"]
fn the_cost_of_one_match_on_the_production_path() {
    let built = Instant::now();
    let mut game = WorldSpec::shipped(1)
        .with_scope(Scope::Everything)
        .build()
        .expect("the world builds");
    eprintln!(
        "P1 world: {} clubs, {} club competitions, built in {:.1?}",
        game.teams.len(),
        club_competitions(&game).count(),
        built.elapsed()
    );

    let mut sample: Vec<(usize, usize)> = Vec::new();
    for (competition_index, competition) in game.competitions.iter().enumerate() {
        if competition.kind == CompetitionType::InternationalNation {
            continue;
        }
        let mut scheduled: Vec<(&str, usize)> = competition
            .fixtures
            .iter()
            .enumerate()
            .filter(|(_, fixture)| fixture.status == FixtureStatus::Scheduled)
            .map(|(index, fixture)| (fixture.date.as_str(), index))
            .collect();
        scheduled.sort_unstable();
        sample.extend(
            scheduled
                .iter()
                .take(SAMPLED_FIXTURES_PER_COMPETITION)
                .map(|(_, fixture_index)| (competition_index, *fixture_index)),
        );
    }

    let mut costs: Vec<Duration> = sample
        .iter()
        .map(|&(competition_index, fixture_index)| {
            time_one_match(&mut game, competition_index, fixture_index)
        })
        .collect();
    costs.sort_unstable();
    let mean = costs.iter().sum::<Duration>() / u32::try_from(costs.len()).unwrap();
    eprintln!(
        "P1 per match ({} sampled): mean {:.2?}, median {:.2?}, p95 {:.2?}, slowest {:.2?}",
        costs.len(),
        mean,
        percentile(&costs, 0.5),
        percentile(&costs, 0.95),
        costs.last().unwrap()
    );
}

/// What one run of a season looked like from outside.
#[derive(Default)]
struct SeasonTimer {
    days: Vec<DayCost>,
    started: Option<Instant>,
    last: Option<Instant>,
    club_fixtures: usize,
    with_report: usize,
    national_fixtures: usize,
}

struct DayCost {
    date: String,
    took: Duration,
    matches_played: usize,
}

impl SeasonObserver for SeasonTimer {
    fn on_day_end(&mut self, game: &Game, _season: u32, _day: u32) {
        let now = Instant::now();
        let before = self.last.unwrap_or(now);
        self.started.get_or_insert(before);
        let today = game.clock.current_date.format("%Y-%m-%d").to_string();
        let matches_played = game
            .competitions
            .iter()
            .flat_map(|competition| &competition.fixtures)
            .filter(|fixture| fixture.date == today && fixture.status == FixtureStatus::Completed)
            .count();
        self.days.push(DayCost {
            date: today,
            took: now - before,
            matches_played,
        });
        self.last = Some(Instant::now());
    }

    fn on_before_rollover(&mut self, game: &Game, _season: u32) {
        for competition in &game.competitions {
            let completed = competition
                .fixtures
                .iter()
                .filter(|fixture| fixture.status == FixtureStatus::Completed);
            if competition.kind == CompetitionType::InternationalNation {
                self.national_fixtures += completed.count();
                continue;
            }
            for fixture in completed {
                self.club_fixtures += 1;
                if fixture
                    .result
                    .as_ref()
                    .is_some_and(|result| result.report.is_some())
                {
                    self.with_report += 1;
                }
            }
        }
    }

    fn on_season_end(&mut self, _: &Game, _: &EndOfSeasonSummary, _: u32) {}
}

fn play_a_season(scope: Scope) -> (SeasonTimer, Duration) {
    let mut game = WorldSpec::shipped(1)
        .with_scope(scope)
        .build()
        .expect("the world builds");
    let mut timer = SeasonTimer::default();
    let started = Instant::now();
    run_seasons(&mut game, RunOptions::seasons(1), &mut timer).expect("the season plays");
    (timer, started.elapsed())
}

fn report(label: &str, timer: &SeasonTimer, total: Duration) {
    let busiest_by_matches = timer
        .days
        .iter()
        .max_by_key(|day| day.matches_played)
        .expect("a season has days");
    let slowest = timer
        .days
        .iter()
        .max_by_key(|day| day.took)
        .expect("a season has days");
    eprintln!(
        "P1 {label}: season in {total:.1?} over {} days; club fixtures {} ({} with a match report, {} settled by scoreline); national-team fixtures {}",
        timer.days.len(),
        timer.club_fixtures,
        timer.with_report,
        timer.club_fixtures - timer.with_report,
        timer.national_fixtures
    );
    eprintln!(
        "P1 {label}: busiest matchday {} with {} matches took {:.2?}; slowest day {} took {:.2?} ({} matches)",
        busiest_by_matches.date,
        busiest_by_matches.matches_played,
        busiest_by_matches.took,
        slowest.date,
        slowest.took,
        slowest.matches_played
    );
}

/// Given the shipped world,
/// When a season is played as a new career plays it, and again with every competition active,
/// Then the fixtures per season, the season's time, the busiest matchday's time, and what playing
///      N pre-career history seasons at world build would cost are reported.
#[test]
#[ignore = "measurement, run by hand in release mode"]
fn a_season_of_the_shipped_world_followed_and_with_every_match_real() {
    let (followed, followed_total) = play_a_season(Scope::Followed);
    report("followed", &followed, followed_total);

    let (everything, everything_total) = play_a_season(Scope::Everything);
    report("everything", &everything, everything_total);

    for &seasons in HISTORY_SEASONS {
        eprintln!(
            "P1 projected cost of {seasons} pre-career history season(s) played for real: {:.1?} (the measured every-match-real season time times {seasons}, rollovers excluded)",
            everything_total * seasons
        );
    }
}
