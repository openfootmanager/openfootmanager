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
//!
//! What is compared: the game itself; the five `#[serde(skip)]` fields the game keeps in tables of
//! their own (`edition_archive`, `retired_players`, `retired_managers`, `archived_messages` and
//! `cash_journal`), which a JSON of the game cannot show (the other `#[serde(skip)]` fields,
//! `calendar_fingerprint`, `day_start`, `cash_journal_dirty_ids` and `squad_floor_top_ups`, are
//! session state that a load starts empty by design); and the stats state, read back with `load_stats_state`. Rules of the comparison:
//! lists of records with an `id` are matched by id and counted (a load re-sorts the inbox, which
//! loses nothing); integers are compared exactly and only floats within a relative 1e-9 (one ulp of
//! f64 text); the game saved has its calendar settled first, see [`career_with_settled_calendar`].

use std::collections::HashMap;
use std::sync::OnceLock;

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
fn saved_and_loaded(game: &Game, stats: &StatsState) -> (Game, StatsState) {
    let saves = tempfile::tempdir().unwrap();
    let mut writer = SaveManager::init(saves.path()).unwrap();
    let save_id = writer
        .create_save_with_stats(game, stats, "Round trip")
        .unwrap();
    let mut reader = SaveManager::init(saves.path()).unwrap();
    let loaded = reader
        .load_game(&save_id)
        .unwrap_or_else(|error| panic!("the save did not load back: {error}"));
    let loaded_stats = reader
        .load_stats_state(&save_id)
        .unwrap_or_else(|error| panic!("the stats did not load back: {error}"));
    (loaded, loaded_stats)
}

/// One place where the game read back is not the game saved.
#[derive(Debug, Clone)]
struct Difference {
    path: String,
    saved: Value,
    loaded: Value,
}

impl std::fmt::Display for Difference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let clip = |value: &Value| {
            let text = value.to_string();
            if text.chars().count() > 120 {
                format!("{}...", text.chars().take(120).collect::<String>())
            } else {
                text
            }
        };
        write!(
            f,
            "{}: saved {} loaded {}",
            self.path,
            clip(&self.saved),
            clip(&self.loaded)
        )
    }
}

fn by_id(items: &[Value]) -> bool {
    !items.is_empty() && items.iter().all(|item| item["id"].is_string())
}

fn records_by_id(items: &[Value]) -> HashMap<String, Vec<&Value>> {
    let mut grouped: HashMap<String, Vec<&Value>> = HashMap::new();
    for item in items {
        grouped
            .entry(item["id"].as_str().unwrap_or_default().to_string())
            .or_default()
            .push(item);
    }
    grouped
}

/// Every place at which `saved` and `loaded` differ. No cap: a cap that is reached before a real
/// loss is listed would hide that loss.
fn differences(saved: &Value, loaded: &Value) -> Vec<Difference> {
    fn walk(path: &str, saved: &Value, loaded: &Value, found: &mut Vec<Difference>) {
        if saved == loaded {
            return;
        }
        let leaf = |found: &mut Vec<Difference>| {
            found.push(Difference {
                path: path.to_string(),
                saved: saved.clone(),
                loaded: loaded.clone(),
            });
        };
        match (saved, loaded) {
            (Value::Object(left), Value::Object(right)) => {
                for key in left
                    .keys()
                    .chain(right.keys().filter(|key| !left.contains_key(*key)))
                {
                    walk(
                        &format!("{path}.{key}"),
                        left.get(key).unwrap_or(&Value::Null),
                        right.get(key).unwrap_or(&Value::Null),
                        found,
                    );
                }
            }
            (Value::Array(left), Value::Array(right)) if by_id(left) && by_id(right) => {
                // Records with an id are compared record by record, not position by position: a
                // load may order a list differently, which loses nothing. The count of each id is
                // compared too, so a dropped record whose id is shared is not collapsed away.
                let (saved_by_id, loaded_by_id) = (records_by_id(left), records_by_id(right));
                for (id, records) in &saved_by_id {
                    let others = loaded_by_id.get(id).map_or(&[][..], Vec::as_slice);
                    if records.len() != others.len() {
                        found.push(Difference {
                            path: format!("{path}[{id}]"),
                            saved: Value::from(records.len()),
                            loaded: Value::from(others.len()),
                        });
                    }
                    for (index, (l, r)) in records.iter().zip(others).enumerate() {
                        let label = if records.len() > 1 {
                            format!("{id}#{index}")
                        } else {
                            id.clone()
                        };
                        walk(&format!("{path}[{label}]"), l, r, found);
                    }
                }
                for (id, records) in &loaded_by_id {
                    if !saved_by_id.contains_key(id) {
                        found.push(Difference {
                            path: format!("{path}[{id}]"),
                            saved: Value::from(0),
                            loaded: Value::from(records.len()),
                        });
                    }
                }
            }
            (Value::Array(left), Value::Array(right)) if left.len() == right.len() => {
                for (index, (l, r)) in left.iter().zip(right).enumerate() {
                    walk(&format!("{path}[{index}]"), l, r, found);
                }
            }
            (Value::Number(left), Value::Number(right))
                if (left.is_f64() || right.is_f64())
                    && left.as_f64().zip(right.as_f64()).is_some_and(|(l, r)| {
                        // A float written as JSON text and read back can land one ulp away.
                        // Integers never get this: a truncated balance or seed must show.
                        (l - r).abs() <= 1e-9 * l.abs().max(r.abs()).max(1.0)
                    }) => {}
            _ => leaf(found),
        }
    }
    let mut found = Vec::new();
    walk("game", saved, loaded, &mut found);
    found
}

