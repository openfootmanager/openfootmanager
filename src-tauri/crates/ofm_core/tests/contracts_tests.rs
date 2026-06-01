use chrono::{TimeZone, Utc};
use domain::manager::Manager;
use domain::player::{
    ContractExitIntent, ContractRenewalState, Player, PlayerAttributes, Position,
    RenewalSessionStatus,
};
use domain::staff::{Staff, StaffAttributes, StaffRole};
use domain::team::{FinancialTransactionKind, Team};
use ofm_core::clock::GameClock;
use ofm_core::contracts::{
    DelegatedRenewalOptions, DelegatedRenewalResultStatus, RenewalDecision, RenewalOffer,
    clear_contract_exit_intent, delegate_renewals, evaluate_renewal_offer, has_let_expire_intent,
    offer_free_agent_contract, preview_contract_termination, project_free_agent_contract_impact,
    propose_renewal, set_contract_exit_intent, terminate_contract_now,
};
use ofm_core::game::Game;
use domain::season::TransferWindowStatus;

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

fn make_player() -> Player {
    let mut player = Player::new(
        "player-1".to_string(),
        "J. Smith".to_string(),
        "John Smith".to_string(),
        "2000-01-01".to_string(),
        "England".to_string(),
        Position::Forward,
        default_attrs(),
    );
    player.team_id = Some("team-1".to_string());
    player.contract_end = Some("2026-10-15".to_string());
    player.wage = 12_000;
    player.morale = 75;
    player.market_value = 350_000;
    player
}

fn make_player_with(id: &str, wage: u32, contract_end: &str) -> Player {
    let mut player = make_player();
    player.id = id.to_string();
    player.match_name = id.to_string();
    player.full_name = format!("Player {}", id);
    player.wage = wage;
    player.contract_end = Some(contract_end.to_string());
    player
}

fn make_player_with_position(id: &str, position: Position) -> Player {
    let mut player = make_player_with(id, 12_000, "2026-10-15");
    player.position = position.clone();
    player.natural_position = position;
    player
}

fn make_team() -> Team {
    let mut team = Team::new(
        "team-1".to_string(),
        "Alpha FC".to_string(),
        "ALP".to_string(),
        "England".to_string(),
        "London".to_string(),
        "Alpha Ground".to_string(),
        30_000,
    );
    team.manager_id = Some("manager-1".to_string());
    team.reputation = 50;
    team.wage_budget = 50_000;
    team
}

fn make_assistant_manager() -> Staff {
    let mut staff = Staff::new(
        "staff-1".to_string(),
        "Alex".to_string(),
        "Assistant".to_string(),
        "1985-01-01".to_string(),
        StaffRole::AssistantManager,
        StaffAttributes {
            coaching: 82,
            judging_ability: 76,
            judging_potential: 74,
            physiotherapy: 30,
        },
    );
    staff.team_id = Some("team-1".to_string());
    staff
}

fn make_game() -> Game {
    let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap());
    let mut manager = Manager::new(
        "manager-1".to_string(),
        "Jane".to_string(),
        "Doe".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("team-1".to_string());

    Game::new(
        clock,
        manager,
        vec![make_team()],
        vec![make_player()],
        vec![],
        vec![],
    )
}

fn make_squad_game() -> Game {
    let mut game = make_game();
    game.players = vec![
        make_player_with_position("gk-1", Position::Goalkeeper),
        make_player_with_position("player-1", Position::Forward),
        make_player_with_position("player-2", Position::Forward),
        make_player_with_position("player-3", Position::Defender),
        make_player_with_position("player-4", Position::Defender),
        make_player_with_position("player-5", Position::Defender),
        make_player_with_position("player-6", Position::Defender),
        make_player_with_position("player-7", Position::Midfielder),
        make_player_with_position("player-8", Position::Midfielder),
        make_player_with_position("player-9", Position::Midfielder),
        make_player_with_position("player-10", Position::Midfielder),
        make_player_with_position("player-11", Position::Forward),
    ];
    game.teams[0].starting_xi_ids = game
        .players
        .iter()
        .take(11)
        .map(|player| player.id.clone())
        .collect();
    game
}

