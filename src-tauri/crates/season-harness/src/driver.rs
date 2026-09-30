//! The season loop: the one place the harness advances the game.
//!
//! Every day is `ofm_core::turn::process_day`, and every rollover is
//! `ofm_core::end_of_season::advance_to_next_season` — the same two entry
//! points the game calls — so a run cannot drift from what a player's game does.
//! The loop mirrors no orchestration of its own beyond "until the season is
//! complete".
//!
//! A day on which the player's own club plays can go either of two ways in the
//! game: `process_day` plays it in the batch, or the player's fixture is played
//! on the live engine and the day finished around it. [`DayPath`] picks, and the
//! live one goes through `ofm_core::matchday` — the function the application's
//! own delegate branch calls — rather than a sequence of the harness's own, so
//! there is nothing here to drift from it.

use std::fmt;

use domain::league::FixtureStatus;
use ofm_core::end_of_season::{self, EndOfSeasonSummary};
use ofm_core::game::Game;
use ofm_core::matchday;
use ofm_core::turn;

/// A season that takes longer than this has stalled. There is no timeout on the
/// CI jobs that run this, so without a cap a season that never completes would
/// hold a runner until GitHub's own ceiling. A real season, with the gap before
/// the next one, is well inside a year.
pub const DEFAULT_MAX_DAYS_PER_SEASON: u32 = 450;

/// Satisfaction the harness holds the manager above. The firing thresholds are
/// private constants in `ofm_core::firing` (the highest is 25), so this sits
/// well clear of all of them rather than copying a number that could change.
const TENURE_FLOOR: u8 = 60;

/// Watches a run. Every method has a default so an observer implements only the
/// moments it cares about.
pub trait SeasonObserver {
    /// After each simulated day. `day` counts from 1 within the season.
    fn on_day_end(&mut self, _game: &Game, _season: u32, _day: u32) {}

    /// The season is complete and has not yet been rolled over: the last moment
    /// the state that is about to be replaced can be read.
    fn on_before_rollover(&mut self, _game: &Game, _season: u32) {}

    /// After the rollover, with the summary of the season that just ended.
    fn on_season_end(&mut self, _game: &Game, _summary: &EndOfSeasonSummary, _season: u32) {}
}

/// How a day on which the player's own club has a fixture is played.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DayPath {
    /// `turn::process_day`, which plays the player's fixture with the rest.
    Batch,
    /// The fixture goes through `ofm_core::matchday::play_user_matchday_with_capture`:
    /// the live engine, with benches and both managers, finished with the day's tail.
    /// It is played with no one deciding in the match, so this covers the day's
    /// apply-and-finish sequence and not a steppable `Live` session.
    UserMatchLive,
}

#[derive(Debug, Clone, Copy)]
pub struct RunOptions {
    pub seasons: u32,
    pub day_path: DayPath,
    /// Days a season may take before the run gives up on it.
    pub max_days_per_season: u32,
    /// Hold the manager's satisfaction above the firing thresholds, before every
    /// day and the rollover. The game checks the manager's job on every day of
    /// the turn, so a run long enough will eventually sack a mid-table club's
    /// manager; and a manager with no club turns every later rollover into a
    /// silent no-op, so the harness would keep running and test nothing. This is
    /// a deliberate deviation from the game, and it is the only one.
    pub keep_manager_employed: bool,
}

impl RunOptions {
    pub fn seasons(seasons: u32) -> Self {
        Self {
            seasons,
            max_days_per_season: DEFAULT_MAX_DAYS_PER_SEASON,
            day_path: DayPath::Batch,
            keep_manager_employed: true,
        }
    }
}

#[derive(Debug)]
pub enum HarnessError {
    /// A season did not complete within [`MAX_DAYS_PER_SEASON`]. `unfinished`
    /// names each competition still holding a scheduled fixture, which is what
    /// turns "it hung" into a bug someone can file.
    Stalled {
        season: u32,
        days: u32,
        unfinished: Vec<String>,
    },
    /// The manager lost the club. With the tenure guard on this is a bug in the
    /// harness or the game, and is reported rather than run through.
    ManagerLost { season: u32, day: Option<u32> },
    /// The live path refused a day the player's club was due to play.
    LiveDay {
        season: u32,
        day: u32,
        reason: String,
    },
    /// The game refused the rollover, although the driver only asks once the
    /// season is complete.
    Rollover { season: u32, reason: String },
}

