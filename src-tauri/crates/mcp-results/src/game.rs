//! Results of the game lifecycle tools.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::tool_results;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveSummary {
    pub id: String,
    pub manager_name: String,
    pub last_played_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveList {
    pub saves: Vec<SaveSummary>,
}

impl fmt::Display for SaveList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.saves.is_empty() {
            return write!(f, "## Saves\n\nNo saves found.");
        }
        write!(
            f,
            "## Saves\n\n| ID | Manager | Last Played |\n|---|---|---|"
        )?;
        for save in &self.saves {
            write!(
                f,
                "\n| {} | {} | {} |",
                save.id, save.manager_name, save.last_played_at
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameSaved {
    pub save_id: String,
    pub date: String,
}

impl fmt::Display for GameSaved {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Game Saved\n\n**Save ID**: {}\n**Date**: {}",
            self.save_id, self.date
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameCreated {
    pub manager_first_name: String,
    pub manager_last_name: String,
    pub nationality: String,
    /// `None` while the game waits for `game_select_team`.
    pub save_id: Option<String>,
}

impl fmt::Display for GameCreated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Game Created\n\nManager: {} {}\nNationality: {}\n",
            self.manager_first_name, self.manager_last_name, self.nationality
        )?;
        match &self.save_id {
            Some(save_id) => write!(
                f,
                "Save ID: {save_id}\n\nUse `info_game_summary` to see your current state."
            ),
            None => write!(
                f,
                "No club yet: call `game_select_team` to start the career."
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamSelected {
    pub save_id: String,
}

impl fmt::Display for TeamSelected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Team Selected\n\n**Save ID**: {}\nTeam assigned and game saved.",
            self.save_id
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveLoaded {
    pub save_id: String,
    pub manager_name: String,
    pub date: String,
}

impl fmt::Display for SaveLoaded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Save Loaded\n\n**Save ID**: {}\n**Manager**: {}\n**Date**: {}",
            self.save_id, self.manager_name, self.date
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReturnedToMenu {
    pub saved: bool,
}

impl fmt::Display for ReturnedToMenu {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.saved {
            write!(
                f,
                "## Returned to Menu\n\nGame saved and cleared. Use `game_load_save` to resume."
            )
        } else {
            write!(
                f,
                "## Returned to Menu\n\nGame cleared without saving. Use `game_load_save` to resume."
            )
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldExported {
    pub path: String,
}

impl fmt::Display for WorldExported {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "## World Exported\n\nWritten to: {}", self.path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveDeleted {
    pub save_id: String,
}

impl fmt::Display for SaveDeleted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Save Deleted\n\nSave {} has been permanently deleted.",
            self.save_id
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldDatabase {
    pub id: String,
    pub name: String,
    pub team_count: usize,
    pub player_count: usize,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldDatabases {
    pub databases: Vec<WorldDatabase>,
}

impl fmt::Display for WorldDatabases {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.databases.is_empty() {
            return write!(f, "## World Databases\n\nNo world databases found.");
        }
        write!(
            f,
            "## World Databases\n\n| ID | Name | Teams | Players | Source |\n|---|---|---|---|---|"
        )?;
        for db in &self.databases {
            write!(
                f,
                "\n| {} | {} | {} | {} | {} |",
                db.id, db.name, db.team_count, db.player_count, db.source
            )?;
        }
        write!(
            f,
            "\n\nUse `game_new` with `world_source` set to a database path to start with that world."
        )
    }
}

tool_results!(
    SaveList,
    GameSaved,
    GameCreated,
    TeamSelected,
    SaveLoaded,
    ReturnedToMenu,
    WorldExported,
    SaveDeleted,
    WorldDatabases,
);
