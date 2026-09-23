//! Season-length probe: what condition do clubs actually arrive at matches in?
//!
//! Run it explicitly — it advances a simulated season day by day, so it is far
//! too slow for the normal suite:
//!
//! ```text
//! cargo test -p ofm_core --test ai_readiness_probe -- --ignored --nocapture
//! ```
//!
//! It reports two numbers per club per matchday, and the gap between them is the
//! point. **Squad** is the average across the whole squad — the number
//! `ai_training`'s intensity bands are computed from. **XI** is the average
//! across the eleven players most likely to actually start. A controller reading
//! the first cannot see an exhausted first eleven propped up by a fresh bench.
//!
//! The user's club is reported separately, because changes on either side of the
//! human/AI line move the difficulty curve in opposite directions and the net
//! effect is not visible from the AI clubs alone.
//!
//! The "likely XI" here is this probe's own approximation (best by rating within
//! position quotas), not a call into the production selector — `ai_select_starting_xi`
//! is private to `turn::squad`. It is a measuring stick, not an assertion about
//! which players the game would pick.
//!
//! Since AI clubs started rotating in earnest, that distinction has teeth: the
//! side a club actually names on a congested week is deliberately *not* its best
//! eleven. So read this column as "what condition are this club's best players
//! arriving in", which is the question the readiness controller ought to be
//! asking, rather than as the average of whoever took the field. The wear behind
//! the number is real either way — every match here is simulated through
//! `turn::process_day`, so it is the production selector that decides who is
//! charged for it.
//!
//! # What this models, and what it does not
//!
//! Every day here goes through `turn::process_day`. A real career takes the same
//! route on ordinary days, and on the day the player watches or delegates its own
//! fixture it takes `turn::finish_live_match_day` instead — which also runs no
//! training, so the recovery ledger below is faithful either way.
//!
//! The **XI** columns are therefore representative. The **squad** columns are
//! pessimistic for the user's club: this probe drives every match through the
//! instant path, which charges a full match to every squad member, whereas the
//! player's own fixture is simulated live and only charges the eleven who played
//! plus the substitutes. Read the user's squad column as "what the instant path
//! does to a squad", not as a shipped number — until slice 1 lands, at which
//! point the two agree.

use chrono::{TimeZone, Utc};
use domain::league::{Fixture, FixtureCompetition, FixtureStatus, League, StandingEntry};
use domain::manager::Manager;
use domain::player::{Player, PlayerAttributes, Position};
use domain::staff::{Staff, StaffAttributes, StaffRole};
use domain::team::Team;
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::turn;

const CLUBS: usize = 8;
const SQUAD_SIZE: usize = 22;
/// What a probe season looks like. The ignored probe reads it from the
/// environment so it can be swept; the red-line tests below spell it out, so no
/// stray `OFM_PROBE_*` variable can change what they assert.
#[derive(Clone, Copy)]
struct Settings {
    /// Weeks to simulate. A full league season is 38, and the trend does not
    /// settle inside a dozen — `OFM_PROBE_WEEKS` when chasing the tail.
    weeks: u32,
    /// Fixtures per week. Two is a league plus a midweek cup — the load that
    /// separates a squad which copes from one which does not.
    /// `OFM_PROBE_MATCHES_PER_WEEK`, 1 or 2.
    per_week: u32,
    /// See [`make_staff`]. `OFM_PROBE_PHYSIO`.
    physio: u8,
}

impl Settings {
    const DEFAULT: Settings = Settings {
        weeks: 12,
        per_week: 1,
        physio: 60,
    };

    fn from_env() -> Self {
        let read = |name: &str| std::env::var(name).ok().and_then(|v| v.parse::<u32>().ok());
        Settings {
            weeks: read("OFM_PROBE_WEEKS").unwrap_or(Self::DEFAULT.weeks),
            per_week: read("OFM_PROBE_MATCHES_PER_WEEK")
                .unwrap_or(Self::DEFAULT.per_week)
                .clamp(1, 2),
            physio: read("OFM_PROBE_PHYSIO").map_or(Self::DEFAULT.physio, |v| v.min(100) as u8),
        }
    }
}
/// 2025-06-16 is a Monday, so matchdays land on the Saturday of each week.
const START: (i32, u32, u32) = (2025, 6, 16);
const FIRST_MATCHDAY_OFFSET: i64 = 5;

