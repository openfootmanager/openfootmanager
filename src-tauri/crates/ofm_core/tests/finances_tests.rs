use chrono::{TimeZone, Utc};
use domain::league::{
    Fixture, FixtureCompetition, FixtureStatus, League, MatchResult, StandingEntry,
};
use domain::manager::Manager;
use domain::player::{ActiveLoan, Player, PlayerAttributes, Position};
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
    p.stage_wage(wage);
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
    let p1 = make_player("p1", "team1", 1_000);
    let p2 = make_player("p2", "team1", 500);
    let s1 = make_staff("s1", "team1", 200);

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

    assert_eq!(snapshot.annual_wage_bill, 1_700);
    assert_eq!(snapshot.weekly_wage_spend, 1_700);
    assert_eq!(snapshot.weekly_wage_budget, 2_000_000);
    assert_eq!(snapshot.weekly_sponsor_income, 2_000);
    assert_eq!(snapshot.weekly_recurring_income, 2_000);
    assert_eq!(snapshot.projected_weekly_net, 300);
    assert_eq!(snapshot.cash_runway_weeks, None);
    assert_eq!(snapshot.wage_budget_usage_percent, 0);
    assert!(!snapshot.currently_in_debt);
    assert!(!snapshot.currently_over_budget);
}

#[test]
fn team_finance_snapshot_flags_wage_pressure() {
    let mut game = make_monday_game();
    game.teams[0].wage_budget = 1_545;

    let snapshot = finances::team_finance_snapshot(&game, "team1").expect("snapshot");

    assert!(snapshot.currently_over_budget);
    assert_eq!(snapshot.wage_budget_usage_percent, 110);
    // 1,700 on 1,545 is 110.03%: past the Critical line, though the whole-number percentage
    // reads 110.
    assert_eq!(
        snapshot.wage_budget_status,
        finances::FinanceHealthLevel::Critical
    );
    assert_eq!(
        snapshot.overall_status,
        finances::FinanceHealthLevel::Critical
    );
}

/// Given a club whose wage bill of 1,700 is one over its budget of 1,699, about 100.06%
/// When its finances are summarised
/// Then it is over budget, and its wage status stays at Watch: Warning starts at 101%
#[test]
fn a_wage_bill_a_fraction_over_budget_is_over_budget_but_only_on_watch() {
    let mut game = make_monday_game();
    game.teams[0].wage_budget = 1_699;

    let snapshot = finances::team_finance_snapshot(&game, "team1").expect("snapshot");

    assert_eq!(snapshot.wage_budget_usage_percent, 100);
    assert!(snapshot.currently_over_budget);
    assert_eq!(
        snapshot.wage_budget_status,
        finances::FinanceHealthLevel::Watch
    );
}

/// Given a club whose wage bill of 1,700 is exactly its budget
/// When its finances are summarised
/// Then it is not over budget, and its wage status is Watch: spending all of it is not a breach
#[test]
fn a_wage_bill_exactly_on_budget_is_a_watch_not_a_warning() {
    let mut game = make_monday_game();
    game.teams[0].wage_budget = 1_700;

    let snapshot = finances::team_finance_snapshot(&game, "team1").expect("snapshot");

    assert!(!snapshot.currently_over_budget);
    assert_eq!(
        snapshot.wage_budget_status,
        finances::FinanceHealthLevel::Watch
    );
}

/// The wage status of a club whose players cost 1,500 and whose one staff member costs
/// `staff_wage`, against `wage_budget`.
fn wage_status_with(staff_wage: u32, wage_budget: i64) -> finances::FinanceHealthLevel {
    let mut game = make_monday_game();
    game.staff[0].wage = staff_wage;
    game.teams[0].wage_budget = wage_budget;
    finances::team_finance_snapshot(&game, "team1")
        .expect("snapshot")
        .wage_budget_status
}

/// Given a club with no players, no staff and a wage budget of 0
/// When its finances are summarised
/// Then its wages are Stable: nothing owed against nothing budgeted is no pressure at all
#[test]
fn no_wages_against_no_budget_is_stable() {
    let mut game = make_monday_game();
    game.players.clear();
    game.staff.clear();
    game.teams[0].wage_budget = 0;

    let snapshot = finances::team_finance_snapshot(&game, "team1").expect("snapshot");

    assert_eq!(
        (snapshot.currently_over_budget, snapshot.wage_budget_status),
        (false, finances::FinanceHealthLevel::Stable)
    );
}

