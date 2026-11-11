//! How a generated staff member's attributes are drawn. The attributes his role's rating reads
//! ([`crate::staff_rating::staff_rating`]) follow a normal bell, as players' ratings do, so a
//! great coach is as rare among coaches as a great player is among players. Each bell keeps the
//! mean of the even range it replaces, because training, recovery, scouting and delegated talks
//! run on those attributes and the game is balanced on their averages. The attributes a role's
//! rating does not read are still drawn evenly from their old ranges.

use domain::staff::{StaffAttributes, StaffRole};
use rand::{Rng, RngExt};

use crate::seed::standard_normal_at;
use crate::staff_rating::ASSISTANT_WEIGHTS;

/// The spread every role's rating is drawn with: about a shipped world's senior players' (6.2 to
/// 6.4 on three seeds), so a rating tier is as wide in rating points for staff as for players.
const RATING_SD: f64 = 6.5;

/// How one attribute is drawn.
#[derive(Debug, Clone, Copy)]
enum Draw {
    /// Evenly from `low` up to, not including, `high`.
    Even { low: u8, high: u8 },
    /// From a normal bell, rounded and held to 1 to 99 as a player's ceiling is.
    ///
    /// The bell's z comes from one 32-bit draw through the normal's inverse distribution, the
    /// number an even draw of a `u8` takes, so the world drawn after the staff (players, clubs,
    /// ids) is the one it was before the bell.
    Bell { mean: f64, sd: f64 },
}

impl Draw {
    fn roll(self, rng: &mut impl Rng) -> u8 {
        match self {
            Draw::Even { low, high } => rng.random_range(low..high),
            Draw::Bell { mean, sd } => {
                // Half a step in, so the quantile is never 0 or 1.
                let quantile = (f64::from(rng.random::<u32>()) + 0.5) / 4_294_967_296.0;
                (mean + sd * standard_normal_at(quantile))
                    .round()
                    .clamp(1.0, 99.0) as u8
            }
        }
    }

    /// The bell with this even range's mean and `sd`.
    fn as_bell(self, sd: f64) -> Draw {
        match self {
            Draw::Even { low, high } => Draw::Bell {
                mean: (f64::from(low) + f64::from(high) - 1.0) / 2.0,
                sd,
            },
            bell @ Draw::Bell { .. } => bell,
        }
    }
}

/// How each of a staff member's four attributes is drawn.
#[derive(Debug, Clone, Copy)]
struct AttributeDraws {
    coaching: Draw,
    judging_ability: Draw,
    judging_potential: Draw,
    physiotherapy: Draw,
}

impl AttributeDraws {
    fn even(coaching: (u8, u8), ability: (u8, u8), potential: (u8, u8), physio: (u8, u8)) -> Self {
        let even = |(low, high)| Draw::Even { low, high };
        Self {
            coaching: even(coaching),
            judging_ability: even(ability),
            judging_potential: even(potential),
            physiotherapy: even(physio),
        }
    }

    /// These draws with the attributes `role`'s rating reads turned into bells, each wide enough
    /// that the rating has [`RATING_SD`].
    fn with_rated_attributes_on_a_bell(self, role: &StaffRole) -> Self {
        match role {
            StaffRole::Coach => Self {
                coaching: self.coaching.as_bell(RATING_SD),
                ..self
            },
            StaffRole::Physio => Self {
                physiotherapy: self.physiotherapy.as_bell(RATING_SD),
                ..self
            },
            // The rating is the mean of the two judging attributes, which narrows a bell by √2.
            StaffRole::Scout => {
                let sd = RATING_SD * std::f64::consts::SQRT_2;
                Self {
                    judging_ability: self.judging_ability.as_bell(sd),
                    judging_potential: self.judging_potential.as_bell(sd),
                    ..self
                }
            }
            // The rating is a weighted mean of three attributes, which narrows a bell by the
            // weights' root sum of squares over their sum.
            StaffRole::AssistantManager => {
                let total: f64 = ASSISTANT_WEIGHTS
                    .iter()
                    .map(|weight| f64::from(*weight))
                    .sum();
                let root_sum_of_squares = ASSISTANT_WEIGHTS
                    .iter()
                    .map(|weight| f64::from(*weight).powi(2))
                    .sum::<f64>()
                    .sqrt();
                let sd = RATING_SD * total / root_sum_of_squares;
                Self {
                    coaching: self.coaching.as_bell(sd),
                    judging_ability: self.judging_ability.as_bell(sd),
                    judging_potential: self.judging_potential.as_bell(sd),
                    ..self
                }
            }
        }
    }

