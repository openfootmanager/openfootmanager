//! A club never runs out of players — proven over seasons, not asserted.
//!
//! These play whole seasons of seeded generated worlds through the real day
//! (`turn::process_day`), with the season end's squad turnover between them —
//! aging, retirements, every club's youth intake and every AI club rebuilding —
//! and check that ordinary squad management keeps every AI club at the squad
//! floor without the emergency top-up ever firing, and that the world's
//! population holds steady rather than draining.
//!
//! The world is seeded; the day is not yet (random events, injuries and the
//! match engine draw from ambient randomness), so the proof is a property: it
//! must hold on every seed, on every run of it. A seeded day is its own change.

use chrono::{TimeZone, Utc};
use domain::manager::Manager;
use domain::player::{Position, SquadRole};
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::generator::{
    DefinitionSources, WorldGenConfig, generate_world_data_seeded_with,
    repair_opening_youth_academies,
};
use ofm_core::squad_floor::squad_shortfall;
use ofm_core::turn;

const SEEDS: [u64; 3] = [7, 19, 42];
/// Seasons the proof plays: long enough for the academies the generator
/// seeded to have graduated, and the free-agent pool it opened with to have
/// turned over, so that what is left is the intake's doing. Without an intake
/// the world drained by the ninth season.
const SEASONS: u32 = 12;

/// A seeded compact world, the player managing its first club, starting at the
/// season's opening, its clubs playing a league.
fn seeded_world(seed: u64) -> Game {
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
    game.league = Some(ofm_core::schedule::generate_league(
        "Floor League",
        2026,
        &team_ids,
        start,
    ));
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

/// The world's population at one season's end.
#[derive(Debug)]
struct Census {
    /// Players who have not retired, wherever they are.
    active: usize,
    /// Of those, the ones with no club.
    free_agents: usize,
    /// Senior players at each AI club.
    ai_seniors: Vec<usize>,
}

fn census(game: &Game) -> Census {
    let active: Vec<_> = game.players.iter().filter(|p| !p.retired).collect();
    Census {
        active: active.len(),
        free_agents: active.iter().filter(|p| p.team_id.is_none()).count(),
        ai_seniors: ai_clubs(game)
            .iter()
            .map(|club| {
                active
                    .iter()
                    .filter(|p| p.team_id.as_deref() == Some(club.as_str()))
                    .filter(|p| p.squad_role == SquadRole::Senior)
                    .count()
            })
            .collect(),
    }
}

/// Play `years` seasons — every day, then the season end's squad turnover —
/// and fail on the first AI club found below the floor at a season's end.
/// Returns the census at each season's end.
fn play_seasons(game: &mut Game, years: u32, label: &str) -> Vec<Census> {
    let mut censuses = Vec::new();
    for year in 0..years {
        for _ in 0..365 {
            turn::process_day(game);
        }
        let today = game.clock.current_date.date_naive();
        ofm_core::end_of_season::apply_season_end_squad_turnover(game, today, 2026 + year);
        for club in ai_clubs(game) {
            assert!(
                squad_shortfall(game, &club).is_empty(),
                "{label}: {club} below the floor after season {year}: {:?}",
                squad_shortfall(game, &club)
            );
        }
        censuses.push(census(game));
    }
    censuses
}

fn ai_emergencies(game: &Game) -> Vec<&ofm_core::squad_floor::SquadFloorTopUp> {
    let ai = ai_clubs(game);
    game.squad_floor_top_ups
        .iter()
        .filter(|top_up| ai.contains(&top_up.team_id))
        .collect()
}

fn academy_players(game: &Game, team_id: &str, group: Position) -> usize {
    game.players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some(team_id))
        .filter(|p| p.squad_role == SquadRole::Youth)
        .filter(|p| p.position.to_group_position() == group)
        .count()
}

// --- World supply ------------------------------------------------------------

