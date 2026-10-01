//! End-to-end gameplay scenarios.
//!
//! These tests drive the real game pipeline (`turn::process_day`) over long
//! stretches of game time and assert *invariants* that must hold for any
//! season, regardless of how matches happen to play out. They are deliberately
//! outcome-independent: a match-engine balance change should never break them.
//!
//! The starting world is built from an explicit seed (`make_scenario_game`), so
//! the initial state is fully reproducible. The season *trajectory* is not yet
//! deterministic (the match engine and several turn subsystems still draw from
//! ambient randomness), which is why assertions check properties rather than
//! exact values. See the scenario-test notes in the PR for the path to full
//! determinism.

use chrono::{TimeZone, Utc};
use domain::manager::Manager;
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::generator::{
    DefinitionSources, WorldGenConfig, generate_world_data_seeded_with,
    repair_opening_youth_academies,
};
use ofm_core::turn;

// ---------------------------------------------------------------------------
// Fixture builder
// ---------------------------------------------------------------------------

/// Build a fully playable game from generated world data, with the manager
/// assigned to the first team in the world. Starts on 2026-07-01 (season start).
///
/// The world is generated from `seed`, so the starting state is reproducible:
/// a failure in CI can be replayed locally by running with the same seed.
fn make_scenario_game(seed: u64) -> Game {
    // A small, reproducible world keeps the full-season simulation fast; the
    // invariants under test hold for any world size.
    let world = generate_world_data_seeded_with(
        seed,
        &WorldGenConfig::compact(),
        &DefinitionSources::embedded_only(),
    );

    let start = Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap();
    let clock = GameClock::new(start);

    let first_team = world
        .teams
        .first()
        .expect("generated world must have at least one team");

    let mut manager = Manager::new(
        "scenario-mgr".to_string(),
        "Scenario".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire(first_team.id.clone());

    let team_ids: Vec<String> = world.teams.iter().map(|t| t.id.clone()).collect();

    let mut game = Game::new(
        clock,
        manager,
        world.teams,
        world.players,
        world.staff,
        vec![],
    );

    game.available_staff_market_last_activity_date = Some(start.format("%Y-%m-%d").to_string());

    repair_opening_youth_academies(&mut game);

    game.league = Some(ofm_core::schedule::generate_league(
        "Scenario League",
        2026,
        &team_ids,
        start,
    ));

    ofm_core::season_context::refresh_game_context(&mut game);

    game
}

// ---------------------------------------------------------------------------
// Driver helpers
// ---------------------------------------------------------------------------

/// Advance the game by `days`, processing one turn per day. Panics in
/// `process_day` are re-raised with the day index so failures are locatable.
fn advance_days(game: &mut Game, days: usize) {
    for day in 0..days {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            turn::process_day(game);
        }));
        if result.is_err() {
            panic!("process_day panicked on day {day}");
        }
    }
}

// ---------------------------------------------------------------------------
// Invariants
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Scenarios
// ---------------------------------------------------------------------------

/// The same seed must always produce the same starting world. This is the
/// foundation every reproducible scenario relies on.
#[test]
fn world_generation_is_reproducible() {
    let a = make_scenario_game(42);
    let b = make_scenario_game(42);

    assert_eq!(a.teams.len(), b.teams.len());
    assert_eq!(a.players.len(), b.players.len());

    // Compare the rng-derived content (attributes, finances, nationalities).
    // IDs are random UUIDs and are deliberately excluded.
    let fingerprint = |g: &Game| -> (i64, i64, Vec<String>) {
        let attr_sum: i64 = g
            .players
            .iter()
            .map(|p| {
                let a = &p.attributes;
                a.pace as i64 + a.passing as i64 + a.shooting as i64 + a.tackling as i64
            })
            .sum();
        let finance_sum: i64 = g
            .teams
            .iter()
            .map(|t| t.finance + t.reputation as i64)
            .sum();
        let nationalities: Vec<String> = g.players.iter().map(|p| p.nationality.clone()).collect();
        (attr_sum, finance_sum, nationalities)
    };

    assert_eq!(
        fingerprint(&a),
        fingerprint(&b),
        "same seed must produce an identical starting world"
    );
}

/// Two different seeds should produce different worlds (the seed actually
/// drives generation, rather than being ignored).
#[test]
fn different_seeds_produce_different_worlds() {
    let a = make_scenario_game(1);
    let b = make_scenario_game(2);

    let nationalities =
        |g: &Game| -> Vec<String> { g.players.iter().map(|p| p.nationality.clone()).collect() };

    assert_ne!(
        nationalities(&a),
        nationalities(&b),
        "different seeds should produce different worlds"
    );
}

