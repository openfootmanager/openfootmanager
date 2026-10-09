//! Results of the season and job tools.

use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::tool_results;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SeasonStatus {
    Complete {},
    InProgress { remaining_fixtures: usize },
    NoLeague {},
}

impl fmt::Display for SeasonStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Complete {} => write!(
                f,
                "## Season Status: Complete ✅\n\nAll fixtures played. Use `season_advance` to proceed."
            ),
            Self::InProgress { remaining_fixtures } => write!(
                f,
                "## Season Status: In Progress\n\n**Remaining fixtures**: {remaining_fixtures}"
            ),
            Self::NoLeague {} => write!(f, "## Season Status: No league active."),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeasonAdvanced {
    /// The completed season's summary, exactly as the end-of-season rollover reports it.
    pub summary: Value,
    pub fired: bool,
}

impl fmt::Display for SeasonAdvanced {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let dismissal = if self.fired {
            "\n\n**You have been fired.** Use `jobs_available` to find a new position."
        } else {
            ""
        };
        write!(
            f,
            "## Season Advanced\n\n### Completed Season Summary\n```json\n{:#}\n```{}\n\nUse `info_game_state` to inspect the regenerated season.",
            self.summary, dismissal
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AwardWinner {
    pub name: String,
    pub team: String,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeasonAwards {
    pub golden_boot: Vec<AwardWinner>,
    pub assist_king: Vec<AwardWinner>,
    pub player_of_year: Vec<AwardWinner>,
    pub clean_sheet_king: Vec<AwardWinner>,
    pub most_appearances: Vec<AwardWinner>,
    pub young_player: Vec<AwardWinner>,
    pub manager_of_season: Vec<AwardWinner>,
}

fn write_award_table(
    f: &mut fmt::Formatter<'_>,
    title: &str,
    who: &str,
    winners: &[AwardWinner],
) -> fmt::Result {
    write!(f, "### {title}\n\n| # | {who} | Team | Value |\n|---|")?;
    writeln!(f, "{}|------|-------|", "-".repeat(who.len() + 2))?;
    for (i, winner) in winners.iter().enumerate() {
        writeln!(
            f,
            "| {} | {} | {} | {:.1} |",
            i + 1,
            winner.name,
            winner.team,
            winner.value
        )?;
    }
    Ok(())
}

impl fmt::Display for SeasonAwards {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "## Season Awards\n\n")?;
        let players = [
            ("🏆 Golden Boot", &self.golden_boot),
            ("🅰️ Assist King", &self.assist_king),
            ("⭐ Player of the Year", &self.player_of_year),
            ("🧤 Clean Sheet King", &self.clean_sheet_king),
            ("📋 Most Appearances", &self.most_appearances),
            ("🌟 Young Player", &self.young_player),
        ];
        for (title, winners) in players {
            if !winners.is_empty() {
                write_award_table(f, title, "Player", winners)?;
                writeln!(f)?;
            }
        }
        if !self.manager_of_season.is_empty() {
            write_award_table(
                f,
                "👔 Manager of the Season",
                "Manager",
                &self.manager_of_season,
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobOpening {
    pub team_id: String,
    pub team_name: String,
    pub city: String,
    pub reputation: u32,
    pub last_league_position: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AvailableJobs {
    pub jobs: Vec<JobOpening>,
}

impl fmt::Display for AvailableJobs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.jobs.is_empty() {
            return write!(
                f,
                "## Available Jobs\n\nNo job openings available right now."
            );
        }
        write!(
            f,
            "## Available Jobs ({} openings)\n\n| # | Team | City | Reputation | Last Position |\n|---|------|------|------------|---------------|\n",
            self.jobs.len()
        )?;
        for (i, job) in self.jobs.iter().enumerate() {
            let position = job
                .last_league_position
                .map_or_else(|| "-".to_string(), |p| p.to_string());
            writeln!(
                f,
                "| {} | {} ({}) | {} | {} | {} |",
                i + 1,
                job.team_name,
                job.team_id,
                job.city,
                job.reputation,
                position
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobApplicationOutcome {
    Hired,
    Rejected,
    InvalidTeam,
    AlreadyEmployed,
    SameTeam,
    NotBetterClub,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobApplication {
    pub outcome: JobApplicationOutcome,
}

impl fmt::Display for JobApplication {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self.outcome {
            JobApplicationOutcome::Hired => "✅ Hired! You are now the manager of this team.",
            JobApplicationOutcome::Rejected => "❌ Rejected. The team chose another candidate.",
            JobApplicationOutcome::InvalidTeam => "⚠️ Invalid team — no opening available.",
            JobApplicationOutcome::AlreadyEmployed => "⚠️ You already have a team. Resign first.",
            JobApplicationOutcome::SameTeam => "⚠️ You are already managing this team.",
            JobApplicationOutcome::NotBetterClub => "⚠️ This club is not a step up from your current position. Only better clubs will consider an employed manager.",
        };
        write!(f, "## Job Application Result\n\n{text}")
    }
}

tool_results!(
    SeasonStatus,
    SeasonAdvanced,
    SeasonAwards,
    AvailableJobs,
    JobApplication,
);
