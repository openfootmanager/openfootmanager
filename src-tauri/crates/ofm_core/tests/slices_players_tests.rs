use chrono::{TimeZone, Utc};
use domain::league::League;
use domain::manager::Manager;
use domain::player::{Injury, Player, PlayerAttributes, Position};
use domain::team::Team;
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::game::ScoutingAssignment;
use ofm_core::player_knowledge::VisibleWage;
use ofm_core::slices::players::{
    PlayerScope, PlayerSortKey, PlayerStatusFilter, PlayersPageQuery, PositionFilter, StatusCounts,
    query_page,
};

fn default_attrs(ovr_hint: u8) -> PlayerAttributes {
    PlayerAttributes {
        pace: ovr_hint,
        stamina: ovr_hint,
        strength: ovr_hint,
        agility: ovr_hint,
        passing: ovr_hint,
        shooting: ovr_hint,
        tackling: ovr_hint,
        dribbling: ovr_hint,
        defending: ovr_hint,
        positioning: ovr_hint,
        vision: ovr_hint,
        decisions: ovr_hint,
        composure: ovr_hint,
        aggression: 50,
        teamwork: ovr_hint,
        leadership: 50,
        handling: 20,
        reflexes: 20,
        aerial: ovr_hint,
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
            id,
            full_name,
            match_name: full_name,
            dob: "2000-01-01",
            nationality: "GB",
            position: Position::Midfielder,
            team_id,
            ovr: 65,
            market_value: 1_000_000,
            transfer_listed: false,
            loan_listed: false,
            injured: false,
            retired: false,
        }
    }

    fn build(self) -> Player {
        let position = self.position;
        let mut p = Player::new(
            self.id.to_string(),
            self.match_name.to_string(),
            self.full_name.to_string(),
            self.dob.to_string(),
            self.nationality.to_string(),
            position.clone(),
            default_attrs(self.ovr),
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
        "m".to_string(),
        "M".to_string(),
        "Gr".to_string(),
        "1980-01-01".to_string(),
        "GB".to_string(),
    );
    Game::new(clock, manager, teams, players, vec![], vec![])
}

fn baseline_query() -> PlayersPageQuery {
    PlayersPageQuery {
        search: None,
        position: None,
        positions: vec![],
        age_min: None,
        age_max: None,
        ovr_min: None,
        ovr_max: None,
        value_min: None,
        value_max: None,
        in_budget: false,
        scope: PlayerScope::All,
        team_id: None,
        status: PlayerStatusFilter::All,
        sort_key: PlayerSortKey::Ovr,
        sort_asc: false,
        page: 1,
        page_size: 50,
    }
}

