use chrono::{TimeZone, Utc};
use domain::league::{
    Fixture, FixtureCompetition, FixtureStatus, League, MatchResult, StandingEntry,
};
use domain::manager::Manager;
use domain::player::{Player, PlayerAttributes, Position};
use domain::staff::{Staff, StaffAttributes, StaffRole};
use domain::team::{
    FinancialTransaction, FinancialTransactionKind, Sponsorship, SponsorshipBonusCriterion, Team,
};
use ofm_core::clock::GameClock;
use ofm_core::finances;
use ofm_core::game::Game;

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn make_team(id: &str, name: &str) -> Team {
    let mut t = Team::new(
        id.to_string(),
        name.to_string(),
        name[..3].to_string(),
        "England".to_string(),
        "London".to_string(),
        "Stadium".to_string(),
        40_000,
    );
    t.finance = 5_000_000;
    t.wage_budget = 2_000_000;
    t
}

fn make_player(id: &str, team_id: &str, wage: u32) -> Player {
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
        "Player".to_string(),
        format!("Full {}", id),
        "1995-01-01".to_string(),
        "GB".to_string(),
        Position::Midfielder,
        attrs,
    );
    p.team_id = Some(team_id.to_string());
    p.wage = wage;
    p.condition = 90;
    p
}

fn make_staff(id: &str, team_id: &str, wage: u32) -> Staff {
    let mut s = Staff::new(
        id.to_string(),
        "Staff".to_string(),
        id.to_string(),
        "1980-01-01".to_string(),
        StaffRole::Coach,
        StaffAttributes {
            coaching: 70,
            judging_ability: 50,
            judging_potential: 50,
            physiotherapy: 30,
        },
    );
    s.team_id = Some(team_id.to_string());
    s.nationality = "GB".to_string();
    s.wage = wage;
    s
}

/// Create a game set on a Monday (weekday 0) so process_weekly_finances runs.
fn make_monday_game() -> Game {
    // 2025-06-16 is a Monday
    let date = Utc.with_ymd_and_hms(2025, 6, 16, 12, 0, 0).unwrap();
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
    let p1 = make_player("p1", "team1", 52_000); // 52000/52 = 1000/week
    let p2 = make_player("p2", "team1", 26_000); // 26000/52 = 500/week
    let s1 = make_staff("s1", "team1", 10_400); // 10400/52 = 200/week

    Game::new(clock, manager, vec![team1], vec![p1, p2], vec![s1], vec![])
}

// ---------------------------------------------------------------------------
// process_weekly_finances — wage deductions
// ---------------------------------------------------------------------------

#[test]
fn calc_wages_sums_player_and_staff_wages_for_a_team() {
    let game = make_monday_game();

    let weekly_wages = finances::calc_wages(&game, "team1");

    assert_eq!(weekly_wages, 1_700);
}

#[test]
fn calc_annual_wages_sums_full_contract_values_for_a_team() {
    let game = make_monday_game();

    let annual_wages = finances::calc_annual_wages(&game, "team1");

    assert_eq!(annual_wages, 88_400);
}

#[test]
fn calc_cash_runway_weeks_uses_projected_weekly_net() {
    assert_eq!(finances::calc_cash_runway_weeks(180_000, -30_000), Some(6));
    assert_eq!(finances::calc_cash_runway_weeks(180_000, 5_000), None);
}

#[test]
fn team_finance_snapshot_uses_canonical_backend_values() {
    let mut game = make_monday_game();
    game.teams[0].sponsorship = Some(Sponsorship {
        sponsor_name: "Acme Corp".to_string(),
        base_value: 2_000,
        remaining_weeks: 8,
        bonus_criteria: vec![],
    });

    let snapshot = finances::team_finance_snapshot(&game, "team1").expect("snapshot");

    assert_eq!(snapshot.annual_wage_bill, 88_400);
    assert_eq!(snapshot.weekly_wage_spend, 1_700);
    assert_eq!(snapshot.weekly_wage_budget, 2_000_000 / 52);
    assert_eq!(snapshot.weekly_sponsor_income, 2_000);
    assert_eq!(snapshot.weekly_recurring_income, 2_000);
    assert_eq!(snapshot.projected_weekly_net, 300);
    assert_eq!(snapshot.cash_runway_weeks, None);
    assert_eq!(snapshot.wage_budget_usage_percent, 4);
    assert!(!snapshot.currently_in_debt);
    assert!(!snapshot.currently_over_budget);
}

