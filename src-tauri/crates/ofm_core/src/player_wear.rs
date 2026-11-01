//! Shared post-match physical wear applied to individual players.
//!
//! Used by both club matches and national-team friendlies so that a call-up
//! carries fatigue, fitness sharpening, and injury risk back to the player's
//! club (the same [`Player`] record is mutated in either context).

use domain::player::{Injury, Player};
use rand::{Rng, RngExt};

/// One severity band of match injuries: how often it occurs, how long it keeps a player out, and
/// the (translated) injuries that fall in it.
struct SeverityBand {
    weight: u32,
    days_out: std::ops::RangeInclusive<u32>,
    injuries: &'static [&'static str],
}

/// The severity mix of time-loss injuries in professional football.
///
/// The weights are the UEFA Elite Club Injury Study's injuries per team per season by days of
/// absence: 19 slight (under 3 days), 15 minor (3-7), 14 moderate (8-28) and 7 major (over 28) out
/// of 55 (UEFA's Elite Club Injury Study newsletter, 2000s seasons; the same study reports match
/// injury risk of about 29 per 1000 hours). Days are uniform within a band, the major band running
/// to 90 days to stand for the long tail.
const SEVERITY_BANDS: [SeverityBand; 4] = [
    SeverityBand {
        weight: 19,
        days_out: 1..=2,
        injuries: &[
            "common.injuries.kneeBruise",
            "common.injuries.minorMuscleStrain",
        ],
    },
    SeverityBand {
        weight: 15,
        days_out: 3..=7,
        injuries: &[
            "common.injuries.twistedAnkle",
            "common.injuries.hamstringTightness",
            "common.injuries.calfStrain",
        ],
    },
    SeverityBand {
        weight: 14,
        days_out: 8..=28,
        injuries: &[
            "common.injuries.ankleLigamentSprain",
            "common.injuries.hamstringTear",
        ],
    },
    SeverityBand {
        weight: 7,
        days_out: 29..=90,
        injuries: &[
            "common.injuries.kneeLigamentTear",
            "common.injuries.metatarsalFracture",
        ],
    },
];

/// Rolls one injury: a severity band by its weight, then a name and a duration within it.
///
/// Shared by club matches and national-team friendlies so an injury means the same thing
/// wherever it was picked up.
pub fn roll_injury(rng: &mut impl Rng) -> Injury {
    let total_weight: u32 = SEVERITY_BANDS.iter().map(|band| band.weight).sum();
    let mut pick = rng.random_range(0..total_weight);
    let band = SEVERITY_BANDS
        .iter()
        .find(|band| {
            if pick < band.weight {
                true
            } else {
                pick -= band.weight;
                false
            }
        })
        .unwrap_or(&SEVERITY_BANDS[0]);

    Injury {
        name: band.injuries[rng.random_range(0..band.injuries.len())].to_string(),
        days_remaining: rng.random_range(band.days_out.clone()),
    }
}

/// Deplete a player's short-term condition based on minutes played and stamina,
/// and gradually sharpen match fitness for players with significant minutes.
///
/// A no-op for players who did not feature (`minutes == 0`).
pub fn apply_match_wear(player: &mut Player, minutes: u8, rng: &mut impl Rng) {
    if minutes == 0 {
        return; // Did not play, no wear.
    }

    let minutes_factor = minutes as f64 / 90.0;
    let stamina_factor = player.attributes.stamina as f64 / 100.0;
    let base_depletion = 40.0 * (1.0 - stamina_factor * 0.4);
    let depletion = (base_depletion * minutes_factor) as u8;
    player.condition = player.condition.saturating_sub(depletion);

    // 60+ minutes builds sharpness; probabilistic to keep changes gradual.
    if minutes >= 60 && player.fitness < 100 && rng.random_bool(0.3) {
        player.fitness = player.fitness.saturating_add(1);
    }
}

/// Injury-risk multiplier scaled by match fitness: less fit players are more
/// prone to picking up a knock. The one rule for every injury roll that reads
/// fitness: on international duty, in a club match, and on the training ground.
pub(crate) fn injury_multiplier_from_fitness(fitness: u8) -> f64 {
    if fitness < 30 {
        3.0
    } else if fitness < 50 {
        2.0
    } else if fitness < 70 {
        1.5
    } else if fitness >= 90 {
        0.7
    } else {
        1.0
    }
}

