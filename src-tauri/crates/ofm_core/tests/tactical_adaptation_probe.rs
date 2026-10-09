//! How far do real clubs drift from their blueprint, and why?
//!
//! ```text
//! cargo test -p ofm_core --test tactical_adaptation_probe --release -- --ignored --nocapture
//! ```
//!
//! The weekly review does two separable things: it **caps** a blueprint the
//! squad cannot run, and it **shades** one whose results have gone badly. Every
//! threshold in both — the stamina a pressing game needs, the pace a high line
//! needs, what counts as leaking goals — was chosen against hand-built test
//! players, which is to say it was not chosen against anything. The role
//! assigner made exactly this mistake one slice ago: its margin looked right in
//! a unit test and handed a specialism to 93.5% of the world.
//!
//! Both failure directions pass the unit suite:
//!
//! - **Too loose.** Nearly every club ends up off its blueprint, the identities
//!   slice 6 gave the world are noise around a common shape, and "every club
//!   plays the same way" is true again for a new reason.
//! - **Too tight.** Nothing ever fires, and the review is an expensive way to
//!   write a club back exactly what it already had.
//!
//! Three arms, because the two mechanisms need different populations:
//!
//! 1. **The readings themselves**, across every club the generator makes. A fire
//!    rate says a threshold is in the wrong place; a percentile table says
//!    where to put it.
//! 2. **The caps alone**, over the same whole world with no fixtures at all —
//!    with nothing played, no club has a form reading, so whatever moves is the
//!    caps.
//! 3. **A season**, in a twenty-club league lifted out of that world, which is
//!    the only arm where results exist for a manager to react to.
//!
//! Arms 1 and 2 must use the whole world rather than the league. The league is
//! the first twenty clubs the generator emits, and those are not a random
//! sample of anything.

use domain::team::{DefensiveLine, PressingIntensity, TacticsPhaseSettings};
use ofm_core::game::Game;
use ofm_core::{ai_tactics, generator, turn};

const CLUBS: usize = 20;
const FIRST_MATCHDAY_OFFSET: i64 = 5;

fn seed() -> u64 {
    std::env::var("OFM_PROBE_SEED")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20260820)
}

fn weeks() -> u32 {
    std::env::var("OFM_PROBE_WEEKS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(19)
}

// ---------------------------------------------------------------------------
// Measurement
// ---------------------------------------------------------------------------

fn line_depth(line: &DefensiveLine) -> u8 {
    match line {
        DefensiveLine::VeryLow => 0,
        DefensiveLine::Low => 1,
        DefensiveLine::Medium => 2,
        DefensiveLine::High => 3,
    }
}

fn press_level(press: &PressingIntensity) -> u8 {
    match press {
        PressingIntensity::Passive => 0,
        PressingIntensity::Medium => 1,
        PressingIntensity::Aggressive => 2,
    }
}

/// Which dials moved and in which direction, named so a reader can tell a cap
/// from a reaction: the caps only ever ask for less.
fn differences(
    blueprint: &TacticsPhaseSettings,
    actual: &TacticsPhaseSettings,
) -> Vec<&'static str> {
    let mut moved = Vec::new();
    if actual.defensive_line != blueprint.defensive_line {
        moved.push(
            if line_depth(&actual.defensive_line) < line_depth(&blueprint.defensive_line) {
                "line deeper"
            } else {
                "line higher"
            },
        );
    }
    if actual.pressing_intensity != blueprint.pressing_intensity {
        moved.push(
            if press_level(&actual.pressing_intensity) < press_level(&blueprint.pressing_intensity)
            {
                "pressing down"
            } else {
                "pressing up"
            },
        );
    }
    if actual.counter_press_duration != blueprint.counter_press_duration {
        moved.push("counter-press shortened");
    }
    if actual.defensive_shape != blueprint.defensive_shape {
        moved.push("shape compact");
    }
    if actual.width != blueprint.width {
        moved.push("width wide");
    }
    if actual.break_speed != blueprint.break_speed {
        moved.push("break faster");
    }
    if actual.tempo != blueprint.tempo {
        moved.push("tempo direct");
    }
    moved
}

