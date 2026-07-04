use chrono::{TimeZone, Utc};
use domain::league::{
    Berth, BerthRule, CompetitionScope, CompetitionType, Fixture, FixtureCompetition,
    FixtureStatus, KnockoutRoundState, League, MatchResult, StandingEntry,
};
use domain::manager::Manager;
use domain::player::{Player, PlayerAttributes, PlayerSeasonStats, Position};
use domain::team::{FinancialTransactionKind, Team};
use ofm_core::clock::GameClock;
use ofm_core::end_of_season::{
    berth_qualified_entrants, continental_qualified_entrants, expected_fixture_count,
    is_season_complete, process_end_of_season, resolve_continental_fields,
};
use ofm_core::game::{BoardObjective, Game, ObjectiveType};

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn make_team(id: &str, name: &str) -> Team {
    Team::new(
        id.to_string(),
        name.to_string(),
        name[..3].to_string(),
        "England".to_string(),
        "London".to_string(),
        "Stadium".to_string(),
        40_000,
    )
}

fn make_player(id: &str, name: &str, team_id: &str, pos: Position) -> Player {
    let attrs = PlayerAttributes {
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
        reflexes: 30,
        aerial: 60,
    };
    let mut p = Player::new(
        id.to_string(),
        name.to_string(),
        format!("Full {}", name),
        "1995-01-01".to_string(),
        "GB".to_string(),
        pos,
        attrs,
    );
    p.team_id = Some(team_id.to_string());
    p.morale = 70;
    p.condition = 90;
    p
}

fn make_completed_fixture(id: &str, home: &str, away: &str, hg: u8, ag: u8) -> Fixture {
    Fixture {
        id: id.to_string(),
        matchday: 1,
        date: "2025-06-01".to_string(),
        home_team_id: home.to_string(),
        away_team_id: away.to_string(),
        competition: FixtureCompetition::League,
        status: FixtureStatus::Completed,
        result: Some(MatchResult {
            home_goals: hg,
            away_goals: ag,
            home_scorers: vec![],
            away_scorers: vec![],
            report: None,
            home_penalties: None,
            away_penalties: None,
        }),
        ..Default::default()
    }
}

fn make_standing(
    team_id: &str,
    won: u32,
    drawn: u32,
    lost: u32,
    gf: u32,
    ga: u32,
) -> StandingEntry {
    StandingEntry {
        team_id: team_id.to_string(),
        played: won + drawn + lost,
        won,
        drawn,
        lost,
        goals_for: gf,
        goals_against: ga,
        points: won * 3 + drawn,
    }
}

/// Build a game with a completed season (2 teams, all fixtures done).
fn make_completed_season_game() -> Game {
    let date = Utc.with_ymd_and_hms(2026, 5, 20, 12, 0, 0).unwrap();
    let clock = GameClock::new(date);
    let mut manager = Manager::new(
        "mgr1".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("team1".to_string());
    manager.satisfaction = 60;

    let team1 = make_team("team1", "Test FC");
    let team2 = make_team("team2", "Rival FC");

    let mut p1 = make_player("p1", "Star", "team1", Position::Forward);
    p1.stats = PlayerSeasonStats {
        appearances: 30,
        goals: 20,
        assists: 10,
        clean_sheets: 0,
        avg_rating: 7.5,
        minutes_played: 2700,
        yellow_cards: 3,
        red_cards: 0,
        ..PlayerSeasonStats::default()
    };

    let mut p2 = make_player("p2", "Rival", "team2", Position::Forward);
    p2.stats = PlayerSeasonStats {
        appearances: 28,
        goals: 15,
        assists: 8,
        clean_sheets: 0,
        avg_rating: 7.0,
        minutes_played: 2500,
        yellow_cards: 1,
        red_cards: 0,
        ..PlayerSeasonStats::default()
    };

    let fixtures = vec![
        make_completed_fixture("f1", "team1", "team2", 2, 1),
        make_completed_fixture("f2", "team2", "team1", 0, 1),
    ];

    // team1 won both: 6 pts, team2 lost both: 0 pts
    let standings = vec![
        make_standing("team1", 2, 0, 0, 3, 1),
        make_standing("team2", 0, 0, 2, 1, 3),
    ];

    let league = League {
        id: "league1".to_string(),
        name: "Test League".to_string(),
        season: 1,
        fixtures,
        standings,
        transfer_log: vec![],
        transfer_rumours: vec![],
        ..Default::default()
    };

    let mut game = Game::new(
        clock,
        manager,
        vec![team1, team2],
        vec![p1, p2],
        vec![],
        vec![],
    );
    game.league = Some(league);
    game
}

fn make_club(id: &str, nation: &str, reputation: u32) -> Team {
    let mut team = make_team(id, id);
    team.football_nation = nation.to_string();
    team.reputation = reputation;
    team
}

/// A single-division domestic league with `clubs` listed strongest-first; the
/// final standings are set so the listed order is the finishing order.
fn first_division(id: &str, country: &str, region: &str, clubs: &[&str]) -> League {
    let total = clubs.len() as u32;
    let standings = clubs
        .iter()
        .enumerate()
        .map(|(rank, club)| {
            let won = total - rank as u32; // strictly decreasing → unique order
            make_standing(club, won, 0, total - won, won * 2, 0)
        })
        .collect();
    League {
        id: id.to_string(),
        name: id.to_string(),
        season: 1,
        kind: CompetitionType::League,
        scope: CompetitionScope::Domestic,
        country_id: Some(country.to_string()),
        region_id: Some(region.to_string()),
        participant_ids: clubs.iter().map(|c| c.to_string()).collect(),
        standings,
        priority: 0,
        ..Default::default()
    }
}

/// A domestic knockout cup whose final has been won by `winner`.
fn domestic_cup(id: &str, country: &str, region: &str, winner: &str, loser: &str) -> League {
    let final_id = format!("{id}-final");
    let mut final_fixture = make_completed_fixture(&final_id, winner, loser, 2, 1);
    final_fixture.competition = FixtureCompetition::Cup;
    League {
        id: id.to_string(),
        name: id.to_string(),
        season: 1,
        kind: CompetitionType::Cup,
        scope: CompetitionScope::Domestic,
        country_id: Some(country.to_string()),
        region_id: Some(region.to_string()),
        fixtures: vec![final_fixture],
        knockout_rounds: vec![KnockoutRoundState {
            id: format!("{id}-final-round"),
            name: "Final".to_string(),
            fixture_ids: vec![final_id],
            bye_team_ids: vec![],
            completed: true,
        }],
        ..Default::default()
    }
}

/// A continental club competition fed by `region`, with a `field_size`-club
/// field (the placeholder entrants are replaced at qualification time).
fn continental_cup(id: &str, region: &str, field_size: usize) -> League {
    League {
        id: id.to_string(),
        name: id.to_string(),
        season: 1,
        kind: CompetitionType::ContinentalClub,
        scope: CompetitionScope::Continental,
        required_region_ids: vec![region.to_string()],
        participant_ids: (0..field_size).map(|i| format!("seed-{i}")).collect(),
        ..Default::default()
    }
}

#[test]
fn continental_fields_requalify_per_region_from_standings_and_cup_winners() {
    // Europe (ES) and South America (BR), each with six clubs whose reputations
    // mirror their finishing order. In each country the cup is won by the 5th-
    // placed club, so it qualifies despite finishing outside the top four.
    let mut teams = Vec::new();
    for (rank, id) in ["es-a", "es-b", "es-c", "es-d", "es-e", "es-f"]
        .iter()
        .enumerate()
    {
        teams.push(make_club(id, "ES", 900 - rank as u32 * 50));
    }
    for (rank, id) in ["br-a", "br-b", "br-c", "br-d", "br-e", "br-f"]
        .iter()
        .enumerate()
    {
        teams.push(make_club(id, "BR", 900 - rank as u32 * 50));
    }

    let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 5, 20, 12, 0, 0).unwrap());
    let mut manager = Manager::new(
        "mgr".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("es-a".to_string());

    let mut game = Game::new(clock, manager, teams, vec![], vec![], vec![]);
    game.competitions = vec![
        first_division(
            "es-1",
            "ES",
            "europe",
            &["es-a", "es-b", "es-c", "es-d", "es-e", "es-f"],
        ),
        domestic_cup("es-cup", "ES", "europe", "es-e", "es-f"),
        first_division(
            "br-1",
            "BR",
            "south-america",
            &["br-a", "br-b", "br-c", "br-d", "br-e", "br-f"],
        ),
        domestic_cup("br-cup", "BR", "south-america", "br-e", "br-f"),
        // Five-club fields so the top four plus the cup winner exactly fill each
        // bracket (no reputation top-up masking the cup berth).
        continental_cup("europe-cup", "europe", 5),
        continental_cup("libertadores", "south-america", 5),
    ];

    let europe = game
        .competitions
        .iter()
        .find(|c| c.id == "europe-cup")
        .unwrap();
    let euro_field = continental_qualified_entrants(&game, europe);
    for club in ["es-a", "es-b", "es-c", "es-d", "es-e"] {
        assert!(
            euro_field.contains(&club.to_string()),
            "europe field should include {club}: {euro_field:?}"
        );
    }
    assert!(
        !euro_field.contains(&"es-f".to_string()),
        "es-f neither finished top four nor won the cup: {euro_field:?}"
    );
    assert!(
        euro_field.iter().all(|id| id.starts_with("es-")),
        "europe field drew a club from another confederation: {euro_field:?}"
    );

    let south_america = game
        .competitions
        .iter()
        .find(|c| c.id == "libertadores")
        .unwrap();
    let sa_field = continental_qualified_entrants(&game, south_america);
    for club in ["br-a", "br-b", "br-c", "br-d", "br-e"] {
        assert!(
            sa_field.contains(&club.to_string()),
            "south american field should include {club}: {sa_field:?}"
        );
    }
    assert!(
        sa_field.iter().all(|id| id.starts_with("br-")),
        "south american field drew a club from another confederation: {sa_field:?}"
    );
}