/// Given a generated world, every club has two senior keepers and an academy
/// keeper, and no two of a club's players share a shirt number.
#[test]
fn a_generated_club_opens_with_an_academy_keeper() {
    let game = seeded_world(SEEDS[0]);
    for team in &game.teams {
        let senior_keepers = game
            .players
            .iter()
            .filter(|p| p.team_id.as_deref() == Some(team.id.as_str()))
            .filter(|p| p.squad_role == SquadRole::Senior)
            .filter(|p| p.position.to_group_position() == Position::Goalkeeper)
            .count();
        assert!(
            senior_keepers >= 2,
            "{} has {senior_keepers} senior keepers",
            team.id
        );
        assert!(
            academy_players(&game, &team.id, Position::Goalkeeper) >= 1,
            "{} has no academy keeper",
            team.id
        );
        let mut numbers: Vec<u8> = game
            .players
            .iter()
            .filter(|p| p.team_id.as_deref() == Some(team.id.as_str()))
            .filter_map(|p| p.jersey_number)
            .collect();
        let count = numbers.len();
        numbers.sort_unstable();
        numbers.dedup();
        assert_eq!(numbers.len(), count, "{} has a shirt number twice", team.id);
    }
}

/// Given a generated world of N clubs, it opens with free agents in every
/// position group, keepers included, in proportion to N: no club, no
/// contract, no wage, not retired.
#[test]
fn a_generated_world_opens_with_a_free_agent_pool_keepers_included() {
    let game = seeded_world(SEEDS[0]);
    let clubs = game.teams.len();
    for group in [
        Position::Goalkeeper,
        Position::Defender,
        Position::Midfielder,
        Position::Forward,
    ] {
        let pool: Vec<_> = game
            .players
            .iter()
            .filter(|p| p.team_id.is_none() && p.position.to_group_position() == group)
            .collect();
        assert_eq!(pool.len(), clubs, "{group:?} free agents");
        for agent in pool {
            assert!(!agent.retired);
            assert_eq!(agent.contract_end(), None);
            assert_eq!(agent.wage(), 0);
        }
    }
}

/// Given the same seed twice, the free-agent pools match in position, age and
/// rating.
#[test]
fn a_seeded_world_opens_with_the_same_pool() {
    let pool = |game: &Game| {
        let mut agents: Vec<(String, String, u8)> = game
            .players
            .iter()
            .filter(|p| p.team_id.is_none())
            .map(|p| (format!("{:?}", p.position), p.date_of_birth.clone(), p.ovr))
            .collect();
        agents.sort();
        agents
    };
    assert_eq!(pool(&seeded_world(SEEDS[1])), pool(&seeded_world(SEEDS[1])));
}

// --- Seasons -----------------------------------------------------------------

/// Given seeded generated worlds, when twelve seasons are played — every day,
/// then the season end's retirements, intake and rebuild — then no AI club is ever
/// found below the floor at a season's end, and the emergency top-up never
/// fires: on every seed, on both runs of it.
#[test]
fn ai_clubs_stay_at_the_floor_for_seasons_without_an_emergency_top_up() {
    for seed in SEEDS {
        for run in 0..2 {
            let label = format!("seed {seed}, run {run}");
            let mut game = seeded_world(seed);
            play_seasons(&mut game, SEASONS, &label);
            assert!(
                ai_emergencies(&game).is_empty(),
                "{label}: emergency top-ups fired: {:?}",
                ai_emergencies(&game)
            );
        }
    }
}

/// Given a world where one AI club has no academy at all, when twelve seasons
/// are played, it keeps to the floor — from the free-agent market until its
/// intake comes through — without an emergency.
#[test]
fn a_club_with_an_empty_academy_keeps_its_floor_through_the_market() {
    let mut game = seeded_world(SEEDS[2]);
    let club = ai_clubs(&game)[0].clone();
    game.players.retain(|p| {
        !(p.team_id.as_deref() == Some(club.as_str()) && p.squad_role == SquadRole::Youth)
    });

    play_seasons(&mut game, SEASONS, "empty academy");

    assert!(
        !game
            .squad_floor_top_ups
            .iter()
            .any(|top_up| top_up.team_id == club),
        "the club with no academy needed an emergency top-up"
    );
}

