use crate::contracts::RenewalFinancialProjection;
use crate::finances::calc_cash_runway_weeks;
use crate::game::Game;
use domain::team::Team;

const WAGE_SOFT_CAP_PCT: i64 = 110;
const LEGACY_OVER_BUDGET_GRACE_PCT: i64 = 3;
const LEGACY_OVER_BUDGET_GRACE_MIN: i64 = 25_000;
const ERR_PLAYER_HAS_NO_TEAM: &str = "be.error.contracts.playerHasNoTeam";

fn backend_error_with_param(key: &str, param_name: &str, param_value: i64) -> String {
    let param_value = param_value.to_string();
    let mut message = String::with_capacity(key.len() + param_name.len() + param_value.len() + 2);
    message.push_str(key);
    message.push('?');
    message.push_str(param_name);
    message.push('=');
    message.push_str(&param_value);
    message
}

fn annual_team_wage_bill(game: &Game, team_id: &str) -> i64 {
    let player_wages: i64 = game
        .players
        .iter()
        .filter(|player| player.team_id.as_deref() == Some(team_id))
        .map(|player| player.wage as i64)
        .sum();

    let staff_wages: i64 = game
        .staff
        .iter()
        .filter(|staff_member| staff_member.team_id.as_deref() == Some(team_id))
        .map(|staff_member| staff_member.wage as i64)
        .sum();

    player_wages + staff_wages
}

fn projected_annual_wage_bill(
    game: &Game,
    team_id: &str,
    current_player_wage: u32,
    offered_wage: u32,
) -> i64 {
    annual_team_wage_bill(game, team_id) - current_player_wage as i64 + offered_wage as i64
}

pub fn project_contract_offer_financial_impact(
    game: &Game,
    team: &Team,
    current_player_wage: u32,
    offered_wage: u32,
) -> RenewalFinancialProjection {
    let current_bill = annual_team_wage_bill(game, &team.id);
    let projected_bill =
        projected_annual_wage_bill(game, &team.id, current_player_wage, offered_wage);
    let annual_wage_budget = team.wage_budget;
    let annual_soft_cap = (annual_wage_budget * WAGE_SOFT_CAP_PCT) / 100;
    let current_weekly_wage_spend = current_bill / 52;
    let projected_weekly_wage_spend = projected_bill / 52;

    let current_cash_runway_weeks =
        calc_cash_runway_weeks(team.finance, -current_weekly_wage_spend);
    let projected_cash_runway_weeks =
        calc_cash_runway_weeks(team.finance, -projected_weekly_wage_spend);

    RenewalFinancialProjection {
        current_annual_wage_bill: current_bill,
        projected_annual_wage_bill: projected_bill,
        annual_wage_budget,
        annual_soft_cap,
        current_weekly_wage_spend,
        projected_weekly_wage_spend,
        current_cash_runway_weeks,
        projected_cash_runway_weeks,
        currently_over_budget: current_bill > annual_wage_budget,
        policy_allows: renewal_wage_policy_allows(game, team, current_player_wage, offered_wage),
    }
}

pub fn renewal_wage_policy_allows(
    game: &Game,
    team: &Team,
    current_player_wage: u32,
    offered_wage: u32,
) -> bool {
    let current_bill = annual_team_wage_bill(game, &team.id);
    let projected_bill =
        projected_annual_wage_bill(game, &team.id, current_player_wage, offered_wage);
    let soft_cap = (team.wage_budget * WAGE_SOFT_CAP_PCT) / 100;

    if current_bill <= team.wage_budget {
        return projected_bill <= soft_cap;
    }

    if projected_bill <= current_bill {
        return true;
    }

    let legacy_grace = std::cmp::max(
        (team.wage_budget * LEGACY_OVER_BUDGET_GRACE_PCT) / 100,
        LEGACY_OVER_BUDGET_GRACE_MIN,
    );

    projected_bill <= current_bill + legacy_grace
}

pub fn renewal_wage_policy_error_message(team: &Team) -> String {
    backend_error_with_param(
        "be.error.contracts.boardWagePolicy",
        "budget",
        team.wage_budget,
    )
}

pub fn project_renewal_financial_impact(
    game: &Game,
    player_id: &str,
    offered_wage: u32,
) -> Result<RenewalFinancialProjection, String> {
    let player = game
        .players
        .iter()
        .find(|player| player.id == player_id)
        .ok_or_else(|| "be.error.playerNotFound".to_string())?;
    let team_id = player
        .team_id
        .as_deref()
        .ok_or_else(|| ERR_PLAYER_HAS_NO_TEAM.to_string())?;
    let team = game
        .teams
        .iter()
        .find(|team| team.id == team_id)
        .ok_or_else(|| "be.error.teamNotFound".to_string())?;

    Ok(project_contract_offer_financial_impact(
        game,
        team,
        player.wage,
        offered_wage,
    ))
}