/// Position quota for the probe's notional first eleven: a 4-4-2.
const XI_QUOTA: [(Position, usize); 4] = [
    (Position::Goalkeeper, 1),
    (Position::Defender, 4),
    (Position::Midfielder, 4),
    (Position::Forward, 2),
];

// ---------------------------------------------------------------------------
// World construction
// ---------------------------------------------------------------------------

fn attrs(level: u8, position: &Position) -> PlayerAttributes {
    let base = level;
    let gk = *position == Position::Goalkeeper;
    PlayerAttributes {
        pace: base,
        stamina: base,
        strength: base,
        agility: base,
        passing: base,
        shooting: if gk { 20 } else { base },
        tackling: if gk { 20 } else { base },
        dribbling: base,
        defending: base,
        positioning: base,
        vision: base,
        decisions: base,
        composure: base,
        aggression: base,
        teamwork: base,
        leadership: base,
        handling: if gk { base } else { 30 },
        reflexes: if gk { base } else { 30 },
        aerial: base,
    }
}

/// A 22-man squad: a first choice and a deputy for every slot, the deputy eight
/// rating points weaker so rotation costs something.
fn make_squad(team_id: &str) -> Vec<Player> {
    let mut players = Vec::with_capacity(SQUAD_SIZE);
    for (tier, level) in [("first", 72u8), ("sub", 64u8)] {
        for (position, count) in XI_QUOTA {
            for i in 0..count {
                let id = format!("{team_id}_{tier}_{position:?}{i}");
                let mut player = Player::new(
                    id.clone(),
                    id.clone(),
                    id.clone(),
                    "1998-01-01".to_string(),
                    "England".to_string(),
                    position.clone(),
                    attrs(level, &position),
                );
                player.team_id = Some(team_id.to_string());
                player.morale = 70;
                player.condition = 100;
                player.fitness = 80;
                players.push(player);
            }
        }
    }
    players
}

/// Generated worlds give every club a full staff, so a probe without one would
/// measure the 0.8 no-coaching penalty that no real club ever pays.
///
/// `physio` multiplies **every** recovery base by `1.0 + physio/100 × 0.4`. It is
/// the single largest lever on the whole ledger and worth sweeping.
fn make_staff(team_id: &str, physio: u8) -> Vec<Staff> {
    [
        (StaffRole::Coach, 60u8, 20u8),
        (StaffRole::Physio, 20, physio),
    ]
    .into_iter()
    .map(|(role, coaching, physiotherapy)| {
        let mut staff = Staff::new(
            format!("{team_id}_{role:?}"),
            "Staff".to_string(),
            format!("{role:?}"),
            "1980-01-01".to_string(),
            role,
            StaffAttributes {
                coaching,
                judging_ability: 50,
                judging_potential: 50,
                physiotherapy,
            },
        );
        staff.nationality = "England".to_string();
        staff.team_id = Some(team_id.to_string());
        staff
    })
    .collect()
}

/// One fixture per club per week, every club playing every week — the load a
/// league season actually applies.
fn weekly_fixtures(
    team_ids: &[String],
    start: chrono::DateTime<Utc>,
    settings: &Settings,
) -> Vec<Fixture> {
    let mut fixtures = Vec::new();
    for week in 0..settings.weeks {
        // Circle method: club 0 is fixed, the rest rotate, so the pairings differ
        // every week and no club meets the same opponent twice in a row.
        let mut order: Vec<&String> = team_ids.iter().collect();
        let rotating = order.split_off(1);
        let shift = week as usize % rotating.len();
        let rotated: Vec<&String> = rotating[shift..]
            .iter()
            .chain(&rotating[..shift])
            .copied()
            .collect();
        order.extend(rotated);

        // Saturday, then the Wednesday before it when a midweek round is asked for.
        for (round, day_offset) in (0..settings.per_week).map(|r| (r, [0i64, -3][r as usize])) {
            let date = (start
                + chrono::Duration::days(FIRST_MATCHDAY_OFFSET + 7 * i64::from(week) + day_offset))
            .format("%Y-%m-%d")
            .to_string();
            for i in 0..CLUBS / 2 {
                // Flip home and away in the midweek round so a club does not play
                // the same fixture twice in one week.
                let (home, away) = if round == 0 {
                    (order[i], order[CLUBS - 1 - i])
                } else {
                    (order[CLUBS - 1 - i], order[i])
                };
                fixtures.push(Fixture {
                    id: format!("w{week}-r{round}-m{i}"),
                    matchday: week + 1,
                    date: date.clone(),
                    home_team_id: home.clone(),
                    away_team_id: away.clone(),
                    competition: FixtureCompetition::League,
                    status: FixtureStatus::Scheduled,
                    result: None,
                    ..Default::default()
                });
            }
        }
    }
    fixtures
}