/// Given a world with no free agents and no academies anywhere, when a season
/// is played, every day still finishes: clubs that cannot be filled play with
/// who they have, and nobody is created for them — the only newcomers are the
/// season end's youth intake, into the academies.
#[test]
fn a_world_with_no_free_agents_and_no_academies_still_finishes_every_day() {
    let mut game = seeded_world(SEEDS[0]);
    game.players
        .retain(|p| p.team_id.is_some() && p.squad_role == SquadRole::Senior);

    for _ in 0..365 {
        turn::process_day(&mut game);
    }
    let before: std::collections::HashSet<String> =
        game.players.iter().map(|p| p.id.clone()).collect();
    // Every academy is empty, so every club takes the intake an empty academy
    // plans for.
    let intake: usize = game
        .teams
        .iter()
        .map(|_| {
            ofm_core::youth_intake::plan_for(std::iter::empty())
                .groups
                .len()
        })
        .sum();
    let today = game.clock.current_date.date_naive();
    ofm_core::end_of_season::apply_season_end_squad_turnover(&mut game, today, 2026);
    for _ in 0..30 {
        turn::process_day(&mut game);
    }

    let newcomers: Vec<_> = game
        .players
        .iter()
        .filter(|p| !before.contains(&p.id))
        .collect();
    assert_eq!(
        newcomers.len(),
        intake,
        "a player was created beyond the intake"
    );
    let season_end = today.format("%Y-%m-%d").to_string();
    for newcomer in newcomers {
        assert!(
            newcomer.contract_start.as_deref() == Some(season_end.as_str())
                && newcomer.team_id.is_some(),
            "{} was created, and not by the season end's intake",
            newcomer.id
        );
    }
}

/// Given seeded generated worlds, when twelve seasons are played, then the
/// world's population holds steady rather than draining: the number of active
/// players stays within a band of where it started, the free-agent market is
/// never emptied nor flooded, and every AI club's senior squad stays between
/// the floor and a squad a club could actually pick from.
///
/// The bands are measured, not chosen. A generated world opens below the
/// intake's equilibrium — four free agents and four academy players a club,
/// against about seven and six once it settles — so it grows for the first
/// dozen seasons and then holds: over twenty-five seasons of seed 42, active
/// players ran 432 at the opening, 514 by the twelfth season and 501–537 from
/// then on, free agents peaked at 141 for sixteen clubs, and every AI club
/// kept 18–27 seniors. Without the intake the same world had lost half its
/// players by the eleventh season.
#[test]
fn the_worlds_population_stays_stable_over_twelve_seasons() {
    for seed in SEEDS {
        let mut game = seeded_world(seed);
        let clubs = game.teams.len();
        let start = census(&game).active;
        let label = format!("seed {seed}");
        let censuses = play_seasons(&mut game, SEASONS, &label);
        for (year, census) in censuses.iter().enumerate() {
            println!("{label}, season {year}: {census:?}");
            assert!(
                census.active * 10 >= start * 9 && census.active * 10 <= start * 14,
                "{label}: {} active players after season {year}, from {start}",
                census.active
            );
            assert!(
                census.free_agents >= clubs && census.free_agents <= clubs * 12,
                "{label}: {} free agents after season {year}, for {clubs} clubs",
                census.free_agents
            );
            for &seniors in &census.ai_seniors {
                assert!(
                    (ofm_core::squad_floor::MIN_SENIOR_PLAYERS..=30).contains(&seniors),
                    "{label}: an AI club holds {seniors} seniors after season {year}"
                );
            }
        }
        assert!(
            ai_emergencies(&game).is_empty(),
            "{label}: emergency top-ups fired: {:?}",
            ai_emergencies(&game)
        );
    }
}
