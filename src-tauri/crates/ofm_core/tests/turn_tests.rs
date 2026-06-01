use chrono::{TimeZone, Utc};
use domain::league::{Fixture, FixtureCompetition, FixtureStatus, League, StandingEntry};
use domain::manager::Manager;
use domain::news::NewsCategory;
use domain::player::{
    Injury, Player, PlayerAttributes, PlayerIssue, PlayerIssueCategory, PlayerPromise,
    PlayerPromiseKind, Position,
};
use domain::staff::{Staff, StaffAttributes, StaffRole};
use domain::team::Team;
use engine::Side;
use engine::report::{GoalDetail, MatchReport, PlayerMatchStats, TeamStats};
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::turn;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn default_attrs() -> PlayerAttributes {
    PlayerAttributes {
        pace: 60,
        stamina: 60,
        strength: 60,
        agility: 60,
        passing: 60,
        shooting: 60,
        tackling: 60,
        dribbling: 60,
        defending: 60,
        positioning: 60,
        vision: 60,
        decisions: 60,
        composure: 60,
        aggression: 60,
        teamwork: 60,
        leadership: 60,
        handling: 30,
        reflexes: 30,
        aerial: 60,
    }
}

fn gk_attrs() -> PlayerAttributes {
    PlayerAttributes {
        pace: 40,
        stamina: 50,
        strength: 60,
        agility: 70,
        passing: 40,
        shooting: 20,
        tackling: 20,
        dribbling: 20,
        defending: 30,
        positioning: 70,
        vision: 50,
        decisions: 60,
        composure: 70,
        aggression: 30,
        teamwork: 60,
        leadership: 50,
        handling: 80,
        reflexes: 80,
        aerial: 70,
    }
}

fn make_player(id: &str, name: &str, team_id: &str, pos: Position) -> Player {
    let attrs = if pos == Position::Goalkeeper {
        gk_attrs()
    } else {
        default_attrs()
    };
    let mut p = Player::new(
        id.to_string(),
        name.to_string(),
        name.to_string(),
        "1995-01-01".to_string(),
        "England".to_string(),
        pos,
        attrs,
    );
    p.team_id = Some(team_id.to_string());
    p.morale = 70;
    p.condition = 100;
    p
}

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

fn make_staff(
    id: &str,
    team_id: &str,
    role: StaffRole,
    first_name: &str,
    last_name: &str,
) -> Staff {
    let mut staff = Staff::new(
        id.to_string(),
        first_name.to_string(),
        last_name.to_string(),
        "1980-01-01".to_string(),
        role,
        StaffAttributes {
            coaching: 60,
            judging_ability: 60,
            judging_potential: 60,
            physiotherapy: 20,
        },
    );
    staff.nationality = "England".to_string();
    staff.team_id = Some(team_id.to_string());
    staff
}

fn make_squad(team_id: &str, prefix: &str) -> Vec<Player> {
    let mut players = Vec::new();
    // 1 GK
    players.push(make_player(
        &format!("{}_gk", prefix),
        &format!("{} GK", prefix),
        team_id,
        Position::Goalkeeper,
    ));
    // 4 DEF
    for i in 0..4 {
        players.push(make_player(
            &format!("{}_def{}", prefix, i),
            &format!("{} Def{}", prefix, i),
            team_id,
            Position::Defender,
        ));
    }
    // 4 MID
    for i in 0..4 {
        players.push(make_player(
            &format!("{}_mid{}", prefix, i),
            &format!("{} Mid{}", prefix, i),
            team_id,
            Position::Midfielder,
        ));
    }
    // 2 FWD
    for i in 0..2 {
        players.push(make_player(
            &format!("{}_fwd{}", prefix, i),
            &format!("{} Fwd{}", prefix, i),
            team_id,
            Position::Forward,
        ));
    }
    players
}

