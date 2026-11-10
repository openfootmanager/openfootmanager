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
//! f64 text); cash and stranded fixtures are normalized before comparison, but the calendar is
//! saved as produced by the game so a load cannot silently reschedule it.

use std::collections::{HashMap, HashSet};
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
                    // A key that is missing on one side is not a key that is null on it: a load
                    // that loses a null-valued key has lost something.
                    if left.contains_key(key) != right.contains_key(key) {
                        found.push(Difference {
                            path: format!("{path}.{key}"),
                            saved: left
                                .get(key)
                                .cloned()
                                .unwrap_or_else(|| Value::from("<missing>")),
                            loaded: right
                                .get(key)
                                .cloned()
                                .unwrap_or_else(|| Value::from("<missing>")),
                        });
                        continue;
                    }
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

/// What the repairs a load makes found to repair in a game the game itself produced.
struct LoadRepairs {
    /// Clashing unplayed matches moved to the earliest legal day and recorded as postponed
    /// (`repair_loaded_calendar`): #1265. The repair reports moved plus dropped; the fixtures it
    /// dropped (ids gone after it) are counted apart, so a drop is not read as a move.
    calendar_moved: usize,
    calendar_dropped: usize,
    /// Fixtures left scheduled in the past, played out by `repair_stranded_fixtures` ("no fixture
    /// stays Scheduled in the past" is an invariant the load enforces).
    stranded: usize,
}

/// A game with what a load repairs already repaired, so that what is compared is what the save has
/// to keep and not the repairs the load is meant to make, in the order the load makes them:
/// - the cash journal: `backfill_opening_balances` gives every club the opening balance post a
///   journal that was never posted to does not have yet (the live game writes it with a club's
///   first post);
/// - stranded fixtures, as in [`LoadRepairs`]. The calendar
///   repair is measured on a separate clone: the actual round trip must detect any load-time move.
fn settled_as_a_load_settles_it(game: &Game) -> (Game, LoadRepairs) {
    let mut settled = game.clone();
    ofm_core::finances::backfill_opening_balances(&mut settled);
    let stranded = ofm_core::catchup::repair_stranded_fixtures(&mut settled);
    let fixture_ids = |game: &Game| -> HashSet<String> {
        game.competitions
            .iter()
            .flat_map(|competition| {
                competition
                    .fixtures
                    .iter()
                    .map(|fixture| fixture.id.clone())
            })
            .collect()
    };
    let mut calendar_probe = settled.clone();
    let before = fixture_ids(&calendar_probe);
    let changed = ofm_core::calendar_repair::repair_loaded_calendar(&mut calendar_probe);
    let calendar_dropped = before.difference(&fixture_ids(&calendar_probe)).count();
    (
        settled,
        LoadRepairs {
            calendar_moved: changed.saturating_sub(calendar_dropped),
            calendar_dropped,
            stranded,
        },
    )
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
    let (settled, _) = settled_as_a_load_settles_it(game);
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

fn assert_nothing_lost(game: &Game, stats: &StatsState, moment: &str) {
    let lost: Vec<String> = every_difference(game, stats)
        .into_iter()
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
    repairs: LoadRepairs,
    calendar_game: Game,
}

fn rolled_over() -> &'static RolledOver {
    static ROLLED_OVER: OnceLock<RolledOver> = OnceLock::new();
    ROLLED_OVER.get_or_init(|| {
        let (mut game, mut stats) = career();
        assert!(
            game.last_season_rollover_on.is_none(),
            "a career that has just begun has not rolled a season over"
        );

        // The harness's loop, twice: a day at a time until the season is complete, then the
        // rollover. Two seasons, because a season of the compact world is about four months and
        // the inbox archive takes only mail more than 180 days old.
        for season in 1..=2 {
            for _ in 0..600 {
                if is_season_complete(&game) {
                    break;
                }
                // A user reads their mail and answers it; the inbox archive takes the read,
                // answered mail that is old enough.
                for message in &mut game.messages {
                    message.read = true;
                    for action in &mut message.actions {
                        action.resolved = true;
                    }
                }
                turn::process_day_with_capture(&mut game, &mut |day_stats| {
                    stats.append(day_stats)
                });
            }
            assert!(
                is_season_complete(&game),
                "season {season} finished in 600 days"
            );
            advance_to_next_season(&mut game).expect("the season rolls over");
        }
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
            "the season archived read mail: {} messages in the inbox, {} read, {} with an unanswered \
             action, oldest dated {:?}, game date {}",
            game.messages.len(),
            game.messages.iter().filter(|m| m.read).count(),
            game.messages
                .iter()
                .filter(|m| m.actions.iter().any(|a| !a.resolved))
                .count(),
            game.messages.iter().map(|m| m.date.clone()).min(),
            game.clock.current_date.date_naive()
        );
        assert!(
            !game.cash_journal.is_empty(),
            "the season posted to the cash journal"
        );
        assert!(
            !stats.player_matches.is_empty(),
            "the season recorded player stats"
        );
        let (calendar_game, repairs) = settled_as_a_load_settles_it(&game);
        let differences = every_difference(&game, &stats);
        RolledOver {
            edition_archive: game.edition_archive.clone(),
            differences,
            repairs,
            calendar_game,
        }
    })
}

