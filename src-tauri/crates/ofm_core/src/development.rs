//! How a player's ability moves with age (#869).
//!
//! One smooth curve, bent per player. Up to the late twenties a player grows by a number of rating
//! points that shrinks with age and with how close he already is to his ceiling, and never stops
//! at a threshold. From then on a table of changes per attribute group takes over: pace and agility
//! go first and fastest, stamina and strength slower, technique slower still, and the mental
//! attributes keep rising for a few years before they fall. Two traits drawn once per player from
//! his own stream shift the whole curve (an early or late bloomer) and slow or speed his decline
//! (how long he lasts), so a few players are still good at 35.
//!
//! The numbers (growth per age, the decline table, the trait spreads, the ceiling drift) are
//! written out as constants below and approved on #869; they are tuned against the multi-season
//! reports of `season-harness::ageing`, not by hand.

use domain::player::{Player, PlayerAttributes, Position};
use rand::{Rng, RngExt};

use crate::player_rating::{move_attributes_to, natural_ovr};
use crate::seed::{rng_for_seed, standard_normal};

/// Growth per season, in rating points, of a player far below his ceiling, for curve ages 16 to
/// 26. Below 16 it is the first value; at 27 it is gone and the decline table governs.
const GROWTH_BY_AGE: [f64; 11] = [6.0, 6.0, 5.5, 5.0, 4.5, 4.0, 3.5, 3.0, 2.5, 2.0, 1.2];
const FIRST_GROWTH_AGE: f64 = 16.0;
/// The curve age from which the decline table replaces growth.
const DECLINE_STARTS_AT: f64 = 27.0;
/// The curve age from which a player's ceiling is his rating. The decline table starts at 27, but
/// it is rating-neutral until about 32 (pace falls while the mind still gains), and a player who
/// still has room to his ceiling at 27 to 29 is still on his way to it: from 30 the table really
/// takes rating points, and a falling player has no room to grow.
const CEILING_FOLLOWS_RATING_FROM: f64 = 30.0;
/// The gap to the ceiling at which growth is half what it would be far below it.
const BRAKE_HALF_GAP: f64 = 6.0;

/// A goalkeeper's curve runs this many years later than an outfield player's.
const GOALKEEPER_YEARS_LATER: f64 = 3.5;
/// A goalkeeper's physical decline is this fraction of an outfield player's.
const GOALKEEPER_PHYSICAL_DECLINE: f64 = 0.5;

const PEAK_OFFSET_SD: f64 = 1.5;
const PEAK_OFFSET_LIMIT: f64 = 3.0;
const RESILIENCE_SIGMA: f64 = 0.35;
const RESILIENCE_RANGE: (f64, f64) = (0.5, 2.2);

/// What the ceiling does each season until the player's peak: a random step (see
/// `ceiling_step_spread`) plus a small pull down so the world does not fill with ever-higher
/// ceilings. There is no pull toward a common centre: a high ceiling stays a high ceiling, so a
/// scout's report of a teenager's potential means something. The generator keeps the world's bell
/// the width it was calibrated to by drawing a young player's ceiling from a narrower bell, through
/// `ceiling_drift_still_to_come`.
const CEILING_PULL: f64 = -0.3;
const FIRST_DRIFT_AGE: u32 = 16;
const LAST_DRIFT_AGE: u32 = 24;

/// Training is a modifier on the season's development, not a second source of it: the season's
/// growth already stands for a typical year in the training ground, and a day's training adds to
/// it by this share of the weight it once carried alone.
const TRAINING_SHARE: f64 = 0.25;

/// The five groups the decline table is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Aspect {
    /// Pace and agility.
    Speed,
    Stamina,
    Strength,
    /// Passing, shooting, tackling, dribbling, defending and the keeping attributes.
    Technical,
    Mental,
}

/// The two traits drawn once per player: when his curve peaks, and how well he holds his level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DevelopmentTraits {
    /// Years the curve is shifted later (positive: a late bloomer).
    pub peak_offset: f64,
    /// Divides the size of a decline: above 1 he declines more slowly.
    pub resilience: f64,
}

/// A player's traits, from the world's seed and his id alone: the same every time they are asked
/// for, so they need no place in a save.
pub fn traits_of(world_seed: u64, player_id: &str) -> DevelopmentTraits {
    let mut rng = rng_for_seed(world_seed, &format!("development/traits/{player_id}"), "-");
    let peak_offset =
        (standard_normal(&mut rng) * PEAK_OFFSET_SD).clamp(-PEAK_OFFSET_LIMIT, PEAK_OFFSET_LIMIT);
    let resilience = (standard_normal(&mut rng) * RESILIENCE_SIGMA)
        .exp()
        .clamp(RESILIENCE_RANGE.0, RESILIENCE_RANGE.1);
    DevelopmentTraits {
        peak_offset,
        resilience,
    }
}

fn is_goalkeeper(player: &Player) -> bool {
    player.natural_position.to_group_position() == Position::Goalkeeper
}

/// Years a position's curve is shifted: wingers and forwards peak a year earlier, central
/// defenders a year later.
fn position_offset(position: &Position) -> f64 {
    match position {
        Position::Forward | Position::Striker | Position::RightWinger | Position::LeftWinger => {
            -1.0
        }
        Position::CenterBack | Position::Defender => 1.0,
        _ => 0.0,
    }
}

/// The age the curve reads for this player: his own, moved by when he peaks, where he plays and
/// whether he is a goalkeeper. A late bloomer of 28 reads as 26.
pub fn curve_age(player: &Player, age: i32, traits: &DevelopmentTraits) -> f64 {
    let keeper_shift = if is_goalkeeper(player) {
        GOALKEEPER_YEARS_LATER
    } else {
        0.0
    };
    f64::from(age) - traits.peak_offset - position_offset(&player.natural_position) - keeper_shift
}

/// Rating points a player at `curve_age` grows in a season when he is far below his ceiling:
/// the table, read smoothly between its ages, and nothing from `DECLINE_STARTS_AT`.
fn base_growth(curve_age: f64) -> f64 {
    if curve_age >= DECLINE_STARTS_AT {
        return 0.0;
    }
    let position = (curve_age - FIRST_GROWTH_AGE).max(0.0);
    let lower = position.floor() as usize;
    let lower_value = GROWTH_BY_AGE[lower.min(GROWTH_BY_AGE.len() - 1)];
    let upper_value = GROWTH_BY_AGE.get(lower + 1).copied().unwrap_or(0.0);
    lower_value + (upper_value - lower_value) * (position - lower as f64)
}