/// Every contract runs out, so an AI club only keeps a squad by renewing it.
/// Four years of a whole generated world: each AI club must still be at the
/// squad floor, and the report prints how big each squad is and how many
/// emergency signings the floor had to make — renewals should carry the load,
/// and the top-up should barely fire. Retirements and sales still shrink
/// squads that no youth intake refills, which renewals cannot fix and are not
/// meant to. About half a minute in a release build, so it is run explicitly;
/// `ai_contracts` holds the renewal rule to a focused test in the normal suite.
#[test]
#[ignore = "four seasons of a generated world; run explicitly with --ignored --nocapture"]
fn ai_clubs_keep_a_squad_across_seasons_by_renewing_contracts() {
    use domain::player::PlayerMovementKind;
    use ofm_core::squad_floor::squad_shortfall;

    let mut game = make_scenario_game(3);
    let user_club = game.manager.team_id.clone();
    let ai_clubs: Vec<String> = game
        .teams
        .iter()
        .map(|team| team.id.clone())
        .filter(|id| Some(id) != user_club.as_ref())
        .collect();
    let opening_squads: usize = game
        .players
        .iter()
        .filter(|player| {
            player
                .team_id
                .as_ref()
                .is_some_and(|team_id| ai_clubs.contains(team_id))
        })
        .count();

    advance_days(&mut game, 4 * 365);

    // AI clubs sign free agents only to get back to the floor.
    let top_up_signings = game
        .players
        .iter()
        .flat_map(|player| player.movement_history.iter())
        .filter(|entry| entry.kind == PlayerMovementKind::FreeAgentSigning)
        .filter(|entry| {
            entry
                .to_team_id
                .as_ref()
                .is_some_and(|team_id| ai_clubs.contains(team_id))
        })
        .count();
    println!(
        "{} AI clubs, {opening_squads} players at the start; \
         {top_up_signings} floor top-up signings in four years",
        ai_clubs.len()
    );

    for club in &ai_clubs {
        let registered = game
            .players
            .iter()
            .filter(|player| player.team_id.as_deref() == Some(club.as_str()))
            .count();
        assert!(
            squad_shortfall(&game, club).is_empty(),
            "{club} fell below the squad floor after four years: {:?}",
            squad_shortfall(&game, club)
        );
        println!("{club}: {registered} players after four years");
    }
}

/// Given a generated world run for four seasons,
/// When AI clubs buy players from one another (each purchase makes a contract at the
/// buyer's standard wage, through the one wage rule),
/// Then what those purchases add to the wage bills is a small part of the total, and
/// the report says how far the bills moved for every other reason.
///
/// Measured, not assumed. The first run of this found that every AI club's bill grew
/// by 30-50% over four seasons and ended past the board's policy, with only five
/// AI-to-AI transfers in the whole world: transfers cannot be the cause, so this
/// attributes their share rather than asserting the whole drift is theirs. The drift
/// itself is reported, not asserted here; it belongs to whatever else raises a bill
/// (renewals at market wage, floor top-ups the policy waives).
#[test]
#[ignore = "four seasons of a generated world; run explicitly with --ignored --nocapture"]
fn ai_to_ai_transfers_are_a_small_part_of_how_wage_bills_move() {
    use domain::contract_ledger::ContractSource;
    use domain::player::PlayerMovementKind;
    use ofm_core::finances::calc_wages;

    let mut game = make_scenario_game(5);
    let user_club = game.manager.team_id.clone();
    let ai_clubs: Vec<String> = game
        .teams
        .iter()
        .map(|team| team.id.clone())
        .filter(|id| Some(id) != user_club.as_ref())
        .collect();
    let opening_total: i64 = ai_clubs.iter().map(|id| calc_wages(&game, id)).sum();

    advance_days(&mut game, 4 * 365);

    // What each AI-to-AI purchase committed: the new contract's weekly wage, less what
    // the player was being paid before it.
    let mut transfers = 0;
    let mut committed: i64 = 0;
    for player in &game.players {
        let mut previous_wage = 0_i64;
        for entry in player.movement_history.iter() {
            let Some(record) = &entry.contract else {
                continue;
            };
            let to_ai = entry
                .to_team_id
                .as_ref()
                .is_some_and(|team_id| ai_clubs.contains(team_id));
            let from_ai = entry
                .from_team_id
                .as_ref()
                .is_some_and(|team_id| ai_clubs.contains(team_id));
            if entry.kind == PlayerMovementKind::PermanentTransfer
                && record.source == ContractSource::Transfer
                && to_ai
                && from_ai
            {
                transfers += 1;
                committed += i64::from(record.weekly_wage) - previous_wage;
            }
            previous_wage = i64::from(record.weekly_wage);
        }
    }
    let closing_total: i64 = ai_clubs.iter().map(|id| calc_wages(&game, id)).sum();
    let drift = closing_total - opening_total;
    println!(
        "{transfers} AI-to-AI transfers added {committed}/wk; AI wage bills moved by {drift}/wk \
         in total ({opening_total} -> {closing_total})"
    );
    assert!(
        transfers > 0,
        "the world made no AI-to-AI transfers, so this measured nothing"
    );
    assert!(
        committed.abs() * 20 <= closing_total,
        "AI-to-AI transfers added {committed}/wk, more than 5% of the {closing_total}/wk AI bill"
    );
}