#[test]
fn team_finance_snapshot_flags_wage_pressure() {
    let mut game = make_monday_game();
    game.teams[0].wage_budget = 80_000;

    let snapshot = finances::team_finance_snapshot(&game, "team1").expect("snapshot");

    assert!(snapshot.currently_over_budget);
    assert_eq!(snapshot.wage_budget_usage_percent, 110);
    assert_eq!(
        snapshot.wage_budget_status,
        finances::FinanceHealthLevel::Warning
    );
    assert_eq!(
        snapshot.overall_status,
        finances::FinanceHealthLevel::Warning
    );
}

#[test]
fn team_finance_snapshot_flags_runway_crisis() {
    let mut game = make_monday_game();
    game.teams[0].finance = 3_400;

    let snapshot = finances::team_finance_snapshot(&game, "team1").expect("snapshot");

    assert_eq!(snapshot.projected_weekly_net, -1_700);
    assert_eq!(snapshot.cash_runway_weeks, Some(2));
    assert_eq!(
        snapshot.runway_status,
        finances::FinanceHealthLevel::Critical
    );
    assert_eq!(
        snapshot.overall_status,
        finances::FinanceHealthLevel::Critical
    );
}

#[test]
fn team_finance_snapshot_treats_negative_balance_as_critical() {
    let mut game = make_monday_game();
    game.teams[0].finance = -1;

    let snapshot = finances::team_finance_snapshot(&game, "team1").expect("snapshot");

    assert!(snapshot.currently_in_debt);
    assert_eq!(
        snapshot.runway_status,
        finances::FinanceHealthLevel::Critical
    );
    assert_eq!(
        snapshot.overall_status,
        finances::FinanceHealthLevel::Critical
    );
}

#[test]
fn request_board_support_recovers_cash_crisis_and_applies_board_costs() {
    let mut game = make_monday_game();
    game.teams[0].finance = -25_000;
    game.teams[0].transfer_budget = 300_000;
    game.manager.satisfaction = 70;

    let result = finances::request_board_support(&mut game, "team1").expect("support");

    assert!(result.support_amount >= 150_000);
    assert_eq!(result.transfer_budget_reduction, result.support_amount / 2);
    assert_eq!(result.satisfaction_penalty, 12);
    assert_eq!(game.manager.satisfaction, 58);
    assert_eq!(game.teams[0].season_income, result.support_amount);
    assert_eq!(
        game.teams[0].transfer_budget,
        300_000 - result.transfer_budget_reduction
    );
    assert_eq!(
        game.teams[0].financial_ledger.last().expect("ledger").kind,
        FinancialTransactionKind::BoardSupport
    );
    assert!(game.teams[0].finance > 0);
}

#[test]
fn request_board_support_rejects_second_support_package_in_same_season() {
    let mut game = make_monday_game();
    game.teams[0].finance = -25_000;

    finances::request_board_support(&mut game, "team1").expect("first support");
    game.teams[0].finance = -10_000;

    let error = finances::request_board_support(&mut game, "team1").expect_err("should fail");

    assert_eq!(error, "be.error.finance.boardSupportAlreadyUsed");
}

#[test]
fn finance_action_previews_are_available_without_mutating_state() {
    let mut game = make_monday_game();
    game.teams[0].finance = -40_000;
    game.teams[0].wage_budget = 50_000;

    let previews = finances::finance_action_previews(&game, "team1").expect("previews");

    assert!(previews.board_support.is_some());
    assert!(previews.sponsor_pitch.is_some());
    assert!(previews.marketing_campaign.is_some());
    assert_eq!(game.teams[0].finance, -40_000);
    assert!(game.messages.is_empty());
    assert!(game.teams[0].financial_ledger.is_empty());
}

