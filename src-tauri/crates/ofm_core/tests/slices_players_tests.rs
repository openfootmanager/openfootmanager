use chrono::{TimeZone, Utc};
use domain::manager::Manager;
use domain::player::{Injury, Player, PlayerAttributes, Position};
use domain::team::Team;
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::slices::players::{
    PlayerSortKey, PlayerStatusFilter, PlayersPageQuery, query_page,
};

fn default_attrs(ovr_hint: u8) -> PlayerAttributes {
    PlayerAttributes {
        pace: ovr_hint, stamina: ovr_hint, strength: ovr_hint, agility: ovr_hint,
        passing: ovr_hint, shooting: ovr_hint, tackling: ovr_hint, dribbling: ovr_hint,
        defending: ovr_hint, positioning: ovr_hint, vision: ovr_hint, decisions: ovr_hint,
        composure: ovr_hint, aggression: 50, teamwork: ovr_hint, leadership: 50,
        handling: 20, reflexes: 20, aerial: ovr_hint,
    }
}

fn make_team(id: &str, name: &str) -> Team {
    let short: String = name.chars().take(3).collect();
    Team::new(
        id.to_string(), name.to_string(), short,
        "GB".to_string(), "City".to_string(), "Ground".to_string(), 20_000,
    )
}

struct PlayerSpec<'a> {
    id: &'a str,
    full_name: &'a str,
    match_name: &'a str,
    dob: &'a str,
    nationality: &'a str,
    position: Position,
    team_id: Option<&'a str>,
    ovr: u8,
    market_value: u64,
    transfer_listed: bool,
    loan_listed: bool,
    injured: bool,
    retired: bool,
}

impl<'a> PlayerSpec<'a> {
    fn new(id: &'a str, full_name: &'a str, team_id: Option<&'a str>) -> Self {
        Self {
            id, full_name, match_name: full_name, dob: "2000-01-01",
            nationality: "GB", position: Position::Midfielder, team_id,
            ovr: 65, market_value: 1_000_000,
            transfer_listed: false, loan_listed: false, injured: false, retired: false,
        }
    }

    fn build(self) -> Player {
        let position = self.position;
        let mut p = Player::new(
            self.id.to_string(), self.match_name.to_string(), self.full_name.to_string(),
            self.dob.to_string(), self.nationality.to_string(),
            position.clone(), default_attrs(self.ovr),
        );
        p.ovr = self.ovr;
        p.natural_position = position;
        p.team_id = self.team_id.map(String::from);
        p.market_value = self.market_value;
        p.transfer_listed = self.transfer_listed;
        p.loan_listed = self.loan_listed;
        p.retired = self.retired;
        if self.injured {
            p.injury = Some(Injury {
                name: "knock".to_string(),
                days_remaining: 5,
            });
        }
        p
    }
}

fn make_game(teams: Vec<Team>, players: Vec<Player>) -> Game {
    let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 7, 1, 12, 0, 0).unwrap());
    let manager = Manager::new(
        "m".to_string(), "M".to_string(), "Gr".to_string(),
        "1980-01-01".to_string(), "GB".to_string(),
    );
    Game::new(clock, manager, teams, players, vec![], vec![])
}

fn baseline_query() -> PlayersPageQuery {
    PlayersPageQuery {
        search: None, position: None, team_id: None,
        status: PlayerStatusFilter::All,
        sort_key: PlayerSortKey::Ovr, sort_asc: false,
        page: 1, page_size: 50,
    }
}

#[test]
fn projection_includes_table_fields_and_denormalized_team_name() {
    let game = make_game(
        vec![make_team("t1", "Alpha FC")],
        vec![PlayerSpec::new("p1", "John Smith", Some("t1")).build()],
    );

    let page = query_page(&game, &baseline_query());

    assert_eq!(page.total, 1);
    assert_eq!(page.items.len(), 1);
    let item = &page.items[0];
    assert_eq!(item.id, "p1");
    assert_eq!(item.full_name, "John Smith");
    assert_eq!(item.team_id.as_deref(), Some("t1"));
    assert_eq!(item.team_name.as_deref(), Some("Alpha FC"));
    assert_eq!(item.ovr, 65);
    assert!(!item.injured);
}

#[test]
fn free_agents_have_no_team_name() {
    let game = make_game(
        vec![make_team("t1", "Alpha FC")],
        vec![PlayerSpec::new("p1", "Free Agent", None).build()],
    );

    let page = query_page(&game, &baseline_query());

    let item = &page.items[0];
    assert!(item.team_id.is_none());
    assert!(item.team_name.is_none());
}

#[test]
fn injury_projects_to_boolean_not_struct() {
    let mut spec = PlayerSpec::new("p1", "Hurt", Some("t1"));
    spec.injured = true;
    let game = make_game(vec![make_team("t1", "Alpha FC")], vec![spec.build()]);

    let page = query_page(&game, &baseline_query());

    assert!(page.items[0].injured);
}