fn make_free_agent() -> Player {
    let mut player = make_player();
    player.id = "free-agent-1".to_string();
    player.match_name = "F. Agent".to_string();
    player.full_name = "Free Agent".to_string();
    player.team_id = None;
    player.contract_end = None;
    player.wage = 0;
    player.market_value = 600_000;
    player
}

fn make_free_agent_game() -> Game {
    let mut game = make_game();
    game.players = vec![make_free_agent()];
    game.season_context.transfer_window.status = TransferWindowStatus::Open;
    game
}

#[test]
fn accepted_offer_updates_wage_and_term_correctly() {
    let mut game = make_game();

    let outcome = propose_renewal(
        &mut game,
        "player-1",
        RenewalOffer {
            weekly_wage: 15_000,
            contract_years: 3,
        },
    )
    .expect("renewal should succeed");

    assert!(matches!(outcome.decision, RenewalDecision::Accepted));
    assert!(outcome.feedback.is_some());
    let player = game.players.iter().find(|p| p.id == "player-1").unwrap();
    assert_eq!(player.wage, 15_000);
    assert_eq!(player.contract_end.as_deref(), Some("2029-08-01"));
}

#[test]
fn let_expire_intent_persists_and_can_be_cleared() {
    let mut game = make_game();

    set_contract_exit_intent(&mut game, "player-1", Some("manager_choice".to_string()))
        .expect("exit intent should be set");

    let player = game.players.iter().find(|p| p.id == "player-1").unwrap();
    assert!(has_let_expire_intent(player));
    let state = player.morale_core.renewal_state.as_ref().unwrap();
    assert_eq!(state.status, RenewalSessionStatus::Blocked);
    assert!(matches!(
        state.exit_intent,
        Some(ContractExitIntent::LetExpire { .. })
    ));

    clear_contract_exit_intent(&mut game, "player-1").expect("exit intent should clear");

    let player = game.players.iter().find(|p| p.id == "player-1").unwrap();
    assert!(!has_let_expire_intent(player));
    assert_eq!(
        player.morale_core.renewal_state.as_ref().unwrap().status,
        RenewalSessionStatus::Idle
    );
}

#[test]
fn termination_preview_reports_severance_and_squad_safety() {
    let game = make_squad_game();

    let preview = preview_contract_termination(&game, "player-1").expect("preview");

    assert_eq!(preview.player_id, "player-1");
    assert_eq!(preview.severance_cost, 132_000);
    assert!(preview.squad_safety.can_field_matchday_squad);
    assert_eq!(preview.squad_safety.projected_roster_size, 11);
}

#[test]
fn terminate_contract_now_releases_player_and_charges_severance() {
    let mut game = make_squad_game();
    let original_finance = game.teams[0].finance;

    let result = terminate_contract_now(&mut game, "player-1").expect("termination succeeds");

    assert_eq!(result.severance_cost, 132_000);
    let player = game.players.iter().find(|p| p.id == "player-1").unwrap();
    assert_eq!(player.team_id, None);
    assert_eq!(player.contract_end, None);
    assert_eq!(player.wage, 0);
    assert!(
        !game.teams[0]
            .starting_xi_ids
            .contains(&"player-1".to_string())
    );
    assert_eq!(game.teams[0].finance, original_finance - 132_000);
    assert_eq!(game.teams[0].season_expenses, 132_000);
    assert_eq!(
        game.teams[0].financial_ledger.last().unwrap().description,
        "be.msg.contractTerminated.ledgerDescription?player=player-1"
    );
    assert_eq!(
        game.teams[0].financial_ledger.last().unwrap().kind,
        FinancialTransactionKind::ContractTermination
    );
    let message = game
        .messages
        .iter()
        .find(|message| message.id == "contract_terminated_player-1")
        .expect("termination should create an inbox message");
    assert_eq!(
        message.subject_key.as_deref(),
        Some("be.msg.contractTerminated.subject")
    );
    assert_eq!(
        message.body_key.as_deref(),
        Some("be.msg.contractTerminated.body")
    );
    assert_eq!(
        message.sender_key.as_deref(),
        Some("be.sender.assistantManager")
    );
    assert_eq!(
        message.sender_role_key.as_deref(),
        Some("be.role.assistantManager")
    );
    assert!(message.subject.is_empty());
    assert!(message.body.is_empty());
    assert!(message.sender.is_empty());
    assert!(message.sender_role.is_empty());
    assert_eq!(
        message.i18n_params.get("player"),
        Some(&"player-1".to_string())
    );
    assert_eq!(
        message.i18n_params.get("team"),
        Some(&"Alpha FC".to_string())
    );
    assert_eq!(
        message.i18n_params.get("severance"),
        Some(&"132000".to_string())
    );
}

