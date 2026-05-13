use chrono::{TimeZone, Utc};
use domain::league::{
    Fixture, FixtureCompetition, FixtureStatus, GoalEvent, League, MatchResult, StandingEntry,
};
use domain::manager::Manager;
use domain::message::{ActionOption, ActionType, MessageAction, MessageContext};
use domain::player::{
    Player, PlayerAttributes, PlayerIssue, PlayerIssueCategory, PlayerMoraleCore, PlayerPromise,
    PlayerPromiseKind, Position, RenewalSessionOutcome, RenewalSessionStatus,
};
use domain::team::Team;
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::player_events;

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

fn make_player(id: &str, name: &str, team_id: &str, pos: Position) -> Player {
    let mut p = Player::new(
        id.to_string(),
        name.to_string(),
        name.to_string(),
        "1995-01-01".to_string(),
        "England".to_string(),
        pos,
        default_attrs(),
    );
    p.team_id = Some(team_id.to_string());
    p.morale = 70;
    p.condition = 90;
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
    let mut players = Vec::new();
    // GK + 4 DEF + 4 MID + 2 FWD
    players.push(make_player("p_gk", "GK", "team1", Position::Goalkeeper));
    for i in 0..4 {
        players.push(make_player(
            &format!("p_def{}", i),
            &format!("Def{}", i),
            "team1",
            Position::Defender,
        ));
    }
    for i in 0..4 {
        players.push(make_player(
            &format!("p_mid{}", i),
            &format!("Mid{}", i),
            "team1",
            Position::Midfielder,
        ));
    }
    for i in 0..2 {
        players.push(make_player(
            &format!("p_fwd{}", i),
            &format!("Fwd{}", i),
            "team1",
            Position::Forward,
        ));
    }

    Game::new(clock, manager, vec![team1], players, vec![], vec![])
}

/// Helper: construct a player event message with a specific prefix and player context
fn inject_player_message(game: &mut Game, msg_id: &str, player_id: &str, action_id: &str) {
    use domain::message::InboxMessage;
    let msg = InboxMessage::new(
        msg_id.to_string(),
        "Test".to_string(),
        "Test body".to_string(),
        "Sender".to_string(),
        "2025-06-15".to_string(),
    )
    .with_context(MessageContext {
        player_id: Some(player_id.to_string()),
        ..Default::default()
    })
    .with_action(MessageAction {
        id: action_id.to_string(),
        label: "Respond".to_string(),
        action_type: ActionType::ChooseOption {
            options: vec![ActionOption {
                id: "test".to_string(),
                label: "Test".to_string(),
                description: "Test option".to_string(),
                label_key: None,
                description_key: None,
            }],
        },
        resolved: false,
        label_key: None,
    });
    game.messages.push(msg);
}

// ---------------------------------------------------------------------------
// check_player_events: low morale
// ---------------------------------------------------------------------------