/// Characterization: the built-in default berths (first division positions 1–4
/// + cup winner) reproduce the inferred continental field exactly, so routing
///   the generated world through berths is behavior-preserving.
#[test]
fn default_berths_reproduce_the_inferred_continental_field() {
    let mut teams = Vec::new();
    for (rank, id) in ["es-a", "es-b", "es-c", "es-d", "es-e", "es-f"]
        .iter()
        .enumerate()
    {
        teams.push(make_club(id, "ES", 900 - rank as u32 * 50));
    }

    let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 5, 20, 12, 0, 0).unwrap());
    let mut manager = Manager::new(
        "mgr".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("es-a".to_string());
    let mut game = Game::new(clock, manager, teams, vec![], vec![], vec![]);

    let mut first = first_division(
        "es-1",
        "ES",
        "europe",
        &["es-a", "es-b", "es-c", "es-d", "es-e", "es-f"],
    );
    first.berths = vec![Berth {
        target: "europe-cup".to_string(),
        rule: BerthRule::PositionRange { from: 1, to: 4 },
        fallback_to: None,
    }];
    let mut cup = domestic_cup("es-cup", "ES", "europe", "es-e", "es-f");
    cup.berths = vec![Berth {
        target: "europe-cup".to_string(),
        rule: BerthRule::CupWinner,
        fallback_to: None,
    }];
    game.competitions = vec![first, cup, continental_cup("europe-cup", "europe", 5)];

    let europe = game
        .competitions
        .iter()
        .find(|c| c.id == "europe-cup")
        .unwrap();

    assert_eq!(
        berth_qualified_entrants(&game, europe),
        continental_qualified_entrants(&game, europe),
        "default berths must reproduce the inferred field"
    );
}

/// Multi-tier qualification: a club lands in only its most prestigious target,
/// so a cup winner who'd otherwise be a Europa club via league position is
/// promoted to the Champions Cup and dropped from Europa.
#[test]
fn berths_keep_each_club_in_its_most_prestigious_continental_target() {
    let mut teams = Vec::new();
    for (rank, id) in ["es-a", "es-b", "es-c", "es-d", "es-e", "es-f"]
        .iter()
        .enumerate()
    {
        teams.push(make_club(id, "ES", 900 - rank as u32 * 50));
    }
    let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 5, 20, 12, 0, 0).unwrap());
    let mut manager = Manager::new(
        "mgr".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("es-a".to_string());
    let mut game = Game::new(clock, manager, teams, vec![], vec![], vec![]);

    let mut first = first_division(
        "es-1",
        "ES",
        "europe",
        &["es-a", "es-b", "es-c", "es-d", "es-e", "es-f"],
    );
    first.berths = vec![
        Berth {
            target: "ucl".to_string(),
            rule: BerthRule::PositionRange { from: 1, to: 2 },
            fallback_to: None,
        },
        Berth {
            target: "uel".to_string(),
            rule: BerthRule::PositionRange { from: 3, to: 4 },
            fallback_to: None,
        },
    ];
    // The cup is won by the 4th-placed club, whose cup berth outranks its Europa
    // league berth.
    let mut cup = domestic_cup("es-cup", "ES", "europe", "es-d", "es-e");
    cup.berths = vec![Berth {
        target: "ucl".to_string(),
        rule: BerthRule::CupWinner,
        fallback_to: Some("uel".to_string()),
    }];

    let mut ucl = continental_cup("ucl", "europe", 3);
    ucl.priority = 100;
    let mut uel = continental_cup("uel", "europe", 2);
    uel.priority = 101;
    game.competitions = vec![first, cup, ucl, uel];

    let fields = resolve_continental_fields(&game);
    let ucl_field = &fields["ucl"];
    let uel_field = &fields["uel"];

    assert!(
        ucl_field.contains(&"es-d".to_string()),
        "cup winner promoted to the top tier: {ucl_field:?}"
    );
    assert!(
        !uel_field.contains(&"es-d".to_string()),
        "a club sits in only one continental tier: {uel_field:?}"
    );
    assert!(ucl_field.contains(&"es-a".to_string()) && ucl_field.contains(&"es-b".to_string()));
    assert!(uel_field.contains(&"es-c".to_string()));
}

#[test]
fn region_for_country_prefers_world_data_over_the_catalog() {
    let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 5, 20, 12, 0, 0).unwrap());
    let manager = Manager::new(
        "m".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    let mut game = Game::new(clock, manager, vec![], vec![], vec![], vec![]);
    // A data-defined confederation: ZZ sits in "galaxy", declared by its league.
    game.competitions = vec![first_division("zz-1", "ZZ", "galaxy", &["zz-a", "zz-b"])];

    assert_eq!(
        game.region_for_country("ZZ"),
        "galaxy",
        "the world's own data wins over the catalog"
    );
    assert_eq!(
        game.region_for_country("ES"),
        "europe",
        "countries the world doesn't place fall back to the catalog"
    );
    assert_eq!(game.region_for_country("BR"), "south-america");
}

