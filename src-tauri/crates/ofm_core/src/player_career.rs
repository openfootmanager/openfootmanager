//! A player's career as his profile reads it: one list, newest first, of the seasons he played at
//! each club with every move written where it happened, the same rows summed by club, and his
//! totals.
//!
//! Three facts about the data decide the shape, and the screen must not rebuild any of them:
//!
//! - **The statistics season is the world's.** Every player's stats are recorded and reset at one
//!   rollover, the rollover division's, so that division's calendar is everyone's statistics season
//!   (`statistics_season`). The season in progress began on the day the stats were last reset
//!   (`Game::last_season_rollover_on`), which is not always the calendar's start; earlier seasons
//!   began on the calendar's start month and day.
//! - **A career row is one club stint**, and a stint with no appearance has no row, so the nth row of
//!   a season is not the nth visit in his ledger. A row is matched to a visit by club, in order
//!   (`place_rows`).
//! - **The season in progress is split by club** (`SeasonStints`), so a player who changed club this
//!   season has a row for each club, not one total on whichever holds him.

use std::collections::HashMap;

use chrono::{Datelike, NaiveDate};
use domain::career_stints::SeasonStints;
use domain::player::{
    ActiveLoan, CareerEntry, Player, PlayerMovementEntry, PlayerMovementKind, PlayerSeasonStats,
};
use serde::Serialize;
use ts_rs::TS;

use crate::game::Game;

/// One club's share of one season: a closed season's row, or a stint of the season in progress.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct CareerSeasonRow {
    pub key: String,
    /// The world's statistics season (the year it starts: 2026 is "2026/27").
    pub season: u32,
    /// Empty for a spell at no club: the screen words it in the player's language.
    pub club_id: String,
    pub club_name: String,
    pub appearances: u32,
    pub goals: u32,
    pub assists: u32,
    /// The stint at the club he is with now, in the season in progress.
    pub in_progress: bool,
    /// He was at the club on loan.
    pub on_loan: bool,
}

/// A move between clubs, written where it happened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct CareerMoveLine {
    pub key: String,
    pub date: String,
    pub movement: PlayerMovementKind,
    pub from_club_name: Option<String>,
    pub to_club_name: Option<String>,
    pub fee: Option<u64>,
    pub loan_end: Option<String>,
}

/// A row of the season-by-season table: a season at a club, or a move between two of them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CareerLine {
    Season(CareerSeasonRow),
    Move(CareerMoveLine),
}

/// The same rows summed by club.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct CareerClubTotals {
    pub key: String,
    pub club_id: String,
    pub club_name: String,
    pub first_season: u32,
    pub last_season: u32,
    pub appearances: u32,
    pub goals: u32,
    pub assists: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, TS)]
pub struct CareerTotals {
    pub appearances: u32,
    pub goals: u32,
    pub assists: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct PlayerCareer {
    /// Newest first: season rows with each move written as a line between them.
    pub by_season: Vec<CareerLine>,
    /// Newest first by the last season at the club, the latest stint first among clubs that tie.
    pub by_club: Vec<CareerClubTotals>,
    pub totals: CareerTotals,
}

/// The world's statistics season, which rolls over with the user's division: the calendar every
/// player's career rows are stamped with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StatisticsSeason {
    /// The season in progress.
    current: u32,
    /// The day its stats were reset, when the game knows it: the real boundary, which can differ
    /// from the calendar's start (a rollover is on the day the season ended).
    current_began_on: Option<NaiveDate>,
    start_month: u8,
    start_day: u8,
}

impl StatisticsSeason {
    /// The calendar's start of `season`: the generator's own season-start rule
    /// (`generator::competition_def::date_utc`), which takes a day the month lacks (31 February)
    /// as the month's last and clamps an out-of-range month or day.
    fn calendar_start(&self, season: u32) -> NaiveDate {
        let year = i32::try_from(season)
            .unwrap_or(i32::MAX)
            .min(NaiveDate::MAX.year());
        crate::generator::competition_def::date_utc(year, self.start_month, self.start_day)
            .date_naive()
    }

    /// The first day `season`'s stats counted: the real reset day for the season in progress,
    /// the calendar's start for any other. The game keeps only the latest reset day
    /// (`last_season_rollover_on`, overwritten at every rollover) and the career rows carry no
    /// dates, so a past season that began off the calendar (a rollover on the day the previous
    /// season ended) is placed by the calendar's start: a stint that began between the real reset
    /// and the calendar's start can sort after a move that preceded it. Recovering it needs the
    /// reset day stored with each career row (#1402).
    fn began_on(&self, season: u32) -> NaiveDate {
        match self.current_began_on {
            Some(day) if season == self.current => day,
            Some(_) | None => self.calendar_start(season),
        }
    }

    /// The days `season`'s stats cover: from the day they began up to, not including, the day the
    /// next season's began. The season in progress has no end.
    fn window(&self, season: u32) -> (NaiveDate, Option<NaiveDate>) {
        let end = (season < self.current).then(|| self.began_on(season + 1));
        (self.began_on(season), end)
    }
}

/// A career row and whether it is the stint he is on now.
#[derive(Debug, Clone, PartialEq)]
struct StintRow {
    entry: CareerEntry,
    in_progress: bool,
}

fn parse_day(text: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(text.get(..10)?, "%Y-%m-%d").ok()
}

/// The names a stint at no club has been stored under: nothing (newer saves), or the English
/// literal or the translation key an older save wrote (the screens' `storedTeamLabel` reads the
/// same markers). The projection sends one form, nothing, so every no-club stint groups as one and
/// the screen words it; a stint at a historical club known by name only keeps its name.
fn is_no_club_marker(name: &str) -> bool {
    matches!(name.trim(), "" | "Free Agent" | "common.freeAgent")
}

fn stored_club_name(entry: &CareerEntry) -> String {
    if entry.team_id.is_empty() && is_no_club_marker(&entry.team_name) {
        String::new()
    } else {
        entry.team_name.clone()
    }
}

/// A spell at one club, from the day he arrived to the day he left (`None` while he is there).
struct Visit {
    club_id: String,
    from: NaiveDate,
    until: Option<NaiveDate>,
    on_loan: bool,
}

