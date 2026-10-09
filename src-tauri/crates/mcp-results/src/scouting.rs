//! Results of the scouting tools.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::tool_results;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoutDispatched {
    pub scout_id: String,
    pub scout_name: String,
    pub player_id: String,
    pub player_name: String,
}

impl fmt::Display for ScoutDispatched {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Scout Dispatched\n\n**{}** will report on **{}**.",
            self.scout_name, self.player_name
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoutReport {
    pub message_id: String,
    pub player_name: String,
    pub position: String,
    pub avg_rating: Option<u32>,
    /// `None` for a free agent.
    pub team: Option<String>,
    pub read: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoutingAssignment {
    pub id: String,
    pub scout_name: String,
    pub player_name: String,
    pub days_remaining: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoutReports {
    pub reports: Vec<ScoutReport>,
    pub active_assignments: Vec<ScoutingAssignment>,
}

impl fmt::Display for ScoutReports {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.reports.is_empty() {
            return write!(f, "## Scout Reports\n\nNo scout reports available.");
        }
        write!(
            f,
            "## Scout Reports ({} reports)\n\n| ID | Player | Pos | Rating | Team | Read |\n|----|--------|-----|--------|------|------|\n",
            self.reports.len()
        )?;
        for report in &self.reports {
            let rating = report
                .avg_rating
                .map_or_else(|| "?".to_string(), |v| format!("{v}/100"));
            writeln!(
                f,
                "| {} | {} | {} | {} | {} | {} |",
                report.message_id,
                report.player_name,
                report.position,
                rating,
                report.team.as_deref().unwrap_or("Free"),
                if report.read { "✓" } else { "●" },
            )?;
        }
        if !self.active_assignments.is_empty() {
            write!(
                f,
                "\n### Active Assignments ({} pending)\n\n| ID | Scout | Player | Days Left |\n|----|-------|--------|------------|\n",
                self.active_assignments.len()
            )?;
            for a in &self.active_assignments {
                writeln!(
                    f,
                    "| {} | {} | {} | {} |",
                    a.id, a.scout_name, a.player_name, a.days_remaining
                )?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct YouthScoutingStarted {
    pub scout_id: String,
}

impl fmt::Display for YouthScoutingStarted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Youth Scouting Started\n\nAssignment created. Check `scout_get_reports` for results over time."
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct YouthScoutingCancelled {
    pub assignment_id: String,
}

impl fmt::Display for YouthScoutingCancelled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Youth Scouting Cancelled\n\nAssignment has been cancelled."
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct YouthScoutingReassigned {
    pub assignment_id: String,
    pub scout_id: String,
}

impl fmt::Display for YouthScoutingReassigned {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "## Youth Scouting Reassigned\n\nScout has been changed.")
    }
}

tool_results!(
    ScoutDispatched,
    ScoutReports,
    YouthScoutingStarted,
    YouthScoutingCancelled,
    YouthScoutingReassigned,
);