#[test]
fn low_morale_generates_message() {
    let mut game = make_game();
    game.players
        .iter_mut()
        .find(|p| p.id == "p_fwd0")
        .unwrap()
        .morale = 20;

    // Now probabilistic (20% per day), run multiple iterations
    let mut found = false;
    for _ in 0..100 {
        game.messages.clear();
        player_events::check_player_events(&mut game);
        if game.messages.iter().any(|m| m.id == "morale_talk_p_fwd0") {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "Should generate morale talk for low morale player within 100 iterations"
    );
}

#[test]
fn normal_morale_no_message() {
    let mut game = make_game();
    // All players have morale 70
    player_events::check_player_events(&mut game);

    let morale_msgs: Vec<_> = game
        .messages
        .iter()
        .filter(|m| m.id.starts_with("morale_talk_"))
        .collect();
    assert!(
        morale_msgs.is_empty(),
        "No morale talk for players with morale >= 30"
    );
}

#[test]
fn injured_player_no_morale_message() {
    let mut game = make_game();
    let player = game.players.iter_mut().find(|p| p.id == "p_fwd0").unwrap();
    player.morale = 20;
    player.injury = Some(domain::player::Injury {
        name: "Muscle".to_string(),
        days_remaining: 5,
    });

    player_events::check_player_events(&mut game);

    let morale_msgs: Vec<_> = game
        .messages
        .iter()
        .filter(|m| m.id == "morale_talk_p_fwd0")
        .collect();
    assert!(morale_msgs.is_empty(), "No morale talk for injured player");
}

#[test]
fn morale_message_not_duplicated() {
    let mut game = make_game();
    game.players
        .iter_mut()
        .find(|p| p.id == "p_fwd0")
        .unwrap()
        .morale = 20;

    for _ in 0..100 {
        player_events::check_player_events(&mut game);
        if game.messages.iter().any(|m| m.id == "morale_talk_p_fwd0") {
            break;
        }
    }

    let count1 = game
        .messages
        .iter()
        .filter(|m| m.id == "morale_talk_p_fwd0")
        .count();
    assert_eq!(count1, 1, "Should generate exactly one morale message");

    for _ in 0..20 {
        player_events::check_player_events(&mut game);
    }
    let count2 = game
        .messages
        .iter()
        .filter(|m| m.id == "morale_talk_p_fwd0")
        .count();

    assert_eq!(count2, 1, "Should not duplicate morale messages");
}

// ---------------------------------------------------------------------------
// check_player_events: no manager team
// ---------------------------------------------------------------------------

#[test]
fn no_team_no_crash() {
    let mut game = make_game();
    game.manager.team_id = None;
    player_events::check_player_events(&mut game);
    assert!(game.messages.is_empty());
}

// ---------------------------------------------------------------------------
// check_player_events: bench complaints
// ---------------------------------------------------------------------------

#[test]
fn bench_complaint_after_5_missed_matches() {
    let mut game = make_game();
    // Set up league with 5 completed fixtures (new minimum); p_fwd0 has 0 appearances
    let fixtures: Vec<Fixture> = (0..5)
        .map(|i| Fixture {
            id: format!("fix{}", i),
            matchday: i + 1,
            date: format!("2025-06-{:02}", 10 + i),
            home_team_id: "team1".to_string(),
            away_team_id: "team2".to_string(),
            competition: FixtureCompetition::League,
            status: FixtureStatus::Completed,
            result: Some(MatchResult {
                home_goals: 1,
                away_goals: 0,
                home_scorers: vec![GoalEvent {
                    player_id: "p_mid0".to_string(),
                    minute: 45,
                }],
                away_scorers: vec![],
                report: None,
            }),
        })
        .collect();
    let league = League {
        id: "league1".to_string(),
        name: "Test League".to_string(),
        season: 1,
        fixtures,
        standings: vec![StandingEntry::new("team1".to_string())],
        transfer_log: vec![],
    };
    game.league = Some(league);

    // Make p_fwd0 have low morale (< 50), 0 appearances, decent OVR
    let player = game.players.iter_mut().find(|p| p.id == "p_fwd0").unwrap();
    player.morale = 40;
    // stats.appearances defaults to 0, so app_ratio = 0/5 = 0.0 < 0.3

    // Now probabilistic (10% daily chance), run multiple iterations
    let mut found = false;
    for _ in 0..200 {
        game.messages.clear();
        player_events::check_player_events(&mut game);
        if game
            .messages
            .iter()
            .any(|m| m.id.starts_with("bench_complaint_"))
        {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "Should generate bench complaint for player who missed 5+ matches within 200 iterations"
    );
}

#[test]
fn bench_complaint_not_for_gk() {
    let mut game = make_game();
    let fixtures: Vec<Fixture> = (0..5)
        .map(|i| Fixture {
            id: format!("fix{}", i),
            matchday: i + 1,
            date: format!("2025-06-{:02}", 10 + i),
            home_team_id: "team1".to_string(),
            away_team_id: "team2".to_string(),
            competition: FixtureCompetition::League,
            status: FixtureStatus::Completed,
            result: Some(MatchResult {
                home_goals: 0,
                away_goals: 0,
                home_scorers: vec![],
                away_scorers: vec![],
                report: None,
            }),
        })
        .collect();
    game.league = Some(League {
        id: "league1".to_string(),
        name: "Test League".to_string(),
        season: 1,
        fixtures,
        standings: vec![],
        transfer_log: vec![],
    });
    // GK has low morale
    game.players
        .iter_mut()
        .find(|p| p.id == "p_gk")
        .unwrap()
        .morale = 30;

    // Run many times — GKs should never get bench complaints
    for _ in 0..100 {
        game.messages.clear();
        player_events::check_player_events(&mut game);
        let gk_complaint = game.messages.iter().any(|m| m.id == "bench_complaint_p_gk");
        assert!(!gk_complaint, "Goalkeepers shouldn't complain about bench");
    }
}

#[test]
fn bench_complaint_not_with_fewer_than_5_fixtures() {
    let mut game = make_game();
    let fixtures: Vec<Fixture> = (0..4)
        .map(|i| Fixture {
            id: format!("fix{}", i),
            matchday: i + 1,
            date: format!("2025-06-{:02}", 10 + i),
            home_team_id: "team1".to_string(),
            away_team_id: "team2".to_string(),
            competition: FixtureCompetition::League,
            status: FixtureStatus::Completed,
            result: Some(MatchResult {
                home_goals: 0,
                away_goals: 0,
                home_scorers: vec![],
                away_scorers: vec![],
                report: None,
            }),
        })
        .collect();
    game.league = Some(League {
        id: "league1".to_string(),
        name: "Test League".to_string(),
        season: 1,
        fixtures,
        standings: vec![],
        transfer_log: vec![],
    });
    // Set low morale so they would complain if threshold was met
    for p in &mut game.players {
        p.morale = 30;
    }

    player_events::check_player_events(&mut game);

    let bench_msgs: Vec<_> = game
        .messages
        .iter()
        .filter(|m| m.id.starts_with("bench_complaint_"))
        .collect();
    assert!(
        bench_msgs.is_empty(),
        "No bench complaints with fewer than 5 completed fixtures"
    );
}

// ---------------------------------------------------------------------------
// check_player_events: happy player (probabilistic, run many times)
// ---------------------------------------------------------------------------

#[test]
fn happy_player_message_with_high_morale() {
    let mut game = make_game();
    // Set all players to high morale
    for p in &mut game.players {
        p.morale = 95;
    }

    // Run many iterations to hit the 10% chance
    let mut found_happy = false;
    for _ in 0..200 {
        game.messages.clear();
        player_events::check_player_events(&mut game);
        if game
            .messages
            .iter()
            .any(|m| m.id.starts_with("happy_player_"))
        {
            found_happy = true;
            break;
        }
    }
    assert!(
        found_happy,
        "With 11 players at morale 95, should get happy player message in 200 iterations"
    );
}

// ---------------------------------------------------------------------------
// check_player_events: contract concern
// ---------------------------------------------------------------------------

#[test]
fn contract_warning_cadence_changes_by_horizon() {
    let mut twelve_month_game = make_game();
    let twelve_month_end = (twelve_month_game.clock.current_date + chrono::Duration::days(330))
        .format("%Y-%m-%d")
        .to_string();
    twelve_month_game
        .players
        .iter_mut()
        .find(|p| p.id == "p_fwd0")
        .unwrap()
        .contract_end = Some(twelve_month_end);

    player_events::check_player_events(&mut twelve_month_game);

    assert!(
        twelve_month_game
            .messages
            .iter()
            .all(|m| m.id != "contract_concern_p_fwd0_12m"),
        "Should defer per-player contract warnings until six months"
    );

    let mut six_month_game = make_game();
    let six_month_end = (six_month_game.clock.current_date + chrono::Duration::days(150))
        .format("%Y-%m-%d")
        .to_string();
    six_month_game
        .players
        .iter_mut()
        .find(|p| p.id == "p_fwd0")
        .unwrap()
        .contract_end = Some(six_month_end);

    player_events::check_player_events(&mut six_month_game);

    assert!(
        six_month_game
            .messages
            .iter()
            .any(|m| m.id == "contract_concern_p_fwd0_6m"),
        "Should generate a 6-month contract warning"
    );

    let mut three_month_game = make_game();
    let three_month_end = (three_month_game.clock.current_date + chrono::Duration::days(60))
        .format("%Y-%m-%d")
        .to_string();
    three_month_game
        .players
        .iter_mut()
        .find(|p| p.id == "p_fwd0")
        .unwrap()
        .contract_end = Some(three_month_end);

    player_events::check_player_events(&mut three_month_game);

    assert!(
        three_month_game
            .messages
            .iter()
            .any(|m| m.id == "contract_concern_p_fwd0_3m"),
        "Should generate a 3-month contract warning"
    );

    let mut final_weeks_game = make_game();
    let final_weeks_end = (final_weeks_game.clock.current_date + chrono::Duration::days(20))
        .format("%Y-%m-%d")
        .to_string();
    final_weeks_game
        .players
        .iter_mut()
        .find(|p| p.id == "p_fwd0")
        .unwrap()
        .contract_end = Some(final_weeks_end);

    player_events::check_player_events(&mut final_weeks_game);

    assert!(
        final_weeks_game
            .messages
            .iter()
            .any(|m| m.id == "contract_concern_p_fwd0_final"),
        "Should generate a final-weeks contract warning"
    );
}

#[test]
fn contract_pressure_intensifies_as_expiry_approaches() {
    let mut twelve_month_game = make_game();
    let mut final_weeks_game = make_game();

    let twelve_month_player = twelve_month_game
        .players
        .iter_mut()
        .find(|p| p.id == "p_fwd0")
        .unwrap();
    twelve_month_player.morale = 80;
    twelve_month_player.contract_end = Some(
        (twelve_month_game.clock.current_date + chrono::Duration::days(330))
            .format("%Y-%m-%d")
            .to_string(),
    );

    let final_weeks_player = final_weeks_game
        .players
        .iter_mut()
        .find(|p| p.id == "p_fwd0")
        .unwrap();
    final_weeks_player.morale = 80;
    final_weeks_player.contract_end = Some(
        (final_weeks_game.clock.current_date + chrono::Duration::days(20))
            .format("%Y-%m-%d")
            .to_string(),
    );

    player_events::check_player_events(&mut twelve_month_game);
    player_events::check_player_events(&mut final_weeks_game);

    let twelve_month_morale = twelve_month_game
        .players
        .iter()
        .find(|p| p.id == "p_fwd0")
        .unwrap()
        .morale;
    let final_weeks_morale = final_weeks_game
        .players
        .iter()
        .find(|p| p.id == "p_fwd0")
        .unwrap()
        .morale;

    assert!(
        twelve_month_morale == 80,
        "Per-player morale pressure should wait until the six-month window"
    );
    assert!(
        final_weeks_morale < twelve_month_morale,
        "Morale pressure should intensify as expiry gets closer"
    );
}

#[test]
fn takeover_contract_review_replaces_first_day_contract_spam() {
    let mut game = make_game();
    game.players
        .iter_mut()
        .find(|p| p.id == "p_fwd0")
        .unwrap()
        .contract_end = Some(
        (game.clock.current_date + chrono::Duration::days(20))
            .format("%Y-%m-%d")
            .to_string(),
    );
    game.players
        .iter_mut()
        .find(|p| p.id == "p_fwd1")
        .unwrap()
        .contract_end = Some(
        (game.clock.current_date + chrono::Duration::days(150))
            .format("%Y-%m-%d")
            .to_string(),
    );

    player_events::generate_takeover_contract_review_message(&mut game);

    assert!(
        game.messages
            .iter()
            .any(|message| message.id == "contract_review_takeover_team1"),
        "Takeover should seed one contract review message"
    );
    assert_eq!(
        game.messages
            .iter()
            .filter(|message| message.id.starts_with("contract_concern_"))
            .count(),
        0,
        "Takeover should not dump individual contract concern messages into the inbox"
    );
}

#[test]
fn no_contract_concern_beyond_twelve_months() {
    let mut game = make_game();
    let end_date = (game.clock.current_date + chrono::Duration::days(420))
        .format("%Y-%m-%d")
        .to_string();
    game.players
        .iter_mut()
        .find(|p| p.id == "p_fwd0")
        .unwrap()
        .contract_end = Some(end_date);

    player_events::check_player_events(&mut game);

    let contract_msgs: Vec<_> = game
        .messages
        .iter()
        .filter(|m| m.id.starts_with("contract_concern_p_fwd0"))
        .collect();
    assert!(
        contract_msgs.is_empty(),
        "No contract concern when more than 12 months remain"
    );
}

#[test]
fn no_contract_concern_if_expired() {
    let mut game = make_game();
    let end_date = (game.clock.current_date - chrono::Duration::days(10))
        .format("%Y-%m-%d")
        .to_string();
    game.players
        .iter_mut()
        .find(|p| p.id == "p_fwd0")
        .unwrap()
        .contract_end = Some(end_date);

    player_events::check_player_events(&mut game);

    let contract_msgs: Vec<_> = game
        .messages
        .iter()
        .filter(|m| m.id == "contract_concern_p_fwd0")
        .collect();
    assert!(
        contract_msgs.is_empty(),
        "No contract concern if contract already expired"
    );
}

// ---------------------------------------------------------------------------
// apply_player_response: morale_talk responses
// ---------------------------------------------------------------------------

#[test]
fn morale_talk_encourage_boosts_morale() {
    let mut game = make_game();
    game.players
        .iter_mut()
        .find(|p| p.id == "p_fwd0")
        .unwrap()
        .morale = 30;
    inject_player_message(&mut game, "morale_talk_p_fwd0", "p_fwd0", "respond");

    // Run many times to get a statistically valid result
    let mut total_delta: i32 = 0;
    let runs = 50;
    for _ in 0..runs {
        let mut g = game.clone();
        let result = player_events::apply_player_response(
            &mut g,
            "morale_talk_p_fwd0",
            "respond",
            "encourage",
        );
        assert!(result.is_some());
        let p = g.players.iter().find(|p| p.id == "p_fwd0").unwrap();
        total_delta += p.morale as i32 - 30;
    }
    let avg_delta = total_delta as f64 / runs as f64;
    assert!(
        avg_delta > 0.0,
        "Encourage should generally boost morale, avg delta: {:.1}",
        avg_delta
    );
}

#[test]
fn morale_talk_promise_time_big_boost() {
    let mut game = make_game();
    game.players
        .iter_mut()
        .find(|p| p.id == "p_fwd0")
        .unwrap()
        .morale = 30;
    inject_player_message(&mut game, "morale_talk_p_fwd0", "p_fwd0", "respond");

    let mut total_delta: i32 = 0;
    let runs = 30;
    for _ in 0..runs {
        let mut g = game.clone();
        player_events::apply_player_response(
            &mut g,
            "morale_talk_p_fwd0",
            "respond",
            "promise_time",
        );
        let p = g.players.iter().find(|p| p.id == "p_fwd0").unwrap();
        total_delta += p.morale as i32 - 30;
    }
    let avg_delta = total_delta as f64 / runs as f64;
    assert!(
        avg_delta >= 8.0,
        "Promise time should generally remain a strong positive option, avg: {:.1}",
        avg_delta
    );
}

#[test]
fn morale_talk_work_harder_varies() {
    let mut game = make_game();
    game.players
        .iter_mut()
        .find(|p| p.id == "p_fwd0")
        .unwrap()
        .morale = 50;
    inject_player_message(&mut game, "morale_talk_p_fwd0", "p_fwd0", "respond");

    let mut positive_count = 0;
    let mut negative_count = 0;
    for _ in 0..100 {
        let mut g = game.clone();
        player_events::apply_player_response(
            &mut g,
            "morale_talk_p_fwd0",
            "respond",
            "work_harder",
        );
        let p = g.players.iter().find(|p| p.id == "p_fwd0").unwrap();
        if p.morale > 50 {
            positive_count += 1;
        } else if p.morale < 50 {
            negative_count += 1;
        }
    }
    // Should see both positive and negative outcomes
    assert!(
        positive_count > 0 && negative_count > 0,
        "Work harder should have varied outcomes: positive={}, negative={}",
        positive_count,
        negative_count
    );
}

#[test]
fn morale_talk_unknown_option_returns_none() {
    let mut game = make_game();
    inject_player_message(&mut game, "morale_talk_p_fwd0", "p_fwd0", "respond");
    let result = player_events::apply_player_response(
        &mut game,
        "morale_talk_p_fwd0",
        "respond",
        "nonexistent",
    );
    assert!(result.is_none());
}

#[test]
fn unresolved_issue_can_cap_visible_morale_growth() {
    let mut game = make_game();
    let player = game.players.iter_mut().find(|p| p.id == "p_fwd0").unwrap();
    player.morale = 70;
    player.morale_core = PlayerMoraleCore {
        manager_trust: 50,
        unresolved_issue: Some(PlayerIssue {
            category: PlayerIssueCategory::Contract,
            severity: 80,
        }),
        recent_treatment: None,
        pending_promise: None,
        talk_cooldown_until: None,
        renewal_state: None,
    };
    inject_player_message(&mut game, "morale_talk_p_fwd0", "p_fwd0", "respond");

    player_events::apply_player_response(
        &mut game,
        "morale_talk_p_fwd0",
        "respond",
        "promise_time",
    );

    let player = game.players.iter().find(|p| p.id == "p_fwd0").unwrap();
    assert!(player.morale <= 70);
}

#[test]
fn trust_changes_even_when_visible_morale_is_capped() {
    let mut game = make_game();
    let player = game.players.iter_mut().find(|p| p.id == "p_fwd0").unwrap();
    player.morale = 70;
    player.morale_core = PlayerMoraleCore {
        manager_trust: 50,
        unresolved_issue: Some(PlayerIssue {
            category: PlayerIssueCategory::Contract,
            severity: 80,
        }),
        recent_treatment: None,
        pending_promise: None,
        talk_cooldown_until: None,
        renewal_state: None,
    };
    inject_player_message(&mut game, "morale_talk_p_fwd0", "p_fwd0", "respond");

    player_events::apply_player_response(
        &mut game,
        "morale_talk_p_fwd0",
        "respond",
        "promise_time",
    );

    let player = game.players.iter().find(|p| p.id == "p_fwd0").unwrap();
    assert!(player.morale <= 70);
    assert!(player.morale_core.manager_trust > 50);
}

#[test]
fn repeated_treatment_becomes_less_effective_over_time() {
    let mut game = make_game();
    let player = game.players.iter_mut().find(|p| p.id == "p_fwd0").unwrap();
    player.morale = 20;
    inject_player_message(&mut game, "morale_talk_p_fwd0", "p_fwd0", "respond");

    player_events::apply_player_response(
        &mut game,
        "morale_talk_p_fwd0",
        "respond",
        "promise_time",
    );
    let after_first = game
        .players
        .iter()
        .find(|p| p.id == "p_fwd0")
        .unwrap()
        .morale_core
        .clone();

    player_events::apply_player_response(
        &mut game,
        "morale_talk_p_fwd0",
        "respond",
        "promise_time",
    );
    let player = game.players.iter().find(|p| p.id == "p_fwd0").unwrap();

    assert!(player.morale_core.recent_treatment.is_some());
    assert_eq!(
        player
            .morale_core
            .recent_treatment
            .as_ref()
            .unwrap()
            .times_recently_used,
        2
    );
    assert!(player.morale_core.manager_trust - after_first.manager_trust < 6);
}

#[test]
fn promise_time_records_a_playing_time_promise() {
    let mut game = make_game();
    inject_player_message(&mut game, "morale_talk_p_fwd0", "p_fwd0", "respond");

    player_events::apply_player_response(
        &mut game,
        "morale_talk_p_fwd0",
        "respond",
        "promise_time",
    );

    let player = game.players.iter().find(|p| p.id == "p_fwd0").unwrap();
    assert_eq!(
        player.morale_core.pending_promise,
        Some(PlayerPromise {
            kind: PlayerPromiseKind::PlayingTime,
            matches_remaining: 1,
        })
    );
}

#[test]
fn recent_player_talk_enters_cooldown_and_blocks_same_day_repeat() {
    let mut game = make_game();
    let player = game.players.iter_mut().find(|p| p.id == "p_fwd0").unwrap();
    player.morale = 20;
    inject_player_message(&mut game, "morale_talk_p_fwd0", "p_fwd0", "respond");

    player_events::apply_player_response(&mut game, "morale_talk_p_fwd0", "respond", "encourage");

    let player = game.players.iter().find(|p| p.id == "p_fwd0").unwrap();
    assert!(player.morale_core.talk_cooldown_until.is_some());

    game.messages.clear();
    for _ in 0..50 {
        player_events::check_player_events(&mut game);
    }

    assert!(
        game.messages
            .iter()
            .all(|message| message.id != "morale_talk_p_fwd0"),
        "cooldown should block immediate repeat morale talks"
    );
}

#[test]
fn weighted_response_bias_changes_with_player_context() {
    let mut volatile = make_player("volatile", "Volatile", "team1", Position::Forward);
    volatile.attributes.aggression = 95;
    volatile.attributes.composure = 20;
    volatile.attributes.leadership = 20;
    volatile.morale_core.manager_trust = 30;

    let mut composed = make_player("composed", "Composed", "team1", Position::Forward);
    composed.attributes.aggression = 20;
    composed.attributes.composure = 95;
    composed.attributes.leadership = 95;
    composed.morale_core.manager_trust = 75;

    let volatile_weights = player_events::build_response_band_weights(
        &volatile,
        "morale_talk_volatile",
        "work_harder",
    );
    let composed_weights = player_events::build_response_band_weights(
        &composed,
        "morale_talk_composed",
        "work_harder",
    );

    let volatile_negative = volatile_weights.strong_negative + volatile_weights.mild_negative;
    let volatile_positive = volatile_weights.strong_positive + volatile_weights.mild_positive;
    let composed_negative = composed_weights.strong_negative + composed_weights.mild_negative;
    let composed_positive = composed_weights.strong_positive + composed_weights.mild_positive;

    assert!(volatile_negative > volatile_positive);
    assert!(composed_positive > composed_negative);
}

#[test]
fn repeated_identical_talk_reduces_positive_weight() {
    let fresh = make_player("fresh", "Fresh", "team1", Position::Forward);

    let mut repeated = make_player("repeated", "Repeated", "team1", Position::Forward);
    repeated.morale_core.recent_treatment = Some(domain::player::RecentTreatmentMemory {
        action_key: "morale_talk:encourage".to_string(),
        times_recently_used: 2,
    });

    let fresh_weights =
        player_events::build_response_band_weights(&fresh, "morale_talk_fresh", "encourage");
    let repeated_weights =
        player_events::build_response_band_weights(&repeated, "morale_talk_repeated", "encourage");

    let fresh_positive = fresh_weights.strong_positive + fresh_weights.mild_positive;
    let repeated_positive = repeated_weights.strong_positive + repeated_weights.mild_positive;

    assert!(repeated_positive < fresh_positive);
}

#[test]
fn deterministic_weighted_band_selection_uses_roll_boundaries() {
    let weights = player_events::ResponseBandWeights {
        strong_positive: 2,
        mild_positive: 3,
        neutral: 2,
        mild_negative: 1,
        strong_negative: 2,
    };

    assert_eq!(
        player_events::pick_response_band(&weights, 0),
        player_events::ResponseOutcomeBand::StrongPositive
    );
    assert_eq!(
        player_events::pick_response_band(&weights, 2),
        player_events::ResponseOutcomeBand::MildPositive
    );
    assert_eq!(
        player_events::pick_response_band(&weights, 5),
        player_events::ResponseOutcomeBand::Neutral
    );
    assert_eq!(
        player_events::pick_response_band(&weights, 7),
        player_events::ResponseOutcomeBand::MildNegative
    );
    assert_eq!(
        player_events::pick_response_band(&weights, 8),
        player_events::ResponseOutcomeBand::StrongNegative
    );
}

// ---------------------------------------------------------------------------
// apply_player_response: bench_complaint responses
// ---------------------------------------------------------------------------

#[test]
fn bench_complaint_explain() {
    let mut game = make_game();
    game.players
        .iter_mut()
        .find(|p| p.id == "p_fwd0")
        .unwrap()
        .morale = 50;
    inject_player_message(&mut game, "bench_complaint_p_fwd0", "p_fwd0", "respond");

    let result = player_events::apply_player_response(
        &mut game,
        "bench_complaint_p_fwd0",
        "respond",
        "explain",
    );
    assert!(result.is_some());
}

#[test]
fn bench_complaint_promise_chance_big_boost() {
    let mut game = make_game();
    game.players
        .iter_mut()
        .find(|p| p.id == "p_fwd0")
        .unwrap()
        .morale = 40;
    inject_player_message(&mut game, "bench_complaint_p_fwd0", "p_fwd0", "respond");

    let mut total_delta: i32 = 0;
    let runs = 30;
    for _ in 0..runs {
        let mut g = game.clone();
        player_events::apply_player_response(
            &mut g,
            "bench_complaint_p_fwd0",
            "respond",
            "promise_chance",
        );
        let p = g.players.iter().find(|p| p.id == "p_fwd0").unwrap();
        total_delta += p.morale as i32 - 40;
    }
    let avg_delta = total_delta as f64 / runs as f64;
    assert!(
        avg_delta >= 8.0,
        "Promise chance should boost 8-14, avg: {:.1}",
        avg_delta
    );
}

#[test]
fn bench_complaint_prove_yourself_varies() {
    let mut game = make_game();
    game.players
        .iter_mut()
        .find(|p| p.id == "p_fwd0")
        .unwrap()
        .morale = 50;
    inject_player_message(&mut game, "bench_complaint_p_fwd0", "p_fwd0", "respond");

    let mut positive = 0;
    let mut negative = 0;
    for _ in 0..100 {
        let mut g = game.clone();
        player_events::apply_player_response(
            &mut g,
            "bench_complaint_p_fwd0",
            "respond",
            "prove_yourself",
        );
        let p = g.players.iter().find(|p| p.id == "p_fwd0").unwrap();
        if p.morale > 50 {
            positive += 1;
        } else if p.morale < 50 {
            negative += 1;
        }
    }
    assert!(
        positive > 0 && negative > 0,
        "Prove yourself should vary: pos={}, neg={}",
        positive,
        negative
    );
}

// ---------------------------------------------------------------------------
// apply_player_response: happy_player responses
// ---------------------------------------------------------------------------

#[test]
fn happy_player_praise_back_boosts() {
    let mut game = make_game();
    game.players
        .iter_mut()
        .find(|p| p.id == "p_fwd0")
        .unwrap()
        .morale = 80;
    inject_player_message(&mut game, "happy_player_p_fwd0", "p_fwd0", "respond");

    let mut total_delta: i32 = 0;
    let runs = 30;
    for _ in 0..runs {
        let mut g = game.clone();
        player_events::apply_player_response(
            &mut g,
            "happy_player_p_fwd0",
            "respond",
            "praise_back",
        );
        let p = g.players.iter().find(|p| p.id == "p_fwd0").unwrap();
        total_delta += p.morale as i32 - 80;
    }
    assert!(
        total_delta > 0,
        "Praise back should boost morale, total delta: {}",
        total_delta
    );
}

#[test]
fn happy_player_stay_professional() {
    let mut game = make_game();
    game.players
        .iter_mut()
        .find(|p| p.id == "p_fwd0")
        .unwrap()
        .morale = 80;
    inject_player_message(&mut game, "happy_player_p_fwd0", "p_fwd0", "respond");

    let result = player_events::apply_player_response(
        &mut game,
        "happy_player_p_fwd0",
        "respond",
        "stay_professional",
    );
    assert!(result.is_some());
}

#[test]
fn happy_player_higher_expectations_varies() {
    let mut game = make_game();
    game.players
        .iter_mut()
        .find(|p| p.id == "p_fwd0")
        .unwrap()
        .morale = 80;
    inject_player_message(&mut game, "happy_player_p_fwd0", "p_fwd0", "respond");

    let mut positive = 0;
    let mut negative = 0;
    for _ in 0..100 {
        let mut g = game.clone();
        player_events::apply_player_response(
            &mut g,
            "happy_player_p_fwd0",
            "respond",
            "higher_expectations",
        );
        let p = g.players.iter().find(|p| p.id == "p_fwd0").unwrap();
        if p.morale > 80 {
            positive += 1;
        } else if p.morale < 80 {
            negative += 1;
        }
    }
    assert!(
        positive > 0 && negative > 0,
        "Higher expectations should vary: pos={}, neg={}",
        positive,
        negative
    );
}

// ---------------------------------------------------------------------------
// apply_player_response: contract_concern responses
// ---------------------------------------------------------------------------

#[test]
fn contract_reassure_boosts_morale() {
    let mut game = make_game();
    game.players
        .iter_mut()
        .find(|p| p.id == "p_fwd0")
        .unwrap()
        .morale = 50;
    inject_player_message(&mut game, "contract_concern_p_fwd0", "p_fwd0", "respond");

    let mut total_delta: i32 = 0;
    let runs = 30;
    for _ in 0..runs {
        let mut g = game.clone();
        player_events::apply_player_response(
            &mut g,
            "contract_concern_p_fwd0",
            "respond",
            "reassure",
        );
        let p = g.players.iter().find(|p| p.id == "p_fwd0").unwrap();
        total_delta += p.morale as i32 - 50;
    }
    let avg = total_delta as f64 / runs as f64;
    assert!(avg >= 4.0, "Reassure should boost 4-10, avg: {:.1}", avg);
}

#[test]
fn contract_reassure_marks_player_open_to_renewal() {
    let mut game = make_game();
    inject_player_message(&mut game, "contract_concern_p_fwd0", "p_fwd0", "respond");

    player_events::apply_player_response(
        &mut game,
        "contract_concern_p_fwd0",
        "respond",
        "reassure",
    );

    let player = game.players.iter().find(|p| p.id == "p_fwd0").unwrap();
    let renewal_state = player
        .morale_core
        .renewal_state
        .as_ref()
        .expect("renewal state should exist");
    assert_eq!(renewal_state.status, RenewalSessionStatus::Open);
    assert_eq!(
        renewal_state.last_outcome,
        Some(RenewalSessionOutcome::Stalled)
    );
}

#[test]
fn contract_noncommittal_generally_negative() {
    let mut game = make_game();
    game.players
        .iter_mut()
        .find(|p| p.id == "p_fwd0")
        .unwrap()
        .morale = 60;
    inject_player_message(&mut game, "contract_concern_p_fwd0", "p_fwd0", "respond");

    let mut total_delta: i32 = 0;
    let runs = 50;
    for _ in 0..runs {
        let mut g = game.clone();
        player_events::apply_player_response(
            &mut g,
            "contract_concern_p_fwd0",
            "respond",
            "noncommittal",
        );
        let p = g.players.iter().find(|p| p.id == "p_fwd0").unwrap();
        total_delta += p.morale as i32 - 60;
    }
    let avg = total_delta as f64 / runs as f64;
    assert!(
        avg < 0.0,
        "Noncommittal should generally be negative, avg: {:.1}",
        avg
    );
}

#[test]
fn contract_no_renewal_tanks_morale() {
    let mut game = make_game();
    game.players
        .iter_mut()
        .find(|p| p.id == "p_fwd0")
        .unwrap()
        .morale = 70;
    inject_player_message(&mut game, "contract_concern_p_fwd0", "p_fwd0", "respond");

    let result = player_events::apply_player_response(
        &mut game,
        "contract_concern_p_fwd0",
        "respond",
        "no_renewal",
    );
    assert!(result.is_some());

    let p = game.players.iter().find(|p| p.id == "p_fwd0").unwrap();
    assert!(
        p.morale < 65,
        "No renewal should tank morale, got {}",
        p.morale
    );
}

#[test]
fn contract_no_renewal_blocks_future_renewal_talks() {
    let mut game = make_game();
    let expected_blocked_until = (game.clock.current_date + chrono::Duration::days(60))
        .format("%Y-%m-%d")
        .to_string();
    inject_player_message(&mut game, "contract_concern_p_fwd0", "p_fwd0", "respond");

    player_events::apply_player_response(
        &mut game,
        "contract_concern_p_fwd0",
        "respond",
        "no_renewal",
    );

    let player = game.players.iter().find(|p| p.id == "p_fwd0").unwrap();
    let renewal_state = player
        .morale_core
        .renewal_state
        .as_ref()
        .expect("renewal state should exist");
    assert_eq!(renewal_state.status, RenewalSessionStatus::Blocked);
    assert_eq!(
        renewal_state.last_outcome,
        Some(RenewalSessionOutcome::BlockedByManager)
    );
    assert_eq!(
        renewal_state.manager_blocked_until.as_deref(),
        Some(expected_blocked_until.as_str())
    );
}

#[test]
fn no_renewal_dressing_room_effect() {
    let mut game = make_game();
    // Set all players to morale 70
    for p in &mut game.players {
        p.morale = 70;
    }
    inject_player_message(&mut game, "contract_concern_p_fwd0", "p_fwd0", "respond");

    player_events::apply_player_response(
        &mut game,
        "contract_concern_p_fwd0",
        "respond",
        "no_renewal",
    );

    // Teammates should also lose morale (dressing room effect)
    let teammates_affected: Vec<_> = game
        .players
        .iter()
        .filter(|p| p.id != "p_fwd0" && p.team_id.as_deref() == Some("team1"))
        .filter(|p| p.morale < 70)
        .collect();
    assert!(
        !teammates_affected.is_empty(),
        "No renewal should affect dressing room morale"
    );
}

// ---------------------------------------------------------------------------
// apply_player_response: edge cases
// ---------------------------------------------------------------------------

#[test]
fn unknown_message_prefix_returns_none() {
    let mut game = make_game();
    inject_player_message(&mut game, "unknown_prefix_p_fwd0", "p_fwd0", "respond");
    let result = player_events::apply_player_response(
        &mut game,
        "unknown_prefix_p_fwd0",
        "respond",
        "encourage",
    );
    assert!(result.is_none());
}

#[test]
fn missing_message_returns_none() {
    let mut game = make_game();
    let result = player_events::apply_player_response(
        &mut game,
        "morale_talk_nonexistent",
        "respond",
        "encourage",
    );
    assert!(result.is_none());
}

#[test]
fn morale_clamps_to_5_100() {
    let mut game = make_game();
    // Test lower bound
    game.players
        .iter_mut()
        .find(|p| p.id == "p_fwd0")
        .unwrap()
        .morale = 8;
    inject_player_message(&mut game, "contract_concern_p_fwd0", "p_fwd0", "respond");

    player_events::apply_player_response(
        &mut game,
        "contract_concern_p_fwd0",
        "respond",
        "no_renewal",
    );

    let p = game.players.iter().find(|p| p.id == "p_fwd0").unwrap();
    assert!(p.morale >= 5, "Morale should clamp at 5, got {}", p.morale);
}

#[test]
fn morale_clamps_at_100() {
    let mut game = make_game();
    game.players
        .iter_mut()
        .find(|p| p.id == "p_fwd0")
        .unwrap()
        .morale = 95;
    inject_player_message(&mut game, "morale_talk_p_fwd0", "p_fwd0", "respond");

    player_events::apply_player_response(
        &mut game,
        "morale_talk_p_fwd0",
        "respond",
        "promise_time",
    );

    let p = game.players.iter().find(|p| p.id == "p_fwd0").unwrap();
    assert!(
        p.morale <= 100,
        "Morale should cap at 100, got {}",
        p.morale
    );
}

#[test]
fn action_marked_resolved() {
    let mut game = make_game();
    game.players
        .iter_mut()
        .find(|p| p.id == "p_fwd0")
        .unwrap()
        .morale = 30;
    inject_player_message(&mut game, "morale_talk_p_fwd0", "p_fwd0", "respond");

    player_events::apply_player_response(&mut game, "morale_talk_p_fwd0", "respond", "encourage");

    let msg = game
        .messages
        .iter()
        .find(|m| m.id == "morale_talk_p_fwd0")
        .unwrap();
    let action = msg.actions.iter().find(|a| a.id == "respond").unwrap();
    assert!(action.resolved, "Action should be marked as resolved");
}

// ---------------------------------------------------------------------------
// Personality factor influence
// ---------------------------------------------------------------------------

#[test]
fn volatile_player_worse_outcomes_from_tough_love() {
    let mut game = make_game();
    // Make player volatile: high aggression, low composure
    let player = game.players.iter_mut().find(|p| p.id == "p_fwd0").unwrap();
    player.morale = 50;
    player.attributes.aggression = 95;
    player.attributes.composure = 20;
    player.attributes.leadership = 20;
    inject_player_message(&mut game, "morale_talk_p_fwd0", "p_fwd0", "respond");

    let mut total_delta: i32 = 0;
    let runs = 100;
    for _ in 0..runs {
        let mut g = game.clone();
        player_events::apply_player_response(
            &mut g,
            "morale_talk_p_fwd0",
            "respond",
            "work_harder",
        );
        let p = g.players.iter().find(|p| p.id == "p_fwd0").unwrap();
        total_delta += p.morale as i32 - 50;
    }
    let avg_volatile = total_delta as f64 / runs as f64;

    // Now test composed player
    let player = game.players.iter_mut().find(|p| p.id == "p_fwd0").unwrap();
    player.attributes.aggression = 20;
    player.attributes.composure = 95;
    player.attributes.leadership = 95;

    let mut total_delta2: i32 = 0;
    for _ in 0..runs {
        let mut g = game.clone();
        player_events::apply_player_response(
            &mut g,
            "morale_talk_p_fwd0",
            "respond",
            "work_harder",
        );
        let p = g.players.iter().find(|p| p.id == "p_fwd0").unwrap();
        total_delta2 += p.morale as i32 - 50;
    }
    let avg_composed = total_delta2 as f64 / runs as f64;

    assert!(
        avg_composed > avg_volatile,
        "Composed player should respond better to tough love: composed={:.1}, volatile={:.1}",
        avg_composed,
        avg_volatile
    );
}