/// Given a club whose wage bill of 1,515 is exactly 101% of its 1,500 budget
/// When its finances are summarised
/// Then its wages are at Warning: 101% is where Warning starts
#[test]
fn a_wage_bill_of_exactly_101_percent_is_a_warning() {
    assert_eq!(
        wage_status_with(15, 1_500),
        finances::FinanceHealthLevel::Warning
    );
}

/// Given a club whose wage bill of 1,514 is one short of 101% of its 1,500 budget
/// When its finances are summarised
/// Then its wages are on Watch, though over budget
#[test]
fn a_wage_bill_one_under_101_percent_is_on_watch() {
    assert_eq!(
        wage_status_with(14, 1_500),
        finances::FinanceHealthLevel::Watch
    );
}

/// Given a club whose wage bill of 1,650 is exactly 110% of its 1,500 budget
/// When its finances are summarised
/// Then its wages are at Warning: 110% is the last bill that is not Critical
#[test]
fn a_wage_bill_of_exactly_110_percent_is_a_warning_not_critical() {
    assert_eq!(
        wage_status_with(150, 1_500),
        finances::FinanceHealthLevel::Warning
    );
}

/// Given a club whose wage bill of 1,651 is one over 110% of its 1,500 budget
/// When its finances are summarised
/// Then its wages are Critical, though the whole-number percentage reads 110
#[test]
fn a_wage_bill_one_over_110_percent_is_critical() {
    assert_eq!(
        wage_status_with(151, 1_500),
        finances::FinanceHealthLevel::Critical
    );
}

/// Given a club whose wage bill of 1,700 is exactly 85% of its 2,000 budget
/// When its finances are summarised
/// Then its wages are on Watch
#[test]
fn a_wage_bill_of_exactly_85_percent_is_on_watch() {
    assert_eq!(
        wage_status_with(200, 2_000),
        finances::FinanceHealthLevel::Watch
    );
}