impl fmt::Display for HarnessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stalled {
                season,
                days,
                unfinished,
            } => write!(
                f,
                "season {season} did not complete in {days} days; unfinished: {unfinished:?}"
            ),
            Self::ManagerLost { season, day } => {
                write!(
                    f,
                    "the manager lost the club in season {season} (day {day:?})"
                )
            }
            Self::LiveDay {
                season,
                day,
                reason,
            } => write!(
                f,
                "the live path refused season {season} day {day}: {reason}"
            ),
            Self::Rollover { season, reason } => {
                write!(f, "the game refused to roll season {season} over: {reason}")
            }
        }
    }
}

impl std::error::Error for HarnessError {}

/// What a run did, so a caller can tell the path it asked for was the path taken.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RunSummary {
    pub days: u32,
    /// Days on which the player's own match went through `ofm_core::matchday`.
    pub live_days: u32,
}

/// Play `options.seasons` consecutive seasons, telling `observer` as it goes.
pub fn run_seasons(
    game: &mut Game,
    options: RunOptions,
    observer: &mut impl SeasonObserver,
) -> Result<RunSummary, HarnessError> {
    let mut summary = RunSummary::default();
    for season in 1..=options.seasons {
        let mut day = 0;
        while !end_of_season::is_season_complete(game) {
            day += 1;
            if day > options.max_days_per_season {
                return Err(HarnessError::Stalled {
                    season,
                    days: options.max_days_per_season,
                    unfinished: unfinished_competitions(game),
                });
            }
            if options.keep_manager_employed {
                keep_employed(game);
            }
            summary.days += 1;
            if advance_one_day(game, options.day_path, season, day)? {
                summary.live_days += 1;
            }
            require_employed(game, season, Some(day))?;
            observer.on_day_end(game, season, day);
        }

        observer.on_before_rollover(game, season);
        if options.keep_manager_employed {
            keep_employed(game);
        }
        let rollover = end_of_season::advance_to_next_season(game)
            .map_err(|reason| HarnessError::Rollover { season, reason })?;
        require_employed(game, season, None)?;
        observer.on_season_end(game, &rollover, season);
    }
    Ok(summary)
}

fn advance_one_day(
    game: &mut Game,
    path: DayPath,
    season: u32,
    day: u32,
) -> Result<bool, HarnessError> {
    let fixture = match path {
        DayPath::Batch => None,
        DayPath::UserMatchLive => users_fixture_today(game),
    };
    match fixture {
        Some((competition_index, fixture_index)) => {
            matchday::play_user_matchday_with_capture(
                game,
                Some(competition_index),
                fixture_index,
                &mut |_| {},
            )
            .map_err(|reason| HarnessError::LiveDay {
                season,
                day,
                reason,
            })?;
            Ok(true)
        }
        None => {
            turn::process_day(game);
            Ok(false)
        }
    }
}

/// The competition and fixture index of the player's own match today, if any.
fn users_fixture_today(game: &Game) -> Option<(usize, usize)> {
    let team_id = game.manager.team_id.as_deref()?;
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    game.competitions
        .iter()
        .enumerate()
        .find_map(|(competition_index, competition)| {
            competition
                .fixtures
                .iter()
                .position(|fixture| {
                    fixture.date == today
                        && fixture.status == FixtureStatus::Scheduled
                        && (fixture.home_team_id == team_id || fixture.away_team_id == team_id)
                })
                .map(|fixture_index| (competition_index, fixture_index))
        })
}

fn keep_employed(game: &mut Game) {
    game.manager.satisfaction = game.manager.satisfaction.max(TENURE_FLOOR);
    game.manager.warning_stage = 0;
}

fn require_employed(game: &Game, season: u32, day: Option<u32>) -> Result<(), HarnessError> {
    if game.manager.team_id.is_none() {
        return Err(HarnessError::ManagerLost { season, day });
    }
    Ok(())
}

fn unfinished_competitions(game: &Game) -> Vec<String> {
    game.competitions
        .iter()
        .filter_map(|competition| {
            let scheduled: Vec<&str> = competition
                .fixtures
                .iter()
                .filter(|fixture| fixture.status == FixtureStatus::Scheduled)
                .map(|fixture| fixture.date.as_str())
                .collect();
            let next = scheduled.iter().min()?;
            Some(format!(
                "{}: {} scheduled, next on {next}",
                competition.id,
                scheduled.len()
            ))
        })
        .collect()
}
