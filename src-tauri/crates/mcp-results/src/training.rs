//! Results of the training tools.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::tool_results;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrainingSettings {
    pub team_name: String,
    pub focus: String,
    pub intensity: String,
    pub schedule: String,
    pub group_count: usize,
    pub avg_condition: u32,
    pub avg_fitness: u32,
    pub injured_players: usize,
}

impl fmt::Display for TrainingSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Training Settings — {}\n\n\
             **Focus**: {}\n\
             **Intensity**: {}\n\
             **Schedule**: {}\n\
             **Training Groups**: {}\n\n\
             ### Squad Fitness Overview\n\
             | Metric | Value |\n|--------|-------|\n\
             | Avg Condition | {}% |\n\
             | Avg Fitness | {}% |\n\
             | Injured Players | {} |",
            self.team_name,
            self.focus,
            self.intensity,
            self.schedule,
            self.group_count,
            self.avg_condition,
            self.avg_fitness,
            self.injured_players,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrainingUpdated {
    pub focus: String,
    pub intensity: String,
}

impl fmt::Display for TrainingUpdated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Training Updated\n\n**Focus**: {}\n**Intensity**: {}",
            self.focus, self.intensity
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrainingScheduleUpdated {
    pub schedule: String,
}

impl fmt::Display for TrainingScheduleUpdated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Training Schedule Updated\n\n**Schedule**: {}",
            self.schedule
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrainingGroupsUpdated {
    pub group_count: usize,
}

impl fmt::Display for TrainingGroupsUpdated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Training Groups Updated\n\n**Groups**: {}",
            self.group_count
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerTrainingFocusUpdated {
    pub player_id: String,
    pub player_name: String,
    /// `None` when the player's own focus was cleared and the team default applies.
    pub focus: Option<String>,
}

impl fmt::Display for PlayerTrainingFocusUpdated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Player Training Focus Updated\n\n**{}**: {}",
            self.player_name,
            self.focus.as_deref().unwrap_or("cleared (team default)")
        )
    }
}

tool_results!(
    TrainingSettings,
    TrainingUpdated,
    TrainingScheduleUpdated,
    TrainingGroupsUpdated,
    PlayerTrainingFocusUpdated,
);
