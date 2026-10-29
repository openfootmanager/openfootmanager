//! What `create_live_match` takes from each club at kick-off: whether both sides can play, who
//! takes the set pieces, and how the AI manager on each side thinks.

use super::*;
use domain::manager::ManagerCareerStats;
use engine::ai::AiPersonality;

fn create(game: &Game) -> live_match_manager::LiveMatchSession {
    live_match_manager::create_live_match(game, 0, MatchMode::Live, false)
        .expect("the match can be created")
}

fn creating_without(team_id: &str) -> Result<(), String> {
    let mut game = make_game_with_fixture();
    game.players
        .retain(|player| player.team_id.as_deref() != Some(team_id));
    live_match_manager::create_live_match(&game, 0, MatchMode::Live, false).map(|_| ())
}

/// Given a home side with a full squad and an away side with nobody on its books
/// When the match is created
/// Then it is refused: one empty side is enough, the engine cannot play against nobody
#[test]
fn a_side_with_nobody_available_is_refused_even_when_the_other_can_play() {
    assert_eq!(
        creating_without("team2").err().as_deref(),
        Some("be.error.liveMatch.emptySquad")
    );
}

/// Given an away side with a full squad and a home side with nobody on its books
/// When the match is created
/// Then it is refused in the same way
#[test]
fn a_home_side_with_nobody_available_is_refused_too() {
    assert_eq!(
        creating_without("team1").err().as_deref(),
        Some("be.error.liveMatch.emptySquad")
    );
}

/// The starters and the four automatic set-piece choices (captain, penalties, free kicks,
/// corners) one side would get.
fn starters_and_auto_takers(game: &Game, home: bool) -> (Vec<String>, [String; 4]) {
    let snapshot = create(game).snapshot();
    let (team, takers) = if home {
        (&snapshot.home_team, &snapshot.home_set_pieces)
    } else {
        (&snapshot.away_team, &snapshot.away_set_pieces)
    };
    let auto = |role: &Option<String>| role.clone().expect("an automatic choice");
    (
        team.players
            .iter()
            .map(|player| player.id.clone())
            .collect(),
        [
            auto(&takers.captain),
            auto(&takers.penalty_taker),
            auto(&takers.free_kick_taker),
            auto(&takers.corner_taker),
        ],
    )
}

/// An outfield starter who is neither of `taken`.
fn another_outfield_starter(starters: &[String], taken: &[&str]) -> String {
    starters
        .iter()
        .find(|id| !id.contains("_gk") && !taken.contains(&id.as_str()))
        .cloned()
        .expect("an outfield starter")
}

fn as_saved(takers: &engine::live_match::SetPieceTakers) -> [Option<String>; 4] {
    [
        takers.captain.clone(),
        takers.penalty_taker.clone(),
        takers.free_kick_taker.clone(),
        takers.corner_taker.clone(),
    ]
}

/// Given both clubs have saved a captain, a penalty taker, a free-kick taker and a corner taker
///       who all start the match, none of them a player the game would pick
/// When the match is created
/// Then each side's four set-piece roles are the ones its club saved
#[test]
fn the_match_uses_each_clubs_saved_set_piece_takers() {
    let mut game = make_game_with_fixture();
    let mut saved = Vec::new();
    for (index, home) in [(0, true), (1, false)] {
        let (starters, auto) = starters_and_auto_takers(&game, home);
        let mut taken: Vec<String> = auto.to_vec();
        let mut choose = || {
            let names: Vec<&str> = taken.iter().map(String::as_str).collect();
            let chosen = another_outfield_starter(&starters, &names);
            taken.push(chosen.clone());
            chosen
        };
        let roles = &mut game.teams[index].match_roles;
        roles.captain = Some(choose());
        roles.penalty_taker = Some(choose());
        roles.free_kick_taker = Some(choose());
        roles.corner_taker = Some(choose());
        saved.push([
            roles.captain.clone(),
            roles.penalty_taker.clone(),
            roles.free_kick_taker.clone(),
            roles.corner_taker.clone(),
        ]);
    }

    let snapshot = create(&game).snapshot();

    assert_eq!(
        vec![
            as_saved(&snapshot.home_set_pieces),
            as_saved(&snapshot.away_set_pieces)
        ],
        saved
    );
}

/// Given the home club saved as captain its reserve goalkeeper, who does not start
/// When the match is created
/// Then the captain is the one the game would have picked: a captain must be on the pitch
#[test]
fn a_saved_captain_who_does_not_start_gives_way_to_the_auto_pick() {
    let mut game = make_game_with_fixture();
    let (starters, [auto_captain, ..]) = starters_and_auto_takers(&game, true);
    game.teams[0].match_roles.captain = Some(reserve_keeper(&starters));

    let snapshot = create(&game).snapshot();

    assert_eq!(snapshot.home_set_pieces.captain, Some(auto_captain));
}

