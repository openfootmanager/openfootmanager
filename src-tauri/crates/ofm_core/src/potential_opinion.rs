//! A judge's opinion of a player's potential: a range, never the number. A scout writes one into
//! his report; the club's staff hold one about its own players. Who may see it is the knowledge
//! rule's to decide (`player_knowledge::potential_opinion`); this module only says what a judge of
//! a given eye would say.

use rand::RngExt;
use serde::Serialize;
use ts_rs::TS;

use crate::game::PotentialRange;
use crate::player_tier::PlayerTier;

/// How sure a judge is, by his `judging_potential`. The band also sets how wide his range is, so
/// it can always be read back off a range's width.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum Certainty {
    Confident,
    FairlySure,
    Unsure,
    Guess,
}

impl Certainty {
    /// The band a judge's eye puts him in: 80 and up, 60 to 79, 40 to 59, and below. These are the
    /// bands the scouting module already reads `judging_potential` by.
    fn of_judge(judging_potential: u8) -> Self {
        if judging_potential >= 80 {
            Certainty::Confident
        } else if judging_potential >= 60 {
            Certainty::FairlySure
        } else if judging_potential >= 40 {
            Certainty::Unsure
        } else {
            Certainty::Guess
        }
    }

    /// How far either side of his centre a judge in this band reaches.
    fn half_width(self) -> u8 {
        match self {
            Certainty::Confident => 2,
            Certainty::FairlySure => 4,
            Certainty::Unsure => 6,
            Certainty::Guess => 9,
        }
    }

    /// The band a range was drawn in, read off its width: a range always keeps its judge's width
    /// (see [`judged_range`]), so nothing about the judge needs storing beside it.
    fn of_range(range: PotentialRange) -> Self {
        match range.high.saturating_sub(range.low) / 2 {
            ..=2 => Certainty::Confident,
            3..=4 => Certainty::FairlySure,
            5..=6 => Certainty::Unsure,
            _ => Certainty::Guess,
        }
    }
}

/// The range a judge with `judging_potential` gives a player whose true potential is `potential`.
///
/// Its centre is the true value moved by an offset no wider than the half-width, so the true value
/// is inside on the day it is formed: the opinion is vague, never wrong. (A scout's range is kept
/// with his report; once the player develops past a season end, it describes him as he was.) The
/// offset is drawn from the player and the
/// judge alone, so the same judge always says the same about the same player. Near either end of
/// the 1-99 scale the window is moved inside rather than cut, so it keeps its width (and the true
/// value) there too.
pub fn judged_range(
    player_id: &str,
    judge_id: &str,
    potential: u8,
    judging_potential: u8,
) -> PotentialRange {
    let half_width = i16::from(Certainty::of_judge(judging_potential).half_width());
    let mut rng = crate::seed::rng_from_key(&format!("potential-opinion/{player_id}/{judge_id}"));
    let offset: i16 = rng.random_range(-half_width..=half_width);
    let centre = i16::from(potential) + offset;
    let low = (centre - half_width).clamp(1, 99 - 2 * half_width);
    PotentialRange {
        low: narrow(low),
        high: narrow(low + 2 * half_width),
    }
}

/// A rating already held inside 1-99.
fn narrow(rating: i16) -> u8 {
    u8::try_from(rating.clamp(1, 99)).unwrap_or(1)
}

/// Whose opinion it is: a scout's written report, or the club's own staff, who see the player
/// train every day.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OpinionSource {
    Scout {
        /// None for a report written before reports named their scout.
        scout_name: Option<String>,
        scouted_on: String,
        /// False for a report written before the last season ended: the player has aged and
        /// developed since, so the range describes him as he was (`scouted_report::is_out_of_date`,
        /// the same verdict `player_knowledge` gives the report).
        current: bool,
    },
    Coaches,
}

