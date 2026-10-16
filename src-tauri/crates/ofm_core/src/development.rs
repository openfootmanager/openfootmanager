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
//! The numbers are those of the research behind the model
//! (`~/.claude/orchestrate/drafts/player-development-research.md`), to be tuned against the
//! season harness, not by hand.

use domain::player::{Player, PlayerAttributes, Position};
use rand::{Rng, RngExt};

use crate::player_rating::{move_attributes_to, natural_ovr};
use crate::seed::rng_for_seed;

/// Growth per season, in rating points, of a player far below his ceiling, for curve ages 16 to
/// 26. Below 16 it is the first value; at 27 it is gone and the decline table governs.
const GROWTH_BY_AGE: [f64; 11] = [6.0, 6.0, 5.5, 5.0, 4.5, 4.0, 3.5, 3.0, 2.5, 2.0, 1.2];
const FIRST_GROWTH_AGE: f64 = 16.0;
/// The curve age from which the decline table replaces growth.
const DECLINE_STARTS_AT: f64 = 27.0;
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

/// What the ceiling does each season until the player's peak: a random step of this size by age,
/// plus a small pull down so the world does not fill with ever-higher ceilings.
const CEILING_PULL: f64 = -0.3;

/// The five groups the decline table is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aspect {
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

/// A standard normal draw (Box-Muller): this crate carries no distributions library.
fn standard_normal(rng: &mut impl Rng) -> f64 {
    let first: f64 = 1.0 - rng.random_range(0.0..1.0);
    let second: f64 = rng.random_range(0.0..1.0);
    (-2.0 * first.ln()).sqrt() * (std::f64::consts::TAU * second).cos()
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
        Position::CenterBack => 1.0,
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
pub fn growth_points(curve_age: f64, gap: u8) -> f64 {
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
    age_factor * headroom_brake(gap)
}

/// The expected change in points of one attribute in a season, for a player the decline table
/// governs (curve age 27 or more), at resilience 1.
pub fn decline_per_season(curve_age: f64, aspect: Aspect) -> f64 {
    let band = match curve_age.floor() as i32 {
        ..=26 => return 0.0,
        27..=29 => [-0.5, 0.0, 0.0, 0.0, 0.5],
        30..=31 => [-2.0, -0.7, -0.3, -0.5, 0.3],
        32..=33 => [-2.5, -1.2, -0.8, -1.0, 0.0],
        34..=35 => [-3.0, -2.0, -1.2, -1.5, -0.5],
        _ => [-4.0, -2.5, -1.5, -2.0, -1.0],
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
    let target = i32::from(player.ovr) + stochastic_round(points, rng);
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

/// Moves the ceiling for the season: a random step that is large for a teenager and gone by 25,
/// with a small pull down. Never below the rating he has.
fn drift_ceiling(player: &mut Player, curve_age: f64, rng: &mut impl Rng) {
    let spread = match curve_age.floor() as i32 {
        ..=18 => 3.0,
        19..=21 => 2.0,
        22..=24 => 1.0,
        _ => 0.0,
    };
    if spread == 0.0 {
        return;
    }
    let step = standard_normal(rng) * spread + CEILING_PULL;
    let ceiling = (f64::from(player.potential) + step)
        .round()
        .clamp(1.0, 99.0) as u8;
    player.potential = ceiling.max(natural_ovr(player).round() as u8);
}

/// One season of development for a player aged `age`: growth while his curve says he grows, the
/// decline table once it says he declines, and the drift of his ceiling on the way. Draws from
/// `rng`, which the caller derives from the player and the date so a replay gives the same season.
pub fn develop_one_season(player: &mut Player, age: i32, world_seed: u64, rng: &mut impl Rng) {
    let traits = traits_of(world_seed, &player.id);
    let curve = curve_age(player, age, &traits);
    if curve < DECLINE_STARTS_AT {
        let gap = player
            .potential
            .saturating_sub(natural_ovr(player).round() as u8);
        grow(player, growth_points(curve, gap), curve, rng);
    } else {
        decline(player, curve, &traits, rng);
    }
    drift_ceiling(player, curve, rng);
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
        assert_eq!(decline_per_season(34.0, Aspect::Technical), -1.5);
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
            let mut p = player("drift", Position::CentralMidfielder, 55, 75);
            let rating = natural_ovr(&p).round() as u8;
            drift_ceiling(&mut p, 17.0, &mut rng(seed));
            assert!(p.potential >= rating);
            total_change += f64::from(p.potential) - 75.0;
        }
        assert!(total_change / 400.0 < 0.0);
        assert!(total_change / 400.0 > -1.5);
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
}