    fn roll(self, rng: &mut impl Rng) -> StaffAttributes {
        StaffAttributes {
            coaching: self.coaching.roll(rng),
            judging_ability: self.judging_ability.roll(rng),
            judging_potential: self.judging_potential.roll(rng),
            physiotherapy: self.physiotherapy.roll(rng),
        }
    }
}

/// The attributes of a staff member generated for a club in `role`.
pub(super) fn club_staff_attributes(role: &StaffRole, rng: &mut impl Rng) -> StaffAttributes {
    let ranges = match role {
        StaffRole::AssistantManager => AttributeDraws::even((50, 90), (50, 85), (40, 80), (20, 50)),
        StaffRole::Coach => AttributeDraws::even((55, 95), (40, 75), (30, 70), (20, 45)),
        StaffRole::Scout => AttributeDraws::even((20, 50), (60, 95), (55, 95), (10, 30)),
        StaffRole::Physio => AttributeDraws::even((10, 40), (20, 50), (15, 45), (60, 95)),
    };
    ranges.with_rated_attributes_on_a_bell(role).roll(rng)
}

/// The attributes of a free agent generated for the staff market in `role`: a weaker pool than the
/// clubs', its rated attributes centred where the market's even ranges were.
pub(super) fn free_agent_staff_attributes(role: &StaffRole, rng: &mut impl Rng) -> StaffAttributes {
    AttributeDraws::even((30, 80), (30, 80), (25, 75), (25, 75))
        .with_rated_attributes_on_a_bell(role)
        .roll(rng)
}

#[cfg(test)]
mod tests {
    use super::super::definitions::default_names_definition;
    use super::super::generation::{
        generate_random_staff_from_def, generate_random_staff_unattached_from_def,
    };
    use super::*;
    use crate::player_tier::PlayerTier;
    use crate::seed::normal_cdf;
    use crate::staff_rating::staff_rating;
    use crate::staff_tier::staff_tier;
    use domain::staff::Staff;
    use rand::SeedableRng;

    const PER_ROLE: usize = 40_000;

    /// Which of the game's two generators a sample comes from.
    #[derive(Debug, Clone, Copy)]
    enum Pool {
        Club,
        FreeAgent,
    }

    /// `PER_ROLE` staff of `role` from the world generator's own function for `pool`.
    fn generated(pool: Pool, role: &StaffRole, seed: u64) -> Vec<Staff> {
        let names_def = default_names_definition();
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        (0..PER_ROLE)
            .map(|_| match pool {
                Pool::Club => generate_random_staff_from_def(
                    "club-id",
                    role.clone(),
                    "ENG",
                    2026,
                    &names_def,
                    &mut rng,
                ),
                Pool::FreeAgent => generate_random_staff_unattached_from_def(
                    role.clone(),
                    "ENG",
                    2026,
                    &names_def,
                    &mut rng,
                ),
            })
            .collect()
    }

    fn ratings(staff: &[Staff]) -> Vec<f64> {
        staff
            .iter()
            .map(|member| f64::from(staff_rating(member)))
            .collect()
    }

    fn mean(values: &[f64]) -> f64 {
        values.iter().sum::<f64>() / values.len() as f64
    }

    fn sd(values: &[f64]) -> f64 {
        let centre = mean(values);
        (values
            .iter()
            .map(|value| (value - centre).powi(2))
            .sum::<f64>()
            / values.len() as f64)
            .sqrt()
    }

