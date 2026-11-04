//! The tier a staff member's rating puts him in, among his own role: the same four tiers a player
//! has (S, A, B and the rest), painted in the same tones, but cut per role.
//!
//! A coach and an assistant are rated on different scales ([`crate::staff_rating`]: a coach by his
//! coaching, an assistant by a blend that centres about ten points lower), so one set of cut-offs
//! would make most coaches A and almost no assistant B. Each role's tiers are shares of that role,
//! the maintainer's ruling of 3 Nov: S its top 2.5%, A the next 9%, B the next 40%.
//!
//! The cut-offs sit on the bell staff are generated on (`generator::staff_bell`): for each role, the
//! whole rating whose share at or above comes closest to 2.5%, 11.5% and 51.5% on that role's
//! bell. Measured on the shipped world (five seeds, 440 staff per role each), the nearest-rank
//! cut-offs fall within a point of these, and they hold over twenty seasons: clubs keep the staff
//! they open with. The generator's own test (`each_roles_tiers_hold_the_shares_the_maintainer_set`)
//! fails if the bell moves away from them.

use domain::staff::StaffRole;

use crate::player_tier::{PlayerTier, TierCutOffs};

/// Each role's cut-offs. On the bell, the share at or above each is:
/// - coach (coaching, centred on 74.5): S 88 (2.3%), A 83 (10.9%), B 75 (50.0%);
/// - physio (physiotherapy, 77): S 90 (2.7%), A 85 (12.4%), B 77 (53.1%);
/// - scout (the judging mean, 75.8): S 89 (2.7%), A 84 (12.4%), B 76 (53.1%);
/// - assistant (delegation's blend, 65.3 after its rounding down): S 79 (2.1%), A 74 (10.4%),
///   B 66 (48.8%).
fn cut_offs(role: &StaffRole) -> TierCutOffs {
    match role {
        StaffRole::Coach => TierCutOffs {
            s: 88,
            a: 83,
            b: 75,
        },
        StaffRole::Physio => TierCutOffs {
            s: 90,
            a: 85,
            b: 77,
        },
        StaffRole::Scout => TierCutOffs {
            s: 89,
            a: 84,
            b: 76,
        },
        StaffRole::AssistantManager => TierCutOffs {
            s: 79,
            a: 74,
            b: 66,
        },
    }
}

/// The tier `rating` puts a member of `role` in, among that role.
pub fn staff_tier(role: &StaffRole, rating: u8) -> PlayerTier {
    cut_offs(role).tier(rating)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Given each role
    /// When its cut-offs are read
    /// Then they are the ones measured on the staff bell: the table every other test leans on.
    #[test]
    fn each_roles_cut_offs_are_the_measured_ones() {
        let table: Vec<(StaffRole, (u8, u8, u8))> = StaffRole::ALL
            .into_iter()
            .map(|role| {
                let cut = cut_offs(&role);
                (role, (cut.s, cut.a, cut.b))
            })
            .collect();
        assert_eq!(
            table,
            vec![
                (StaffRole::AssistantManager, (79, 74, 66)),
                (StaffRole::Coach, (88, 83, 75)),
                (StaffRole::Scout, (89, 84, 76)),
                (StaffRole::Physio, (90, 85, 77)),
            ]
        );
    }

    /// Given each role's three cut-offs
    /// When the rating at each cut-off and one below it are tiered
    /// Then the tier changes exactly there: S at S's cut-off and A one below, A and B likewise,
    ///      and B one above the rest.
    #[test]
    fn each_roles_tier_changes_exactly_at_its_cut_offs() {
        for role in StaffRole::ALL {
            let cut = cut_offs(&role);
            assert_eq!(staff_tier(&role, cut.s), PlayerTier::S, "{role:?}");
            assert_eq!(staff_tier(&role, cut.s - 1), PlayerTier::A, "{role:?}");
            assert_eq!(staff_tier(&role, cut.a), PlayerTier::A, "{role:?}");
            assert_eq!(staff_tier(&role, cut.a - 1), PlayerTier::B, "{role:?}");
            assert_eq!(staff_tier(&role, cut.b), PlayerTier::B, "{role:?}");
            assert_eq!(staff_tier(&role, cut.b - 1), PlayerTier::Rest, "{role:?}");
        }
    }

    /// Given a coach and an assistant who both rate 80
    /// When each is tiered among his own role
    /// Then the assistant is S (his role's blend centres about ten points lower) and the coach only
    ///      B: a rating means what it means for that role.
    #[test]
    fn one_rating_is_tiered_by_the_members_own_role() {
        assert_eq!(staff_tier(&StaffRole::AssistantManager, 80), PlayerTier::S);
        assert_eq!(staff_tier(&StaffRole::Coach, 80), PlayerTier::B);
    }
}