#[test]
fn terminate_contract_now_blocks_when_goalkeeper_would_be_lost() {
    let mut game = make_squad_game();
    let original_finance = game.teams[0].finance;

    let error = terminate_contract_now(&mut game, "gk-1").expect_err("termination should fail");

    assert_eq!(
        error,
        "be.error.contracts.terminationWouldLeaveMatchdaySquadShort"
    );
    let player = game.players.iter().find(|p| p.id == "gk-1").unwrap();
    assert_eq!(player.team_id.as_deref(), Some("team-1"));
    assert_eq!(game.teams[0].finance, original_finance);
}

#[test]
fn rejected_offer_leaves_state_unchanged() {
    let mut game = make_game();
    let original_wage = game.players[0].wage;
    let original_end = game.players[0].contract_end.clone();

    let outcome = propose_renewal(
        &mut game,
        "player-1",
        RenewalOffer {
            weekly_wage: 7_000,
            contract_years: 1,
        },
    )
    .expect("renewal should return a decision");

    assert!(matches!(outcome.decision, RenewalDecision::Rejected));
    assert_eq!(game.players[0].wage, original_wage);
    assert_eq!(game.players[0].contract_end, original_end);
}

#[test]
fn counter_offer_returns_understandable_feedback() {
    let mut game = make_game();

    let outcome = propose_renewal(
        &mut game,
        "player-1",
        RenewalOffer {
            weekly_wage: 13_000,
            contract_years: 2,
        },
    )
    .expect("renewal should return a counter offer");

    assert!(matches!(outcome.decision, RenewalDecision::CounterOffer));
    assert_eq!(outcome.suggested_wage, Some(14_000));
    assert_eq!(outcome.suggested_years, Some(3));
    let feedback = outcome.feedback.expect("feedback should be present");
    assert_eq!(feedback.round, 1);
    assert!(feedback.tension > 0);
}

#[test]
fn high_value_star_expects_more_than_fringe_player() {
    let current_date = Utc
        .with_ymd_and_hms(2026, 8, 1, 12, 0, 0)
        .unwrap()
        .date_naive();
    let team = make_team();

    let mut star = make_player();
    star.contract_end = Some("2028-08-01".to_string());
    star.market_value = 2_500_000;
    star.attributes.pace = 88;
    star.attributes.shooting = 90;
    star.attributes.dribbling = 87;

    let mut fringe = make_player();
    fringe.contract_end = Some("2028-08-01".to_string());
    fringe.market_value = 80_000;
    fringe.attributes.pace = 50;
    fringe.attributes.shooting = 48;
    fringe.attributes.dribbling = 49;

    let offer = RenewalOffer {
        weekly_wage: 14_000,
        contract_years: 3,
    };

    let star_outcome = evaluate_renewal_offer(&star, &team, current_date, &offer);
    let fringe_outcome = evaluate_renewal_offer(&fringe, &team, current_date, &offer);

    assert!(matches!(fringe_outcome.decision, RenewalDecision::Accepted));
    assert!(matches!(
        star_outcome.decision,
        RenewalDecision::CounterOffer
    ));
    assert!(star_outcome.suggested_wage > fringe_outcome.suggested_wage);
}

