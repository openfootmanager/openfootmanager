//! The money a club can raise when it is under pressure, and when it may ask: the sponsor pitch,
//! the marketing campaign, and the weekly income the finance screen projects. Every club here is
//! test FC from `make_monday_game`, with a weekly wage bill of 1,700.

use super::*;

/// Given test FC with no sponsor and a home match won on Saturday, in a 40,000 stadium
/// When its finance snapshot is taken on Monday
/// Then its weekly recurring income is the 608,000 that gate brings: sponsor income plus the
///      week's estimated home gate
#[test]
fn the_finance_snapshot_adds_last_weeks_home_gate_to_sponsor_income() {
    let mut game = make_monday_game();
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

    let snapshot = finances::team_finance_snapshot(&game, "team1").expect("a snapshot");

    assert_eq!(snapshot.weekly_sponsor_income, 0);
    assert_eq!(snapshot.weekly_recurring_income, 608_000);
}

/// Given test FC in debt (−60,000) and far over its wage budget: the campaign's gross revenue
///       clamps to its 250,000 ceiling
/// When it runs a marketing campaign
/// Then the campaign costs a quarter of that, 62,500
#[test]
fn a_marketing_campaign_costs_a_quarter_of_its_gross_revenue() {
    let mut game = make_monday_game();
    game.teams[0].wage_budget = 50_000;
    game.teams[0].finance = -60_000;

    let result = finances::request_marketing_campaign(&mut game, "team1").expect("campaign");

    assert_eq!(
        (result.gross_revenue, result.campaign_cost),
        (250_000, 62_500)
    );
}

/// Given a small club (reputation 100, 20,000 seats) in debt and over its wage budget
/// When it runs a marketing campaign
/// Then the gross revenue is the sum of its parts, under the ceiling: 25,000 for reputation,
///      60,000 for the stadium, 40,000 for Critical finances, 20,000 for debt and 15,000 for wage
///      pressure, 160,000 in all
#[test]
fn a_marketing_campaigns_gross_revenue_sums_reputation_stadium_and_pressure() {
    let mut game = make_monday_game();
    game.teams[0].reputation = 100;
    game.teams[0].stadium_capacity = 20_000;
    game.teams[0].finance = -60_000;
    game.teams[0].wage_budget = 1_000;

    let result = finances::request_marketing_campaign(&mut game, "team1").expect("campaign");

    assert_eq!(result.gross_revenue, 160_000);
}

/// Given test FC (reputation 500) in debt and over its wage budget, with no league position
/// When it pitches for a sponsor
/// Then the offer is 160,000 a week: the 40,000 base, 60,000 for reputation, 25,000 for Critical
///      finances, 15,000 for wage pressure and 20,000 for debt
#[test]
fn a_sponsor_pitch_pays_more_for_a_club_under_financial_pressure() {
    let mut game = make_monday_game();
    game.teams[0].wage_budget = 1_000;
    game.teams[0].finance = -10_000;

    let result = finances::request_sponsor_pitch(&mut game, "team1").expect("pitch");

    assert_eq!(result.weekly_amount, 160_000);
}

/// Given test FC solvent (10,000 in cash) and well within its wage budget, but with only about
///       five weeks of cash left
/// When it asks for a marketing campaign
/// Then it gets one: a short runway is enough on its own
#[test]
fn a_solvent_club_with_eight_weeks_of_cash_left_or_less_can_run_a_marketing_campaign() {
    let mut game = make_monday_game();
    game.teams[0].finance = 10_000;

    assert!(finances::request_marketing_campaign(&mut game, "team1").is_ok());
}

/// Given test FC solvent (10,000 in cash) and well within its wage budget, but with only about
///       five weeks of cash left
/// When it pitches for a sponsor
/// Then it may: a short runway is enough on its own
#[test]
fn a_solvent_club_with_eight_weeks_of_cash_left_or_less_can_pitch_for_a_sponsor() {
    let mut game = make_monday_game();
    game.teams[0].finance = 10_000;

    assert!(finances::request_sponsor_pitch(&mut game, "team1").is_ok());
}

/// Test FC with plenty of cash, its 1,700 bill 10 over a 1,690 budget: over budget, but at less
/// than 1% over its status is only Watch.
fn a_club_just_over_its_wage_budget() -> Game {
    let mut game = make_monday_game();
    game.teams[0].wage_budget = 1_690;
    game
}

/// Given a club less than 1% over its wage budget, with plenty of cash
/// When it pitches for a sponsor
/// Then it may: being over budget at all is reason enough
#[test]
fn a_club_less_than_one_percent_over_its_wage_budget_can_pitch_for_a_sponsor() {
    let mut game = a_club_just_over_its_wage_budget();

    assert!(finances::request_sponsor_pitch(&mut game, "team1").is_ok());
}

/// Given a club less than 1% over its wage budget, with plenty of cash
/// When it asks for a marketing campaign
/// Then it gets one: being over budget at all is reason enough
#[test]
fn a_club_less_than_one_percent_over_its_wage_budget_can_run_a_marketing_campaign() {
    let mut game = a_club_just_over_its_wage_budget();

    assert!(finances::request_marketing_campaign(&mut game, "team1").is_ok());
}