#[test]
fn request_board_support_can_recover_runway_without_random_events() {
    let mut game = make_monday_game();
    game.teams[0].finance = 13_600;

    let preview = finances::preview_board_support(&game, "team1").expect("preview");
    let result = finances::request_board_support(&mut game, "team1").expect("support");
    let snapshot = finances::team_finance_snapshot(&game, "team1").expect("snapshot");

    assert_eq!(result.support_amount, preview.support_amount);
    assert_eq!(result.transfer_budget_reduction, preview.transfer_budget_reduction);
    assert_eq!(result.satisfaction_penalty, preview.satisfaction_penalty);
    assert_eq!(snapshot.overall_status, finances::FinanceHealthLevel::Stable);
}

#[test]
fn request_sponsor_pitch_creates_pending_offer_for_over_budget_team() {
    let mut game = make_monday_game();
    game.teams[0].wage_budget = 50_000;

    let result = finances::request_sponsor_pitch(&mut game, "team1").expect("pitch response");

    assert_eq!(result.duration_weeks, 12);
    assert!(result.weekly_amount >= 40_000);
    let message = game
        .messages
        .iter()
        .find(|message| message.id == result.message_id)
        .expect("sponsor offer message");
    assert!(message.id.starts_with("sponsor_"));
    assert!(message.actions.iter().any(|action| !action.resolved));
    assert_eq!(message.subject_key.as_deref(), Some("be.msg.sponsor.subject"));
    assert_eq!(message.body_key.as_deref(), Some("be.msg.sponsor.body"));
    assert_eq!(message.sender_key.as_deref(), Some("be.sender.commercialDirector"));
    assert!(message.subject.is_empty());
    assert!(message.body.is_empty());
    assert!(message.sender.is_empty());
}

#[test]
fn request_sponsor_pitch_rejects_healthy_club() {
    let mut game = make_monday_game();
    game.teams[0].wage_budget = 5_000_000;
    game.teams[0].finance = 2_000_000;

    let error =
        finances::request_sponsor_pitch(&mut game, "team1").expect_err("healthy club should fail");

    assert_eq!(error, "be.error.finance.sponsorPitchUnavailable");
}

#[test]
fn request_sponsor_pitch_rejects_when_offer_is_already_pending() {
    let mut game = make_monday_game();
    game.teams[0].wage_budget = 50_000;
    finances::request_sponsor_pitch(&mut game, "team1").expect("first pitch");

    let error = finances::request_sponsor_pitch(&mut game, "team1")
        .expect_err("second pending pitch should fail");

    assert_eq!(error, "be.error.finance.sponsorPitchPendingOffer");
}

#[test]
fn request_marketing_campaign_generates_cash_for_pressured_club() {
    let mut game = make_monday_game();
    game.teams[0].wage_budget = 50_000;
    game.teams[0].finance = -60_000;

    let result = finances::request_marketing_campaign(&mut game, "team1").expect("campaign");

    assert!(result.gross_revenue >= result.net_income);
    assert!(result.campaign_cost > 0);
    assert!(result.net_income > 0);
    assert_eq!(result.cooldown_days, 28);
    assert!(result.message_id.starts_with("marketing_campaign_"));
    assert_eq!(game.teams[0].finance, -60_000 + result.net_income);
    assert_eq!(game.teams[0].season_income, result.gross_revenue);
    assert_eq!(game.teams[0].season_expenses, result.campaign_cost);
    assert_eq!(
        game.teams[0]
            .financial_ledger
            .iter()
            .filter(|entry| entry.kind == FinancialTransactionKind::CommercialCampaign)
            .count(),
        2
    );
    let message = game
        .messages
        .iter()
        .find(|message| message.id == result.message_id)
        .expect("marketing campaign message");
    assert_eq!(message.subject_key.as_deref(), Some("be.msg.marketingCampaign.subject"));
    assert_eq!(message.body_key.as_deref(), Some("be.msg.marketingCampaign.body"));
    assert_eq!(message.sender_key.as_deref(), Some("be.sender.commercialDirector"));
    assert!(message.subject.is_empty());
    assert!(message.body.is_empty());
    assert!(message.sender.is_empty());
}

