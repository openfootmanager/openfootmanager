use crate::game::Game;
use domain::league::StandingEntry;
use std::collections::HashMap;

/// What a place in the table was worth when clubs were scored against their expected finishing
/// place.
const REPUTATION_PER_PLACE: i32 = 12;
/// Reputation per point per game above or below what a club's reputation predicted. A place in a
/// 20-club table is worth about 0.08 points per game, so a place stays worth about
/// [`REPUTATION_PER_PLACE`].
const POINTS_PER_GAME_WEIGHT: f64 = 150.0;
const CHAMPION_BONUS: i32 = 12;
const BOTTOM_FINISH_PENALTY: i32 = 8;
const MIN_REPUTATION: i32 = 0;
const MAX_REPUTATION: i32 = 1000;

fn points_per_game(standing: &StandingEntry) -> f64 {
    if standing.played == 0 {
        return 0.0;
    }
    f64::from(standing.points) / f64::from(standing.played)
}

/// The points per game each club's reputation predicted: the least-squares line through the
/// division's points per game against reputation, never sloping down. Only the clubs that
/// contested these standings are fitted, so a club is measured against its own division.
///
/// A line, unlike an expected finishing place, can be beaten from the top: a club expected to
/// finish first used to be able only to lose places, so the top of every division drained. The
/// residuals of a fitted line sum to zero, so the division's reputation is conserved apart from
/// the champion's bonus and the last club's penalty.
fn expected_points_per_game(
    game: &Game,
    final_standings: &[StandingEntry],
) -> HashMap<String, f64> {
    let clubs: Vec<(&str, f64, f64)> = final_standings
        .iter()
        .filter_map(|standing| {
            let team = game.teams.iter().find(|team| team.id == standing.team_id)?;
            Some((
                standing.team_id.as_str(),
                f64::from(team.reputation),
                points_per_game(standing),
            ))
        })
        .collect();
    if clubs.is_empty() {
        return HashMap::new();
    }
    let count = clubs.len() as f64;
    let mean_reputation = clubs
        .iter()
        .map(|(_, reputation, _)| reputation)
        .sum::<f64>()
        / count;
    let mean_points = clubs.iter().map(|(_, _, points)| points).sum::<f64>() / count;
    let (covariance, variance) = clubs.iter().fold(
        (0.0, 0.0),
        |(covariance, variance), (_, reputation, points)| {
            let spread = reputation - mean_reputation;
            (
                covariance + spread * (points - mean_points),
                variance + spread * spread,
            )
        },
    );
    let slope = if variance > 0.0 {
        (covariance / variance).max(0.0)
    } else {
        0.0
    };
    clubs
        .iter()
        .map(|(id, reputation, _)| {
            (
                id.to_string(),
                mean_points + slope * (reputation - mean_reputation),
            )
        })
        .collect()
}

fn next_reputation(
    current_reputation: u32,
    points_per_game_above_expectation: f64,
    final_position: usize,
    team_count: usize,
) -> u32 {
    // A season moves a reputation at most as far as the old place-based rule could: in a tiny
    // or short division a few results swing points per game far more than in a full table.
    let most_a_season_can_move = i32::try_from(team_count.saturating_sub(1))
        .unwrap_or(i32::MAX)
        .saturating_mul(REPUTATION_PER_PLACE);
    let performance_delta = ((points_per_game_above_expectation * POINTS_PER_GAME_WEIGHT).round()
        as i32)
        .clamp(-most_a_season_can_move, most_a_season_can_move);
    let champion_bonus = if final_position == 1 {
        CHAMPION_BONUS
    } else {
        0
    };
    let bottom_finish_penalty = if final_position == team_count {
        BOTTOM_FINISH_PENALTY
    } else {
        0
    };

    (current_reputation as i32 + performance_delta + champion_bonus - bottom_finish_penalty)
        .clamp(MIN_REPUTATION, MAX_REPUTATION) as u32
}

