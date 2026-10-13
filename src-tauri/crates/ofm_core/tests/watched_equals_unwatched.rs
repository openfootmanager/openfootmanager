//! A match nobody watches is the same match as one a person watches: same report, same event
//! log, however the minutes are stepped. This is what keeps background matches from becoming a
//! cheaper copy of the real thing (#810).

use domain::manager::Manager;
use domain::stats::StatsState;
use engine::{MatchCommand, PlayStyle, Side};
use ofm_core::career::{CareerScope, begin_career};
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::generator::{
    DefinitionSources, WorldGenConfig, generate_world_data_seeded_with,
    repair_opening_youth_academies,
};
use ofm_core::live_match_manager::{
    LiveMatchSession, MatchMode, kick_off_live_match, play_unwatched_fixture,
};
use ofm_core::matchday::fixture_allows_extra_time;
use ofm_core::world::start_date_for_year;
use serde_json::Value;

const DEFAULT_FIXTURES: usize = 32;
const SOAK_FIXTURES: usize = 256;

/// England's two divisions: real squads and benches, enough clubs for hundreds of distinct fixtures.
fn england(seed: u64) -> Game {
    let sources = DefinitionSources::embedded_only();
    let mut config = WorldGenConfig::standard_from(&sources);
    config.nations.retain(|nation| nation.code == "ENG");
    config.opening_year = Some(2033);
    let world = generate_world_data_seeded_with(seed, &config, &sources);
    let game_seed = world.generation_seed.unwrap_or(seed);
    let start = start_date_for_year(2033).expect("2033 has a start date");
    let manager = Manager::new(
        "watched-test-manager".into(),
        "Watched".into(),
        "Test".into(),
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

/// `count` fixture indices spread across the user's league, so different clubs and rounds play.
fn spread_fixtures(game: &Game, count: usize) -> Vec<usize> {
    let total = game.league.as_ref().expect("a league").fixtures.len();
    (0..count).map(|n| n * total / count).collect()
}

/// A small deterministic stream for the step budgets, so a failure replays.
struct Budgets(u64);

impl Budgets {
    fn next(&mut self, at_most: u64) -> u16 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        1 + ((self.0 >> 33) % at_most) as u16
    }
}

struct Played {
    report: Value,
    events: Value,
}

fn played(session: LiveMatchSession) -> Played {
    let report = session.match_state.into_report();
    Played {
        events: serde_json::to_value(&report.events).unwrap(),
        report: serde_json::to_value(&report).unwrap(),
    }
}

fn unwatched(game: &Game, fixture: usize) -> Played {
    let mut game = game.clone();
    let report = play_unwatched_fixture(&mut game, fixture)
        .expect("the fixture can be played")
        .report;
    Played {
        events: serde_json::to_value(&report.events).unwrap(),
        report: serde_json::to_value(&report).unwrap(),
    }
}

/// The match as a person sits through it: stepped in random amounts, with a look at the
/// snapshot between steps, and no side belonging to the user.
fn watched(game: &Game, fixture: usize, budgets: &mut Budgets) -> Played {
    let mut game = game.clone();
    let extra_time = fixture_allows_extra_time(&game, fixture);
    let mut session = kick_off_live_match(&mut game, fixture, MatchMode::Spectator, extra_time)
        .expect("the fixture can be kicked off");
    session.user_side = None;
    while !session.is_finished() {
        session.step_many(budgets.next(12));
        let _looked_at = session.snapshot();
    }
    played(session)
}

fn assert_same(label: &str, left: &Played, right: &Played) {
    assert_eq!(left.report, right.report, "{label}: the reports differ");
    assert_eq!(left.events, right.events, "{label}: the event logs differ");
}

fn watched_matches_unwatched(fixtures: usize) {
    let game = england(7);
    let mut budgets = Budgets(11);
    for fixture in spread_fixtures(&game, fixtures) {
        assert_same(
            &format!("fixture {fixture}"),
            &unwatched(&game, fixture),
            &watched(&game, fixture, &mut budgets),
        );
    }
}

/// Given fixtures of a generated league and their kick-off state,
/// When each is played unwatched and, separately, through a live session with no user side
///      stepped in random amounts,
/// Then the reports and the event logs are identical.
#[test]
fn watched_and_unwatched_give_the_same_match() {
    watched_matches_unwatched(DEFAULT_FIXTURES);
}

/// The same, over a long run by hand: `cargo test --test watched_equals_unwatched -- --ignored`.
#[test]
#[ignore = "256 fixtures; run by hand"]
fn watched_and_unwatched_give_the_same_match_over_a_long_run() {
    watched_matches_unwatched(SOAK_FIXTURES);
}

/// The manager commands a person gives during the match, keyed by the minute they are given at.
fn scripted_commands(session: &LiveMatchSession) -> Vec<(u8, MatchCommand)> {
    let snapshot = session.snapshot();
    // A like-for-like change, so the substitution is one the engine accepts.
    let (off, on) = snapshot
        .home_team
        .players
        .iter()
        .find_map(|starter| {
            snapshot
                .home_bench
                .iter()
                .find(|candidate| candidate.position == starter.position)
                .map(|candidate| (starter.id.clone(), candidate.id.clone()))
        })
        .expect("some starter has a bench player for his position");
    vec![
        (
            55,
            MatchCommand::Substitute {
                side: Side::Home,
                player_off_id: off,
                player_on_id: on,
            },
        ),
        (
            70,
            MatchCommand::ChangePlayStyle {
                side: Side::Home,
                play_style: PlayStyle::Attacking,
            },
        ),
    ]
}

/// Plays a live match for a user at home, giving the script's commands at their minutes, and
/// stepping by `budget_for(minutes_left_before_next_command)` each time.
fn scripted_live_match(
    game: &Game,
    fixture: usize,
    mut budget_for: impl FnMut(u16) -> u16,
) -> Played {
    let mut game = game.clone();
    let extra_time = fixture_allows_extra_time(&game, fixture);
    let mut session = kick_off_live_match(&mut game, fixture, MatchMode::Live, extra_time)
        .expect("the fixture can be kicked off");
    session.user_side = Some(Side::Home);
    let mut script = scripted_commands(&session).into_iter().peekable();
    while !session.is_finished() {
        let minute = u16::from(session.match_state.minute());
        while let Some((_, command)) = script.next_if(|(at, _)| u16::from(*at) <= minute) {
            session
                .apply_command(command)
                .expect("a scripted command is one the match accepts");
        }
        let until_next_command = script.peek().map_or(u16::MAX, |(at, _)| {
            u16::from(*at).saturating_sub(minute).max(1)
        });
        let budget = budget_for(until_next_command)
            .min(until_next_command)
            .max(1);
        session.step_many(budget);
    }
    played(session)
}

/// Given a live match with a scripted set of manager commands,
/// When it is stepped one minute at a time, and again in larger random steps,
/// Then the result is identical.
#[test]
fn step_budgets_do_not_change_a_scripted_match() {
    let game = england(7);
    let mut budgets = Budgets(23);
    for fixture in spread_fixtures(&game, DEFAULT_FIXTURES / 2) {
        let one_at_a_time = scripted_live_match(&game, fixture, |_| 1);
        let in_chunks = scripted_live_match(&game, fixture, |_| budgets.next(15));
        assert_same(&format!("fixture {fixture}"), &one_at_a_time, &in_chunks);
    }
}