#[test]
fn request_marketing_campaign_rejects_healthy_club() {
    let mut game = make_monday_game();
    game.teams[0].wage_budget = 5_000_000;
    game.teams[0].finance = 2_000_000;

    let error = finances::request_marketing_campaign(&mut game, "team1")
        .expect_err("healthy club should fail");

    assert_eq!(error, "be.error.finance.marketingCampaignUnavailable");
}

#[test]
fn request_marketing_campaign_respects_cooldown() {
    let mut game = make_monday_game();
    game.teams[0].wage_budget = 50_000;
    game.teams[0].finance = -10_000;

    finances::request_marketing_campaign(&mut game, "team1").expect("first campaign");

    let error = finances::request_marketing_campaign(&mut game, "team1")
        .expect_err("second campaign should fail");

    assert_eq!(error, "be.error.finance.marketingCampaignCoolingDown");
}

#[test]
fn team_finance_snapshot_reports_marketing_campaign_cooldown() {
    let mut game = make_monday_game();
    game.teams[0].financial_ledger.push(FinancialTransaction {
        date: "2025-06-02".to_string(),
        description: "Marketing campaign merchandise revenue".to_string(),
        amount: 120_000,
        kind: FinancialTransactionKind::CommercialCampaign,
    });

    let snapshot = finances::team_finance_snapshot(&game, "team1").expect("snapshot");

    assert_eq!(snapshot.marketing_campaign_cooldown_days_remaining, 14);
}

#[test]
fn calc_matchday_uses_explicit_attendance_and_ticket_inputs() {
    let revenue = finances::calc_matchday(40_000, 2, 0.75, 20.0);

    assert_eq!(revenue, 1_200_000);
}

#[test]
fn calc_upkeep_defaults_to_zero_for_now() {
    let game = make_monday_game();

    let upkeep = finances::calc_upkeep(&game.teams[0]);

    assert_eq!(upkeep, 0);
}

#[test]
fn evaluate_sponsorship_bonus_sums_met_criteria_for_team_context() {
    let sponsorship = Sponsorship {
        sponsor_name: "Acme Corp".to_string(),
        base_value: 100_000,
        remaining_weeks: 8,
        bonus_criteria: vec![
            SponsorshipBonusCriterion::LeaguePosition {
                max_position: 2,
                bonus_amount: 50_000,
            },
            SponsorshipBonusCriterion::UnbeatenRun {
                required_matches: 3,
                bonus_amount: 25_000,
            },
        ],
    };

    let bonus = finances::evaluate_sponsorship_bonus(
        Some(1),
        &["W".to_string(), "D".to_string(), "W".to_string()],
        &sponsorship,
    );

    assert_eq!(bonus, 75_000);
}

#[test]
fn weekly_sponsorship_payout_is_applied_and_duration_decrements_on_monday() {
    let mut game = make_monday_game();
    let initial_finance = game.teams[0].finance;
    game.teams[0].form = vec!["W".to_string(), "D".to_string(), "W".to_string()];
    game.teams[0].sponsorship = Some(Sponsorship {
        sponsor_name: "Acme Corp".to_string(),
        base_value: 100_000,
        remaining_weeks: 2,
        bonus_criteria: vec![SponsorshipBonusCriterion::UnbeatenRun {
            required_matches: 3,
            bonus_amount: 25_000,
        }],
    });

    finances::process_weekly_finances(&mut game);

    let wages = (52_000 + 26_000 + 10_400) / 52;
    let expected_sponsor_income = 125_000;
    assert_eq!(
        game.teams[0].finance,
        initial_finance - wages + expected_sponsor_income
    );
    assert_eq!(game.teams[0].season_income, expected_sponsor_income);
    assert_eq!(
        game.teams[0].sponsorship.as_ref().unwrap().remaining_weeks,
        1
    );
}

