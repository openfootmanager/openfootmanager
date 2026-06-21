use chrono::{TimeZone, Utc};
use domain::league::{
    CompetitionScope, CompetitionType, League, StandingEntry,
};
use domain::manager::Manager;
use domain::player::{Player, PlayerAttributes, Position};
use domain::team::Team;
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::slices::teams::{
    TeamsDirectoryQuery, UNGROUPED_LEAGUE_ID, query_directory,
};

fn default_attrs() -> PlayerAttributes {
    PlayerAttributes {
        pace: 60, stamina: 60, strength: 60, agility: 60,
        passing: 60, shooting: 60, tackling: 60, dribbling: 60,
        defending: 60, positioning: 60, vision: 60, decisions: 60,
        composure: 60, aggression: 50, teamwork: 60, leadership: 50,
        handling: 20, reflexes: 20, aerial: 60,
    }
}

fn make_team(id: &str, name: &str, country: &str, city: &str) -> Team {
    let short: String = name.chars().take(3).collect();
    Team::new(
        id.to_string(), name.to_string(), short,
        country.to_string(), city.to_string(),
        "Ground".to_string(), 20_000,
    )
}

fn make_player(id: &str, team_id: Option<&str>, ovr: u8, value: u64) -> Player {
    let mut p = Player::new(
        id.to_string(), format!("{id} short"), format!("{id} full"),
        "2000-01-01".to_string(), "GB".to_string(),
        Position::Midfielder, default_attrs(),
    );
    p.team_id = team_id.map(String::from);
    p.ovr = ovr;
    p.market_value = value;
    p
}

fn make_domestic_league(id: &str, name: &str, country: &str, region: Option<&str>, team_ids: &[&str]) -> League {
    let mut league = League::default();
    league.id = id.to_string();
    league.name = name.to_string();
    league.kind = CompetitionType::League;
    league.scope = CompetitionScope::Domestic;
    league.country_id = Some(country.to_string());
    league.region_id = region.map(String::from);
    league.participant_ids = team_ids.iter().map(|s| s.to_string()).collect();
    league.standings = team_ids
        .iter()
        .map(|id| StandingEntry::new(id.to_string()))
        .collect();
    league
}

fn make_game(teams: Vec<Team>, players: Vec<Player>, competitions: Vec<League>) -> Game {
    let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 7, 1, 12, 0, 0).unwrap());
    let manager = Manager::new(
        "m".to_string(), "M".to_string(), "Gr".to_string(),
        "1980-01-01".to_string(), "GB".to_string(),
    );
    let mut game = Game::new(clock, manager, teams, players, vec![], vec![]);
    game.competitions = competitions;
    game
}

fn empty_query() -> TeamsDirectoryQuery {
    TeamsDirectoryQuery { search: None }
}

#[test]
fn directory_groups_teams_under_their_domestic_league_and_region() {
    let teams = vec![
        make_team("eng1", "Alpha FC", "ENG", "London"),
        make_team("eng2", "Beta FC", "ENG", "Liverpool"),
    ];
    let league = make_domestic_league("epl", "Premier", "ENG", Some("UEFA"), &["eng1", "eng2"]);
    let game = make_game(teams, vec![], vec![league]);

    let dir = query_directory(&game, &empty_query());

    assert_eq!(dir.regions.len(), 1);
    let region = &dir.regions[0];
    assert_eq!(region.id, "UEFA");
    assert_eq!(region.team_count, 2);
    assert_eq!(region.leagues.len(), 1);
    let league = &region.leagues[0];
    assert_eq!(league.id, "epl");
    assert_eq!(league.name, "Premier");
    assert_eq!(league.teams.len(), 2);
}

#[test]
fn cards_include_roster_stats_and_league_position() {
    let team = make_team("t1", "Alpha FC", "ENG", "London");
    let league = make_domestic_league("epl", "Premier", "ENG", Some("UEFA"), &["t1"]);
    let players = vec![
        make_player("p1", Some("t1"), 80, 1_000_000),
        make_player("p2", Some("t1"), 70, 500_000),
    ];
    let game = make_game(vec![team], players, vec![league]);

    let dir = query_directory(&game, &empty_query());

    let card = &dir.regions[0].leagues[0].teams[0];
    assert_eq!(card.roster_size, 2);
    assert_eq!(card.avg_ovr, 75);
    assert_eq!(card.total_value, 1_500_000);
    assert_eq!(card.league_pos, 1);
    let standing = card.standing.as_ref().expect("standing");
    assert_eq!(standing.points, 0);
}

