//! Results of the club, staff and finance tools.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::tool_results;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FacilityUpgraded {
    pub facility: String,
}

impl fmt::Display for FacilityUpgraded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "## Facility Upgraded\n\n**{}** upgraded.", self.facility)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffMember {
    pub id: String,
    pub name: String,
    pub role: String,
    /// The club's name, or `None` for a free agent.
    pub team: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffList {
    pub staff: Vec<StaffMember>,
}

impl fmt::Display for StaffList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Staff\n\n| ID | Name | Role | Team |\n|----|------|------|------|\n"
        )?;
        for member in &self.staff {
            let team = member.team.as_deref().unwrap_or("Unattached");
            writeln!(
                f,
                "| {} | {} | {} | {} |",
                member.id, member.name, member.role, team
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffHired {
    pub staff_id: String,
}

impl fmt::Display for StaffHired {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "## Staff Hired\n\nStaff member {} hired.", self.staff_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffReleased {
    pub staff_id: String,
}

impl fmt::Display for StaffReleased {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Staff Released\n\nStaff member {} released.",
            self.staff_id
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoardSupport {
    pub support_amount: i64,
    pub transfer_budget_reduction: i64,
    pub satisfaction_penalty: u8,
}

impl fmt::Display for BoardSupport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Board Support\n\n**Amount**: {}\n**Transfer Budget Reduction**: {}\n**Satisfaction Penalty**: {}",
            self.support_amount, self.transfer_budget_reduction, self.satisfaction_penalty
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketingCampaign {
    pub gross_revenue: i64,
}

impl fmt::Display for MarketingCampaign {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Marketing Campaign\n\n**Gross Revenue**: {}",
            self.gross_revenue
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SponsorPitch {
    pub sponsor_name: String,
    pub weekly_amount: i64,
    pub duration_weeks: u32,
}

impl fmt::Display for SponsorPitch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Sponsor Pitch\n\n**Sponsor**: {}\n**Weekly Amount**: {}\n**Duration**: {} weeks",
            self.sponsor_name, self.weekly_amount, self.duration_weeks
        )
    }
}

tool_results!(
    FacilityUpgraded,
    StaffList,
    StaffHired,
    StaffReleased,
    BoardSupport,
    MarketingCampaign,
    SponsorPitch,
);
