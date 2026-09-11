use crate::finances::{CashKind, post};
use crate::game::Game;
use domain::team::{Facilities, FacilityType, Team};

pub const BASE_FACILITY_UPGRADE_COST: i64 = 250_000;

fn facility_upgrade_insufficient_funds_error(amount: i64) -> String {
    let amount = amount.to_string();
    let key = "be.error.finance.facilityUpgradeInsufficientFunds";
    let param_name = "amount";
    let mut message = String::with_capacity(key.len() + param_name.len() + amount.len() + 2);
    message.push_str(key);
    message.push('?');
    message.push_str(param_name);
    message.push('=');
    message.push_str(&amount);
    message
}

fn facility_level(facilities: &Facilities, facility_type: &FacilityType) -> u8 {
    match facility_type {
        FacilityType::Training => facilities.training,
        FacilityType::Medical => facilities.medical,
        FacilityType::Scouting => facilities.scouting,
    }
}

pub fn next_upgrade_cost(team: &Team, facility_type: &FacilityType) -> i64 {
    i64::from(facility_level(&team.facilities, facility_type)) * BASE_FACILITY_UPGRADE_COST
}

pub fn upgrade_facility(
    game: &mut Game,
    team_id: &str,
    facility_type: FacilityType,
) -> Result<i64, String> {
    let cost = {
        let team = game
            .teams
            .iter()
            .find(|team| team.id == team_id)
            .ok_or_else(|| "be.error.managedTeamNotFound".to_string())?;
        let cost = next_upgrade_cost(team, &facility_type);
        if team.finance < cost {
            return Err(facility_upgrade_insufficient_funds_error(cost));
        }
        cost
    };

    let date = game.clock.current_date.date_naive();
    post(game, team_id, -cost, CashKind::Facilities, date)?;

    let team = game
        .teams
        .iter_mut()
        .find(|team| team.id == team_id)
        .ok_or_else(|| "be.error.managedTeamNotFound".to_string())?;
    match facility_type {
        FacilityType::Training => {
            team.facilities.training = team.facilities.training.saturating_add(1);
        }
        FacilityType::Medical => {
            team.facilities.medical = team.facilities.medical.saturating_add(1);
        }
        FacilityType::Scouting => {
            team.facilities.scouting = team.facilities.scouting.saturating_add(1);
        }
    }

    Ok(cost)
}