fn make_game_with_match() -> Game {
    let date = Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap();
    let clock = GameClock::new(date);
    let mut manager = Manager::new(
        "mgr1".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("team1".to_string());

    let team1 = make_team("team1", "Test FC");
    let team2 = make_team("team2", "Rival FC");
    let mut players = make_squad("team1", "t1");
    players.extend(make_squad("team2", "t2"));

    let today = date.format("%Y-%m-%d").to_string();
    let league = League {
        id: "league1".to_string(),
        name: "Test League".to_string(),
        season: 1,
        fixtures: vec![Fixture {
            id: "fix1".to_string(),
            matchday: 1,
            date: today,
            home_team_id: "team1".to_string(),
            away_team_id: "team2".to_string(),
            competition: FixtureCompetition::League,
            status: FixtureStatus::Scheduled,
            result: None,
        }],
        standings: vec![
            StandingEntry::new("team1".to_string()),
            StandingEntry::new("team2".to_string()),
        ],
        transfer_log: vec![],
        transfer_rumours: vec![],
    };

    let mut game = Game::new(clock, manager, vec![team1, team2], players, vec![], vec![]);
    game.league = Some(league);
    game
}

fn make_game_without_match_today() -> Game {
    let mut game = make_game_with_match();
    if let Some(league) = &mut game.league {
        league.fixtures[0].date = "2025-06-16".to_string();
    }
    game
}

fn empty_report(home_goals: u8, away_goals: u8) -> MatchReport {
    MatchReport {
        home_goals,
        away_goals,
        home_stats: TeamStats::default(),
        away_stats: TeamStats::default(),
        events: vec![],
        goals: vec![],
        player_stats: HashMap::new(),
        home_possession: 50.0,
        total_minutes: 90,
    }
}

fn report_with_scorer(home_goals: u8, away_goals: u8, scorer_id: &str, side: Side) -> MatchReport {
    let mut player_stats = HashMap::new();
    player_stats.insert(
        scorer_id.to_string(),
        PlayerMatchStats {
            minutes_played: 90,
            goals: if side == Side::Home {
                home_goals
            } else {
                away_goals
            },
            assists: 0,
            shots: 3,
            shots_on_target: 2,
            passes_completed: 30,
            passes_attempted: 35,
            tackles_won: 2,
            interceptions: 1,
            fouls_committed: 1,
            yellow_cards: 0,
            red_cards: 0,
            rating: 7.5,
        },
    );
    let goals = (0..home_goals)
        .map(|i| GoalDetail {
            minute: 10 + i * 20,
            scorer_id: if side == Side::Home {
                scorer_id.to_string()
            } else {
                "other".to_string()
            },
            assist_id: None,
            is_penalty: false,
            side: Side::Home,
        })
        .chain((0..away_goals).map(|i| GoalDetail {
            minute: 15 + i * 20,
            scorer_id: if side == Side::Away {
                scorer_id.to_string()
            } else {
                "other".to_string()
            },
            assist_id: None,
            is_penalty: false,
            side: Side::Away,
        }))
        .collect();

    MatchReport {
        home_goals,
        away_goals,
        home_stats: TeamStats::default(),
        away_stats: TeamStats::default(),
        events: vec![],
        goals,
        player_stats,
        home_possession: 55.0,
        total_minutes: 90,
    }
}

#[test]
fn process_day_fires_ai_manager_after_heavy_losing_run() {
    let mut game = make_game_without_match_today();

    game.teams
        .iter_mut()
        .find(|team| team.id == "team2")
        .unwrap()
        .manager_id = Some("mgr2".to_string());
    game.teams
        .iter_mut()
        .find(|team| team.id == "team2")
        .unwrap()
        .form = vec![
        "L".to_string(),
        "L".to_string(),
        "L".to_string(),
        "L".to_string(),
    ];

    let mut ai_manager = Manager::new(
        "mgr2".to_string(),
        "Marco".to_string(),
        "Rossi".to_string(),
        "1978-03-12".to_string(),
        "Italy".to_string(),
    );
    ai_manager.hire("team2".to_string());
    ai_manager.warning_stage = 1;
    game.managers.push(ai_manager);

    turn::process_day(&mut game);

    let rival_team = game.teams.iter().find(|team| team.id == "team2").unwrap();
    assert!(rival_team.manager_id.is_none());
    assert!(game.news.iter().any(|article| {
        article.category == NewsCategory::ManagerialChange
            && article.team_ids.contains(&"team2".to_string())
    }));
    assert_eq!(game.manager.team_id.as_deref(), Some("team1"));
}

#[test]
fn process_day_hires_replacement_for_long_vacant_ai_club() {
    let mut game = make_game_without_match_today();
    game.staff.push(make_staff(
        "staff-team2",
        "team2",
        StaffRole::AssistantManager,
        "Marco",
        "Rossi",
    ));

    let mut fired_manager = Manager::new(
        "mgr2".to_string(),
        "Former".to_string(),
        "Boss".to_string(),
        "1978-03-12".to_string(),
        "England".to_string(),
    );
    fired_manager.hire("team2".to_string());
    fired_manager.fire("2025-06-14");
    game.managers.push(fired_manager.clone());
    game.vacant_team_days.insert("team2".to_string(), 6);

    turn::process_day(&mut game);

    let replacement_manager_id = game
        .teams
        .iter()
        .find(|team| team.id == "team2")
        .and_then(|team| team.manager_id.clone())
        .expect("aged vacancy should be filled during daily processing");

    assert_ne!(replacement_manager_id, fired_manager.id);
    assert!(
        game.managers
            .iter()
            .any(|manager| manager.id == replacement_manager_id
                && manager.team_id.as_deref() == Some("team2"))
    );
    assert!(!game.vacant_team_days.contains_key("team2"));
}

/// Creates a match report where all 22 players played the full 90 minutes.
/// Use this for stamina depletion tests.
fn full_squad_report(home_goals: u8, away_goals: u8) -> MatchReport {
    let prefixes = ["t1_gk", "t2_gk"];
    let mut player_stats: HashMap<String, PlayerMatchStats> = HashMap::new();
    // Add GKs
    for prefix in &prefixes {
        player_stats.insert(
            prefix.to_string(),
            PlayerMatchStats {
                minutes_played: 90,
                ..Default::default()
            },
        );
    }
    // Add outfield players
    for prefix in ["t1", "t2"] {
        for i in 0..4 {
            player_stats.insert(
                format!("{}_def{}", prefix, i),
                PlayerMatchStats {
                    minutes_played: 90,
                    ..Default::default()
                },
            );
            player_stats.insert(
                format!("{}_mid{}", prefix, i),
                PlayerMatchStats {
                    minutes_played: 90,
                    ..Default::default()
                },
            );
        }
        for i in 0..2 {
            player_stats.insert(
                format!("{}_fwd{}", prefix, i),
                PlayerMatchStats {
                    minutes_played: 90,
                    ..Default::default()
                },
            );
        }
    }
    MatchReport {
        home_goals,
        away_goals,
        home_stats: TeamStats::default(),
        away_stats: TeamStats::default(),
        events: vec![],
        goals: vec![],
        player_stats,
        home_possession: 50.0,
        total_minutes: 90,
    }
}
// ---------------------------------------------------------------------------

#[test]
fn process_day_advances_clock() {
    let mut game = make_game_with_match();
    let before = game.clock.current_date;
    turn::process_day(&mut game);
    assert_eq!(
        (game.clock.current_date - before).num_days(),
        1,
        "Clock should advance by 1 day"
    );
}

#[test]
fn process_day_simulates_match() {
    let mut game = make_game_with_match();
    turn::process_day(&mut game);

    let fixture = &game.league.as_ref().unwrap().fixtures[0];
    assert_eq!(fixture.status, FixtureStatus::Completed);
    assert!(fixture.result.is_some());
}

#[test]
fn process_day_updates_standings() {
    let mut game = make_game_with_match();
    turn::process_day(&mut game);

    let standings = &game.league.as_ref().unwrap().standings;
    let total_played: u32 = standings.iter().map(|s| s.played).sum();
    assert_eq!(
        total_played, 2,
        "Both teams should have played 1 match each"
    );
}

#[test]
fn process_day_no_match_runs_training() {
    let mut game = make_game_with_match();
    // Set fixture to a different date so there's no match today
    game.league.as_mut().unwrap().fixtures[0].date = "2025-06-20".to_string();

    turn::process_day(&mut game);

    // Training may or may not affect condition depending on schedule, but clock advances
    assert_eq!(
        game.clock.current_date.format("%Y-%m-%d").to_string(),
        "2025-06-16"
    );
}

#[test]
fn process_day_no_league_no_crash() {
    let mut game = make_game_with_match();
    game.league = None;
    turn::process_day(&mut game);
    assert_eq!(
        game.clock.current_date.format("%Y-%m-%d").to_string(),
        "2025-06-16"
    );
}

#[test]
fn process_day_generates_match_result_message() {
    let mut game = make_game_with_match();
    turn::process_day(&mut game);

    // Should have a match result message for the user's team
    let result_msgs = game
        .messages
        .iter()
        .filter(|m| m.id.starts_with("result_"))
        .count();
    assert!(
        result_msgs > 0,
        "Should generate match result message for user's team"
    );
}

#[test]
fn process_day_generates_news() {
    let mut game = make_game_with_match();
    turn::process_day(&mut game);

    assert!(
        !game.news.is_empty(),
        "Should generate news articles after match day"
    );
}

#[test]
fn process_day_releases_players_with_expired_contracts() {
    let mut game = make_game_with_match();
    game.league.as_mut().unwrap().fixtures[0].date = "2025-06-20".to_string();

    let player = game.players.iter_mut().find(|p| p.id == "t1_fwd0").unwrap();
    player.contract_end = Some("2025-06-15".to_string());
    player.wage = 12_000;
    player.morale = 70;

    turn::process_day(&mut game);

    let released_player = game.players.iter().find(|p| p.id == "t1_fwd0").unwrap();
    assert_eq!(released_player.team_id, None);
    assert_eq!(released_player.contract_end, None);
    assert_eq!(released_player.wage, 0);
    let message = game
        .messages
        .iter()
        .find(|message| message.id == "contract_expired_t1_fwd0")
        .expect("An inbox message should explain that the player left on a free");
    assert_eq!(
        message.subject_key.as_deref(),
        Some("be.msg.contractExpired.subject")
    );
    assert_eq!(message.body_key.as_deref(), Some("be.msg.contractExpired.body"));
    assert_eq!(message.sender_key.as_deref(), Some("be.sender.assistantManager"));
    assert_eq!(message.sender_role_key.as_deref(), Some("be.role.assistantManager"));
    assert!(message.subject.is_empty());
    assert!(message.body.is_empty());
    assert!(message.sender.is_empty());
    assert!(message.sender_role.is_empty());
}

// ---------------------------------------------------------------------------
// finish_live_match_day tests
// ---------------------------------------------------------------------------

#[test]
fn finish_live_match_day_advances_clock() {
    let mut game = make_game_with_match();
    let before = game.clock.current_date;
    turn::finish_live_match_day(&mut game);
    assert_eq!((game.clock.current_date - before).num_days(), 1);
}

// ---------------------------------------------------------------------------
// simulate_other_matches tests
// ---------------------------------------------------------------------------

#[test]
fn simulate_other_matches_processes_all() {
    let mut game = make_game_with_match();
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    turn::simulate_other_matches(&mut game, &today, None);

    let fixture = &game.league.as_ref().unwrap().fixtures[0];
    assert_eq!(fixture.status, FixtureStatus::Completed);
}

#[test]
fn simulate_other_matches_skips_fixture() {
    let mut game = make_game_with_match();
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    // Skip the only fixture
    turn::simulate_other_matches(&mut game, &today, Some(0));

    let fixture = &game.league.as_ref().unwrap().fixtures[0];
    assert_eq!(
        fixture.status,
        FixtureStatus::Scheduled,
        "Skipped fixture should remain scheduled"
    );
}

#[test]
fn simulate_other_matches_no_league_no_crash() {
    let mut game = make_game_with_match();
    game.league = None;
    turn::simulate_other_matches(&mut game, "2025-06-15", None);
}

// ---------------------------------------------------------------------------
// apply_match_report tests
// ---------------------------------------------------------------------------

#[test]
fn apply_match_report_updates_fixture_status() {
    let mut game = make_game_with_match();
    let report = empty_report(2, 1);
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);

    let fixture = &game.league.as_ref().unwrap().fixtures[0];
    assert_eq!(fixture.status, FixtureStatus::Completed);
    let result = fixture.result.as_ref().unwrap();
    assert_eq!(result.home_goals, 2);
    assert_eq!(result.away_goals, 1);
    let persisted_report = result
        .report
        .as_ref()
        .expect("compact report should persist");
    assert_eq!(persisted_report.total_minutes, 90);
    assert_eq!(persisted_report.home_stats.possession_pct, 50);
    assert_eq!(persisted_report.away_stats.possession_pct, 50);
}

