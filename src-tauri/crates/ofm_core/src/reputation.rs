use crate::game::Game;
use domain::league::StandingEntry;
use std::collections::HashMap;

const POSITION_DELTA_WEIGHT: i32 = 12;
const CHAMPION_BONUS: i32 = 12;
const BOTTOM_FINISH_PENALTY: i32 = 8;
const MIN_REPUTATION: i32 = 0;
const MAX_REPUTATION: i32 = 1000;

fn expected_positions(game: &Game) -> HashMap<String, usize> {
    let mut ordered_teams: Vec<_> = game.teams.iter().collect();
    ordered_teams.sort_by(|left, right| {
        right
            .reputation
            .cmp(&left.reputation)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.id.cmp(&right.id))
    });

    ordered_teams
        .into_iter()
        .enumerate()
        .map(|(index, team)| (team.id.clone(), index + 1))
        .collect()
}

fn next_reputation(
    current_reputation: u32,
    expected_position: usize,
    final_position: usize,
    team_count: usize,
) -> u32 {
    let expected_position = expected_position as i32;
    let final_position = final_position as i32;
    let position_delta = (expected_position - final_position) * POSITION_DELTA_WEIGHT;
    let champion_bonus = if final_position == 1 {
        CHAMPION_BONUS
    } else {
        0
    };
    let bottom_finish_penalty = if final_position == team_count as i32 {
        BOTTOM_FINISH_PENALTY
    } else {
        0
    };

    (current_reputation as i32 + position_delta + champion_bonus - bottom_finish_penalty)
        .clamp(MIN_REPUTATION, MAX_REPUTATION) as u32
}

pub fn update_team_reputation(game: &mut Game, final_standings: &[StandingEntry]) {
    if final_standings.is_empty() {
        return;
    }

    let expected_positions = expected_positions(game);
    let team_count = final_standings.len();

    for (index, standing) in final_standings.iter().enumerate() {
        if let Some(team) = game
            .teams
            .iter_mut()
            .find(|team| team.id == standing.team_id)
        {
            let expected_position = expected_positions
                .get(&standing.team_id)
                .copied()
                .unwrap_or(index + 1);
            team.reputation =
                next_reputation(team.reputation, expected_position, index + 1, team_count);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::next_reputation;

    #[test]
    fn champion_outperforming_expectation_gains_reputation() {
        assert!(next_reputation(320, 8, 1, 10) > 320);
    }

    #[test]
    fn bottom_finish_after_high_expectation_loses_reputation() {
        assert!(next_reputation(860, 1, 10, 10) < 860);
    }

    #[test]
    fn reputation_is_clamped_within_supported_bounds() {
        assert_eq!(next_reputation(995, 10, 1, 10), 1000);
        assert_eq!(next_reputation(5, 1, 10, 10), 0);
    }
}
