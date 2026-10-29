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

    if player.contract_club_id() != Some(manager_team_id) {
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

    if game.players[player_index].contract_club_id() != Some(manager_team_id) {
        return Err(ERR_PLAYER_NOT_OWNED_BY_CLUB.to_string());
    }

    Ok(player_index)
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
    let age = domain::age::age_on_or_unknown(current_date, &player.date_of_birth);
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
    if player.wage() > 0 {
        return player.wage();
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
    let age = domain::age::age_on_or_unknown(current_date, &player.date_of_birth);

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

pub(crate) fn remaining_contract_days(player: &Player, current_date: NaiveDate) -> i64 {
    contract_days_remaining(player.contract_end(), current_date)
        .unwrap_or(0)
        .max(0)
}

pub(crate) fn round_up_to_nearest_thousand(value: u32) -> u32 {
    if value == 0 {
        return 0;
    }

    value.div_ceil(1000) * 1000
}

/// A contract date as stored: `"YYYY-MM-DD"`, or nothing if it is not one.
///
/// Dates are plain strings on the wire and in saves, so every reader parses them;
/// this is the one place that says how.
pub(crate) fn parse_contract_date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}

pub(crate) fn contract_days_remaining(
    contract_end: Option<&str>,
    current_date: NaiveDate,
) -> Option<i64> {
    let contract_end_date = parse_contract_date(contract_end?)?;
    Some((contract_end_date - current_date).num_days())
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::player::{PlayerAttributes, Position};

    fn on_the_books(born: &str, weekly_wage: u32, market_value: u64) -> Player {
        let mut player = Player::new(
            "p".to_string(),
            "P".to_string(),
            "P".to_string(),
            born.to_string(),
            "England".to_string(),
            Position::Midfielder,
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
            },
        );
        player.stage_wage(weekly_wage);
        // A hundred days left on 2026-08-01: inside the six months that add a tenth.
        player.stage_contract_end(Some("2026-11-09".to_string()));
        player.market_value = market_value;
        player.morale = 75;
        player
    }

    fn club_of_standing() -> Team {
        let mut team = Team::new(
            "club".to_string(),
            "Club".to_string(),
            "CLB".to_string(),
            "England".to_string(),
            "Town".to_string(),
            "Ground".to_string(),
            20_000,
        );
        team.reputation = 50;
        team
    }

    fn first_of_august() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 8, 1).unwrap()
    }

    /// Given a player just 32 on 100,000 a week, valued 400,000, content, 100 days from the end of
    ///       his contract at a club of some standing
    /// When his expected wage is worked out
    /// Then it is 105,000: from 32 his age takes 5% off the tenth his expiring contract adds
    #[test]
    fn a_player_of_thirty_two_or_more_expects_five_percent_less() {
        let veteran = on_the_books("1994-01-01", 100_000, 400_000);

        assert_eq!(
            expected_wage(&veteran, &club_of_standing(), first_of_august()),
            105_000
        );
    }

    /// Given a 29-year-old on the same terms
    /// When his expected wage is worked out
    /// Then it is 110,000: between 28 and 31 age neither adds nor takes anything
    #[test]
    fn a_player_in_his_prime_expects_no_age_adjustment() {
        let prime = on_the_books("1997-01-01", 100_000, 400_000);

        assert_eq!(
            expected_wage(&prime, &club_of_standing(), first_of_august()),
            110_000
        );
    }

    /// Given squad players valued 400,000, 150,000 and 150,001
    /// When their importance to the club is weighed for wage demands
    /// Then the 400,000 and 150,001 players carry no premium or discount, and the 150,000 one
    ///      takes 5% less
    #[test]
    fn a_squad_player_valued_between_150k_and_750k_carries_no_importance_premium_or_discount() {
        let multiplier = |value| importance_wage_multiplier(&on_the_books("1997-01-01", 0, value));

        assert_eq!(
            [
                multiplier(400_000),
                multiplier(150_001),
                multiplier(150_000)
            ],
            [1.0, 1.0, 0.95]
        );
    }
}