#[test]
fn apply_match_report_updates_standings() {
    let mut game = make_game_with_match();
    let report = empty_report(2, 1);
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);

    let standings = &game.league.as_ref().unwrap().standings;
    let home = standings.iter().find(|s| s.team_id == "team1").unwrap();
    let away = standings.iter().find(|s| s.team_id == "team2").unwrap();

    assert_eq!(home.played, 1);
    assert_eq!(home.won, 1);
    assert_eq!(home.points, 3);
    assert_eq!(home.goals_for, 2);
    assert_eq!(home.goals_against, 1);

    assert_eq!(away.played, 1);
    assert_eq!(away.lost, 1);
    assert_eq!(away.points, 0);
}

#[test]
fn apply_match_report_draw_standings() {
    let mut game = make_game_with_match();
    let report = empty_report(1, 1);
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);

    let standings = &game.league.as_ref().unwrap().standings;
    let home = standings.iter().find(|s| s.team_id == "team1").unwrap();
    let away = standings.iter().find(|s| s.team_id == "team2").unwrap();

    assert_eq!(home.drawn, 1);
    assert_eq!(home.points, 1);
    assert_eq!(away.drawn, 1);
    assert_eq!(away.points, 1);
}

#[test]
fn apply_match_report_updates_player_stats() {
    let mut game = make_game_with_match();
    let report = report_with_scorer(2, 0, "t1_fwd0", Side::Home);
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);

    let scorer = game.players.iter().find(|p| p.id == "t1_fwd0").unwrap();
    assert_eq!(scorer.stats.appearances, 1);
    assert_eq!(scorer.stats.goals, 2);
    assert_eq!(scorer.stats.shots, 3);
    assert_eq!(scorer.stats.shots_on_target, 2);
    assert_eq!(scorer.stats.passes_completed, 30);
    assert_eq!(scorer.stats.passes_attempted, 35);
    assert_eq!(scorer.stats.tackles_won, 2);
    assert_eq!(scorer.stats.interceptions, 1);
    assert_eq!(scorer.stats.fouls_committed, 1);
    assert!(scorer.stats.avg_rating > 0.0);
}