pub fn update_team_reputation(game: &mut Game, final_standings: &[StandingEntry]) {
    if final_standings.is_empty() {
        return;
    }

    let expected = expected_points_per_game(game, final_standings);
    let team_count = final_standings.len();

    for (index, standing) in final_standings.iter().enumerate() {
        let Some(&expected_points) = expected.get(&standing.team_id) else {
            continue;
        };
        if let Some(team) = game
            .teams
            .iter_mut()
            .find(|team| team.id == standing.team_id)
        {
            team.reputation = next_reputation(
                team.reputation,
                points_per_game(standing) - expected_points,
                index + 1,
                team_count,
            );
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
        let mut game = make_game_with_reputations(&[("giant", 900), ("mid", 500), ("minnow", 100)]);
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
        assert_eq!(
            giant.reputation, 900,
            "teams outside the division are untouched"
        );
    }

    /// A division's final table, top first: each club's id, games played and points.
    fn final_table(rows: &[(&str, u32, u32)]) -> Vec<StandingEntry> {
        rows.iter()
            .map(|(id, played, points)| {
                let mut entry = StandingEntry::new(id.to_string());
                entry.played = *played;
                entry.points = *points;
                entry
            })
            .collect()
    }

    fn reputation_of(game: &Game, id: &str) -> u32 {
        game.teams.iter().find(|t| t.id == id).unwrap().reputation
    }

    /// Given a division whose points follow its clubs' reputations, the biggest club winning it
    /// When reputations are updated at the season's end
    /// Then the champion keeps at least the reputation it had
    #[test]
    fn a_champion_expected_to_finish_first_holds_its_reputation() {
        let mut game =
            make_game_with_reputations(&[("big", 800), ("b", 600), ("c", 400), ("d", 200)]);

        update_team_reputation(
            &mut game,
            &final_table(&[("big", 30, 60), ("b", 30, 50), ("c", 30, 40), ("d", 30, 30)]),
        );

        assert!(reputation_of(&game, "big") >= 800);
    }

    /// Given the biggest club finishing second, a point behind a rival of nearly the same
    ///       reputation, both well clear of the rest
    /// When reputations are updated at the season's end
    /// Then the big club keeps its reputation within a few points: it did what its standing
    ///      predicted, and only being unable to beat a first-place expectation would cost it
    #[test]
    fn a_big_club_a_point_behind_an_equal_rival_keeps_its_reputation() {
        let mut game =
            make_game_with_reputations(&[("big", 800), ("rival", 790), ("c", 500), ("d", 300)]);

        update_team_reputation(
            &mut game,
            &final_table(&[
                ("rival", 30, 61),
                ("big", 30, 60),
                ("c", 30, 40),
                ("d", 30, 30),
            ]),
        );

        let big = reputation_of(&game, "big");
        assert!(
            (795..=805).contains(&big),
            "a big club meeting its standing kept {big}, not about 800"
        );
    }

    /// Given a mid-table club that finishes second with far more points than its reputation
    ///       predicts
    /// When reputations are updated at the season's end
    /// Then its reputation rises
    #[test]
    fn an_over_performing_mid_table_club_rises() {
        let mut game = make_game_with_reputations(&[
            ("a", 800),
            ("b", 700),
            ("mid", 500),
            ("d", 300),
            ("e", 200),
        ]);

        update_team_reputation(
            &mut game,
            &final_table(&[
                ("a", 30, 62),
                ("mid", 30, 60),
                ("b", 30, 48),
                ("d", 30, 30),
                ("e", 30, 25),
            ]),
        );

        assert!(reputation_of(&game, "mid") > 500);
    }

    /// Given the biggest club in a division finishing last, relegated with the fewest points
    /// When reputations are updated at the season's end
    /// Then its reputation falls
    #[test]
    fn a_relegated_big_club_falls() {
        let mut game =
            make_game_with_reputations(&[("big", 800), ("b", 600), ("c", 400), ("d", 200)]);

        update_team_reputation(
            &mut game,
            &final_table(&[("b", 30, 60), ("c", 30, 50), ("d", 30, 40), ("big", 30, 20)]),
        );

        assert!(reputation_of(&game, "big") < 800);
    }

    /// Given a four-club division where the smaller the club, the more it scored
    /// When reputations are updated at the season's end
    /// Then every club is measured against the division's mean points per game: a bigger
    ///      reputation is never expected to do worse, so the smallest club rises and the biggest
    ///      falls by the same amount, before the champion's bonus and the last club's penalty
    #[test]
    fn a_division_where_small_clubs_do_better_is_not_fitted_with_a_downward_line() {
        let mut game =
            make_game_with_reputations(&[("big", 800), ("b", 600), ("c", 400), ("small", 200)]);

        update_team_reputation(
            &mut game,
            &final_table(&[
                ("small", 20, 29),
                ("c", 20, 27),
                ("b", 20, 23),
                ("big", 20, 21),
            ]),
        );

        assert_eq!(reputation_of(&game, "small"), 200 + 30 + 12);
        assert_eq!(reputation_of(&game, "big"), 800 - 30 - 8);
    }

    /// Given a two-club division where the small club takes every point from the big one
    /// When reputations are updated at the season's end
    /// Then neither moves further than a place's worth: a division of two decides one place, so
    ///      a season there can move a reputation no more than the old place-based rule allowed
    #[test]
    fn an_upset_in_a_two_club_division_moves_reputation_no_more_than_a_place() {
        let mut game = make_game_with_reputations(&[("big", 700), ("small", 300)]);

        update_team_reputation(&mut game, &final_table(&[("small", 2, 6), ("big", 2, 0)]));

        assert_eq!(reputation_of(&game, "small"), 300 + 12 + 12);
        assert_eq!(reputation_of(&game, "big"), 700 - 12 - 8);
    }

    /// Given a full 20-club division of equal reputations where the champion took a point per
    ///       game more than the rest and the last club a point per game fewer
    /// When reputations are updated at the season's end
    /// Then both move by the full 150 a point per game is worth: the cap on a season's movement
    ///      leaves a normal table alone
    #[test]
    fn a_full_division_is_not_held_back_by_the_cap() {
        let mut clubs: Vec<(String, u32)> = vec![("top".to_string(), 500)];
        clubs.extend((0..18).map(|i| (format!("mid{i}"), 500)));
        clubs.push(("bottom".to_string(), 500));
        let reputations: Vec<(&str, u32)> = clubs.iter().map(|(id, r)| (id.as_str(), *r)).collect();
        let mut game = make_game_with_reputations(&reputations);
        let mut table: Vec<(&str, u32, u32)> = vec![("top", 38, 76)];
        table.extend(clubs[1..19].iter().map(|(id, _)| (id.as_str(), 38, 38)));
        table.push(("bottom", 38, 0));

        update_team_reputation(&mut game, &final_table(&table));

        assert_eq!(reputation_of(&game, "top"), 500 + 150 + 12);
        assert_eq!(reputation_of(&game, "mid0"), 500);
        assert_eq!(reputation_of(&game, "bottom"), 500 - 150 - 8);
    }

    #[test]
    fn champion_outperforming_expectation_gains_reputation() {
        assert!(next_reputation(320, 0.5, 1, 10) > 320);
    }

    #[test]
    fn bottom_finish_after_high_expectation_loses_reputation() {
        assert!(next_reputation(860, -1.0, 10, 10) < 860);
    }

    #[test]
    fn reputation_is_clamped_within_supported_bounds() {
        assert_eq!(next_reputation(995, 0.5, 1, 10), 1000);
        assert_eq!(next_reputation(5, -0.5, 10, 10), 0);
    }
}