#[test]
fn free_agent_offer_accepts_and_assigns_player_to_manager_team() {
    let mut game = make_free_agent_game();

    let outcome = offer_free_agent_contract(
        &mut game,
        "free-agent-1",
        RenewalOffer {
            weekly_wage: 4_000,
            contract_years: 3,
        },
    )
    .expect("free-agent offer should resolve");

    assert!(matches!(outcome.decision, RenewalDecision::Accepted));
    let player = game.players.iter().find(|player| player.id == "free-agent-1").unwrap();
    assert_eq!(player.team_id.as_deref(), Some("team-1"));
    assert_eq!(player.wage, 4_000);
    assert_eq!(player.contract_end.as_deref(), Some("2029-08-01"));
    let message = game
        .messages
        .iter()
        .find(|message| message.id == "free_agent_signed_free-agent-1")
        .expect("free-agent signing should create an inbox message");
    assert_eq!(
        message.subject_key.as_deref(),
        Some("be.msg.freeAgentSigned.subject")
    );
    assert_eq!(
        message.body_key.as_deref(),
        Some("be.msg.freeAgentSigned.body")
    );
    assert_eq!(
        message.sender_key.as_deref(),
        Some("be.sender.assistantManager")
    );
    assert_eq!(
        message.sender_role_key.as_deref(),
        Some("be.role.assistantManager")
    );
    assert!(message.subject.is_empty());
    assert!(message.body.is_empty());
    assert!(message.sender.is_empty());
    assert!(message.sender_role.is_empty());
    assert_eq!(
        message.i18n_params.get("player"),
        Some(&"Free Agent".to_string())
    );
    assert_eq!(message.i18n_params.get("team"), Some(&"Alpha FC".to_string()));
    assert_eq!(message.i18n_params.get("years"), Some(&"3".to_string()));
}

#[test]
fn free_agent_offer_returns_counter_when_terms_are_close_but_short() {
    let mut game = make_free_agent_game();

    let outcome = offer_free_agent_contract(
        &mut game,
        "free-agent-1",
        RenewalOffer {
            weekly_wage: 3_000,
            contract_years: 2,
        },
    )
    .expect("free-agent offer should resolve");

    assert!(matches!(outcome.decision, RenewalDecision::CounterOffer));
    assert_eq!(outcome.suggested_wage, Some(4_000));
    assert_eq!(outcome.suggested_years, Some(3));
    assert_eq!(game.players[0].team_id, None);
}

#[test]
fn free_agent_offer_rejects_lowball_terms() {
    let mut game = make_free_agent_game();

    let outcome = offer_free_agent_contract(
        &mut game,
        "free-agent-1",
        RenewalOffer {
            weekly_wage: 1_000,
            contract_years: 1,
        },
    )
    .expect("free-agent offer should resolve");

    assert!(matches!(outcome.decision, RenewalDecision::Rejected));
    assert_eq!(game.players[0].team_id, None);
    assert_eq!(game.players[0].wage, 0);
}

#[test]
fn free_agent_offer_rejects_contracts_longer_than_five_years() {
    let mut game = make_free_agent_game();

    let outcome = offer_free_agent_contract(
        &mut game,
        "free-agent-1",
        RenewalOffer {
            weekly_wage: 4_000,
            contract_years: 6,
        },
    )
    .expect("free-agent offer should resolve");

    assert!(matches!(outcome.decision, RenewalDecision::Rejected));
    assert_eq!(game.players[0].team_id, None);
    assert_eq!(game.players[0].contract_end, None);
}

#[test]
fn free_agent_projection_uses_manager_team_wage_context() {
    let game = make_free_agent_game();

    let projection =
        project_free_agent_contract_impact(&game, "free-agent-1", 4_000).expect("projection");

    assert_eq!(projection.current_annual_wage_bill, 0);
    assert_eq!(projection.projected_annual_wage_bill, 4_000);
    assert!(projection.policy_allows);
}

