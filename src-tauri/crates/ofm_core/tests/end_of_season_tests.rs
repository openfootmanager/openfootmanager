use chrono::{TimeZone, Utc};
use domain::league::{
    Fixture, FixtureCompetition, FixtureStatus, League, MatchResult, StandingEntry,
};
use domain::manager::Manager;
use domain::player::{Player, PlayerAttributes, PlayerSeasonStats, Position};
use domain::team::{FinancialTransactionKind, Team};
use ofm_core::clock::GameClock;
use ofm_core::end_of_season::{is_season_complete, process_end_of_season};
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
        }),
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

// ---------------------------------------------------------------------------
// is_season_complete
// ---------------------------------------------------------------------------

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
                }),
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
                }),
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
    let veteran = game.players.iter_mut().find(|player| player.id == "p1").unwrap();
    veteran.date_of_birth = "1988-01-01".to_string();
    veteran.contract_end = Some("2026-05-01".to_string());
    veteran.attributes.pace = 16;
    veteran.attributes.passing = 70;

    process_end_of_season(&mut game);

    let veteran = game.players.iter().find(|player| player.id == "p1").unwrap();
    assert!(veteran.retired, "older out-of-contract veterans should retire");
    assert_eq!(veteran.team_id, None, "retired players should leave their club");
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
    assert_eq!(league.season, 2, "Should be season 2");
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
    assert_eq!(msg.subject_key.as_deref(), Some("be.msg.seasonReview.subject"));
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
    assert_eq!(msg.subject_key.as_deref(), Some("be.msg.newSeasonSchedule.subject"));
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
    assert_eq!(msg.subject_key.as_deref(), Some("be.msg.seasonPayout.subject"));
    assert_eq!(msg.body_key.as_deref(), Some("be.msg.seasonPayout.body"));
    assert_eq!(msg.sender_key.as_deref(), Some("be.sender.boardOfDirectors"));
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