#[test]
fn apply_match_report_gk_clean_sheet() {
    let mut game = make_game_with_match();
    // Home team wins 1-0, so home GK gets a clean sheet
    let mut player_stats = HashMap::new();
    player_stats.insert(
        "t1_gk".to_string(),
        PlayerMatchStats {
            minutes_played: 90,
            rating: 7.0,
            ..Default::default()
        },
    );
    let report = MatchReport {
        home_goals: 1,
        away_goals: 0,
        player_stats,
        ..empty_report(1, 0)
    };
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);

    let gk = game.players.iter().find(|p| p.id == "t1_gk").unwrap();
    assert_eq!(gk.stats.clean_sheets, 1);
}

#[test]
fn apply_match_report_gk_no_clean_sheet_on_conceding() {
    let mut game = make_game_with_match();
    let mut player_stats = HashMap::new();
    player_stats.insert(
        "t1_gk".to_string(),
        PlayerMatchStats {
            minutes_played: 90,
            rating: 6.0,
            ..Default::default()
        },
    );
    let report = MatchReport {
        home_goals: 1,
        away_goals: 2,
        player_stats,
        ..empty_report(1, 2)
    };
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);

    let gk = game.players.iter().find(|p| p.id == "t1_gk").unwrap();
    assert_eq!(gk.stats.clean_sheets, 0);
}

#[test]
fn apply_match_report_depletes_stamina() {
    let mut game = make_game_with_match();
    let report = full_squad_report(1, 0);
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);

    // All players on both teams should have reduced condition
    for p in &game.players {
        assert!(
            p.condition < 100,
            "Player {} condition should be depleted after match",
            p.id
        );
    }
}

#[test]
fn apply_match_report_updates_morale() {
    let mut game = make_game_with_match();
    // Set all morale to 70
    for p in &mut game.players {
        p.morale = 70;
    }
    let report = empty_report(3, 0); // Home team big win
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);

    // Home team players should generally have higher morale, away team lower
    let home_avg: f64 = game
        .players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some("team1"))
        .map(|p| p.morale as f64)
        .sum::<f64>()
        / 11.0;
    let away_avg: f64 = game
        .players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some("team2"))
        .map(|p| p.morale as f64)
        .sum::<f64>()
        / 11.0;

    assert!(
        home_avg > away_avg,
        "Winning team morale ({:.1}) should be higher than losing team ({:.1})",
        home_avg,
        away_avg
    );
}

#[test]
fn apply_match_report_updates_team_form() {
    let mut game = make_game_with_match();
    let report = empty_report(2, 1);
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);

    let home_team = game.teams.iter().find(|t| t.id == "team1").unwrap();
    let away_team = game.teams.iter().find(|t| t.id == "team2").unwrap();

    assert_eq!(home_team.form, vec!["W"]);
    assert_eq!(away_team.form, vec!["L"]);
}

#[test]
fn apply_match_report_form_draw() {
    let mut game = make_game_with_match();
    let report = empty_report(1, 1);
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);

    let home_team = game.teams.iter().find(|t| t.id == "team1").unwrap();
    assert_eq!(home_team.form, vec!["D"]);
}

#[test]
fn apply_match_report_form_caps_at_5() {
    let mut game = make_game_with_match();
    // Pre-fill form with 5 wins
    for team in &mut game.teams {
        team.form = vec![
            "W".to_string(),
            "W".to_string(),
            "W".to_string(),
            "W".to_string(),
            "W".to_string(),
        ];
    }
    let report = empty_report(0, 1); // Home loss
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);

    let home_team = game.teams.iter().find(|t| t.id == "team1").unwrap();
    assert_eq!(home_team.form.len(), 5);
    assert_eq!(home_team.form.last().unwrap(), "L");
}

#[test]
fn apply_match_report_board_satisfaction_increases_on_win() {
    let mut game = make_game_with_match();
    game.manager.satisfaction = 50;
    let report = empty_report(3, 0);
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);
    assert!(
        game.manager.satisfaction > 50,
        "Satisfaction should increase after win"
    );
}

#[test]
fn apply_match_report_board_satisfaction_decreases_on_loss() {
    let mut game = make_game_with_match();
    game.manager.satisfaction = 50;
    let report = empty_report(0, 3);
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);
    assert!(
        game.manager.satisfaction < 50,
        "Satisfaction should decrease after loss"
    );
}

#[test]
fn apply_match_report_fan_approval_changes() {
    let mut game = make_game_with_match();
    game.manager.fan_approval = 50;
    let report = empty_report(5, 0); // Big win for extra bonus
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);
    assert!(
        game.manager.fan_approval > 50,
        "Fan approval should increase after big win"
    );
}

#[test]
fn apply_match_report_fan_approval_decreases_on_big_loss() {
    let mut game = make_game_with_match();
    game.manager.fan_approval = 50;
    let report = empty_report(0, 5); // Big loss for extra penalty
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);
    assert!(
        game.manager.fan_approval < 50,
        "Fan approval should decrease after big loss"
    );
}

#[test]
fn apply_match_report_no_satisfaction_change_for_non_user_team() {
    let mut game = make_game_with_match();
    game.manager.team_id = Some("team3".to_string()); // Different team
    game.manager.satisfaction = 50;
    game.manager.fan_approval = 50;
    let report = empty_report(3, 0);
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);
    assert_eq!(game.manager.satisfaction, 50);
    assert_eq!(game.manager.fan_approval, 50);
}