impl Visit {
    fn overlaps(&self, start: NaiveDate, end: Option<NaiveDate>) -> bool {
        end.is_none_or(|end| self.from < end) && self.until.is_none_or(|until| until > start)
    }
}

fn close_open(visits: &mut [Visit], club: Option<&str>, day: NaiveDate) {
    let Some(club) = club else { return };
    if let Some(open) = visits
        .iter_mut()
        .rev()
        .find(|visit| visit.club_id == club && visit.until.is_none())
    {
        open.until = Some(day);
    }
}

fn open_visit(visits: &mut Vec<Visit>, club: Option<&str>, day: NaiveDate, on_loan: bool) {
    if let Some(club) = club {
        visits.push(Visit {
            club_id: club.to_string(),
            from: day,
            until: None,
            on_loan,
        });
    }
}

/// The opening contract starts his first spell at its club, unless he is already there.
fn open_first_visit(visits: &mut Vec<Visit>, club: Option<&str>, day: NaiveDate) {
    let already_there = club.is_some_and(|club| {
        visits
            .iter()
            .any(|visit| visit.club_id == club && visit.until.is_none())
    });
    if !already_there {
        open_visit(visits, club, day, false);
    }
}

/// The clubs he has been at, in order, from his ledger. Where he plays changes exactly when
/// `changes_where_he_plays` says it does; a loan's purchase ends the loan without moving him, and
/// the opening contract starts his first spell. An entry with no date cannot place a visit.
///
/// `active_loan` is authoritative for a loan in progress: a save older than the ledger's loan
/// entries keeps a loanee with the loan and no `LoanStart`, and his stint at the borrower is then
/// seeded from it (the parent's spell ends the day the loan began).
fn visits_from(movements: &[PlayerMovementEntry], active_loan: Option<&ActiveLoan>) -> Vec<Visit> {
    let mut visits = Vec::new();
    for entry in movements {
        let from = entry.from_team_id.as_deref();
        let to = entry.to_team_id.as_deref();
        let Some(day) = parse_day(&entry.date) else {
            // A migrated save's opening contract has no date, but his first spell at that club is
            // real and began before anything the ledger dates: seed it from the beginning of time.
            if entry.kind == PlayerMovementKind::InitialContract {
                open_first_visit(&mut visits, to, NaiveDate::MIN);
            }
            continue;
        };
        if entry.kind.changes_where_he_plays() {
            close_open(&mut visits, from, day);
            open_visit(
                &mut visits,
                to,
                day,
                entry.kind == PlayerMovementKind::LoanStart,
            );
        } else if entry.kind == PlayerMovementKind::InitialContract {
            open_first_visit(&mut visits, to, day);
        } else if entry.kind == PlayerMovementKind::LoanToBuy {
            close_open(&mut visits, to, day);
            open_visit(&mut visits, to, day, false);
        } else if entry.kind == PlayerMovementKind::Retired {
            close_open(&mut visits, from, day);
        }
    }
    if let Some(loan) = active_loan
        && let Some(start) = parse_day(&loan.start_date)
    {
        let seeded = visits.iter().any(|visit| {
            visit.club_id == loan.loan_team_id && visit.on_loan && visit.until.is_none()
        });
        if !seeded {
            close_open(&mut visits, Some(&loan.parent_team_id), start);
            open_visit(&mut visits, Some(&loan.loan_team_id), start, true);
        }
    }
    visits
}

/// Where a season row falls in time, and whether it was a loan.
struct Placing {
    when: NaiveDate,
    on_loan: bool,
}

/// Each row's place: the day he arrived at the club for that spell, or the first day of the
/// season if he was already there. Rows of one season are in the order they happened, and each
/// takes the first visit at its club after the one the row before it took: a stint with no
/// appearance has no row, so the rows are a subsequence of the season's visits, not all of them.
///
/// Only a club he visits twice in one season with a gap he played none of is ambiguous (#1402): the row
/// takes the first visit, which is right unless the first spell was the one with no appearance.
fn place_rows(rows: &[StintRow], visits: &[Visit], seasons: &StatisticsSeason) -> Vec<Placing> {
    let mut cursors: HashMap<u32, usize> = HashMap::new();
    rows.iter()
        .map(|row| {
            let (start, end) = seasons.window(row.entry.season);
            let cursor = cursors.entry(row.entry.season).or_insert(0);
            let overlapping: Vec<&Visit> = visits
                .iter()
                .filter(|visit| visit.overlaps(start, end))
                .collect();
            let found = overlapping
                .iter()
                .enumerate()
                .skip(*cursor)
                .find(|(_, visit)| visit.club_id == row.entry.team_id);
            match found {
                Some((index, visit)) => {
                    *cursor = index + 1;
                    Placing {
                        when: visit.from.max(start),
                        on_loan: visit.on_loan,
                    }
                }
                None => Placing {
                    when: start,
                    on_loan: false,
                },
            }
        })
        .collect()
}

/// A move as a line of the career. A renewal is a contract matter and a loan's return is carried
/// by the loan's own line, so neither is one; a move with no date cannot be placed.
fn move_line(
    entry: &PlayerMovementEntry,
    position: usize,
    club_name: &dyn Fn(&str) -> Option<String>,
) -> Option<(NaiveDate, CareerMoveLine)> {
    let is_a_line = matches!(
        entry.kind,
        PlayerMovementKind::InitialContract
            | PlayerMovementKind::PermanentTransfer
            | PlayerMovementKind::LoanStart
            | PlayerMovementKind::LoanToBuy
            | PlayerMovementKind::FreeAgentSigning
            | PlayerMovementKind::Released
            | PlayerMovementKind::Retired
    );
    let day = parse_day(&entry.date)?;
    if !is_a_line {
        return None;
    }
    let name_of = |name: Option<&str>, id: Option<&str>| {
        name.filter(|name| !name.is_empty())
            .map(str::to_string)
            .or_else(|| id.and_then(club_name))
    };
    Some((
        day,
        CareerMoveLine {
            key: format!("{}-{}", entry.date, position),
            date: day.format("%Y-%m-%d").to_string(),
            movement: entry.kind.clone(),
            from_club_name: name_of(
                entry.from_team_name.as_deref(),
                entry.from_team_id.as_deref(),
            ),
            to_club_name: name_of(entry.to_team_name.as_deref(), entry.to_team_id.as_deref()),
            fee: entry.fee.filter(|fee| *fee > 0),
            loan_end: entry.loan_end_date.clone(),
        },
    ))
}