#[test]
fn search_matches_full_name_case_insensitively() {
    let game = make_game(
        vec![make_team("t1", "Alpha FC")],
        vec![
            PlayerSpec::new("p1", "John Smith", Some("t1")).build(),
            PlayerSpec::new("p2", "Alex Keeper", Some("t1")).build(),
        ],
    );
    let mut q = baseline_query();
    q.search = Some("KEEPER".to_string());

    let page = query_page(&game, &q);

    assert_eq!(page.total, 1);
    assert_eq!(page.items[0].id, "p2");
}

#[test]
fn search_matches_match_name_or_nationality() {
    let mut spec1 = PlayerSpec::new("p1", "John Smith", Some("t1"));
    spec1.nationality = "Brazil";
    let mut spec2 = PlayerSpec::new("p2", "Alex Keeper", Some("t1"));
    spec2.match_name = "A. Kpr";
    let game = make_game(vec![make_team("t1", "X")], vec![spec1.build(), spec2.build()]);

    let mut by_match = baseline_query();
    by_match.search = Some("kpr".to_string());
    let by_match_page = query_page(&game, &by_match);
    assert_eq!(by_match_page.items.len(), 1);
    assert_eq!(by_match_page.items[0].id, "p2");

    let mut by_nat = baseline_query();
    by_nat.search = Some("brazil".to_string());
    let by_nat_page = query_page(&game, &by_nat);
    assert_eq!(by_nat_page.items.len(), 1);
    assert_eq!(by_nat_page.items[0].id, "p1");
}

#[test]
fn empty_or_none_search_returns_all() {
    let game = make_game(
        vec![make_team("t1", "X")],
        vec![
            PlayerSpec::new("p1", "A", Some("t1")).build(),
            PlayerSpec::new("p2", "B", Some("t1")).build(),
        ],
    );

    let mut empty = baseline_query();
    empty.search = Some(String::new());
    assert_eq!(query_page(&game, &empty).total, 2);
    assert_eq!(query_page(&game, &baseline_query()).total, 2);
}

#[test]
fn position_filter_matches_grouped_subvariants() {
    let mut striker_spec = PlayerSpec::new("p1", "S", Some("t1"));
    striker_spec.position = Position::Striker;
    let mut gk_spec = PlayerSpec::new("p2", "K", Some("t1"));
    gk_spec.position = Position::Goalkeeper;
    let game = make_game(
        vec![make_team("t1", "X")],
        vec![striker_spec.build(), gk_spec.build()],
    );

    let mut q = baseline_query();
    q.position = Some(Position::Forward);
    let page = query_page(&game, &q);

    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].id, "p1");
}

#[test]
fn team_id_filter_is_exact_match() {
    let game = make_game(
        vec![make_team("t1", "A"), make_team("t2", "B")],
        vec![
            PlayerSpec::new("p1", "Alpha", Some("t1")).build(),
            PlayerSpec::new("p2", "Beta", Some("t2")).build(),
        ],
    );

    let mut q = baseline_query();
    q.team_id = Some("t2".to_string());
    let page = query_page(&game, &q);

    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].id, "p2");
}

#[test]
fn status_transfer_filter_keeps_only_listed_players() {
    let mut transfer_spec = PlayerSpec::new("p1", "X", Some("t1"));
    transfer_spec.transfer_listed = true;
    let mut loan_spec = PlayerSpec::new("p2", "Y", Some("t1"));
    loan_spec.loan_listed = true;
    let game = make_game(
        vec![make_team("t1", "Z")],
        vec![
            transfer_spec.build(),
            loan_spec.build(),
            PlayerSpec::new("p3", "N", Some("t1")).build(),
        ],
    );

    let mut transfer_q = baseline_query();
    transfer_q.status = PlayerStatusFilter::Transfer;
    let transfer_page = query_page(&game, &transfer_q);
    assert_eq!(transfer_page.items.len(), 1);
    assert_eq!(transfer_page.items[0].id, "p1");

    let mut loan_q = baseline_query();
    loan_q.status = PlayerStatusFilter::Loan;
    let loan_page = query_page(&game, &loan_q);
    assert_eq!(loan_page.items.len(), 1);
    assert_eq!(loan_page.items[0].id, "p2");
}

#[test]
fn sort_by_name_supports_both_directions() {
    let game = make_game(
        vec![make_team("t1", "X")],
        vec![
            PlayerSpec::new("p1", "Charlie", Some("t1")).build(),
            PlayerSpec::new("p2", "Alice", Some("t1")).build(),
            PlayerSpec::new("p3", "Bob", Some("t1")).build(),
        ],
    );

    let mut asc = baseline_query();
    asc.sort_key = PlayerSortKey::Name;
    asc.sort_asc = true;
    let asc_page = query_page(&game, &asc);
    let asc_ids: Vec<&str> = asc_page.items.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(asc_ids, vec!["p2", "p3", "p1"]);

    let mut desc = baseline_query();
    desc.sort_key = PlayerSortKey::Name;
    desc.sort_asc = false;
    let desc_ids: Vec<String> =
        query_page(&game, &desc).items.into_iter().map(|p| p.id).collect();
    assert_eq!(desc_ids, vec!["p1", "p3", "p2"]);
}