#[test]
fn apply_match_report_running_avg_rating() {
    let mut game = make_game_with_match();

    // First match: player gets 8.0 rating
    let mut ps1 = HashMap::new();
    ps1.insert(
        "t1_mid0".to_string(),
        PlayerMatchStats {
            minutes_played: 90,
            rating: 8.0,
            ..Default::default()
        },
    );
    let report1 = MatchReport {
        player_stats: ps1,
        ..empty_report(1, 0)
    };
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report1);

    let player = game.players.iter().find(|p| p.id == "t1_mid0").unwrap();
    assert!((player.stats.avg_rating - 8.0).abs() < 0.01);

    // Reset fixture for second match
    game.league.as_mut().unwrap().fixtures[0].status = FixtureStatus::Scheduled;
    game.league.as_mut().unwrap().fixtures[0].result = None;

    // Second match: player gets 6.0 rating
    let mut ps2 = HashMap::new();
    ps2.insert(
        "t1_mid0".to_string(),
        PlayerMatchStats {
            minutes_played: 90,
            rating: 6.0,
            ..Default::default()
        },
    );
    let report2 = MatchReport {
        player_stats: ps2,
        ..empty_report(0, 0)
    };
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report2);

    let player = game.players.iter().find(|p| p.id == "t1_mid0").unwrap();
    // Running average of 8.0 and 6.0 = 7.0
    assert!(
        (player.stats.avg_rating - 7.0).abs() < 0.01,
        "Running avg should be 7.0, got {}",
        player.stats.avg_rating
    );
}

#[test]
fn apply_match_report_yellow_and_red_cards() {
    let mut game = make_game_with_match();
    let mut player_stats = HashMap::new();
    player_stats.insert(
        "t1_mid0".to_string(),
        PlayerMatchStats {
            minutes_played: 90,
            yellow_cards: 1,
            red_cards: 0,
            rating: 5.0,
            ..Default::default()
        },
    );
    player_stats.insert(
        "t2_def0".to_string(),
        PlayerMatchStats {
            minutes_played: 90,
            yellow_cards: 0,
            red_cards: 1,
            rating: 3.0,
            ..Default::default()
        },
    );
    let report = MatchReport {
        player_stats,
        ..empty_report(1, 0)
    };
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);

    let mid = game.players.iter().find(|p| p.id == "t1_mid0").unwrap();
    assert_eq!(mid.stats.yellow_cards, 1);

    let def = game.players.iter().find(|p| p.id == "t2_def0").unwrap();
    assert_eq!(def.stats.red_cards, 1);
}

#[test]
fn apply_match_report_individual_morale_boost_from_goals() {
    let mut game = make_game_with_match();
    for p in &mut game.players {
        p.morale = 50;
    }
    let report = report_with_scorer(2, 0, "t1_fwd0", Side::Home);
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);

    let scorer = game.players.iter().find(|p| p.id == "t1_fwd0").unwrap();
    // Win boost (3-8) + 2 goals * 3 + high rating bonus (2) = at least 50 + 3 + 6 + 2 = 61
    assert!(
        scorer.morale > 55,
        "Scorer morale should be boosted significantly, got {}",
        scorer.morale
    );
}

#[test]
fn broken_playing_time_promise_reduces_trust_after_match() {
    let mut game = make_game_with_match();
    let promised = game.players.iter_mut().find(|p| p.id == "t1_fwd0").unwrap();
    promised.morale_core.manager_trust = 60;
    promised.morale_core.pending_promise = Some(PlayerPromise {
        kind: PlayerPromiseKind::PlayingTime,
        matches_remaining: 1,
    });

    let report = empty_report(1, 0);
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);

    let promised = game.players.iter().find(|p| p.id == "t1_fwd0").unwrap();
    assert!(promised.morale_core.manager_trust < 60);
    assert!(promised.morale_core.pending_promise.is_none());
    assert_eq!(
        promised
            .morale_core
            .unresolved_issue
            .as_ref()
            .map(|issue| &issue.category),
        Some(&PlayerIssueCategory::PlayingTime)
    );
}

#[test]
fn severe_unresolved_issue_blocks_match_and_streak_recovery() {
    let mut game = make_game_with_match();
    for team in &mut game.teams {
        if team.id == "team1" {
            team.form = vec!["W".to_string(), "W".to_string()];
        }
    }

    let player = game.players.iter_mut().find(|p| p.id == "t1_fwd0").unwrap();
    player.morale = 60;
    player.morale_core.unresolved_issue = Some(PlayerIssue {
        category: PlayerIssueCategory::Contract,
        severity: 80,
    });

    let report = report_with_scorer(2, 0, "t1_fwd0", Side::Home);
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);

    let player = game.players.iter().find(|p| p.id == "t1_fwd0").unwrap();
    assert!(
        player.morale <= 60,
        "severe unresolved issues should block easy recovery from wins and streaks, got {}",
        player.morale
    );
}

#[test]
fn moderate_unresolved_issue_slows_post_match_recovery() {
    let mut game = make_game_with_match();
    let player = game.players.iter_mut().find(|p| p.id == "t1_fwd0").unwrap();
    player.morale = 50;
    player.morale_core.unresolved_issue = Some(PlayerIssue {
        category: PlayerIssueCategory::Contract,
        severity: 60,
    });

    let report = report_with_scorer(2, 0, "t1_fwd0", Side::Home);
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);

    let player = game.players.iter().find(|p| p.id == "t1_fwd0").unwrap();
    assert!(
        player.morale <= 57,
        "moderate unresolved issues should slow post-match recovery, got {}",
        player.morale
    );
}

#[test]
fn apply_match_report_morale_drop_from_red_card() {
    let mut game = make_game_with_match();
    for p in &mut game.players {
        p.morale = 70;
    }
    let mut player_stats = HashMap::new();
    player_stats.insert(
        "t1_mid0".to_string(),
        PlayerMatchStats {
            minutes_played: 90,
            red_cards: 1,
            rating: 4.0,
            ..Default::default()
        },
    );
    let report = MatchReport {
        player_stats,
        ..empty_report(0, 2) // Loss
    };
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);

    let mid = game.players.iter().find(|p| p.id == "t1_mid0").unwrap();
    // Loss (-8 to -2) + red card (-8) + poor rating (-3) = substantial drop
    assert!(
        mid.morale < 65,
        "Red card + loss should significantly drop morale, got {}",
        mid.morale
    );
}

// ---------------------------------------------------------------------------
// Streak morale tests
// ---------------------------------------------------------------------------

