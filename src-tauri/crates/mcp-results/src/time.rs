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
    /// Stopped by blockers, as the screen's skip is: nothing to skip past until they are resolved
    /// (`days_advanced` is how many days passed before one appeared, 0 when one stood at the start).
    Blocked {
        days_advanced: u32,
        blockers: Vec<Blocker>,
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
            Self::Blocked {
                days_advanced,
                blockers,
            } => {
                let list: Vec<String> = blockers.iter().map(|b| format!("- {b}")).collect();
                write!(
                    f,
                    "## Skip Blocked\n\n**{days_advanced} days advanced** before something needs you:\n\n{}\n\nResolve these, as a manager would on the screen, then skip again.",
                    list.join("\n")
                )
            }
        }
    }
}

/// One thing that stops time, exactly as the game's own blocker check reports it to the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Blocker {
    /// A stable name such as `injured_xi`, `incomplete_xi`, `squad_size_crisis`,
    /// `planned_contract_exit_crisis`, `key_contract_risk` or `contract_wage_risk`.
    pub id: String,
    /// `warn` for everything that stops time today.
    pub severity: String,
    /// The screen that resolves it.
    pub tab: String,
    /// Counts and names the blocker is about (`{"count": "2"}`), when it has any.
    pub params: Option<serde_json::Value>,
}

impl fmt::Display for Blocker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let params = self
            .params
            .as_ref()
            .map_or_else(String::new, |params| format!(" {params}"));
        write!(
            f,
            "[{}] {} (resolve on the {} screen){params}",
            self.severity, self.id, self.tab
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Blockers {
    /// What stops `time_advance` and `time_skip_to_match_day`, which refuse while any stands, as
    /// the UI's Continue and skip do: the same list and severities the UI uses, informational items
    /// (unread urgent mail) left out.
    pub blockers: Vec<Blocker>,
    /// A live match is in progress: finish it first.
    #[serde(default)]
    pub live_match_in_progress: bool,
}

impl fmt::Display for Blockers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.blockers.is_empty() && !self.live_match_in_progress {
            return write!(f, "## No Blockers\n\nTime can be advanced safely.");
        }
        let mut lines = Vec::new();
        if self.live_match_in_progress {
            lines.push("- Live match in progress — finish the match first".to_string());
        }
        for blocker in &self.blockers {
            lines.push(format!("- {blocker}"));
        }
        write!(f, "## ⚠️ Blockers Detected\n\n{}", lines.join("\n"))
    }
}

tool_results!(DayAdvanced, SkipToMatchDay, Blockers);
