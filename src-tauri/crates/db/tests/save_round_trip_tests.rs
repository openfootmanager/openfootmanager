//! A save loses nothing (#396, "the world survives").
//!
//! The game is saved to a real `.db` through [`SaveManager`] and read back by a fresh one, then the
//! game before and the game after are compared field by field. The repositories write positional
//! SQL (`crates/db/src/repositories/`), so a field a column list or a `row.get(N)` forgets is
//! dropped on every save and load without any error. A JSON round trip of the game, which is what
//! the season harness's `reloaded` does, cannot see that.
//!
//! The career is the one the app builds: a generated world, a manager at its strongest club,
//! competitions, calendar and berths from `begin_career`. It is read back at three moments: after
//! the career starts, after the manager signs a free agent, and after a season has rolled over.

use chrono::{TimeZone, Utc};
use db::save_manager::SaveManager;
use domain::manager::Manager;
use domain::stats::StatsState;
use ofm_core::career::{CareerScope, begin_career};
use ofm_core::clock::GameClock;
use ofm_core::contracts::{RenewalDecision, free_agent_opening_offer, offer_free_agent_contract};
use ofm_core::end_of_season::{advance_to_next_season, is_season_complete};
use ofm_core::game::Game;
use ofm_core::generator::{
    DefinitionSources, WorldGenConfig, generate_world_data_seeded_with,
    repair_opening_youth_academies,
};
use ofm_core::turn;
use serde_json::Value;

const SEED: u64 = 11;