/// A game with what a load repairs already repaired, so that what is compared is what the save has
/// to keep and not the repairs the load is meant to make:
/// - the calendar: a load runs `repair_loaded_calendar`, which moves clashing unplayed matches to
///   the earliest legal day and records them as postponed. On a game the game itself just produced
///   it should find nothing to move; it does (#1265), and the count is returned, see
///   `known_bug_a_reload_reschedules_matches_the_live_game_scheduled`;
/// - the cash journal: a load runs `backfill_opening_balances`, which gives every club the opening
///   balance post a journal that was never posted to does not have yet (the live game writes it
///   with a club's first post).
fn career_with_settled_calendar(game: &Game) -> (Game, usize) {
    let mut settled = game.clone();
    let moved = ofm_core::calendar_repair::repair_loaded_calendar(&mut settled);
    ofm_core::finances::backfill_opening_balances(&mut settled);
    (settled, moved)
}

/// Everything the save keeps, as one value for each part: the game (its `#[serde(skip)]` fields
/// are not in its JSON), the five fields kept in tables of their own, and the stats.
fn parts(game: &Game, stats: &StatsState) -> [(&'static str, Value); 7] {
    [
        ("game", serde_json::to_value(game).unwrap()),
        (
            "edition_archive",
            serde_json::to_value(&game.edition_archive).unwrap(),
        ),
        (
            "retired_players",
            serde_json::to_value(&game.retired_players).unwrap(),
        ),
        (
            "retired_managers",
            serde_json::to_value(&game.retired_managers).unwrap(),
        ),
        (
            "archived_messages",
            serde_json::to_value(&game.archived_messages).unwrap(),
        ),
        (
            "cash_journal",
            serde_json::to_value(&*game.cash_journal).unwrap(),
        ),
        ("stats", serde_json::to_value(stats).unwrap()),
    ]
}

/// Every difference between `game` (with `stats`) and what a save and load gives back.
fn every_difference(game: &Game, stats: &StatsState) -> Vec<Difference> {
    let (settled, _) = career_with_settled_calendar(game);
    let (loaded, loaded_stats) = saved_and_loaded(&settled, stats);
    let mut found = Vec::new();
    for ((name, saved), (_, read_back)) in parts(&settled, stats)
        .into_iter()
        .zip(parts(&loaded, &loaded_stats))
    {
        let (saved, read_back) = if name == "game" {
            (saved, read_back)
        } else {
            (
                records_in_a_fixed_order(&saved),
                records_in_a_fixed_order(&read_back),
            )
        };
        for mut difference in differences(&saved, &read_back) {
            difference.path = difference.path.replacen("game", name, 1);
            found.push(difference);
        }
    }
    found
}

/// An archive with its records put in a fixed order (by the text of each record), and nothing
/// below them touched. The archives are read back in the order their tables give, not the order
/// they were saved in, which loses nothing; what must match is the set of records. The order
/// INSIDE a record can be the data (an archived edition's `standings` has no position field: its
/// order is the final table), so it is left alone and a scrambled table is a difference.
fn records_in_a_fixed_order(value: &Value) -> Value {
    match value {
        Value::Array(items) => {
            let mut items = items.clone();
            items.sort_by_key(Value::to_string);
            Value::Array(items)
        }
        other => other.clone(),
    }
}

fn text(value: &Value) -> &str {
    value.as_str().unwrap_or_default()
}

/// The known bugs, by what they are. A difference is a known bug only in the direction and for the
/// population the bug describes, so a wider loss on the same field still fails the loss check.
/// Each is pinned by a test that says it STILL happens (so fixing it makes that test fail, to be
/// deleted with the fix).
fn known_bug(difference: &Difference) -> Option<&'static str> {
    let path = difference.path.as_str();
    if path == "game.days_since_last_job_offer"
        && difference.saved.is_number()
        && difference.loaded.is_null()
    {
        return Some("the job-offer cooldown is not saved (#1260)");
    }
    let staff_nationality = path.starts_with("game.staff[")
        && (path.ends_with("].birth_country") || path.ends_with("].football_nation"));
    if staff_nationality
        && (difference.saved.is_null() || text(&difference.saved).is_empty())
        && !text(&difference.loaded).is_empty()
    {
        return Some("staff made during a season have no nationality until a reload (#1261)");
    }
    // Any player, not only the season's own: a player develops during the season, and the load
    // re-infers his weak foot from the attributes he has now, one step away from the one he kept.
    if path.starts_with("game.players[")
        && path.ends_with("].weak_foot")
        && let (Some(saved), Some(loaded)) = (difference.saved.as_i64(), difference.loaded.as_i64())
        && (saved - loaded).abs() == 1
    {
        return Some("a player's weak foot changes on a reload (#1262)");
    }
    None
}

