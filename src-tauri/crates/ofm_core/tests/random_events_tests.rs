use chrono::{TimeZone, Utc};
use domain::league::{
    Fixture, FixtureCompetition, FixtureStatus, League, MatchResult, StandingEntry,
};
use domain::manager::Manager;
use domain::message::{
    ActionOption, ActionType, InboxMessage, MessageAction, MessageCategory, MessageContext,
    MessagePriority,
};
use domain::player::{Player, PlayerAttributes, Position};
use domain::team::{SponsorshipBonusCriterion, Team};
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::random_events::{apply_event_response, check_random_events, rival_interest_weight};
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

fn make_player(id: &str, name: &str, team_id: &str) -> Player {
    let mut p = Player::new(
        id.to_string(),
        name.to_string(),
        name.to_string(),
        "1995-01-01".to_string(),
        "England".to_string(),
        Position::Midfielder,
        default_attrs(),
    );
    p.team_id = Some(team_id.to_string());
    p.morale = 70;
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

fn make_game() -> Game {
    let clock = GameClock::new(Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap());
    let mut manager = Manager::new(
        "mgr1".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("team1".to_string());

    let team = make_team("team1", "Test FC");
    let players: Vec<Player> = (0..11)
        .map(|i| make_player(&format!("p{}", i), &format!("Player {}", i), "team1"))
        .collect();

    Game::new(clock, manager, vec![team], players, vec![], vec![])
}

fn make_game_with_league() -> Game {
    let mut game = make_game();
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    game.league = Some(League {
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
        standings: vec![StandingEntry::new("team1".to_string())],
        transfer_log: vec![],
    });
    game
}

fn assert_choose_option_keys(message: &InboxMessage) {
    for action in &message.actions {
        if let ActionType::ChooseOption { options } = &action.action_type {
            assert!(
                options.iter().all(|option| option.label_key.is_some()),
                "all choose-option labels should have i18n keys"
            );
            assert!(
                options
                    .iter()
                    .all(|option| option.description_key.is_some()),
                "all choose-option descriptions should have i18n keys"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// check_random_events tests
// ---------------------------------------------------------------------------

#[test]
fn check_random_events_no_crash_basic() {
    let mut game = make_game();
    check_random_events(&mut game);
    // Should not crash; messages may or may not be generated due to randomness
}

#[test]
fn check_random_events_no_team_id_is_noop() {
    let mut game = make_game();
    game.manager.team_id = None;
    let msg_count_before = game.messages.len();
    check_random_events(&mut game);
    assert_eq!(game.messages.len(), msg_count_before);
}

#[test]
fn check_random_events_empty_players_no_crash() {
    let mut game = make_game();
    game.players.clear();
    check_random_events(&mut game);
}

#[test]
fn check_random_events_no_league_no_crash() {
    let mut game = make_game();
    game.league = None;
    check_random_events(&mut game);
}

#[test]
fn check_random_events_with_league_no_crash() {
    let mut game = make_game_with_league();
    check_random_events(&mut game);
}

#[test]
fn check_random_events_does_not_duplicate_messages() {
    let mut game = make_game();
    // Run multiple times on the same day
    for _ in 0..20 {
        check_random_events(&mut game);
    }
    // Message IDs should be unique
    let ids: Vec<String> = game.messages.iter().map(|m| m.id.clone()).collect();
    let unique: std::collections::HashSet<String> = ids.iter().cloned().collect();
    assert_eq!(ids.len(), unique.len(), "No duplicate message IDs");
}

#[test]
fn check_random_events_generates_messages_over_many_days() {
    let mut game = make_game();
    // Run for many simulated days to exercise all event branches
    for _ in 0..500 {
        check_random_events(&mut game);
        game.clock.advance_days(1);
    }
    assert!(
        !game.messages.is_empty(),
        "Over 500 days, at least some messages should be generated"
    );
}

#[test]
fn check_random_events_sponsor_offer_has_correct_structure() {
    let mut game = make_game();
    // Run many days to get a sponsor message
    for _ in 0..2000 {
        check_random_events(&mut game);
        game.clock.advance_days(1);
    }
    let sponsor_msgs: Vec<&InboxMessage> = game
        .messages
        .iter()
        .filter(|m| m.id.starts_with("sponsor_"))
        .collect();
    if !sponsor_msgs.is_empty() {
        let msg = sponsor_msgs[0];
        assert_eq!(msg.category, MessageCategory::Finance);
        assert_eq!(msg.priority, MessagePriority::Normal);
        assert!(!msg.actions.is_empty());
        assert!(msg.i18n_params.contains_key("amount"));
        assert_choose_option_keys(msg);
    }
}

#[test]
fn check_random_events_training_injury_applies_injury() {
    let mut game = make_game();
    // Remove the league so no match-day exclusion applies
    game.league = None;
    // Run many days
    for _ in 0..2000 {
        check_random_events(&mut game);
        game.clock.advance_days(1);
    }
    let injury_msgs: Vec<&InboxMessage> = game
        .messages
        .iter()
        .filter(|m| m.id.starts_with("training_injury_"))
        .collect();
    if !injury_msgs.is_empty() {
        // At least one player should have an injury
        let injured = game.players.iter().any(|p| p.injury.is_some());
        assert!(
            injured,
            "A training injury event should apply the injury to the player"
        );
    }
}

#[test]
fn check_random_events_media_story_affects_morale() {
    let mut game = make_game();
    // Set all players to morale 70
    for p in &mut game.players {
        p.morale = 70;
    }
    for _ in 0..2000 {
        check_random_events(&mut game);
        game.clock.advance_days(1);
    }
    let media_msgs: Vec<&InboxMessage> = game
        .messages
        .iter()
        .filter(|m| m.id.starts_with("media_"))
        .collect();
    if !media_msgs.is_empty() {
        // At least one player's morale should have changed
        let morale_changed = game.players.iter().any(|p| p.morale != 70);
        assert!(morale_changed, "Media stories should affect player morale");
    }
}

#[test]
fn check_random_events_board_confidence_triggers_on_losses() {
    let mut game = make_game();
    let _today = game.clock.current_date.format("%Y-%m-%d").to_string();
    // Set up 3 consecutive losses
    game.league = Some(League {
        id: "league1".to_string(),
        name: "Test League".to_string(),
        season: 1,
        fixtures: vec![
            Fixture {
                id: "f1".to_string(),
                matchday: 1,
                date: "2025-06-01".to_string(),
                home_team_id: "team1".to_string(),
                away_team_id: "opp1".to_string(),
                competition: FixtureCompetition::League,
                status: FixtureStatus::Completed,
                result: Some(MatchResult {
                    home_goals: 0,
                    away_goals: 2,
                    home_scorers: vec![],
                    away_scorers: vec![],
                    report: None,
                }),
            },
            Fixture {
                id: "f2".to_string(),
                matchday: 2,
                date: "2025-06-05".to_string(),
                home_team_id: "opp2".to_string(),
                away_team_id: "team1".to_string(),
                competition: FixtureCompetition::League,
                status: FixtureStatus::Completed,
                result: Some(MatchResult {
                    home_goals: 3,
                    away_goals: 1,
                    home_scorers: vec![],
                    away_scorers: vec![],
                    report: None,
                }),
            },
            Fixture {
                id: "f3".to_string(),
                matchday: 3,
                date: "2025-06-10".to_string(),
                home_team_id: "team1".to_string(),
                away_team_id: "opp3".to_string(),
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
        ],
        standings: vec![],
        transfer_log: vec![],
    });

    check_random_events(&mut game);

    let board_msgs: Vec<&InboxMessage> = game
        .messages
        .iter()
        .filter(|m| m.id.starts_with("board_confidence_"))
        .collect();
    assert_eq!(
        board_msgs.len(),
        1,
        "Board confidence message should trigger after 3 losses"
    );
    // Use the most recent lost match as the msg id
    assert_eq!(board_msgs[0].id, "board_confidence_2025-06-10");
    // Date of the message should still be game date
    assert_eq!(board_msgs[0].date, _today);
    assert_eq!(board_msgs[0].category, MessageCategory::BoardDirective);
    assert_eq!(board_msgs[0].priority, MessagePriority::Urgent);
    assert_choose_option_keys(board_msgs[0]);
}

#[test]
fn check_random_events_board_confidence_no_trigger_without_losses() {
    let mut game = make_game();
    // Set up 3 wins
    game.league = Some(League {
        id: "league1".to_string(),
        name: "Test League".to_string(),
        season: 1,
        fixtures: vec![
            Fixture {
                id: "f1".to_string(),
                matchday: 1,
                date: "2025-06-01".to_string(),
                home_team_id: "team1".to_string(),
                away_team_id: "opp1".to_string(),
                competition: FixtureCompetition::League,
                status: FixtureStatus::Completed,
                result: Some(MatchResult {
                    home_goals: 3,
                    away_goals: 0,
                    home_scorers: vec![],
                    away_scorers: vec![],
                    report: None,
                }),
            },
            Fixture {
                id: "f2".to_string(),
                matchday: 2,
                date: "2025-06-05".to_string(),
                home_team_id: "team1".to_string(),
                away_team_id: "opp2".to_string(),
                competition: FixtureCompetition::League,
                status: FixtureStatus::Completed,
                result: Some(MatchResult {
                    home_goals: 2,
                    away_goals: 1,
                    home_scorers: vec![],
                    away_scorers: vec![],
                    report: None,
                }),
            },
            Fixture {
                id: "f3".to_string(),
                matchday: 3,
                date: "2025-06-10".to_string(),
                home_team_id: "team1".to_string(),
                away_team_id: "opp3".to_string(),
                competition: FixtureCompetition::League,
                status: FixtureStatus::Completed,
                result: Some(MatchResult {
                    home_goals: 1,
                    away_goals: 0,
                    home_scorers: vec![],
                    away_scorers: vec![],
                    report: None,
                }),
            },
        ],
        standings: vec![],
        transfer_log: vec![],
    });

    check_random_events(&mut game);
    let board_msgs = game
        .messages
        .iter()
        .filter(|m| m.id.starts_with("board_confidence_"))
        .count();
    assert_eq!(
        board_msgs, 0,
        "Board confidence should not trigger with wins"
    );
}

#[test]
fn check_random_events_international_callup_with_upcoming_match() {
    let mut game = make_game();
    // Set up a match 3 days from now
    let future_date = (game.clock.current_date + chrono::Duration::days(3))
        .format("%Y-%m-%d")
        .to_string();
    game.league = Some(League {
        id: "league1".to_string(),
        name: "Test League".to_string(),
        season: 1,
        fixtures: vec![Fixture {
            id: "fix1".to_string(),
            matchday: 1,
            date: future_date,
            home_team_id: "team1".to_string(),
            away_team_id: "opp1".to_string(),
            competition: FixtureCompetition::League,
            status: FixtureStatus::Scheduled,
            result: None,
        }],
        standings: vec![],
        transfer_log: vec![],
    });

    // Run many times to trigger the 5% chance
    for _ in 0..2000 {
        check_random_events(&mut game);
        game.clock.advance_days(1);
        // Reset league fixture to always be upcoming
        if let Some(league) = &mut game.league {
            let future = (game.clock.current_date + chrono::Duration::days(3))
                .format("%Y-%m-%d")
                .to_string();
            league.fixtures[0].date = future;
        }
    }
    let callup_msgs = game
        .messages
        .iter()
        .filter(|m| m.id.starts_with("intl_callup_"))
        .count();
    assert!(
        callup_msgs > 0,
        "Should generate international call-up messages over many days"
    );
}

#[test]
fn check_random_events_mood_report_structure() {
    let mut game = make_game();
    for _ in 0..200 {
        check_random_events(&mut game);
        game.clock.advance_days(1);
    }
    let mood_msgs: Vec<&InboxMessage> = game
        .messages
        .iter()
        .filter(|m| m.id.starts_with("mood_report_"))
        .collect();
    if !mood_msgs.is_empty() {
        let msg = mood_msgs[0];
        assert_eq!(msg.category, MessageCategory::PlayerMorale);
        assert!(msg.i18n_params.contains_key("mood"));
        assert!(msg.i18n_params.contains_key("avgMorale"));
        assert!(msg.i18n_params.contains_key("highCount"));
        assert!(msg.i18n_params.contains_key("lowCount"));
        assert!(msg.i18n_params.contains_key("total"));
    }
}

#[test]
fn check_random_events_fan_petition_structure() {
    let mut game = make_game();
    for _ in 0..2000 {
        check_random_events(&mut game);
        game.clock.advance_days(1);
    }
    let fan_msgs: Vec<&InboxMessage> = game
        .messages
        .iter()
        .filter(|m| m.id.starts_with("fan_petition_"))
        .collect();
    if !fan_msgs.is_empty() {
        let msg = fan_msgs[0];
        assert_eq!(msg.category, MessageCategory::Media);
        assert!(!msg.actions.is_empty());
        assert_choose_option_keys(msg);
    }
}

#[test]
fn check_random_events_rival_interest_structure() {
    let mut game = make_game();
    for _ in 0..2000 {
        check_random_events(&mut game);
        game.clock.advance_days(1);
    }
    let rival_msgs: Vec<&InboxMessage> = game
        .messages
        .iter()
        .filter(|m| m.id.starts_with("rival_interest_"))
        .collect();
    if !rival_msgs.is_empty() {
        let msg = rival_msgs[0];
        assert_eq!(msg.category, MessageCategory::Transfer);
        assert!(msg.context.player_id.is_some());
        assert_choose_option_keys(msg);
    }
}

#[test]
fn expiring_contract_players_draw_more_rival_interest() {
    let current_date = Utc
        .with_ymd_and_hms(2025, 6, 15, 12, 0, 0)
        .unwrap()
        .date_naive();

    let mut expiring_player = make_player("expiring", "Expiring Star", "team1");
    expiring_player.contract_end = Some("2025-08-01".to_string());

    let mut secure_player = make_player("secure", "Secure Squad", "team1");
    secure_player.contract_end = Some("2028-06-30".to_string());

    let expiring_weight = rival_interest_weight(&expiring_player, current_date);
    let secure_weight = rival_interest_weight(&secure_player, current_date);

    assert!(
        expiring_weight > secure_weight,
        "Players with expiring deals should attract more rival interest"
    );
}

#[test]
fn check_random_events_community_event_structure() {
    let mut game = make_game();
    for _ in 0..2000 {
        check_random_events(&mut game);
        game.clock.advance_days(1);
    }
    let comm_msgs: Vec<&InboxMessage> = game
        .messages
        .iter()
        .filter(|m| m.id.starts_with("community_"))
        .collect();
    if !comm_msgs.is_empty() {
        let msg = comm_msgs[0];
        assert_eq!(msg.category, MessageCategory::System);
        assert_eq!(msg.priority, MessagePriority::Low);
    }
}

// ---------------------------------------------------------------------------
// apply_event_response tests
// ---------------------------------------------------------------------------

fn sponsor_message(amount: u64) -> InboxMessage {
    let mut params = HashMap::new();
    params.insert("amount".to_string(), amount.to_string());
    params.insert("sponsor".to_string(), "Test Sponsor".to_string());
    InboxMessage {
        id: "sponsor_2025-06-15".to_string(),
        subject: "Sponsorship Offer".to_string(),
        body: "Test sponsor offer".to_string(),
        sender: "Commercial Director".to_string(),
        sender_role: "Commercial Director".to_string(),
        date: "2025-06-15".to_string(),
        read: false,
        category: MessageCategory::Finance,
        priority: MessagePriority::Normal,
        actions: vec![MessageAction {
            id: "respond".to_string(),
            label: "Respond".to_string(),
            action_type: ActionType::ChooseOption {
                options: vec![
                    ActionOption {
                        id: "accept".to_string(),
                        label: "Accept".to_string(),
                        description: "Accept the deal".to_string(),
                        label_key: None,
                        description_key: None,
                    },
                    ActionOption {
                        id: "decline".to_string(),
                        label: "Decline".to_string(),
                        description: "Decline the offer".to_string(),
                        label_key: None,
                        description_key: None,
                    },
                ],
            },
            resolved: false,
            label_key: None,
        }],
        context: MessageContext::default(),
        subject_key: None,
        body_key: None,
        sender_key: None,
        sender_role_key: None,
        i18n_params: params,
    }
}

#[test]
fn apply_sponsor_accept_adds_finance() {
    let mut game = make_game();
    let initial_finance = game.teams[0].finance;
    game.messages.push(sponsor_message(100_000));

    let result = apply_event_response(&mut game, "sponsor_2025-06-15", "respond", "accept");

    assert!(result.is_some());
    assert!(result.unwrap().contains("deal signed"));
    assert_eq!(game.teams[0].finance, initial_finance);
    assert_eq!(game.teams[0].season_income, 0);
    let sponsorship = game.teams[0]
        .sponsorship
        .as_ref()
        .expect("accepted sponsor should create active sponsorship state");
    assert_eq!(sponsorship.base_value, 100_000);
    assert_eq!(sponsorship.remaining_weeks, 12);
    assert_eq!(sponsorship.sponsor_name, "Test Sponsor");
    assert!(matches!(
        sponsorship.bonus_criteria.as_slice(),
        [SponsorshipBonusCriterion::UnbeatenRun { .. }]
    ));
    // Actions should be resolved
    let msg = game
        .messages
        .iter()
        .find(|m| m.id == "sponsor_2025-06-15")
        .unwrap();
    assert!(msg.actions.iter().all(|a| a.resolved));
}

#[test]
fn apply_sponsor_decline_no_finance_change() {
    let mut game = make_game();
    let initial_finance = game.teams[0].finance;
    game.messages.push(sponsor_message(100_000));

    let result = apply_event_response(&mut game, "sponsor_2025-06-15", "respond", "decline");

    assert!(result.is_some());
    assert!(result.unwrap().contains("declined"));
    assert_eq!(game.teams[0].finance, initial_finance);
    assert!(game.teams[0].sponsorship.is_none());
    let msg = game
        .messages
        .iter()
        .find(|m| m.id == "sponsor_2025-06-15")
        .unwrap();
    assert!(msg.actions.iter().all(|a| a.resolved));
}

#[test]
fn apply_sponsor_unknown_option_returns_none() {
    let mut game = make_game();
    game.messages.push(sponsor_message(100_000));
    let result = apply_event_response(&mut game, "sponsor_2025-06-15", "respond", "unknown");
    assert!(result.is_none());
}

#[test]
fn apply_sponsor_no_manager_team_returns_none() {
    let mut game = make_game();
    game.manager.team_id = None;
    game.messages.push(sponsor_message(100_000));
    let result = apply_event_response(&mut game, "sponsor_2025-06-15", "respond", "accept");
    assert!(result.is_none());
}

fn board_message() -> InboxMessage {
    InboxMessage {
        id: "board_confidence_2025-06-15".to_string(),
        subject: "Board Meeting".to_string(),
        body: "The board is concerned...".to_string(),
        sender: "Board of Directors".to_string(),
        sender_role: "Chairman".to_string(),
        date: "2025-06-15".to_string(),
        read: false,
        category: MessageCategory::BoardDirective,
        priority: MessagePriority::Urgent,
        actions: vec![MessageAction {
            id: "respond".to_string(),
            label: "Respond".to_string(),
            action_type: ActionType::ChooseOption {
                options: vec![
                    ActionOption {
                        id: "reassure_board".to_string(),
                        label: "Reassure".to_string(),
                        description: "Reassure with a plan".to_string(),
                        label_key: None,
                        description_key: None,
                    },
                    ActionOption {
                        id: "accept_pressure".to_string(),
                        label: "Accept".to_string(),
                        description: "Accept responsibility".to_string(),
                        label_key: None,
                        description_key: None,
                    },
                    ActionOption {
                        id: "blame_circumstances".to_string(),
                        label: "Blame".to_string(),
                        description: "Blame circumstances".to_string(),
                        label_key: None,
                        description_key: None,
                    },
                ],
            },
            resolved: false,
            label_key: None,
        }],
        context: MessageContext::default(),
        subject_key: None,
        body_key: None,
        sender_key: None,
        sender_role_key: None,
        i18n_params: HashMap::new(),
    }
}

#[test]
fn apply_board_reassure_resolves() {
    let mut game = make_game();
    game.messages.push(board_message());
    let result = apply_event_response(
        &mut game,
        "board_confidence_2025-06-15",
        "respond",
        "reassure_board",
    );
    assert!(result.is_some());
    assert!(result.unwrap().contains("reassured"));
    let msg = game
        .messages
        .iter()
        .find(|m| m.id == "board_confidence_2025-06-15")
        .unwrap();
    assert!(msg.actions.iter().all(|a| a.resolved));
}

#[test]
fn apply_board_accept_pressure_resolves() {
    let mut game = make_game();
    game.messages.push(board_message());
    let result = apply_event_response(
        &mut game,
        "board_confidence_2025-06-15",
        "respond",
        "accept_pressure",
    );
    assert!(result.is_some());
    assert!(result.unwrap().contains("honesty"));
}

#[test]
fn apply_board_blame_resolves() {
    let mut game = make_game();
    game.messages.push(board_message());
    let result = apply_event_response(
        &mut game,
        "board_confidence_2025-06-15",
        "respond",
        "blame_circumstances",
    );
    assert!(result.is_some());
    assert!(result.unwrap().contains("wait and see"));
}

#[test]
fn apply_board_unknown_option_returns_none() {
    let mut game = make_game();
    game.messages.push(board_message());
    let result = apply_event_response(
        &mut game,
        "board_confidence_2025-06-15",
        "respond",
        "unknown",
    );
    assert!(result.is_none());
}

fn fan_petition_msg() -> InboxMessage {
    InboxMessage {
        id: "fan_petition_2025-06-15".to_string(),
        subject: "Fan Petition".to_string(),
        body: "Fans want changes".to_string(),
        sender: "Community Manager".to_string(),
        sender_role: "Community Manager".to_string(),
        date: "2025-06-15".to_string(),
        read: false,
        category: MessageCategory::Media,
        priority: MessagePriority::Normal,
        actions: vec![MessageAction {
            id: "respond".to_string(),
            label: "Respond".to_string(),
            action_type: ActionType::ChooseOption { options: vec![] },
            resolved: false,
            label_key: None,
        }],
        context: MessageContext::default(),
        subject_key: None,
        body_key: None,
        sender_key: None,
        sender_role_key: None,
        i18n_params: HashMap::new(),
    }
}

#[test]
fn apply_fan_listen_boosts_morale() {
    let mut game = make_game();
    for p in &mut game.players {
        p.morale = 50;
    }
    game.messages.push(fan_petition_msg());
    let result = apply_event_response(
        &mut game,
        "fan_petition_2025-06-15",
        "respond",
        "listen_fans",
    );
    assert!(result.is_some());
    assert!(result.unwrap().contains("morale improved"));
    // At least some players should have morale > 50 now
    let boosted = game.players.iter().any(|p| p.morale > 50);
    assert!(boosted, "Listening to fans should boost morale");
}

#[test]
fn apply_fan_ignore_resolves() {
    let mut game = make_game();
    game.messages.push(fan_petition_msg());
    let result = apply_event_response(
        &mut game,
        "fan_petition_2025-06-15",
        "respond",
        "ignore_fans",
    );
    assert!(result.is_some());
    assert!(result.unwrap().contains("disappointed"));
}

#[test]
fn apply_fan_address_publicly_resolves() {
    let mut game = make_game();
    game.messages.push(fan_petition_msg());
    let result = apply_event_response(
        &mut game,
        "fan_petition_2025-06-15",
        "respond",
        "address_publicly",
    );
    assert!(result.is_some());
    assert!(result.unwrap().contains("well received"));
}

#[test]
fn apply_fan_unknown_option_returns_none() {
    let mut game = make_game();
    game.messages.push(fan_petition_msg());
    let result = apply_event_response(&mut game, "fan_petition_2025-06-15", "respond", "unknown");
    assert!(result.is_none());
}

fn rival_interest_msg(player_id: &str) -> InboxMessage {
    InboxMessage {
        id: "rival_interest_2025-06-15".to_string(),
        subject: "Transfer Rumour".to_string(),
        body: "A rival is interested".to_string(),
        sender: "Director of Football".to_string(),
        sender_role: "Director of Football".to_string(),
        date: "2025-06-15".to_string(),
        read: false,
        category: MessageCategory::Transfer,
        priority: MessagePriority::Normal,
        actions: vec![MessageAction {
            id: "respond".to_string(),
            label: "Respond".to_string(),
            action_type: ActionType::ChooseOption { options: vec![] },
            resolved: false,
            label_key: None,
        }],
        context: MessageContext {
            player_id: Some(player_id.to_string()),
            ..Default::default()
        },
        subject_key: None,
        body_key: None,
        sender_key: None,
        sender_role_key: None,
        i18n_params: HashMap::new(),
    }
}

#[test]
fn apply_rival_not_for_sale_boosts_morale() {
    let mut game = make_game();
    game.players[0].morale = 50;
    game.messages.push(rival_interest_msg("p0"));
    let result = apply_event_response(
        &mut game,
        "rival_interest_2025-06-15",
        "respond",
        "not_for_sale",
    );
    assert!(result.is_some());
    assert!(result.unwrap().contains("valued"));
    assert!(game.players[0].morale > 50, "Morale should increase");
}

#[test]
fn apply_rival_open_to_offers_drops_morale() {
    let mut game = make_game();
    game.players[0].morale = 80;
    game.messages.push(rival_interest_msg("p0"));
    let result = apply_event_response(
        &mut game,
        "rival_interest_2025-06-15",
        "respond",
        "open_to_offers",
    );
    assert!(result.is_some());
    assert!(result.unwrap().contains("unsettled"));
    assert!(game.players[0].morale < 80, "Morale should decrease");
}

#[test]
fn apply_rival_no_comment_resolves() {
    let mut game = make_game();
    game.messages.push(rival_interest_msg("p0"));
    let result = apply_event_response(
        &mut game,
        "rival_interest_2025-06-15",
        "respond",
        "no_comment",
    );
    assert!(result.is_some());
    assert!(result.unwrap().contains("rumour mill"));
}

#[test]
fn apply_rival_unknown_option_returns_none() {
    let mut game = make_game();
    game.messages.push(rival_interest_msg("p0"));
    let result = apply_event_response(&mut game, "rival_interest_2025-06-15", "respond", "unknown");
    assert!(result.is_none());
}

#[test]
fn apply_unknown_message_type_returns_none() {
    let mut game = make_game();
    let result = apply_event_response(&mut game, "unknown_msg", "respond", "accept");
    assert!(result.is_none());
}

// ---------------------------------------------------------------------------
// Edge cases for check_random_events
// ---------------------------------------------------------------------------

#[test]
fn training_injury_skipped_on_match_day() {
    let mut game = make_game_with_league();
    // Today has a scheduled match, so training injuries should not occur
    let initial_injuries = game.players.iter().filter(|p| p.injury.is_some()).count();
    // Run 100 times on the same match day
    for _ in 0..100 {
        // Reset messages to allow re-check
        game.messages
            .retain(|m| !m.id.starts_with("training_injury_"));
        check_random_events(&mut game);
    }
    let final_injuries = game.players.iter().filter(|p| p.injury.is_some()).count();
    assert_eq!(
        initial_injuries, final_injuries,
        "No training injuries on match day"
    );
}

#[test]
fn already_injured_players_excluded_from_training_injury() {
    let mut game = make_game();
    game.league = None;
    // Injure all players
    for p in &mut game.players {
        p.injury = Some(domain::player::Injury {
            name: "Existing injury".to_string(),
            days_remaining: 5,
        });
    }
    for _ in 0..500 {
        check_random_events(&mut game);
        game.clock.advance_days(1);
    }
    let injury_msgs = game
        .messages
        .iter()
        .filter(|m| m.id.starts_with("training_injury_"))
        .count();
    assert_eq!(
        injury_msgs, 0,
        "Already injured players should not get training injuries"
    );
}

#[test]
fn mood_report_categorizes_morale_levels() {
    // Test with different average morale levels to exercise all branches
    let mut game = make_game();

    // High morale (>= 75) -> "Excellent"
    for p in &mut game.players {
        p.morale = 90;
    }
    for _ in 0..100 {
        check_random_events(&mut game);
        game.clock.advance_days(1);
    }
    let mood_msgs: Vec<&InboxMessage> = game
        .messages
        .iter()
        .filter(|m| m.id.starts_with("mood_report_"))
        .collect();
    if !mood_msgs.is_empty() {
        assert_eq!(
            mood_msgs[0].i18n_params.get("mood").map(|s| s.as_str()),
            Some("common.moods.excellent")
        );
    }
}

#[test]
fn mood_report_low_morale() {
    let mut game = make_game();
    for p in &mut game.players {
        p.morale = 30;
    }
    for _ in 0..100 {
        check_random_events(&mut game);
        game.clock.advance_days(1);
    }
    let mood_msgs: Vec<&InboxMessage> = game
        .messages
        .iter()
        .filter(|m| m.id.starts_with("mood_report_"))
        .collect();
    if !mood_msgs.is_empty() {
        assert_eq!(
            mood_msgs[0].i18n_params.get("mood").map(|s| s.as_str()),
            Some("common.moods.poor")
        );
        assert_eq!(mood_msgs[0].priority, MessagePriority::High);
    }
}

#[test]
fn check_random_events_all_message_types_generated() {
    // Run for a very long time to exercise all event types
    let mut game = make_game();
    game.league = None; // No league to simplify

    for _ in 0..5000 {
        check_random_events(&mut game);
        game.clock.advance_days(1);
    }

    let has_sponsor = game.messages.iter().any(|m| m.id.starts_with("sponsor_"));
    let has_media = game.messages.iter().any(|m| m.id.starts_with("media_"));
    let has_community = game.messages.iter().any(|m| m.id.starts_with("community_"));
    let has_mood = game
        .messages
        .iter()
        .any(|m| m.id.starts_with("mood_report_"));
    let has_fan = game
        .messages
        .iter()
        .any(|m| m.id.starts_with("fan_petition_"));
    let has_rival = game
        .messages
        .iter()
        .any(|m| m.id.starts_with("rival_interest_"));
    let has_injury = game
        .messages
        .iter()
        .any(|m| m.id.starts_with("training_injury_"));

    assert!(
        has_sponsor,
        "Should generate sponsor messages over 5000 days"
    );
    assert!(has_media, "Should generate media messages over 5000 days");
    assert!(
        has_community,
        "Should generate community messages over 5000 days"
    );
    assert!(
        has_mood,
        "Should generate mood report messages over 5000 days"
    );
    assert!(
        has_fan,
        "Should generate fan petition messages over 5000 days"
    );
    assert!(
        has_rival,
        "Should generate rival interest messages over 5000 days"
    );
    assert!(
        has_injury,
        "Should generate training injury messages over 5000 days"
    );
}

// ---------------------------------------------------------------------------
// Fitness affects injury probability
// ---------------------------------------------------------------------------

#[test]
fn unfit_players_get_more_training_injuries() {
    // Players with very low fitness should be injured more often than peak-fitness players.
    // We run many simulated days and compare injury message counts.

    fn run_days_and_count_injury_msgs(fitness: u8) -> usize {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2025, 1, 1, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "mgr1".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("team1".to_string());
        let team = Team::new(
            "team1".to_string(),
            "Test FC".to_string(),
            "TFC".to_string(),
            "England".to_string(),
            "London".to_string(),
            "Stadium".to_string(),
            40_000,
        );
        // Single player so all picks go to them
        let mut player = Player::new(
            "p1".to_string(),
            "TestPlayer".to_string(),
            "TestPlayer".to_string(),
            "1995-01-01".to_string(),
            "England".to_string(),
            Position::Midfielder,
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
            },
        );
        player.team_id = Some("team1".to_string());
        player.fitness = fitness;

        let mut game = Game::new(clock, manager, vec![team], vec![player], vec![], vec![]);

        let mut injury_count = 0;
        for _ in 0..3000 {
            let before = game.players[0].injury.is_some();
            check_random_events(&mut game);
            let after = game.players[0].injury.is_some();
            if !before && after {
                injury_count += 1;
                // Clear injury so player is eligible again
                game.players[0].injury = None;
            }
            // Clear injury messages so the ID dedup doesn't block future events
            game.messages
                .retain(|m| !m.id.starts_with("training_injury_"));
            game.clock.advance_days(1);
        }
        injury_count
    }

    let unfit_injuries = run_days_and_count_injury_msgs(20);
    let peak_injuries = run_days_and_count_injury_msgs(95);

    assert!(
        unfit_injuries > peak_injuries,
        "Unfit players ({} injuries) should be injured more than peak-fitness players ({} injuries)",
        unfit_injuries,
        peak_injuries
    );
}
