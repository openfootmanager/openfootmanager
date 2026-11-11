//! A manager's reputation: the band a screen reads it in, and the reputation a retired player
//! starts his managing career with.
//!
//! Every manager is created on one scale, [`CREATED_REPUTATION`]: the generator's opening draw, a
//! stand-in's coaching-based standing (`ai_hiring`), and a retiree's here. The retiree rule used
//! to be `200 + ovr×6 + seasons×30`, clamped at 900. A generated world gives every player twelve
//! seasons of history, so the seasons term alone added 360 and 99% of retirees started at 900
//! (#1160), above every manager the world opened with.

use std::collections::BTreeSet;

use domain::manager::CREATED_REPUTATION;
use domain::player::CareerEntry;
use serde::Serialize;
use ts_rs::TS;

use crate::rated_tone::RatedTone;

/// The word a manager's reputation reads as, highest first (maintainer, 2 Nov 2026: seven manager
/// words, not the club bands). Serialized snake_case, as each word's translation key is named
/// (`managersWorld.reputationBand.<band>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ReputationBand {
    WorldClass,
    Established,
    Great,
    Good,
    Average,
    BelowAverage,
    Unknown,
}

/// The lowest reputation of each band, highest band first. Placed on the real spread of the
/// managers a world opens with, roughly uniform over 200..=700 (approved 3 Nov 2026): World class
/// is the top 6% or so, Unknown the bottom 10%.
const BAND_FLOORS: [(u32, ReputationBand); 6] = [
    (670, ReputationBand::WorldClass),
    (620, ReputationBand::Established),
    (550, ReputationBand::Great),
    (450, ReputationBand::Good),
    (350, ReputationBand::Average),
    (250, ReputationBand::BelowAverage),
];

impl ReputationBand {
    /// The band `reputation` falls in.
    pub fn of(reputation: u32) -> Self {
        BAND_FLOORS
            .iter()
            .find(|(floor, _)| reputation >= *floor)
            .map_or(Self::Unknown, |(_, band)| *band)
    }

    /// Where the band sits on the good-to-bad scale the screens paint with.
    pub fn tone(self) -> RatedTone {
        match self {
            Self::WorldClass => RatedTone::Top,
            Self::Established | Self::Great => RatedTone::Good,
            Self::Good => RatedTone::Fair,
            Self::Average | Self::BelowAverage => RatedTone::Low,
            Self::Unknown => RatedTone::Bad,
        }
    }
}

/// A manager's reputation as a screen shows it: the number, its band and the band's tone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
pub struct ManagerReputation {
    pub value: u32,
    pub band: ReputationBand,
    pub tone: RatedTone,
}

impl ManagerReputation {
    pub fn of(value: u32) -> Self {
        let band = ReputationBand::of(value);
        Self {
            value,
            band,
            tone: band.tone(),
        }
    }
}

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

    /// Given each band's floor and the reputation one below it
    /// When the band is read
    /// Then the floor is in the band and one below is in the band beneath (the approved
    ///      thresholds: 670, 620, 550, 450, 350, 250)
    #[test]
    fn each_band_starts_at_its_floor() {
        use ReputationBand::*;
        let edges = [
            (670, WorldClass, Established),
            (620, Established, Great),
            (550, Great, Good),
            (450, Good, Average),
            (350, Average, BelowAverage),
            (250, BelowAverage, Unknown),
        ];
        for (floor, at_floor, below) in edges {
            assert_eq!(
                (ReputationBand::of(floor), ReputationBand::of(floor - 1)),
                (at_floor, below),
                "the band edge at {floor}"
            );
        }
    }

    /// Given the ends of every scale a reputation is on: 0, and past any cap
    /// When the band is read
    /// Then 0 is Unknown and a huge reputation World class
    #[test]
    fn the_bands_cover_every_reputation() {
        assert_eq!(ReputationBand::of(0), ReputationBand::Unknown);
        assert_eq!(ReputationBand::of(u32::MAX), ReputationBand::WorldClass);
    }

    /// Given each band
    /// When its tone is read
    /// Then World class is top, Established and Great good, Good fair, Average and Below average
    ///      low, Unknown bad (approved 3 Nov 2026)
    #[test]
    fn each_band_has_its_tone() {
        use ReputationBand::*;
        let tones: Vec<RatedTone> = [
            WorldClass,
            Established,
            Great,
            Good,
            Average,
            BelowAverage,
            Unknown,
        ]
        .into_iter()
        .map(ReputationBand::tone)
        .collect();
        assert_eq!(
            tones,
            vec![
                RatedTone::Top,
                RatedTone::Good,
                RatedTone::Good,
                RatedTone::Fair,
                RatedTone::Low,
                RatedTone::Low,
                RatedTone::Bad,
            ]
        );
    }

    /// Given a manager at 600
    /// When his reputation is projected for a screen
    /// Then it carries the number, Great and its tone, and serializes as the screens read it
    #[test]
    fn a_reputation_carries_its_band_and_tone_on_the_wire() {
        let reputation = ManagerReputation::of(600);

        assert_eq!(
            serde_json::to_value(reputation).unwrap(),
            serde_json::json!({ "value": 600, "band": "great", "tone": "good" })
        );
    }

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