#[test]
fn projection_includes_table_fields_and_denormalized_team_name() {
    let mut game = make_game(
        vec![make_team("t1", "Alpha FC")],
        vec![PlayerSpec::new("p1", "John Smith", Some("t1")).build()],
    );
    // The user's club's own player: his rating is the club's to see.
    game.manager.team_id = Some("t1".to_string());

    let page = query_page(&game, &baseline_query());

    assert_eq!(page.total, 1);
    assert_eq!(page.items.len(), 1);
    let item = &page.items[0];
    assert_eq!(item.id, "p1");
    assert_eq!(item.full_name, "John Smith");
    assert_eq!(item.team_id.as_deref(), Some("t1"));
    assert_eq!(item.team_name.as_deref(), Some("Alpha FC"));
    assert_eq!(item.ovr, Some(65));
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
    let game = make_game(
        vec![make_team("t1", "X")],
        vec![spec1.build(), spec2.build()],
    );

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
    let desc_ids: Vec<String> = query_page(&game, &desc)
        .items
        .into_iter()
        .map(|p| p.id)
        .collect();
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
    // Named so the name tie-break would put the low one first: only the rating can order them.
    let mut low = PlayerSpec::new("lo", "Alpha Sub", Some("t1"));
    low.ovr = 55;
    let mut game = make_game(vec![make_team("t1", "X")], vec![low.build(), high.build()]);
    game.manager.team_id = Some("t1".to_string());

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
    let game = make_game(
        vec![make_team("t1", "X")],
        vec![cheap.build(), pricey.build()],
    );

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
        spec.ovr = 90 - i as u8;
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

#[test]
fn projection_carries_the_authored_photo_from_a_package() {
    // A photo shipped in an `.ofm` reaches the player as a package-qualified
    // `media.face`. The players table is the one avatar surface fed by this
    // projection rather than by the full player, so dropping the field here
    // shows every packaged player a placeholder.
    let mut player = PlayerSpec::new("p1", "Pele", Some("t1")).build();
    player.media.face = Some("brazil-1962/assets/images/pele.png".to_string());
    let game = make_game(vec![make_team("t1", "Santos")], vec![player]);

    let page = query_page(&game, &baseline_query());

    assert_eq!(
        page.items[0].media.face.as_deref(),
        Some("brazil-1962/assets/images/pele.png"),
    );
}

#[test]
fn serialized_photo_matches_the_shape_the_frontend_parses() {
    // `PlayerSummary` in playersService.ts is a hand-written mirror of this
    // struct, so the wire key names are the contract. A rename here would show
    // up as placeholder avatars, not as a build failure.
    let mut player = PlayerSpec::new("p1", "Pele", Some("t1")).build();
    player.media.face = Some("brazil-1962/assets/images/pele.png".to_string());
    let game = make_game(vec![make_team("t1", "Santos")], vec![player]);

    let page = query_page(&game, &baseline_query());
    let json = serde_json::to_value(&page.items[0]).expect("summary serializes");

    assert_eq!(
        json["media"]["face"],
        serde_json::json!("brazil-1962/assets/images/pele.png"),
    );
}

#[test]
fn projection_leaves_the_photo_empty_for_a_generated_player() {
    let game = make_game(
        vec![make_team("t1", "Santos")],
        vec![PlayerSpec::new("p1", "Nobody", Some("t1")).build()],
    );

    let page = query_page(&game, &baseline_query());

    assert_eq!(page.items[0].media.face, None);
}

// ---------------------------------------------------------------------------
// Filters added for the one shared player list (#570): ranges, budget, scope, exact positions
// ---------------------------------------------------------------------------

/// A game where the user manages "mine", with `mine_cash` / `mine_budget` to spend. "mine" and
/// "other" share a division, so the club knows other's players' ratings; "faraway" plays in
/// another, so it knows nothing private about them.
fn game_managing_mine(mine_cash: i64, mine_budget: i64, players: Vec<Player>) -> Game {
    let mut mine = make_team("mine", "Mine FC");
    mine.finance = mine_cash;
    mine.transfer_budget = mine_budget;
    let mut game = make_game(
        vec![
            mine,
            make_team("other", "Other FC"),
            make_team("faraway", "Faraway FC"),
        ],
        players,
    );
    game.manager.team_id = Some("mine".to_string());
    let clubs = |ids: &[&str]| ids.iter().map(|id| id.to_string()).collect::<Vec<_>>();
    game.competitions = vec![
        League::new(
            "division-one".to_string(),
            "Division One".to_string(),
            2026,
            &clubs(&["mine", "other"]),
        ),
        League::new(
            "division-two".to_string(),
            "Division Two".to_string(),
            2026,
            &clubs(&["faraway"]),
        ),
    ];
    game
}

fn ids(game: &Game, query: &PlayersPageQuery) -> Vec<String> {
    let mut found: Vec<String> = query_page(game, query)
        .items
        .into_iter()
        .map(|p| p.id)
        .collect();
    found.sort();
    found
}

/// Given players born on either side of a birthday that falls just after the game date
/// (1 July 2026), when the age range starts at 26, then only the one who has turned 26 passes:
/// the age is the age on the game's date, not the year of birth.
#[test]
fn the_age_range_counts_whole_years_on_the_game_date() {
    let mut turned = PlayerSpec::new("turned", "Has Turned", Some("other"));
    turned.dob = "2000-06-30";
    let mut not_yet = PlayerSpec::new("not_yet", "Not Yet", Some("other"));
    not_yet.dob = "2000-07-02";
    let game = game_managing_mine(0, 0, vec![turned.build(), not_yet.build()]);
    let mut q = baseline_query();
    q.age_min = Some(26);

    assert_eq!(ids(&game, &q), vec!["turned"]);

    q.age_min = None;
    q.age_max = Some(25);
    assert_eq!(ids(&game, &q), vec!["not_yet"]);
}

/// Given players of different overall rating, when an ovr range is set, then both ends are
/// inclusive.
#[test]
fn the_ovr_range_is_inclusive() {
    let game = game_managing_mine(
        0,
        0,
        [60u8, 70, 80]
            .iter()
            .map(|ovr| {
                let id = format!("p{ovr}");
                let mut spec = PlayerSpec::new(&id, &id, Some("other"));
                spec.ovr = *ovr;
                spec.build()
            })
            .collect(),
    );
    let mut q = baseline_query();
    q.ovr_min = Some(70);
    q.ovr_max = Some(80);

    assert_eq!(ids(&game, &q), vec!["p70", "p80"]);
}

/// Given players of different market value, when a value range is set, then both ends are
/// inclusive.
#[test]
fn the_value_range_is_inclusive() {
    let game = game_managing_mine(
        0,
        0,
        [1_000_000u64, 2_000_000, 3_000_000]
            .iter()
            .map(|value| {
                let id = format!("v{value}");
                let mut spec = PlayerSpec::new(&id, &id, Some("other"));
                spec.market_value = *value;
                spec.build()
            })
            .collect(),
    );
    let mut q = baseline_query();
    q.value_min = Some(2_000_000);
    q.value_max = Some(2_000_000);

    assert_eq!(ids(&game, &q), vec!["v2000000"]);
}

/// Given a club with 3M cash but a 5M transfer budget, when "in budget" is on, then a 2M player
/// passes and a 4M one does not: the fee must fit both, as `can_afford_fee` decides everywhere.
#[test]
fn in_budget_means_the_fee_fits_both_cash_and_transfer_budget() {
    let mut cheap = PlayerSpec::new("cheap", "Cheap", Some("other"));
    cheap.market_value = 2_000_000;
    let mut dear = PlayerSpec::new("dear", "Dear", Some("other"));
    dear.market_value = 4_000_000;
    let game = game_managing_mine(3_000_000, 5_000_000, vec![cheap.build(), dear.build()]);
    let mut q = baseline_query();
    q.in_budget = true;

    assert_eq!(ids(&game, &q), vec!["cheap"]);
}

/// Given a free agent and a loan-only player worth more than the club can pay, and a player listed
/// for both loan and transfer at the same price, when "in budget" is on, then the first two still
/// pass (neither is bought for a fee) and the third does not: he can be bought, so he is held to
/// the club's means.
#[test]
fn free_agents_and_loan_only_players_skip_the_affordability_check() {
    let mut free_agent = PlayerSpec::new("free", "Free", None);
    free_agent.market_value = 9_000_000;
    let mut loan_only = PlayerSpec::new("loan", "Loan", Some("other"));
    loan_only.market_value = 9_000_000;
    loan_only.loan_listed = true;
    let mut dear = PlayerSpec::new("dear", "Dear", Some("other"));
    dear.market_value = 9_000_000;
    dear.transfer_listed = true;
    let mut both = PlayerSpec::new("both", "Both", Some("other"));
    both.market_value = 9_000_000;
    both.transfer_listed = true;
    both.loan_listed = true;
    let game = game_managing_mine(
        1_000_000,
        1_000_000,
        vec![
            free_agent.build(),
            loan_only.build(),
            dear.build(),
            both.build(),
        ],
    );
    let mut q = baseline_query();
    q.in_budget = true;

    assert_eq!(ids(&game, &q), vec!["free", "loan"]);
}

/// Given players the user's club could and could not approach, when the market scope is on, then
/// the user's own player, a player out on loan and one with an agreed move are left out, while
/// free agents and ordinary players stay.
#[test]
fn the_market_scope_leaves_out_own_loaned_and_already_agreed_players() {
    use domain::player::{ActiveLoan, TransferOffer, TransferOfferStatus};

    let own = PlayerSpec::new("own", "Own", Some("mine")).build();
    let mut on_loan = PlayerSpec::new("on_loan", "On Loan", Some("other")).build();
    on_loan.active_loan = Some(ActiveLoan {
        parent_team_id: "other".to_string(),
        loan_team_id: "third".to_string(),
        start_date: "2026-01-01".to_string(),
        end_date: "2026-12-31".to_string(),
        wage_contribution_pct: 50,
        buy_option_fee: None,
        loan_start_minutes: 0,
        loan_start_appearances: 0,
        development_reported_minutes: 0,
        development_reported_appearances: 0,
    });
    let mut agreed = PlayerSpec::new("agreed", "Agreed", Some("other")).build();
    agreed.transfer_offers.push(TransferOffer {
        id: "o1".to_string(),
        from_team_id: "someone".to_string(),
        fee: 1,
        wage_offered: 1,
        last_manager_fee: None,
        negotiation_round: 1,
        suggested_counter_fee: None,
        status: TransferOfferStatus::PendingRegistration,
        date: "2026-06-01".to_string(),
        registration_date: None,
        registration_failure_reason: None,
        closed_on: None,
    });
    let free = PlayerSpec::new("free", "Free", None).build();
    let ordinary = PlayerSpec::new("ordinary", "Ordinary", Some("other")).build();
    let game = game_managing_mine(0, 0, vec![own, on_loan, agreed, free, ordinary]);
    let mut q = baseline_query();
    q.scope = PlayerScope::Market;

    assert_eq!(ids(&game, &q), vec!["free", "ordinary"]);
}

/// Given the user's own player and one a scout is already watching, when the scoutable scope is
/// on, then the own player is left out and the watched one stays listed: the screen marks him as
/// already being scouted, and `send_scout` refuses a second scout.
#[test]
fn the_scoutable_scope_leaves_out_own_players_but_keeps_those_already_scouted() {
    let own = PlayerSpec::new("own", "Own", Some("mine")).build();
    let watched = PlayerSpec::new("watched", "Watched", Some("other")).build();
    let open = PlayerSpec::new("open", "Open", Some("other")).build();
    let mut game = game_managing_mine(0, 0, vec![own, watched, open]);
    game.scouting_assignments.push(ScoutingAssignment {
        id: "a1".to_string(),
        scout_id: "s1".to_string(),
        player_id: "watched".to_string(),
        days_remaining: 3,
    });
    let mut q = baseline_query();
    q.scope = PlayerScope::Scoutable;

    assert_eq!(ids(&game, &q), vec!["open", "watched"]);
}

fn transfer_offer_from(from_team_id: &str) -> domain::player::TransferOffer {
    use domain::player::{TransferOffer, TransferOfferStatus};
    TransferOffer {
        id: format!("offer-from-{from_team_id}"),
        from_team_id: from_team_id.to_string(),
        fee: 1,
        wage_offered: 1,
        last_manager_fee: None,
        negotiation_round: 1,
        suggested_counter_fee: None,
        status: TransferOfferStatus::Pending,
        date: "2026-06-01".to_string(),
        registration_date: None,
        registration_failure_reason: None,
        closed_on: None,
    }
}

/// Given the user's own players, one listed for transfer, one listed for loan, one unlisted, one
/// listed but already agreed to move, and another club's listed player, when the user-listed scope
/// is on, then only the two listed and still-free own players are returned.
#[test]
fn the_user_listed_scope_is_the_own_players_put_up_for_transfer_or_loan() {
    let mut for_sale = PlayerSpec::new("for_sale", "For Sale", Some("mine"));
    for_sale.transfer_listed = true;
    let mut for_loan = PlayerSpec::new("for_loan", "For Loan", Some("mine"));
    for_loan.loan_listed = true;
    let unlisted = PlayerSpec::new("unlisted", "Unlisted", Some("mine")).build();
    let mut agreed = PlayerSpec::new("agreed", "Agreed", Some("mine"));
    agreed.transfer_listed = true;
    let mut agreed = agreed.build();
    let mut offer = transfer_offer_from("other");
    offer.status = domain::player::TransferOfferStatus::PendingRegistration;
    agreed.transfer_offers.push(offer);
    let mut elsewhere = PlayerSpec::new("elsewhere", "Elsewhere", Some("other"));
    elsewhere.transfer_listed = true;
    let game = game_managing_mine(
        0,
        0,
        vec![
            for_sale.build(),
            for_loan.build(),
            unlisted,
            agreed,
            elsewhere.build(),
        ],
    );
    let mut q = baseline_query();
    q.scope = PlayerScope::UserListed;

    assert_eq!(ids(&game, &q), vec!["for_loan", "for_sale"]);
}

/// Given a player listed for transfer who is out on loan, when the user-listed scope is on, then
/// he is not offered for sale again.
#[test]
fn the_user_listed_scope_leaves_out_a_player_out_on_loan() {
    let mut spec = PlayerSpec::new("away", "Away", Some("mine"));
    spec.transfer_listed = true;
    let mut away = spec.build();
    away.active_loan = Some(domain::player::ActiveLoan {
        parent_team_id: "mine".to_string(),
        loan_team_id: "other".to_string(),
        start_date: "2026-01-01".to_string(),
        end_date: "2026-12-31".to_string(),
        wage_contribution_pct: 50,
        buy_option_fee: None,
        loan_start_minutes: 0,
        loan_start_appearances: 0,
        development_reported_minutes: 0,
        development_reported_appearances: 0,
    });
    let game = game_managing_mine(0, 0, vec![away]);
    let mut q = baseline_query();
    q.scope = PlayerScope::UserListed;

    assert!(ids(&game, &q).is_empty());
}

/// Given an own player with an offer on him, another club's player the user has bid for, another
/// club's player only a third club has bid for, and a player with no offers, when the with-offers
/// scope is on, then the first two are returned.
#[test]
fn the_with_offers_scope_is_the_players_the_users_club_has_dealings_over() {
    let mut own = PlayerSpec::new("own", "Own", Some("mine")).build();
    own.transfer_offers.push(transfer_offer_from("other"));
    let mut bid_for = PlayerSpec::new("bid_for", "Bid For", Some("other")).build();
    bid_for.transfer_offers.push(transfer_offer_from("mine"));
    let mut third_party = PlayerSpec::new("third_party", "Third Party", Some("other")).build();
    third_party
        .transfer_offers
        .push(transfer_offer_from("third"));
    let quiet = PlayerSpec::new("quiet", "Quiet", Some("other")).build();
    let game = game_managing_mine(0, 0, vec![own, bid_for, third_party, quiet]);
    let mut q = baseline_query();
    q.scope = PlayerScope::WithOffers;

    assert_eq!(ids(&game, &q), vec!["bid_for", "own"]);
}

/// Given loan offers, when the with-offers scope is on, then a loan the user's club made, or one
/// for a player the user's club owns, counts; a loan between two other clubs does not.
#[test]
fn the_with_offers_scope_counts_loan_offers_by_either_end() {
    use domain::player::{LoanOffer, LoanOfferStatus};
    fn loan(from: &str, parent: &str) -> LoanOffer {
        LoanOffer {
            id: format!("loan-{from}-{parent}"),
            from_team_id: from.to_string(),
            parent_team_id: parent.to_string(),
            start_date: "2026-07-01".to_string(),
            end_date: "2027-06-30".to_string(),
            wage_contribution_pct: 50,
            buy_option_fee: None,
            last_manager_wage_contribution_pct: None,
            last_manager_end_date: None,
            last_manager_buy_option_fee: None,
            negotiation_round: 1,
            suggested_wage_contribution_pct: None,
            suggested_end_date: None,
            suggested_buy_option_fee: None,
            status: LoanOfferStatus::Pending,
            date: "2026-06-01".to_string(),
            closed_on: None,
        }
    }
    let mut made_by_user = PlayerSpec::new("made_by_user", "Made By User", Some("other")).build();
    made_by_user.loan_offers.push(loan("mine", "other"));
    let mut users_player = PlayerSpec::new("users_player", "Users Player", Some("mine")).build();
    users_player.loan_offers.push(loan("other", "mine"));
    let mut elsewhere = PlayerSpec::new("elsewhere", "Elsewhere", Some("other")).build();
    elsewhere.loan_offers.push(loan("third", "other"));
    let game = game_managing_mine(0, 0, vec![made_by_user, users_player, elsewhere]);
    let mut q = baseline_query();
    q.scope = PlayerScope::WithOffers;

    assert_eq!(ids(&game, &q), vec!["made_by_user", "users_player"]);
}

/// Given a game with no user club, when either club-bound scope is on, then nothing matches:
/// there is no club to have a list or offers.
#[test]
fn the_club_bound_scopes_are_empty_without_a_user_club() {
    let mut listed = PlayerSpec::new("listed", "Listed", Some("mine"));
    listed.transfer_listed = true;
    let mut offered = PlayerSpec::new("offered", "Offered", Some("other")).build();
    offered.transfer_offers.push(transfer_offer_from("mine"));
    let mut game = game_managing_mine(0, 0, vec![listed.build(), offered]);
    game.manager.team_id = None;

    for scope in [PlayerScope::UserListed, PlayerScope::WithOffers] {
        let mut q = baseline_query();
        q.scope = scope;
        assert!(ids(&game, &q).is_empty());
    }
}

/// Given one player listed for transfer, one for loan, one for both, a free agent and one on no
/// list, when the page is asked for transfer-listed players, then the page holds the two transfer
/// ones while the counts say how many players each status would have shown.
#[test]
fn the_page_counts_every_status_whichever_one_it_was_asked_for() {
    let mut transfer = PlayerSpec::new("transfer", "Transfer", Some("other"));
    transfer.transfer_listed = true;
    let mut loan = PlayerSpec::new("loan", "Loan", Some("other"));
    loan.loan_listed = true;
    let mut both = PlayerSpec::new("both", "Both", Some("other"));
    both.transfer_listed = true;
    both.loan_listed = true;
    let free = PlayerSpec::new("free", "Free", None).build();
    let unlisted = PlayerSpec::new("unlisted", "Unlisted", Some("other")).build();
    let game = game_managing_mine(
        0,
        0,
        vec![transfer.build(), loan.build(), both.build(), free, unlisted],
    );
    let mut q = baseline_query();
    q.status = PlayerStatusFilter::Transfer;

    let page = query_page(&game, &q);

    assert_eq!(page.total, 2);
    assert_eq!(
        page.status_counts,
        StatusCounts {
            all: 5,
            transfer: 2,
            loan: 2,
            free_agent: 1,
            available: 4,
        }
    );
}

/// Given the same players, when a search narrows the list, then the counts follow the search: a
/// status chip says how many players it would show with every other filter as it is.
#[test]
fn the_status_counts_follow_the_other_filters() {
    let mut wanted = PlayerSpec::new("wanted", "Wanted Winger", Some("other"));
    wanted.transfer_listed = true;
    let mut other = PlayerSpec::new("other", "Someone Else", Some("other"));
    other.transfer_listed = true;
    let free = PlayerSpec::new("free", "Wanted Free", None).build();
    let game = game_managing_mine(0, 0, vec![wanted.build(), other.build(), free]);
    let mut q = baseline_query();
    q.search = Some("wanted".to_string());

    let counts = query_page(&game, &q).status_counts;

    assert_eq!(
        counts,
        StatusCounts {
            all: 2,
            transfer: 1,
            loan: 0,
            free_agent: 1,
            available: 2,
        }
    );
}

/// Given players at two clubs, when the search is a club's name, then that club's players are
/// found: the scouting search has always matched the club, and every list now shares the rule.
#[test]
fn search_matches_the_name_of_the_players_club() {
    let game = game_managing_mine(
        0,
        0,
        vec![
            PlayerSpec::new("at_other", "Anyone", Some("other")).build(),
            PlayerSpec::new("at_mine", "Someone", Some("mine")).build(),
            PlayerSpec::new("free", "Nobody", None).build(),
        ],
    );
    let mut q = baseline_query();
    q.search = Some("other fc".to_string());

    assert_eq!(ids(&game, &q), vec!["at_other"]);
}

/// Given a retired player among the others, when the scoutable scope is on, then he is left out:
/// there is nothing left to watch.
#[test]
fn the_scoutable_scope_leaves_out_retired_players() {
    let mut retired = PlayerSpec::new("retired", "Retired", Some("other"));
    retired.retired = true;
    let game = game_managing_mine(
        0,
        0,
        vec![
            retired.build(),
            PlayerSpec::new("active", "Active", Some("other")).build(),
        ],
    );
    let mut q = baseline_query();
    q.scope = PlayerScope::Scoutable;

    assert_eq!(ids(&game, &q), vec!["active"]);
}

fn with_position(id: &str, position: Position) -> Player {
    let mut spec = PlayerSpec::new(id, id, Some("other"));
    spec.position = position;
    spec.build()
}

/// Given centre-backs, left-backs and a striker, when one exact position is asked for, then only
/// that position matches, not the rest of its group.
#[test]
fn an_exact_position_matches_only_that_position() {
    let game = game_managing_mine(
        0,
        0,
        vec![
            with_position("cb", Position::CenterBack),
            with_position("lb", Position::LeftBack),
            with_position("st", Position::Striker),
        ],
    );
    let mut q = baseline_query();
    q.positions = vec![PositionFilter::Exact(Position::CenterBack)];

    assert_eq!(ids(&game, &q), vec!["cb"]);
}

/// Given the same players, when the defender group is asked for, then every kind of defender
/// matches and the striker does not.
#[test]
fn a_group_shorthand_matches_every_position_in_the_group() {
    let game = game_managing_mine(
        0,
        0,
        vec![
            with_position("cb", Position::CenterBack),
            with_position("lb", Position::LeftBack),
            with_position("st", Position::Striker),
        ],
    );
    let mut q = baseline_query();
    q.positions = vec![PositionFilter::Group(Position::Defender)];

    assert_eq!(ids(&game, &q), vec!["cb", "lb"]);
}

/// Given an exact position and a group together, when both are asked for, then a player matching
/// either passes.
#[test]
fn several_position_tests_are_combined_with_or() {
    let game = game_managing_mine(
        0,
        0,
        vec![
            with_position("cb", Position::CenterBack),
            with_position("lb", Position::LeftBack),
            with_position("st", Position::Striker),
            with_position("gk", Position::Goalkeeper),
        ],
    );
    let mut q = baseline_query();
    q.positions = vec![
        PositionFilter::Exact(Position::CenterBack),
        PositionFilter::Group(Position::Forward),
    ];

    assert_eq!(ids(&game, &q), vec!["cb", "st"]);
}

/// Given the JSON the frontend sends, when a position test is read, then `{"group": ...}` and
/// `{"exact": ...}` are the two shapes, and a query without the new fields still reads.
#[test]
fn the_new_query_fields_read_from_json_and_default_when_absent() {
    let query: PlayersPageQuery = serde_json::from_value(serde_json::json!({
        "search": null, "position": null, "team_id": null, "status": "all",
        "sort_key": "ovr", "sort_asc": false, "page": 1, "page_size": 10,
        "positions": [{"group": "Defender"}, {"exact": "Striker"}],
        "age_min": 18, "in_budget": true, "scope": "market"
    }))
    .expect("a query with the new fields");
    assert_eq!(
        query.positions,
        vec![
            PositionFilter::Group(Position::Defender),
            PositionFilter::Exact(Position::Striker)
        ]
    );
    assert_eq!(query.age_min, Some(18));
    assert!(query.in_budget);
    assert_eq!(query.scope, PlayerScope::Market);

    let legacy: PlayersPageQuery = serde_json::from_value(serde_json::json!({
        "search": null, "position": null, "team_id": null, "status": "all",
        "sort_key": "ovr", "sort_asc": false, "page": 1, "page_size": 10
    }))
    .expect("a query from before the new fields");
    assert!(legacy.positions.is_empty() && !legacy.in_budget);
    assert_eq!(legacy.scope, PlayerScope::All);
}

/// Given the club's own player, born 1 January 2000, on a 5,000 weekly wage, when summarised on
/// 1 July 2026, then the row carries his age on that date and his exact weekly wage.
#[test]
fn the_summary_carries_age_on_the_game_date_and_the_weekly_wage() {
    let mut player = PlayerSpec::new("p1", "John Smith", Some("mine")).build();
    player.stage_wage(5_000);
    let game = game_managing_mine(0, 0, vec![player]);

    let item = &query_page(&game, &baseline_query()).items[0];

    assert_eq!(item.age, 26);
    assert_eq!(item.wage, VisibleWage::Exact { weekly: 5_000 });
}

// ---------------------------------------------------------------------------
// The list shows what the club may see (player_knowledge; maintainer rulings of 29 Oct)
// ---------------------------------------------------------------------------

/// A player at `club` rated `ovr` on 5,000 a week.
fn rated_at(id: &str, club: &str, ovr: u8) -> Player {
    let mut spec = PlayerSpec::new(id, id, Some(club));
    spec.ovr = ovr;
    let mut player = spec.build();
    player.stage_wage(5_000);
    player
}

/// Given a player at a club in another division, never scouted, rated 90 on 5,000 a week
/// When the list shows him
/// Then his row carries no rating and no wage: nothing beyond what is public
#[test]
fn an_unknown_players_row_carries_no_rating_or_wage() {
    let game = game_managing_mine(0, 0, vec![rated_at("far", "faraway", 90)]);

    let item = &query_page(&game, &baseline_query()).items[0];

    assert_eq!(item.ovr, None);
    assert_eq!(item.wage, VisibleWage::Hidden);
}

/// Given a player at a club in the manager's own division, rated 80 on 5,000 a week
/// When the list shows him
/// Then his row carries his rating, as the staff see him every season, but not his wage
#[test]
fn a_division_rivals_row_carries_his_rating_but_not_his_wage() {
    let game = game_managing_mine(0, 0, vec![rated_at("rival", "other", 80)]);

    let item = &query_page(&game, &baseline_query()).items[0];

    assert_eq!(item.ovr, Some(80));
    assert_eq!(item.wage, VisibleWage::Hidden);
}

/// Given division rivals rated 80 and 70, and an unknown player rated 90
/// When the list is sorted by rating, highest first and then lowest first
/// Then the known ratings run in that direction and the unknown player comes last both times,
///      never placed by the 90 the club cannot see
#[test]
fn a_rating_sort_puts_an_unknown_player_after_every_known_one_both_ways() {
    let game = game_managing_mine(
        0,
        0,
        vec![
            rated_at("seventy", "other", 70),
            rated_at("unknown", "faraway", 90),
            rated_at("eighty", "other", 80),
        ],
    );
    let order = |ascending| {
        let mut query = baseline_query();
        query.sort_key = PlayerSortKey::Ovr;
        query.sort_asc = ascending;
        query_page(&game, &query)
            .items
            .into_iter()
            .map(|item| item.id)
            .collect::<Vec<_>>()
    };

    assert_eq!(order(false), vec!["eighty", "seventy", "unknown"]);
    assert_eq!(order(true), vec!["seventy", "eighty", "unknown"]);
}

/// Given a division rival rated 80 and an unknown player rated 90
/// When the list is filtered to ratings of 1 and up, or of 99 and down
/// Then only the rival passes: a rating bound leaves out a player whose rating the club cannot see,
///      so the filter never tells it; with no bound set, both are listed
#[test]
fn a_rating_bound_leaves_out_a_player_whose_rating_is_unknown() {
    let game = game_managing_mine(
        0,
        0,
        vec![
            rated_at("rival", "other", 80),
            rated_at("unknown", "faraway", 90),
        ],
    );
    let mut query = baseline_query();

    assert_eq!(ids(&game, &query), vec!["rival", "unknown"]);

    query.ovr_min = Some(1);
    assert_eq!(ids(&game, &query), vec!["rival"]);

    query.ovr_min = None;
    query.ovr_max = Some(99);
    assert_eq!(ids(&game, &query), vec!["rival"]);
}

/// Given two players born in the same year, one in January and one in November, when sorted by
/// age ascending, then the younger (November) comes first: the order uses the whole birth date.
#[test]
fn sorting_by_age_uses_the_whole_birth_date_not_the_year() {
    let mut january = PlayerSpec::new("jan", "Jan", Some("other"));
    january.dob = "2000-01-15";
    let mut november = PlayerSpec::new("nov", "Nov", Some("other"));
    november.dob = "2000-11-15";
    let game = game_managing_mine(0, 0, vec![january.build(), november.build()]);
    let mut q = baseline_query();
    q.sort_key = PlayerSortKey::Age;
    q.sort_asc = true;

    let order: Vec<String> = query_page(&game, &q)
        .items
        .into_iter()
        .map(|p| p.id)
        .collect();

    assert_eq!(order, vec!["nov", "jan"]);
}

/// Given players tied on the sort key, when the list is sorted either way, then they keep one
/// order by name and then id, so a page boundary never reshuffles them.
#[test]
fn ties_are_broken_by_name_then_id_in_either_direction() {
    let game = game_managing_mine(
        0,
        0,
        vec![
            PlayerSpec::new("b", "Same Name", Some("other")).build(),
            PlayerSpec::new("a", "Same Name", Some("other")).build(),
            PlayerSpec::new("c", "Another", Some("other")).build(),
        ],
    );
    for sort_asc in [true, false] {
        let mut q = baseline_query();
        q.sort_asc = sort_asc;
        let order: Vec<String> = query_page(&game, &q)
            .items
            .into_iter()
            .map(|p| p.id)
            .collect();
        assert_eq!(order, vec!["c", "a", "b"], "sort_asc = {sort_asc}");
    }
}

/// Given a budget, a scope, an age range and a position together, when all are set, then a player
/// must satisfy every one of them.
#[test]
fn filters_combine_with_and() {
    let mut fits = PlayerSpec::new("fits", "Fits", Some("other"));
    fits.dob = "2004-03-01";
    fits.position = Position::CenterBack;
    fits.market_value = 1_000_000;
    fits.transfer_listed = true;
    let mut too_old = PlayerSpec::new("old", "Old", Some("other"));
    too_old.dob = "1990-03-01";
    too_old.position = Position::CenterBack;
    too_old.market_value = 1_000_000;
    let mut too_dear = PlayerSpec::new("dear", "Dear", Some("other"));
    too_dear.dob = "2004-03-01";
    too_dear.position = Position::CenterBack;
    too_dear.market_value = 9_000_000;
    too_dear.transfer_listed = true;
    let mut own = PlayerSpec::new("own", "Own", Some("mine"));
    own.dob = "2004-03-01";
    own.position = Position::CenterBack;
    own.market_value = 1_000_000;
    let game = game_managing_mine(
        2_000_000,
        2_000_000,
        vec![fits.build(), too_old.build(), too_dear.build(), own.build()],
    );
    let mut q = baseline_query();
    q.age_max = Some(25);
    q.in_budget = true;
    q.scope = PlayerScope::Market;
    q.positions = vec![PositionFilter::Exact(Position::CenterBack)];

    assert_eq!(ids(&game, &q), vec!["fits"]);
}

/// Given players whose names carry accents, a slashed letter and Cyrillic, when the search is typed
///       in the other case, then each is found: case is ignored for every letter, not just ASCII.
#[test]
fn search_ignores_case_for_every_letter_not_just_ascii() {
    let game = game_managing_mine(
        0,
        0,
        vec![
            PlayerSpec::new("alvarez", "Julián Álvarez", Some("other")).build(),
            PlayerSpec::new("odegaard", "Martin Ødegaard", Some("other")).build(),
            PlayerSpec::new("ivan", "Иван Петров", Some("other")).build(),
            PlayerSpec::new("plain", "John Smith", Some("other")).build(),
        ],
    );
    let found = |text: &str| {
        let mut q = baseline_query();
        q.search = Some(text.to_string());
        ids(&game, &q)
    };

    assert_eq!(found("álvarez"), vec!["alvarez"]);
    assert_eq!(found("ÁLVAREZ"), vec!["alvarez"]);
    assert_eq!(found("ØDEGAARD"), vec!["odegaard"]);
    assert_eq!(found("ødegaard"), vec!["odegaard"]);
    assert_eq!(found("иван"), vec!["ivan"]);
    assert_eq!(found("ПЕТРОВ"), vec!["ivan"]);
    assert_eq!(found("SMITH"), vec!["plain"]);
    assert_eq!(found("jOhN"), vec!["plain"]);
}

/// Given players with and without a club, listed and not, when the free-agent status is asked for,
/// then only the players without a club are returned.
#[test]
fn the_free_agent_status_lists_only_players_without_a_club() {
    let mut listed = PlayerSpec::new("listed", "Listed", Some("other"));
    listed.transfer_listed = true;
    let game = game_managing_mine(
        0,
        0,
        vec![
            PlayerSpec::new("free", "Free", None).build(),
            listed.build(),
            PlayerSpec::new("kept", "Kept", Some("other")).build(),
        ],
    );
    let mut q = baseline_query();
    q.status = PlayerStatusFilter::FreeAgent;

    assert_eq!(ids(&game, &q), vec!["free"]);
}

/// Given players listed for transfer, listed for loan, unlisted and without a club, when the
/// available status is asked for, then everyone a club can approach is returned and the unlisted
/// player under contract is not.
#[test]
fn the_available_status_lists_listed_and_free_players() {
    let mut for_sale = PlayerSpec::new("sale", "Sale", Some("other"));
    for_sale.transfer_listed = true;
    let mut for_loan = PlayerSpec::new("loan", "Loan", Some("other"));
    for_loan.loan_listed = true;
    let game = game_managing_mine(
        0,
        0,
        vec![
            for_sale.build(),
            for_loan.build(),
            PlayerSpec::new("free", "Free", None).build(),
            PlayerSpec::new("kept", "Kept", Some("other")).build(),
        ],
    );
    let mut q = baseline_query();
    q.status = PlayerStatusFilter::Available;

    assert_eq!(ids(&game, &q), vec!["free", "loan", "sale"]);
}

/// Given a retired free agent among the players, when the market scope is on, then he is left
/// out: nobody can sign him. The world directory (scope all) still lists him.
#[test]
fn the_market_scope_leaves_out_retired_players() {
    let retired = {
        let mut spec = PlayerSpec::new("retired", "Retired", None);
        spec.retired = true;
        spec.build()
    };
    let free = PlayerSpec::new("free", "Free", None).build();
    let game = game_managing_mine(0, 0, vec![retired, free]);
    let mut q = baseline_query();
    q.scope = PlayerScope::Market;

    assert_eq!(ids(&game, &q), vec!["free"]);

    q.scope = PlayerScope::All;
    assert_eq!(ids(&game, &q), vec!["free", "retired"]);
}