/// What the club may read of a player's potential: the range, the band its middle falls in (the
/// backend's tier, so the screen picks no threshold), how sure the judge is, and whose opinion it
/// is. Never the true potential. The range held the true value when it was formed: the coaches'
/// is formed today, a scout's on the day of his report, so an out-of-date report's range may no
/// longer hold it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct PotentialOpinion {
    pub low: u8,
    pub high: u8,
    pub band: PlayerTier,
    pub certainty: Certainty,
    pub source: OpinionSource,
}

impl PotentialOpinion {
    pub fn of(range: PotentialRange, source: OpinionSource) -> Self {
        let middle = u8::try_from((u16::from(range.low) + u16::from(range.high)) / 2).unwrap_or(1);
        Self {
            low: range.low,
            high: range.high,
            band: crate::player_tier::player_tier(middle),
            certainty: Certainty::of_range(range),
            source,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const JUDGES: [u8; 4] = [90, 70, 50, 20];

    /// Given every potential on the scale, judges of every band, and several players
    /// When each judge gives his range
    /// Then the true potential is inside it, and it stays on the 1-99 scale
    #[test]
    fn a_judged_range_always_holds_the_true_potential() {
        for potential in 1..=99u8 {
            for judging in JUDGES {
                for player in ["p0", "p1", "p2", "p3", "p4"] {
                    let range = judged_range(player, "judge", potential, judging);
                    assert!(
                        range.low <= potential && potential <= range.high,
                        "{potential} outside {range:?} (judging {judging}, {player})"
                    );
                    assert!(
                        range.low >= 1 && range.high <= 99,
                        "{range:?} off the scale"
                    );
                }
            }
        }
    }

    /// Given the same player judged by a scout with an eye of 85 and one with an eye of 30
    /// When each gives his range
    /// Then the better scout's is narrower: 4 points wide against 18
    #[test]
    fn a_better_judge_gives_a_narrower_range() {
        let sharp = judged_range("p", "sharp", 60, 85);
        let poor = judged_range("p", "poor", 60, 30);

        assert_eq!(sharp.high - sharp.low, 4);
        assert_eq!(poor.high - poor.low, 18);
    }

    /// Given a judge and a player
    /// When the judge gives his range twice
    /// Then it is the same range: opening the profile again does not roll a new one
    #[test]
    fn the_same_judge_gives_the_same_range_twice() {
        assert_eq!(
            judged_range("p", "judge", 72, 55),
            judged_range("p", "judge", 72, 55)
        );
    }

    /// Given players at the very top and bottom of the scale and a poor judge
    /// When he gives his ranges
    /// Then each keeps its full width, moved inside the scale rather than cut short
    #[test]
    fn a_range_at_the_edge_of_the_scale_keeps_its_width() {
        for potential in [1, 2, 98, 99] {
            let range = judged_range("p", "poor", potential, 20);
            assert_eq!(range.high - range.low, 18, "{range:?} for {potential}");
        }
    }

    /// Given judges of every band and players across the scale, its ends included
    /// When a range is read back
    /// Then its width names the judge's band
    #[test]
    fn certainty_reads_the_judges_band_off_the_range() {
        for judging in JUDGES {
            for potential in [1, 50, 99] {
                let range = judged_range("p", "judge", potential, judging);
                assert_eq!(
                    Certainty::of_range(range),
                    Certainty::of_judge(judging),
                    "judging {judging}, potential {potential}"
                );
            }
        }
    }

    /// Given a range of 76-84, whose middle is 80
    /// When it becomes an opinion
    /// Then its band is the tier of 80 (A), read by the backend's one tier rule
    #[test]
    fn an_opinions_band_is_the_tier_of_its_middle() {
        let opinion =
            PotentialOpinion::of(PotentialRange { low: 76, high: 84 }, OpinionSource::Coaches);

        assert_eq!(opinion.band, PlayerTier::A);
        assert_eq!(opinion.certainty, Certainty::FairlySure);
    }
}
