//! Results of the squad tools.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::tool_results;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SquadPlayer {
    pub id: String,
    pub name: String,
    pub in_starting_xi: bool,
    pub injured: bool,
    pub position: String,
    /// "?" when the date of birth cannot be read.
    pub age: String,
    pub ovr: u8,
    pub condition: u8,
    pub morale: u8,
    pub wage: u32,
    pub contract_end: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SquadOverview {
    pub team_name: String,
    /// Starting XI first, then the rest by overall rating.
    pub players: Vec<SquadPlayer>,
    /// "<position> <name>" for each starting-XI slot.
    pub starting_xi: Vec<String>,
    pub formation: String,
    pub play_style: String,
}

impl fmt::Display for SquadOverview {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## {} — Squad Overview\n\n\
             | ID | Name | Pos | Age | OVR | Con | Mor | Wage | Contract |\n\
             |----|-------|-----|-----|-----|-----|-----|------|----------|\n",
            self.team_name
        )?;
        for p in &self.players {
            writeln!(
                f,
                "| {} | {}{}{} | {} | {} | {} | {} | {} | {} | {} |",
                p.id,
                p.name,
                if p.in_starting_xi { "★" } else { "" },
                if p.injured { "⚠" } else { "" },
                p.position,
                p.age,
                p.ovr,
                p.condition,
                p.morale,
                p.wage,
                p.contract_end.as_deref().unwrap_or("-"),
            )?;
        }
        write!(
            f,
            "\n**Starting XI**: {}\n**Formation**: {} | **Play Style**: {}",
            self.starting_xi.join(", "),
            self.formation,
            self.play_style
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartingXiUpdated {
    pub starting_xi: Vec<String>,
    pub formation: String,
}

impl fmt::Display for StartingXiUpdated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Starting XI Updated\n\n{}\n**Formation**: {}",
            self.starting_xi.join(", "),
            self.formation
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormationChanged {
    pub formation: String,
}

impl fmt::Display for FormationChanged {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Formation Changed\n\n**New Formation**: {}\n**Note**: Outfield player positions have been reassigned based on defending ability.",
            self.formation
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayStyleChanged {
    pub play_style: String,
}

impl fmt::Display for PlayStyleChanged {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Play Style Changed\n\n**New Style**: {}",
            self.play_style
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchRolesUpdated {
    pub captain: Option<String>,
    pub vice_captain: Option<String>,
    pub penalty_taker: Option<String>,
    pub free_kick_taker: Option<String>,
    pub corner_taker: Option<String>,
}

impl fmt::Display for MatchRolesUpdated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "## Match Roles Updated\n\n")?;
        let roles = [
            ("Captain", &self.captain),
            ("Vice-captain", &self.vice_captain),
            ("Penalties", &self.penalty_taker),
            ("Free Kicks", &self.free_kick_taker),
            ("Corners", &self.corner_taker),
        ];
        let lines: Vec<String> = roles
            .into_iter()
            .map(|(label, holder)| format!("{label}: {}", holder.as_deref().unwrap_or("none")))
            .collect();
        write!(f, "{}", lines.join("\n"))
    }
}

/// A set-piece role and the player given it, as a player id and a match name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleHolder {
    pub player_id: String,
    pub player_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetPiecesAssigned {
    pub captain: Option<RoleHolder>,
    pub penalty_taker: Option<RoleHolder>,
    pub free_kick_taker: Option<RoleHolder>,
    pub corner_taker: Option<RoleHolder>,
}

impl fmt::Display for SetPiecesAssigned {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let lines: Vec<String> = [
            ("Captain", &self.captain),
            ("Penalties", &self.penalty_taker),
            ("Free Kicks", &self.free_kick_taker),
            ("Corners", &self.corner_taker),
        ]
        .into_iter()
        .filter_map(|(label, holder)| {
            holder
                .as_ref()
                .map(|holder| format!("{label}: {}", holder.player_name))
        })
        .collect();
        write!(f, "## Auto-Assigned Set Pieces\n\n{}", lines.join("\n"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerRoleUpdated {
    pub player_id: String,
    pub player_name: String,
    pub squad_role: String,
}

impl fmt::Display for PlayerRoleUpdated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Player Role Updated\n\n**{}**: {}",
            self.player_name, self.squad_role
        )
    }
}

tool_results!(
    SquadOverview,
    StartingXiUpdated,
    FormationChanged,
    PlayStyleChanged,
    MatchRolesUpdated,
    SetPiecesAssigned,
    PlayerRoleUpdated,
);
