//! Leaf helpers shared across the contract clusters: owned-player lookups,
//! wage/tenure maths, and contract-date arithmetic. Byte-faithful extraction
//! from the original contracts.rs.

use super::*;

pub(crate) fn owned_player<'a>(game: &'a Game, player_id: &str) -> Result<&'a Player, String> {
    let manager_team_id = game
        .manager
        .team_id
        .as_deref()
        .ok_or(ERR_NO_TEAM_ASSIGNED.to_string())?;
    let player = game
        .players
        .iter()
        .find(|candidate| candidate.id == player_id)
        .ok_or(ERR_PLAYER_NOT_FOUND.to_string())?;

    if contract_owner_team_id(player) != Some(manager_team_id) {
        return Err(ERR_PLAYER_NOT_OWNED_BY_CLUB.to_string());
    }

    Ok(player)
}

pub(crate) fn owned_player_index(game: &Game, player_id: &str) -> Result<usize, String> {
    let manager_team_id = game
        .manager
        .team_id
        .as_deref()
        .ok_or(ERR_NO_TEAM_ASSIGNED.to_string())?;
    let player_index = game
        .players
        .iter()
        .position(|candidate| candidate.id == player_id)
        .ok_or(ERR_PLAYER_NOT_FOUND.to_string())?;

    if contract_owner_team_id(&game.players[player_index]) != Some(manager_team_id) {
        return Err(ERR_PLAYER_NOT_OWNED_BY_CLUB.to_string());
    }

    Ok(player_index)
}

pub(crate) fn contract_owner_team_id(player: &Player) -> Option<&str> {
    player
        .active_loan
        .as_ref()
        .map(|loan| loan.parent_team_id.as_str())
        .or(player.team_id.as_deref())
}

pub(crate) fn backend_text_with_param(key: &str, param_name: &str, param_value: &str) -> String {
    let mut message = String::with_capacity(key.len() + param_name.len() + param_value.len() + 2);
    message.push_str(key);
    message.push('?');
    message.push_str(param_name);
    message.push('=');
    message.push_str(param_value);
    message
}

pub(crate) fn expected_wage(player: &Player, team: &Team, current_date: NaiveDate) -> u32 {
    let mut wage = reference_player_wage(player) as f32;
    let age = player_age_on(current_date, &player.date_of_birth);
    let remaining_days = remaining_contract_days(player, current_date);

    if age <= 27 {
        wage *= 1.05;
    } else if age >= 32 {
        wage *= 0.95;
    }

    if player.morale <= 50 {
        wage *= 1.10;
    }

    wage *= importance_wage_multiplier(player);

    if team.reputation < 40 {
        wage *= 1.05;
    }

    if remaining_days <= 180 {
        wage *= 1.10;
    } else if remaining_days <= 365 {
        wage *= 1.05;
    }

    let rounded = round_up_to_nearest_thousand(wage.ceil() as u32);
    rounded.max(reference_player_wage(player))
}

pub(crate) fn reference_player_wage(player: &Player) -> u32 {
    if player.wage > 0 {
        return player.wage;
    }

    let derived_wage = (player.market_value / MARKET_VALUE_TO_WAGE_RATIO).max(MINIMUM_DEFAULT_WAGE);

    round_up_to_nearest_thousand(derived_wage.min(u32::MAX as u64) as u32)
}

pub(crate) fn importance_wage_multiplier(player: &Player) -> f32 {
    if player.market_value >= 2_000_000 {
        return 1.18;
    }

    if player.market_value >= 750_000 {
        return 1.10;
    }

    if player.market_value <= 150_000 {
        return 0.95;
    }

    1.0
}

pub(crate) fn expected_contract_years(player: &Player, current_date: NaiveDate) -> u32 {
    let age = player_age_on(current_date, &player.date_of_birth);

    if age <= 28 {
        return 3;
    }

    if age <= 32 {
        return 2;
    }

    1
}

pub(crate) fn minimum_acceptable_wage(current_wage: u32) -> u32 {
    ((current_wage as f32) * 0.85).floor() as u32
}

pub(crate) fn is_insulting_wage_offer(
    reference_wage: u32,
    expected_wage: u32,
    offered_wage: u32,
) -> bool {
    let anchor_wage = reference_wage.max(expected_wage);
    let insulting_floor = ((anchor_wage as f32) * 0.65).floor() as u32;

    offered_wage < insulting_floor
}

pub(crate) fn player_age_on(current_date: NaiveDate, date_of_birth: &str) -> i32 {
    let Ok(dob) = NaiveDate::parse_from_str(date_of_birth, "%Y-%m-%d") else {
        return 30;
    };

    let mut age = current_date.year() - dob.year();
    if current_date.ordinal() < dob.ordinal() {
        age -= 1;
    }
    age
}

pub(crate) fn remaining_contract_days(player: &Player, current_date: NaiveDate) -> i64 {
    contract_days_remaining(player.contract_end.as_deref(), current_date)
        .unwrap_or(0)
        .max(0)
}

pub(crate) fn round_up_to_nearest_thousand(value: u32) -> u32 {
    if value == 0 {
        return 0;
    }

    value.div_ceil(1000) * 1000
}

pub(crate) fn contract_days_remaining(
    contract_end: Option<&str>,
    current_date: NaiveDate,
) -> Option<i64> {
    let contract_end = contract_end?;
    let contract_end_date = NaiveDate::parse_from_str(contract_end, "%Y-%m-%d").ok()?;
    Some((contract_end_date - current_date).num_days())
}