/// The share of that growth left when `gap` rating points remain to the ceiling: half at a gap of
/// six, tending to nothing as the gap closes, and never a gate.
fn headroom_brake(gap: u8) -> f64 {
    let gap = f64::from(gap);
    gap / (gap + BRAKE_HALF_GAP)
}

/// Expected rating points grown in a season by a player at `curve_age` with `gap` left to his
/// ceiling.
fn growth_points(curve_age: f64, gap: u8) -> f64 {
    base_growth(curve_age) * headroom_brake(gap)
}

/// How much of a day's training a player at `curve_age` still takes, with `gap` points left to his
/// ceiling: the most when young, tapering smoothly to nothing by the mid-thirties, and slowed by
/// the same brake as growth. Replaces both the step by age and the gate at the ceiling.
pub fn training_factor(curve_age: f64, gap: u8) -> f64 {
    const POINTS: [(f64, f64); 5] = [
        (21.0, 1.5),
        (26.0, 1.0),
        (30.0, 0.3),
        (34.0, 0.0),
        (99.0, 0.0),
    ];
    let age_factor = if curve_age <= POINTS[0].0 {
        POINTS[0].1
    } else {
        POINTS
            .windows(2)
            .find(|pair| curve_age <= pair[1].0)
            .map_or(0.0, |pair| {
                let share = (curve_age - pair[0].0) / (pair[1].0 - pair[0].0);
                pair[0].1 + (pair[1].1 - pair[0].1) * share
            })
    };
    TRAINING_SHARE * age_factor * headroom_brake(gap)
}

/// The expected change in points of one attribute in a season, for a player the decline table
/// governs (curve age 27 or more), at resilience 1.
fn decline_per_season(curve_age: f64, aspect: Aspect) -> f64 {
    let band = match curve_age.floor() as i32 {
        ..=26 => return 0.0,
        27..=29 => [-0.5, 0.0, 0.0, 0.0, 0.5],
        30..=31 => [-2.0, -0.7, -0.3, -0.8, 0.3],
        32..=33 => [-2.5, -1.2, -0.8, -1.5, 0.0],
        34..=35 => [-3.0, -2.0, -1.2, -2.2, -0.5],
        _ => [-4.0, -2.5, -1.5, -3.0, -1.0],
    };
    match aspect {
        Aspect::Speed => band[0],
        Aspect::Stamina => band[1],
        Aspect::Strength => band[2],
        Aspect::Technical => band[3],
        Aspect::Mental => band[4],
    }
}

/// How a season's growth is shared between physical, technical and mental attributes at this
/// curve age (they add to 1).
fn growth_shares(curve_age: f64) -> (f64, f64, f64) {
    match curve_age.floor() as i32 {
        ..=17 => (0.45, 0.35, 0.20),
        18..=21 => (0.25, 0.45, 0.30),
        22..=25 => (0.10, 0.40, 0.50),
        _ => (0.0, 0.30, 0.70),
    }
}

/// Rounds `value` to a whole number without losing the fraction on average: 1.3 is 1 seven times
/// in ten and 2 three times in ten.
fn stochastic_round(value: f64, rng: &mut impl Rng) -> i32 {
    let whole = value.floor();
    let fraction = value - whole;
    whole as i32 + i32::from(rng.random_range(0.0..1.0) < fraction)
}

fn nudge(attribute: &mut u8, change: i32) {
    *attribute = (i32::from(*attribute) + change).clamp(1, 99) as u8;
}

/// Each attribute with the aspect it belongs to.
fn aspects_of(attributes: &mut PlayerAttributes) -> [(&mut u8, Aspect); 19] {
    [
        (&mut attributes.pace, Aspect::Speed),
        (&mut attributes.agility, Aspect::Speed),
        (&mut attributes.stamina, Aspect::Stamina),
        (&mut attributes.strength, Aspect::Strength),
        (&mut attributes.passing, Aspect::Technical),
        (&mut attributes.shooting, Aspect::Technical),
        (&mut attributes.tackling, Aspect::Technical),
        (&mut attributes.dribbling, Aspect::Technical),
        (&mut attributes.defending, Aspect::Technical),
        (&mut attributes.handling, Aspect::Technical),
        (&mut attributes.reflexes, Aspect::Technical),
        (&mut attributes.aerial, Aspect::Technical),
        (&mut attributes.positioning, Aspect::Mental),
        (&mut attributes.vision, Aspect::Mental),
        (&mut attributes.decisions, Aspect::Mental),
        (&mut attributes.composure, Aspect::Mental),
        (&mut attributes.aggression, Aspect::Mental),
        (&mut attributes.teamwork, Aspect::Mental),
        (&mut attributes.leadership, Aspect::Mental),
    ]
}

fn is_physical(aspect: Aspect) -> bool {
    matches!(aspect, Aspect::Speed | Aspect::Stamina | Aspect::Strength)
}

/// Grows a player by the season's expected points: spread across his attributes by the age's
/// shares and each one's own luck, then the whole brought to the rating the points call for, so
/// the rounding of many small changes cannot drift the total.
fn grow(player: &mut Player, points: f64, curve_age: f64, rng: &mut impl Rng) {
    // A season's growth closes the gap to the ceiling at most; it does not cross it, because a
    // rating above the ceiling would lift the ceiling with it.
    let room = i32::from(player.potential) - i32::from(player.ovr);
    let target = i32::from(player.ovr) + stochastic_round(points, rng).min(room.max(0));
    let (physical, technical, mental) = growth_shares(curve_age);
    let total_attribute_points = points * 19.0;
    let counts = (4.0, 8.0, 7.0);
    for (attribute, aspect) in aspects_of(&mut player.attributes) {
        let (share, count) = if is_physical(aspect) {
            (physical, counts.0)
        } else if aspect == Aspect::Technical {
            (technical, counts.1)
        } else {
            (mental, counts.2)
        };
        let luck: f64 = rng.random_range(0.5..1.5);
        let change = stochastic_round(total_attribute_points * share / count * luck, rng);
        nudge(attribute, change);
    }
    let target = target.clamp(1, 99) as u8;
    let position = player.natural_position.clone();
    move_attributes_to(&mut player.attributes, &position, target);
}