/// Given the home club saved as penalty taker its reserve goalkeeper, who does not start
/// When the match is created
/// Then the penalty taker is the one the game would have picked
#[test]
fn a_saved_penalty_taker_who_does_not_start_gives_way_to_the_auto_pick() {
    let mut game = make_game_with_fixture();
    let (starters, [_, auto_penalty, ..]) = starters_and_auto_takers(&game, true);
    game.teams[0].match_roles.penalty_taker = Some(reserve_keeper(&starters));

    let snapshot = create(&game).snapshot();

    assert_eq!(snapshot.home_set_pieces.penalty_taker, Some(auto_penalty));
}

/// The home club's goalkeeper who is not in `starters`.
fn reserve_keeper(starters: &[String]) -> String {
    ["team1_gk0", "team1_gk1"]
        .into_iter()
        .find(|id| !starters.iter().any(|starter| starter == id))
        .expect("one keeper is on the bench")
        .to_string()
}

fn record(matches_managed: u32, wins: u32) -> ManagerCareerStats {
    ManagerCareerStats {
        matches_managed,
        wins,
        losses: matches_managed - wins,
        ..ManagerCareerStats::default()
    }
}

fn ai_manager(id: &str, team_id: &str, stats: ManagerCareerStats) -> Manager {
    let mut manager = Manager::new(
        id.to_string(),
        "Ai".to_string(),
        id.to_string(),
        "1975-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire(team_id.to_string());
    manager.career_stats = stats;
    manager
}

/// Given two mid-ranking clubs (reputation 500): the home manager has won 20 of his 30 matches
///       and is listed after the away club's manager, who has no record
/// When the match is created
/// Then the home side's AI is Reactive, read from its own manager's winning record, and the away
///      side's stays a Pragmatist
#[test]
fn a_manager_with_a_winning_record_makes_his_side_reactive() {
    let mut game = make_game_with_fixture();
    game.manager.team_id = None;
    game.teams[0].reputation = 500;
    game.teams[1].reputation = 500;
    game.teams[0].manager_id = Some("ai1".to_string());
    game.teams[1].manager_id = Some("ai2".to_string());
    game.managers = vec![
        ai_manager("ai2", "team2", record(0, 0)),
        ai_manager("ai1", "team1", record(30, 20)),
    ];

    let session = create(&game);

    assert_eq!(
        (session.ai_home.personality, session.ai_away.personality),
        (AiPersonality::Reactive, AiPersonality::Pragmatist)
    );
}

/// Given the user manages the home club (reputation 500) with 20 wins from 30 matches
/// When the match is created
/// Then the home side's AI profile is shaped by the user's own record: Reactive
#[test]
fn the_users_record_shapes_his_own_sides_ai_profile() {
    let mut game = make_game_with_fixture();
    game.teams[0].reputation = 500;
    game.teams[0].manager_id = Some(game.manager.id.clone());
    game.manager.career_stats = record(30, 20);
    // The match reads managers from `game.managers`, which holds a copy of the user; the game
    // copies the user's record into it on load and on every job change.
    game.sync_user_manager_record();

    let session = create(&game);

    assert_eq!(session.ai_home.personality, AiPersonality::Reactive);
}

/// Given a home club of reputation 900 and an away club of reputation 300
/// When the match is created
/// Then each side's AI works from its own club's reputation, and so does its profile: the big
///      club's AI is a Visionary and the small one's a Pragmatist
#[test]
fn each_sides_ai_takes_its_own_clubs_reputation() {
    let mut game = make_game_with_fixture();
    game.teams[0].reputation = 900;
    game.teams[1].reputation = 300;

    let session = create(&game);

    assert_eq!(
        (session.ai_home.reputation, session.ai_away.reputation),
        (900, 300)
    );
    assert_eq!(
        (session.ai_home.personality, session.ai_away.personality),
        (AiPersonality::Visionary, AiPersonality::Pragmatist)
    );
}

/// Given the user manages elsewhere with a winning record, and the home club's manager id names
///       nobody the game knows
/// When the match is created
/// Then the home side's AI has no record to go on and stays a Pragmatist: it does not borrow the
///      user's record
#[test]
fn a_club_whose_manager_is_unknown_does_not_borrow_the_users_record() {
    let mut game = make_game_with_fixture();
    game.manager.team_id = None;
    game.manager.career_stats = record(30, 20);
    game.sync_user_manager_record();
    game.teams[0].reputation = 500;
    game.teams[0].manager_id = Some("nobody-we-know".to_string());

    let session = create(&game);

    assert_eq!(session.ai_home.personality, AiPersonality::Pragmatist);
}
