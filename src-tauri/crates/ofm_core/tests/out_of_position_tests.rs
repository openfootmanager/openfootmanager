//! A player fielded out of position plays worse in a club match than the same player at home,
//! through the path the user's own live match takes.
//!
//! Each test plays two arms whose sides are identical but for where their players are naturally
//! at home: every attribute is the same, so before the handicap the two arms played exactly the
//! same matches, and every delta below was zero.

use chrono::{TimeZone, Utc};
use domain::league::{Fixture, FixtureCompetition, FixtureStatus, League, StandingEntry};
use domain::manager::Manager;
use domain::player::{Player, PlayerAttributes, Position};
use domain::team::Team;
use engine::{EventType, MatchCommand, Side};
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::live_match_manager::{self, LiveMatchSession, MatchMode};

const MATCHES: u64 = 600;

fn attributes() -> PlayerAttributes {
    PlayerAttributes {
        pace: 65,
        stamina: 65,
        strength: 65,
        agility: 65,
        passing: 65,
        shooting: 65,
        tackling: 65,
        dribbling: 65,
        defending: 65,
        positioning: 65,
        vision: 65,
        decisions: 65,
        composure: 65,
        aggression: 65,
        teamwork: 65,
        leadership: 65,
        handling: 65,
        reflexes: 65,
        aerial: 65,
    }
}

/// A player of `team_id` whose natural position is `position`; every attribute is the same.
fn player(team_id: &str, id: &str, position: Position) -> Player {
    let id = format!("{team_id}_{id}");
    let mut player = Player::new(
        id.clone(),
        id.clone(),
        id,
        "1995-01-01".to_string(),
        "England".to_string(),
        position,
        attributes(),
    );
    player.team_id = Some(team_id.to_string());
    player.condition = 100;
    player.fitness = 75;
    player
}

/// A side at home in every slot of a 4-4-2.
fn natural_side(team_id: &str) -> Vec<Player> {
    use Position::*;
    [
        ("gk", Goalkeeper),
        ("lb", LeftBack),
        ("cb1", CenterBack),
        ("cb2", CenterBack),
        ("rb", RightBack),
        ("lm", LeftMidfielder),
        ("cm1", CentralMidfielder),
        ("cm2", CentralMidfielder),
        ("rm", RightMidfielder),
        ("st1", Striker),
        ("st2", Striker),
    ]
    .into_iter()
    .map(|(id, position)| player(team_id, id, position))
    .collect()
}

/// The natural side with its four defenders replaced by strikers alike in every attribute.
fn strikers_at_the_back(team_id: &str) -> Vec<Player> {
    let mut side = natural_side(team_id);
    for player in side
        .iter_mut()
        .filter(|player| player.natural_position.to_group_position() == Position::Defender)
    {
        *player = self::player(
            team_id,
            &format!("{}_striker", player.id),
            Position::Striker,
        );
    }
    side
}

fn club(id: &str) -> Team {
    Team::new(
        id.to_string(),
        format!("{id} FC"),
        id[..3].to_uppercase(),
        "England".to_string(),
        "London".to_string(),
        "Stadium".to_string(),
        30_000,
    )
}

