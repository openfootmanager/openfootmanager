use chrono::{TimeZone, Utc};
use domain::manager::Manager;
use domain::player::{Player, PlayerAttributes, Position};
use domain::staff::{Staff, StaffAttributes, StaffRole};
use domain::team::Team;
use ofm_core::clock::GameClock;
use ofm_core::game::{Game, ScoutingAssignment, YouthScoutingAssignment};
use ofm_core::player_tier::PlayerTier;
use ofm_core::slices::staff::query_staff;
use ofm_core::staff_rating::staff_rating;
use ofm_core::staff_tier::staff_tier;

fn default_player_attrs() -> PlayerAttributes {
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

fn make_staff(id: &str, team_id: Option<&str>, role: StaffRole) -> Staff {
    let attrs = StaffAttributes {
        coaching: 70,
        judging_ability: 60,
        judging_potential: 60,
        physiotherapy: 40,
    };
    let mut s = Staff::new(
        id.to_string(),
        "First".to_string(),
        "Last".to_string(),
        "1980-01-01".to_string(),
        role,
        attrs,
    );
    s.team_id = team_id.map(String::from);
    s
}

fn make_scouting_assignment(id: &str, scout_id: &str, player_id: &str) -> ScoutingAssignment {
    ScoutingAssignment {
        id: id.to_string(),
        scout_id: scout_id.to_string(),
        player_id: player_id.to_string(),
        days_remaining: 14,
    }
}

fn make_youth_assignment(id: &str, scout_id: &str) -> YouthScoutingAssignment {
    YouthScoutingAssignment {
        id: id.to_string(),
        scout_id: scout_id.to_string(),
        region: Default::default(),
        objective: Default::default(),
        target_position: None,
        days_remaining: 30,
    }
}

fn make_game(staff: Vec<Staff>) -> Game {
    let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap());
    let manager = Manager::new(
        "m".to_string(),
        "M".to_string(),
        "Gr".to_string(),
        "1980-01-01".to_string(),
        "GB".to_string(),
    );
    let team = make_team("t1", "Alpha FC");
    let p = {
        let mut pl = Player::new(
            "p1".to_string(),
            "p1".to_string(),
            "Player 1".to_string(),
            "2000-01-01".to_string(),
            "GB".to_string(),
            Position::Midfielder,
            default_player_attrs(),
        );
        pl.team_id = Some("t1".to_string());
        pl
    };
    Game::new(clock, manager, vec![team], vec![p], staff, vec![])
}

#[test]
fn team_staff_returns_only_staff_for_the_requested_team() {
    let game = make_game(vec![
        make_staff("s1", Some("t1"), StaffRole::Coach),
        make_staff("s2", Some("t1"), StaffRole::Scout),
        make_staff("s3", Some("t2"), StaffRole::Coach),
        make_staff("fa", None, StaffRole::AssistantManager),
    ]);

    let slice = query_staff(&game, "t1");

    let ids: Vec<&str> = slice
        .team_staff
        .iter()
        .map(|s| s.staff.id.as_str())
        .collect();
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&"s1"));
    assert!(ids.contains(&"s2"));
    assert!(!ids.contains(&"s3"));
    assert!(!ids.contains(&"fa"));
}

#[test]
fn available_staff_returns_unattached_staff() {
    let game = make_game(vec![
        make_staff("s1", Some("t1"), StaffRole::Coach),
        make_staff("fa1", None, StaffRole::Scout),
        make_staff("fa2", None, StaffRole::Physio),
    ]);

    let slice = query_staff(&game, "t1");

    let ids: Vec<&str> = slice
        .available_staff
        .iter()
        .map(|s| s.staff.id.as_str())
        .collect();
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&"fa1"));
    assert!(ids.contains(&"fa2"));
    assert!(!ids.contains(&"s1"));
}

#[test]
fn scouting_assignments_returns_all_assignments() {
    let mut game = make_game(vec![make_staff("scout1", Some("t1"), StaffRole::Scout)]);
    game.scouting_assignments = vec![
        make_scouting_assignment("a1", "scout1", "p1"),
        make_scouting_assignment("a2", "scout1", "p2"),
    ];

    let slice = query_staff(&game, "t1");

    assert_eq!(slice.scouting_assignments.len(), 2);
    assert!(slice.scouting_assignments.iter().any(|a| a.id == "a1"));
    assert!(slice.scouting_assignments.iter().any(|a| a.id == "a2"));
}

#[test]
fn youth_scouting_assignments_returns_all_assignments() {
    let mut game = make_game(vec![make_staff("scout1", Some("t1"), StaffRole::Scout)]);
    game.youth_scouting_assignments = vec![make_youth_assignment("ya1", "scout1")];

    let slice = query_staff(&game, "t1");

    assert_eq!(slice.youth_scouting_assignments.len(), 1);
    assert_eq!(slice.youth_scouting_assignments[0].id, "ya1");
}

