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

/// What the board says to paying a player a wage, to keep him or to sign him.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WagePolicyVerdict {
    /// The wage bill stays within the board's policy.
    WithinPolicy,
    /// Over the policy, allowed because the club would be below the squad
    /// floor in his group without him.
    OverPolicyToKeepSquadFloor,
    /// Over the policy, refused.
    OverPolicy,
}

impl WagePolicyVerdict {
    pub fn permits(self) -> bool {
        self != Self::OverPolicy
    }

    /// The line a manager is shown when the board waived its policy, for a
    /// deal they struck themselves.
    pub(crate) fn waiver_feedback_detail_key(self) -> Option<&'static str> {
        (self == Self::OverPolicyToKeepSquadFloor)
            .then_some("playerProfile.renewalFeedbackAcceptedToKeepSquadFloorDetail")
    }
}

/// Whether the board lets `team` pay `player` `offered_wage`.
///
/// The one rule for every contract a club offers to keep or sign a player —
/// the manager's renewals and free-agent signings, the assistant's delegated
/// renewals, an AI club's renewals and the squad floor's top-up. The wage
/// policy holds, except when the club would be below the squad floor in the
/// player's group without him (counted as the floor counts): a club that cannot
/// put a side out has no wage bill worth protecting.
pub fn wage_policy_verdict(
    game: &Game,
    team: &Team,
    player: &domain::player::Player,
    offered_wage: u32,
) -> WagePolicyVerdict {
    let (current_bill, projected_bill) = projected_wage_bills(game, &team.id, player, offered_wage);
    verdict_for_bills(team, current_bill, projected_bill, || {
        crate::squad_floor::club_needs_him_for_the_floor(game, &team.id, player)
    })
}

/// The same verdict for a player *joining* `team` who is not on its wage bill yet,
/// though he may already be registered to it — a youngster taken into its academy.
/// His whole wage is new to the bill. [`wage_policy_verdict`] takes a registered
/// player's current wage off before adding the offer, which is right for a renewal
/// and would let a newcomer pay for himself.
pub(crate) fn joining_wage_policy_verdict(
    game: &Game,
    team: &Team,
    player: &domain::player::Player,
    offered_wage: u32,
) -> WagePolicyVerdict {
    let current_bill = calc_wages(game, &team.id);
    let projected_bill =
        current_bill + weekly_commitment_at_wage(player, &team.id, i64::from(offered_wage));
    verdict_for_bills(team, current_bill, projected_bill, || {
        crate::squad_floor::club_needs_him_for_the_floor(game, &team.id, player)
    })
}

/// The same verdict for a club *buying* the player, who will then be on its books at
/// the full `offered_wage`. A player already on loan at the buyer is counted at his
/// loan share today and at the whole wage after the purchase, which
/// [`wage_policy_verdict`] would not do (it keeps the loan split).
pub fn purchase_wage_policy_verdict(
    game: &Game,
    team: &Team,
    player: &domain::player::Player,
    offered_wage: u32,
) -> WagePolicyVerdict {
    BuyerWageFacts::of(game, &team.id).purchase_verdict(team, player, offered_wage)
}

/// What judging a buyer's purchases needs to know about the buyer: its wage bill and its
/// senior counts. A sweep that judges many players for one club (the AI market) works
/// these out once and asks for each verdict without scanning the world again; one
/// purchase goes through [`purchase_wage_policy_verdict`], which builds them itself.
pub(crate) struct BuyerWageFacts {
    current_bill: i64,
    seniors: [usize; 4],
}

impl BuyerWageFacts {
    pub(crate) fn of(game: &Game, team_id: &str) -> Self {
        Self::new(
            calc_wages(game, team_id),
            crate::squad_floor::senior_counts(game, team_id),
        )
    }

    /// For a caller that already holds the senior counts (the market sweep's depth map).
    pub(crate) fn new(current_bill: i64, seniors: [usize; 4]) -> Self {
        Self {
            current_bill,
            seniors,
        }
    }

    pub(crate) fn purchase_verdict(
        &self,
        team: &Team,
        player: &domain::player::Player,
        offered_wage: u32,
    ) -> WagePolicyVerdict {
        let current_contribution = player_weekly_wage_for_team(player, &team.id);
        let projected_bill = self.current_bill - current_contribution + i64::from(offered_wage);
        verdict_for_bills(team, self.current_bill, projected_bill, || {
            crate::squad_floor::club_needs_him_given(self.seniors, &team.id, player)
        })
    }
}

/// The rule itself, once: the policy on the two bills, then the squad-floor waiver
/// (asked only when the policy says no).
fn verdict_for_bills(
    team: &Team,
    current_bill: i64,
    projected_bill: i64,
    club_needs_him_for_the_floor: impl FnOnce() -> bool,
) -> WagePolicyVerdict {
    if wage_policy_allows_projection(team, current_bill, projected_bill) {
        WagePolicyVerdict::WithinPolicy
    } else if club_needs_him_for_the_floor() {
        WagePolicyVerdict::OverPolicyToKeepSquadFloor
    } else {
        WagePolicyVerdict::OverPolicy
    }
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
        .contract_club_id()
        .ok_or_else(|| ERR_PLAYER_HAS_NO_TEAM.to_string())?;
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