/// A career on a compact generated world, at the start of its first season.
fn career() -> (Game, StatsState) {
    let world = generate_world_data_seeded_with(
        SEED,
        &WorldGenConfig::compact(),
        &DefinitionSources::embedded_only(),
    );
    let start = Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap();
    let club = ofm_core::world::strongest_club(&world.teams, "ENG")
        .or_else(|| world.teams.first().map(|team| team.id.as_str()))
        .expect("the world has a club")
        .to_string();
    let mut manager = Manager::new(
        "save-round-trip-manager".to_string(),
        "Save".to_string(),
        "Tester".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire(club.clone());
    let mut game = Game::new(
        GameClock::new(start),
        manager,
        world.teams,
        world.players,
        world.staff,
        vec![],
    );
    game.seed = SEED;
    game.available_staff_market_last_activity_date = Some(start.format("%Y-%m-%d").to_string());
    repair_opening_youth_academies(&mut game);
    let stats = begin_career(
        &mut game,
        &club,
        CareerScope::default(),
        StatsState::default(),
    )
    .expect("the career begins");
    (game, stats)
}

/// `game` written to a new save and read back by a fresh `SaveManager`, as quitting and loading do.
fn saved_and_loaded(game: &Game, stats: &StatsState) -> Game {
    let saves = tempfile::tempdir().unwrap();
    let mut writer = SaveManager::init(saves.path()).unwrap();
    let save_id = writer
        .create_save_with_stats(game, stats, "Round trip")
        .unwrap();
    let mut reader = SaveManager::init(saves.path()).unwrap();
    reader
        .load_game(&save_id)
        .unwrap_or_else(|error| panic!("the save did not load back: {error}"))
}

fn by_id(items: &[Value]) -> bool {
    !items.is_empty() && items.iter().all(|item| item["id"].is_string())
}

/// The paths at which `before` and `after` differ, at most `limit` of them, each with both values.
fn differences(before: &Value, after: &Value, limit: usize) -> Vec<String> {
    fn walk(path: &str, before: &Value, after: &Value, found: &mut Vec<String>, limit: usize) {
        if found.len() >= limit || before == after {
            return;
        }
        match (before, after) {
            (Value::Object(left), Value::Object(right)) => {
                for key in left
                    .keys()
                    .chain(right.keys().filter(|key| !left.contains_key(*key)))
                {
                    let (l, r) = (
                        left.get(key).unwrap_or(&Value::Null),
                        right.get(key).unwrap_or(&Value::Null),
                    );
                    walk(&format!("{path}.{key}"), l, r, found, limit);
                }
            }
            (Value::Array(left), Value::Array(right)) if by_id(left) && by_id(right) => {
                // Lists of records with an id are compared record by record, not position by
                // position: a load may order a list differently (the inbox is sorted), which loses
                // nothing. A record on one side only is a difference.
                let id_of = |item: &Value| item["id"].as_str().unwrap_or_default().to_string();
                let loaded: std::collections::HashMap<String, &Value> =
                    right.iter().map(|item| (id_of(item), item)).collect();
                let saved: std::collections::HashSet<String> = left.iter().map(id_of).collect();
                for item in left {
                    let id = id_of(item);
                    match loaded.get(&id) {
                        Some(other) => walk(&format!("{path}[{id}]"), item, other, found, limit),
                        None if found.len() < limit => {
                            found.push(format!("{path}[{id}]: saved, missing after the load"));
                        }
                        None => {}
                    }
                }
                for id in loaded.keys().filter(|id| !saved.contains(*id)) {
                    if found.len() < limit {
                        found.push(format!("{path}[{id}]: not saved, present after the load"));
                    }
                }
            }
            (Value::Array(left), Value::Array(right)) if left.len() == right.len() => {
                for (index, (l, r)) in left.iter().zip(right).enumerate() {
                    walk(&format!("{path}[{index}]"), l, r, found, limit);
                }
            }
            (Value::Number(left), Value::Number(right))
                if left.as_f64().zip(right.as_f64()).is_some_and(|(l, r)| {
                    // A float written as JSON text and read back can land one ulp away.
                    (l - r).abs() <= 1e-9 * l.abs().max(r.abs()).max(1.0)
                }) => {}
            _ => {
                let clip = |value: &Value| {
                    let text = value.to_string();
                    if text.len() > 120 {
                        format!("{}...", &text[..120])
                    } else {
                        text
                    }
                };
                found.push(format!(
                    "{path}: saved {} loaded {}",
                    clip(before),
                    clip(after)
                ));
            }
        }
    }
    let mut found = Vec::new();
    walk("game", before, after, &mut found, limit);
    found
}

/// Every path at which the game read back differs from the game saved. A load settles the
/// calendar (`repair_loaded_calendar`: clashing unplayed matches move to the earliest legal day and
/// are recorded as postponed). That is a repair the load is meant to make, not a loss, so the game
/// saved is the game with its calendar already settled; what is compared is everything else the
/// save has to keep.
fn every_difference(game: &Game, stats: &StatsState) -> Vec<String> {
    let mut game = game.clone();
    ofm_core::calendar_repair::repair_loaded_calendar(&mut game);
    let loaded = saved_and_loaded(&game, stats);
    differences(
        &serde_json::to_value(&game).unwrap(),
        &serde_json::to_value(&loaded).unwrap(),
        10_000,
    )
}

/// The differences a load still makes that are known bugs, by what they are. Each is pinned by a
/// test that says it STILL happens (so fixing it makes that test fail, to be deleted with the fix),
/// and left out of the comparison that says nothing else is lost.
fn known_bug(difference: &str) -> Option<&'static str> {
    if difference.starts_with("game.days_since_last_job_offer") {
        Some("the job-offer cooldown is not saved (#1260)")
    } else if difference.starts_with("game.staff[")
        && (difference.contains("].birth_country") || difference.contains("].football_nation"))
    {
        Some("staff made during a season have no nationality until a reload (#1261)")
    } else if difference.starts_with("game.players[") && difference.contains("].weak_foot") {
        Some("players made during a season change weak foot on a reload (#1262)")
    } else {
        None
    }
}

fn assert_nothing_lost(game: &Game, stats: &StatsState, moment: &str) {
    let lost: Vec<String> = every_difference(game, stats)
        .into_iter()
        .filter(|difference| known_bug(difference).is_none())
        .collect();
    assert!(
        lost.is_empty(),
        "{moment}: the game read back differs from the game saved ({} paths, first 40):\n{}",
        lost.len(),
        lost.iter().take(40).cloned().collect::<Vec<_>>().join("\n")
    );
}

