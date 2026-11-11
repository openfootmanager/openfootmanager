//! A staff member's one rating, as the Staff page shows it: the attribute (or blend) his role's
//! work runs on, so the frontend reads it instead of recomputing a rule of its own (#589).

use domain::staff::{Staff, StaffRole};

/// The tenths delegated contract talks weigh an assistant manager's coaching, judging ability and
/// judging potential by. The generator's bell reads them too, to give his rating its spread.
pub(crate) const ASSISTANT_WEIGHTS: [i32; 3] = [4, 3, 3];

/// The assistant manager's quality, as delegated contract talks weigh it ([`ASSISTANT_WEIGHTS`]).
/// This is the engine's figure (it rounds down), shared by the delegation score and the staff
/// rating so the two can't drift.
pub(crate) fn assistant_quality(staff: &Staff) -> i32 {
    let [coaching, ability, potential] = ASSISTANT_WEIGHTS;
    (i32::from(staff.attributes.coaching) * coaching
        + i32::from(staff.attributes.judging_ability) * ability
        + i32::from(staff.attributes.judging_potential) * potential)
        / 10
}

/// A staff member's rating, 0 to 100, from the work his role does:
/// - a coach: his coaching, which training averages;
/// - a physio: his physiotherapy, which recovery averages;
/// - an assistant manager: [`assistant_quality`];
/// - a scout: the mean of judging ability and judging potential, rounded half up.
///
/// The scout's mean is a display aggregate only. Scouting uses the two apart (ability for senior
/// assignments' speed and slots, potential for youth and accuracy), so never feed this figure into
/// scouting's maths: read the attribute the work needs.
pub fn staff_rating(staff: &Staff) -> u8 {
    let attributes = &staff.attributes;
    match staff.role {
        StaffRole::Coach => clamp_to_rating(i32::from(attributes.coaching)),
        StaffRole::Physio => clamp_to_rating(i32::from(attributes.physiotherapy)),
        StaffRole::AssistantManager => clamp_to_rating(assistant_quality(staff)),
        StaffRole::Scout => {
            let sum =
                i32::from(attributes.judging_ability) + i32::from(attributes.judging_potential);
            clamp_to_rating((sum + 1) / 2)
        }
    }
}

/// The top of the rating scale. Generated staff stay within it, but nothing range-checks a
/// package's authored attributes (a `u8` holds up to 255), so the rating caps itself.
const MAX_RATING: i32 = 100;

/// A quality as a rating, held to 0 to 100 whatever a package wrote.
fn clamp_to_rating(quality: i32) -> u8 {
    u8::try_from(quality.clamp(0, MAX_RATING)).unwrap_or(u8::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::staff::StaffAttributes;

    /// Every attribute different, so a role that reads the wrong one gets the wrong number.
    fn staff(role: StaffRole) -> Staff {
        Staff::new(
            "s".to_string(),
            "First".to_string(),
            "Last".to_string(),
            "1980-01-01".to_string(),
            role,
            StaffAttributes {
                coaching: 81,
                judging_ability: 64,
                judging_potential: 57,
                physiotherapy: 42,
            },
        )
    }

    /// Given a coach whose coaching is 81 (his other attributes all different)
    /// When he is rated
    /// Then his rating is his coaching, the one attribute training reads from him.
    #[test]
    fn a_coach_is_rated_by_his_coaching() {
        assert_eq!(staff_rating(&staff(StaffRole::Coach)), 81);
    }

    /// Given a physio whose physiotherapy is 42
    /// When he is rated
    /// Then his rating is his physiotherapy, the one attribute recovery reads from him.
    #[test]
    fn a_physio_is_rated_by_his_physiotherapy() {
        assert_eq!(staff_rating(&staff(StaffRole::Physio)), 42);
    }

    /// Given an assistant manager with coaching 81, judging ability 64 and judging potential 57
    /// When he is rated
    /// Then his rating is the quality delegated contract talks weigh him by: (81·4 + 64·3 + 57·3)
    /// / 10 = 68.7, rounded down to 68 as the engine does.
    #[test]
    fn an_assistant_is_rated_by_the_quality_delegation_weighs() {
        let assistant = staff(StaffRole::AssistantManager);
        assert_eq!(assistant_quality(&assistant), 68);
        assert_eq!(staff_rating(&assistant), 68);
    }

    /// Given a scout with judging ability 64 and judging potential 57
    /// When he is rated
    /// Then his rating is their mean rounded half up, 60.5 to 61: a figure to show, which scouting
    /// itself never reads.
    #[test]
    fn a_scout_is_rated_by_the_mean_of_his_judging_rounded_half_up() {
        assert_eq!(staff_rating(&staff(StaffRole::Scout)), 61);
    }

    /// Given a package that authored a member of each role with every attribute at 150, which
    /// nothing range-checks
    /// When each is rated
    /// Then each reads 100, the top of the scale, never 150.
    #[test]
    fn an_authored_attribute_above_the_scale_rates_at_its_top() {
        for role in StaffRole::ALL {
            let mut member = staff(role);
            member.attributes = StaffAttributes {
                coaching: 150,
                judging_ability: 150,
                judging_potential: 150,
                physiotherapy: 150,
            };
            assert_eq!(staff_rating(&member), 100, "{:?}", member.role);
        }
    }
}