/// Given a career played through a whole season and its rollover,
/// When it is saved and loaded back,
/// Then nothing differs: the new season's calendar, tables, ladders,
///      players, staff, the three archives and the stats included.
#[test]
fn a_career_after_the_rollover_loads_back_unchanged() {
    let state = rolled_over();

    let lost: Vec<String> = state
        .differences
        .iter()
        .map(|difference| difference.to_string())
        .collect();
    assert!(
        lost.is_empty(),
        "after the rollover: what was read back differs from what was saved ({} paths, first 40):\n{}",
        lost.len(),
        lost.iter().take(40).cloned().collect::<Vec<_>>().join("\n")
    );
}

/// Given a freshly generated career, when the load-time calendar repair is measured, then it
/// neither moves nor drops fixtures.
#[test]
fn a_new_career_has_no_calendar_repairs() {
    let (game, _) = career();
    let repairs = settled_as_a_load_settles_it(&game).1;
    assert_eq!(
        repairs.calendar_moved, 0,
        "#1265: fixtures moved at career start"
    );
    assert_eq!(
        repairs.calendar_dropped, 0,
        "#1265: fixtures dropped at career start"
    );
}

/// Given a career played through two rollovers, when the load-time calendar repair is measured,
/// then it neither moves nor drops fixtures.
#[test]
fn a_career_after_two_seasons_has_no_calendar_repairs() {
    use domain::league::{FixtureCompetition, FixtureStatus};

    let rolled = rolled_over();
    let game = &rolled.calendar_game;
    let today = game.clock.current_date.date_naive();
    let blackout =
        ofm_core::national_team::international_blackout_around(&game.competitions, today);
    let mut checked = 0;
    for fixture in game.competitions.iter().flat_map(|c| &c.fixtures) {
        if fixture.status != FixtureStatus::Scheduled
            || fixture.competition == FixtureCompetition::InternationalNation
        {
            continue;
        }
        checked += 1;
        let date = chrono::NaiveDate::parse_from_str(&fixture.date, "%Y-%m-%d")
            .expect("a scheduled club fixture has a readable date");
        assert!(
            !blackout.contains(&date),
            "club fixture {} is on blackout {date}",
            fixture.id
        );
    }
    assert!(
        checked > 0,
        "the two-season calendar has scheduled club fixtures to check"
    );
    let plan = ofm_core::calendar_reconcile::plan(
        &game.competitions,
        &blackout,
        today.pred_opt().unwrap(),
    )
    .expect("the two-season calendar can be planned");
    assert!(
        plan.moves.is_empty(),
        "the two-season plan has moves: {:?}",
        plan.moves
    );
    assert!(
        plan.dropped_friendlies.is_empty(),
        "the two-season plan drops friendlies: {:?}",
        plan.dropped_friendlies
    );
    let repairs = &rolled.repairs;
    assert_eq!(
        repairs.calendar_moved, 0,
        "#1265: fixtures moved after two seasons"
    );
    assert_eq!(
        repairs.calendar_dropped, 0,
        "#1265: fixtures dropped after two seasons"
    );
}

/// Given a career played through two seasons,
/// When the load repair checks for fixtures scheduled before the current date,
/// Then it finds none to play out.
#[test]
fn the_live_game_leaves_no_fixtures_scheduled_in_the_past() {
    let stranded = rolled_over().repairs.stranded;

    assert_eq!(
        stranded, 0,
        "a reload played out fixtures left scheduled in the past"
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

/// Given a record that loses a key whose value was null,
/// When the saved and the loaded record are compared, in either direction,
/// Then the difference is reported: a missing key is not a null one.
#[test]
fn a_key_lost_with_a_null_value_is_a_difference() {
    let with_key = serde_json::json!({"id": "p1", "retired": null});
    let without_key = serde_json::json!({"id": "p1"});

    assert_eq!(differences(&with_key, &without_key).len(), 1);
    assert_eq!(differences(&without_key, &with_key).len(), 1);
    assert!(differences(&with_key, &with_key).is_empty());
}
