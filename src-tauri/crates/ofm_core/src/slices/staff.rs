use serde::{Deserialize, Serialize};

use crate::game::{Game, ScoutingAssignment, YouthScoutingAssignment};
use domain::staff::Staff;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaffSlice {
    pub team_staff: Vec<Staff>,
    pub available_staff: Vec<Staff>,
    pub scouting_assignments: Vec<ScoutingAssignment>,
    pub youth_scouting_assignments: Vec<YouthScoutingAssignment>,
}

/// Returns the staff slice for a given team:
/// - `team_staff`: staff contracted to this team
/// - `available_staff`: unattached staff (no team_id) — the hiring market
/// - `scouting_assignments` / `youth_scouting_assignments`: all active assignments
///   (single-manager game means all assignments belong to this team's scouts)
pub fn query_staff(game: &Game, team_id: &str) -> StaffSlice {
    StaffSlice {
        team_staff: game
            .staff
            .iter()
            .filter(|s| s.team_id.as_deref() == Some(team_id))
            .cloned()
            .collect(),
        available_staff: game
            .staff
            .iter()
            .filter(|s| s.team_id.is_none())
            .cloned()
            .collect(),
        scouting_assignments: game.scouting_assignments.clone(),
        youth_scouting_assignments: game.youth_scouting_assignments.clone(),
    }
}