/// Given a club whose wage bill of 1,699 is one short of 85% of its 2,000 budget
/// When its finances are summarised
/// Then its wages are Stable
#[test]
fn a_wage_bill_one_under_85_percent_is_stable() {
    assert_eq!(
        wage_status_with(199, 2_000),
        finances::FinanceHealthLevel::Stable
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

/// Given a club in good financial health with funds for the first upgrade
/// When the finance previews are read
/// Then each facility reports its next cost, can be afforded, and has no block reason.
#[test]
fn facility_upgrade_previews_report_cost_and_availability_for_a_healthy_club() {
    let mut game = make_monday_game();
    game.teams[0].finance = 10_000_000;
    game.teams[0].facilities.training = 2;

    let previews = finances::finance_action_previews(&game, "team1").expect("previews");

    let training = previews
        .facility_upgrades
        .iter()
        .find(|preview| preview.facility == "Training")
        .expect("training preview");
    assert_eq!(previews.facility_upgrades.len(), 3);
    assert_eq!(training.next_cost, 500_000);
    assert!(training.can_afford);
    assert_eq!(training.block_reason, None);
}

/// Given a club whose wage bill is over its budget and which cannot afford an upgrade
/// When the finance previews are read
/// Then every facility carries the backend's block reason and reports it cannot be afforded.
#[test]
fn facility_upgrade_previews_carry_the_block_reason_and_affordability() {
    let mut game = make_monday_game();
    game.teams[0].finance = 1_000;
    game.teams[0].wage_budget = 1_000;

    let previews = finances::finance_action_previews(&game, "team1").expect("previews");

    assert_eq!(previews.facility_upgrades.len(), 3);
    assert!(previews.facility_upgrades.iter().all(|preview| {
        !preview.can_afford
            && preview.block_reason.as_deref() == Some("be.error.finance.facilityUpgradeOverBudget")
    }));
}

fn snapshot_at(overall_status: finances::FinanceHealthLevel) -> finances::TeamFinanceSnapshot {
    finances::TeamFinanceSnapshot {
        annual_wage_bill: 520_000,
        weekly_wage_spend: 10_000,
        weekly_wage_budget: 20_000,
        weekly_recurring_income: 15_000,
        weekly_sponsor_income: 5_000,
        projected_weekly_net: 5_000,
        cash_runway_weeks: None,
        wage_budget_usage_percent: 50,
        currently_in_debt: false,
        currently_over_budget: false,
        wage_budget_status: finances::FinanceHealthLevel::Stable,
        runway_status: overall_status,
        overall_status,
        marketing_campaign_cooldown_days_remaining: 0,
    }
}

/// Given a club whose overall finances are at warning or critical level, within its wage budget
/// When its facility upgrades are judged
/// Then they are blocked for financial distress
#[test]
fn a_club_in_warning_or_critical_distress_cannot_upgrade_facilities() {
    for status in [
        finances::FinanceHealthLevel::Warning,
        finances::FinanceHealthLevel::Critical,
    ] {
        assert_eq!(
            finances::facility_upgrade_block_reason(&snapshot_at(status)),
            Some("be.error.finance.facilityUpgradeCritical"),
            "{status:?}"
        );
    }
}

/// Given a club whose overall finances are stable or only on watch, within its wage budget
/// When its facility upgrades are judged
/// Then nothing blocks them
#[test]
fn a_stable_or_watched_club_may_upgrade_facilities() {
    for status in [
        finances::FinanceHealthLevel::Stable,
        finances::FinanceHealthLevel::Watch,
    ] {
        assert_eq!(
            finances::facility_upgrade_block_reason(&snapshot_at(status)),
            None,
            "{status:?}"
        );
    }
}

/// Given a club that is over its wage budget and also in critical distress
/// When its facility upgrades are judged
/// Then the reason given is the wage budget
#[test]
fn being_over_the_wage_budget_outranks_distress_as_the_reason() {
    let mut snapshot = snapshot_at(finances::FinanceHealthLevel::Critical);
    snapshot.currently_over_budget = true;

    assert_eq!(
        finances::facility_upgrade_block_reason(&snapshot),
        Some("be.error.finance.facilityUpgradeOverBudget")
    );
}

#[test]
fn request_board_support_can_recover_runway_without_random_events() {
    let mut game = make_monday_game();
    game.teams[0].finance = 13_600;

    let preview = finances::preview_board_support(&game, "team1").expect("preview");
    let result = finances::request_board_support(&mut game, "team1").expect("support");
    let snapshot = finances::team_finance_snapshot(&game, "team1").expect("snapshot");

    assert_eq!(result.support_amount, preview.support_amount);
    assert_eq!(
        result.transfer_budget_reduction,
        preview.transfer_budget_reduction
    );
    assert_eq!(result.satisfaction_penalty, preview.satisfaction_penalty);
    assert_eq!(
        snapshot.overall_status,
        finances::FinanceHealthLevel::Stable
    );
}

#[test]
fn request_sponsor_pitch_creates_pending_offer_for_over_budget_team() {
    let mut game = make_monday_game();
    game.teams[0].wage_budget = 1_000;

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
    assert_eq!(
        message.subject_key.as_deref(),
        Some("be.msg.sponsor.subject")
    );
    assert_eq!(message.body_key.as_deref(), Some("be.msg.sponsor.body"));
    assert_eq!(
        message.sender_key.as_deref(),
        Some("be.sender.commercialDirector")
    );
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
    game.teams[0].wage_budget = 1_000;
    finances::request_sponsor_pitch(&mut game, "team1").expect("first pitch");

    let error = finances::request_sponsor_pitch(&mut game, "team1")
        .expect_err("second pending pitch should fail");

    assert_eq!(error, "be.error.finance.sponsorPitchPendingOffer");
}

#[test]
fn request_sponsor_pitch_stays_capped_at_one_a_day_after_the_offer_is_gone() {
    // The daily cap is the only thing stopping a club pitching sponsors on a
    // loop. It has to survive the offer leaving the inbox — deleted by the
    // player, or resolved and later purged — which is the whole point of the
    // sent-ledger.
    let mut game = make_monday_game();
    game.teams[0].wage_budget = 1_000;
    let first = finances::request_sponsor_pitch(&mut game, "team1").expect("first pitch");

    game.messages
        .retain(|message| message.id != first.message_id);

    let error = finances::request_sponsor_pitch(&mut game, "team1")
        .expect_err("a second pitch on the same day should be refused");

    assert_eq!(error, "be.error.finance.sponsorPitchAlreadyAttemptedToday");
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
    assert_eq!(
        message.subject_key.as_deref(),
        Some("be.msg.marketingCampaign.subject")
    );
    assert_eq!(
        message.body_key.as_deref(),
        Some("be.msg.marketingCampaign.body")
    );
    assert_eq!(
        message.sender_key.as_deref(),
        Some("be.sender.commercialDirector")
    );
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
    game.teams[0].wage_budget = 1_000;
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

    let wages = 1_700;
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

    let expected_deduction = 1_700;
    assert_eq!(
        game.teams[0].finance,
        initial_finance - expected_deduction,
        "Finance should be reduced by weekly wages"
    );
}

#[test]
fn stored_weekly_wage_is_posted_in_full_on_monday() {
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
    let mut game = Game::new(
        clock,
        manager,
        vec![make_team("team1", "Test FC")],
        vec![make_player("p1", "team1", 5_000)],
        vec![],
        vec![],
    );
    let initial_finance = game.teams[0].finance;

    finances::process_weekly_finances(&mut game);

    assert_eq!(game.teams[0].finance, initial_finance - 5_000);
    let wages = game
        .cash_journal
        .iter()
        .find(|post| post.kind == finances::CashKind::PlayerWages)
        .expect("player wage post");
    assert_eq!(wages.amount, -5_000);
    assert_eq!(wages.club_id, "team1");
}

#[test]
fn monday_payday_splits_loan_wages_between_parent_and_loanee() {
    let date = Utc.with_ymd_and_hms(2025, 6, 16, 12, 0, 0).unwrap();
    let clock = GameClock::new(date);
    let mut manager = Manager::new(
        "mgr1".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("parent".to_string());
    let mut loanee = make_player("loaned", "loanee", 10_000);
    loanee.active_loan = Some(ActiveLoan {
        parent_team_id: "parent".to_string(),
        loan_team_id: "loanee".to_string(),
        start_date: "2025-01-01".to_string(),
        end_date: "2026-06-30".to_string(),
        wage_contribution_pct: 50,
        buy_option_fee: None,
        loan_start_minutes: 0,
        loan_start_appearances: 0,
        development_reported_minutes: 0,
        development_reported_appearances: 0,
    });
    let mut game = Game::new(
        clock,
        manager,
        vec![
            make_team("parent", "Parent FC"),
            make_team("loanee", "Loan FC"),
        ],
        vec![loanee],
        vec![],
        vec![],
    );
    let parent_finance = game.teams[0].finance;
    let loanee_finance = game.teams[1].finance;

    finances::process_weekly_finances(&mut game);

    assert_eq!(game.teams[0].finance, parent_finance - 5_000);
    assert_eq!(game.teams[1].finance, loanee_finance - 5_000);
    assert_eq!(finances::calc_wages(&game, "parent"), 5_000);
    assert_eq!(finances::calc_wages(&game, "loanee"), 5_000);
}

#[test]
fn weekly_unit_runway_floor_credits_clubs_below_sixteen_weeks() {
    let mut game = make_monday_game();
    game.teams[0].finance = 1_000;

    assert!(finances::apply_weekly_unit_runway_floor(&mut game));
    assert_eq!(
        game.teams[0].finance,
        1_700 * finances::MIN_OPENING_RUNWAY_WEEKS
    );
    assert!(finances::journal_matches_cash(&game));
    assert!(!finances::apply_weekly_unit_runway_floor(&mut game));
}

#[test]
fn season_expenses_tracked() {
    let mut game = make_monday_game();
    assert_eq!(game.teams[0].season_expenses, 0);

    finances::process_weekly_finances(&mut game);

    let expected = 1_700;
    assert_eq!(game.teams[0].season_expenses, expected);
}

#[test]
fn weekly_wages_are_charged_to_each_member_own_team() {
    // Given: two teams on a Monday, each with its own players and staff
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

    let team1 = make_team("team1", "Alpha FC");
    let team2 = make_team("team2", "Beta FC");
    let a_player = make_player("a1", "team1", 1_000);
    let a_staff = make_staff("as", "team1", 200);
    let b_player1 = make_player("b1", "team2", 500);
    let b_player2 = make_player("b2", "team2", 500);

    let mut game = Game::new(
        clock,
        manager,
        vec![team1, team2],
        vec![a_player, b_player1, b_player2],
        vec![a_staff],
        vec![],
    );
    let finance_of = |g: &Game, id: &str| g.teams.iter().find(|t| t.id == id).unwrap().finance;
    let t1_initial = finance_of(&game, "team1");
    let t2_initial = finance_of(&game, "team2");

    // When: the weekly tick runs
    finances::process_weekly_finances(&mut game);

    // Then: each team is charged exactly its OWN members' wages, not the other's
    assert_eq!(finance_of(&game, "team1"), t1_initial - 1_200);
    assert_eq!(finance_of(&game, "team2"), t2_initial - 1_000);
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

/// Given a club whose wage bill of 1,700 is about 100.5% of its 1,692 budget
/// When the weekly finances run
/// Then the board neither docks satisfaction nor sends a pressure message: a bill under 101% of
///      budget stays penalty-free, as it always was (maintainer ruling on #978)
#[test]
fn a_wage_bill_half_a_percent_over_budget_draws_no_board_penalty() {
    let mut game = make_monday_game();
    game.teams[0].wage_budget = 1_692;
    game.manager.satisfaction = 60;

    finances::process_weekly_finances(&mut game);

    let pressure: Vec<Option<&str>> = game
        .messages
        .iter()
        .filter(|m| m.id.starts_with("finance_board_pressure_"))
        .map(|m| m.body_key.as_deref())
        .collect();
    assert_eq!(
        (game.manager.satisfaction, pressure),
        (60, Vec::<Option<&str>>::new())
    );
}

#[test]
fn warning_finances_reduce_board_satisfaction_midseason() {
    let mut game = make_monday_game();
    // 1,700 on 1,600 is 106%: over budget, short of Critical's 110%.
    game.teams[0].wage_budget = 1_600;
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
    assert_eq!(
        pressure_msgs[0].priority,
        domain::message::MessagePriority::Urgent
    );
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

/// Given a club 100,000 in debt, with 1,700 of wages to pay that week,
/// When the weekly finances run,
/// Then the board's warning carries the debt after the wages (101,700) as a whole number, not as a formatted string.
#[test]
fn the_debt_warning_carries_the_amount_as_a_number() {
    let mut game = make_monday_game();
    game.teams[0].finance = -100_000;

    finances::process_weekly_finances(&mut game);

    let message = game
        .messages
        .iter()
        .find(|m| m.id.starts_with("finance_critical_"))
        .expect("the board warns of the debt");
    assert_eq!(message.i18n_params["amount"], "101700");
}

/// Given a club with two weeks of cash and a 1,700 weekly wage bill,
/// When the weekly finances run,
/// Then the runway warning carries the weekly wages as a whole number.
#[test]
fn the_runway_warning_carries_the_weekly_wages_as_a_number() {
    let mut game = make_monday_game();
    game.teams[0].finance = 3400;

    finances::process_weekly_finances(&mut game);

    let message = game
        .messages
        .iter()
        .find(|m| m.id.starts_with("finance_warning_"))
        .expect("the finance director warns of the runway");
    assert_eq!(message.i18n_params["weeklyWages"], "1700");
}

/// Given a club with cash to spare whose 1,700 weekly wage bill is over its 1,545 budget,
/// When the weekly finances run,
/// Then the message names the figure for what it is, the weekly wage bill, as whole numbers.
#[test]
fn the_over_budget_message_carries_the_weekly_wage_bill_and_budget_as_numbers() {
    let mut game = make_monday_game();
    game.teams[0].finance = 1_000_000;
    game.teams[0].wage_budget = 1_545;

    finances::process_weekly_finances(&mut game);

    let message = game
        .messages
        .iter()
        .find(|m| m.id.starts_with("wage_over_budget_"))
        .expect("the finance director warns of the wage bill");
    assert_eq!(message.i18n_params["weeklyWageBill"], "1700");
    assert_eq!(message.i18n_params["wageBudget"], "1545");
    assert!(
        !message.i18n_params.contains_key("annualWages"),
        "the weekly bill is no longer called annual"
    );
}

/// Given a club with cash to spare whose 1,700 weekly wage bill is exactly its 1,700 budget,
/// When the weekly finances run,
/// Then it is not over budget, and the finance director says nothing.
#[test]
fn a_wage_bill_equal_to_the_budget_is_not_over_it() {
    let mut game = make_monday_game();
    game.teams[0].finance = 1_000_000;
    game.teams[0].wage_budget = 1_700;

    finances::process_weekly_finances(&mut game);

    assert!(
        !game
            .messages
            .iter()
            .any(|m| m.id.starts_with("wage_over_budget_"))
    );
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
    game.teams[0].wage_budget = 1_000;

    // Weekly wages 1_700 > 1_000 budget
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

fn lever_messages(game: &Game) -> Vec<&domain::message::InboxMessage> {
    game.messages
        .iter()
        .filter(|m| m.id.starts_with("finance_levers_"))
        .collect()
}

/// The levers a message offers, by the id of the action that takes the manager to each.
fn offered_levers(message: &domain::message::InboxMessage) -> Vec<&str> {
    message
        .actions
        .iter()
        .map(|action| action.id.as_str())
        .collect()
}

/// Given a club whose wage bill is over its budget but which is not in debt
/// When the weekly finances run
/// Then the financial director offers the two commercial levers the Finances tab would allow, each
///      taking the manager to that tab, and not board support, which the tab would refuse.
#[test]
fn an_over_budget_club_is_offered_exactly_the_commercial_levers() {
    let mut game = make_monday_game();
    game.teams[0].finance = 5_000_000;
    game.teams[0].wage_budget = 1_000;

    finances::process_weekly_finances(&mut game);

    let messages = lever_messages(&game);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].body_key.as_deref(),
        Some("be.msg.financeLevers.body")
    );
    assert_eq!(
        messages[0].sender_key.as_deref(),
        Some("be.sender.financialDirector")
    );
    assert_eq!(
        offered_levers(messages[0]),
        ["lever_sponsor_pitch", "lever_marketing_campaign"]
    );
    assert!(messages[0].actions.iter().all(|action| matches!(
        &action.action_type,
        domain::message::ActionType::NavigateTo { route } if route == "/dashboard?tab=Finances"
    )));
}

/// Given a club in debt
/// When the weekly finances run
/// Then every lever is offered, board support first.
#[test]
fn a_club_in_debt_is_offered_every_lever() {
    let mut game = make_monday_game();
    game.teams[0].finance = -100_000;

    finances::process_weekly_finances(&mut game);

    let messages = lever_messages(&game);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        offered_levers(messages[0]),
        [
            "lever_board_support",
            "lever_sponsor_pitch",
            "lever_marketing_campaign"
        ]
    );
}

/// Given a club in debt with a sponsor already signed and a marketing campaign still cooling down
/// When the weekly finances run
/// Then only board support is offered: the sponsor pitch and the campaign are refused by the tab.
#[test]
fn a_lever_the_tab_would_refuse_is_not_offered() {
    let mut game = make_monday_game();
    // Deep enough that the campaign's income does not lift the club out of debt.
    game.teams[0].finance = -5_000_000;
    game.teams[0].wage_budget = 1_000;
    game.teams[0].sponsorship = Some(Sponsorship {
        sponsor_name: "Acme Corp".to_string(),
        base_value: 2_000,
        remaining_weeks: 8,
        bonus_criteria: vec![],
    });
    finances::request_marketing_campaign(&mut game, "team1").expect("campaign");

    finances::process_weekly_finances(&mut game);

    let messages = lever_messages(&game);
    assert_eq!(messages.len(), 1);
    assert_eq!(offered_levers(messages[0]), ["lever_board_support"]);
}

/// Given an over-budget club with a sponsor signed and a campaign cooling down
/// When the weekly finances run
/// Then no lever message is sent, because nothing is open to it.
#[test]
fn no_message_is_sent_when_no_lever_is_open() {
    let mut game = make_monday_game();
    game.teams[0].finance = 5_000_000;
    game.teams[0].wage_budget = 1_000;
    game.teams[0].sponsorship = Some(Sponsorship {
        sponsor_name: "Acme Corp".to_string(),
        base_value: 2_000,
        remaining_weeks: 8,
        bonus_criteria: vec![],
    });
    finances::request_marketing_campaign(&mut game, "team1").expect("campaign");

    finances::process_weekly_finances(&mut game);

    assert!(lever_messages(&game).is_empty());
}

/// Given a club in good financial health
/// When the weekly finances run
/// Then no lever message is sent.
#[test]
fn a_healthy_club_hears_nothing_about_levers() {
    let mut game = make_monday_game();
    game.teams[0].finance = 5_000_000;

    finances::process_weekly_finances(&mut game);

    assert!(lever_messages(&game).is_empty());
}

/// Given a club that stays in trouble for several weeks and whose manager deletes the message
/// When the weekly finances keep running in the same season with the same levers open
/// Then the message is not sent again: the ledger remembers it, not the mailbox.
#[test]
fn the_lever_message_is_sent_once_per_season_per_open_set_even_after_deletion() {
    let mut game = make_monday_game();
    game.teams[0].finance = -100_000;

    finances::process_weekly_finances(&mut game);
    game.messages
        .retain(|m| !m.id.starts_with("finance_levers_"));
    game.clock.current_date += chrono::Duration::days(7);
    finances::process_weekly_finances(&mut game);

    assert!(lever_messages(&game).is_empty());
}

/// Given a club in debt told about every lever, whose sponsor lever then closes (a sponsor signs)
///       and later reopens (the sponsor leaves), all in one season
/// When the weekly finances run each week
/// Then it is not told again: a lever closing never triggers a message, and a lever that reopens
///      has already been announced this season.
#[test]
fn levers_that_close_and_reopen_in_a_season_are_not_announced_again() {
    let mut game = make_monday_game();
    game.teams[0].finance = -100_000;
    finances::process_weekly_finances(&mut game);
    assert_eq!(lever_messages(&game).len(), 1);

    game.teams[0].sponsorship = Some(Sponsorship {
        sponsor_name: "Acme Corp".to_string(),
        base_value: 2_000,
        remaining_weeks: 8,
        bonus_criteria: vec![],
    });
    game.clock.current_date += chrono::Duration::days(7);
    finances::process_weekly_finances(&mut game);
    game.teams[0].sponsorship = None;
    game.clock.current_date += chrono::Duration::days(7);
    finances::process_weekly_finances(&mut game);

    assert_eq!(lever_messages(&game).len(), 1);
}

/// Given a club told only about the levers open to it (a sponsor is signed, so no pitch)
/// When the sponsor leaves and the pitch becomes a lever it has not been told about
/// Then it is told again, and the message names every lever open now.
#[test]
fn a_lever_that_opens_for_the_first_time_in_a_season_is_announced() {
    let mut game = make_monday_game();
    game.teams[0].finance = 5_000_000;
    game.teams[0].wage_budget = 1_000;
    game.teams[0].sponsorship = Some(Sponsorship {
        sponsor_name: "Acme Corp".to_string(),
        base_value: 2_000,
        remaining_weeks: 8,
        bonus_criteria: vec![],
    });
    finances::process_weekly_finances(&mut game);
    assert_eq!(lever_messages(&game).len(), 1);
    assert_eq!(
        offered_levers(lever_messages(&game)[0]),
        ["lever_marketing_campaign"]
    );

    game.teams[0].sponsorship = None;
    game.clock.current_date += chrono::Duration::days(7);
    finances::process_weekly_finances(&mut game);

    let messages = lever_messages(&game);
    assert_eq!(messages.len(), 2);
    assert_eq!(
        offered_levers(messages[1]),
        ["lever_sponsor_pitch", "lever_marketing_campaign"]
    );
}

/// Given a club told about the levers that is still in trouble a season later
/// When the weekly finances run in the new season
/// Then it is told again, because the key is scoped to the season.
#[test]
fn the_lever_message_is_sent_again_in_a_new_season() {
    let mut game = make_monday_game();
    game.teams[0].finance = -100_000;
    finances::process_weekly_finances(&mut game);
    // 52 weeks later is a Monday again, in the next calendar season.
    game.clock.current_date += chrono::Duration::days(364);

    finances::process_weekly_finances(&mut game);

    assert_eq!(lever_messages(&game).len(), 2);
}

/// Given a club told about the levers, in a season where the league's own season number moves on
///       while the calendar year does not
/// When the weekly finances run again a week later
/// Then it is told again: the key follows the league's season, not only the calendar year.
#[test]
fn the_lever_message_is_sent_again_when_the_league_season_rolls_over() {
    let mut game = make_monday_game();
    game.teams[0].finance = -100_000;
    game.league = Some(League {
        id: "l1".to_string(),
        name: "Test League".to_string(),
        season: 1,
        ..Default::default()
    });
    finances::process_weekly_finances(&mut game);
    assert_eq!(lever_messages(&game).len(), 1);

    game.league.as_mut().expect("league").season = 2;
    game.clock.current_date += chrono::Duration::days(7);
    finances::process_weekly_finances(&mut game);

    assert_eq!(lever_messages(&game).len(), 2);
}

/// Given a manager told about the levers open to their first club, who then moves to a second club
///       in the same season that is also in trouble
/// When the weekly finances run for the new club
/// Then they are told about the new club's levers: the announcement belongs to the club, not to the
///      season alone.
#[test]
fn a_manager_who_changes_club_mid_season_hears_about_the_new_clubs_levers() {
    let mut game = make_monday_game();
    game.teams[0].finance = -100_000;
    let mut other = make_team("team2", "Other FC");
    other.finance = -100_000;
    game.teams.push(other);
    finances::process_weekly_finances(&mut game);
    assert_eq!(lever_messages(&game).len(), 1);

    game.manager.team_id = Some("team2".to_string());
    game.clock.current_date += chrono::Duration::days(7);
    finances::process_weekly_finances(&mut game);

    assert_eq!(lever_messages(&game).len(), 2);
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
                home_penalties: None,
                away_penalties: None,
            }),
            ..Default::default()
        }],
        standings: vec![StandingEntry::new("team1".to_string())],
        transfer_log: vec![],
        transfer_rumours: vec![],
        ..Default::default()
    };
    game.league = Some(league);

    finances::process_weekly_finances(&mut game);

    // After wage deduction AND matchday income
    let wages = 1_700;
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
                home_penalties: None,
                away_penalties: None,
            }),
            ..Default::default()
        }],
        standings: vec![StandingEntry::new("team1".to_string())],
        transfer_log: vec![],
        transfer_rumours: vec![],
        ..Default::default()
    };
    game.league = Some(league);

    let initial_finance = game.teams[0].finance;
    finances::process_weekly_finances(&mut game);

    let wages = 1_700;
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

    let p3 = make_player("p3", "team2", 2_000);
    game.players.push(p3);

    let initial_t1 = game.teams[0].finance;
    let initial_t2 = game.teams[1].finance;

    finances::process_weekly_finances(&mut game);

    let t1_wages = 1_700;
    let t2_wages = 2_000;
    assert_eq!(game.teams[0].finance, initial_t1 - t1_wages);
    assert_eq!(game.teams[1].finance, initial_t2 - t2_wages);
}