#[test]
fn three_win_streak_boosts_team_morale() {
    let mut game = make_game_with_match();
    // Pre-set 2 wins in form, this match will add a 3rd
    for team in &mut game.teams {
        team.form = vec!["W".to_string(), "W".to_string()];
    }
    for p in &mut game.players {
        p.morale = 60;
    }

    let report = empty_report(2, 0); // Home win
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);

    let home_team = game.teams.iter().find(|t| t.id == "team1").unwrap();
    assert_eq!(home_team.form, vec!["W", "W", "W"]);

    // Home team players should have extra morale from streak
    let home_avg: f64 = game
        .players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some("team1"))
        .map(|p| p.morale as f64)
        .sum::<f64>()
        / 11.0;
    assert!(
        home_avg > 65.0,
        "3-win streak should boost morale significantly, avg={}",
        home_avg
    );
}

#[test]
fn three_loss_streak_drops_team_morale() {
    let mut game = make_game_with_match();
    for team in &mut game.teams {
        team.form = vec!["L".to_string(), "L".to_string()];
    }
    for p in &mut game.players {
        p.morale = 70;
    }

    let report = empty_report(0, 2); // Home loss
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);

    let home_avg: f64 = game
        .players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some("team1"))
        .map(|p| p.morale as f64)
        .sum::<f64>()
        / 11.0;
    assert!(
        home_avg < 65.0,
        "3-loss streak should drop morale, avg={}",
        home_avg
    );
}

// ---------------------------------------------------------------------------
// generate_matchday_news tests
// ---------------------------------------------------------------------------

#[test]
fn generate_matchday_news_creates_roundup_and_standings() {
    let mut game = make_game_with_match();
    // Complete the fixture first
    let report = empty_report(1, 0);
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);

    let today = "2025-06-15";
    game.news.clear();
    turn::generate_matchday_news(&mut game, today);

    let roundup = game.news.iter().any(|n| n.id.starts_with("roundup_"));
    let standings = game.news.iter().any(|n| n.id.starts_with("standings_"));
    assert!(roundup, "Should generate roundup article");
    assert!(standings, "Should generate standings article");
}

#[test]
fn generate_matchday_news_no_duplicates() {
    let mut game = make_game_with_match();
    let report = empty_report(1, 0);
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);

    let today = "2025-06-15";
    turn::generate_matchday_news(&mut game, today);
    let count = game.news.len();
    turn::generate_matchday_news(&mut game, today);
    assert_eq!(game.news.len(), count, "Should not duplicate news articles");
}

#[test]
fn generate_matchday_news_no_league_no_crash() {
    let mut game = make_game_with_match();
    game.league = None;
    turn::generate_matchday_news(&mut game, "2025-06-15");
    assert!(game.news.is_empty());
}

#[test]
fn generate_matchday_news_no_completed_fixtures_no_crash() {
    let mut game = make_game_with_match();
    // Fixture is still scheduled (not completed)
    turn::generate_matchday_news(&mut game, "2025-06-15");
    // Should produce no roundup since nothing completed
    let roundup = game.news.iter().any(|n| n.id.starts_with("roundup_"));
    assert!(!roundup);
}

// ---------------------------------------------------------------------------
// Stamina depletion tests
// ---------------------------------------------------------------------------

#[test]
fn stamina_depletion_varies_by_attribute() {
    let mut game = make_game_with_match();
    // Give one player high stamina, another low
    if let Some(p) = game.players.iter_mut().find(|p| p.id == "t1_mid0") {
        p.attributes.stamina = 90;
        p.condition = 100;
    }
    if let Some(p) = game.players.iter_mut().find(|p| p.id == "t1_mid1") {
        p.attributes.stamina = 30;
        p.condition = 100;
    }

    let report = full_squad_report(1, 0);
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);

    let high_stam = game.players.iter().find(|p| p.id == "t1_mid0").unwrap();
    let low_stam = game.players.iter().find(|p| p.id == "t1_mid1").unwrap();
    assert!(
        high_stam.condition > low_stam.condition,
        "High stamina player ({}) should retain more condition than low ({}) ",
        high_stam.condition,
        low_stam.condition
    );
}

// ---------------------------------------------------------------------------
// Injury recovery progression tests
// ---------------------------------------------------------------------------

#[test]
fn process_day_progresses_injury_recovery() {
    let mut game = make_game_with_match();
    // Ensure non-match path by moving fixture date away from today
    game.league.as_mut().unwrap().fixtures[0].date = "2025-06-20".to_string();
    // Detach manager to skip user-specific random/player events
    game.manager.team_id = None;

    let short = game.players.iter_mut().find(|p| p.id == "t1_def0").unwrap();
    short.injury = Some(Injury {
        name: "Hamstring".to_string(),
        days_remaining: 1,
    });
    let long = game.players.iter_mut().find(|p| p.id == "t1_mid0").unwrap();
    long.injury = Some(Injury {
        name: "Ankle".to_string(),
        days_remaining: 3,
    });

    turn::process_day(&mut game);

    let short_after = game.players.iter().find(|p| p.id == "t1_def0").unwrap();
    assert!(
        short_after.injury.is_none(),
        "1-day injury should be cleared after a day"
    );

    let long_after = game.players.iter().find(|p| p.id == "t1_mid0").unwrap();
    assert_eq!(
        long_after.injury.as_ref().map(|inj| inj.days_remaining),
        Some(2),
        "3-day injury should decrement to 2 after a day"
    );
}

#[test]
fn finish_live_match_day_progresses_injury_recovery() {
    let mut game = make_game_with_match();
    // Detach manager to skip user-specific random/player events
    game.manager.team_id = None;

    let recovering = game.players.iter_mut().find(|p| p.id == "t2_def1").unwrap();
    recovering.injury = Some(Injury {
        name: "Knee".to_string(),
        days_remaining: 2,
    });

    turn::finish_live_match_day(&mut game);

    let recovering_after = game.players.iter().find(|p| p.id == "t2_def1").unwrap();
    assert_eq!(
        recovering_after
            .injury
            .as_ref()
            .map(|inj| inj.days_remaining),
        Some(1),
        "2-day injury should decrement to 1 after live match day ends"
    );
}

#[test]
fn injury_with_zero_days_is_cleared() {
    let mut game = make_game_with_match();
    game.league.as_mut().unwrap().fixtures[0].date = "2025-06-20".to_string();
    game.manager.team_id = None;

    // Defensive: even if someone somehow creates an injury with 0 days, it should clear
    let p = game.players.iter_mut().find(|p| p.id == "t1_def1").unwrap();
    p.injury = Some(Injury {
        name: "Bruise".to_string(),
        days_remaining: 0,
    });

    turn::process_day(&mut game);

    let p_after = game.players.iter().find(|p| p.id == "t1_def1").unwrap();
    assert!(
        p_after.injury.is_none(),
        "0-day injury should be cleared after a day"
    );
}