#[test]
fn sponsorship_expires_after_the_final_weekly_tick() {
    let mut game = make_monday_game();
    game.teams[0].sponsorship = Some(Sponsorship {
        sponsor_name: "Acme Corp".to_string(),
        base_value: 100_000,
        remaining_weeks: 1,
        bonus_criteria: vec![],
    });

    finances::process_weekly_finances(&mut game);

    assert!(game.teams[0].sponsorship.is_none());
}

#[test]
fn wages_deducted_on_monday() {
    let mut game = make_monday_game();
    let initial_finance = game.teams[0].finance;

    finances::process_weekly_finances(&mut game);

    // Weekly wages: (52000+26000+10400)/52 = 1700
    let expected_deduction = (52_000 + 26_000 + 10_400) / 52;
    assert_eq!(
        game.teams[0].finance,
        initial_finance - expected_deduction,
        "Finance should be reduced by weekly wages"
    );
}

#[test]
fn season_expenses_tracked() {
    let mut game = make_monday_game();
    assert_eq!(game.teams[0].season_expenses, 0);

    finances::process_weekly_finances(&mut game);

    let expected = (52_000 + 26_000 + 10_400) / 52;
    assert_eq!(game.teams[0].season_expenses, expected);
}

#[test]
fn no_processing_on_non_monday() {
    let mut game = make_monday_game();
    // Change to Tuesday
    game.clock.current_date = Utc.with_ymd_and_hms(2025, 6, 17, 12, 0, 0).unwrap();
    let initial_finance = game.teams[0].finance;

    finances::process_weekly_finances(&mut game);

    assert_eq!(
        game.teams[0].finance, initial_finance,
        "Should not process on non-Monday"
    );
}

// ---------------------------------------------------------------------------
// Financial warnings
// ---------------------------------------------------------------------------

#[test]
fn no_warning_when_finances_healthy() {
    let mut game = make_monday_game();
    game.teams[0].finance = 5_000_000;
    game.manager.satisfaction = 77;

    finances::process_weekly_finances(&mut game);

    let finance_msgs: Vec<_> = game
        .messages
        .iter()
        .filter(|m| m.id.starts_with("finance_") || m.id.starts_with("wage_"))
        .collect();
    assert!(
        finance_msgs.is_empty(),
        "No warning when finances are healthy"
    );
    assert_eq!(game.manager.satisfaction, 77);
}

#[test]
fn warning_finances_reduce_board_satisfaction_midseason() {
    let mut game = make_monday_game();
    game.teams[0].wage_budget = 80_000;
    game.manager.satisfaction = 60;

    finances::process_weekly_finances(&mut game);

    assert_eq!(game.manager.satisfaction, 58);

    let pressure_msgs: Vec<_> = game
        .messages
        .iter()
        .filter(|m| m.id.starts_with("finance_board_pressure_"))
        .collect();
    assert_eq!(pressure_msgs.len(), 1);
    assert_eq!(
        pressure_msgs[0].subject_key.as_deref(),
        Some("be.msg.financeBoardPressure.subject")
    );
    assert_eq!(
        pressure_msgs[0].body_key.as_deref(),
        Some("be.msg.financeBoardPressure.bodyWarning")
    );
}

#[test]
fn critical_finances_reduce_board_satisfaction_more_aggressively() {
    let mut game = make_monday_game();
    game.teams[0].finance = 3_400;
    game.manager.satisfaction = 60;

    finances::process_weekly_finances(&mut game);

    assert_eq!(game.manager.satisfaction, 56);

    let pressure_msgs: Vec<_> = game
        .messages
        .iter()
        .filter(|m| m.id.starts_with("finance_board_pressure_"))
        .collect();
    assert_eq!(pressure_msgs.len(), 1);
    assert_eq!(
        pressure_msgs[0].body_key.as_deref(),
        Some("be.msg.financeBoardPressure.bodyCritical")
    );
    assert_eq!(pressure_msgs[0].priority, domain::message::MessagePriority::Urgent);
}

