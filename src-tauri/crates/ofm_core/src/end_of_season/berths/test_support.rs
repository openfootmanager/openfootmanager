//! Fixtures shared by the pyramid and domestic test modules.

use crate::clock::GameClock;
use crate::game::Game;
use chrono::{TimeZone, Utc};
use domain::league::{Berth, BerthRule, League, StandingEntry};
use domain::manager::Manager;

pub(super) fn empty_game() -> Game {
    let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 5, 20, 12, 0, 0).unwrap());
    let manager = Manager::new(
        "mgr".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    Game::new(clock, manager, vec![], vec![], vec![], vec![])
}

/// Domestic league table with finished standings (`played > 0` so the
/// season-ended guard treats it as complete). Clubs are listed best-first.
pub(super) fn division(
    id: &str,
    priority: u32,
    country: &str,
    standings: &[(&str, u32)],
) -> League {
    let team_ids: Vec<String> = standings.iter().map(|(id, _)| id.to_string()).collect();
    let mut league = League::new(id.to_string(), id.to_string(), 2026, &team_ids);
    league.priority = priority;
    league.country_id = Some(country.to_string());
    league.standings = standings
        .iter()
        .map(|(team, points)| {
            let mut entry = StandingEntry::new(team.to_string());
            entry.points = *points;
            entry.played = 1;
            entry
        })
        .collect();
    league
}

pub(super) fn position_berth(target: &str, from: u32, to: u32) -> Berth {
    Berth {
        target: target.to_string(),
        rule: BerthRule::PositionRange { from, to },
        fallback_to: None,
    }
}

pub(super) fn two_region_central_pyramid() -> Game {
    let mut north = division(
        "north",
        1,
        "BR",
        &[("n1", 40), ("n2", 30), ("n3", 20), ("n4", 10)],
    );
    north.berths = vec![position_berth("central", 1, 2)];
    let mut south = division(
        "south",
        1,
        "BR",
        &[("s1", 40), ("s2", 30), ("s3", 20), ("s4", 10)],
    );
    south.berths = vec![position_berth("central", 1, 2)];
    let central = division(
        "central",
        0,
        "BR",
        &[
            ("c1", 80),
            ("c2", 70),
            ("c3", 60),
            ("c4", 50),
            ("c5", 40),
            ("c6", 30),
            ("c7", 20),
            ("c8", 10),
        ],
    );
    let mut game = empty_game();
    game.competitions = vec![central, north, south];
    game
}

/// 4-club Central, four 2-club feeders each sending two — 8 winners, 4 places.
pub(super) fn four_region_oversubscribed_pyramid() -> Game {
    let central = division(
        "central",
        0,
        "BR",
        &[("c1", 40), ("c2", 30), ("c3", 20), ("c4", 10)],
    );
    let regions = [
        ("north", "n1", "n2"),
        ("south", "s1", "s2"),
        ("east", "e1", "e2"),
        ("west", "w1", "w2"),
    ];
    let mut game = empty_game();
    game.competitions = std::iter::once(central)
        .chain(regions.iter().map(|(id, a, b)| {
            let mut league = division(id, 1, "BR", &[(a, 20), (b, 10)]);
            league.berths = vec![position_berth("central", 1, 2)];
            league
        }))
        .collect();
    game
}

pub(super) fn filler_club(id: &str, nation: &str, reputation: u32) -> domain::team::Team {
    let mut team = domain::team::Team::new(
        id.to_string(),
        id.to_string(),
        id.to_string(),
        nation.to_string(),
        "City".to_string(),
        "Stadium".to_string(),
        10_000,
    );
    team.football_nation = nation.to_string();
    team.reputation = reputation;
    team
}
