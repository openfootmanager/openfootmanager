//! How often a club match starts a player away from his position, and what his post-match rating
//! is there, on the path the game plays.
//!
//! ```text
//! cargo test --release --manifest-path src-tauri/Cargo.toml -p ofm_core --test familiarity_probe -- --ignored --nocapture
//! ```
//!
//! Generates England's divisions, begins a career at the best-reputed club, and kicks off every
//! fixture of the user's league from the opening state through `kick_off_live_match` +
//! `run_to_completion`, with AI managers on both touchlines. Each starter is labelled by his
//! familiarity (`player_rating::familiarity`) with the formation slot he kicks off in, read from
//! the kickoff snapshot, the lineup the engine plays. The user's club is reported apart from AI
//! clubs. Calibration probe for the out-of-position handicap; it measures, it asserts nothing.

use std::collections::HashMap;

use domain::manager::Manager;
use domain::player::Player;
use domain::stats::StatsState;
use ofm_core::career::{CareerScope, begin_career};
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::generator::{
    DefinitionSources, WorldGenConfig, generate_world_data_seeded_with,
    repair_opening_youth_academies,
};
use ofm_core::live_match_manager::{MatchMode, kick_off_live_match};
use ofm_core::matchday::fixture_allows_extra_time;
use ofm_core::player_rating::{Familiarity, familiarity, formation_slots};
use ofm_core::world::start_date_for_year;