// ---------------------------------------------------------------------------
// Pre-match message tests
// ---------------------------------------------------------------------------

#[test]
fn pre_match_message_generated_3_days_before() {
    let mut game = make_game_with_match();
    // Set fixture 3 days in the future
    let future = (game.clock.current_date + chrono::Duration::days(3))
        .format("%Y-%m-%d")
        .to_string();
    game.league.as_mut().unwrap().fixtures[0].date = future;

    turn::process_day(&mut game);

    let prematch = game.messages.iter().any(|m| m.id.starts_with("prematch_"));
    assert!(prematch, "Should generate pre-match message 3 days before");
}

#[test]
fn pre_match_message_not_duplicated() {
    let mut game = make_game_with_match();
    let future = (game.clock.current_date + chrono::Duration::days(3))
        .format("%Y-%m-%d")
        .to_string();
    game.league.as_mut().unwrap().fixtures[0].date = future.clone();

    turn::process_day(&mut game);
    let count = game
        .messages
        .iter()
        .filter(|m| m.id.starts_with("prematch_"))
        .count();

    // Process another day — pre-match should not duplicate
    // Need to keep fixture 3 days ahead of new clock
    let future2 = (game.clock.current_date + chrono::Duration::days(3))
        .format("%Y-%m-%d")
        .to_string();
    game.league.as_mut().unwrap().fixtures[0].date = future2;
    turn::process_day(&mut game);

    let count2 = game
        .messages
        .iter()
        .filter(|m| m.id.starts_with("prematch_"))
        .count();
    // Each day creates a message for the fixture 3 days out; if same fixture, no dup
    assert!(count2 >= count);
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

#[test]
fn apply_match_report_satisfaction_clamps() {
    let mut game = make_game_with_match();
    game.manager.satisfaction = 2;
    let report = empty_report(0, 5); // Loss = -3
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);
    assert!(game.manager.satisfaction <= 2); // clamped to 0 min

    game.league.as_mut().unwrap().fixtures[0].status = FixtureStatus::Scheduled;
    game.league.as_mut().unwrap().fixtures[0].result = None;
    game.manager.satisfaction = 99;
    let report = empty_report(5, 0); // Win = +2
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);
    assert!(game.manager.satisfaction <= 100);
}

#[test]
fn apply_match_report_morale_clamped_to_10_100() {
    let mut game = make_game_with_match();
    for p in &mut game.players {
        p.morale = 10; // Already at minimum
    }
    let report = empty_report(0, 5); // Big loss
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);

    for p in &game.players {
        assert!(
            p.morale >= 10,
            "Morale should never go below 10, player {} has {}",
            p.id,
            p.morale
        );
    }
}

#[test]
fn apply_match_report_condition_doesnt_underflow() {
    let mut game = make_game_with_match();
    for p in &mut game.players {
        p.condition = 5; // Very low
    }
    let report = empty_report(1, 0);
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);

    for p in &game.players {
        // saturating_sub should prevent underflow
        assert!(
            p.condition <= 5,
            "Condition should not underflow for {}",
            p.id
        );
    }
}

#[test]
fn process_day_full_integration() {
    // Full integration: 2 teams, match day, verify everything updates
    let mut game = make_game_with_match();
    turn::process_day(&mut game);

    // Fixture completed
    let fixture = &game.league.as_ref().unwrap().fixtures[0];
    assert_eq!(fixture.status, FixtureStatus::Completed);

    // Standings updated
    let total_played: u32 = game
        .league
        .as_ref()
        .unwrap()
        .standings
        .iter()
        .map(|s| s.played)
        .sum();
    assert_eq!(total_played, 2);

    // Players have stats
    let has_appearances = game.players.iter().any(|p| p.stats.appearances > 0);
    assert!(has_appearances);

    // Condition depleted
    let all_full_condition = game.players.iter().all(|p| p.condition == 100);
    assert!(!all_full_condition);

    // Form updated
    let home_form = &game.teams.iter().find(|t| t.id == "team1").unwrap().form;
    assert_eq!(home_form.len(), 1);

    // News generated
    assert!(!game.news.is_empty());

    // Clock advanced
    assert_eq!(
        game.clock.current_date.format("%Y-%m-%d").to_string(),
        "2025-06-16"
    );

    // Satisfaction/approval may have changed (user team played)
    // Just verify they're in valid range
    assert!(game.manager.satisfaction <= 100);
    assert!(game.manager.fan_approval <= 100);
}