fn build_world(settings: &Settings) -> Game {
    let start = Utc
        .with_ymd_and_hms(START.0, START.1, START.2, 12, 0, 0)
        .unwrap();
    let team_ids: Vec<String> = (0..CLUBS).map(|i| format!("club{i}")).collect();

    let mut teams = Vec::new();
    let mut players = Vec::new();
    let mut staff = Vec::new();
    for id in &team_ids {
        teams.push(Team::new(
            id.clone(),
            format!("{id} FC"),
            id[..3].to_string(),
            "England".to_string(),
            "London".to_string(),
            "Stadium".to_string(),
            30_000,
        ));
        players.extend(make_squad(id));
        staff.extend(make_staff(id, settings.physio));
    }

    let mut manager = Manager::new(
        "mgr1".to_string(),
        "Probe".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire(team_ids[0].clone());

    let league = League {
        id: "league1".to_string(),
        name: "Probe League".to_string(),
        season: 1,
        fixtures: weekly_fixtures(&team_ids, start, settings),
        standings: team_ids.iter().cloned().map(StandingEntry::new).collect(),
        transfer_log: vec![],
        transfer_rumours: vec![],
        ..Default::default()
    };

    let mut game = Game::new(
        GameClock::new(start),
        manager,
        teams,
        players,
        staff,
        vec![],
    );
    game.league = Some(league);
    game
}

// ---------------------------------------------------------------------------
// Measurement
// ---------------------------------------------------------------------------

fn mean(values: &[u8]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().map(|v| f64::from(*v)).sum::<f64>() / values.len() as f64
}

fn squad_condition(game: &Game, team_id: &str) -> f64 {
    let conditions: Vec<u8> = game
        .players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some(team_id))
        .map(|p| p.condition)
        .collect();
    mean(&conditions)
}

/// The best available player for each slot of a 4-4-2, by rating — the probe's
/// stand-in for whoever the game would actually start.
fn likely_xi_condition(game: &Game, team_id: &str) -> f64 {
    let mut chosen = Vec::with_capacity(11);
    for (position, count) in XI_QUOTA {
        let mut candidates: Vec<&Player> = game
            .players
            .iter()
            .filter(|p| p.team_id.as_deref() == Some(team_id))
            .filter(|p| p.injury.is_none())
            .filter(|p| p.position.to_group_position() == position.clone())
            .collect();
        candidates.sort_by_key(|player| std::cmp::Reverse(player.ovr));
        chosen.extend(candidates.into_iter().take(count).map(|p| p.condition));
    }
    mean(&chosen)
}

fn is_matchday(game: &Game) -> bool {
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    game.league.as_ref().is_some_and(|league| {
        league
            .fixtures
            .iter()
            .any(|f| f.date == today && f.status == FixtureStatus::Scheduled)
    })
}