fn env_or(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn england(seed: u64) -> Game {
    let sources = DefinitionSources::embedded_only();
    let mut config = WorldGenConfig::standard_from(&sources);
    config.nations.retain(|nation| nation.code == "ENG");
    config.opening_year = Some(2033);
    let world = generate_world_data_seeded_with(seed, &config, &sources);
    let game_seed = world.generation_seed.unwrap_or(seed);
    let start = start_date_for_year(2033).expect("2033 has a start date");
    let manager = Manager::new(
        "probe-manager".into(),
        "Probe".into(),
        "Manager".into(),
        "1980-01-01".into(),
        "England".into(),
    );
    let mut game = Game::new(
        GameClock::new(start),
        manager,
        world.teams,
        world.players,
        world.staff,
        vec![],
    );
    game.seed = game_seed;
    game.available_staff_market_last_activity_date = Some(start.format("%Y-%m-%d").to_string());
    repair_opening_youth_academies(&mut game);
    let club = game
        .teams
        .iter()
        .max_by(|a, b| {
            a.reputation
                .cmp(&b.reputation)
                .then_with(|| b.id.cmp(&a.id))
        })
        .map(|team| team.id.clone())
        .expect("the world has clubs");
    begin_career(
        &mut game,
        &club,
        CareerScope::default(),
        StatsState::default(),
    )
    .expect("the career begins");
    game
}

/// Starts and minutes in each familiarity, and the post-match ratings of rated starts.
#[derive(Default)]
struct Bucket {
    starts: u64,
    minutes: u64,
    rating_sum: f64,
    rated: u64,
}

#[derive(Default)]
struct Clubs {
    by_familiarity: [Bucket; 3],
    /// Out-of-position starts by (natural position, slot).
    misplaced: HashMap<(String, String), u32>,
    /// Out-of-position starts while the rest of the fit squad held a player natural at that
    /// slot, or one who can play it.
    misplaced_with_a_natural_on_the_bench: u64,
    misplaced_with_a_can_play_on_the_bench: u64,
}

fn index(fit: Familiarity) -> usize {
    match fit {
        Familiarity::Natural => 0,
        Familiarity::CanPlay => 1,
        Familiarity::OutOfPosition => 2,
    }
}

fn report(label: &str, clubs: &Clubs) {
    let minutes: u64 = clubs.by_familiarity.iter().map(|b| b.minutes).sum();
    let starts: u64 = clubs.by_familiarity.iter().map(|b| b.starts).sum();
    let names = ["Natural", "CanPlay", "OutOfPosition"];
    let parts: Vec<String> = clubs
        .by_familiarity
        .iter()
        .zip(names)
        .map(|(bucket, name)| {
            format!(
                "{name} {:.2}% of minutes ({} starts, mean rating {:.3} over {})",
                100.0 * bucket.minutes as f64 / minutes.max(1) as f64,
                bucket.starts,
                bucket.rating_sum / bucket.rated.max(1) as f64,
                bucket.rated
            )
        })
        .collect();
    println!(
        "FAMILIARITY {label}: {starts} starts | {}",
        parts.join(" | ")
    );
    println!(
        "FAMILIARITY {label}: of {} out-of-position starts, the rest of the fit squad held a natural for the slot in {} and a player who can play it in {}",
        clubs.by_familiarity[2].starts,
        clubs.misplaced_with_a_natural_on_the_bench,
        clubs.misplaced_with_a_can_play_on_the_bench
    );
    let mut common: Vec<_> = clubs.misplaced.iter().collect();
    common.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    for ((natural, slot), count) in common.into_iter().take(10) {
        println!("FAMILIARITY {label} misplaced: a natural {natural} in {slot}: {count} starts");
    }
}

/// Given England's opening state, three worlds,
/// When every fixture of the user's league is kicked off and played,
/// Then lines report starters by familiarity with their kickoff slot, for AI clubs and the
/// user's club, with their mean post-match ratings and the commonest misplacements.
#[test]
#[ignore = "plays whole leagues; run explicitly"]
fn report_starters_by_familiarity() {
    let seeds = env_or("OFM_PROBE_SEEDS", 3);
    let mut ai = Clubs::default();
    let mut user = Clubs::default();
    let mut matches = 0u64;
    for seed in 0..seeds as u64 {
        let game = england(20261018 + seed);
        let players: HashMap<String, Player> = game
            .players
            .iter()
            .map(|player| (player.id.clone(), player.clone()))
            .collect();
        let user_club = game.manager.team_id.clone();
        let fixtures = game.league.as_ref().expect("a league").fixtures.len();
        for fixture in 0..fixtures {
            let mut g = game.clone();
            let extra_time = fixture_allows_extra_time(&g, fixture);
            let Ok(mut session) =
                kick_off_live_match(&mut g, fixture, MatchMode::Instant, extra_time)
            else {
                continue;
            };
            session.user_side = None;
            let kickoff = session.snapshot();
            let mut starters: HashMap<String, (bool, Familiarity, String, String)> = HashMap::new();
            // The best familiarity anyone else in the fit squad has with each misplaced starter's slot.
            let mut bench_best: HashMap<String, Familiarity> = HashMap::new();
            for team in [&kickoff.home_team, &kickoff.away_team] {
                let is_user = user_club.as_deref() == Some(team.id.as_str());
                let slots = formation_slots(&team.formation);
                for (slot_index, engine_player) in team.players.iter().enumerate() {
                    let (Some(slot), Some(player)) =
                        (slots.get(slot_index), players.get(&engine_player.id))
                    else {
                        continue;
                    };
                    let fit = familiarity(player, slot);
                    if fit == Familiarity::OutOfPosition {
                        // The rest of the fit squad, on the bench or not named at all.
                        let starting: Vec<&str> = team
                            .players
                            .iter()
                            .map(|starter| starter.id.as_str())
                            .collect();
                        let best = players
                            .values()
                            .filter(|other| {
                                other.team_id.as_deref() == Some(team.id.as_str())
                                    && other.injury.is_none()
                                    && !starting.contains(&other.id.as_str())
                            })
                            .map(|other| familiarity(other, slot))
                            .min_by_key(|bench_fit| index(*bench_fit))
                            .unwrap_or(Familiarity::OutOfPosition);
                        bench_best.insert(engine_player.id.clone(), best);
                    }
                    starters.insert(
                        engine_player.id.clone(),
                        (
                            is_user,
                            familiarity(player, slot),
                            format!("{:?}", player.natural_position),
                            format!("{slot:?}"),
                        ),
                    );
                }
            }
            session.run_to_completion();
            let match_report = session.match_state.into_report();
            matches += 1;
            for (id, (is_user, fit, natural, slot)) in starters {
                let clubs = if is_user { &mut user } else { &mut ai };
                let bucket = &mut clubs.by_familiarity[index(fit)];
                bucket.starts += 1;
                if let Some(stats) = match_report.player_stats.get(&id) {
                    bucket.minutes += u64::from(stats.minutes_played);
                    if stats.rating > 0.0 {
                        bucket.rating_sum += f64::from(stats.rating);
                        bucket.rated += 1;
                    }
                }
                if fit == Familiarity::OutOfPosition {
                    *clubs.misplaced.entry((natural, slot)).or_default() += 1;
                    match bench_best.get(&id) {
                        Some(Familiarity::Natural) => {
                            clubs.misplaced_with_a_natural_on_the_bench += 1
                        }
                        Some(Familiarity::CanPlay) => {
                            clubs.misplaced_with_a_can_play_on_the_bench += 1
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    println!("FAMILIARITY {matches} matches, {seeds} England worlds, opening state, live path");
    report("AI clubs", &ai);
    report("user club", &user);
}
