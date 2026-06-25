use chrono::{TimeZone, Utc};
use domain::manager::Manager;
use domain::player::{Player, PlayerAttributes, Position};
use domain::team::Team;
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::slices::squad::query_squad;

fn default_attrs() -> PlayerAttributes {
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
        aggression: 50,
        teamwork: 65,
        leadership: 50,
        handling: 20,
        reflexes: 20,
        aerial: 65,
    }
}

fn make_team(id: &str, name: &str) -> Team {
    let short: String = name.chars().take(3).collect();
    Team::new(
        id.to_string(),
        name.to_string(),
        short,
        "GB".to_string(),
        "City".to_string(),
        "Ground".to_string(),
        20_000,
    )
}

fn make_player(id: &str, team_id: Option<&str>) -> Player {
    let mut p = Player::new(
        id.to_string(),
        id.to_string(),
        format!("Player {id}"),
        "2000-01-01".to_string(),
        "GB".to_string(),
        Position::Midfielder,
        default_attrs(),
    );
    p.team_id = team_id.map(String::from);
    p
}

fn make_game(teams: Vec<Team>, players: Vec<Player>) -> Game {
    let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 7, 1, 12, 0, 0).unwrap());
    let manager = Manager::new(
        "m".to_string(),
        "M".to_string(),
        "Gr".to_string(),
        "1980-01-01".to_string(),
        "GB".to_string(),
    );
    Game::new(clock, manager, teams, players, vec![], vec![])
}

#[test]
fn squad_returns_only_players_for_the_requested_team() {
    let game = make_game(
        vec![make_team("t1", "Alpha FC"), make_team("t2", "Beta FC")],
        vec![
            make_player("p1", Some("t1")),
            make_player("p2", Some("t1")),
            make_player("p3", Some("t2")),
            make_player("fa", None),
        ],
    );

    let squad = query_squad(&game, "t1");

    let ids: Vec<&str> = squad.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&"p1"));
    assert!(ids.contains(&"p2"));
    assert!(!ids.contains(&"p3"));
    assert!(!ids.contains(&"fa"));
}

#[test]
fn squad_includes_all_players_regardless_of_squad_role() {
    let mut youth = make_player("youth", Some("t1"));
    youth.squad_role = domain::player::SquadRole::Youth;
    let mut senior = make_player("senior", Some("t1"));
    senior.squad_role = domain::player::SquadRole::Senior;
    let game = make_game(vec![make_team("t1", "Alpha FC")], vec![youth, senior]);

    let squad = query_squad(&game, "t1");

    assert_eq!(squad.len(), 2);
}

#[test]
fn squad_matches_full_game_player_filter() {
    let players = vec![
        make_player("p1", Some("t1")),
        make_player("p2", Some("t1")),
        make_player("p3", Some("t2")),
        make_player("fa", None),
    ];
    let game = make_game(
        vec![make_team("t1", "Alpha FC"), make_team("t2", "Beta FC")],
        players.clone(),
    );

    let squad = query_squad(&game, "t1");
    let expected: Vec<&domain::player::Player> = game
        .players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some("t1"))
        .collect();

    assert_eq!(squad.len(), expected.len());
    for (got, exp) in squad.iter().zip(expected.iter()) {
        assert_eq!(got.id, exp.id);
    }
}

#[test]
fn squad_returns_empty_for_unknown_team() {
    let game = make_game(
        vec![make_team("t1", "Alpha FC")],
        vec![make_player("p1", Some("t1"))],
    );

    let squad = query_squad(&game, "t999");

    assert!(squad.is_empty());
}