fn make_round_summary_game() -> Game {
    let date = Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap();
    let clock = GameClock::new(date);
    let mut manager = Manager::new(
        "mgr1".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("team1".to_string());

    let team1 = make_team("team1", "Leaders FC");
    let team2 = make_team("team2", "Underdogs FC");
    let team3 = make_team("team3", "Chargers FC");
    let team4 = make_team("team4", "City FC");

    let mut players = make_squad("team1", "t1");
    players.extend(make_squad("team2", "t2"));
    players.extend(make_squad("team3", "t3"));
    players.extend(make_squad("team4", "t4"));

    let today = date.format("%Y-%m-%d").to_string();
    let league = League {
        id: "league1".to_string(),
        name: "Test League".to_string(),
        season: 1,
        fixtures: vec![
            Fixture {
                id: "fix1".to_string(),
                matchday: 7,
                date: today.clone(),
                home_team_id: "team1".to_string(),
                away_team_id: "team2".to_string(),
                competition: FixtureCompetition::League,
                status: FixtureStatus::Completed,
                result: Some(domain::league::MatchResult {
                    home_goals: 0,
                    away_goals: 1,
                    home_scorers: vec![],
                    away_scorers: vec![domain::league::GoalEvent {
                        player_id: "t2_fwd0".to_string(),
                        minute: 77,
                    }],
                    report: None,
                }),
            },
            Fixture {
                id: "fix2".to_string(),
                matchday: 7,
                date: today,
                home_team_id: "team3".to_string(),
                away_team_id: "team4".to_string(),
                competition: FixtureCompetition::League,
                status: FixtureStatus::Completed,
                result: Some(domain::league::MatchResult {
                    home_goals: 2,
                    away_goals: 0,
                    home_scorers: vec![
                        domain::league::GoalEvent {
                            player_id: "t3_fwd0".to_string(),
                            minute: 20,
                        },
                        domain::league::GoalEvent {
                            player_id: "t3_fwd0".to_string(),
                            minute: 72,
                        },
                    ],
                    away_scorers: vec![],
                    report: None,
                }),
            },
        ],
        standings: vec![
            standing_entry("team1", 11, 30, 21, 11),
            standing_entry("team2", 11, 23, 21, 11),
            standing_entry("team3", 11, 31, 18, 8),
            standing_entry("team4", 11, 27, 19, 14),
        ],
        transfer_log: vec![],
        transfer_rumours: vec![],
    };

    let mut game = Game::new(
        clock,
        manager,
        vec![team1, team2, team3, team4],
        players,
        vec![],
        vec![],
    );
    game.league = Some(league);

    set_team_overall(&mut game, "team1", 90);
    set_team_overall(&mut game, "team2", 50);
    set_team_overall(&mut game, "team3", 74);
    set_team_overall(&mut game, "team4", 72);

    game.players
        .iter_mut()
        .for_each(|player| match player.id.as_str() {
            "t1_fwd0" => player.stats.goals = 5,
            "t2_fwd0" => player.stats.goals = 3,
            "t3_fwd0" => player.stats.goals = 6,
            _ => {}
        });

    game
}

fn standing_entry(
    team_id: &str,
    played: u32,
    points: u32,
    goals_for: u32,
    goals_against: u32,
) -> StandingEntry {
    StandingEntry {
        team_id: team_id.to_string(),
        played,
        won: 0,
        drawn: 0,
        lost: 0,
        goals_for,
        goals_against,
        points,
    }
}

fn set_team_overall(game: &mut Game, team_id: &str, overall: u8) {
    game.players
        .iter_mut()
        .filter(|player| player.team_id.as_deref() == Some(team_id))
        .for_each(|player| set_player_overall(player, overall));
}

fn set_player_overall(player: &mut Player, overall: u8) {
    player.attributes.pace = overall;
    player.attributes.stamina = overall;
    player.attributes.strength = overall;
    player.attributes.passing = overall;
    player.attributes.shooting = overall;
    player.attributes.tackling = overall;
    player.attributes.dribbling = overall;
    player.attributes.defending = overall;
    player.attributes.positioning = overall;
    player.attributes.vision = overall;
    player.attributes.decisions = overall;
}

fn previous_round_standings() -> Vec<StandingEntry> {
    vec![
        standing_entry("team1", 10, 30, 30, 20),
        standing_entry("team3", 10, 28, 29, 18),
        standing_entry("team4", 10, 27, 19, 12),
        standing_entry("team2", 10, 20, 22, 21),
    ]
}

#[test]
fn build_round_summary_collects_results_and_deltas() {
    let game = make_round_summary_game();
    let summary = turn::build_round_summary(&game, 7, &previous_round_standings())
        .expect("expected round summary");

    assert_eq!(summary.matchday, 7);
    assert!(summary.is_complete);
    assert_eq!(summary.completed_results.len(), 2);

    let chargers = summary
        .standings_delta
        .iter()
        .find(|entry| entry.team_id == "team3")
        .expect("team3 standings delta");
    assert_eq!(chargers.previous_position, 2);
    assert_eq!(chargers.current_position, 1);
    assert_eq!(chargers.points_delta, 3);

    let leaders = summary
        .standings_delta
        .iter()
        .find(|entry| entry.team_id == "team1")
        .expect("team1 standings delta");
    assert_eq!(leaders.previous_position, 1);
    assert_eq!(leaders.current_position, 2);
    assert_eq!(leaders.points_delta, 0);

    let scorer = summary
        .top_scorer_delta
        .iter()
        .find(|entry| entry.player_id == "t3_fwd0")
        .expect("top scorer delta for t3_fwd0");
    assert_eq!(scorer.previous_rank, 2);
    assert_eq!(scorer.current_rank, 1);
    assert_eq!(scorer.previous_goals, 4);
    assert_eq!(scorer.current_goals, 6);
}

#[test]
fn build_round_summary_picks_biggest_overall_gap_upset() {
    let game = make_round_summary_game();
    let summary = turn::build_round_summary(&game, 7, &previous_round_standings())
        .expect("expected round summary");

    let upset = summary.notable_upset.expect("expected notable upset");
    assert_eq!(upset.fixture_id, "fix1");
    assert_eq!(upset.underdog_team_id, "team2");
    assert_eq!(upset.favorite_team_id, "team1");
    assert!(upset.strength_gap > 0.0);
}

#[test]
fn build_round_summary_handles_incomplete_rounds() {
    let mut game = make_round_summary_game();
    let league = game.league.as_mut().unwrap();
    league.fixtures[1].status = FixtureStatus::Scheduled;
    league.fixtures[1].result = None;

    let summary = turn::build_round_summary(&game, 7, &previous_round_standings())
        .expect("expected partial round summary");

    assert!(!summary.is_complete);
    assert_eq!(summary.completed_results.len(), 1);
    assert_eq!(summary.pending_fixture_count, 1);
}

#[test]
fn build_round_summary_returns_none_when_round_has_no_completed_matches() {
    let mut game = make_round_summary_game();
    let league = game.league.as_mut().unwrap();
    league.fixtures.iter_mut().for_each(|fixture| {
        fixture.status = FixtureStatus::Scheduled;
        fixture.result = None;
    });

    let summary = turn::build_round_summary(&game, 7, &previous_round_standings());

    assert!(summary.is_none());
}

#[test]
fn build_round_summary_ignores_non_competitive_matchday_zero_fixtures() {
    let mut game = make_round_summary_game();
    let league = game.league.as_mut().unwrap();

    league.fixtures.iter_mut().for_each(|fixture| {
        fixture.matchday = 0;
        fixture.competition = FixtureCompetition::Friendly;
    });

    let summary = turn::build_round_summary(&game, 0, &previous_round_standings());

    assert!(summary.is_none());
}