#[test]
fn repeated_critical_finance_weeks_continue_to_reduce_satisfaction() {
    let mut game = make_monday_game();
    game.teams[0].finance = -50_000;
    game.manager.satisfaction = 80;

    finances::process_weekly_finances(&mut game);
    game.clock.current_date += chrono::Duration::days(7);
    finances::process_weekly_finances(&mut game);
    game.clock.current_date += chrono::Duration::days(7);
    finances::process_weekly_finances(&mut game);

    assert_eq!(game.manager.satisfaction, 68);
    assert_eq!(
        game.messages
            .iter()
            .filter(|message| message.id.starts_with("finance_board_pressure_"))
            .count(),
        3
    );
}

#[test]
fn critical_warning_when_in_debt() {
    let mut game = make_monday_game();
    game.teams[0].finance = -100_000;

    finances::process_weekly_finances(&mut game);

    let critical_msgs: Vec<_> = game
        .messages
        .iter()
        .filter(|m| m.id.starts_with("finance_critical_"))
        .collect();
    assert_eq!(
        critical_msgs.len(),
        1,
        "Should send critical warning when in debt"
    );
    assert_eq!(
        critical_msgs[0].subject_key.as_deref(),
        Some("be.msg.financeCritical.subject")
    );
    assert_eq!(
        critical_msgs[0].body_key.as_deref(),
        Some("be.msg.financeCritical.body")
    );
    assert_eq!(
        critical_msgs[0].sender_key.as_deref(),
        Some("be.sender.boardOfDirectors")
    );
    assert!(critical_msgs[0].subject.is_empty());
    assert!(critical_msgs[0].body.is_empty());
    assert!(critical_msgs[0].sender.is_empty());
}

#[test]
fn warning_when_low_runway() {
    let mut game = make_monday_game();
    // Set finance to ~2 weeks of wages (weekly wages ~1700, so ~3400)
    game.teams[0].finance = 3400;

    finances::process_weekly_finances(&mut game);

    let warning_msgs: Vec<_> = game
        .messages
        .iter()
        .filter(|m| m.id.starts_with("finance_warning_"))
        .collect();
    // After deducting wages (1700), finance=1700, weeks_left=1700/1700=1 → < 4
    assert_eq!(warning_msgs.len(), 1, "Should send low reserves warning");
}

#[test]
fn sponsorship_income_prevents_false_low_runway_warning() {
    let mut game = make_monday_game();
    game.teams[0].finance = 3_400;
    game.teams[0].sponsorship = Some(Sponsorship {
        sponsor_name: "Acme Corp".to_string(),
        base_value: 2_000,
        remaining_weeks: 8,
        bonus_criteria: vec![],
    });

    finances::process_weekly_finances(&mut game);

    let warning_msgs: Vec<_> = game
        .messages
        .iter()
        .filter(|m| m.id.starts_with("finance_warning_"))
        .collect();
    assert!(
        warning_msgs.is_empty(),
        "Positive sponsorship support should avoid a false runway warning"
    );
}

#[test]
fn wage_over_budget_warning() {
    let mut game = make_monday_game();
    game.teams[0].finance = 5_000_000; // healthy
    game.teams[0].wage_budget = 50_000; // very low budget

    // Annual wages = (52000+26000+10400) = 88400 > 50000 budget
    finances::process_weekly_finances(&mut game);

    let budget_msgs: Vec<_> = game
        .messages
        .iter()
        .filter(|m| m.id.starts_with("wage_over_budget_"))
        .collect();
    assert_eq!(
        budget_msgs.len(),
        1,
        "Should warn about exceeding wage budget"
    );
}

#[test]
fn financial_warnings_not_duplicated() {
    let mut game = make_monday_game();
    game.teams[0].finance = -100_000;

    finances::process_weekly_finances(&mut game);
    // Process again on same day (shouldn't add duplicate)
    finances::process_weekly_finances(&mut game);

    let critical_msgs: Vec<_> = game
        .messages
        .iter()
        .filter(|m| m.id.starts_with("finance_critical_"))
        .collect();
    // Note: process only runs on Monday, so second call on same Monday
    // The message dedup uses the date-based ID
    assert_eq!(
        critical_msgs.len(),
        1,
        "Should not duplicate critical warning"
    );
}

