//! The fixture the engine's calibration tests share: two equal sides of 70-rated players, so a
//! rate measured between them reads the engine's base rates and nothing else.

use engine::{PlayStyle, PlayerData, PlayerRole, Position, TacticsConfig, TeamData};

/// A fit player rated 70 in every attribute, at home in every line.
fn player(id: &str, position: Position) -> PlayerData {
    PlayerData {
        id: id.to_string(),
        name: id.to_string(),
        position,
        ovr: 70,
        condition: 100,
        fitness: 100,
        injury_risk: 1.0,
        line_familiarity: [1.0; 4],
        morale_factor: 1.0,
        pace: 70,
        stamina: 70,
        strength: 70,
        agility: 70,
        passing: 70,
        shooting: 70,
        tackling: 70,
        dribbling: 70,
        defending: 70,
        positioning: 70,
        vision: 70,
        decisions: 70,
        composure: 70,
        aggression: 70,
        teamwork: 70,
        leadership: 70,
        handling: 70,
        reflexes: 70,
        aerial: 70,
        traits: vec![],
        role: PlayerRole::Standard,
    }
}

/// A 4-4-2 of equal players, Balanced, on default tactics.
pub fn equal_side(prefix: &str) -> TeamData {
    let mut players = vec![player(&format!("{prefix}_gk"), Position::Goalkeeper)];
    for number in 1..=10 {
        let position = match number {
            1..=4 => Position::Defender,
            5..=8 => Position::Midfielder,
            _ => Position::Forward,
        };
        players.push(player(&format!("{prefix}_{number}"), position));
    }
    TeamData {
        id: prefix.to_string(),
        name: prefix.to_string(),
        formation: "4-4-2".to_string(),
        play_style: PlayStyle::Balanced,
        tactics: TacticsConfig::default(),
        players,
    }
}
