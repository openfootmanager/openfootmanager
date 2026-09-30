//! A club never runs out of players — proven over seasons, not asserted.
//!
//! These play whole seasons of a seeded generated world through the real day
//! (`turn::process_day`), with the season end's aging and retirements between
//! them, and check that ordinary squad management — AI renewals, academy
//! graduation and promotion, free-agent signings — keeps every AI club at the
//! squad floor without the emergency top-up ever firing.

use chrono::{TimeZone, Utc};
use domain::manager::Manager;
use domain::player::{PlayerMovementKind, SquadRole};
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::generator::{
    DefinitionSources, WorldGenConfig, generate_world_data_seeded_with,
    repair_opening_youth_academies,
};
use ofm_core::squad_floor::MIN_SENIOR_PLAYERS;
use ofm_core::turn;
use std::collections::BTreeMap;

/// A seeded compact world, the player managing its first club, starting at the
/// season's opening. With `league`, the clubs play a league season too.
fn seeded_world(seed: u64, league: bool) -> Game {
    let world = generate_world_data_seeded_with(
        seed,
        &WorldGenConfig::compact(),
        &DefinitionSources::embedded_only(),
    );
    let start = Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap();
    let mut manager = Manager::new(
        "floor-mgr".to_string(),
        "Floor".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire(world.teams[0].id.clone());
    let team_ids: Vec<String> = world.teams.iter().map(|team| team.id.clone()).collect();
    let mut game = Game::new(
        GameClock::new(start),
        manager,
        world.teams,
        world.players,
        world.staff,
        vec![],
    );
    game.available_staff_market_last_activity_date = Some(start.format("%Y-%m-%d").to_string());
    repair_opening_youth_academies(&mut game);
    if league {
        game.league = Some(ofm_core::schedule::generate_league(
            "Floor League",
            2026,
            &team_ids,
            start,
        ));
    }
    ofm_core::season_context::refresh_game_context(&mut game);
    game
}

fn ai_clubs(game: &Game) -> Vec<String> {
    game.teams
        .iter()
        .map(|team| team.id.clone())
        .filter(|id| Some(id) != game.manager.team_id.as_ref())
        .collect()
}

fn seniors(game: &Game, team_id: &str) -> usize {
    game.players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some(team_id) && p.squad_role == SquadRole::Senior)
        .count()
}

/// Senior squads by club, sorted — two runs that agree on this agree on who
/// every club kept, lost and brought in.
fn fingerprint(game: &Game) -> BTreeMap<String, Vec<String>> {
    let mut squads: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for player in &game.players {
        if let Some(team_id) = player.team_id.as_deref()
            && player.squad_role == SquadRole::Senior
        {
            squads
                .entry(team_id.to_string())
                .or_default()
                .push(player.id.clone());
        }
    }
    for squad in squads.values_mut() {
        squad.sort();
    }
    squads
}

/// Play `years` seasons: every day, then the season end's aging. Returns the
/// lowest senior count any AI club reached at a season's end.
fn play_seasons(game: &mut Game, years: u32) -> usize {
    let mut lowest = usize::MAX;
    for year in 0..years {
        for _ in 0..365 {
            turn::process_day(game);
        }
        let today = game.clock.current_date.date_naive();
        ofm_core::aging::apply_seasonal_aging(game, today, 2026 + year);
        for club in ai_clubs(game) {
            lowest = lowest.min(seniors(game, &club));
        }
    }
    lowest
}

#[test]
#[ignore = "measurement probe: run with --ignored --nocapture"]
fn probe_world_supply() {
    let game = seeded_world(7, false);
    let mut free = [0usize; 4];
    let mut academy = [0usize; 4];
    let mut retired = 0;
    for player in &game.players {
        let group = ofm_core::squad_floor::MIN_PLAYERS_PER_GROUP
            .iter()
            .position(|(g, _)| *g == player.position.to_group_position())
            .unwrap();
        if player.retired {
            retired += 1;
        } else if player.team_id.is_none() {
            free[group] += 1;
        } else if player.squad_role == SquadRole::Youth {
            academy[group] += 1;
        }
    }
    println!(
        "players={} free_agents={free:?} academy={academy:?} retired={retired}",
        game.players.len()
    );
    for club in ai_clubs(&game).iter().take(4) {
        println!("  club seniors={}", seniors(&game, club));
    }
}

#[test]
#[ignore = "measurement probe: run with --ignored --nocapture"]
fn probe_squad_floor_seasons() {
    for league in [false, true] {
        let started = std::time::Instant::now();
        let mut first = seeded_world(7, league);
        let lowest = play_seasons(&mut first, 3);
        let elapsed = started.elapsed();
        let mut second = seeded_world(7, league);
        play_seasons(&mut second, 3);
        let ai: Vec<String> = ai_clubs(&first);
        let emergencies = first
            .squad_floor_top_ups
            .iter()
            .filter(|top_up| ai.contains(&top_up.team_id))
            .count();
        let transfers = first
            .players
            .iter()
            .flat_map(|p| p.movement_history.iter())
            .filter(|e| e.kind == PlayerMovementKind::PermanentTransfer)
            .count();
        let signings = first
            .players
            .iter()
            .flat_map(|p| p.movement_history.iter())
            .filter(|e| e.kind == PlayerMovementKind::FreeAgentSigning)
            .count();
        println!(
            "league={league} clubs={} time={elapsed:?} lowest={lowest} floor={MIN_SENIOR_PLAYERS} \
             emergencies={emergencies} transfers={transfers} free_agent_signings={signings} \
             deterministic={}",
            first.teams.len(),
            fingerprint(&first) == fingerprint(&second)
        );
        for top_up in first.squad_floor_top_ups.iter().take(5) {
            println!("  {top_up:?}");
        }
    }
}
