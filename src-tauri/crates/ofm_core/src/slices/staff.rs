use serde::{Deserialize, Serialize};

use crate::game::{Game, ScoutingAssignment, YouthScoutingAssignment};
use crate::player_tier::PlayerTier;
use crate::staff_rating::staff_rating;
use crate::staff_tier::staff_tier;
use domain::staff::Staff;

/// A staff member as the Staff page lists him: his record, with his rating and its tier beside it,
/// so the frontend shows the game's figure and tone instead of formulas of its own (#589).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaffRow {
    #[serde(flatten)]
    pub staff: Staff,
    /// [`staff_rating`]: the attribute, or blend, his role's work runs on. Public for the hiring
    /// market's free agents too, as their attributes are.
    #[serde(default)]
    pub rating: u8,
    /// [`staff_tier`]: where that rating stands among his own role, which the page paints.
    #[serde(default)]
    pub tier: PlayerTier,
}

impl StaffRow {
    fn of(staff: &Staff) -> Self {
        let rating = staff_rating(staff);
        Self {
            rating,
            tier: staff_tier(&staff.role, rating),
            staff: staff.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaffSlice {
    pub team_staff: Vec<StaffRow>,
    pub available_staff: Vec<StaffRow>,
    pub scouting_assignments: Vec<ScoutingAssignment>,
    pub youth_scouting_assignments: Vec<YouthScoutingAssignment>,
}

/// Returns the staff slice for a given team:
/// - `team_staff`: staff contracted to this team, each with his rating
/// - `available_staff`: unattached staff (no team_id), the hiring market, each with his rating
/// - `scouting_assignments` / `youth_scouting_assignments`: all active assignments
///   (single-manager game means all assignments belong to this team's scouts)
pub fn query_staff(game: &Game, team_id: &str) -> StaffSlice {
    StaffSlice {
        team_staff: game
            .staff
            .iter()
            .filter(|s| s.team_id.as_deref() == Some(team_id))
            .map(StaffRow::of)
            .collect(),
        available_staff: game
            .staff
            .iter()
            .filter(|s| s.team_id.is_none())
            .map(StaffRow::of)
            .collect(),
        scouting_assignments: game.scouting_assignments.clone(),
        youth_scouting_assignments: game.youth_scouting_assignments.clone(),
    }
}