#[test]
fn sort_by_position_groups_keeper_defender_midfielder_forward() {
    let mut fwd = PlayerSpec::new("fwd", "F", Some("t1"));
    fwd.position = Position::Striker;
    let mut mid = PlayerSpec::new("mid", "M", Some("t1"));
    mid.position = Position::CentralMidfielder;
    let mut def = PlayerSpec::new("def", "D", Some("t1"));
    def.position = Position::CenterBack;
    let mut gk = PlayerSpec::new("gk", "G", Some("t1"));
    gk.position = Position::Goalkeeper;
    let game = make_game(
        vec![make_team("t1", "X")],
        vec![fwd.build(), mid.build(), def.build(), gk.build()],
    );

    let mut q = baseline_query();
    q.sort_key = PlayerSortKey::Position;
    q.sort_asc = true;
    let page = query_page(&game, &q);

    let ids: Vec<&str> = page.items.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, vec!["gk", "def", "mid", "fwd"]);
}

#[test]
fn sort_by_age_ascending_lists_youngest_first() {
    let mut old = PlayerSpec::new("old", "Veteran", Some("t1"));
    old.dob = "1990-01-01";
    let mut young = PlayerSpec::new("young", "Rookie", Some("t1"));
    young.dob = "2005-01-01";
    let game = make_game(vec![make_team("t1", "X")], vec![old.build(), young.build()]);

    let mut q = baseline_query();
    q.sort_key = PlayerSortKey::Age;
    q.sort_asc = true;
    let page = query_page(&game, &q);

    assert_eq!(page.items[0].id, "young");
    assert_eq!(page.items[1].id, "old");
}

#[test]
fn sort_by_ovr_uses_player_ovr_field() {
    let mut high = PlayerSpec::new("hi", "Star", Some("t1"));
    high.ovr = 90;
    let mut low = PlayerSpec::new("lo", "Sub", Some("t1"));
    low.ovr = 55;
    let game = make_game(vec![make_team("t1", "X")], vec![low.build(), high.build()]);

    let mut q = baseline_query();
    q.sort_key = PlayerSortKey::Ovr;
    q.sort_asc = false;
    let page = query_page(&game, &q);

    assert_eq!(page.items[0].id, "hi");
    assert_eq!(page.items[1].id, "lo");
}

#[test]
fn sort_by_value_uses_market_value() {
    let mut cheap = PlayerSpec::new("c", "Cheap", Some("t1"));
    cheap.market_value = 50_000;
    let mut pricey = PlayerSpec::new("p", "Pricey", Some("t1"));
    pricey.market_value = 5_000_000;
    let game =
        make_game(vec![make_team("t1", "X")], vec![cheap.build(), pricey.build()]);

    let mut q = baseline_query();
    q.sort_key = PlayerSortKey::Value;
    q.sort_asc = true;
    let page = query_page(&game, &q);

    assert_eq!(page.items[0].id, "c");
    assert_eq!(page.items[1].id, "p");
}

#[test]
fn sort_by_team_uses_denormalized_team_name() {
    let game = make_game(
        vec![make_team("t1", "Zulu FC"), make_team("t2", "Alpha FC")],
        vec![
            PlayerSpec::new("z", "Z", Some("t1")).build(),
            PlayerSpec::new("a", "A", Some("t2")).build(),
        ],
    );

    let mut q = baseline_query();
    q.sort_key = PlayerSortKey::Team;
    q.sort_asc = true;
    let page = query_page(&game, &q);

    assert_eq!(page.items[0].id, "a");
    assert_eq!(page.items[1].id, "z");
}

#[test]
fn pagination_returns_requested_window_and_total() {
    let mut players = Vec::new();
    for i in 0..7 {
        let mut spec = PlayerSpec::new(
            Box::leak(format!("p{i}").into_boxed_str()),
            Box::leak(format!("Player {i}").into_boxed_str()),
            Some("t1"),
        );
        spec.ovr = (90 - i as u8) * 1;
        players.push(spec.build());
    }
    let game = make_game(vec![make_team("t1", "X")], players);

    let mut q = baseline_query();
    q.page = 2;
    q.page_size = 3;
    let page = query_page(&game, &q);

    assert_eq!(page.total, 7);
    assert_eq!(page.page, 2);
    assert_eq!(page.page_size, 3);
    assert_eq!(page.items.len(), 3);
}

#[test]
fn pagination_beyond_total_returns_empty_items() {
    let game = make_game(
        vec![make_team("t1", "X")],
        vec![PlayerSpec::new("p1", "Only", Some("t1")).build()],
    );

    let mut q = baseline_query();
    q.page = 5;
    q.page_size = 10;
    let page = query_page(&game, &q);

    assert_eq!(page.total, 1);
    assert!(page.items.is_empty());
}
