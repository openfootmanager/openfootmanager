//! Players and sides for the engine's unit tests that pin an exact figure: every attribute is
//! the same, so a figure moves only by what a test changes.

use crate::types::{PlayStyle, PlayerData, PlayerRole, Position, TacticsConfig, TeamData};

pub(crate) fn player(id: &str, position: Position) -> PlayerData {
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

/// A 4-4-2 of eleven equal players: `{prefix}_gk`, `_def0` to `_def3`, `_mid0` to `_mid3` and
/// `_fwd0`, `_fwd1`, on the Standard role, Balanced, with neutral dials.
pub(crate) fn full_side(prefix: &str) -> TeamData {
    let line = |name: &str, count: usize, position: Position| -> Vec<PlayerData> {
        (0..count)
            .map(|index| player(&format!("{prefix}_{name}{index}"), position))
            .collect()
    };
    let mut players = vec![player(&format!("{prefix}_gk"), Position::Goalkeeper)];
    players.extend(line("def", 4, Position::Defender));
    players.extend(line("mid", 4, Position::Midfielder));
    players.extend(line("fwd", 2, Position::Forward));
    TeamData {
        id: prefix.to_string(),
        name: prefix.to_string(),
        formation: "4-4-2".to_string(),
        play_style: PlayStyle::Balanced,
        tactics: TacticsConfig::default(),
        players,
    }
}

/// Whether two figures are the same to within rounding.
pub(crate) fn same(left: f64, right: f64) -> bool {
    (left - right).abs() < 1e-9
}