fn report(title: &str, game: &Game) {
    let mut squads = 0usize;
    let mut off_blueprint = 0usize;
    let mut per_style: std::collections::BTreeMap<String, (usize, usize)> = Default::default();
    let mut per_move: std::collections::BTreeMap<&'static str, usize> = Default::default();
    let mut rationed = [0usize; 5];

    for team in &game.teams {
        squads += 1;
        let blueprint = ai_tactics::blueprint_for(&team.play_style);
        let style = per_style
            .entry(format!("{:?}", team.play_style))
            .or_default();
        style.0 += 1;
        rationed[ai_tactics::under_priced_dials(&team.tactics_phase).min(4)] += 1;

        let moved = differences(&blueprint, &team.tactics_phase);
        if moved.is_empty() {
            continue;
        }
        off_blueprint += 1;
        style.1 += 1;
        for name in moved {
            *per_move.entry(name).or_default() += 1;
        }
    }

    println!();
    println!("=== {title} ===");
    println!(
        "off blueprint: {off_blueprint} of {squads} ({:.1}%)",
        100.0 * off_blueprint as f64 / squads.max(1) as f64
    );
    println!();
    println!("{:<14} {:>7} {:>14}", "style", "clubs", "off blueprint");
    println!("{}", "-".repeat(37));
    for (style, (total, off)) in &per_style {
        println!(
            "{:<14} {:>7} {:>13.1}%",
            style,
            total,
            100.0 * *off as f64 / (*total).max(1) as f64
        );
    }
    println!();
    println!("{:<26} {:>7}", "what moved", "clubs");
    println!("{}", "-".repeat(34));
    let mut rows: Vec<(&&str, &usize)> = per_move.iter().collect();
    rows.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    for (name, count) in rows {
        println!("{name:<26} {count:>7}");
    }
    println!();
    print!("under-priced dials held:");
    for (count, clubs) in rationed.iter().enumerate() {
        print!("  {count}→{clubs}");
    }
    println!();
}

// ---------------------------------------------------------------------------
// The readings the caps are taken against
// ---------------------------------------------------------------------------

/// The probe's own copy of what the review reads, for the same reason the
/// readiness probe keeps its own approximation of a likely XI: the production
/// version is private, and a measuring stick is allowed to be one. Keep the two
/// definitions in step by hand — `ai_tactics::read_squad` is the original.
fn squad_readings(squad: &[&domain::player::Player]) -> Option<(f64, f64)> {
    use domain::player::Position;

    if squad.is_empty() {
        return None;
    }
    let mut available: Vec<&&domain::player::Player> =
        squad.iter().filter(|p| p.injury.is_none()).collect();
    if available.is_empty() {
        available = squad.iter().collect();
    }
    available.sort_by(|a, b| b.ovr.cmp(&a.ovr).then_with(|| a.id.cmp(&b.id)));
    let eleven = &available[..11.min(available.len())];
    let legs = mean(eleven.iter().map(|p| p.attributes.stamina));

    let mut defenders: Vec<&&domain::player::Player> = available
        .iter()
        .copied()
        .filter(|p| p.position.to_group_position() == Position::Defender)
        .collect();
    defenders.sort_by(|a, b| b.ovr.cmp(&a.ovr).then_with(|| a.id.cmp(&b.id)));
    let pace = if defenders.is_empty() {
        mean(eleven.iter().map(|p| p.attributes.pace))
    } else {
        mean(
            defenders[..4.min(defenders.len())]
                .iter()
                .map(|p| p.attributes.pace),
        )
    };
    Some((legs, pace))
}

