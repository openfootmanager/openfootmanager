//! Generated staff weekly wages and the save-load repair that fills zeros.

use domain::staff::{Staff, StaffRole};
use domain::team::Team;
use std::collections::HashSet;

use crate::game::Game;

/// Reputation used to price unattached / market staff.
pub(crate) const UNATTACHED_STAFF_REPUTATION: u32 = 500;

/// Closed weekly wage for generated staff. Unattached market uses
/// [`UNATTACHED_STAFF_REPUTATION`].
pub(crate) fn generated_staff_weekly_wage(
    role: &StaffRole,
    club_reputation: u32,
    attrs: &domain::staff::StaffAttributes,
) -> u32 {
    let (base, attr): (i64, u8) = match role {
        StaffRole::AssistantManager => (2_500, attrs.coaching.max(attrs.judging_ability)),
        StaffRole::Coach => (1_200, attrs.coaching),
        StaffRole::Scout => (1_000, attrs.judging_ability.max(attrs.judging_potential)),
        StaffRole::Physio => (1_000, attrs.physiotherapy),
    };
    let reputation_component = i64::from(club_reputation) * 2;
    let attr_component = i64::from(attr) * 20;
    (base + reputation_component + attr_component).max(500) as u32
}

pub(super) fn assign_generated_staff_wage(staff: &mut Staff, club_reputation: u32) {
    staff.wage = generated_staff_weekly_wage(&staff.role, club_reputation, &staff.attributes);
}

/// Fill `wage == 0` staff on load, including unattached market candidates.
///
/// Content-gated, not a save-format bump: the weekly-unit lock already restamped
/// format, so a `< N` check would skip careers that loaded after that lock.
/// Old saves sized the wage cap on player wages only; filling employed zeros would
/// otherwise push those clubs through the 110% hire gate. Lift the cap to the
/// new bill only for clubs that had unpaid employed staff. Unattached zeros are
/// priced without raising any cap. Do not cut the transfer limit — this is a
/// one-time repair, like the weekly-unit runway floor.
pub fn backfill_zero_staff_wages(game: &mut Game) -> bool {
    let clubs_to_repair: HashSet<String> = game
        .staff
        .iter()
        .filter_map(|member| (member.wage == 0).then(|| member.team_id.clone()).flatten())
        .collect();
    if !price_zero_staff_wages_on(&mut game.staff, &game.teams) {
        return false;
    }
    let bills = crate::finances::wage_bills_by_club(game);
    for team in &mut game.teams {
        if !clubs_to_repair.contains(&team.id) {
            continue;
        }
        if let Some(&bill) = bills.get(&team.id) {
            team.wage_budget = team.wage_budget.max(bill);
        }
    }
    true
}

pub(super) fn price_zero_staff_wages_on(staff: &mut [Staff], teams: &[Team]) -> bool {
    let reputation_by_team: std::collections::HashMap<&str, u32> = teams
        .iter()
        .map(|team| (team.id.as_str(), team.reputation))
        .collect();
    let mut changed = false;
    for member in staff {
        if member.wage != 0 {
            continue;
        }
        let reputation = member
            .team_id
            .as_deref()
            .and_then(|id| reputation_by_team.get(id).copied())
            .unwrap_or(UNATTACHED_STAFF_REPUTATION);
        member.wage = generated_staff_weekly_wage(&member.role, reputation, &member.attributes);
        changed = true;
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::staff::StaffAttributes;

    fn staff_attrs(
        coaching: u8,
        judging_ability: u8,
        judging_potential: u8,
        physiotherapy: u8,
    ) -> StaffAttributes {
        StaffAttributes {
            coaching,
            judging_ability,
            judging_potential,
            physiotherapy,
        }
    }

    /// Given the closed staff-wage formula at reputation 500
    /// When each role is priced from its rated attribute
    /// Then the weekly wage is base + reputation×2 + attr×20.
    #[test]
    fn generated_staff_weekly_wage_matches_the_closed_formula() {
        assert_eq!(
            generated_staff_weekly_wage(
                &StaffRole::AssistantManager,
                500,
                &staff_attrs(70, 70, 50, 30)
            ),
            4_900
        );
        assert_eq!(
            generated_staff_weekly_wage(&StaffRole::Coach, 500, &staff_attrs(70, 50, 50, 30)),
            3_600
        );
        assert_eq!(
            generated_staff_weekly_wage(&StaffRole::Scout, 500, &staff_attrs(40, 80, 80, 20)),
            3_600
        );
        assert_eq!(
            generated_staff_weekly_wage(&StaffRole::Physio, 500, &staff_attrs(20, 20, 20, 60)),
            3_200
        );
    }

    /// Given unequal rated attributes and a club reputation other than 500
    /// When an assistant and a scout are priced
    /// Then the wage uses the higher of the pair and reputation×2.
    #[test]
    fn generated_staff_weekly_wage_uses_the_higher_rated_attribute_and_club_reputation() {
        assert_eq!(
            generated_staff_weekly_wage(
                &StaffRole::AssistantManager,
                800,
                &staff_attrs(80, 40, 50, 30)
            ),
            5_700
        );
        assert_eq!(
            generated_staff_weekly_wage(&StaffRole::Scout, 800, &staff_attrs(40, 40, 90, 20)),
            4_400
        );
    }
}
