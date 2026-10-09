//! Results of the time tools.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::tool_results;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayedMatch {
    pub home_team: String,
    pub home_goals: u8,
    pub away_goals: u8,
    pub away_team: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Won,
    Lost,
    Drew,
}

impl Outcome {
    fn verb(self) -> &'static str {
        match self {
            Self::Won => "won",
            Self::Lost => "lost",
            Self::Drew => "drew",
        }
    }
}

/// The user's own match among the day's results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct YourMatch {
    pub outcome: Outcome,
    pub your_goals: u8,
    pub their_goals: u8,
    pub opponent: String,
    pub at_home: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StandingsUpdate {
    pub position: usize,
    pub points: u32,
    pub goal_difference: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DayAdvanced {
    /// The new date, as `%d %B %Y`, or "Unknown" when the game could not be read back.
    pub date: String,
    pub results: Vec<PlayedMatch>,
    pub your_match: Option<YourMatch>,
    pub standings: Option<StandingsUpdate>,
    pub fired: bool,
    pub auto_saved: bool,
}

impl fmt::Display for DayAdvanced {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "## Day Advanced — {}\n\n", self.date)?;
        if !self.results.is_empty() {
            write!(
                f,
                "### Match Results\n\n| Home | Score | Away |\n|------|-------|------|\n"
            )?;
            for result in &self.results {
                writeln!(
                    f,
                    "| {} | {} - {} | {} |",
                    result.home_team, result.home_goals, result.away_goals, result.away_team
                )?;
            }
        }
        if let Some(mine) = &self.your_match {
            write!(
                f,
                "\nYour team {} {}-{} vs {} ({}).",
                mine.outcome.verb(),
                mine.your_goals,
                mine.their_goals,
                mine.opponent,
                if mine.at_home { "H" } else { "A" }
            )?;
        }
        if let Some(standing) = &self.standings {
            write!(
                f,
                "\n\n### Standings Update\n\nLeague position: {} | Points: {} | GD: {:+}",
                standing.position, standing.points, standing.goal_difference
            )?;
        }
        if self.fired {
            write!(
                f,
                "\n\n**⚠️ You have been fired!** Use `jobs_available` to find a new position."
            )?;
        }
        if self.auto_saved {
            write!(f, "\n\n💾 *Auto-saved.*")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum SkipToMatchDay {
    NoUpcomingMatch {},
    MatchDayToday {},
    Aborted {
        days_advanced: u32,
    },
    Skipped {
        days_advanced: u32,
        target_date: String,
    },
}

impl fmt::Display for SkipToMatchDay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoUpcomingMatch {} => write!(
                f,
                "## No Upcoming Match\n\nNo more fixtures scheduled for your team."
            ),
            Self::MatchDayToday {} => write!(
                f,
                "## Match Day Today\n\nYour next match is today. Use `time_advance` to play it."
            ),
            Self::Aborted { .. } => write!(
                f,
                "## Skip Aborted\n\nSkipped more than 365 days without reaching match. Something may be wrong."
            ),
            Self::Skipped {
                days_advanced,
                target_date,
            } => write!(
                f,
                "## Skipped to Match Day\n\n**{days_advanced} days advanced** to {target_date}.\nUse `time_advance` to play the match."
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Blockers {
    pub blockers: Vec<String>,
}

impl fmt::Display for Blockers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.blockers.is_empty() {
            return write!(f, "## No Blockers\n\nTime can be advanced safely.");
        }
        let list: Vec<String> = self.blockers.iter().map(|b| format!("- {b}")).collect();
        write!(f, "## ⚠️ Blockers Detected\n\n{}", list.join("\n"))
    }
}

tool_results!(DayAdvanced, SkipToMatchDay, Blockers);