#[test]
fn staff_slice_matches_frontend_projections() {
    let game = make_game(vec![
        make_staff("s1", Some("t1"), StaffRole::Coach),
        make_staff("s2", Some("t2"), StaffRole::Scout),
        make_staff("fa", None, StaffRole::AssistantManager),
    ]);

    let slice = query_staff(&game, "t1");

    let expected_team: Vec<&str> = game
        .staff
        .iter()
        .filter(|s| s.team_id.as_deref() == Some("t1"))
        .map(|s| s.id.as_str())
        .collect();
    let expected_free: Vec<&str> = game
        .staff
        .iter()
        .filter(|s| s.team_id.is_none())
        .map(|s| s.id.as_str())
        .collect();

    let got_team: Vec<&str> = slice
        .team_staff
        .iter()
        .map(|s| s.staff.id.as_str())
        .collect();
    let got_free: Vec<&str> = slice
        .available_staff
        .iter()
        .map(|s| s.staff.id.as_str())
        .collect();

    assert_eq!(got_team, expected_team);
    assert_eq!(got_free, expected_free);
}

/// Given a coach, a scout and a physio at the club, and an assistant manager on the hiring market
/// When the Staff page's slice is built
/// Then every row, the club's and the market's, carries the rating the game computes for that
///      member's role, so the page shows the game's figure instead of a formula of its own.
#[test]
fn every_staff_row_carries_its_roles_rating() {
    let game = make_game(vec![
        make_staff("coach", Some("t1"), StaffRole::Coach),
        make_staff("scout", Some("t1"), StaffRole::Scout),
        make_staff("physio", Some("t1"), StaffRole::Physio),
        make_staff("free", None, StaffRole::AssistantManager),
    ]);

    let slice = query_staff(&game, "t1");

    for row in slice.team_staff.iter().chain(slice.available_staff.iter()) {
        assert_eq!(row.rating, staff_rating(&row.staff), "{}", row.staff.id);
    }
    let ratings: Vec<(&str, u8)> = slice
        .team_staff
        .iter()
        .chain(slice.available_staff.iter())
        .map(|row| (row.staff.id.as_str(), row.rating))
        .collect();
    // make_staff: coaching 70, judging 60 and 60, physiotherapy 40.
    assert_eq!(
        ratings,
        vec![("coach", 70), ("scout", 60), ("physio", 40), ("free", 64)]
    );
}

/// Given a club's coach rated 88 (his role's S), its assistant rated 64, a physio rated 40, and a
///       scout on the market rated 76 (his role's B)
/// When the Staff page's slice is built
/// Then every row carries the tier his rating has among his own role: S, the rest, the rest and B.
#[test]
fn every_staff_row_carries_its_rating_tier_among_its_role() {
    let mut coach = make_staff("coach", Some("t1"), StaffRole::Coach);
    coach.attributes.coaching = 88;
    let mut scout = make_staff("scout", None, StaffRole::Scout);
    scout.attributes.judging_ability = 76;
    scout.attributes.judging_potential = 76;
    let game = make_game(vec![
        coach,
        make_staff("assistant", Some("t1"), StaffRole::AssistantManager),
        make_staff("physio", Some("t1"), StaffRole::Physio),
        scout,
    ]);

    let slice = query_staff(&game, "t1");

    let tiers: Vec<(&str, u8, PlayerTier)> = slice
        .team_staff
        .iter()
        .chain(slice.available_staff.iter())
        .map(|row| (row.staff.id.as_str(), row.rating, row.tier))
        .collect();
    assert_eq!(
        tiers,
        vec![
            ("coach", 88, PlayerTier::S),
            ("assistant", 64, PlayerTier::Rest),
            ("physio", 40, PlayerTier::Rest),
            ("scout", 76, PlayerTier::B),
        ]
    );
    for row in slice.team_staff.iter().chain(slice.available_staff.iter()) {
        assert_eq!(
            row.tier,
            staff_tier(&row.staff.role, row.rating),
            "{}",
            row.staff.id
        );
    }
}

/// Given a staff row
/// When it is sent to the frontend
/// Then it is the staff member's own fields with `rating` beside them, not a nested object, so
///      the page's existing reading of a staff member still holds.
#[test]
fn a_staff_row_is_sent_flat_with_its_rating() {
    let game = make_game(vec![make_staff("coach", Some("t1"), StaffRole::Coach)]);
    let slice = query_staff(&game, "t1");

    let json = serde_json::to_value(&slice.team_staff[0]).expect("a staff row serialises");

    assert_eq!(json["id"], "coach");
    assert_eq!(json["role"], "Coach");
    assert_eq!(json["rating"], 70);
    assert!(json.get("staff").is_none());
}