fn club_totals<'a>(rows: impl Iterator<Item = &'a CareerSeasonRow>) -> Vec<CareerClubTotals> {
    // Each club with the position of its latest row, to order clubs that share their seasons.
    let mut clubs: Vec<(CareerClubTotals, usize)> = Vec::new();
    for (order, row) in rows.enumerate() {
        let key = if row.club_id.is_empty() {
            format!("name:{}", row.club_name)
        } else {
            row.club_id.clone()
        };
        if let Some((club, latest)) = clubs.iter_mut().find(|(club, _)| club.key == key) {
            club.first_season = club.first_season.min(row.season);
            club.last_season = club.last_season.max(row.season);
            club.appearances += row.appearances;
            club.goals += row.goals;
            club.assists += row.assists;
            *latest = order;
        } else {
            clubs.push((
                CareerClubTotals {
                    key,
                    club_id: row.club_id.clone(),
                    club_name: row.club_name.clone(),
                    first_season: row.season,
                    last_season: row.season,
                    appearances: row.appearances,
                    goals: row.goals,
                    assists: row.assists,
                },
                order,
            ));
        }
    }
    clubs.sort_by(|(left, left_latest), (right, right_latest)| {
        right
            .last_season
            .cmp(&left.last_season)
            .then(right_latest.cmp(left_latest))
            .then(right.first_season.cmp(&left.first_season))
    });
    clubs.into_iter().map(|(club, _)| club).collect()
}

/// The career as one list from its parts: his stints (closed seasons then the season in progress,
/// in the order they happened), his ledger, the statistics season and a way to name a club by id.
///
/// On one day a move comes before the stint it began, so the line sits between the season he
/// arrived and the one he left.
fn build_career(
    rows: &[StintRow],
    movements: &[PlayerMovementEntry],
    seasons: &StatisticsSeason,
    club_name: &dyn Fn(&str) -> Option<String>,
    active_loan: Option<&ActiveLoan>,
) -> PlayerCareer {
    let visits = visits_from(movements, active_loan);
    let placings = place_rows(rows, &visits, seasons);
    let mut chronology: Vec<(NaiveDate, u8, usize, CareerLine)> = Vec::new();
    for (position, (row, placing)) in rows.iter().zip(placings).enumerate() {
        let entry = &row.entry;
        chronology.push((
            placing.when,
            1,
            position,
            CareerLine::Season(CareerSeasonRow {
                key: format!("{}-{}-{}", entry.season, entry.team_id, position),
                season: entry.season,
                club_id: entry.team_id.clone(),
                club_name: stored_club_name(entry),
                appearances: entry.appearances,
                goals: entry.goals,
                assists: entry.assists,
                in_progress: row.in_progress,
                on_loan: placing.on_loan,
            }),
        ));
    }
    for (position, entry) in movements.iter().enumerate() {
        if let Some((day, line)) = move_line(entry, position, club_name) {
            chronology.push((day, 0, position, CareerLine::Move(line)));
        }
    }
    chronology.sort_by_key(|(when, rank, position, _)| (*when, *rank, *position));

    let season_rows = || {
        chronology.iter().filter_map(|(_, _, _, line)| match line {
            CareerLine::Season(row) => Some(row),
            CareerLine::Move(_) => None,
        })
    };
    let by_club = club_totals(season_rows());
    let totals = season_rows().fold(CareerTotals::default(), |sum, row| CareerTotals {
        appearances: sum.appearances + row.appearances,
        goals: sum.goals + row.goals,
        assists: sum.assists + row.assists,
    });
    let by_season = chronology
        .into_iter()
        .rev()
        .map(|(_, _, _, line)| line)
        .collect();
    PlayerCareer {
        by_season,
        by_club,
        totals,
    }
}

/// The season in progress as rows: a row for each club he has played for since it began (his stints
/// are closed at every move), and one for the club he is with now even before his first
/// appearance there. A player with no club and no appearance has none.
fn in_progress_rows(
    stints: &SeasonStints,
    stats: &PlayerSeasonStats,
    season: u32,
    club_id: &str,
    club_name: &str,
) -> Vec<StintRow> {
    let mut entries = stints.career_entries(season, stats, club_id, club_name);
    let has_current_stint = entries.len() > stints.closed.len();
    if !has_current_stint && !club_id.is_empty() {
        entries.push(CareerEntry {
            season,
            team_id: club_id.to_string(),
            team_name: club_name.to_string(),
            appearances: 0,
            goals: 0,
            assists: 0,
        });
    }
    let current_exists = has_current_stint || !club_id.is_empty();
    let last = entries.len().checked_sub(1);
    entries
        .into_iter()
        .enumerate()
        .map(|(index, entry)| StintRow {
            entry,
            in_progress: current_exists && Some(index) == last,
        })
        .collect()
}

/// The world's statistics season: the rollover division's calendar, and the day the stats were
/// last reset. Without a division (a world with no league) the calendar is the default's.
fn statistics_season(game: &Game) -> StatisticsSeason {
    let began = game.last_season_rollover_on.as_deref().and_then(parse_day);
    match crate::end_of_season::rollover_division(game) {
        Some(league) => StatisticsSeason {
            current: league.season,
            current_began_on: began,
            start_month: league.season_start_month,
            start_day: league.season_start_day,
        },
        None => StatisticsSeason {
            current: u32::try_from(game.clock.current_date.year()).unwrap_or_default(),
            current_began_on: began,
            start_month: 8,
            start_day: 1,
        },
    }
}