#[test]
fn low_morale_player_becomes_harder_to_renew_than_content_player() {
    let current_date = Utc
        .with_ymd_and_hms(2026, 8, 1, 12, 0, 0)
        .unwrap()
        .date_naive();
    let team = make_team();

    let mut content_player = make_player();
    content_player.contract_end = Some("2028-08-01".to_string());
    content_player.morale = 85;

    let mut unhappy_player = make_player();
    unhappy_player.contract_end = Some("2028-08-01".to_string());
    unhappy_player.morale = 35;

    let offer = RenewalOffer {
        weekly_wage: 13_000,
        contract_years: 3,
    };

    let content_outcome = evaluate_renewal_offer(&content_player, &team, current_date, &offer);
    let unhappy_outcome = evaluate_renewal_offer(&unhappy_player, &team, current_date, &offer);

    assert!(matches!(
        content_outcome.decision,
        RenewalDecision::Accepted
    ));
    assert!(matches!(
        unhappy_outcome.decision,
        RenewalDecision::CounterOffer
    ));
}

#[test]
fn shorter_remaining_term_increases_renewal_demands() {
    let current_date = Utc
        .with_ymd_and_hms(2026, 8, 1, 12, 0, 0)
        .unwrap()
        .date_naive();
    let team = make_team();

    let mut secure_player = make_player();
    secure_player.contract_end = Some("2028-08-01".to_string());

    let mut expiring_player = make_player();
    expiring_player.contract_end = Some("2026-10-01".to_string());

    let offer = RenewalOffer {
        weekly_wage: 13_000,
        contract_years: 3,
    };

    let secure_outcome = evaluate_renewal_offer(&secure_player, &team, current_date, &offer);
    let expiring_outcome = evaluate_renewal_offer(&expiring_player, &team, current_date, &offer);

    assert!(matches!(secure_outcome.decision, RenewalDecision::Accepted));
    assert!(matches!(
        expiring_outcome.decision,
        RenewalDecision::CounterOffer
    ));
}

#[test]
fn low_manager_trust_player_can_refuse_manual_renewal_even_at_fair_terms() {
    let mut game = make_game();
    game.players[0].morale_core.manager_trust = 18;

    let outcome = propose_renewal(
        &mut game,
        "player-1",
        RenewalOffer {
            weekly_wage: 15_000,
            contract_years: 3,
        },
    )
    .expect("renewal should produce an outcome");

    assert!(matches!(outcome.decision, RenewalDecision::Rejected));
}

#[test]
fn manager_block_prevents_manual_renewal_until_it_expires() {
    let mut game = make_game();
    game.players[0].morale_core.renewal_state = Some(ContractRenewalState {
        status: RenewalSessionStatus::Blocked,
        manager_blocked_until: Some("2026-09-01".to_string()),
        last_attempt_date: None,
        last_assistant_attempt_date: None,
        last_outcome: None,
        conversation_round: 0,
        exit_intent: None,
    });

    let outcome = propose_renewal(
        &mut game,
        "player-1",
        RenewalOffer {
            weekly_wage: 16_000,
            contract_years: 3,
        },
    )
    .expect("renewal should produce an outcome");

    assert!(matches!(outcome.decision, RenewalDecision::Rejected));
    assert_eq!(outcome.session_status, RenewalSessionStatus::Blocked);
    assert!(outcome.is_terminal);
}

#[test]
fn stale_manual_renewal_talks_cool_off_and_restart_from_round_one() {
    let mut game = make_game();
    game.players[0].morale_core.renewal_state = Some(ContractRenewalState {
        status: RenewalSessionStatus::Open,
        manager_blocked_until: None,
        last_attempt_date: Some("2026-08-10".to_string()),
        last_assistant_attempt_date: None,
        last_outcome: None,
        conversation_round: 3,
        exit_intent: None,
    });
    game.clock = GameClock::new(Utc.with_ymd_and_hms(2026, 8, 26, 12, 0, 0).unwrap());

    let outcome = propose_renewal(
        &mut game,
        "player-1",
        RenewalOffer {
            weekly_wage: 13_000,
            contract_years: 2,
        },
    )
    .expect("renewal should produce an outcome");

    assert!(outcome.cooled_off);
    let feedback = outcome.feedback.expect("feedback should be present");
    assert_eq!(feedback.round, 1);

    let renewal_state = game.players[0]
        .morale_core
        .renewal_state
        .as_ref()
        .expect("renewal state should be stored");
    assert_eq!(renewal_state.conversation_round, 1);
    assert_eq!(
        renewal_state.last_attempt_date.as_deref(),
        Some("2026-08-26")
    );
}

