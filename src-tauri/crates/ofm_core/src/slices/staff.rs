use serde::{Deserialize, Serialize};

use crate::game::{Game, ScoutingAssignment, YouthScoutingAssignment};
use crate::staff_rating::staff_rating;
use domain::staff::Staff;

/// A staff member as the Staff page lists him: his record, with his rating beside it, so the
/// frontend shows the rating the game computes instead of a formula of its own (#589).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaffRow {
    #[serde(flatten)]
    pub staff: Staff,
    /// [`staff_rating`]: the attribute, or blend, his role's work runs on. Public for the hiring
    /// market's free agents too, as their attributes are.
    #[serde(default)]
    pub rating: u8,
}

impl StaffRow {
    fn of(staff: &Staff) -> Self {
        Self {
            rating: staff_rating(staff),
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