#[test]
fn no_warning_without_manager_team() {
    let mut game = make_monday_game();
    game.manager.team_id = None;
    game.teams[0].finance = -100_000;

    finances::process_weekly_finances(&mut game);

    let finance_msgs: Vec<_> = game
        .messages
        .iter()
        .filter(|m| m.id.starts_with("finance_"))
        .collect();
    assert!(finance_msgs.is_empty(), "No warning without manager team");
}

// ---------------------------------------------------------------------------
// Matchday income
// ---------------------------------------------------------------------------

#[test]
fn home_match_generates_income() {
    let mut game = make_monday_game();
    let initial_finance = game.teams[0].finance;

    // Add a completed home fixture within the last 7 days
    let league = League {
        id: "l1".to_string(),
        name: "Test League".to_string(),
        season: 1,
        fixtures: vec![Fixture {
            id: "f1".to_string(),
            matchday: 1,
            date: "2025-06-14".to_string(), // Saturday, within 7 days of Monday 2025-06-16
            home_team_id: "team1".to_string(),
            away_team_id: "team2".to_string(),
            competition: FixtureCompetition::League,
            status: FixtureStatus::Completed,
            result: Some(MatchResult {
                home_goals: 2,
                away_goals: 1,
                home_scorers: vec![],
                away_scorers: vec![],
                report: None,
            }),
        }],
        standings: vec![StandingEntry::new("team1".to_string())],
        transfer_log: vec![],
        transfer_rumours: vec![],
    };
    game.league = Some(league);

    finances::process_weekly_finances(&mut game);

    // After wage deduction AND matchday income
    let wages = (52_000 + 26_000 + 10_400) / 52;
    // Income should make final finance > initial - wages
    // (stadium capacity 40000, attendance 60-92%, ticket €15-25)
    // Min income: 40000 * 0.60 * 15 = 360,000
    let final_finance = game.teams[0].finance;
    assert!(
        final_finance > initial_finance - wages,
        "Matchday income should offset some wages. Got {} (started {}, wages {})",
        final_finance,
        initial_finance,
        wages
    );
}

#[test]
fn away_match_no_income() {
    let mut game = make_monday_game();

    // Add a completed away fixture (team1 is away)
    let league = League {
        id: "l1".to_string(),
        name: "Test League".to_string(),
        season: 1,
        fixtures: vec![Fixture {
            id: "f1".to_string(),
            matchday: 1,
            date: "2025-06-14".to_string(),
            home_team_id: "team2".to_string(), // team1 is away
            away_team_id: "team1".to_string(),
            competition: FixtureCompetition::League,
            status: FixtureStatus::Completed,
            result: Some(MatchResult {
                home_goals: 0,
                away_goals: 1,
                home_scorers: vec![],
                away_scorers: vec![],
                report: None,
            }),
        }],
        standings: vec![StandingEntry::new("team1".to_string())],
        transfer_log: vec![],
        transfer_rumours: vec![],
    };
    game.league = Some(league);

    let initial_finance = game.teams[0].finance;
    finances::process_weekly_finances(&mut game);

    let wages = (52_000 + 26_000 + 10_400) / 52;
    assert_eq!(
        game.teams[0].finance,
        initial_finance - wages,
        "Away match should generate no income for team1"
    );
}

// ---------------------------------------------------------------------------
// Multiple teams
// ---------------------------------------------------------------------------

#[test]
fn multiple_teams_processed_independently() {
    let mut game = make_monday_game();
    let mut team2 = make_team("team2", "Rival FC");
    team2.finance = 3_000_000;
    game.teams.push(team2);

    let p3 = make_player("p3", "team2", 104_000); // 2000/week
    game.players.push(p3);

    let initial_t1 = game.teams[0].finance;
    let initial_t2 = game.teams[1].finance;

    finances::process_weekly_finances(&mut game);

    let t1_wages = (52_000 + 26_000 + 10_400) / 52; // 1700
    let t2_wages = 104_000 / 52; // 2000
    assert_eq!(game.teams[0].finance, initial_t1 - t1_wages);
    assert_eq!(game.teams[1].finance, initial_t2 - t2_wages);
}
