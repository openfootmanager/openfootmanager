//! Results of the live match and press conference tools.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::tool_results;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveMatchStarted {
    /// "<competition>/<fixture>" for an exact fixture, otherwise "Index <n>".
    pub fixture: String,
    pub mode: String,
    pub minute: u8,
    pub home_score: u8,
    pub away_score: u8,
}

impl fmt::Display for LiveMatchStarted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Live Match Started\n\n**Fixture**: {}\n**Mode**: {}\n**Minute**: {}\n**Score**: {} - {}\n\nUse `match_step` to advance, `match_command` to issue tactical commands, and `match_finish` to end.",
            self.fixture, self.mode, self.minute, self.home_score, self.away_score
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchEventLine {
    pub minute: u8,
    /// "Home" or "Away".
    pub side: String,
    pub event: String,
    pub player_id: Option<String>,
}

impl fmt::Display for MatchEventLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}': {} {} ({})",
            self.minute,
            self.side,
            self.event,
            self.player_id.as_deref().unwrap_or("?")
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchAdvanced {
    pub minute: u8,
    pub phase: String,
    pub home_score: u8,
    pub away_score: u8,
    pub events: Vec<MatchEventLine>,
}

impl fmt::Display for MatchAdvanced {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Match Advanced\n\n**Minute**: {}\n**Phase**: {}\n**Score**: {} - {}\n\n### Events\n",
            self.minute, self.phase, self.home_score, self.away_score
        )?;
        if self.events.is_empty() {
            return write!(f, "No events occurred.");
        }
        let lines: Vec<String> = self.events.iter().map(ToString::to_string).collect();
        write!(f, "{}", lines.join("\n"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandApplied {
    pub minute: u8,
    pub home_score: u8,
    pub away_score: u8,
}

impl fmt::Display for CommandApplied {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Command Applied\n\n**Minute**: {}\n**Score**: {} - {}",
            self.minute, self.home_score, self.away_score
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchSnapshot {
    pub minute: u8,
    pub home_score: u8,
    pub away_score: u8,
    pub phase: String,
    /// Shares of possession, 0.0 to 1.0.
    pub home_possession: f64,
    pub away_possession: f64,
}

impl fmt::Display for MatchSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Match Snapshot\n\n**Minute**: {}\n**Score**: {} - {}\n**Phase**: {}\n**Possession**: Home {:.0}% / Away {:.0}%",
            self.minute,
            self.home_score,
            self.away_score,
            self.phase,
            self.home_possession * 100.0,
            self.away_possession * 100.0,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoundScore {
    pub home_team: String,
    pub home_goals: u8,
    pub away_goals: u8,
    pub away_team: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchFinished {
    pub date: String,
    /// The rest of the round's results, when the round was summarised.
    pub round_results: Option<Vec<RoundScore>>,
}

impl fmt::Display for MatchFinished {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "## Match Finished\n\n**Date**: {}", self.date)?;
        if let Some(results) = &self.round_results {
            let lines: Vec<String> = results
                .iter()
                .map(|r| {
                    format!(
                        "- {} {} - {} {}",
                        r.home_team, r.home_goals, r.away_goals, r.away_team
                    )
                })
                .collect();
            write!(f, "\n\n### Round Results\n{}", lines.join("\n"))?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoraleChange {
    pub player_id: String,
    pub delta: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamTalkApplied {
    pub tone: String,
    pub context: String,
    pub reactions: Vec<MoraleChange>,
}

impl fmt::Display for TeamTalkApplied {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Team Talk Applied\n\n**Tone**: {}\n**Context**: {}\n\n### Player Reactions\n",
            self.tone, self.context
        )?;
        if self.reactions.is_empty() {
            return write!(f, "No morale changes.");
        }
        let lines: Vec<String> = self
            .reactions
            .iter()
            .map(|r| {
                let emoji = match r.delta {
                    d if d > 0 => "📈",
                    d if d < 0 => "📉",
                    _ => "➡️",
                };
                format!("- {} {}: morale {:+}", emoji, r.player_id, r.delta)
            })
            .collect();
        write!(f, "{}", lines.join("\n"))
    }
}

/// What a press conference did, once applied: the squad morale it moved and the match it was about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PressConferenceComplete {
    pub squad_morale_delta: i16,
    /// How many of the user's players actually ended the conference on a different morale, and
    /// how many there were. Both effects clamp, so the delta alone says nothing about movement.
    pub squad_players_moved: usize,
    pub squad_size: usize,
    pub home_team_name: String,
    pub away_team_name: String,
    pub home_score: u8,
    pub away_score: u8,
}

/// The movement count is not decoration: a squad already on 100 absorbs a "+3" entirely, and a
/// `deflect` on a player question moves one player while leaving the squad delta at zero.
impl fmt::Display for PressConferenceComplete {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let emoji = match (self.squad_players_moved, self.squad_morale_delta) {
            (0, _) => "➡️",
            (_, delta) if delta > 0 => "📈",
            (_, delta) if delta < 0 => "📉",
            _ => "➡️",
        };
        write!(
            f,
            "## Press Conference Complete\n\n{} Squad morale {:+}: {} of {} players moved\n**Match**: {} {} - {} {}",
            emoji,
            self.squad_morale_delta,
            self.squad_players_moved,
            self.squad_size,
            self.home_team_name,
            self.home_score,
            self.away_score,
            self.away_team_name
        )
    }
}

tool_results!(
    LiveMatchStarted,
    MatchAdvanced,
    CommandApplied,
    MatchSnapshot,
    MatchFinished,
    TeamTalkApplied,
    PressConferenceComplete,
);