/// Ages a player's attributes by a season of decline, each by the table's change for its group,
/// slowed by his resilience, halved in the body for a goalkeeper, and given its own luck.
fn decline(player: &mut Player, curve_age: f64, traits: &DevelopmentTraits, rng: &mut impl Rng) {
    let keeper = is_goalkeeper(player);
    for (attribute, aspect) in aspects_of(&mut player.attributes) {
        let mut change = decline_per_season(curve_age, aspect);
        if change < 0.0 {
            change /= traits.resilience;
            if keeper && is_physical(aspect) {
                change *= GOALKEEPER_PHYSICAL_DECLINE;
            }
        }
        let luck: f64 = rng.random_range(0.5..1.5);
        nudge(attribute, stochastic_round(change * luck, rng));
    }
}

/// The standard deviation of one season's step of the ceiling at this age: largest for a
/// teenager and gone after 24 (about three points either way over the whole run, a year's step
/// is a fraction of that).
fn ceiling_step_spread(age: u32) -> f64 {
    match age {
        ..=18 => 1.0,
        19..=21 => 0.7,
        22..=24 => 0.35,
        _ => 0.0,
    }
}

/// The mean and the variance of the ceiling's drift from a player aged `age` until it stops, so
/// the generator can draw a young player's ceiling from a bell narrower by that variance and
/// everyone's ceiling at 25 follows the one bell. (A player's rating holds his ceiling up once
/// he has grown into it, so the mean is only the drift before that; see `shaped_ceiling`.)
pub(crate) fn ceiling_drift_still_to_come(age: u32) -> (f64, f64) {
    let steps = FIRST_DRIFT_AGE.max(age)..=LAST_DRIFT_AGE;
    let count = steps.clone().count() as f64;
    let variance = steps.map(|a| ceiling_step_spread(a).powi(2)).sum();
    (CEILING_PULL * count, variance)
}

/// Moves the ceiling for the season: a random step that is largest for a teenager and gone by 25,
/// with a small pull down. Never below the rating he has. It runs on calendar age, not curve age,
/// because the generator narrows a young player's ceiling by the drift still to come at his
/// calendar age (`ceiling_drift_still_to_come`): a keeper's curve is 3.5 years later, and reading
/// it here would give him seven seasons of the widest step where the generator allowed three.
fn drift_ceiling(player: &mut Player, age: u32, rng: &mut impl Rng) {
    let spread = ceiling_step_spread(age);
    if spread == 0.0 {
        return;
    }
    let step = standard_normal(rng) * spread + CEILING_PULL;
    let ceiling = (f64::from(player.potential) + step)
        .round()
        .clamp(1.0, 99.0) as u8;
    player.potential = ceiling.max(natural_ovr(player).round() as u8);
}

/// Once a player's curve reads `CEILING_FOLLOWS_RATING_FROM` his ceiling is his rating: it comes
/// down with him and, being the rating, never sits above it, so a falling veteran never shows room
/// to grow. A young player's ceiling is not touched. The switch is on the curve, not the
/// birthday: a late bloomer keeps his ceiling while his own curve is short of it.
fn bring_ceiling_down_to_rating(player: &mut Player) {
    player.potential = (natural_ovr(player).round() as u8).max(1);
}