/// A career played through one whole season and its rollover.
fn career_after_a_season() -> (Game, StatsState) {
    let (mut game, stats) = career();
    assert!(
        game.last_season_rollover_on.is_none(),
        "a career that has just begun has not rolled a season over"
    );

    // The harness's loop: a day at a time until the season is complete, then the rollover.
    for _ in 0..600 {
        if is_season_complete(&game) {
            break;
        }
        turn::process_day(&mut game);
    }
    assert!(is_season_complete(&game), "the season finished in 600 days");
    advance_to_next_season(&mut game).expect("the season rolls over");
    assert!(
        game.last_season_rollover_on.is_some(),
        "the game records the rollover that is saved"
    );
    (game, stats)
}

/// Given a career that has just begun on a generated world,
/// When it is saved and loaded back,
/// Then the game is the one that was saved, field for field.
#[test]
fn a_career_at_its_start_loads_back_unchanged() {
    let (game, stats) = career();

    assert_nothing_lost(&game, &stats, "after career start");
}

/// Given a career in which the manager has signed a free agent,
/// When it is saved and loaded back,
/// Then the game is the one that was saved, with the signing in it.
#[test]
fn a_career_after_a_transfer_loads_back_unchanged() {
    let (mut game, stats) = career();
    let user_club = game
        .manager
        .team_id
        .clone()
        .expect("the manager has a club");
    let free_agents: Vec<String> = game
        .players
        .iter()
        .filter(|player| player.team_id.is_none() && !player.retired)
        .map(|player| player.id.clone())
        .take(40)
        .collect();
    let signed = free_agents.iter().find(|id| {
        free_agent_opening_offer(&game, id)
            .and_then(|offer| offer_free_agent_contract(&mut game, id, offer))
            .is_ok_and(|outcome| matches!(outcome.decision, RenewalDecision::Accepted))
    });
    let signed = signed
        .expect("one of the free agents accepts the opening offer")
        .clone();
    assert_eq!(
        game.players
            .iter()
            .find(|player| player.id == signed)
            .and_then(|player| player.team_id.as_deref()),
        Some(user_club.as_str()),
        "the signing is in the game that is saved"
    );

    assert_nothing_lost(&game, &stats, "after a transfer");
}

/// Given a career played through a whole season and its rollover,
/// When it is saved and loaded back,
/// Then nothing but the known bugs below differs: the new season's calendar, tables, ladders,
///      players and staff included.
#[test]
fn a_career_after_the_rollover_loads_back_unchanged() {
    let (game, stats) = career_after_a_season();

    assert_nothing_lost(&game, &stats, "after the rollover");
}

fn known_bug_still_happens(bug: &str) -> bool {
    let (game, stats) = career_after_a_season();
    every_difference(&game, &stats)
        .iter()
        .any(|difference| known_bug(difference) == Some(bug))
}

/// KNOWN BUG #1260 (job-offer cooldown): `days_since_last_job_offer` has no column; the load sets it to
/// `None` (`game_persistence.rs`), so a save and load restarts the cooldown. Delete this test with
/// the fix.
#[test]
fn known_bug_the_job_offer_cooldown_is_not_saved() {
    assert!(known_bug_still_happens(
        "the job-offer cooldown is not saved"
    ));
}

/// KNOWN BUG #1261 (staff nationality): a staff member made during a season has no `birth_country` and an
/// empty `football_nation` until the save is loaded, which fills both in. Delete this test with the
/// fix.
#[test]
fn known_bug_staff_made_during_a_season_have_no_nationality_until_a_reload() {
    assert!(known_bug_still_happens(
        "staff made during a season have no nationality until a reload (#1261)"
    ));
}

/// KNOWN BUG #1262 (weak foot): a player made during a season has the weak foot the generator gave him
/// until the load re-infers it (`player_identity.rs`), so the value a user sees changes on a
/// reload. Delete this test with the fix.
#[test]
fn known_bug_players_made_during_a_season_change_weak_foot_on_a_reload() {
    assert!(known_bug_still_happens(
        "players made during a season change weak foot on a reload (#1262)"
    ));
}