#[test]
#[ignore = "season-length probe: run explicitly with --ignored --nocapture"]
fn report_pre_match_readiness_over_a_season() {
    let settings = Settings::from_env();
    let mut game = build_world(&settings);
    let user_team = game.manager.team_id.clone().expect("user club");
    let team_ids: Vec<String> = game.teams.iter().map(|t| t.id.clone()).collect();

    println!();
    println!(
        "Pre-match condition, {CLUBS} clubs × {SQUAD_SIZE} players, {} fixture(s) per week, {} weeks, physio {}.",
        settings.per_week, settings.weeks, settings.physio
    );
    println!("'squad' is what ai_training's intensity bands read; 'XI' is who actually plays.");
    println!();
    println!(
        "{:<6} {:>12} {:>10} {:>14} {:>12}",
        "match", "user squad", "user XI", "AI squad avg", "AI XI avg"
    );
    println!("{}", "-".repeat(58));

    let mut week = 0;
    // One extra week of days so the last matchday is reached and reported.
    for _ in 0..(settings.weeks + 1) * 7 {
        if is_matchday(&game) {
            week += 1;
            let ai_squad: Vec<f64> = team_ids
                .iter()
                .filter(|id| **id != user_team)
                .map(|id| squad_condition(&game, id))
                .collect();
            let ai_xi: Vec<f64> = team_ids
                .iter()
                .filter(|id| **id != user_team)
                .map(|id| likely_xi_condition(&game, id))
                .collect();
            let avg = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;

            println!(
                "{:<6} {:>12.1} {:>10.1} {:>14.1} {:>12.1}",
                week,
                squad_condition(&game, &user_team),
                likely_xi_condition(&game, &user_team),
                avg(&ai_squad),
                avg(&ai_xi),
            );
        }
        turn::process_day(&mut game);
    }

    println!("{}", "-".repeat(58));
    println!(
        "Read the gap between the two AI columns: that is what a squad-average \
         controller cannot see."
    );
    println!();
}

// ---------------------------------------------------------------------------
// The red line
// ---------------------------------------------------------------------------

/// Pre-match condition an AI club must reach, its likely eleven and its squad
/// both, on every settled week.
const FRESH_ENOUGH: f64 = 80.0;

/// Weeks allowed to settle before the line applies. Every club starts at 100,
/// and the first two weeks are the fall to wherever its ledger holds it.
const SETTLING_WEEKS: usize = 2;

/// Pre-match (AI squad, AI likely eleven) for every match of a probe season.
fn season_readings(settings: &Settings) -> Vec<(f64, f64)> {
    let mut game = build_world(settings);
    let user_team = game.manager.team_id.clone().expect("user club");
    let ai_teams: Vec<String> = game
        .teams
        .iter()
        .map(|t| t.id.clone())
        .filter(|id| *id != user_team)
        .collect();
    let average = |read: &dyn Fn(&Game, &str) -> f64, game: &Game| {
        ai_teams.iter().map(|id| read(game, id)).sum::<f64>() / ai_teams.len() as f64
    };

    let mut readings = Vec::new();
    for _ in 0..(settings.weeks + 1) * 7 {
        if is_matchday(&game) {
            readings.push((
                average(&squad_condition, &game),
                average(&likely_xi_condition, &game),
            ));
        }
        turn::process_day(&mut game);
    }
    readings
}

/// Checked match by match rather than on the season's average: a side that
/// arrives at 90 one week and 70 the next has had one bad week, and the average
/// would hide it.
fn assert_every_settled_match_fresh(settings: &Settings) {
    let readings = season_readings(settings);
    let settled = &readings[SETTLING_WEEKS * settings.per_week as usize..];
    assert!(
        !settled.is_empty(),
        "the probe world played no settled matches"
    );
    for (n, (squad, xi)) in settled.iter().enumerate() {
        let n = n + SETTLING_WEEKS * settings.per_week as usize + 1;
        assert!(
            *xi >= FRESH_ENOUGH,
            "{} fixture(s) a week, match {n}: AI clubs sent their likely eleven out \
             at {xi:.1}, below {FRESH_ENOUGH} (squad {squad:.1})",
            settings.per_week
        );
        assert!(
            *squad >= FRESH_ENOUGH,
            "{} fixture(s) a week, match {n}: AI squads reached the match at \
             {squad:.1}, below {FRESH_ENOUGH}",
            settings.per_week
        );
    }
}

/// A normal week: one fixture, the rest of the week to prepare for it.
#[test]
fn an_ai_club_on_one_fixture_a_week_reaches_every_match_fresh() {
    assert_every_settled_match_fresh(&Settings::DEFAULT);
}

/// A congested week: a league match on Saturday and a cup tie the Wednesday
/// before it. Recovery alone cannot carry a first eleven through two matches in
/// four days; the manager has to share the load.
#[test]
fn an_ai_club_on_two_fixtures_a_week_reaches_every_match_fresh() {
    assert_every_settled_match_fresh(&Settings {
        per_week: 2,
        ..Settings::DEFAULT
    });
}