/// Roll for a match-day injury. On a hit, sets the player's injury and returns
/// `true`. Already-injured players are skipped (returns `false`).
pub fn roll_match_injury(player: &mut Player, rng: &mut impl Rng) -> bool {
    if player.injury.is_some() {
        return false;
    }

    // Base match-day risk, slightly higher than the training-ground rate to
    // reflect competitive intensity.
    let base_prob = 1.0_f64 / 40.0;
    let prob = (base_prob * injury_multiplier_from_fitness(player.fitness)).min(1.0);
    if !rng.random_bool(prob) {
        return false;
    }

    player.injury = Some(roll_injury(rng));
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::player::{Player, PlayerAttributes, Position};
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn attrs(stamina: u8) -> PlayerAttributes {
        PlayerAttributes {
            pace: 70,
            stamina,
            strength: 70,
            agility: 70,
            passing: 70,
            shooting: 70,
            tackling: 70,
            dribbling: 70,
            defending: 70,
            positioning: 70,
            vision: 70,
            decisions: 70,
            composure: 70,
            aggression: 70,
            teamwork: 70,
            leadership: 70,
            handling: 50,
            reflexes: 50,
            aerial: 60,
        }
    }

    fn make_player(stamina: u8) -> Player {
        Player::new(
            "p1".to_string(),
            "J. Doe".to_string(),
            "John Doe".to_string(),
            "2000-01-01".to_string(),
            "ENG".to_string(),
            Position::Midfielder,
            attrs(stamina),
        )
    }

    #[test]
    fn apply_match_wear_depletes_condition_by_minutes_and_stamina() {
        let mut player = make_player(100); // stamina 100 -> base depletion 24 over 90'
        player.condition = 100;
        let mut rng = StdRng::seed_from_u64(1);

        apply_match_wear(&mut player, 90, &mut rng);

        assert_eq!(player.condition, 76);
    }

    #[test]
    fn apply_match_wear_is_noop_for_unused_player() {
        let mut player = make_player(70);
        player.condition = 88;
        player.fitness = 90;
        let mut rng = StdRng::seed_from_u64(2);

        apply_match_wear(&mut player, 0, &mut rng);

        assert_eq!(player.condition, 88);
        assert_eq!(player.fitness, 90);
    }

    #[test]
    fn apply_match_wear_never_lowers_fitness() {
        let mut player = make_player(70);
        player.fitness = 80;
        let mut rng = StdRng::seed_from_u64(3);

        apply_match_wear(&mut player, 90, &mut rng);

        assert!(player.fitness >= 80);
    }

    /// Given many rolled injuries
    /// When they are grouped by severity band (days out)
    /// Then the shares follow the UEFA Elite Club Injury Study's 19/15/14/7 per 55 injuries:
    ///      slight (under 3 days) 34.5%, minor (3-7) 27.3%, moderate (8-28) 25.5%, major (over 28) 12.7%.
    #[test]
    fn rolled_injuries_follow_the_uefa_severity_bands() {
        let mut rng = StdRng::seed_from_u64(11);
        let mut counts = [0u32; 4];
        let rolls = 20_000;
        for _ in 0..rolls {
            let days = roll_injury(&mut rng).days_remaining;
            let band = match days {
                0..=2 => 0,
                3..=7 => 1,
                8..=28 => 2,
                _ => 3,
            };
            counts[band] += 1;
        }
        for (count, expected) in counts.iter().zip([34.5, 27.3, 25.5, 12.7]) {
            let share = f64::from(*count) / f64::from(rolls) * 100.0;
            assert!(
                (share - expected).abs() < 1.5,
                "band share {share:.1}% should be near {expected}%"
            );
        }
    }

    /// Given any rolled injury
    /// When its name and duration are read
    /// Then the name is a translated injury key and the duration is at least a day.
    #[test]
    fn a_rolled_injury_has_a_translated_name_and_at_least_a_day() {
        let mut rng = StdRng::seed_from_u64(12);
        for _ in 0..500 {
            let injury = roll_injury(&mut rng);
            assert!(injury.name.starts_with("common.injuries."));
            assert!(injury.days_remaining >= 1);
        }
    }

    #[test]
    fn roll_match_injury_skips_already_injured_player() {
        let mut player = make_player(40);
        player.injury = Some(Injury {
            name: "existing".to_string(),
            days_remaining: 10,
        });
        let mut rng = StdRng::seed_from_u64(4);

        assert!(!roll_match_injury(&mut player, &mut rng));
        assert_eq!(player.injury.as_ref().unwrap().name, "existing");
    }

    #[test]
    fn roll_match_injury_eventually_injures_a_low_fitness_player() {
        let mut rng = StdRng::seed_from_u64(5);
        let mut injured = false;
        for _ in 0..500 {
            let mut player = make_player(70);
            player.fitness = 20; // 3x multiplier
            if roll_match_injury(&mut player, &mut rng) {
                assert!(player.injury.is_some());
                let injury = player.injury.unwrap();
                assert!(injury.name.starts_with("common.injuries."));
                assert!(injury.days_remaining >= 1);
                injured = true;
                break;
            }
        }
        assert!(
            injured,
            "a low-fitness player should be injured within 500 rolls"
        );
    }

    /// Given 4,000 fresh players at fitness 75, each in one match
    /// When each rolls for a match injury
    /// Then between 1.5% and 3.5% are hurt: about one in forty
    #[test]
    fn a_fit_player_is_injured_in_about_one_match_in_forty() {
        let mut rng = StdRng::seed_from_u64(11);
        let injured = (0..4_000)
            .filter(|_| {
                let mut player = make_player(70);
                player.fitness = 75;
                roll_match_injury(&mut player, &mut rng)
            })
            .count();

        assert!((60..=140).contains(&injured), "{injured} of 4,000");
    }

    /// Given 4,000 fresh players at fitness 20, each in one match
    /// When each rolls for a match injury
    /// Then between 5.5% and 9.5% are hurt: three times the fit player's one in forty
    #[test]
    fn an_exhausted_player_is_injured_three_times_as_often() {
        let mut rng = StdRng::seed_from_u64(11);
        let injured = (0..4_000)
            .filter(|_| {
                let mut player = make_player(70);
                player.fitness = 20;
                roll_match_injury(&mut player, &mut rng)
            })
            .count();

        assert!((220..=380).contains(&injured), "{injured} of 4,000");
    }

    /// Given match fitness either side of each step: 29, 30, 49, 50, 69, 70, 89 and 90
    /// When the injury-risk multiplier is read
    /// Then it steps from 3.0 to 2.0 at 30, to 1.5 at 50, to 1.0 at 70 and to 0.7 at 90
    #[test]
    fn the_injury_risk_multiplier_steps_at_thirty_fifty_seventy_and_ninety() {
        let multipliers: Vec<f64> = [29, 30, 49, 50, 69, 70, 89, 90]
            .into_iter()
            .map(injury_multiplier_from_fitness)
            .collect();

        assert_eq!(multipliers, vec![3.0, 2.0, 2.0, 1.5, 1.5, 1.0, 1.0, 0.7]);
    }
}