#[test]
fn card_projection_denormalizes_team_fields_needed_by_renderer() {
    let mut team = make_team("t1", "Alpha FC", "ENG", "London");
    team.colors.primary = "#ff0000".to_string();
    team.colors.secondary = "#ffffff".to_string();
    team.founded_year = 1898;
    team.reputation = 750;
    let league = make_domestic_league("epl", "Premier", "ENG", Some("UEFA"), &["t1"]);
    let game = make_game(vec![team], vec![], vec![league]);

    let dir = query_directory(&game, &empty_query());
    let card_team = &dir.regions[0].leagues[0].teams[0].team;

    assert_eq!(card_team.id, "t1");
    assert_eq!(card_team.name, "Alpha FC");
    assert_eq!(card_team.city, "London");
    assert_eq!(card_team.country, "ENG");
    assert_eq!(card_team.colors.primary, "#ff0000");
    assert_eq!(card_team.colors.secondary, "#ffffff");
    assert_eq!(card_team.founded_year, 1898);
    assert_eq!(card_team.reputation, 750);
}

#[test]
fn teams_without_a_league_go_into_ungrouped_bucket() {
    let team = make_team("orphan", "Wandering FC", "ENG", "Nowhere");
    let game = make_game(vec![team], vec![], vec![]);

    let dir = query_directory(&game, &empty_query());

    assert_eq!(dir.regions.len(), 1);
    let region = &dir.regions[0];
    assert_eq!(region.leagues.len(), 1);
    let league = &region.leagues[0];
    assert_eq!(league.id, UNGROUPED_LEAGUE_ID);
    assert_eq!(league.name, "");
    assert_eq!(league.teams.len(), 1);
}

#[test]
fn domestic_filter_excludes_continental_and_cup_competitions() {
    let team = make_team("t1", "Alpha FC", "ENG", "London");
    let mut continental = make_domestic_league("ucl", "Champions", "ENG", Some("UEFA"), &["t1"]);
    continental.scope = CompetitionScope::Continental;
    let mut cup = make_domestic_league("fac", "Cup", "ENG", Some("UEFA"), &["t1"]);
    cup.kind = CompetitionType::Cup;
    let game = make_game(vec![team], vec![], vec![continental, cup]);

    let dir = query_directory(&game, &empty_query());

    let league = &dir.regions[0].leagues[0];
    assert_eq!(league.id, UNGROUPED_LEAGUE_ID);
}

#[test]
fn legacy_game_league_is_used_when_competitions_is_empty() {
    let team = make_team("t1", "Alpha FC", "ENG", "London");
    let legacy = make_domestic_league("legacy", "Legacy", "ENG", Some("UEFA"), &["t1"]);
    let mut game = make_game(vec![team], vec![], vec![]);
    game.league = Some(legacy);

    let dir = query_directory(&game, &empty_query());

    let league = &dir.regions[0].leagues[0];
    assert_eq!(league.id, "legacy");
    assert_eq!(league.name, "Legacy");
}

#[test]
fn league_position_uses_sorted_standings() {
    let teams = vec![
        make_team("t1", "First FC", "ENG", "A"),
        make_team("t2", "Second FC", "ENG", "B"),
        make_team("t3", "Third FC", "ENG", "C"),
    ];
    let mut league = make_domestic_league("epl", "Premier", "ENG", Some("UEFA"), &["t1", "t2", "t3"]);
    league.standings[0].points = 5;
    league.standings[1].points = 9;
    league.standings[2].points = 7;
    let game = make_game(teams, vec![], vec![league]);

    let dir = query_directory(&game, &empty_query());
    let cards = &dir.regions[0].leagues[0].teams;

    assert_eq!(cards[0].team.id, "t2");
    assert_eq!(cards[0].league_pos, 1);
    assert_eq!(cards[1].team.id, "t3");
    assert_eq!(cards[1].league_pos, 2);
    assert_eq!(cards[2].team.id, "t1");
    assert_eq!(cards[2].league_pos, 3);
}