    /// Given staff of each role generated for clubs and for the free-agent market
    /// When the share of ratings 1 and 2 standard deviations above the mean is counted
    /// Then each is within 15% of what a normal bell of the same mean and spread has there: a tier
    ///      is a share of the role, and an even draw has no tail at all past 1.73 sd.
    ///
    /// The tiers' cut-offs (the top 11.5% and 2.5%) sit at about 1.2 and 1.96 sd. Past about 2.4
    /// sd a scout's tail thins, since each judging attribute stops at 99 (its bell centres at 77
    /// with a spread of 9.2), so no line is drawn there.
    #[test]
    fn every_roles_rating_has_a_normal_tail() {
        for pool in [Pool::Club, Pool::FreeAgent] {
            for (index, role) in StaffRole::ALL.iter().enumerate() {
                let rated = ratings(&generated(pool, role, 0x57AF_0000 + index as u64));
                let (centre, spread) = (mean(&rated), sd(&rated));
                for z in [1.0, 2.0] {
                    // Ratings are whole numbers: compare the share at or above the first whole
                    // number past the line with the normal's share of that number's interval.
                    let line = (centre + z * spread).ceil();
                    let observed = rated.iter().filter(|rating| **rating >= line).count() as f64
                        / rated.len() as f64;
                    let normal = 1.0 - normal_cdf((line - 0.5 - centre) / spread);
                    let ratio = observed / normal;
                    assert!(
                        (0.85..=1.15).contains(&ratio),
                        "{pool:?} {role:?}, {z} sd above the mean ({centre:.2}, sd {spread:.2}): \
                         {:.3}% of ratings against {:.3}% for a normal bell (ratio {ratio:.2})",
                        observed * 100.0,
                        normal * 100.0
                    );
                }
            }
        }
    }

    /// Given two generators from one seed
    /// When one draws a staff member's attributes on the bell, for each role and each pool, and the
    ///      other draws four even attributes as the generator did before the bell
    /// Then both generators' next number is the same: the bell takes the numbers an even draw took,
    ///      so every player, club and id drawn after the staff is the one it was.
    #[test]
    fn the_bell_takes_as_many_numbers_as_an_even_draw() {
        for (index, role) in StaffRole::ALL.iter().enumerate() {
            for pool in [Pool::Club, Pool::FreeAgent] {
                let seed = 0xD7A0_0000 + index as u64;
                let mut bell = rand::rngs::StdRng::seed_from_u64(seed);
                let mut even = rand::rngs::StdRng::seed_from_u64(seed);
                match pool {
                    Pool::Club => club_staff_attributes(role, &mut bell),
                    Pool::FreeAgent => free_agent_staff_attributes(role, &mut bell),
                };
                for _ in 0..4 {
                    let _ = even.random_range(10u8..95);
                }
                assert_eq!(
                    bell.random::<u64>(),
                    even.random::<u64>(),
                    "{pool:?} {role:?}"
                );
            }
        }
    }

    /// Given staff of each role generated for clubs and for the free-agent market
    /// When their ratings' spread is measured
    /// Then it is a senior player's, 6.5 within 0.25, for every role: a coach's rating no longer
    ///      spreads twice as wide as a scout's.
    #[test]
    fn every_roles_rating_spreads_as_a_players_does() {
        for pool in [Pool::Club, Pool::FreeAgent] {
            for (index, role) in StaffRole::ALL.iter().enumerate() {
                let spread = sd(&ratings(&generated(pool, role, 0x5D00_0000 + index as u64)));
                assert!(
                    (RATING_SD - 0.25..=RATING_SD + 0.25).contains(&spread),
                    "{pool:?} {role:?}: sd {spread:.2}"
                );
            }
        }
    }

