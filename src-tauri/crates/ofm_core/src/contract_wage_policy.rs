use crate::contracts::RenewalFinancialProjection;
use crate::finances::{
    calc_cash_runway_weeks, calc_wages, player_weekly_wage_for_team, weekly_commitment_at_wage,
};
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

fn contract_owner_team_id(player: &domain::player::Player) -> Option<&str> {
    player
        .active_loan
        .as_ref()
        .map(|loan| loan.parent_team_id.as_str())
        .or(player.team_id.as_deref())
}

fn projected_wage_bills(
    game: &Game,
    team_id: &str,
    player: &domain::player::Player,
    offered_wage: u32,
) -> (i64, i64) {
    let current_bill = calc_wages(game, team_id);
    let current_contribution = player_weekly_wage_for_team(player, team_id);
    let offered_contribution = weekly_commitment_at_wage(player, team_id, i64::from(offered_wage));
    (
        current_bill,
        current_bill - current_contribution + offered_contribution,
    )
}

pub fn project_contract_offer_financial_impact(
    game: &Game,
    team: &Team,
    player: &domain::player::Player,
    offered_wage: u32,
) -> RenewalFinancialProjection {
    let (current_bill, projected_bill) = projected_wage_bills(game, &team.id, player, offered_wage);
    let wage_budget = team.wage_budget;
    let soft_cap = (wage_budget * WAGE_SOFT_CAP_PCT) / 100;

    let current_cash_runway_weeks = calc_cash_runway_weeks(team.finance, -current_bill);
    let projected_cash_runway_weeks = calc_cash_runway_weeks(team.finance, -projected_bill);

    RenewalFinancialProjection {
        current_annual_wage_bill: current_bill,
        projected_annual_wage_bill: projected_bill,
        annual_wage_budget: wage_budget,
        annual_soft_cap: soft_cap,
        current_weekly_wage_spend: current_bill,
        projected_weekly_wage_spend: projected_bill,
        current_cash_runway_weeks,
        projected_cash_runway_weeks,
        currently_over_budget: current_bill > wage_budget,
        policy_allows: wage_policy_allows_projection(team, current_bill, projected_bill),
    }
}

pub fn wage_policy_allows_projection(team: &Team, current_bill: i64, projected_bill: i64) -> bool {
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

pub fn renewal_wage_policy_allows(
    game: &Game,
    team: &Team,
    player: &domain::player::Player,
    offered_wage: u32,
) -> bool {
    let (current_bill, projected_bill) = projected_wage_bills(game, &team.id, player, offered_wage);
    wage_policy_allows_projection(team, current_bill, projected_bill)
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
    let team_id =
        contract_owner_team_id(player).ok_or_else(|| ERR_PLAYER_HAS_NO_TEAM.to_string())?;
    let team = game
        .teams
        .iter()
        .find(|team| team.id == team_id)
        .ok_or_else(|| "be.error.teamNotFound".to_string())?;

    Ok(project_contract_offer_financial_impact(
        game,
        team,
        player,
        offered_wage,
    ))
}