/// One player's career for his profile.
pub fn player_career(game: &Game, player: &Player) -> PlayerCareer {
    let seasons = statistics_season(game);
    let club_id = player.team_id.clone().unwrap_or_default();
    let name_of = |id: &str| {
        game.teams
            .iter()
            .find(|team| team.id == id)
            .map(|team| team.name.clone())
    };
    let club_name = name_of(&club_id).unwrap_or_default();
    let mut rows: Vec<StintRow> = player
        .career
        .iter()
        .map(|entry| StintRow {
            entry: entry.clone(),
            in_progress: false,
        })
        .collect();
    rows.extend(in_progress_rows(
        &player.season_stints,
        &player.stats,
        seasons.current,
        &club_id,
        &club_name,
    ));
    build_career(
        &rows,
        &player.movement_history,
        &seasons,
        &name_of,
        player.active_loan.as_ref(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use domain::career_stints::{ClosedStint, StintTally};
    use domain::league::League;
    use domain::manager::Manager;
    use domain::player::Position;

    use crate::clock::GameClock;
    use crate::test_support::uniform_attributes;

    fn day(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap()
    }

    /// 2026 is the season in progress; the calendar starts on 1 of `month`.
    fn calendar(month: u8) -> StatisticsSeason {
        StatisticsSeason {
            current: 2026,
            current_began_on: None,
            start_month: month,
            start_day: 1,
        }
    }

    fn stint(season: u32, club: &str, appearances: u32, goals: u32, assists: u32) -> StintRow {
        StintRow {
            entry: CareerEntry {
                season,
                team_id: club.to_string(),
                team_name: club.to_string(),
                appearances,
                goals,
                assists,
            },
            in_progress: false,
        }
    }

    fn now(season: u32, club: &str, appearances: u32, goals: u32, assists: u32) -> StintRow {
        StintRow {
            in_progress: true,
            ..stint(season, club, appearances, goals, assists)
        }
    }

    fn moved(
        date: &str,
        kind: PlayerMovementKind,
        from: Option<&str>,
        to: Option<&str>,
    ) -> PlayerMovementEntry {
        let mut entry = PlayerMovementEntry::new(date, kind);
        entry.from_team_id = from.map(str::to_string);
        entry.from_team_name = from.map(str::to_string);
        entry.to_team_id = to.map(str::to_string);
        entry.to_team_name = to.map(str::to_string);
        entry
    }

    fn career_of(
        rows: &[StintRow],
        movements: &[PlayerMovementEntry],
        seasons: &StatisticsSeason,
    ) -> PlayerCareer {
        build_career(rows, movements, seasons, &|_| None, None)
    }

    /// The list read compactly: a season row as "2025 north 33" (" now" when in progress), a move
    /// as "move:<kind>".
    fn read(career: &PlayerCareer) -> Vec<String> {
        career
            .by_season
            .iter()
            .map(|line| match line {
                CareerLine::Season(row) => format!(
                    "{} {} {}{}",
                    row.season,
                    row.club_id,
                    row.appearances,
                    if row.in_progress { " now" } else { "" }
                ),
                CareerLine::Move(line) => format!("move:{:?}", line.movement),
            })
            .collect()
    }

    fn season_row<'a>(career: &'a PlayerCareer, season: u32, club: &str) -> &'a CareerSeasonRow {
        career
            .by_season
            .iter()
            .find_map(|line| match line {
                CareerLine::Season(row) if row.season == season && row.club_id == club => Some(row),
                CareerLine::Season(_) | CareerLine::Move(_) => None,
            })
            .unwrap_or_else(|| panic!("no {season} row for {club} in {career:?}"))
    }

    // Daniel's career as board Career A draws it: a first contract in 2020, a loan in January 2024,
    // a transfer in July 2024 and the season in progress.
    fn board_rows() -> Vec<StintRow> {
        vec![
            stint(2020, "ashford", 9, 1, 0),
            stint(2021, "ashford", 28, 9, 3),
            stint(2022, "ashford", 34, 17, 5),
            stint(2023, "ashford", 17, 6, 2),
            stint(2023, "westgate", 14, 8, 3),
            stint(2024, "northbridge", 36, 16, 7),
            stint(2025, "northbridge", 33, 15, 8),
            now(2026, "northbridge", 27, 19, 6),
        ]
    }

    fn board_moves() -> Vec<PlayerMovementEntry> {
        let mut transfer = moved(
            "2024-07-01",
            PlayerMovementKind::PermanentTransfer,
            Some("ashford"),
            Some("northbridge"),
        );
        transfer.fee = Some(9_500_000);
        let mut loan = moved(
            "2024-01-02",
            PlayerMovementKind::LoanStart,
            Some("ashford"),
            Some("westgate"),
        );
        loan.loan_end_date = Some("2024-06-30".to_string());
        vec![
            moved(
                "2020-07-01",
                PlayerMovementKind::InitialContract,
                None,
                Some("ashford"),
            ),
            loan,
            moved(
                "2024-06-30",
                PlayerMovementKind::LoanReturn,
                Some("westgate"),
                Some("ashford"),
            ),
            transfer,
        ]
    }

    /// Given the board's career: a first contract, a loan, a transfer and the season in progress
    /// When the career is built
    /// Then the seasons are newest first with each move written where it happened: the loan line
    ///      between the loan club's season and the one he left, the transfer line between his first
    ///      season at the new club and his last at the old one, the first contract at the bottom
    ///      (a loan's return is no line of its own: the loan line carries its end)
    #[test]
    fn each_move_is_written_where_it_happened_with_the_seasons_newest_first() {
        let career = career_of(&board_rows(), &board_moves(), &calendar(8));

        assert_eq!(
            read(&career),
            [
                "2026 northbridge 27 now",
                "2025 northbridge 33",
                "2024 northbridge 36",
                "move:PermanentTransfer",
                "2023 westgate 14",
                "move:LoanStart",
                "2023 ashford 17",
                "2022 ashford 34",
                "2021 ashford 28",
                "2020 ashford 9",
                "move:InitialContract",
            ]
        );
    }

    /// Given the board's career
    /// When the career is built
    /// Then the transfer line carries its fee, clubs and day, the loan line its end, and the loan
    ///      season is the only one marked as a loan
    #[test]
    fn a_line_carries_its_fee_clubs_and_loan_end_and_only_the_loan_season_is_a_loan() {
        let career = career_of(&board_rows(), &board_moves(), &calendar(8));

        let lines: Vec<&CareerMoveLine> = career
            .by_season
            .iter()
            .filter_map(|line| match line {
                CareerLine::Move(line) => Some(line),
                CareerLine::Season(_) => None,
            })
            .collect();
        let transfer = lines
            .iter()
            .find(|line| line.movement == PlayerMovementKind::PermanentTransfer)
            .unwrap();
        assert_eq!(transfer.fee, Some(9_500_000));
        assert_eq!(transfer.from_club_name.as_deref(), Some("ashford"));
        assert_eq!(transfer.to_club_name.as_deref(), Some("northbridge"));
        assert_eq!(transfer.date, "2024-07-01");
        let loan = lines
            .iter()
            .find(|line| line.movement == PlayerMovementKind::LoanStart)
            .unwrap();
        assert_eq!(loan.loan_end.as_deref(), Some("2024-06-30"));
        let on_loan: Vec<&str> = career
            .by_season
            .iter()
            .filter_map(|line| match line {
                CareerLine::Season(row) if row.on_loan => Some(row.club_id.as_str()),
                CareerLine::Season(_) | CareerLine::Move(_) => None,
            })
            .collect();
        assert_eq!(on_loan, ["westgate"]);
    }

    /// Given the board's career
    /// When the career is built
    /// Then the totals add every row, the season in progress included, and the by-club rows sum
    ///      the same figures: one row per club with its first and last season, newest first
    #[test]
    fn the_totals_and_the_club_rows_sum_every_season_row() {
        let career = career_of(&board_rows(), &board_moves(), &calendar(8));

        assert_eq!(
            career.totals,
            CareerTotals {
                appearances: 198,
                goals: 91,
                assists: 34
            }
        );
        let clubs: Vec<(&str, u32, u32, u32, u32, u32)> = career
            .by_club
            .iter()
            .map(|club| {
                (
                    club.club_id.as_str(),
                    club.first_season,
                    club.last_season,
                    club.appearances,
                    club.goals,
                    club.assists,
                )
            })
            .collect();
        assert_eq!(
            clubs,
            [
                ("northbridge", 2024, 2026, 96, 50, 21),
                ("westgate", 2023, 2023, 14, 8, 3),
                ("ashford", 2020, 2023, 88, 33, 10),
            ]
        );
    }

    /// Given a league whose season starts on 1 March and a loan in May 2024, in the middle of its
    ///       2024 season, between his stint at the first club and his stint at the loan club
    /// When the career is built
    /// Then the loan line sits between those two 2024 rows: the season's own start (1 March 2024)
    ///      puts the first club's row before the loan, where a 1 July calendar would put it after
    #[test]
    fn a_move_is_placed_by_the_calendar_of_the_world_season_not_by_july() {
        let rows = [stint(2024, "first", 10, 2, 1), stint(2024, "loan", 5, 1, 0)];
        let movements = [
            moved(
                "2023-08-01",
                PlayerMovementKind::InitialContract,
                None,
                Some("first"),
            ),
            moved(
                "2024-05-10",
                PlayerMovementKind::LoanStart,
                Some("first"),
                Some("loan"),
            ),
        ];

        let career = career_of(&rows, &movements, &calendar(3));

        assert_eq!(
            read(&career),
            [
                "2024 loan 5",
                "move:LoanStart",
                "2024 first 10",
                "move:InitialContract"
            ]
        );
    }

    /// Given a season at club A before a loan to B (no appearance for B, so no row) and after it,
    ///       with A's two stints as the only rows
    /// When the career is built
    /// Then the second A row is the spell after the loan, not the loan: it is not marked as a loan
    #[test]
    fn a_stint_with_no_appearance_does_not_shift_later_rows_onto_the_wrong_visit() {
        let rows = [stint(2025, "a", 5, 1, 0), stint(2025, "a", 7, 2, 1)];
        let movements = [
            moved(
                "2024-07-01",
                PlayerMovementKind::InitialContract,
                None,
                Some("a"),
            ),
            moved(
                "2025-10-01",
                PlayerMovementKind::LoanStart,
                Some("a"),
                Some("b"),
            ),
            moved(
                "2026-01-05",
                PlayerMovementKind::LoanReturn,
                Some("b"),
                Some("a"),
            ),
        ];

        let career = career_of(&rows, &movements, &calendar(8));

        assert!(
            career.by_season.iter().all(|line| match line {
                CareerLine::Season(row) => !row.on_loan,
                CareerLine::Move(_) => true,
            }),
            "{career:?}"
        );
        assert_eq!(
            read(&career),
            [
                "2025 a 7",
                "move:LoanStart",
                "2025 a 5",
                "move:InitialContract"
            ]
        );
    }

    /// Given club A's first spell of a season with no appearance, a loan to B and a return to A,
    ///       with a single A row (the second spell's)
    /// When the career is built
    /// Then the row takes the first A visit: the known ambiguity (the rows carry no dates), which
    ///      shows as the row sitting before the loan line instead of after it (order only, the figures
    ///      are right; exact placement needs the stint's dates persisted, #1402)
    #[test]
    fn an_unplayed_first_spell_at_a_club_is_a_known_ambiguity() {
        let rows = [stint(2025, "a", 7, 2, 1)];
        let movements = [
            moved(
                "2025-08-01",
                PlayerMovementKind::FreeAgentSigning,
                None,
                Some("a"),
            ),
            moved(
                "2025-09-01",
                PlayerMovementKind::LoanStart,
                Some("a"),
                Some("b"),
            ),
            moved(
                "2026-01-05",
                PlayerMovementKind::LoanReturn,
                Some("b"),
                Some("a"),
            ),
        ];

        let career = career_of(&rows, &movements, &calendar(8));

        // The 7 appearances were after the loan, but the row cannot say so: it sits before it.
        assert_eq!(
            read(&career),
            ["move:LoanStart", "2025 a 7", "move:FreeAgentSigning"]
        );
    }

    /// Given a loan that the borrower then buys, and a season after it
    /// When the career is built
    /// Then the season of the loan is a loan, the purchase ends the loan without moving him, and
    ///      the next season at that club is not a loan
    #[test]
    fn a_loans_purchase_ends_the_loan_without_moving_him() {
        let rows = [
            stint(2023, "parent", 20, 3, 2),
            stint(2023, "borrower", 5, 1, 0),
            stint(2024, "borrower", 30, 9, 4),
        ];
        let movements = [
            moved(
                "2022-08-01",
                PlayerMovementKind::InitialContract,
                None,
                Some("parent"),
            ),
            moved(
                "2024-01-10",
                PlayerMovementKind::LoanStart,
                Some("parent"),
                Some("borrower"),
            ),
            moved(
                "2024-03-01",
                PlayerMovementKind::LoanToBuy,
                Some("parent"),
                Some("borrower"),
            ),
        ];

        let career = career_of(&rows, &movements, &calendar(8));

        assert!(season_row(&career, 2023, "borrower").on_loan);
        assert!(!season_row(&career, 2024, "borrower").on_loan);
        assert!(!season_row(&career, 2023, "parent").on_loan);
    }

    /// Given two clubs that share the only season he played at either, the earlier one first
    /// When the career is summed by club
    /// Then the club he played for later is listed first
    #[test]
    fn clubs_that_share_their_seasons_list_the_latest_stint_first() {
        let rows = [
            stint(2025, "early", 10, 1, 0),
            stint(2025, "late", 12, 2, 1),
        ];

        let career = career_of(&rows, &[], &calendar(8));

        let clubs: Vec<&str> = career
            .by_club
            .iter()
            .map(|club| club.club_id.as_str())
            .collect();
        assert_eq!(clubs, ["late", "early"]);
    }

    /// Given a player who changed club in the season in progress: 20 appearances at South (closed
    ///       when he left) and 4 at North since
    /// When his season in progress is built and summed by club
    /// Then each club has its own row and figures, and only North's is in progress
    #[test]
    fn the_season_in_progress_is_split_by_club() {
        let stints = SeasonStints {
            stats_at_arrival: StintTally {
                appearances: 20,
                goals: 5,
                assists: 2,
            },
            closed: vec![ClosedStint {
                team_id: "south".to_string(),
                team_name: "south".to_string(),
                tally: StintTally {
                    appearances: 20,
                    goals: 5,
                    assists: 2,
                },
            }],
        };
        let stats = PlayerSeasonStats {
            appearances: 24,
            goals: 6,
            assists: 2,
            ..PlayerSeasonStats::default()
        };
        let movements = [moved(
            "2027-01-10",
            PlayerMovementKind::PermanentTransfer,
            Some("south"),
            Some("north"),
        )];

        let rows = in_progress_rows(&stints, &stats, 2026, "north", "north");
        let career = career_of(&rows, &movements, &calendar(8));

        assert_eq!(
            read(&career),
            [
                "2026 north 4 now",
                "move:PermanentTransfer",
                "2026 south 20"
            ]
        );
        let clubs: Vec<(&str, u32)> = career
            .by_club
            .iter()
            .map(|club| (club.club_id.as_str(), club.appearances))
            .collect();
        assert_eq!(clubs, [("north", 4), ("south", 20)]);
    }

    /// Given a club he has joined this season and not yet played for, and a free agent with no
    ///       appearance
    /// When the season in progress is built
    /// Then the first has a zero row in progress for his club, and the second has no row
    #[test]
    fn the_season_in_progress_has_a_row_for_his_club_even_before_he_plays() {
        let stats = PlayerSeasonStats::default();

        let with_club = in_progress_rows(&SeasonStints::default(), &stats, 2026, "north", "North");
        let without = in_progress_rows(&SeasonStints::default(), &stats, 2026, "", "");

        assert_eq!(with_club.len(), 1);
        assert!(with_club[0].in_progress);
        assert_eq!(with_club[0].entry.appearances, 0);
        assert!(without.is_empty());
    }

    /// Given a season rolled over on 20 June 2026 while the calendar starts on 1 August, a free
    ///       signing on 25 June at B and a loan to C on 10 July
    /// When the career is built
    /// Then the stats season began on the day the stats reset: B's row (his stint from 25 June)
    ///      comes before the loan line, where a calendar that began the season on 1 August would put
    ///      it after
    #[test]
    fn the_season_in_progress_begins_on_the_day_the_stats_were_reset() {
        let seasons = StatisticsSeason {
            current: 2026,
            current_began_on: Some(day("2026-06-20")),
            start_month: 8,
            start_day: 1,
        };
        let rows = [stint(2026, "b", 4, 1, 0), now(2026, "c", 2, 0, 1)];
        let movements = [
            moved(
                "2026-06-25",
                PlayerMovementKind::FreeAgentSigning,
                None,
                Some("b"),
            ),
            moved(
                "2026-07-10",
                PlayerMovementKind::LoanStart,
                Some("b"),
                Some("c"),
            ),
        ];

        let career = career_of(&rows, &movements, &seasons);

        assert_eq!(
            read(&career),
            [
                "2026 c 2 now",
                "move:LoanStart",
                "2026 b 4",
                "move:FreeAgentSigning"
            ]
        );
        assert_eq!(seasons.began_on(2026), day("2026-06-20"));
        assert_eq!(seasons.began_on(2025), day("2025-08-01"));
        assert_eq!(seasons.window(2025).1, Some(day("2026-06-20")));
    }

    /// Given a renewal, a move with no date and a loan's return
    /// When the career is built
    /// Then none of them is a line: a renewal is a contract matter, a move with no date cannot be
    ///      placed, and the return is carried by the loan's line
    #[test]
    fn renewals_loan_returns_and_undated_moves_are_not_lines() {
        let rows = [stint(2025, "north", 30, 5, 2)];
        let movements = [
            moved(
                "2025-08-01",
                PlayerMovementKind::Renewal,
                None,
                Some("north"),
            ),
            moved(
                "",
                PlayerMovementKind::PermanentTransfer,
                None,
                Some("north"),
            ),
            moved(
                "2026-06-30",
                PlayerMovementKind::LoanReturn,
                None,
                Some("north"),
            ),
        ];

        let career = career_of(&rows, &movements, &calendar(8));

        assert_eq!(read(&career), ["2025 north 30"]);
    }

    /// Given a save from before the ledger existed (seasons and no moves), and no career at all
    /// When the career is built
    /// Then the seasons are listed newest first with no lines, and an empty career is empty
    #[test]
    fn a_career_with_no_ledger_lists_its_seasons_and_an_empty_one_is_empty() {
        let rows = [stint(2023, "a", 10, 1, 1), stint(2024, "b", 20, 2, 2)];

        assert_eq!(
            read(&career_of(&rows, &[], &calendar(8))),
            ["2024 b 20", "2023 a 10"]
        );
        assert_eq!(
            career_of(&[], &[], &calendar(8)),
            PlayerCareer {
                by_season: vec![],
                by_club: vec![],
                totals: CareerTotals::default()
            }
        );
    }

    /// Given a move without a stored club name, and a club the game knows by id
    /// When the career is built
    /// Then the line names the club from the game
    #[test]
    fn a_move_with_no_stored_name_is_named_from_the_game() {
        let mut entry =
            PlayerMovementEntry::new("2025-08-01", PlayerMovementKind::PermanentTransfer);
        entry.to_team_id = Some("north".to_string());

        let career = build_career(
            &[stint(2025, "north", 30, 5, 2)],
            &[entry],
            &calendar(8),
            &|id| (id == "north").then(|| "North FC".to_string()),
            None,
        );

        let line = career.by_season.iter().find_map(|line| match line {
            CareerLine::Move(line) => Some(line),
            CareerLine::Season(_) => None,
        });
        assert_eq!(
            line.and_then(|line| line.to_club_name.as_deref()),
            Some("North FC")
        );
    }

    /// Given a career line of each kind
    /// When it is serialized for the screen
    /// Then it is tagged with its kind, so the screen tells a season from a move without guessing
    #[test]
    fn a_line_is_tagged_with_its_kind_on_the_wire() {
        let career = career_of(
            &[stint(2025, "north", 30, 5, 2)],
            &[moved(
                "2025-08-01",
                PlayerMovementKind::PermanentTransfer,
                None,
                Some("north"),
            )],
            &calendar(8),
        );

        let wire = serde_json::to_value(&career).unwrap();

        assert_eq!(wire["by_season"][0]["kind"], "season");
        assert_eq!(wire["by_season"][1]["kind"], "move");
        assert_eq!(wire["by_season"][1]["movement"], "permanent_transfer");
    }

    fn world() -> Game {
        let manager = Manager::new(
            "m1".to_string(),
            "A".to_string(),
            "B".to_string(),
            "1980-01-01".to_string(),
            "GB".to_string(),
        );
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 9, 15, 12, 0, 0).unwrap());
        Game::new(clock, manager, vec![], vec![], vec![], vec![])
    }

    /// Given a world whose rollover division starts its season on 1 March, and a rollover on 20
    ///       June 2026
    /// When the statistics season is read
    /// Then it is that division's season and calendar, and began on the day the stats were reset
    #[test]
    fn the_statistics_season_is_the_rollover_divisions_and_began_when_the_stats_reset() {
        let mut game = world();
        let mut division = League::new(
            "division".to_string(),
            "Division".to_string(),
            2026,
            &["a".to_string(), "b".to_string()],
        );
        division.season_start_month = 3;
        division.season_start_day = 15;
        game.competitions = vec![division];
        game.last_season_rollover_on = Some("2026-06-20".to_string());

        let seasons = statistics_season(&game);

        assert_eq!(
            seasons,
            StatisticsSeason {
                current: 2026,
                current_began_on: Some(day("2026-06-20")),
                start_month: 3,
                start_day: 15,
            }
        );
        assert_eq!(seasons.began_on(2025), day("2025-03-15"));
    }

    /// Given a season rolled over on 20 June 2026 (the real reset day, which the game knows), and
    ///       the season before it, whose own reset day the game no longer holds (the rollover
    ///       overwrites it), with a free signing on 10 July 2025 and a loan on 20 July 2025
    /// When the career is built
    /// Then the 2025 season begins on the calendar's start (1 August 2025), not on its lost reset
    ///      day: the signing's row (on 10 July) is placed at 1 August, after the loan line. Known
    ///      limitation, pinned as it is today; recovering the day needs the reset day stored with
    ///      each career row (#1402)
    #[test]
    fn a_historical_season_begins_on_the_calendar_start_because_its_reset_day_is_not_kept() {
        let seasons = StatisticsSeason {
            current: 2026,
            current_began_on: Some(day("2026-06-20")),
            start_month: 8,
            start_day: 1,
        };
        let rows = [stint(2025, "b", 4, 1, 0), stint(2025, "c", 2, 0, 1)];
        let movements = [
            moved(
                "2025-07-10",
                PlayerMovementKind::FreeAgentSigning,
                None,
                Some("b"),
            ),
            moved(
                "2025-07-20",
                PlayerMovementKind::LoanStart,
                Some("b"),
                Some("c"),
            ),
        ];

        let career = career_of(&rows, &movements, &seasons);

        assert_eq!(seasons.began_on(2025), day("2025-08-01"));
        // Were the lost reset day (before 10 July) known, "2025 b 4" would sit below the loan line.
        assert_eq!(
            read(&career),
            [
                "2025 c 2",
                "2025 b 4",
                "move:LoanStart",
                "move:FreeAgentSigning"
            ]
        );
    }

    /// Given a club he played at for years and left, a club he was at for one season, and a
    ///       return to the first in that same last season
    /// When the career is summed by club
    /// Then the club he returned to is listed first: among clubs whose last season is the same,
    ///      the latest stint decides before the first season does
    #[test]
    fn a_club_he_returned_to_lists_ahead_of_one_he_visited_once_in_the_same_last_season() {
        let rows = [
            stint(2020, "home", 30, 5, 1),
            stint(2021, "home", 30, 6, 2),
            stint(2022, "home", 30, 7, 3),
            stint(2023, "visit", 10, 1, 0),
            stint(2023, "home", 12, 2, 1),
        ];

        let career = career_of(&rows, &[], &calendar(8));

        let clubs: Vec<&str> = career
            .by_club
            .iter()
            .map(|club| club.club_id.as_str())
            .collect();
        assert_eq!(clubs, ["home", "visit"]);
    }

    /// Given a loanee from a save older than the ledger's loan entries: `active_loan` says he is at
    ///       the borrower since 10 October 2026, but his ledger holds only his opening contract at
    ///       the parent club
    /// When the career is built
    /// Then the stint at the borrower is a loan: the authoritative `active_loan` seeds the visit
    ///      the ledger lacks, and the parent's spell ends the day the loan began
    #[test]
    fn a_loanee_the_ledger_has_no_loan_entry_for_is_seeded_from_his_active_loan() {
        let rows = [
            stint(2026, "parent", 5, 1, 0),
            now(2026, "borrower", 3, 0, 1),
        ];
        let movements = [moved(
            "2025-08-01",
            PlayerMovementKind::InitialContract,
            None,
            Some("parent"),
        )];
        let loan = ActiveLoan {
            parent_team_id: "parent".to_string(),
            loan_team_id: "borrower".to_string(),
            start_date: "2026-10-10".to_string(),
            end_date: "2027-06-30".to_string(),
            wage_contribution_pct: 50,
            buy_option_fee: None,
            loan_start_minutes: 0,
            loan_start_appearances: 0,
            development_reported_minutes: 0,
            development_reported_appearances: 0,
        };

        let career = build_career(&rows, &movements, &calendar(8), &|_| None, Some(&loan));

        assert!(season_row(&career, 2026, "borrower").on_loan);
        assert!(!season_row(&career, 2026, "parent").on_loan);
        assert_eq!(
            read(&career),
            [
                "2026 borrower 3 now",
                "2026 parent 5",
                "move:InitialContract"
            ]
        );
    }

    /// Given a league whose season starts on 31 February (a day its month lacks), in a leap and a
    ///       common year
    /// When the calendar's start of a season is read
    /// Then it is the month's last day (29 February, 28 February): the one clamping rule the
    ///      generator's season starts use. Pinned so the rule stays shared, not copied
    #[test]
    fn a_start_day_the_month_lacks_is_the_months_last() {
        let seasons = StatisticsSeason {
            current: 2030,
            current_began_on: None,
            start_month: 2,
            start_day: 31,
        };

        assert_eq!(seasons.began_on(2028), day("2028-02-29"));
        assert_eq!(seasons.began_on(2029), day("2029-02-28"));
    }

    /// Given a migrated save: the player's opening contract at club A has no date, and within one
    ///       season he played 5 matches for A, was loaned to B (3 matches) and returned to A (2)
    /// When the career is built
    /// Then the three stints keep their clubs in order: the opening contract still seeds A's first
    ///      spell, so A's first row does not take the return visit and B stays a loan
    #[test]
    fn an_undated_opening_contract_still_seeds_the_first_spell_at_its_club() {
        let rows = [
            stint(2025, "a", 5, 1, 0),
            stint(2025, "b", 3, 0, 1),
            stint(2025, "a", 2, 0, 0),
        ];
        let movements = [
            moved("", PlayerMovementKind::InitialContract, None, Some("a")),
            moved(
                "2025-10-01",
                PlayerMovementKind::LoanStart,
                Some("a"),
                Some("b"),
            ),
            moved(
                "2026-01-05",
                PlayerMovementKind::LoanReturn,
                Some("b"),
                Some("a"),
            ),
        ];

        let career = career_of(&rows, &movements, &calendar(8));

        assert!(season_row(&career, 2025, "b").on_loan);
        assert_eq!(
            read(&career),
            ["2025 a 2", "2025 b 3", "move:LoanStart", "2025 a 5"]
        );
    }

    /// Given stints at no club stored as nothing (newer saves), as the English literal "Free Agent"
    ///       and as the translation key (older saves), and one at a historical club known by name
    ///       only
    /// When the career is summed by club
    /// Then every no-club stint is one row with no name (the screen words it) and the name-only
    ///      club stays its own row
    #[test]
    fn every_way_a_save_stored_no_club_is_one_row_by_club() {
        let named = |season: u32, name: &str, apps: u32| StintRow {
            entry: CareerEntry {
                team_name: name.to_string(),
                ..stint(season, "", apps, 0, 0).entry
            },
            in_progress: false,
        };
        let rows = [
            named(2022, "Legacy FC", 8),
            named(2023, "", 10),
            named(2024, "Free Agent", 5),
            named(2025, "common.freeAgent", 4),
        ];

        let career = career_of(&rows, &[], &calendar(8));

        let clubs: Vec<(&str, u32)> = career
            .by_club
            .iter()
            .map(|club| (club.club_name.as_str(), club.appearances))
            .collect();
        assert_eq!(clubs, [("", 19), ("Legacy FC", 8)]);
        assert_eq!(season_row(&career, 2024, "").club_name, "");
    }

    /// Given a retired player whose last season's stats were never reset (12 appearances), with a
    ///       closed season of 31 and no club
    /// When his career is built
    /// Then he has no season in progress: only the closed season, and the totals are its own
    #[test]
    fn a_retired_player_has_no_season_in_progress() {
        let mut player = Player::new(
            "p".to_string(),
            "P".to_string(),
            "P".to_string(),
            "1988-01-01".to_string(),
            "GB".to_string(),
            Position::Midfielder,
            uniform_attributes(60),
        );
        player.retired = true;
        player.stats.appearances = 12;
        player.career.push(CareerEntry {
            season: 2030,
            team_id: "north".to_string(),
            team_name: "North".to_string(),
            appearances: 31,
            goals: 4,
            assists: 9,
        });

        let career = player_career(&world(), &player);

        assert_eq!(read(&career), ["2030 north 31"]);
        assert_eq!(career.totals.appearances, 31);
    }
}