#[test]
fn leagues_within_region_sort_with_ungrouped_last() {
    let teams = vec![
        make_team("z1", "Zulu FC", "ENG", "Z"),
        make_team("a1", "Alpha FC", "ENG", "A"),
        make_team("o1", "Orphan FC", "ENG", "O"),
    ];
    let alpha_league =
        make_domestic_league("a", "Alpha League", "ENG", Some("europe"), &["a1"]);
    let zulu_league =
        make_domestic_league("z", "Zulu League", "ENG", Some("europe"), &["z1"]);
    let game = make_game(teams, vec![], vec![alpha_league, zulu_league]);

    let dir = query_directory(&game, &empty_query());
    let leagues = &dir.regions[0].leagues;

    assert_eq!(leagues[0].id, "a");
    assert_eq!(leagues[1].id, "z");
    assert_eq!(leagues[2].id, UNGROUPED_LEAGUE_ID);
}

#[test]
fn search_filters_team_name_case_insensitively() {
    let teams = vec![
        make_team("t1", "Alpha FC", "ENG", "London"),
        make_team("t2", "Beta FC", "ENG", "Liverpool"),
    ];
    let league = make_domestic_league("epl", "Premier", "ENG", Some("UEFA"), &["t1", "t2"]);
    let game = make_game(teams, vec![], vec![league]);

    let mut q = empty_query();
    q.search = Some("ALPHA".to_string());
    let dir = query_directory(&game, &q);

    let league = &dir.regions[0].leagues[0];
    assert_eq!(league.teams.len(), 1);
    assert_eq!(league.teams[0].team.id, "t1");
    assert_eq!(dir.regions[0].team_count, 1);
}

#[test]
fn search_filters_team_city_case_insensitively() {
    let teams = vec![
        make_team("t1", "Alpha FC", "ENG", "London"),
        make_team("t2", "Beta FC", "ENG", "Liverpool"),
    ];
    let league = make_domestic_league("epl", "Premier", "ENG", Some("UEFA"), &["t1", "t2"]);
    let game = make_game(teams, vec![], vec![league]);

    let mut q = empty_query();
    q.search = Some("liver".to_string());
    let dir = query_directory(&game, &q);

    let league = &dir.regions[0].leagues[0];
    assert_eq!(league.teams.len(), 1);
    assert_eq!(league.teams[0].team.id, "t2");
}

#[test]
fn search_drops_empty_leagues_and_regions() {
    let teams = vec![
        make_team("t1", "Alpha FC", "ENG", "London"),
        make_team("t2", "Beta FC", "ENG", "Liverpool"),
    ];
    let league = make_domestic_league("epl", "Premier", "ENG", Some("UEFA"), &["t1", "t2"]);
    let game = make_game(teams, vec![], vec![league]);

    let mut q = empty_query();
    q.search = Some("NOMATCH".to_string());
    let dir = query_directory(&game, &q);

    assert!(dir.regions.is_empty());
}

#[test]
fn roster_stats_use_only_team_members() {
    let team_a = make_team("a", "Alpha FC", "ENG", "A");
    let team_b = make_team("b", "Beta FC", "ENG", "B");
    let players = vec![
        make_player("ap1", Some("a"), 90, 1_000_000),
        make_player("bp1", Some("b"), 50, 100_000),
        make_player("bp2", Some("b"), 60, 200_000),
    ];
    let league = make_domestic_league("epl", "Premier", "ENG", Some("UEFA"), &["a", "b"]);
    let game = make_game(vec![team_a, team_b], players, vec![league]);

    let dir = query_directory(&game, &empty_query());
    let teams = &dir.regions[0].leagues[0].teams;
    let card_a = teams.iter().find(|c| c.team.id == "a").unwrap();
    let card_b = teams.iter().find(|c| c.team.id == "b").unwrap();

    assert_eq!(card_a.roster_size, 1);
    assert_eq!(card_a.avg_ovr, 90);
    assert_eq!(card_a.total_value, 1_000_000);

    assert_eq!(card_b.roster_size, 2);
    assert_eq!(card_b.avg_ovr, 55);
    assert_eq!(card_b.total_value, 300_000);
}