/// The user's club alpha, with the players `alpha`, at home to bravo, a natural side. Alpha's
/// first eleven players are its saved starting XI, in the formation's slot order (goalkeeper,
/// left-back, two centre-backs, right-back, left, two central and right midfielders, two
/// strikers), so each plays exactly the slot it is listed for; the rest are its bench.
fn game(seed: u64, alpha: Vec<Player>) -> Game {
    let mut alpha_club = club("alpha");
    alpha_club.starting_xi_ids = alpha
        .iter()
        .take(11)
        .map(|player| player.id.clone())
        .collect();
    let date = Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap();
    let mut manager = Manager::new(
        "mgr".to_string(),
        "Probe".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("alpha".to_string());
    let mut players = alpha;
    players.extend(natural_side("bravo"));
    let league = League {
        id: "league1".to_string(),
        name: "Test League".to_string(),
        season: 1,
        fixtures: vec![Fixture {
            id: "fix1".to_string(),
            matchday: 1,
            date: "2025-06-15".to_string(),
            home_team_id: "alpha".to_string(),
            away_team_id: "bravo".to_string(),
            competition: FixtureCompetition::League,
            status: FixtureStatus::Scheduled,
            result: None,
            ..Default::default()
        }],
        standings: vec![
            StandingEntry::new("alpha".to_string()),
            StandingEntry::new("bravo".to_string()),
        ],
        ..Default::default()
    };
    let mut game = Game::new(
        GameClock::new(date),
        manager,
        vec![alpha_club, club("bravo")],
        players,
        vec![],
        vec![],
    );
    game.league = Some(league);
    game.seed = seed;
    game
}

/// What bravo (away) did against alpha over every match.
#[derive(Default)]
struct Against {
    duels_won: u32,
    duels_tackled: u32,
    goals: u32,
    saved: u32,
}

impl Against {
    /// bravo's won attacking-third duels against those won or lost to a tackle.
    fn duel_rate(&self) -> f64 {
        f64::from(self.duels_won) / f64::from(self.duels_won + self.duels_tackled)
    }

    /// bravo's goals against its goals and saved shots.
    fn conversion(&self) -> f64 {
        f64::from(self.goals) / f64::from(self.goals + self.saved)
    }
}

/// Plays alpha, built by `alpha`, against bravo over every seed; `before_kick_off` acts on the
/// session first.
fn against(
    matches: u64,
    alpha: &dyn Fn() -> Vec<Player>,
    before_kick_off: &dyn Fn(&mut LiveMatchSession),
) -> Against {
    let mut against = Against::default();
    for seed in 0..matches {
        let mut session =
            live_match_manager::create_live_match(&game(seed, alpha()), 0, MatchMode::Live, false)
                .expect("the fixture starts");
        before_kick_off(&mut session);
        session.run_to_completion();
        for event in session
            .snapshot()
            .events
            .iter()
            .filter(|event| event.side == Side::Away)
        {
            match event.event_type {
                EventType::Dribble => against.duels_won += 1,
                EventType::DribbleTackled => against.duels_tackled += 1,
                EventType::Goal => against.goals += 1,
                EventType::ShotSaved => against.saved += 1,
                _ => {}
            }
        }
    }
    against
}

fn nothing(_: &mut LiveMatchSession) {}

// The out-of-position factor (0.90) on a defender's rating moves an opponent's duel rate by about
// +0.024 between these identical 65-rated sides at the default home advantage, and on a
// goalkeeper's rating moves its conversion on target by about +0.043 (conversion is
// base + (shooter - keeper) / 150, and the keeper loses a tenth of about 65). The bands are half
// to one and a half times that; their floors are about three times the seed-to-seed noise at the
// matches each test plays. Before the handicap every delta was exactly zero.

/// Given the user's side with natural strikers playing all four defensive slots, alike in every
/// attribute to the natural defenders they replace,
/// When the side plays its club matches,
/// Then the opponent wins clearly more attacking-third duels against those defenders.
#[test]
fn strikers_fielded_in_defence_lose_more_duels_than_natural_defenders() {
    let natural = against(MATCHES, &|| natural_side("alpha"), &nothing);
    let misplaced = against(MATCHES, &|| strikers_at_the_back("alpha"), &nothing);
    let delta = misplaced.duel_rate() - natural.duel_rate();
    assert!(
        (0.012..=0.036).contains(&delta),
        "the opponent's duel rate moved {delta:+.4}"
    );
}

/// Given the user's side with no goalkeeper, so a natural centre-back alike in every attribute
/// keeps goal,
/// When the side plays its club matches,
/// Then the opponent converts more of its shots on target than against a natural goalkeeper.
#[test]
fn an_outfielder_in_goal_saves_less_than_a_natural_goalkeeper() {
    let without_a_keeper = || {
        let mut side = natural_side("alpha");
        side[0] = player("alpha", "cb3", Position::CenterBack);
        side
    };
    let natural = against(2 * MATCHES, &|| natural_side("alpha"), &nothing);
    let outfielder = against(2 * MATCHES, &without_a_keeper, &nothing);
    let delta = outfielder.conversion() - natural.conversion();
    assert!(
        (0.022..=0.065).contains(&delta),
        "the opponent's conversion moved {delta:+.4}"
    );
}

const DEFENDERS: [&str; 4] = ["lb", "cb1", "cb2", "rb"];

/// Given the user's side with four natural strikers on the bench, alike in every attribute to the
/// four natural defenders on the other arm's bench,
/// When the four substitutes come on for the back four before kick-off,
/// Then the opponent wins clearly more attacking-third duels against the strikers at the back.
#[test]
fn strikers_brought_on_in_defence_play_at_the_defences_cost() {
    let with_on_the_bench = |position: Position| {
        move || {
            let mut side = natural_side("alpha");
            for defender in DEFENDERS {
                side.push(player(
                    "alpha",
                    &format!("sub_{defender}"),
                    position.clone(),
                ));
            }
            side
        }
    };
    let bring_on_the_subs = |session: &mut LiveMatchSession| {
        for defender in DEFENDERS {
            session
                .apply_command(MatchCommand::Substitute {
                    side: Side::Home,
                    player_off_id: format!("alpha_{defender}"),
                    player_on_id: format!("alpha_sub_{defender}"),
                })
                .expect("the substitute is on the bench");
        }
    };
    let natural = against(
        MATCHES,
        &with_on_the_bench(Position::CenterBack),
        &bring_on_the_subs,
    );
    let misplaced = against(
        MATCHES,
        &with_on_the_bench(Position::Striker),
        &bring_on_the_subs,
    );
    let delta = misplaced.duel_rate() - natural.duel_rate();
    assert!(
        (0.012..=0.036).contains(&delta),
        "the opponent's duel rate moved {delta:+.4}"
    );
}