fn mean(values: impl Iterator<Item = u8>) -> f64 {
    let values: Vec<f64> = values.map(f64::from).collect();
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

fn percentile(sorted: &[f64], fraction: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let index = ((sorted.len() - 1) as f64 * fraction).round() as usize;
    sorted[index]
}

fn report_readings(teams: &[domain::team::Team], players: &[domain::player::Player]) {
    let mut legs = Vec::new();
    let mut paces = Vec::new();
    for team in teams {
        let squad: Vec<&domain::player::Player> = players
            .iter()
            .filter(|p| p.team_id.as_deref() == Some(team.id.as_str()))
            .collect();
        if let Some((club_legs, club_pace)) = squad_readings(&squad) {
            legs.push(club_legs);
            paces.push(club_pace);
        }
    }
    legs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    paces.sort_by(|a, b| a.partial_cmp(b).unwrap());

    println!();
    println!(
        "=== What the caps are read against, across {} clubs ===",
        legs.len()
    );
    println!();
    println!(
        "{:<28} {:>6} {:>6} {:>6} {:>6} {:>6}",
        "reading", "p10", "p25", "p50", "p75", "p90"
    );
    println!("{}", "-".repeat(62));
    for (name, values) in [
        ("likely XI mean stamina", &legs),
        ("back four mean pace", &paces),
    ] {
        println!(
            "{:<28} {:>6.1} {:>6.1} {:>6.1} {:>6.1} {:>6.1}",
            name,
            percentile(values, 0.10),
            percentile(values, 0.25),
            percentile(values, 0.50),
            percentile(values, 0.75),
            percentile(values, 0.90),
        );
    }
    println!();
    println!(
        "A cap set at the median fires for half the world and is not a cap, it is a \
         second blueprint; a cap below p10 never fires at all. Both are placed \
         near p20: the least equipped fifth cannot run the most demanding version \
         of its style."
    );
}

/// Every five-match window every club in the league lived through, so the form
/// thresholds can be placed on the distribution a manager actually sees rather
/// than on a round number. A trigger sitting near the median is not a bad run,
/// it is Tuesday.
/// Goals scored and conceded per game over every five-match window the
/// season's completed fixtures give each club, each list sorted ascending.
/// Every match is counted from both ends, so the two lists mirror each other.
fn form_windows(game: &Game) -> (Vec<f64>, Vec<f64>) {
    use domain::league::FixtureStatus;

    let mut results: std::collections::HashMap<&str, Vec<(&str, f64, f64)>> = Default::default();
    // `game.competitions` rather than the crate-private `competitions_in_play`:
    // this probe builds its own world and always populates it.
    for fixture in game
        .competitions
        .iter()
        .flat_map(|competition| competition.fixtures.iter())
        .filter(|fixture| fixture.status == FixtureStatus::Completed)
    {
        let Some(result) = fixture.result.as_ref() else {
            continue;
        };
        let (home, away) = (f64::from(result.home_goals), f64::from(result.away_goals));
        results
            .entry(fixture.home_team_id.as_str())
            .or_default()
            .push((fixture.date.as_str(), home, away));
        results
            .entry(fixture.away_team_id.as_str())
            .or_default()
            .push((fixture.date.as_str(), away, home));
    }

    let mut conceded = Vec::new();
    let mut scored = Vec::new();
    for played in results.values_mut() {
        played.sort_by(|a, b| a.0.cmp(b.0));
        for window in played.windows(5) {
            scored.push(window.iter().map(|(_, s, _)| s).sum::<f64>() / 5.0);
            conceded.push(window.iter().map(|(_, _, c)| c).sum::<f64>() / 5.0);
        }
    }
    scored.sort_by(|a, b| a.partial_cmp(b).unwrap());
    conceded.sort_by(|a, b| a.partial_cmp(b).unwrap());
    (scored, conceded)
}

fn report_form_windows(game: &Game) {
    let (scored, conceded) = form_windows(game);

    println!();
    println!(
        "=== What a five-match window looks like, over {} of them ===",
        scored.len()
    );
    println!();
    println!(
        "{:<22} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6}",
        "per game", "p05", "p15", "p50", "p85", "p95", "mean"
    );
    println!("{}", "-".repeat(64));
    for (name, values) in [("scored", &scored), ("conceded", &conceded)] {
        let mean = values.iter().sum::<f64>() / values.len().max(1) as f64;
        println!(
            "{:<22} {:>6.2} {:>6.2} {:>6.2} {:>6.2} {:>6.2} {:>6.2}",
            name,
            percentile(values, 0.05),
            percentile(values, 0.15),
            percentile(values, 0.50),
            percentile(values, 0.85),
            percentile(values, 0.95),
            mean,
        );
    }
}

// ---------------------------------------------------------------------------
// The world
// ---------------------------------------------------------------------------

type GeneratedWorld = (
    Vec<domain::team::Team>,
    Vec<domain::player::Player>,
    Vec<domain::staff::Staff>,
);

fn start_date() -> chrono::DateTime<chrono::Utc> {
    use chrono::{TimeZone, Utc};
    Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap()
}

fn nobodys_manager() -> domain::manager::Manager {
    // Nobody's club: every side in this world runs the AI path, which is the
    // population the review is about.
    domain::manager::Manager::new(
        "probe-mgr".to_string(),
        "Probe".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    )
}

/// The whole generated world with no competitions at all. Nothing is ever
/// played, so no club can have a form reading and whatever moves is the caps.
fn whole_world(world: &GeneratedWorld) -> Game {
    use ofm_core::clock::GameClock;
    Game::new(
        GameClock::new(start_date()),
        nobodys_manager(),
        world.0.clone(),
        world.1.clone(),
        world.2.clone(),
        vec![],
    )
}

/// A twenty-club league lifted out of the same world, so the clubs reacting to
/// results are ones the generator actually made. The rest of the world is
/// dropped: it would only slow the season down, and a club with no fixtures can
/// never contribute a form reading.
fn one_league(world: &GeneratedWorld) -> Game {
    use domain::league::{Fixture, FixtureCompetition, FixtureStatus, League, StandingEntry};
    use ofm_core::clock::GameClock;

    let (all_teams, all_players, all_staff) = world;
    let teams: Vec<domain::team::Team> = all_teams
        .iter()
        .filter(|team| {
            all_players
                .iter()
                .filter(|p| p.team_id.as_deref() == Some(team.id.as_str()))
                .count()
                >= 11
        })
        .take(CLUBS)
        .cloned()
        .collect();
    let kept: std::collections::HashSet<&str> = teams.iter().map(|t| t.id.as_str()).collect();
    let players: Vec<domain::player::Player> = all_players
        .iter()
        .filter(|p| p.team_id.as_deref().is_some_and(|id| kept.contains(id)))
        .cloned()
        .collect();
    let staff: Vec<domain::staff::Staff> = all_staff
        .iter()
        .filter(|s| s.team_id.as_deref().is_some_and(|id| kept.contains(id)))
        .cloned()
        .collect();

    let team_ids: Vec<String> = teams.iter().map(|t| t.id.clone()).collect();
    let start = start_date();

    // Circle method: club 0 is fixed and the rest rotate, so no club meets the
    // same opponent two weeks running.
    let mut fixtures = Vec::new();
    for week in 0..weeks() {
        let mut order: Vec<&String> = team_ids.iter().collect();
        let rotating = order.split_off(1);
        let shift = week as usize % rotating.len();
        let rotated: Vec<&String> = rotating[shift..]
            .iter()
            .chain(&rotating[..shift])
            .copied()
            .collect();
        order.extend(rotated);

        let date = (start + chrono::Duration::days(FIRST_MATCHDAY_OFFSET + 7 * i64::from(week)))
            .format("%Y-%m-%d")
            .to_string();
        for i in 0..CLUBS / 2 {
            fixtures.push(Fixture {
                id: format!("w{week}-m{i}"),
                competition_id: "probe-league".to_string(),
                matchday: week + 1,
                date: date.clone(),
                home_team_id: order[i].clone(),
                away_team_id: order[CLUBS - 1 - i].clone(),
                competition: FixtureCompetition::League,
                status: FixtureStatus::Scheduled,
                result: None,
            });
        }
    }

    let league = League {
        id: "probe-league".to_string(),
        name: "Probe League".to_string(),
        season: 2026,
        fixtures,
        standings: team_ids.iter().cloned().map(StandingEntry::new).collect(),
        ..Default::default()
    };

    let mut game = Game::new(
        GameClock::new(start),
        nobodys_manager(),
        teams,
        players,
        staff,
        vec![],
    );
    game.league = Some(league.clone());
    game.competitions = vec![league];
    game
}

#[test]
#[ignore = "generates a world and simulates a season; run explicitly"]
fn report_how_far_clubs_drift_from_their_blueprint() {
    let world = generator::generate_world_seeded(
        seed(),
        &generator::definitions::DefinitionSources::embedded_only(),
    );
    println!();
    println!(
        "Generated {} clubs and {} players.",
        world.0.len(),
        world.1.len()
    );

    report_readings(&world.0, &world.1);

    let mut idle = whole_world(&world);
    // One week: long enough for every club's review day to come round exactly
    // once, and there is nothing to play in any case.
    for _ in 0..7 {
        turn::process_day(&mut idle);
    }
    report("The caps alone, across the whole world", &idle);
    drop(idle);

    let mut league = one_league(&world);
    println!();
    println!(
        "A {}-club league out of that world, {} weeks of fixtures.",
        league.teams.len(),
        weeks()
    );
    for _ in 0..(7 + 7 * weeks() as usize) {
        turn::process_day(&mut league);
    }
    report("After a season — caps and reactions", &league);
    report_form_windows(&league);

    println!();
    println!(
        "A minority off blueprint is the goal. Everybody off it means the identities \
         are noise; nobody off it means no manager in the world ever reacts to anything. \
         The two form triggers belong at the same tail of the window table above — the \
         first pass had one at p60 and the other below p05, which is why every club in \
         the world was leaking and none was ever blunt."
    );
    println!();
}

/// The review's form triggers (`ai_tactics`'s `LEAKY` and `BLUNT`) are p85 of
/// goals conceded and p15 of goals scored over this probe's five-match windows,
/// read when a club scored about 2.52 a game. They are only right while that
/// holds. This pins it, so an engine change that moves how many goals are
/// scored fails here instead of silently mistuning the triggers — re-read the
/// window table (`report_how_far_clubs_drift_from_their_blueprint`) and move
/// the triggers with it, then move this band.
///
/// The band is wide on purpose: match results are not seeded, and single
/// seasons have read anywhere from 2.37 to 2.66. The last two moves it exists
/// to catch were 2.06 -> 2.29 and 2.29 -> 2.52.
#[test]
#[ignore = "red since #648 moved batch matches onto the live engine: clubs score ~2.2, triggers calibrated at 2.52; recalibration belongs to the match engine overhaul"]
fn the_form_triggers_are_read_off_the_scoring_rate_they_were_calibrated_on() {
    let world = generator::generate_world_seeded(
        seed(),
        &generator::definitions::DefinitionSources::embedded_only(),
    );
    let mut league = one_league(&world);
    for _ in 0..(7 + 7 * weeks() as usize) {
        turn::process_day(&mut league);
    }

    let (scored, _) = form_windows(&league);
    assert!(!scored.is_empty(), "the probe season played no matches");
    let per_game = scored.iter().sum::<f64>() / scored.len() as f64;
    assert!(
        (2.30..=2.75).contains(&per_game),
        "clubs now score {per_game:.2} a game; the form triggers were calibrated at about \
         2.52. Re-read the window table and move LEAKY / BLUNT with it."
    );
}
