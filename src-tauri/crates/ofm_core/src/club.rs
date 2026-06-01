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

pub fn upgrade_facility(team: &mut Team, facility_type: FacilityType) -> Result<i64, String> {
    let cost = next_upgrade_cost(team, &facility_type);
    if team.finance < cost {
        return Err(facility_upgrade_insufficient_funds_error(cost));
    }

    team.finance -= cost;
    team.season_expenses += cost;

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
