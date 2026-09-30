//! A game's days can be replayed: the same seed and the same inputs give the same world.
//!
//! What is compared is a projection of the world — results, tables, players' bodies and
//! clubs' money — and not the whole `Game`: ids minted with `Uuid::new_v4` (inbox
//! messages, news) are a separate source of difference from the dice, and this
//! file is about the dice. A world is built once and cloned for each run, because two
//! generations from one seed still mint different ids. The world is dated for the current year, because a world
//! generated for one year and played on another's clock releases every player.

use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::generator::{
    DefinitionSources, WorldGenConfig, default_opening_year, generate_world_data_seeded_with,
};
use ofm_core::turn;
use ofm_core::world::{ensure_multi_competition_foundations, start_date_for_year};

const DAYS: u32 = 120;

/// The same world with a player in charge of its first club, so the things only a player's
/// club has — scouting, offers, inbox events, finances — run as well.
fn managed_world(world_seed: u64, game_seed: u64) -> Game {
    let mut game = world(world_seed, game_seed);
    let club = game.teams[0].id.clone();
    ofm_core::career::begin_career(
        &mut game,
        &club,
        ofm_core::career::CareerScope::default(),
        domain::stats::StatsState::default(),
    )
    .expect("the career begins");
    game
}

fn world(world_seed: u64, game_seed: u64) -> Game {
    let sources = DefinitionSources::embedded_only();
    let config = WorldGenConfig::compact();
    let data = generate_world_data_seeded_with(world_seed, &config, &sources);
    let manager = domain::manager::Manager::new(
        "determinism-manager".to_string(),
        "Det".to_string(),
        "Erminism".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    let mut game = Game::new(
        GameClock::new(start_date_for_year(default_opening_year() as i32).unwrap()),
        manager,
        data.teams,
        data.players,
        data.staff,
        vec![],
    );
    game.seed = game_seed;
    ensure_multi_competition_foundations(&mut game);
    game
}

/// Only the club scorelines. National teams are left out until their own scheduling is
/// seeded: which nations meet in a window is itself drawn.
fn results(game: &Game) -> String {
    outcome(game)
        .lines()
        .filter(|line| line.starts_with("result ") && !line.contains(" nt-"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Everything the dice decide, in a form that compares equal exactly when the dice agreed.
fn outcome(game: &Game) -> String {
    let mut lines = Vec::new();
    for competition in &game.competitions {
        for fixture in &competition.fixtures {
            if let Some(result) = &fixture.result {
                // Not the fixture id: it is a `Uuid::new_v4` from when the schedule was built.
                lines.push(format!(
                    "result {} {} {}-{} {}-{}",
                    competition.name,
                    fixture.date,
                    fixture.home_team_id,
                    fixture.away_team_id,
                    result.home_goals,
                    result.away_goals
                ));
            }
        }
    }
    for player in &game.players {
        lines.push(format!(
            "{} ovr{} cond{} fit{} morale{} inj{:?} stats{:?}",
            player.id,
            player.ovr,
            player.condition,
            player.fitness,
            player.morale,
            player.injury,
            player.stats
        ));
    }
    for team in &game.teams {
        lines.push(format!("{} finance{}", team.id, team.finance));
    }
    lines.join("\n")
}

/// The first line two outcomes disagree on, so a failure names something to go and look at
/// rather than two walls of text.
fn first_difference(first: &str, second: &str) -> String {
    first
        .lines()
        .zip(second.lines())
        .find(|(a, b)| a != b)
        .map(|(a, b)| format!("{a}\n  !=\n{b}"))
        .unwrap_or_else(|| "the outcomes differ in length only".to_string())
}

fn reseeded(game: &Game, seed: u64) -> Game {
    let mut game = game.clone();
    game.seed = seed;
    game
}

fn play(mut game: Game) -> Game {
    for _ in 0..DAYS {
        turn::process_day(&mut game);
    }
    game
}

/// Given one world and one seed,
/// When it is played for the same days twice,
/// Then the results, tables, bodies and money are the same.
#[test]
fn the_same_seed_and_the_same_inputs_give_the_same_days() {
    let save = world(1, 7);
    let first = outcome(&play(save.clone()));
    let second = outcome(&play(save.clone()));

    assert!(first == second, "{}", first_difference(&first, &second));
}

/// Given one world and one seed,
/// When the same days are played twice,
/// Then every match ends with the same score — the match dice, on their own,
///      before anything around the match is seeded.
#[test]
fn the_same_seed_gives_the_same_scorelines() {
    let save = world(1, 7);
    let first = results(&play(save.clone()));
    let second = results(&play(save.clone()));

    assert!(
        first.lines().count() > 10,
        "the days must have played matches"
    );
    assert!(first == second, "{}", first_difference(&first, &second));
}

/// The control: a different seed over the same world must change something, or the
/// test above could not tell a seeded game from one that ignores its seed.
#[test]
fn a_different_seed_gives_different_days() {
    let save = world(1, 7);
    let first = results(&play(save.clone()));
    let other = results(&play(reseeded(&save, 8)));

    assert_ne!(first, other);
}

/// Given a world with a player in charge of a club,
/// When it is played for the same days twice from the same save and the same seed,
/// Then everything the dice decide is the same, the player's club included.
#[test]
fn a_managed_club_replays_the_same_days() {
    let save = managed_world(1, 7);

    let first = outcome(&play(save.clone()));
    let second = outcome(&play(save));

    assert!(first == second, "{}", first_difference(&first, &second));
}