#[test]
fn rollover_passes_continental_qualifiers_into_the_next_season() {
    // 2027 is neither a World Cup summer nor a qualifying season, so the rollover
    // just regenerates club competitions.
    let clock = GameClock::new(Utc.with_ymd_and_hms(2027, 5, 20, 12, 0, 0).unwrap());
    let mut manager = Manager::new(
        "mgr".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("eng-a".to_string());

    let teams = vec![
        make_club("eng-a", "ENG", 800),
        make_club("eng-b", "ENG", 700),
    ];
    let mut p1 = make_player("p1", "Star", "eng-a", Position::Forward);
    p1.stats = PlayerSeasonStats {
        appearances: 30,
        goals: 20,
        ..PlayerSeasonStats::default()
    };
    let mut p2 = make_player("p2", "Rival", "eng-b", Position::Forward);
    p2.stats = PlayerSeasonStats {
        appearances: 28,
        goals: 8,
        ..PlayerSeasonStats::default()
    };

    let league = League {
        id: "eng-1".to_string(),
        name: "England League".to_string(),
        season: 1,
        kind: CompetitionType::League,
        scope: CompetitionScope::Domestic,
        country_id: Some("ENG".to_string()),
        region_id: Some("europe".to_string()),
        participant_ids: vec!["eng-a".to_string(), "eng-b".to_string()],
        fixtures: vec![
            make_completed_fixture("f1", "eng-a", "eng-b", 2, 0),
            make_completed_fixture("f2", "eng-b", "eng-a", 0, 1),
        ],
        // eng-a won both: 6 pts; eng-b: 0.
        standings: vec![
            make_standing("eng-a", 2, 0, 0, 3, 0),
            make_standing("eng-b", 0, 0, 2, 0, 3),
        ],
        ..Default::default()
    };
    let continental = continental_cup("ucl", "europe", 2);

    let mut game = Game::new(clock, manager, teams, vec![p1, p2], vec![], vec![]);
    game.competitions = vec![league, continental];

    process_end_of_season(&mut game);

    let ucl = game.competitions.iter().find(|c| c.id == "ucl").unwrap();
    assert!(
        ucl.participant_ids.contains(&"eng-a".to_string())
            && ucl.participant_ids.contains(&"eng-b".to_string()),
        "next season's continental field should be the domestic qualifiers, got {:?}",
        ucl.participant_ids
    );
    assert!(
        !ucl.participant_ids.iter().any(|id| id.starts_with("seed-")),
        "placeholder seeds should be replaced by real qualifiers: {:?}",
        ucl.participant_ids
    );
}

#[test]
fn process_end_of_season_reschedules_national_team_windows() {
    use domain::national_team::NationalTeam;

    let mut game = make_completed_season_game();
    // A neutral season so the windows host friendlies: 2027/28 neither stages
    // a World Cup nor hosts any part of the two-season qualifying campaign.
    game.clock = GameClock::new(Utc.with_ymd_and_hms(2027, 5, 20, 12, 0, 0).unwrap());

    let mut nt_a = NationalTeam::new("nt-a".into(), "A".into(), "AAA".into(), None);
    nt_a.squad_player_ids = vec!["p1".into()];
    nt_a.fixtures.push(Fixture {
        id: "old-friendly".into(),
        competition_id: "international-friendlies".into(),
        matchday: 1,
        date: "2026-09-09".into(),
        home_team_id: "nt-a".into(),
        away_team_id: "nt-b".into(),
        competition: FixtureCompetition::InternationalNation,
        status: FixtureStatus::Completed,
        result: None,
    });
    let mut nt_b = NationalTeam::new("nt-b".into(), "B".into(), "BBB".into(), None);
    nt_b.squad_player_ids = vec!["p2".into()];
    game.national_teams = vec![nt_a, nt_b];

    process_end_of_season(&mut game);

    let fixtures: Vec<&Fixture> = game
        .national_teams
        .iter()
        .flat_map(|team| team.fixtures.iter())
        .collect();

    assert!(!fixtures.is_empty(), "new national-team fixtures scheduled");
    assert!(
        !fixtures.iter().any(|f| f.id == "old-friendly"),
        "last season's national-team fixtures are cleared"
    );
    assert!(
        fixtures
            .iter()
            .all(|f| f.status == FixtureStatus::Scheduled),
        "rescheduled friendlies start the new season unplayed"
    );
    // Windows for the new season fall after the rollover date.
    assert!(fixtures.iter().all(|f| f.date.as_str() > "2027-06-01"));
}

#[test]
fn process_end_of_season_records_history_and_prizes_for_every_division() {
    fn prize_money_total(team: &domain::team::Team) -> i64 {
        team.financial_ledger
            .iter()
            .filter(|t| matches!(t.kind, domain::team::FinancialTransactionKind::PrizeMoney))
            .map(|t| t.amount)
            .sum()
    }

    let mut game = make_completed_season_game();
    game.teams.push(make_team("team3", "Third FC"));
    game.teams.push(make_team("team4", "Fourth FC"));
    let team3_initial_reputation = game
        .teams
        .iter()
        .find(|t| t.id == "team3")
        .unwrap()
        .reputation;

    let div1 = League {
        id: "eng-1".to_string(),
        name: "ENG First Division".to_string(),
        country_id: Some("ENG".to_string()),
        priority: 0,
        season: 1,
        participant_ids: vec!["team1".to_string(), "team2".to_string()],
        fixtures: vec![
            make_completed_fixture("d1f1", "team1", "team2", 2, 0),
            make_completed_fixture("d1f2", "team2", "team1", 0, 1),
        ],
        standings: vec![
            make_standing("team1", 2, 0, 0, 3, 0),
            make_standing("team2", 0, 0, 2, 0, 3),
        ],
        ..Default::default()
    };
    let div2 = League {
        id: "eng-2".to_string(),
        name: "ENG Second Division".to_string(),
        country_id: Some("ENG".to_string()),
        priority: 1,
        season: 1,
        participant_ids: vec!["team3".to_string(), "team4".to_string()],
        fixtures: vec![make_completed_fixture("d2f1", "team3", "team4", 3, 0)],
        standings: vec![
            make_standing("team3", 1, 0, 0, 3, 0),
            make_standing("team4", 0, 0, 1, 0, 3),
        ],
        ..Default::default()
    };
    game.league = Some(div1.clone());
    game.competitions = vec![div1, div2];

    process_end_of_season(&mut game);

    let team1 = game.teams.iter().find(|t| t.id == "team1").unwrap();
    let team3 = game.teams.iter().find(|t| t.id == "team3").unwrap();

    // Both divisions record season history with within-division positions.
    let team3_record = team3
        .history
        .last()
        .expect("second-division history recorded");
    assert_eq!(team3_record.league_position, 1);
    assert_eq!(team3_record.season, 1);

    // The second-division champion earns half the top-flight champion's prize.
    assert!(prize_money_total(team1) > 0);
    assert_eq!(prize_money_total(team3) * 2, prize_money_total(team1));

    // Winning its division improves the club's reputation.
    assert!(team3.reputation > team3_initial_reputation);
}

/// Two-division game: div1 = team1 (champion), team2 (relegated);
/// div2 = team3 (promoted), team4. The manager runs `user_team`.
fn make_two_division_game(user_team: &str) -> Game {
    let mut game = make_completed_season_game();
    game.teams.push(make_team("team3", "Third FC"));
    game.teams.push(make_team("team4", "Fourth FC"));
    game.manager.team_id = Some(user_team.to_string());

    let div1 = League {
        id: "eng-1".to_string(),
        name: "ENG First Division".to_string(),
        country_id: Some("ENG".to_string()),
        priority: 0,
        season: 1,
        participant_ids: vec!["team1".to_string(), "team2".to_string()],
        fixtures: vec![
            make_completed_fixture("d1f1", "team1", "team2", 2, 0),
            make_completed_fixture("d1f2", "team2", "team1", 0, 1),
        ],
        standings: vec![
            make_standing("team1", 2, 0, 0, 3, 0),
            make_standing("team2", 0, 0, 2, 0, 3),
        ],
        ..Default::default()
    };
    let div2 = League {
        id: "eng-2".to_string(),
        name: "ENG Second Division".to_string(),
        country_id: Some("ENG".to_string()),
        priority: 1,
        season: 1,
        participant_ids: vec!["team3".to_string(), "team4".to_string()],
        fixtures: vec![make_completed_fixture("d2f1", "team3", "team4", 3, 0)],
        standings: vec![
            make_standing("team3", 1, 0, 0, 3, 0),
            make_standing("team4", 0, 0, 1, 0, 3),
        ],
        ..Default::default()
    };
    game.league = Some(div1.clone());
    game.competitions = vec![div1, div2];
    game
}

#[test]
fn relegated_user_receives_a_relegation_message() {
    let mut game = make_two_division_game("team2");

    process_end_of_season(&mut game);

    let msg = game
        .messages
        .iter()
        .find(|m| m.id == "relegation_2")
        .expect("relegated user must be told");
    assert_eq!(
        msg.subject_key.as_deref(),
        Some("be.msg.relegation.subject")
    );
    assert_eq!(msg.body_key.as_deref(), Some("be.msg.relegation.body"));
    assert_eq!(
        msg.i18n_params.get("division"),
        Some(&"ENG Second Division".to_string())
    );
    assert!(
        !game.messages.iter().any(|m| m.id == "promotion_2"),
        "a relegated user must not also be congratulated"
    );
}

#[test]
fn promoted_user_receives_a_promotion_message() {
    let mut game = make_two_division_game("team3");

    process_end_of_season(&mut game);

    let msg = game
        .messages
        .iter()
        .find(|m| m.id == "promotion_2")
        .expect("promoted user must be told");
    assert_eq!(msg.subject_key.as_deref(), Some("be.msg.promotion.subject"));
    assert_eq!(
        msg.i18n_params.get("division"),
        Some(&"ENG First Division".to_string())
    );
}

#[test]
fn season_awards_are_scoped_to_the_users_division() {
    let mut game = make_two_division_game("team1");
    // A second-division striker outscores everyone in the user's division.
    let mut lower_star = make_player("p3", "Lower Star", "team3", Position::Forward);
    lower_star.stats = PlayerSeasonStats {
        appearances: 20,
        goals: 30,
        avg_rating: 8.0,
        minutes_played: 1800,
        ..PlayerSeasonStats::default()
    };
    game.players.push(lower_star);

    let summary = process_end_of_season(&mut game);

    assert_eq!(
        summary.golden_boot_player, "Star",
        "the golden boot belongs to the user's division, not the whole world"
    );
    assert_eq!(summary.golden_boot_goals, 20);
}

#[test]
fn user_staying_in_their_division_gets_no_movement_message() {
    let mut game = make_two_division_game("team1");

    process_end_of_season(&mut game);

    assert!(
        !game
            .messages
            .iter()
            .any(|m| m.id == "promotion_2" || m.id == "relegation_2"),
        "the champion stays up and gets no movement message"
    );
}

#[test]
fn rollover_before_a_world_cup_hosts_qualifying_then_stages_the_cup_from_it() {
    // Spring 2025 rolls into the 2025/26 season, which leads into the 2026 cup.
    let mut game = make_two_division_game("team1");
    game.clock = GameClock::new(Utc.with_ymd_and_hms(2025, 5, 20, 12, 0, 0).unwrap());

    process_end_of_season(&mut game);

    let qualifying = game
        .competitions
        .iter()
        .find(|c| ofm_core::world_cup::is_world_cup_qualifying(c))
        .expect("the pre-cup season hosts qualifying");
    assert!(!qualifying.groups.is_empty());
    // The tournament itself isn't staged yet.
    assert!(
        !game.competitions.iter().any(|c| {
            ofm_core::world_cup::is_world_cup_competition(c)
                && !ofm_core::world_cup::is_world_cup_qualifying(c)
        }),
        "the World Cup waits until the cup summer"
    );
    // No preseason friendlies clash with the qualifying windows.
    let window_dates: Vec<String> = qualifying.fixtures.iter().map(|f| f.date.clone()).collect();
    assert!(
        game.competitions
            .iter()
            .filter(|c| !ofm_core::world_cup::is_world_cup_qualifying(c))
            .flat_map(|c| c.fixtures.iter())
            .all(|f| !window_dates.contains(&f.date)),
        "club fixtures stay off the qualifying window dates"
    );
}

#[test]
fn rollover_stages_a_world_cup_in_cup_years_and_retires_it_afterwards() {
    // make_two_division_game's clock sits in May 2026 — a World Cup summer.
    let mut game = make_two_division_game("team1");

    process_end_of_season(&mut game);

    let cup = game
        .competitions
        .iter()
        .find(|c| ofm_core::world_cup::is_world_cup_competition(c))
        .expect("a 2026 rollover stages the World Cup");
    assert_eq!(cup.name, "World Cup 2026");
    assert_eq!(
        cup.participant_ids.len(),
        48,
        "the 2026 format fields 48 nations"
    );
    assert_eq!(cup.groups.len(), 12);
    assert!(
        !is_season_complete(&game),
        "the freshly regenerated club season is open and the tournament never \
         counts toward season completion"
    );
    // Clubs play no preseason friendlies while the World Cup runs.
    assert!(
        game.competitions
            .iter()
            .flat_map(|c| c.fixtures.iter())
            .all(|f| f.competition != FixtureCompetition::Friendly),
        "no club friendlies may be scheduled during a World Cup summer"
    );

    // The next rollover (summer 2027 — not a cup year) retires the tournament
    // instead of regenerating it, and stages no new one.
    game.clock = GameClock::new(Utc.with_ymd_and_hms(2027, 5, 20, 12, 0, 0).unwrap());
    process_end_of_season(&mut game);
    assert!(
        game.competitions
            .iter()
            .all(|c| !ofm_core::world_cup::is_world_cup_competition(c)),
        "last cycle's World Cup must be retired at the next rollover"
    );
    // Ordinary summers bring the preseason friendlies back.
    assert!(
        game.competitions
            .iter()
            .flat_map(|c| c.fixtures.iter())
            .any(|f| f.competition == FixtureCompetition::Friendly),
        "non-cup summers schedule preseason friendlies as before"
    );
}

#[test]
fn a_two_season_qualifying_campaign_survives_the_rollover_and_feeds_the_cup() {
    // Spring 2024 rolls into 2024/25 — two summers before the 2026 cup — so
    // the full home-and-away qualifying campaign gets under way.
    let mut game = make_two_division_game("team1");
    game.clock = GameClock::new(Utc.with_ymd_and_hms(2024, 5, 20, 12, 0, 0).unwrap());
    process_end_of_season(&mut game);

    let (qualifying_id, first_season_days) = {
        let qualifying = game
            .competitions
            .iter()
            .find(|c| ofm_core::world_cup::is_world_cup_qualifying(c))
            .expect("the campaign starts two seasons out");
        assert_eq!(qualifying.season, 2026);
        let windows = ofm_core::national_team::international_window_dates(
            Utc.with_ymd_and_hms(2024, 8, 1, 0, 0, 0).unwrap(),
        );
        (
            qualifying.id.clone(),
            ofm_core::national_team::international_window_span_dates(&windows),
        )
    };

    // Play the first season's share of the campaign only up to the club
    // season's end in May: the rollover outruns the June 2025 window.
    let mut rng = <rand::rngs::StdRng as rand::SeedableRng>::seed_from_u64(3);
    for date in first_season_days
        .iter()
        .filter(|date| date.as_str() < "2025-06-01")
    {
        ofm_core::world_cup::process_world_cup_fixtures_due(&mut game, date, &mut rng);
    }
    let played_midway = game
        .competitions
        .iter()
        .find(|c| c.id == qualifying_id)
        .unwrap()
        .fixtures
        .iter()
        .filter(|f| f.status == FixtureStatus::Completed)
        .count();
    assert!(played_midway > 0, "the campaign's first half is played");

    // The intermediate rollover (summer 2025) carries the campaign over
    // instead of retiring it: same competition, results intact, and its
    // remaining fixtures re-anchored onto the new season's windows.
    game.clock = GameClock::new(Utc.with_ymd_and_hms(2025, 5, 20, 12, 0, 0).unwrap());
    process_end_of_season(&mut game);
    let second_season_days = {
        let qualifying = game
            .competitions
            .iter()
            .find(|c| ofm_core::world_cup::is_world_cup_qualifying(c))
            .expect("an in-progress campaign survives the rollover");
        assert_eq!(qualifying.id, qualifying_id, "the same edition continues");
        let played_after = qualifying
            .fixtures
            .iter()
            .filter(|f| f.status == FixtureStatus::Completed)
            .count();
        assert!(
            played_after > played_midway,
            "played results carry over and the outrun June window is settled"
        );
        let windows = ofm_core::national_team::international_window_dates(
            Utc.with_ymd_and_hms(2025, 8, 1, 0, 0, 0).unwrap(),
        );
        let window_days: std::collections::HashSet<String> =
            ofm_core::national_team::international_window_span_dates(&windows)
                .into_iter()
                .collect();
        assert!(
            qualifying
                .fixtures
                .iter()
                .filter(|f| f.status == FixtureStatus::Scheduled)
                .all(|f| window_days.contains(&f.date)),
            "the campaign's second half sits on the new season's windows"
        );
        ofm_core::national_team::international_window_span_dates(&windows)
    };

    // Play the campaign through the March window only: the club season ends in
    // mid-May, *before* the playoff's June dates — the realistic flow.
    for date in second_season_days
        .iter()
        .filter(|date| date.as_str() < "2026-06-01")
    {
        ofm_core::world_cup::process_world_cup_fixtures_due(&mut game, date, &mut rng);
    }
    assert!(
        game.competitions
            .iter()
            .any(|c| ofm_core::world_cup::is_world_cup_playoff(c)),
        "the finished groups stage the June playoff"
    );

    // The cup-summer rollover settles the still-scheduled June playoff, then
    // derives the 48-team field from the finished campaign and retires both
    // the campaign and its playoff.
    game.clock = GameClock::new(Utc.with_ymd_and_hms(2026, 5, 20, 12, 0, 0).unwrap());
    process_end_of_season(&mut game);
    assert!(
        game.news.iter().any(|a| a.id == "world_cup_playoff_2026"),
        "the playoff bracket was played out, not dropped"
    );
    let cup = game
        .competitions
        .iter()
        .find(|c| ofm_core::world_cup::is_world_cup_competition(c))
        .expect("the cup summer stages the finals");
    assert_eq!(cup.participant_ids.len(), 48, "a full qualified field");
    assert!(
        !game
            .competitions
            .iter()
            .any(|c| ofm_core::world_cup::is_world_cup_qualifying(c)
                || ofm_core::world_cup::is_world_cup_playoff(c)),
        "the finished campaign and playoff are retired with the rollover"
    );
}

#[test]
fn season_not_complete_while_another_division_is_unfinished() {
    let mut game = make_completed_season_game();
    game.teams.push(make_team("team3", "Third FC"));
    game.teams.push(make_team("team4", "Fourth FC"));

    // div1 (the primary) is fully played; div2 still has a scheduled fixture.
    let div1 = game.league.clone().unwrap();
    let mut div2 = League {
        id: "eng-2".to_string(),
        name: "ENG Second Division".to_string(),
        country_id: Some("ENG".to_string()),
        priority: 1,
        season: 1,
        participant_ids: vec!["team3".to_string(), "team4".to_string()],
        fixtures: vec![
            make_completed_fixture("d2f1", "team3", "team4", 3, 0),
            make_completed_fixture("d2f2", "team4", "team3", 0, 1),
        ],
        standings: vec![
            make_standing("team3", 2, 0, 0, 4, 0),
            make_standing("team4", 0, 0, 2, 0, 4),
        ],
        ..Default::default()
    };
    div2.fixtures[1].status = FixtureStatus::Scheduled;
    div2.fixtures[1].result = None;
    game.competitions = vec![div1, div2];

    assert!(
        !is_season_complete(&game),
        "season must wait for every division, not just the primary league"
    );

    // Finish the second division and the season completes.
    game.competitions[1].fixtures[1].status = FixtureStatus::Completed;
    game.competitions[1].fixtures[1].result = Some(MatchResult {
        home_goals: 0,
        away_goals: 1,
        home_scorers: vec![],
        away_scorers: vec![],
        report: None,
        home_penalties: None,
        away_penalties: None,
    });
    assert!(is_season_complete(&game));
}

#[test]
fn summary_reflects_the_users_division_when_not_in_the_primary_competition() {
    let mut game = make_completed_season_game();
    game.teams.push(make_team("team3", "Third FC"));
    game.teams.push(make_team("team4", "Fourth FC"));
    // The user manages the second-division champion, not a primary-league club.
    game.manager.team_id = Some("team3".to_string());

    let div1 = League {
        id: "eng-1".to_string(),
        name: "ENG First Division".to_string(),
        country_id: Some("ENG".to_string()),
        priority: 0,
        season: 1,
        participant_ids: vec!["team1".to_string(), "team2".to_string()],
        fixtures: vec![
            make_completed_fixture("d1f1", "team1", "team2", 2, 0),
            make_completed_fixture("d1f2", "team2", "team1", 0, 1),
        ],
        standings: vec![
            make_standing("team1", 2, 0, 0, 3, 0),
            make_standing("team2", 0, 0, 2, 0, 3),
        ],
        ..Default::default()
    };
    let div2 = League {
        id: "eng-2".to_string(),
        name: "ENG Second Division".to_string(),
        country_id: Some("ENG".to_string()),
        priority: 1,
        season: 1,
        participant_ids: vec!["team3".to_string(), "team4".to_string()],
        fixtures: vec![make_completed_fixture("d2f1", "team3", "team4", 3, 0)],
        standings: vec![
            make_standing("team3", 1, 0, 0, 3, 0),
            make_standing("team4", 0, 0, 1, 0, 3),
        ],
        ..Default::default()
    };
    game.league = Some(div1.clone());
    game.competitions = vec![div1, div2];

    let summary = process_end_of_season(&mut game);

    assert_eq!(summary.league_name, "ENG Second Division");
    assert_eq!(summary.user_position, 1);
    assert_eq!(summary.champion_id, "team3");
    assert_eq!(summary.total_teams, 2);
    assert_eq!(
        game.manager.career_stats.trophies, 1,
        "winning your division counts as a trophy"
    );

    // The payout message matches the tier-scaled ledger (half the top-flight
    // champion's 5,000,000).
    let payout = game
        .messages
        .iter()
        .find(|m| m.id == "season_payout_1")
        .expect("payout message for the user's division");
    assert_eq!(
        payout.i18n_params.get("amount"),
        Some(&"2500000".to_string())
    );
}

#[test]
fn process_end_of_season_promotes_and_relegates_between_divisions() {
    let mut game = make_completed_season_game();
    game.teams.push(make_team("team3", "Third FC"));
    game.teams.push(make_team("team4", "Fourth FC"));

    let div1 = League {
        id: "eng-1".to_string(),
        name: "ENG First Division".to_string(),
        country_id: Some("ENG".to_string()),
        priority: 0,
        season: 1,
        participant_ids: vec!["team1".to_string(), "team2".to_string()],
        fixtures: vec![
            make_completed_fixture("d1f1", "team1", "team2", 2, 0),
            make_completed_fixture("d1f2", "team2", "team1", 0, 1),
        ],
        standings: vec![
            make_standing("team1", 2, 0, 0, 3, 0), // champion
            make_standing("team2", 0, 0, 2, 0, 3), // relegated
        ],
        ..Default::default()
    };
    let div2 = League {
        id: "eng-2".to_string(),
        name: "ENG Second Division".to_string(),
        country_id: Some("ENG".to_string()),
        priority: 1,
        season: 1,
        participant_ids: vec!["team3".to_string(), "team4".to_string()],
        fixtures: vec![make_completed_fixture("d2f1", "team3", "team4", 3, 0)],
        standings: vec![
            make_standing("team3", 1, 0, 0, 3, 0), // promoted
            make_standing("team4", 0, 0, 1, 0, 3),
        ],
        ..Default::default()
    };
    game.league = Some(div1.clone());
    game.competitions = vec![div1, div2];

    process_end_of_season(&mut game);

    let new_div1 = game.competitions.iter().find(|c| c.id == "eng-1").unwrap();
    let new_div2 = game.competitions.iter().find(|c| c.id == "eng-2").unwrap();

    // Two-club divisions -> one up / one down: team2 relegated, team3 promoted.
    assert!(new_div1.participant_ids.contains(&"team3".to_string()));
    assert!(!new_div1.participant_ids.contains(&"team2".to_string()));
    assert!(new_div2.participant_ids.contains(&"team2".to_string()));
    assert!(!new_div2.participant_ids.contains(&"team3".to_string()));

    // Fixtures regenerated for the new season (calendar year, not sequential).
    assert_eq!(new_div1.season, 2026);
    assert!(
        new_div1
            .fixtures
            .iter()
            .any(|f| f.status == FixtureStatus::Scheduled)
    );
}

// ---------------------------------------------------------------------------
// is_season_complete
// ---------------------------------------------------------------------------

#[test]
fn expected_fixture_count_covers_odd_team_counts() {
    // Double round robin always needs n * (n - 1) fixtures; odd leagues play
    // it out with byes.
    assert_eq!(expected_fixture_count(4), Some(12));
    assert_eq!(expected_fixture_count(5), Some(20));
    assert_eq!(expected_fixture_count(1), None);
}

#[test]
fn season_complete_when_all_fixtures_completed() {
    let game = make_completed_season_game();
    assert!(is_season_complete(&game));
}

#[test]
fn season_not_complete_with_scheduled_fixtures() {
    // One of the two league fixtures is still Scheduled.
    // has_full_schedule returns true (2 == 2), but .all(Completed) returns false.
    let mut game = make_completed_season_game();
    if let Some(league) = &mut game.league {
        league.fixtures[1].status = FixtureStatus::Scheduled;
        league.fixtures[1].result = None;
    }
    assert!(!is_season_complete(&game));
}

#[test]
fn season_not_complete_with_no_league() {
    let mut game = make_completed_season_game();
    game.league = None;
    assert!(!is_season_complete(&game));
}

#[test]
fn season_not_complete_with_empty_fixtures() {
    let mut game = make_completed_season_game();
    if let Some(league) = &mut game.league {
        league.fixtures.clear();
    }
    assert!(!is_season_complete(&game));
}

#[test]
fn season_not_complete_with_truncated_completed_fixture_list() {
    let mut game = make_completed_season_game();
    game.teams.push(make_team("team3", "Third FC"));
    game.teams.push(make_team("team4", "Fourth FC"));

    if let Some(league) = &mut game.league {
        league.standings = vec![
            make_standing("team1", 1, 0, 0, 2, 0),
            make_standing("team4", 1, 0, 0, 1, 0),
            make_standing("team3", 0, 0, 1, 0, 1),
            make_standing("team2", 0, 0, 1, 0, 2),
        ];
        league.fixtures = vec![
            Fixture {
                id: "f1".to_string(),
                matchday: 1,
                date: "2026-08-01".to_string(),
                home_team_id: "team1".to_string(),
                away_team_id: "team2".to_string(),
                competition: FixtureCompetition::League,
                status: FixtureStatus::Completed,
                result: Some(MatchResult {
                    home_goals: 2,
                    away_goals: 0,
                    home_scorers: vec![],
                    away_scorers: vec![],
                    report: None,
                    home_penalties: None,
                    away_penalties: None,
                }),
                ..Default::default()
            },
            Fixture {
                id: "f2".to_string(),
                matchday: 1,
                date: "2026-08-01".to_string(),
                home_team_id: "team3".to_string(),
                away_team_id: "team4".to_string(),
                competition: FixtureCompetition::League,
                status: FixtureStatus::Completed,
                result: Some(MatchResult {
                    home_goals: 0,
                    away_goals: 1,
                    home_scorers: vec![],
                    away_scorers: vec![],
                    report: None,
                    home_penalties: None,
                    away_penalties: None,
                }),
                ..Default::default()
            },
        ];
    }

    assert!(
        !is_season_complete(&game),
        "A truncated fixture list must not count as a completed season"
    );
}

// ---------------------------------------------------------------------------
// process_end_of_season — summary
// ---------------------------------------------------------------------------

#[test]
fn summary_has_correct_champion() {
    let mut game = make_completed_season_game();
    let summary = process_end_of_season(&mut game);
    assert_eq!(summary.champion_id, "team1");
    assert_eq!(summary.champion_name, "Test FC");
    assert_eq!(summary.season, 1);
}

#[test]
fn summary_has_correct_user_position() {
    let mut game = make_completed_season_game();
    let summary = process_end_of_season(&mut game);
    // team1 (user) is champion
    assert_eq!(summary.user_position, 1);
    assert_eq!(summary.user_points, 6);
    assert_eq!(summary.user_won, 2);
    assert_eq!(summary.user_drawn, 0);
    assert_eq!(summary.user_lost, 0);
}

#[test]
fn summary_has_correct_goals() {
    let mut game = make_completed_season_game();
    let summary = process_end_of_season(&mut game);
    assert_eq!(summary.user_goals_for, 3);
    assert_eq!(summary.user_goals_against, 1);
}

#[test]
fn summary_total_teams() {
    let mut game = make_completed_season_game();
    let summary = process_end_of_season(&mut game);
    assert_eq!(summary.total_teams, 2);
}

// ---------------------------------------------------------------------------
// process_end_of_season — history recording
// ---------------------------------------------------------------------------

#[test]
fn team_history_recorded() {
    let mut game = make_completed_season_game();
    process_end_of_season(&mut game);

    let team1 = game.teams.iter().find(|t| t.id == "team1").unwrap();
    assert_eq!(team1.history.len(), 1);
    let record = &team1.history[0];
    assert_eq!(record.season, 1);
    assert_eq!(record.league_position, 1);
    assert_eq!(record.won, 2);
    assert_eq!(record.drawn, 0);
    assert_eq!(record.lost, 0);

    let team2 = game.teams.iter().find(|t| t.id == "team2").unwrap();
    assert_eq!(team2.history.len(), 1);
    assert_eq!(team2.history[0].league_position, 2);
}

#[test]
fn team_form_cleared() {
    let mut game = make_completed_season_game();
    // Give team1 some form
    game.teams
        .iter_mut()
        .find(|t| t.id == "team1")
        .unwrap()
        .form = vec!["W".to_string(), "W".to_string()];

    process_end_of_season(&mut game);

    let team1 = game.teams.iter().find(|t| t.id == "team1").unwrap();
    assert!(
        team1.form.is_empty(),
        "Form should be cleared after season end"
    );
}

// ---------------------------------------------------------------------------
// process_end_of_season — player career/stats reset
// ---------------------------------------------------------------------------

#[test]
fn player_career_entry_added() {
    let mut game = make_completed_season_game();
    process_end_of_season(&mut game);

    let p1 = game.players.iter().find(|p| p.id == "p1").unwrap();
    assert_eq!(p1.career.len(), 1);
    let entry = &p1.career[0];
    assert_eq!(entry.season, 1);
    assert_eq!(entry.appearances, 30);
    assert_eq!(entry.goals, 20);
    assert_eq!(entry.assists, 10);
}

#[test]
fn player_stats_reset() {
    let mut game = make_completed_season_game();
    process_end_of_season(&mut game);

    let p1 = game.players.iter().find(|p| p.id == "p1").unwrap();
    assert_eq!(p1.stats.appearances, 0);
    assert_eq!(p1.stats.goals, 0);
    assert_eq!(p1.stats.assists, 0);
}

#[test]
fn season_end_ages_players_and_retires_out_of_contract_veterans() {
    let mut game = make_completed_season_game();
    let veteran = game
        .players
        .iter_mut()
        .find(|player| player.id == "p1")
        .unwrap();
    veteran.date_of_birth = "1988-01-01".to_string();
    veteran.contract_end = Some("2026-05-01".to_string());
    veteran.attributes.pace = 16;
    veteran.attributes.passing = 70;

    process_end_of_season(&mut game);

    let veteran = game
        .players
        .iter()
        .find(|player| player.id == "p1")
        .unwrap();
    assert!(
        veteran.retired,
        "older out-of-contract veterans should retire"
    );
    assert_eq!(
        veteran.team_id, None,
        "retired players should leave their club"
    );
    assert!(
        veteran.attributes.pace < 16,
        "seasonal aging should reduce veteran pace"
    );
    assert!(
        veteran.attributes.passing >= 70,
        "technical attributes should not decline before the post-32 flat phase"
    );
}

#[test]
fn player_with_zero_appearances_no_career_entry() {
    let mut game = make_completed_season_game();
    // Add a player with 0 appearances
    let p3 = make_player("p3", "Bench", "team1", Position::Defender);
    game.players.push(p3);

    process_end_of_season(&mut game);

    let p3 = game.players.iter().find(|p| p.id == "p3").unwrap();
    assert!(
        p3.career.is_empty(),
        "No career entry for 0-appearance player"
    );
}

// ---------------------------------------------------------------------------
// process_end_of_season — manager career
// ---------------------------------------------------------------------------

#[test]
fn manager_career_stats_updated() {
    let mut game = make_completed_season_game();
    process_end_of_season(&mut game);

    assert_eq!(game.manager.career_stats.matches_managed, 2);
    assert_eq!(game.manager.career_stats.wins, 2);
    assert_eq!(game.manager.career_stats.draws, 0);
    assert_eq!(game.manager.career_stats.losses, 0);
}

#[test]
fn manager_trophy_awarded_for_first_place() {
    let mut game = make_completed_season_game();
    process_end_of_season(&mut game);
    assert_eq!(game.manager.career_stats.trophies, 1);
}

#[test]
fn manager_no_trophy_for_non_first() {
    let mut game = make_completed_season_game();
    // Swap standings so team2 is first
    if let Some(league) = &mut game.league {
        league.standings = vec![
            make_standing("team2", 2, 0, 0, 3, 1),
            make_standing("team1", 0, 0, 2, 1, 3),
        ];
    }
    process_end_of_season(&mut game);
    assert_eq!(game.manager.career_stats.trophies, 0);
}

#[test]
fn manager_best_finish_set() {
    let mut game = make_completed_season_game();
    process_end_of_season(&mut game);
    assert_eq!(game.manager.career_stats.best_finish, Some(1));
}

#[test]
fn manager_career_history_entry_created() {
    let mut game = make_completed_season_game();
    process_end_of_season(&mut game);

    assert_eq!(game.manager.career_history.len(), 1);
    let entry = &game.manager.career_history[0];
    assert_eq!(entry.team_id, "team1");
    assert_eq!(entry.matches, 2);
    assert_eq!(entry.wins, 2);
    assert_eq!(entry.best_league_position, Some(1));
}

#[test]
fn manager_career_history_entry_updated_on_second_season() {
    let mut game = make_completed_season_game();
    // Add pre-existing career history entry
    game.manager
        .career_history
        .push(domain::manager::ManagerCareerEntry {
            team_id: "team1".to_string(),
            team_name: "Test FC".to_string(),
            start_date: "2025-08-01".to_string(),
            end_date: None,
            matches: 10,
            wins: 5,
            draws: 3,
            losses: 2,
            best_league_position: Some(3),
        });

    process_end_of_season(&mut game);

    // Should update existing entry, not create new
    assert_eq!(game.manager.career_history.len(), 1);
    let entry = &game.manager.career_history[0];
    assert_eq!(entry.matches, 12); // 10 + 2
    assert_eq!(entry.wins, 7); // 5 + 2
    assert_eq!(entry.best_league_position, Some(1)); // improved from 3 to 1
}

// ---------------------------------------------------------------------------
// process_end_of_season — next season generation
// ---------------------------------------------------------------------------

#[test]
fn new_league_generated() {
    let mut game = make_completed_season_game();
    process_end_of_season(&mut game);

    let league = game.league.as_ref().unwrap();
    assert_eq!(league.season, 2026, "Should be season 2026");
    assert!(
        !league.fixtures.is_empty(),
        "Should have fixtures for new season"
    );
    // All fixtures should be Scheduled
    assert!(
        league
            .fixtures
            .iter()
            .all(|f| f.status == FixtureStatus::Scheduled),
        "All new fixtures should be Scheduled"
    );
}

#[test]
fn board_objectives_cleared() {
    let mut game = make_completed_season_game();
    game.board_objectives.push(BoardObjective {
        id: "obj1".to_string(),
        objective_type: ObjectiveType::LeaguePosition,
        description: "Finish top 2".to_string(),
        target: 2,
        met: true,
    });

    process_end_of_season(&mut game);
    assert!(
        game.board_objectives.is_empty(),
        "Objectives should be cleared"
    );
}

#[test]
fn news_cleared() {
    let mut game = make_completed_season_game();
    game.news.push(domain::news::NewsArticle::new(
        "n1".to_string(),
        "Old news".to_string(),
        "...".to_string(),
        "Source".to_string(),
        "2025-01-01".to_string(),
        domain::news::NewsCategory::MatchReport,
    ));

    process_end_of_season(&mut game);
    assert!(
        game.news.iter().all(|article| article.id != "n1"),
        "Old news from the previous season should be cleared"
    );
    assert!(
        game.news
            .iter()
            .any(|article| article.category == domain::news::NewsCategory::SeasonPreview),
        "A season preview should be added for the new season"
    );
}

#[test]
fn season_awards_article_added_when_marquee_winners_exist() {
    // The fixture has a top scorer (Star, 20 goals) and a clear POTY (Star, 7.5 rating).
    let mut game = make_completed_season_game();
    process_end_of_season(&mut game);

    let awards_article = game
        .news
        .iter()
        .find(|article| article.id == "season_awards_1");
    assert!(
        awards_article.is_some(),
        "A season awards article should be published when there are marquee winners"
    );
}

#[test]
fn season_awards_article_references_top_scorer_and_their_team() {
    let mut game = make_completed_season_game();
    process_end_of_season(&mut game);

    let awards_article = game
        .news
        .iter()
        .find(|article| article.id == "season_awards_1")
        .expect("awards article should exist");

    assert_eq!(awards_article.body, "");
    assert_eq!(
        awards_article.headline_key.as_deref(),
        Some("be.news.seasonAwards.headline")
    );
    assert_eq!(
        awards_article.body_key.as_deref(),
        Some("be.news.seasonAwards.bodyBoth")
    );
    assert_eq!(
        awards_article.i18n_params.get("goldenBootWinner"),
        Some(&"Star".to_string())
    );
    assert_eq!(
        awards_article.i18n_params.get("goldenBootTeam"),
        Some(&"Test FC".to_string())
    );
    assert!(
        awards_article.player_ids.contains(&"p1".to_string()),
        "Awards article should link the winning player by id"
    );
    assert!(
        awards_article.team_ids.contains(&"team1".to_string()),
        "Awards article should link the winning club by id"
    );
}

#[test]
fn season_awards_article_not_added_when_no_marquee_winners() {
    // Strip players' goals and ratings so there's no Golden Boot / POTY winner.
    let mut game = make_completed_season_game();
    for player in game.players.iter_mut() {
        player.stats.goals = 0;
        player.stats.assists = 0;
        player.stats.avg_rating = 0.0;
    }
    process_end_of_season(&mut game);

    assert!(
        game.news
            .iter()
            .all(|article| article.id != "season_awards_1"),
        "No awards article should be published when no marquee award has a winner"
    );
}

// ---------------------------------------------------------------------------
// process_end_of_season — messages
// ---------------------------------------------------------------------------

#[test]
fn champion_receives_prize_money_and_ledger_entry() {
    let mut game = make_completed_season_game();
    let initial_finance = game
        .teams
        .iter()
        .find(|team| team.id == "team1")
        .unwrap()
        .finance;

    process_end_of_season(&mut game);

    let team1 = game.teams.iter().find(|team| team.id == "team1").unwrap();
    assert_eq!(team1.finance, initial_finance + 5_000_000);
    assert_eq!(team1.season_income, 5_000_000);
    assert_eq!(team1.financial_ledger.len(), 1);
    assert_eq!(
        team1.financial_ledger[0].kind,
        FinancialTransactionKind::PrizeMoney
    );
    assert_eq!(team1.financial_ledger[0].amount, 5_000_000);
    assert_eq!(
        team1.financial_ledger[0].description,
        "be.msg.seasonPayout.ledgerDescription?season=1&position=1&suffix=st"
    );
}

#[test]
fn top_half_finish_receives_expected_prize_money() {
    let mut game = make_completed_season_game();
    game.teams.push(make_team("team3", "Third FC"));
    game.teams.push(make_team("team4", "Fourth FC"));

    if let Some(league) = &mut game.league {
        league.standings = vec![
            make_standing("team2", 6, 0, 0, 12, 2),
            make_standing("team1", 4, 0, 2, 8, 5),
            make_standing("team3", 2, 0, 4, 4, 8),
            make_standing("team4", 0, 0, 6, 2, 12),
        ];
    }

    let initial_finance = game
        .teams
        .iter()
        .find(|team| team.id == "team1")
        .unwrap()
        .finance;

    process_end_of_season(&mut game);

    let team1 = game.teams.iter().find(|team| team.id == "team1").unwrap();
    assert_eq!(team1.finance, initial_finance + 3_000_000);
}

#[test]
fn lower_table_finish_receives_expected_prize_money() {
    let mut game = make_completed_season_game();

    for i in 3..=10 {
        let team_id = format!("team{}", i);
        game.teams
            .push(make_team(&team_id, &format!("Team{} FC", i)));
    }

    if let Some(league) = &mut game.league {
        let mut standings = Vec::new();

        for i in 2..=10 {
            standings.push(make_standing(&format!("team{}", i), 10, 2, 6, 20, 15));
        }

        standings.push(make_standing("team1", 0, 0, 18, 2, 40));
        league.standings = standings;
    }

    let initial_finance = game
        .teams
        .iter()
        .find(|team| team.id == "team1")
        .unwrap()
        .finance;

    process_end_of_season(&mut game);

    let team1 = game.teams.iter().find(|team| team.id == "team1").unwrap();
    assert_eq!(team1.finance, initial_finance + 150_000);
}

#[test]
fn prize_money_message_sent_once_per_season() {
    let mut game = make_completed_season_game();
    game.messages.push(domain::message::InboxMessage::new(
        "season_payout_1".to_string(),
        "Already exists".to_string(),
        "...".to_string(),
        "Board".to_string(),
        "2026-05-20".to_string(),
    ));

    process_end_of_season(&mut game);

    let payout_messages = game
        .messages
        .iter()
        .filter(|message| message.id == "season_payout_1")
        .count();

    assert_eq!(payout_messages, 1);
}

#[test]
fn season_end_message_sent() {
    let mut game = make_completed_season_game();
    process_end_of_season(&mut game);

    let msg = game.messages.iter().find(|m| m.id == "season_end_1");
    assert!(msg.is_some(), "Should send season end message");
    let msg = msg.unwrap();
    assert_eq!(msg.subject, "");
    assert_eq!(
        msg.subject_key.as_deref(),
        Some("be.msg.seasonReview.subject")
    );
    assert_eq!(msg.sender, "");
    assert_eq!(msg.sender_role, "");
}

#[test]
fn new_season_schedule_message_sent() {
    let mut game = make_completed_season_game();
    process_end_of_season(&mut game);

    let msg = game.messages.iter().find(|m| m.id == "new_season_2");
    assert!(msg.is_some(), "Should send new season message");
    let msg = msg.unwrap();
    assert_eq!(msg.subject, "");
    assert_eq!(
        msg.subject_key.as_deref(),
        Some("be.msg.newSeasonSchedule.subject")
    );
    assert_eq!(msg.sender, "");
    assert_eq!(msg.sender_role, "");
}

#[test]
fn messages_not_duplicated() {
    let mut game = make_completed_season_game();
    // Pre-add the messages
    game.messages.push(domain::message::InboxMessage::new(
        "season_end_1".to_string(),
        "Already exists".to_string(),
        "...".to_string(),
        "Board".to_string(),
        "2026-05-20".to_string(),
    ));
    game.messages.push(domain::message::InboxMessage::new(
        "new_season_2".to_string(),
        "Already exists".to_string(),
        "...".to_string(),
        "League".to_string(),
        "2026-05-20".to_string(),
    ));

    process_end_of_season(&mut game);

    let season_end_count = game
        .messages
        .iter()
        .filter(|m| m.id == "season_end_1")
        .count();
    let new_season_count = game
        .messages
        .iter()
        .filter(|m| m.id == "new_season_2")
        .count();
    assert_eq!(
        season_end_count, 1,
        "Should not duplicate season_end message"
    );
    assert_eq!(
        new_season_count, 1,
        "Should not duplicate new_season message"
    );
}

// ---------------------------------------------------------------------------
// process_end_of_season — board message variations
// ---------------------------------------------------------------------------

#[test]
fn champion_gets_congratulatory_message() {
    let mut game = make_completed_season_game();
    process_end_of_season(&mut game);

    let msg = game
        .messages
        .iter()
        .find(|m| m.id == "season_end_1")
        .unwrap();
    assert_eq!(msg.body, "");
    assert_eq!(
        msg.body_key.as_deref(),
        Some("be.msg.seasonReview.body.champion")
    );
}

#[test]
fn mid_table_gets_appropriate_message() {
    let mut game = make_completed_season_game();
    // Make a 4-team league where team1 finishes 3rd (mid-table for total_teams=4)
    let team3 = make_team("team3", "Third FC");
    let team4 = make_team("team4", "Fourth FC");
    game.teams.push(team3);
    game.teams.push(team4);

    if let Some(league) = &mut game.league {
        league.standings = vec![
            make_standing("team2", 6, 0, 0, 12, 2),
            make_standing("team3", 4, 0, 2, 8, 5),
            make_standing("team1", 2, 0, 4, 4, 8), // user team 3rd of 4
            make_standing("team4", 0, 0, 6, 2, 12),
        ];
    }

    let summary = process_end_of_season(&mut game);
    // 3rd out of 4 → user_position=3, total_teams=4, 3 <= 4/2=2 is false, so it's "below mid"
    // Actually 3 <= 4/2=2 → false → goes to else branch (disappointing)
    assert_eq!(summary.user_position, 3);
}

#[test]
fn bottom_half_gets_concerned_message() {
    let mut game = make_completed_season_game();
    // Add enough teams so that finishing last (10th of 10) triggers the disappointed branch
    // (user_position > 4 AND user_position > total_teams / 2)
    for i in 3..=10 {
        let tid = format!("team{}", i);
        game.teams.push(make_team(&tid, &format!("Team{} FC", i)));
    }
    if let Some(league) = &mut game.league {
        let mut standings = Vec::new();
        for i in 2..=10 {
            standings.push(make_standing(&format!("team{}", i), 10, 2, 6, 20, 15));
        }
        // team1 (user) finishes dead last
        standings.push(make_standing("team1", 0, 0, 18, 2, 40));
        league.standings = standings;
    }

    process_end_of_season(&mut game);

    let msg = game
        .messages
        .iter()
        .find(|m| m.id == "season_end_1")
        .unwrap();
    assert_eq!(msg.body, "");
    assert_eq!(
        msg.body_key.as_deref(),
        Some("be.msg.seasonReview.body.lowerHalf")
    );
}

// ---------------------------------------------------------------------------
// process_end_of_season — no league edge case
// ---------------------------------------------------------------------------

#[test]
fn no_league_returns_default_summary() {
    let date = Utc.with_ymd_and_hms(2026, 5, 20, 12, 0, 0).unwrap();
    let clock = GameClock::new(date);
    let mut manager = Manager::new(
        "mgr1".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("team1".to_string());

    let mut game = Game::new(clock, manager, vec![], vec![], vec![], vec![]);
    // No league set
    let summary = process_end_of_season(&mut game);
    assert_eq!(summary.season, 0);
    assert!(summary.league_name.is_empty());
}

// ---------------------------------------------------------------------------
// process_end_of_season — satisfaction adjustment
// ---------------------------------------------------------------------------

#[test]
fn satisfaction_adjusted_after_season() {
    let mut game = make_completed_season_game();
    let initial_sat = game.manager.satisfaction;
    process_end_of_season(&mut game);
    // With no objectives, evaluate_objectives returns 0, so satisfaction unchanged
    assert_eq!(game.manager.satisfaction, initial_sat);
}

// ---------------------------------------------------------------------------
// process_end_of_season — team reputation updates
// ---------------------------------------------------------------------------

#[test]
fn low_reputation_champion_gains_reputation() {
    let mut game = make_completed_season_game();
    let team1 = game
        .teams
        .iter_mut()
        .find(|team| team.id == "team1")
        .unwrap();
    team1.reputation = 320;

    process_end_of_season(&mut game);

    let updated_team1 = game.teams.iter().find(|team| team.id == "team1").unwrap();
    assert!(
        updated_team1.reputation > 320,
        "Winning the league from a low starting reputation should raise the club's reputation"
    );
}

#[test]
fn high_reputation_bottom_side_loses_reputation() {
    let mut game = make_completed_season_game();

    for i in 3..=10 {
        let team_id = format!("team{}", i);
        game.teams
            .push(make_team(&team_id, &format!("Team{} FC", i)));
    }

    game.teams
        .iter_mut()
        .find(|team| team.id == "team1")
        .unwrap()
        .reputation = 860;

    if let Some(league) = &mut game.league {
        let mut standings = Vec::new();
        for i in 2..=10 {
            standings.push(make_standing(&format!("team{}", i), 10, 2, 6, 20, 15));
        }
        standings.push(make_standing("team1", 0, 0, 18, 2, 40));
        league.standings = standings;
    }

    process_end_of_season(&mut game);

    let updated_team1 = game.teams.iter().find(|team| team.id == "team1").unwrap();
    assert!(
        updated_team1.reputation < 860,
        "A high-reputation club collapsing to the bottom should lose reputation"
    );
}

// ---------------------------------------------------------------------------
// is_season_complete — season not started guard
// ---------------------------------------------------------------------------

#[test]
fn season_not_complete_when_no_matches_played() {
    // Full schedule exists but all fixtures are still Scheduled (preseason state).
    // is_season_complete must return false — we must not trigger end-of-season
    // processing before the campaign has even begun.
    let mut game = make_completed_season_game();
    if let Some(league) = &mut game.league {
        for fixture in &mut league.fixtures {
            fixture.status = FixtureStatus::Scheduled;
            fixture.result = None;
        }
        for standing in &mut league.standings {
            *standing = StandingEntry::new(standing.team_id.clone());
        }
    }
    assert!(
        !is_season_complete(&game),
        "Season with no matches played must not be considered complete"
    );
}

// ---------------------------------------------------------------------------
// process_end_of_season — message dates
// ---------------------------------------------------------------------------

#[test]
fn season_end_messages_dated_on_last_fixture_date() {
    // make_completed_season_game() sets the clock to 2026-05-20 but both league
    // fixtures are dated 2025-06-01 (see make_completed_fixture).
    // End-of-season messages must be dated on the last completed fixture date
    // (2025-06-01), not on the clock date (2026-05-20).
    let mut game = make_completed_season_game();
    process_end_of_season(&mut game);

    let board_msg = game
        .messages
        .iter()
        .find(|m| m.id == "season_end_1")
        .expect("season_end_1 message must be present");
    assert_eq!(
        board_msg.date, "2025-06-01",
        "Board review must be dated on the last fixture date, not the clock date"
    );

    let payout_msg = game
        .messages
        .iter()
        .find(|m| m.id == "season_payout_1")
        .expect("season_payout_1 message must be present");
    assert_eq!(
        payout_msg.date, "2025-06-01",
        "Prize money message must be dated on the last fixture date"
    );

    let schedule_msg = game
        .messages
        .iter()
        .find(|m| m.id == "new_season_2")
        .expect("new_season_2 message must be present");
    assert_eq!(
        schedule_msg.date, "2025-06-01",
        "New season schedule message must be dated on the last fixture date"
    );
}

// ---------------------------------------------------------------------------
// process_end_of_season — i18n on end-of-season messages
// ---------------------------------------------------------------------------

#[test]
fn season_end_board_message_has_i18n_keys() {
    let mut game = make_completed_season_game();
    process_end_of_season(&mut game);

    let msg = game
        .messages
        .iter()
        .find(|m| m.id == "season_end_1")
        .expect("season_end_1 message must be present");

    assert_eq!(
        msg.subject_key.as_deref(),
        Some("be.msg.seasonReview.subject"),
        "Board review subject must have i18n key"
    );
    assert!(
        msg.body_key.is_some(),
        "Board review body must have an i18n key"
    );
    assert!(
        msg.body_key
            .as_deref()
            .unwrap_or("")
            .starts_with("be.msg.seasonReview.body."),
        "Board review body key must be under be.msg.seasonReview.body, got: {:?}",
        msg.body_key
    );
    assert!(
        msg.i18n_params.contains_key("season"),
        "Board review i18n params must contain 'season'"
    );
    assert!(
        msg.i18n_params.contains_key("team"),
        "Board review i18n params must contain 'team'"
    );
    assert!(
        msg.i18n_params.contains_key("points"),
        "Board review i18n params must contain 'points'"
    );
}

#[test]
fn season_end_board_message_has_sender_i18n() {
    let mut game = make_completed_season_game();
    process_end_of_season(&mut game);

    let msg = game
        .messages
        .iter()
        .find(|m| m.id == "season_end_1")
        .expect("season_end_1 message must be present");

    assert_eq!(
        msg.sender_key.as_deref(),
        Some("be.sender.boardOfDirectors"),
        "Board review sender must have i18n key"
    );
    assert_eq!(
        msg.sender_role_key.as_deref(),
        Some("be.role.chairman"),
        "Board review sender role must have i18n key"
    );
    assert_eq!(msg.sender, "");
    assert_eq!(msg.sender_role, "");
}

#[test]
fn season_end_new_schedule_message_has_i18n_keys() {
    let mut game = make_completed_season_game();
    process_end_of_season(&mut game);

    let msg = game
        .messages
        .iter()
        .find(|m| m.id == "new_season_2")
        .expect("new_season_2 message must be present");

    assert_eq!(
        msg.subject_key.as_deref(),
        Some("be.msg.newSeasonSchedule.subject"),
        "New season schedule subject must have i18n key"
    );
    assert_eq!(
        msg.body_key.as_deref(),
        Some("be.msg.newSeasonSchedule.body"),
        "New season schedule body must have i18n key"
    );
    assert_eq!(
        msg.sender_key.as_deref(),
        Some("be.sender.leagueOffice"),
        "New season schedule sender must have i18n key"
    );
    assert_eq!(
        msg.sender_role_key.as_deref(),
        Some("be.role.competitionSecretary"),
        "New season schedule sender role must have i18n key"
    );
    assert!(
        msg.i18n_params.contains_key("season"),
        "New season schedule i18n params must contain 'season'"
    );
    assert_eq!(msg.body, "");
    assert_eq!(msg.sender, "");
    assert_eq!(msg.sender_role, "");
}

#[test]
fn season_end_payout_message_has_i18n_keys() {
    let mut game = make_completed_season_game();
    process_end_of_season(&mut game);

    let msg = game
        .messages
        .iter()
        .find(|m| m.id == "season_payout_1")
        .expect("season_payout_1 message must be present");

    assert_eq!(msg.subject, "");
    assert_eq!(msg.body, "");
    assert_eq!(msg.sender, "");
    assert_eq!(msg.sender_role, "");
    assert_eq!(
        msg.subject_key.as_deref(),
        Some("be.msg.seasonPayout.subject")
    );
    assert_eq!(msg.body_key.as_deref(), Some("be.msg.seasonPayout.body"));
    assert_eq!(
        msg.sender_key.as_deref(),
        Some("be.sender.boardOfDirectors")
    );
    assert_eq!(msg.sender_role_key.as_deref(), Some("be.role.chairman"));
    assert_eq!(msg.i18n_params.get("season"), Some(&"1".to_string()));
    assert_eq!(msg.i18n_params.get("position"), Some(&"1".to_string()));
}

#[test]
fn season_end_board_message_top_four_uses_correct_body_key() {
    let mut game = make_completed_season_game();
    // Make team1 finish 2nd (top-4 branch)
    if let Some(league) = &mut game.league {
        league.standings = vec![
            make_standing("team2", 2, 0, 0, 3, 1),
            make_standing("team1", 0, 0, 2, 1, 3),
        ];
    }
    process_end_of_season(&mut game);

    let msg = game
        .messages
        .iter()
        .find(|m| m.id == "season_end_1")
        .unwrap();
    assert_eq!(
        msg.body_key.as_deref(),
        Some("be.msg.seasonReview.body.topFour"),
        "2nd-place finish should use topFour body key"
    );
    assert!(
        msg.i18n_params.contains_key("position"),
        "topFour key must include position param"
    );
    assert!(
        msg.i18n_params.contains_key("suffix"),
        "topFour key must include suffix param"
    );
}

// ---------------------------------------------------------------------------
// Retiree conversion and pool replenishment
// ---------------------------------------------------------------------------

fn make_retired_player(id: &str) -> domain::player::Player {
    use domain::player::{CareerEntry, PlayerAttributes, Position};
    let attrs = PlayerAttributes {
        pace: 55,
        stamina: 55,
        strength: 55,
        agility: 55,
        passing: 70,
        shooting: 60,
        tackling: 65,
        dribbling: 60,
        defending: 65,
        positioning: 72,
        vision: 74,
        decisions: 70,
        composure: 65,
        aggression: 50,
        teamwork: 68,
        leadership: 60,
        handling: 20,
        reflexes: 20,
        aerial: 55,
    };
    let mut p = domain::player::Player::new(
        id.to_string(),
        format!("{} Ret", id),
        format!("Full {}", id),
        "1988-01-01".to_string(),
        "ENG".to_string(),
        Position::Midfielder,
        attrs,
    );
    p.team_id = None;
    p.retired = true;
    p.career.push(CareerEntry {
        season: 1,
        team_id: "team1".to_string(),
        team_name: "Test FC".to_string(),
        appearances: 30,
        goals: 5,
        assists: 8,
    });
    p
}

#[test]
fn retired_player_creates_manager_candidate_with_deterministic_id() {
    let mut game = make_completed_season_game();
    game.players.push(make_retired_player("retiree1"));

    process_end_of_season(&mut game);

    let mgr_id = "mgr_retired_retiree1";
    assert!(
        game.managers.iter().any(|m| m.id == mgr_id),
        "unemployed manager with id '{}' should exist after season end",
        mgr_id
    );
}

#[test]
fn retired_player_creates_scout_candidate_with_deterministic_id() {
    let mut game = make_completed_season_game();
    game.players.push(make_retired_player("retiree2"));

    process_end_of_season(&mut game);

    let scout_id = "staff_retired_scout_retiree2";
    assert!(
        game.staff.iter().any(|s| s.id == scout_id
            && matches!(s.role, domain::staff::StaffRole::Scout)
            && s.team_id.is_none()),
        "unattached scout '{}' should exist after season end",
        scout_id
    );
}

#[test]
fn retiree_conversion_is_idempotent() {
    let mut game = make_completed_season_game();
    game.players.push(make_retired_player("idem1"));

    process_end_of_season(&mut game);
    // Build a fresh completed-season state and re-trigger (simulates second season)
    // Manually reset enough state so process_end_of_season can run again
    if let Some(league) = &mut game.league {
        // Mark new season fixtures as completed for the guard check
        for f in league.fixtures.iter_mut() {
            f.status = domain::league::FixtureStatus::Completed;
            f.result = Some(domain::league::MatchResult {
                home_goals: 1,
                away_goals: 0,
                home_scorers: vec![],
                away_scorers: vec![],
                report: None,
                home_penalties: None,
                away_penalties: None,
            });
        }
    }
    process_end_of_season(&mut game);

    let mgr_count = game
        .managers
        .iter()
        .filter(|m| m.id == "mgr_retired_idem1")
        .count();
    assert_eq!(
        mgr_count, 1,
        "duplicate manager must not be created on second call"
    );

    let scout_count = game
        .staff
        .iter()
        .filter(|s| s.id == "staff_retired_scout_idem1")
        .count();
    assert_eq!(
        scout_count, 1,
        "duplicate scout must not be created on second call"
    );
}

#[test]
fn retired_player_with_no_career_is_not_converted() {
    let mut game = make_completed_season_game();
    let mut p = make_retired_player("no_career");
    p.career.clear(); // ineligible
    game.players.push(p);

    process_end_of_season(&mut game);

    assert!(
        !game
            .managers
            .iter()
            .any(|m| m.id == "mgr_retired_no_career"),
        "players with no career entries must not become manager candidates"
    );
}

#[test]
fn manager_pool_is_topped_up_to_floor_after_season() {
    let mut game = make_completed_season_game();
    let team_count = game.teams.len(); // 2 teams → floor = 4

    process_end_of_season(&mut game);

    let user_mgr_id = game.manager.id.clone();
    let unemployed = game
        .managers
        .iter()
        .filter(|m| m.id != user_mgr_id && m.team_id.is_none())
        .count();
    assert!(
        unemployed >= team_count * 2,
        "unemployed manager pool ({}) should be >= floor ({})",
        unemployed,
        team_count * 2
    );
}

#[test]
fn scout_pool_is_topped_up_to_floor_after_season() {
    let mut game = make_completed_season_game();
    let team_count = game.teams.len(); // 2 teams → floor = 4

    process_end_of_season(&mut game);

    let unemployed_scouts = game
        .staff
        .iter()
        .filter(|s| s.team_id.is_none() && matches!(s.role, domain::staff::StaffRole::Scout))
        .count();
    assert!(
        unemployed_scouts >= team_count * 2,
        "unemployed scout pool ({}) should be >= floor ({})",
        unemployed_scouts,
        team_count * 2
    );
}

#[test]
fn retiree_manager_reputation_is_derived_from_ovr_and_career() {
    let mut game = make_completed_season_game();
    game.players.push(make_retired_player("ovr_test"));

    process_end_of_season(&mut game);

    let mgr = game
        .managers
        .iter()
        .find(|m| m.id == "mgr_retired_ovr_test")
        .expect("manager should exist");
    assert!(mgr.reputation >= 200, "reputation should be >= 200");
    assert!(mgr.reputation <= 900, "reputation should be <= 900");
    assert_eq!(mgr.satisfaction, 50);
    assert_eq!(mgr.fan_approval, 50);
}

#[test]
fn retiree_scout_judging_attributes_derived_from_player_attrs() {
    let mut game = make_completed_season_game();
    game.players.push(make_retired_player("attr_test"));

    process_end_of_season(&mut game);

    let scout = game
        .staff
        .iter()
        .find(|s| s.id == "staff_retired_scout_attr_test")
        .expect("scout should exist");
    // vision=74, decisions=70 → judging_ability = (74+70)/2 = 72
    assert_eq!(scout.attributes.judging_ability, 72);
    // positioning=72, teamwork=68 → judging_potential = (72+68)/2 = 70
    assert_eq!(scout.attributes.judging_potential, 70);
}

#[test]
fn process_end_of_season_refreshes_transfer_budget_from_finance() {
    let mut game = make_completed_season_game();
    // Simulate a season where the user emptied their transfer envelope and
    // another club still has an unspent chunk. Both should be reset to a
    // fresh envelope sized to post-prize finance — the "unspent chunk" is
    // deliberately blown away to match the real-football convention of a
    // new board grant each season.
    let team1_idx = game.teams.iter().position(|t| t.id == "team1").unwrap();
    game.teams[team1_idx].finance = 10_000_000;
    game.teams[team1_idx].transfer_budget = 0;
    let team2_idx = game.teams.iter().position(|t| t.id == "team2").unwrap();
    game.teams[team2_idx].finance = 2_000_000;
    game.teams[team2_idx].transfer_budget = 750_000;

    process_end_of_season(&mut game);

    // Formula matches worldgen (generator/mod.rs:543): 15% of finance.
    // We assert the invariant against post-prize finance so the test stays
    // valid regardless of how much prize money each team receives.
    let team1 = game.teams.iter().find(|t| t.id == "team1").unwrap();
    let team2 = game.teams.iter().find(|t| t.id == "team2").unwrap();
    assert_eq!(team1.transfer_budget, (team1.finance as f64 * 0.15) as i64);
    assert_eq!(team2.transfer_budget, (team2.finance as f64 * 0.15) as i64);
    // Team1 went in with an empty envelope; the refill must have run.
    assert!(team1.transfer_budget > 0);
}
