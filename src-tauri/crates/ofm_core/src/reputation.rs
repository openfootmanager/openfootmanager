use crate::game::Game;
use domain::league::StandingEntry;
use std::collections::HashMap;

const POSITION_DELTA_WEIGHT: i32 = 12;
const CHAMPION_BONUS: i32 = 12;
const BOTTOM_FINISH_PENALTY: i32 = 8;
const MIN_REPUTATION: i32 = 0;
const MAX_REPUTATION: i32 = 1000;

/// Expected finishing positions ranked by reputation, considering only the
/// teams that actually contested these standings — so a second-division club is
/// measured against its division, not the whole game world.
fn expected_positions(game: &Game, final_standings: &[StandingEntry]) -> HashMap<String, usize> {
    let participating: std::collections::HashSet<&str> = final_standings
        .iter()
        .map(|standing| standing.team_id.as_str())
        .collect();
    let mut ordered_teams: Vec<_> = game
        .teams
        .iter()
        .filter(|team| participating.contains(team.id.as_str()))
        .collect();
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

    let expected_positions = expected_positions(game, final_standings);
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
    use super::{next_reputation, update_team_reputation};
    use crate::clock::GameClock;
    use crate::game::Game;
    use chrono::{TimeZone, Utc};
    use domain::league::StandingEntry;

    fn make_game_with_reputations(reputations: &[(&str, u32)]) -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 5, 20, 12, 0, 0).unwrap());
        let manager = domain::manager::Manager::new(
            "mgr".to_string(),
            "Alex".to_string(),
            "Boss".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let teams = reputations
            .iter()
            .map(|(id, reputation)| {
                let mut team = domain::team::Team::new(
                    id.to_string(),
                    id.to_string(),
                    id.to_string(),
                    "Country".to_string(),
                    "City".to_string(),
                    "Stadium".to_string(),
                    10_000,
                );
                team.reputation = *reputation;
                team
            })
            .collect();
        Game::new(clock, manager, teams, vec![], vec![], vec![])
    }

    #[test]
    fn expectation_is_relative_to_the_division_not_the_whole_world() {
        // "minnow" is the weakest club in the world, but within its two-club
        // division it is expected to finish last — so finishing last there is
        // meeting expectations (no positional gain), and the bottom-finish
        // penalty applies.
        let mut game =
            make_game_with_reputations(&[("giant", 900), ("mid", 500), ("minnow", 100)]);
        let division_standings = vec![
            StandingEntry::new("mid".to_string()),
            StandingEntry::new("minnow".to_string()),
        ];

        update_team_reputation(&mut game, &division_standings);

        let minnow = game.teams.iter().find(|t| t.id == "minnow").unwrap();
        assert!(
            minnow.reputation < 100,
            "meeting a last-place expectation must not be rewarded as an overperformance \
             against the global table (got {})",
            minnow.reputation
        );
        let giant = game.teams.iter().find(|t| t.id == "giant").unwrap();
        assert_eq!(giant.reputation, 900, "teams outside the division are untouched");
    }

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
