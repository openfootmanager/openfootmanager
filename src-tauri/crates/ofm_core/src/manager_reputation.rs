//! The reputation a retired player starts his managing career with.
//!
//! Every manager is created on one scale, [`CREATED_REPUTATION`]: the generator's opening draw, a
//! stand-in's coaching-based standing (`ai_hiring`), and a retiree's here. The retiree rule used
//! to be `200 + ovr×6 + seasons×30`, clamped at 900. A generated world gives every player twelve
//! seasons of history, so the seasons term alone added 360 and 99% of retirees started at 900
//! (#1160), above every manager the world opened with.

use std::collections::BTreeSet;

use domain::manager::CREATED_REPUTATION;
use domain::player::CareerEntry;

/// The rating at which a retiree's ability starts to count. Measured on retiring players (#1160
/// probe, seeds 1 and 2): p1 54–56, p50 69–70, p99 81, max 85.
const ABILITY_FLOOR: u8 = 55;

/// Reputation per rating point above the floor. With the career term at twelve seasons, the
/// measured retirees land at p10 ≈ 370, p50 ≈ 480, p90 ≈ 560 and p99 ≈ 630: inside the scale the
/// opening managers fill uniformly (200..=700), and a retiring star starts as a promising manager,
/// not an established one.
const PER_ABILITY_POINT: u32 = 13;

/// Seasons played after which a longer career adds nothing. A generated world's retirees all have
/// 12 or 13, later ones up to 15.
const FULL_CAREER_SEASONS: u32 = 15;

/// Reputation per season played, up to [`FULL_CAREER_SEASONS`]: a full career is worth about eight
/// rating points, so ability decides the standing and a short career only trims it.
const PER_SEASON_PLAYED: u32 = 7;

/// The reputation a player who retired at `ovr` after `seasons_played` seasons starts managing
/// with: on [`CREATED_REPUTATION`], never past its top.
pub(crate) fn retired_player_reputation(ovr: u8, seasons_played: u32) -> u32 {
    let ability = u32::from(ovr.saturating_sub(ABILITY_FLOOR)) * PER_ABILITY_POINT;
    let career = seasons_played.min(FULL_CAREER_SEASONS) * PER_SEASON_PLAYED;
    (CREATED_REPUTATION.start() + ability + career).min(*CREATED_REPUTATION.end())
}

/// The seasons a player actually played: distinct seasons in which he made an appearance. A season
/// split between two clubs is one season, and a season on the books without a game is none.
pub(crate) fn seasons_played(career: &[CareerEntry]) -> u32 {
    let seasons: BTreeSet<u32> = career
        .iter()
        .filter(|entry| entry.appearances > 0)
        .map(|entry| entry.season)
        .collect();
    u32::try_from(seasons.len()).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn season(season: u32, team_id: &str, appearances: u32) -> CareerEntry {
        CareerEntry {
            season,
            team_id: team_id.into(),
            team_name: team_id.into(),
            appearances,
            goals: 0,
            assists: 0,
        }
    }

    /// Given a retiree at the measured median: rating 70 after twelve seasons
    /// When his starting reputation is worked out
    /// Then it is 200 + 15×13 + 12×7 = 479, in the middle of the scale the opening managers fill
    #[test]
    fn a_median_retiree_starts_mid_scale() {
        assert_eq!(retired_player_reputation(70, 12), 479);
    }

    /// Given two retirees with the same career, one rating point apart
    /// When their starting reputations are worked out
    /// Then the better player starts 13 higher
    #[test]
    fn each_rating_point_is_worth_thirteen() {
        assert_eq!(
            retired_player_reputation(71, 12) - retired_player_reputation(70, 12),
            13
        );
    }

    /// Given two retirees of the same rating, one season apart, both short of a full career
    /// When their starting reputations are worked out
    /// Then the longer career starts 7 higher
    #[test]
    fn each_season_played_is_worth_seven() {
        assert_eq!(
            retired_player_reputation(70, 13) - retired_player_reputation(70, 12),
            7
        );
    }

    /// Given two retirees of the same rating, with 15 and 20 seasons played
    /// When their starting reputations are worked out
    /// Then they are equal: a career past fifteen seasons adds nothing
    #[test]
    fn a_career_past_fifteen_seasons_adds_nothing() {
        assert_eq!(
            retired_player_reputation(70, 20),
            retired_player_reputation(70, 15)
        );
    }

    /// Given retirees rated at and below the ability floor of 55
    /// When their starting reputations are worked out
    /// Then their ability adds nothing: the weakest start at the floor plus their career
    #[test]
    fn ability_below_the_floor_adds_nothing() {
        assert_eq!(retired_player_reputation(55, 0), 200);
        assert_eq!(retired_player_reputation(40, 0), 200);
    }

    /// Given the best retiree the probe saw (85) after a full career, and an impossible 99
    /// When their starting reputations are worked out
    /// Then neither passes 700, the top of the scale every manager is created on (it was 900)
    #[test]
    fn no_retiree_starts_above_the_created_scale() {
        assert_eq!(retired_player_reputation(85, 15), 695);
        assert_eq!(retired_player_reputation(99, 15), 700);
    }

    /// Given a career with a season split between two clubs and a season without a game
    /// When the seasons he played are counted
    /// Then the split season counts once and the empty one not at all
    #[test]
    fn seasons_played_counts_seasons_with_a_game_once() {
        let career = vec![
            season(2030, "a", 30),
            season(2031, "a", 12),
            season(2031, "b", 15),
            season(2032, "b", 0),
        ];

        assert_eq!(seasons_played(&career), 2);
    }
}