fn assert_nothing_lost(game: &Game, stats: &StatsState, moment: &str) {
    let lost: Vec<String> = every_difference(game, stats)
        .into_iter()
        .filter(|difference| known_bug(difference).is_none())
        .map(|difference| difference.to_string())
        .collect();
    assert!(
        lost.is_empty(),
        "{moment}: what was read back differs from what was saved ({} paths, first 40):\n{}",
        lost.len(),
        lost.iter().take(40).cloned().collect::<Vec<_>>().join("\n")
    );
}

/// Given a career that has just begun on a generated world,
/// When it is saved and loaded back,
/// Then the game, its archives and its stats are the ones that were saved, field for field.
#[test]
fn a_career_at_its_start_loads_back_unchanged() {
    let (game, stats) = career();

    assert_nothing_lost(&game, &stats, "after career start");
}

/// Given a career in which the manager has signed a free agent,
/// When it is saved and loaded back,
/// Then everything is the one that was saved, with the signing in it.
#[test]
fn a_career_after_a_free_agent_signing_loads_back_unchanged() {
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

    assert_nothing_lost(&game, &stats, "after a free-agent signing");
}

/// A career played through one whole season and its rollover, with what the season did: the stats
/// it recorded, the differences a save and load makes, and the players the world began with. Built
/// once for every test that reads it (a season of the generated world is the cost of this file).
struct RolledOver {
    edition_archive: Vec<domain::edition_archive::CompletedEdition>,
    differences: Vec<Difference>,
    calendar_moved_on_load: usize,
}

fn rolled_over() -> &'static RolledOver {
    static ROLLED_OVER: OnceLock<RolledOver> = OnceLock::new();
    ROLLED_OVER.get_or_init(|| {
        let (mut game, mut stats) = career();
        assert!(
            game.last_season_rollover_on.is_none(),
            "a career that has just begun has not rolled a season over"
        );

        // The harness's loop: a day at a time until the season is complete, then the rollover.
        for _ in 0..600 {
            if is_season_complete(&game) {
                break;
            }
            // A user reads their mail and answers it; the inbox archive takes the read, answered
            // mail that is old enough.
            for message in &mut game.messages {
                message.read = true;
                for action in &mut message.actions {
                    action.resolved = true;
                }
            }
            turn::process_day_with_capture(&mut game, &mut |day_stats| stats.append(day_stats));
        }
        assert!(is_season_complete(&game), "the season finished in 600 days");
        advance_to_next_season(&mut game).expect("the season rolls over");
        assert!(
            game.last_season_rollover_on.is_some(),
            "the game records the rollover that is saved"
        );
        // So that the archive checks cannot pass for want of anything in them.
        assert!(
            !game.edition_archive.is_empty(),
            "the season archived its editions"
        );
        assert!(
            !game.retired_players.is_empty(),
            "the rollover retired players"
        );
        assert!(
            !game.retired_managers.is_empty(),
            "the rollover retired managers"
        );
        assert!(
            !game.archived_messages.is_empty(),
            "the season archived read mail"
        );
        assert!(
            !game.cash_journal.is_empty(),
            "the season posted to the cash journal"
        );
        assert!(
            !stats.player_matches.is_empty(),
            "the season recorded player stats"
        );
        let calendar_moved_on_load = career_with_settled_calendar(&game).1;
        let differences = every_difference(&game, &stats);
        RolledOver {
            edition_archive: game.edition_archive.clone(),
            differences,
            calendar_moved_on_load,
        }
    })
}