    /// Given staff of each role generated for clubs and for the free-agent market
    /// When the mean of each attribute their rating reads is measured
    /// Then it is the mean of the even range the game drew it from before the bell (within 0.3):
    ///      training, recovery, scouting and delegated talks run on these averages.
    #[test]
    fn each_rated_attribute_keeps_the_mean_the_game_was_balanced_on() {
        type Read = fn(&Staff) -> u8;
        let coaching: Read = |staff| staff.attributes.coaching;
        let ability: Read = |staff| staff.attributes.judging_ability;
        let potential: Read = |staff| staff.attributes.judging_potential;
        let physio: Read = |staff| staff.attributes.physiotherapy;
        let expected: [(Pool, StaffRole, &str, Read, f64); 14] = [
            (Pool::Club, StaffRole::Coach, "coaching", coaching, 74.5),
            (Pool::Club, StaffRole::Physio, "physiotherapy", physio, 77.0),
            (
                Pool::Club,
                StaffRole::Scout,
                "judging ability",
                ability,
                77.0,
            ),
            (
                Pool::Club,
                StaffRole::Scout,
                "judging potential",
                potential,
                74.5,
            ),
            (
                Pool::Club,
                StaffRole::AssistantManager,
                "coaching",
                coaching,
                69.5,
            ),
            (
                Pool::Club,
                StaffRole::AssistantManager,
                "judging ability",
                ability,
                67.0,
            ),
            (
                Pool::Club,
                StaffRole::AssistantManager,
                "judging potential",
                potential,
                59.5,
            ),
            (
                Pool::FreeAgent,
                StaffRole::Coach,
                "coaching",
                coaching,
                54.5,
            ),
            (
                Pool::FreeAgent,
                StaffRole::Physio,
                "physiotherapy",
                physio,
                49.5,
            ),
            (
                Pool::FreeAgent,
                StaffRole::Scout,
                "judging ability",
                ability,
                54.5,
            ),
            (
                Pool::FreeAgent,
                StaffRole::Scout,
                "judging potential",
                potential,
                49.5,
            ),
            (
                Pool::FreeAgent,
                StaffRole::AssistantManager,
                "coaching",
                coaching,
                54.5,
            ),
            (
                Pool::FreeAgent,
                StaffRole::AssistantManager,
                "judging ability",
                ability,
                54.5,
            ),
            (
                Pool::FreeAgent,
                StaffRole::AssistantManager,
                "judging potential",
                potential,
                49.5,
            ),
        ];
        for (index, (pool, role, name, read, want)) in expected.into_iter().enumerate() {
            let staff = generated(pool, &role, 0xBA1A_0000 + index as u64);
            let values: Vec<f64> = staff.iter().map(|member| f64::from(read(member))).collect();
            let got = mean(&values);
            assert!(
                (got - want).abs() <= 0.3,
                "{pool:?} {role:?} {name}: mean {got:.2}, want {want}"
            );
        }
    }

    /// Given staff of each role generated for clubs
    /// When each is tiered among his own role (`staff_tier`)
    /// Then S holds about 2.5% of the role (within 0.75 points), A the next 9% (within 1.5) and B
    ///      the next 40% (within 2.5, as one whole rating moves B by about 6 points): the
    ///      maintainer's tiers, which this bell and the cut-offs must keep together.
    #[test]
    fn each_roles_tiers_hold_the_shares_the_maintainer_set() {
        for (index, role) in StaffRole::ALL.iter().enumerate() {
            let staff = generated(Pool::Club, role, 0x71E2_0000 + index as u64);
            let share = |tier: PlayerTier| {
                staff
                    .iter()
                    .filter(|member| staff_tier(role, staff_rating(member)) == tier)
                    .count() as f64
                    / staff.len() as f64
            };
            let (s, a, b) = (
                share(PlayerTier::S),
                share(PlayerTier::A),
                share(PlayerTier::B),
            );
            assert!(
                (0.0175..=0.0325).contains(&s),
                "{role:?}: S {:.2}%",
                s * 100.0
            );
            assert!(
                (0.075..=0.105).contains(&a),
                "{role:?}: A {:.2}%",
                a * 100.0
            );
            assert!(
                (0.375..=0.425).contains(&b),
                "{role:?}: B {:.2}%",
                b * 100.0
            );
        }
    }

    /// Given a coach, a physio, a scout and an assistant generated for clubs and for the market
    /// When the two pools' mean ratings are compared, role by role
    /// Then the market's is at least 10 below the clubs' (the closest, an assistant's, is about
    ///      12.8 apart): a club that loses a member hires from a weaker pool, as before the bell.
    #[test]
    fn a_free_agent_rates_well_below_a_club_member_of_his_role() {
        for (index, role) in StaffRole::ALL.iter().enumerate() {
            let seed = 0xF4EE_0000 + index as u64;
            let at_clubs = mean(&ratings(&generated(Pool::Club, role, seed)));
            let free = mean(&ratings(&generated(Pool::FreeAgent, role, seed + 0x100)));
            assert!(
                at_clubs - free >= 10.0,
                "{role:?}: clubs {at_clubs:.2}, free agents {free:.2}"
            );
        }
    }
}