// ---------------------------------------------------------------------------
// Matchday income comes from the game's seed
// ---------------------------------------------------------------------------

fn finance_after_a_home_week(seed: u64) -> i64 {
    let mut game = make_monday_game();
    game.seed = seed;
    game.league = Some(League {
        id: "l1".to_string(),
        name: "Test League".to_string(),
        season: 1,
        fixtures: vec![Fixture {
            id: "f1".to_string(),
            matchday: 1,
            date: "2025-06-14".to_string(),
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
                home_penalties: None,
                away_penalties: None,
            }),
            ..Default::default()
        }],
        standings: vec![StandingEntry::new("team1".to_string())],
        ..Default::default()
    });

    finances::process_weekly_finances(&mut game);
    game.teams[0].finance
}

/// Given a club that played at home this week,
/// When the week's finances are processed twice from the same seed,
/// Then the gate is the same both times.
#[test]
fn matchday_income_is_the_same_from_the_same_seed() {
    assert_eq!(finance_after_a_home_week(11), finance_after_a_home_week(11));
}

/// The control: attendance and ticket price are what the seed decides.
#[test]
fn the_seed_decides_the_gate() {
    let takings: std::collections::BTreeSet<i64> = (0..40).map(finance_after_a_home_week).collect();

    assert!(takings.len() > 1, "forty seeds all drew the same crowd");
}