/// Given a career played through a whole season and its rollover,
/// When it is saved and loaded back,
/// Then nothing but the known bugs below differs: the new season's calendar, tables, ladders,
///      players, staff, the three archives and the stats included.
#[test]
fn a_career_after_the_rollover_loads_back_unchanged() {
    let state = rolled_over();

    let lost: Vec<String> = state
        .differences
        .iter()
        .filter(|difference| known_bug(difference).is_none())
        .map(|difference| difference.to_string())
        .collect();
    assert!(
        lost.is_empty(),
        "after the rollover: what was read back differs from what was saved ({} paths, first 40):\n{}",
        lost.len(),
        lost.iter().take(40).cloned().collect::<Vec<_>>().join("\n")
    );
}

fn known_bug_still_happens(bug: &str) -> bool {
    let state = rolled_over();
    state
        .differences
        .iter()
        .any(|difference| known_bug(difference) == Some(bug))
}

/// KNOWN BUG #1260 (job-offer cooldown): `days_since_last_job_offer` has no column; the load sets
/// it to `None` (`game_persistence.rs`), so a save and load restarts the cooldown. Delete this test
/// with the fix.
#[test]
fn known_bug_the_job_offer_cooldown_is_not_saved() {
    assert!(known_bug_still_happens(
        "the job-offer cooldown is not saved (#1260)"
    ));
}

/// KNOWN BUG #1261 (staff nationality): a staff member made during a season has no `birth_country`
/// and an empty `football_nation` until the save is loaded, which fills both in. Delete this test
/// with the fix.
#[test]
fn known_bug_staff_made_during_a_season_have_no_nationality_until_a_reload() {
    assert!(known_bug_still_happens(
        "staff made during a season have no nationality until a reload (#1261)"
    ));
}

/// KNOWN BUG #1262 (weak foot): the weak foot a player keeps through a season is not the one the
/// load re-infers from the attributes he has by then (`player_identity.rs`), one step away (2 and 3
/// in the first run, two players), so the value a user sees changes on a reload. Which side is right is the fixer's to decide: if the generator's value is right and the
/// inference is wrong, the fix is in `player_identity`, not in the save (a save that stored the
/// generator's value would only have it overwritten by the load). Delete this test with the fix.
#[test]
fn known_bug_a_players_weak_foot_changes_by_one_step_on_a_reload() {
    assert!(known_bug_still_happens(
        "a player's weak foot changes on a reload (#1262)"
    ));
}

/// KNOWN BUG #1265 (a reload reschedules matches). The settled calendar is only a harmless step if the game never leaves anything to settle. On a
/// game the game itself just produced the repair moves fixtures, so a reload reschedules matches
/// the live game scheduled and tells the user they were postponed (`save_manager.rs`). The count is
/// measured below. Delete this test, and settle the calendar nowhere in this file, when the live
/// scheduler stops building clashes.
#[test]
fn known_bug_a_reload_reschedules_matches_the_live_game_scheduled() {
    let (start, _) = career();
    let moved_at_start = career_with_settled_calendar(&start).1;
    let moved_after_a_season = rolled_over().calendar_moved_on_load;

    assert!(
        moved_at_start > 0 && moved_after_a_season > 0,
        "#1265: a reload moved {moved_at_start} fixtures at career start and {moved_after_a_season} after a season"
    );
}

/// Given the archived editions of a season,
/// When one edition's final table is reversed,
/// Then the comparison sees it: the order of `standings` is the table, and the archives are only
///      sorted at the level of their records. (A mutant of the comparison, not of the game.)
#[test]
fn a_reversed_archived_table_is_a_difference() {
    let archive = &rolled_over().edition_archive;
    let mut changed = archive.clone();
    let edition = changed
        .iter_mut()
        .find(|edition| edition.standings.len() > 1)
        .expect("an archived edition has a table");
    edition.standings.reverse();

    let found = differences(
        &records_in_a_fixed_order(&serde_json::to_value(archive).unwrap()),
        &records_in_a_fixed_order(&serde_json::to_value(&changed).unwrap()),
    );

    assert!(!found.is_empty(), "a reversed final table compared equal");
}