/// One season of development for a player aged `age`: growth while his curve says he grows, the
/// decline table once it says he declines, the drift of a young player's ceiling on the way, and
/// from the curve age at which he really falls, the ceiling brought down to his rating. Draws from
/// `rng`, which the caller derives from the player and the date so a replay gives the same season.
pub fn develop_one_season(player: &mut Player, age: i32, world_seed: u64, rng: &mut impl Rng) {
    let traits = traits_of(world_seed, &player.id);
    let curve = curve_age(player, age, &traits);
    if curve >= DECLINE_STARTS_AT {
        decline(player, curve, &traits, rng);
    } else {
        let gap = player
            .potential
            .saturating_sub(natural_ovr(player).round() as u8);
        grow(player, growth_points(curve, gap), curve, rng);
    }
    drift_ceiling(player, age.max(0) as u32, rng);
    if curve >= CEILING_FOLLOWS_RATING_FROM {
        bring_ceiling_down_to_rating(player);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::uniform_attributes;
    use rand::SeedableRng;
    use rand_chacha::ChaCha12Rng;

    fn player(id: &str, position: Position, ovr: u8, potential: u8) -> Player {
        let mut player = Player::new(
            id.to_string(),
            "P".to_string(),
            "P P".to_string(),
            "2000-01-01".to_string(),
            "England".to_string(),
            position,
            uniform_attributes(ovr),
        );
        player.ovr = natural_ovr(&player).round() as u8;
        player.potential = potential;
        player
    }

    fn rng(seed: u64) -> ChaCha12Rng {
        ChaCha12Rng::seed_from_u64(seed)
    }

    fn neutral() -> DevelopmentTraits {
        DevelopmentTraits {
            peak_offset: 0.0,
            resilience: 1.0,
        }
    }

    /// Given the growth table's own ages,
    /// When the base growth is read,
    /// Then it is the table's value at 16, 21 and 26, smooth between them, and none from 27.
    #[test]
    fn growth_follows_the_table_and_is_smooth_between_its_ages() {
        assert_eq!(base_growth(14.0), 6.0);
        assert_eq!(base_growth(16.0), 6.0);
        assert_eq!(base_growth(21.0), 4.0);
        assert_eq!(base_growth(26.0), 1.2);
        assert!((base_growth(21.5) - 3.75).abs() < 1e-9);
        assert!((base_growth(26.5) - 0.6).abs() < 1e-9);
        assert_eq!(base_growth(27.0), 0.0);
        assert_eq!(base_growth(33.0), 0.0);
    }

    /// Given a gap of zero, six and thirty-six points to the ceiling,
    /// When the brake is read,
    /// Then growth is nothing, half and six sevenths of what it would be far below the ceiling.
    #[test]
    fn the_brake_slows_growth_toward_the_ceiling_without_a_gate() {
        assert_eq!(headroom_brake(0), 0.0);
        assert!((headroom_brake(6) - 0.5).abs() < 1e-9);
        assert!((headroom_brake(36) - 36.0 / 42.0).abs() < 1e-9);
        assert!(growth_points(22.0, 1) > 0.0);
    }

    /// Given the decline table's bands,
    /// When the change per season is read at an age in each,
    /// Then it is the table's, for every group, and nothing below 27.
    #[test]
    fn the_decline_table_is_read_by_age_band_and_group() {
        assert_eq!(decline_per_season(25.0, Aspect::Speed), 0.0);
        assert_eq!(decline_per_season(28.0, Aspect::Mental), 0.5);
        assert_eq!(decline_per_season(28.0, Aspect::Speed), -0.5);
        assert_eq!(decline_per_season(30.0, Aspect::Speed), -2.0);
        assert_eq!(decline_per_season(31.9, Aspect::Stamina), -0.7);
        assert_eq!(decline_per_season(32.0, Aspect::Strength), -0.8);
        assert_eq!(decline_per_season(34.0, Aspect::Technical), -2.2);
        assert_eq!(decline_per_season(34.0, Aspect::Mental), -0.5);
        assert_eq!(decline_per_season(36.0, Aspect::Speed), -4.0);
        assert_eq!(decline_per_season(40.0, Aspect::Mental), -1.0);
    }

    /// Given a goalkeeper, a winger, a central defender and a midfielder of 30,
    /// When their curve age is read with neutral traits,
    /// Then the keeper reads 3.5 years younger, the winger a year older and the defender a year
    ///      younger than the midfielder.
    #[test]
    fn the_curve_age_is_moved_by_position_and_by_the_goalkeeper_shift() {
        let age = |position| curve_age(&player("p", position, 60, 60), 30, &neutral());
        assert_eq!(age(Position::CentralMidfielder), 30.0);
        assert_eq!(age(Position::Goalkeeper), 26.5);
        assert_eq!(age(Position::RightWinger), 31.0);
        assert_eq!(age(Position::CenterBack), 29.0);
        let late = DevelopmentTraits {
            peak_offset: 2.0,
            resilience: 1.0,
        };
        assert_eq!(
            curve_age(&player("p", Position::CentralMidfielder, 60, 60), 28, &late),
            26.0
        );
    }

    /// Given a world seed and two player ids,
    /// When their traits are drawn twice,
    /// Then each player gets the same traits both times and the two differ from one another.
    #[test]
    fn a_players_traits_are_his_own_and_never_change() {
        assert_eq!(traits_of(7, "alpha"), traits_of(7, "alpha"));
        assert_ne!(traits_of(7, "alpha"), traits_of(7, "beta"));
        assert_ne!(traits_of(7, "alpha"), traits_of(8, "alpha"));
    }

    /// Given three thousand players,
    /// When their traits are drawn,
    /// Then every one is inside its limits, the peak offsets centre on zero with a spread near
    ///      1.5, and the resilience centres on one.
    #[test]
    fn the_traits_have_the_distribution_the_model_asks_for() {
        let drawn: Vec<DevelopmentTraits> = (0..3000)
            .map(|n| traits_of(11, &format!("player-{n}")))
            .collect();
        assert!(
            drawn
                .iter()
                .all(|t| t.peak_offset.abs() <= PEAK_OFFSET_LIMIT)
        );
        assert!(
            drawn
                .iter()
                .all(|t| (RESILIENCE_RANGE.0..=RESILIENCE_RANGE.1).contains(&t.resilience))
        );
        let offsets: Vec<f64> = drawn.iter().map(|t| t.peak_offset).collect();
        let mean = offsets.iter().sum::<f64>() / offsets.len() as f64;
        let spread =
            (offsets.iter().map(|o| (o - mean).powi(2)).sum::<f64>() / offsets.len() as f64).sqrt();
        assert!(mean.abs() < 0.1, "mean peak offset {mean}");
        assert!((1.3..=1.6).contains(&spread), "spread {spread}");
        let mut resilience: Vec<f64> = drawn.iter().map(|t| t.resilience).collect();
        resilience.sort_by(|a, b| a.total_cmp(b));
        let median = resilience[resilience.len() / 2];
        assert!(
            (0.95..=1.05).contains(&median),
            "median resilience {median}"
        );
    }

    /// Given a 17-year-old with room to grow, averaged over many seasons,
    /// When he develops,
    /// Then he gains close to the curve's expected points.
    #[test]
    fn a_young_player_far_below_his_ceiling_grows_by_about_the_curves_points() {
        let expected = growth_points(17.0, 30);
        let mut gained = 0.0;
        for seed in 0..200 {
            let mut young = player("young", Position::CentralMidfielder, 50, 80);
            young.date_of_birth = "2013-01-01".to_string();
            let before = f64::from(young.ovr);
            let traits = neutral();
            let curve = curve_age(&young, 17, &traits);
            grow(&mut young, growth_points(curve, 30), curve, &mut rng(seed));
            gained += f64::from(natural_ovr(&young).round() as u8) - before;
        }
        let mean = gained / 200.0;
        assert!(
            (mean - expected).abs() < 0.6,
            "mean {mean} vs expected {expected}"
        );
    }

    /// Given a 33-year-old, averaged over many seasons,
    /// When he declines at resilience 1,
    /// Then he loses pace and agility more than strength.
    #[test]
    fn decline_takes_the_fast_attributes_before_the_strong_ones() {
        let mut speed = 0.0;
        let mut strength = 0.0;
        for seed in 0..200 {
            let mut old = player("old", Position::CentralMidfielder, 70, 70);
            decline(&mut old, 33.0, &neutral(), &mut rng(seed));
            speed += f64::from(70 - old.attributes.pace);
            strength += f64::from(70 - old.attributes.strength);
        }
        assert!(
            speed / 200.0 > 2.0 && speed / 200.0 < 3.0,
            "pace lost {}",
            speed / 200.0
        );
        assert!(strength / 200.0 < 1.1, "strength lost {}", strength / 200.0);
    }

    /// Given two players of 33 and a high and a low resilience,
    /// When each declines for many seasons,
    /// Then the resilient one loses less.
    #[test]
    fn resilience_slows_a_decline() {
        let loss = |resilience: f64| {
            let traits = DevelopmentTraits {
                peak_offset: 0.0,
                resilience,
            };
            (0..200)
                .map(|seed| {
                    let mut old = player("old", Position::CentralMidfielder, 70, 70);
                    decline(&mut old, 34.0, &traits, &mut rng(seed));
                    f64::from(70 - old.attributes.pace)
                })
                .sum::<f64>()
        };
        assert!(loss(2.0) < loss(1.0) * 0.7);
    }

    /// Given a goalkeeper and an outfield player of the same age and traits,
    /// When each declines,
    /// Then the keeper loses about half as much pace.
    #[test]
    fn a_goalkeepers_body_declines_at_half_the_pace() {
        let pace_lost = |position: Position| {
            (0..300)
                .map(|seed| {
                    let mut old = player("old", position.clone(), 70, 70);
                    decline(&mut old, 34.0, &neutral(), &mut rng(seed));
                    f64::from(70 - old.attributes.pace)
                })
                .sum::<f64>()
        };
        let ratio = pace_lost(Position::Goalkeeper) / pace_lost(Position::CentralMidfielder);
        assert!((0.4..=0.6).contains(&ratio), "keeper/outfield {ratio}");
    }

    /// Given the same player, seed and stream,
    /// When he develops twice,
    /// Then the two seasons are identical: a replay gives the same year.
    #[test]
    fn a_season_replays_the_same() {
        let season = || {
            let mut p = player("replay", Position::CentralMidfielder, 55, 75);
            develop_one_season(&mut p, 19, 5, &mut rng(9));
            (p.attributes.clone(), p.potential)
        };
        assert_eq!(season(), season());
    }

    /// Given a player at his ceiling,
    /// When he develops at 20,
    /// Then he does not grow past the point where his ceiling meets his rating: the brake leaves
    ///      only the small ceiling drift.
    #[test]
    fn a_player_at_his_ceiling_gains_nothing_from_growth() {
        let mut gained = 0.0;
        for seed in 0..100 {
            let mut p = player("peak", Position::CentralMidfielder, 70, 70);
            let before = natural_ovr(&p).round();
            grow(&mut p, growth_points(20.0, 0), 20.0, &mut rng(seed));
            gained += natural_ovr(&p).round() - before;
        }
        assert_eq!(gained, 0.0);
    }

    /// Given many teenagers' ceilings drifting for a season,
    /// When the average ceiling is read,
    /// Then it falls slightly (the pull down), and never below a player's rating.
    #[test]
    fn the_ceiling_drifts_down_on_average_and_never_below_the_rating() {
        let mut total_change = 0.0;
        for seed in 0..400 {
            let mut p = player("drift", Position::CentralMidfielder, 55, 70);
            let rating = natural_ovr(&p).round() as u8;
            drift_ceiling(&mut p, 17, &mut rng(seed));
            assert!(p.potential >= rating);
            total_change += f64::from(p.potential) - 70.0;
        }
        assert!(total_change / 400.0 < 0.0);
        assert!(total_change / 400.0 > -1.5);
    }

    fn ceilings_after_the_drift(
        start: impl Fn(&mut ChaCha12Rng) -> f64,
        from_age: u32,
    ) -> Vec<f64> {
        let mut dealer = rng(7);
        let mut cohort: Vec<Player> = (0..2000)
            .map(|n| {
                let ceiling = start(&mut dealer).round() as u8;
                player(
                    &format!("bell{n}"),
                    Position::CentralMidfielder,
                    40,
                    ceiling,
                )
            })
            .collect();
        for age in from_age..=LAST_DRIFT_AGE {
            for (n, p) in cohort.iter_mut().enumerate() {
                drift_ceiling(p, age, &mut rng(u64::from(age) * 10_000 + n as u64));
            }
        }
        cohort.iter().map(|p| f64::from(p.potential)).collect()
    }

    fn mean_and_spread(values: &[f64]) -> (f64, f64) {
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
        (mean, variance.sqrt())
    }

    /// Given sixteen-year-olds whose ceiling is 90,
    /// When their ceilings drift until the drift stops,
    /// Then they keep most of it: a scout's report of a teenager's potential means something.
    #[test]
    fn a_high_ceiling_at_sixteen_is_still_high_at_twenty_five() {
        let kept = ceilings_after_the_drift(|_| 90.0, 16);
        let (mean, _) = mean_and_spread(&kept);
        assert!(mean > 86.0, "mean ceiling at 25 was {mean}");
        let still_s = kept.iter().filter(|c| **c >= 81.0).count() as f64 / kept.len() as f64;
        assert!(still_s > 0.95, "{still_s} of them are still S");
    }

    /// Given a bell built directly: narrower by the variance `ceiling_drift_still_to_come` reports
    /// for sixteen (and centred by its mean) -- the arithmetic `shaped_ceiling` relies on, not
    /// `shaped_ceiling` itself, with ratings far below the ceilings so the rating floor stays out,
    /// When `drift_ceiling` runs on every ceiling until the drift stops,
    /// Then they land on the target bell: the centre and a spread of 5.
    #[test]
    fn the_drift_arithmetic_lands_a_narrowed_start_on_the_target_bell() {
        let (drift_mean, drift_variance) = ceiling_drift_still_to_come(16);
        let centre = 69.6 - drift_mean;
        let spread = (25.0_f64 - drift_variance).sqrt();
        let landed =
            ceilings_after_the_drift(|dealer| centre + spread * standard_normal(dealer), 16);
        let (mean, width) = mean_and_spread(&landed);
        assert!((mean - 69.6).abs() < 0.5, "mean {mean}");
        assert!((width - 5.0).abs() < 0.4, "spread {width}");
    }

    /// Given sixteen-year-old keepers and midfielders with the same ceiling and a rating far below it,
    /// When a season of development is run for each of nine years,
    /// Then their ceilings end at the same spread and the same level: a keeper's later curve does
    ///      not give him more years of the widest drift than the generator allowed for.
    #[test]
    fn a_keepers_ceiling_drifts_as_much_as_an_outfield_players() {
        // Players whose own curve is not an early bloomer's, so none of them is declining by 24
        // (the ceiling rule would then pull his ceiling down to his rating, which this is not
        // about): the same ids for both positions.
        let ids: Vec<String> = (0..4000)
            .map(|n| format!("drifter{n}"))
            .filter(|id| traits_of(9, id).peak_offset.abs() <= 1.5)
            .take(1500)
            .collect();
        let spread_at_25 = |position: Position| {
            let mut cohort: Vec<Player> = ids
                .iter()
                .map(|id| player(id, position.clone(), 30, 80))
                .collect();
            for (year, age) in (16..=24).enumerate() {
                for (n, p) in cohort.iter_mut().enumerate() {
                    let mut stream = rng((year * 10_000 + n) as u64);
                    develop_one_season(p, age, 9, &mut stream);
                }
            }
            let ceilings: Vec<f64> = cohort.iter().map(|p| f64::from(p.potential)).collect();
            mean_and_spread(&ceilings)
        };
        let (keeper_mean, keeper_spread) = spread_at_25(Position::Goalkeeper);
        let (field_mean, field_spread) = spread_at_25(Position::CentralMidfielder);
        assert!(
            (keeper_spread - field_spread).abs() < 0.2,
            "keepers {keeper_spread:.2}, outfield {field_spread:.2}"
        );
        assert!(
            (keeper_mean - field_mean).abs() < 0.4,
            "keepers {keeper_mean:.2}, outfield {field_mean:.2}"
        );
    }

    /// An id whose peak offset lies in `offsets`, so a player of a known age has a known curve age.
    fn id_with_peak_offset(offsets: std::ops::RangeInclusive<f64>) -> String {
        (0..5000)
            .map(|n| format!("vet{n}"))
            .find(|id| offsets.contains(&traits_of(5, id).peak_offset))
            .expect("some player has a peak offset in the range")
    }

    /// Given players of 31 to 39 (their curve past 30), each with a ceiling far above his rating,
    /// When a season of development is run,
    /// Then the ceiling sits exactly at the rating afterwards: a falling player never shows
    ///      room to grow.
    #[test]
    fn a_declining_players_ceiling_comes_down_to_his_rating() {
        let id = id_with_peak_offset(-0.4..=0.4);
        for age in 31..=39 {
            for seed in 0..40 {
                let mut p = player(&id, Position::CentralMidfielder, 70, 90);
                develop_one_season(&mut p, age, 5, &mut rng(seed));
                assert_eq!(
                    p.potential,
                    natural_ovr(&p).round() as u8,
                    "age {age}, seed {seed}: rating {}",
                    natural_ovr(&p)
                );
            }
        }
    }

    /// Given a 22-year-old with a high ceiling and room to grow,
    /// When a season of development is run,
    /// Then his ceiling is only the drift of a young player's: it keeps most of its height and he
    ///      still has room to grow.
    #[test]
    fn a_twenty_two_year_olds_ceiling_is_not_touched_by_the_rule() {
        let id = id_with_peak_offset(-0.4..=0.4);
        for seed in 0..40 {
            let mut p = player(&id, Position::CentralMidfielder, 60, 85);
            develop_one_season(&mut p, 22, 5, &mut rng(seed));
            assert!(p.potential >= 82, "ceiling {}", p.potential);
            assert!(p.potential > natural_ovr(&p).round() as u8 + 10);
        }
    }

    /// Given a 28-year-old (growth ended on his curve, the table not yet taking rating) with a gap
    ///       of 10 to his ceiling,
    /// When a season of development is run,
    /// Then he keeps the room: the ceiling follows the rating only from curve 30.
    #[test]
    fn a_twenty_eight_year_old_keeps_the_room_he_has() {
        let id = id_with_peak_offset(-0.4..=0.4);
        for seed in 0..40 {
            let mut p = player(&id, Position::CentralMidfielder, 70, 80);
            develop_one_season(&mut p, 28, 5, &mut rng(seed));
            assert!(
                p.potential > natural_ovr(&p).round() as u8 + 5,
                "seed {seed}: ceiling {}",
                p.potential
            );
        }
    }

    /// Given a player whose curve reads exactly 30 (thirty-three, a late bloomer clamped to +3
    ///       years) and one whose curve reads 29,
    /// When a season of development is run,
    /// Then the first has his ceiling brought down to his rating and the second keeps his room.
    #[test]
    fn the_ceiling_follows_the_rating_from_a_curve_of_exactly_thirty() {
        let id = (0..2000)
            .map(|n| format!("late{n}"))
            .find(|id| traits_of(5, id).peak_offset == PEAK_OFFSET_LIMIT)
            .expect("about one player in forty is clamped at the late end");
        for seed in 0..20 {
            let mut thirty = player(&id, Position::CentralMidfielder, 70, 85);
            develop_one_season(&mut thirty, 33, 5, &mut rng(seed));
            assert_eq!(thirty.potential, natural_ovr(&thirty).round() as u8);

            let mut twenty_nine = player(&id, Position::CentralMidfielder, 70, 85);
            develop_one_season(&mut twenty_nine, 32, 5, &mut rng(seed));
            assert!(twenty_nine.potential > natural_ovr(&twenty_nine).round() as u8 + 5);
        }
    }

    /// Given a late bloomer of 29 whose curve still reads 26 and who has room to grow,
    /// When a season of development is run,
    /// Then he is still growing and keeps his ceiling: the rule follows the curve, not the birthday.
    #[test]
    fn a_late_bloomer_keeps_his_ceiling_while_his_curve_is_still_growing() {
        let id = id_with_peak_offset(2.9..=3.0);
        let mut gained = 0.0;
        for seed in 0..40 {
            let mut p = player(&id, Position::CentralMidfielder, 70, 80);
            let before = natural_ovr(&p);
            develop_one_season(&mut p, 29, 5, &mut rng(seed));
            assert!(p.potential > natural_ovr(&p).round() as u8, "seed {seed}");
            gained += natural_ovr(&p) - before;
        }
        assert!(gained > 0.0, "he grew by {gained} over 40 seasons");
    }

    /// Given players of each age,
    /// When the drift they still have to come is read,
    /// Then it shrinks with age and is nothing once the drift has stopped.
    #[test]
    fn the_drift_still_to_come_shrinks_to_nothing() {
        let mut previous = f64::MAX;
        for age in 14..=30 {
            let (_, variance) = ceiling_drift_still_to_come(age);
            assert!(variance <= previous + 1e-12, "age {age}");
            previous = variance;
        }
        assert_eq!(ceiling_drift_still_to_come(25), (0.0, 0.0));
        assert_eq!(
            ceiling_drift_still_to_come(10),
            ceiling_drift_still_to_come(16)
        );
    }

    /// Given the decline table,
    /// When every group is read in every band,
    /// Then each cell is the table's: nothing rises but the mind before 30, and from 30 every
    ///      group falls, faster with age.
    #[test]
    fn every_cell_of_the_decline_table_is_what_the_research_says() {
        use Aspect::{Mental, Speed, Stamina, Strength, Technical};
        let cells = [
            (28.0, [-0.5, 0.0, 0.0, 0.0, 0.5]),
            (30.0, [-2.0, -0.7, -0.3, -0.8, 0.3]),
            (31.9, [-2.0, -0.7, -0.3, -0.8, 0.3]),
            (32.0, [-2.5, -1.2, -0.8, -1.5, 0.0]),
            (33.9, [-2.5, -1.2, -0.8, -1.5, 0.0]),
            (34.0, [-3.0, -2.0, -1.2, -2.2, -0.5]),
            (35.9, [-3.0, -2.0, -1.2, -2.2, -0.5]),
            (36.0, [-4.0, -2.5, -1.5, -3.0, -1.0]),
            (41.0, [-4.0, -2.5, -1.5, -3.0, -1.0]),
        ];
        for (age, expected) in cells {
            let read = [Speed, Stamina, Strength, Technical, Mental]
                .map(|aspect| decline_per_season(age, aspect));
            assert_eq!(read, expected, "at {age}");
        }
    }

    /// Given the growth shares,
    /// When they are read at each age band,
    /// Then they are physical first, technical in the early twenties, mental last, and add to 1.
    #[test]
    fn growth_is_shared_between_the_groups_by_age() {
        for (age, expected) in [
            (14.0, (0.45, 0.35, 0.20)),
            (17.9, (0.45, 0.35, 0.20)),
            (18.0, (0.25, 0.45, 0.30)),
            (21.9, (0.25, 0.45, 0.30)),
            (22.0, (0.10, 0.40, 0.50)),
            (25.9, (0.10, 0.40, 0.50)),
            (26.0, (0.0, 0.30, 0.70)),
        ] {
            let shares = growth_shares(age);
            assert_eq!(shares, expected, "at {age}");
            assert!((shares.0 + shares.1 + shares.2 - 1.0).abs() < 1e-9);
        }
    }

    /// Given the season's growth at a curve age and a gap to the ceiling,
    /// When the points are read,
    /// Then they are the table's value times the brake: a gap of six halves them.
    #[test]
    fn a_seasons_growth_is_the_table_times_the_brake() {
        assert_eq!(growth_points(21.0, 6), 2.0);
        assert!((growth_points(17.0, 30) - 6.0 * 30.0 / 36.0).abs() < 1e-9);
    }

    /// The average change of each group of attributes after one `grow`, over many seasons, for a
    /// player with every attribute at 50 and the ceiling far above.
    fn mean_group_changes(curve_age: f64, points: f64) -> (f64, f64, f64) {
        let (mut physical, mut technical, mut mental) = (0.0, 0.0, 0.0);
        let seasons = 400;
        for seed in 0..seasons {
            let mut p = player("grower", Position::CentralMidfielder, 50, 99);
            grow(&mut p, points, curve_age, &mut rng(seed));
            let a = &p.attributes;
            physical += f64::from(a.pace)
                + f64::from(a.agility)
                + f64::from(a.stamina)
                + f64::from(a.strength);
            technical += [
                a.passing,
                a.shooting,
                a.tackling,
                a.dribbling,
                a.defending,
                a.handling,
                a.reflexes,
                a.aerial,
            ]
            .iter()
            .map(|v| f64::from(*v))
            .sum::<f64>();
            mental += [
                a.positioning,
                a.vision,
                a.decisions,
                a.composure,
                a.aggression,
                a.teamwork,
                a.leadership,
            ]
            .iter()
            .map(|v| f64::from(*v))
            .sum::<f64>();
        }
        let n = f64::from(seasons as u32);
        (
            physical / n / 4.0 - 50.0,
            technical / n / 8.0 - 50.0,
            mental / n / 7.0 - 50.0,
        )
    }

    /// Given a teenager, a twenty-year-old, a player of 23 and one of 26 growing from a flat profile,
    /// When many seasons of growth are averaged by group,
    /// Then the groups differ by the shares per attribute (the rating is pinned afterwards, so only
    ///      the differences between the groups are the shares'): the body first, the mind last.
    #[test]
    fn growth_goes_to_the_groups_the_age_gives_it_to() {
        // points * 19 * (share / attributes in the group), for 4 points.
        let per = |share: f64, count: f64| 4.0 * 19.0 * share / count;
        for (age, shares) in [
            (16.0, (0.45, 0.35, 0.20)),
            (19.0, (0.25, 0.45, 0.30)),
            (23.0, (0.10, 0.40, 0.50)),
            (26.0, (0.0, 0.30, 0.70)),
        ] {
            let (physical, technical, mental) = mean_group_changes(age, 4.0);
            let expected_physical_over_mental = per(shares.0, 4.0) - per(shares.2, 7.0);
            let expected_technical_over_mental = per(shares.1, 8.0) - per(shares.2, 7.0);
            assert!(
                (physical - mental - expected_physical_over_mental).abs() < 0.6,
                "at {age}: physical over mental {:.2}, expected {expected_physical_over_mental:.2}",
                physical - mental
            );
            assert!(
                (technical - mental - expected_technical_over_mental).abs() < 0.6,
                "at {age}: technical over mental {:.2}, expected {expected_technical_over_mental:.2}",
                technical - mental
            );
        }
    }

    /// Given a teenager growing from a flat profile,
    /// When one attribute's change is compared across seasons,
    /// Then each attribute has its own luck as a multiplier: the body's changes vary by a few points.
    #[test]
    fn each_attributes_growth_has_its_own_luck_as_a_multiplier() {
        let mut changes = Vec::new();
        for seed in 0..400 {
            let mut p = player("grower", Position::CentralMidfielder, 50, 99);
            grow(&mut p, 4.0, 16.0, &mut rng(seed));
            changes.push(f64::from(p.attributes.pace));
        }
        let (_, spread) = mean_and_spread(&changes);
        // 4 points * 19 * 0.45 / 4 = 8.6 on average, times a luck of 0.5 to 1.5 (sd 0.29).
        assert!((1.8..3.2).contains(&spread), "spread {spread}");
    }

    /// Given a 33-year-old outfield player with neutral traits,
    /// When many seasons of decline are averaged,
    /// Then pace falls by the table's 2.5 a season: the luck is a multiplier with a mean of 1.
    #[test]
    fn the_average_decline_is_the_tables_not_a_steeper_one() {
        let mut total = 0.0;
        let seasons = 3000;
        for seed in 0..seasons {
            let mut p = player("old", Position::CentralMidfielder, 70, 70);
            let before = f64::from(p.attributes.pace);
            decline(&mut p, 33.0, &neutral(), &mut rng(seed));
            total += before - f64::from(p.attributes.pace);
        }
        let mean = total / f64::from(seasons as u32);
        assert!((mean - 2.5).abs() < 0.1, "mean decline {mean}");
    }

    /// Given a 34-year-old goalkeeper with neutral traits,
    /// When many seasons of decline are averaged,
    /// Then his body falls at half the rate (pace -1.5) and his keeping attributes at the full
    ///      technical rate (handling -2.2).
    #[test]
    fn only_a_goalkeepers_body_declines_at_half_the_pace() {
        let (mut pace, mut handling) = (0.0, 0.0);
        let seasons = 3000;
        for seed in 0..seasons {
            let mut p = player("keeper", Position::Goalkeeper, 70, 70);
            let (pace_before, handling_before) = (
                f64::from(p.attributes.pace),
                f64::from(p.attributes.handling),
            );
            decline(&mut p, 34.0, &neutral(), &mut rng(seed));
            pace += pace_before - f64::from(p.attributes.pace);
            handling += handling_before - f64::from(p.attributes.handling);
        }
        let n = f64::from(seasons as u32);
        assert!((pace / n - 1.5).abs() < 0.1, "pace {}", pace / n);
        assert!(
            (handling / n - 2.2).abs() < 0.1,
            "handling {}",
            handling / n
        );
    }

    /// Given the traits of five thousand players,
    /// When the spread of the resilience is read,
    /// Then its logarithm has the spread the model asks for (about 0.35, trimmed a little by the
    ///      range), and few players sit at the range's ends.
    #[test]
    fn resilience_varies_by_about_a_third_in_logarithm() {
        let resilience: Vec<f64> = (0..5000)
            .map(|n| traits_of(42, &format!("p{n}")).resilience)
            .collect();
        let logs: Vec<f64> = resilience.iter().map(|r| r.ln()).collect();
        let (_, spread) = mean_and_spread(&logs);
        assert!((0.30..0.37).contains(&spread), "spread {spread}");
        let at_an_end = resilience
            .iter()
            .filter(|r| **r <= RESILIENCE_RANGE.0 || **r >= RESILIENCE_RANGE.1)
            .count();
        assert!(at_an_end < 250, "{at_an_end} of 5000 sit at an end");
    }

    /// Given ceilings at 16, 20 and 23 and none from 25,
    /// When one season's step is read over many players,
    /// Then its spread is the band's (1.0, 0.7, 0.35, a little more for the rounding) and nothing
    ///      moves at 25.
    #[test]
    fn a_ceilings_step_has_the_spread_of_its_age() {
        for (age, spread) in [(16, 1.0_f64), (20, 0.7), (23, 0.35), (25, 0.0)] {
            let steps: Vec<f64> = (0..3000)
                .map(|seed| {
                    let mut p = player("drift", Position::CentralMidfielder, 40, 70);
                    drift_ceiling(&mut p, age, &mut rng(seed));
                    f64::from(p.potential) - 70.0
                })
                .collect();
            let (_, read) = mean_and_spread(&steps);
            let expected = if spread == 0.0 {
                0.0
            } else {
                (spread * spread + 1.0 / 12.0).sqrt()
            };
            assert!(
                (read - expected).abs() < 0.12,
                "at {age}: {read} vs {expected}"
            );
        }
    }

    /// Given a player whose curve age reads exactly 27 (thirty, a late bloomer clamped to +3 years),
    /// When he develops over many seasons,
    /// Then he is on the decline table, not stuck on growth: his mind still gains a little and
    ///      his pace falls.
    #[test]
    fn a_curve_age_of_exactly_twenty_seven_is_on_the_decline_table() {
        let id = (0..2000)
            .map(|n| format!("late{n}"))
            .find(|id| traits_of(5, id).peak_offset == PEAK_OFFSET_LIMIT)
            .expect("about one player in forty is clamped at the late end");
        let (mut pace, mut vision) = (0.0, 0.0);
        let seasons = 600;
        for seed in 0..seasons {
            let mut p = player(&id, Position::CentralMidfielder, 70, 70);
            let (pace_before, vision_before) =
                (f64::from(p.attributes.pace), f64::from(p.attributes.vision));
            develop_one_season(&mut p, 30, 5, &mut rng(seed));
            pace += pace_before - f64::from(p.attributes.pace);
            vision += f64::from(p.attributes.vision) - vision_before;
        }
        let n = f64::from(seasons as u32);
        assert!(pace / n > 0.15, "pace fell by {} a season", pace / n);
        assert!(vision / n > 0.15, "vision rose by {} a season", vision / n);
    }

    /// Given players from nineteen to forty with the same room to grow,
    /// When their training factor is read,
    /// Then it never rises with age, is nothing from the mid-thirties and is never negative.
    #[test]
    fn training_takes_less_with_age_and_fades_out_without_a_step() {
        let mut previous = f64::MAX;
        for tenths in 190..=400 {
            let factor = training_factor(f64::from(tenths) / 10.0, 20);
            assert!(
                factor >= 0.0 && factor <= previous + 1e-12,
                "age {}",
                f64::from(tenths) / 10.0
            );
            previous = factor;
        }
        assert_eq!(training_factor(34.0, 20), 0.0);
        assert!(training_factor(30.0, 20) > 0.0);
        assert!(training_factor(22.0, 20) > training_factor(28.0, 20));
    }

    /// Given a player at his ceiling and one a point under it,
    /// When their training factor is read,
    /// Then the first takes none and the second a little: the brake, not a gate.
    #[test]
    fn training_slows_toward_the_ceiling_without_a_gate() {
        assert_eq!(training_factor(20.0, 0), 0.0);
        assert!(training_factor(20.0, 1) > 0.0);
        assert!(training_factor(20.0, 1) < training_factor(20.0, 10));
    }

    /// Given a 17-year-old two points under his ceiling and a large growth to give him,
    /// When the growth is applied,
    /// Then he reaches the ceiling and does not pass it, however the rounding falls.
    #[test]
    fn a_seasons_growth_closes_the_gap_to_the_ceiling_and_stops_there() {
        for seed in 0..100 {
            let mut young = player("capped", Position::CentralMidfielder, 60, 62);
            let ceiling = young.potential;
            grow(&mut young, 6.0, 17.0, &mut rng(seed));
            assert!(
                natural_ovr(&young).round() as u8 <= ceiling,
                "seed {seed}: {}",
                natural_ovr(&young)
            );
        }
    }
}