#[test]
fn assistant_can_complete_routine_delegate_renewal_even_when_manager_trust_is_low() {
    let mut game = make_game();
    game.staff.push(make_assistant_manager());
    game.players[0].morale_core.manager_trust = 24;
    game.players[0].morale = 74;

    let report = delegate_renewals(
        &mut game,
        DelegatedRenewalOptions {
            player_ids: Some(vec!["player-1".to_string()]),
            max_wage_increase_pct: 35,
            max_contract_years: 3,
        },
    )
    .expect("assistant delegation should return a report");

    assert_eq!(report.success_count, 1);
    assert_eq!(report.failure_count, 0);
    assert_eq!(report.stalled_count, 0);
    assert_eq!(report.cases.len(), 1);
    assert_eq!(report.cases[0].player_id, "player-1");
    assert_eq!(
        report.cases[0].status,
        DelegatedRenewalResultStatus::Successful
    );

    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-1")
        .unwrap();
    assert_eq!(player.contract_end.as_deref(), Some("2029-08-01"));
    assert!(player.wage >= 14_000);

    let report_message = game
        .messages
        .iter()
        .find(|message| message.id.starts_with("delegated_renewals_"))
        .expect("assistant delegation should create an inbox report");
    assert_eq!(
        report_message.subject_key.as_deref(),
        Some("be.msg.delegatedRenewals.subject")
    );
    assert_eq!(
        report_message.body_key.as_deref(),
        Some("be.msg.delegatedRenewals.body")
    );
    assert_eq!(
        report_message.sender_key.as_deref(),
        Some("be.sender.assistantManager")
    );
    assert!(report_message.subject.is_empty());
    assert!(report_message.body.is_empty());
    assert!(report_message.sender.is_empty());
    assert!(report_message.sender_role.is_empty());
    let structured_report = report_message
        .context
        .delegated_renewal_report
        .as_ref()
        .expect("assistant report should carry structured i18n-safe case data");
    assert_eq!(structured_report.success_count, 1);
    assert_eq!(structured_report.cases.len(), 1);
    assert_eq!(structured_report.cases[0].status, "successful");
    assert_eq!(
        structured_report.cases[0].note_key.as_deref(),
        Some("be.msg.delegatedRenewals.notes.completed")
    );
}

#[test]
fn renewal_is_blocked_when_offer_pushes_healthy_club_far_over_soft_cap() {
    let mut game = make_game();
    game.teams[0].wage_budget = 200_000;

    let err = propose_renewal(
        &mut game,
        "player-1",
        RenewalOffer {
            weekly_wage: 250_000,
            contract_years: 3,
        },
    )
    .expect_err("renewal should be blocked by wage policy");

    assert_eq!(err, "be.error.contracts.boardWagePolicy?budget=200000");
}

#[test]
fn renewal_allows_small_increase_for_legacy_over_budget_saves() {
    let mut game = make_game();
    game.teams[0].wage_budget = 50_000;
    game.players[0].wage = 48_000;
    game.players
        .push(make_player_with("player-2", 40_000, "2027-06-30"));

    let outcome = propose_renewal(
        &mut game,
        "player-1",
        RenewalOffer {
            weekly_wage: 70_000,
            contract_years: 2,
        },
    );

    assert!(
        outcome.is_ok(),
        "legacy saves should allow manageable wage increases without policy lock"
    );
}

#[test]
fn renewal_blocks_large_worsening_for_legacy_over_budget_saves() {
    let mut game = make_game();
    game.teams[0].wage_budget = 50_000;
    game.players[0].wage = 48_000;
    game.players
        .push(make_player_with("player-2", 40_000, "2027-06-30"));

    let err = propose_renewal(
        &mut game,
        "player-1",
        RenewalOffer {
            weekly_wage: 120_000,
            contract_years: 3,
        },
    )
    .expect_err("large worsening should still be blocked");

    assert_eq!(err, "be.error.contracts.boardWagePolicy?budget=50000");
}
