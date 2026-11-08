//! How the board and the fans feel about the user's manager, in the words the Manager tab shows.
//!
//! The screen used to pick the words from the two numbers itself. Its board said "under serious
//! threat" below 30, while the board only warns at 25 (`firing`), so a manager on 27 was told of
//! a threat the board never issues. The board's words now follow its own warning threshold, and
//! the fans' bands live here as the one statement of them.

use domain::manager::Manager;
use serde::Serialize;
use ts_rs::TS;

use crate::firing::WARN_THRESHOLD;

/// The board's view of the manager, best first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum BoardMood {
    VeryPleased,
    Satisfied,
    Concerned,
    /// At or below the board's warning threshold, where the board warns him if it has not yet,
    /// gives a final warning lower down, and sacks a warned manager at the bottom
    /// (`firing::firing_decision`).
    UnderThreat,
}

/// The fans' view of the manager, best first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum FansMood {
    Adore,
    Behind,
    Mixed,
    Restless,
    Unrest,
}

/// The board's and the fans' view of a manager, as the Manager tab shows them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
pub struct ManagerStanding {
    pub board: BoardMood,
    pub fans: FansMood,
}

/// The board's view at `satisfaction`. Under threat exactly where the board warns.
pub fn board_mood(satisfaction: u8) -> BoardMood {
    if satisfaction <= WARN_THRESHOLD {
        BoardMood::UnderThreat
    } else if satisfaction < 50 {
        BoardMood::Concerned
    } else if satisfaction < 80 {
        BoardMood::Satisfied
    } else {
        BoardMood::VeryPleased
    }
}

/// The fans' view at `fan_approval`.
pub fn fans_mood(fan_approval: u8) -> FansMood {
    match fan_approval {
        80.. => FansMood::Adore,
        60..=79 => FansMood::Behind,
        40..=59 => FansMood::Mixed,
        20..=39 => FansMood::Restless,
        _ => FansMood::Unrest,
    }
}

pub fn manager_standing(manager: &Manager) -> ManagerStanding {
    ManagerStanding {
        board: board_mood(manager.satisfaction),
        fans: fans_mood(manager.fan_approval),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Given the board's satisfaction just above and at its warning threshold (25)
    /// When its mood is read
    /// Then 27 and 26 are concerned, and 25 is under threat, where the board warns (the screen
    ///      called 27 a threat)
    #[test]
    fn the_board_is_under_threat_exactly_where_it_warns() {
        assert_eq!(board_mood(27), BoardMood::Concerned);
        assert_eq!(board_mood(26), BoardMood::Concerned);
        assert_eq!(board_mood(25), BoardMood::UnderThreat);
        assert_eq!(board_mood(0), BoardMood::UnderThreat);
    }

    /// Given the board's satisfaction at each band edge above the threat
    /// When its mood is read
    /// Then 49 is concerned, 50 and 79 satisfied, 80 very pleased
    #[test]
    fn the_boards_other_bands_start_at_fifty_and_eighty() {
        assert_eq!(board_mood(49), BoardMood::Concerned);
        assert_eq!(board_mood(50), BoardMood::Satisfied);
        assert_eq!(board_mood(79), BoardMood::Satisfied);
        assert_eq!(board_mood(80), BoardMood::VeryPleased);
    }

    /// Given the fans' approval at each band edge
    /// When their mood is read
    /// Then the bands start at 80, 60, 40 and 20, as the screen showed them
    #[test]
    fn the_fans_bands_start_at_eighty_sixty_forty_and_twenty() {
        let read: Vec<FansMood> = [80, 79, 60, 59, 40, 39, 20, 19]
            .into_iter()
            .map(fans_mood)
            .collect();
        assert_eq!(
            read,
            vec![
                FansMood::Adore,
                FansMood::Behind,
                FansMood::Behind,
                FansMood::Mixed,
                FansMood::Mixed,
                FansMood::Restless,
                FansMood::Restless,
                FansMood::Unrest,
            ]
        );
    }

    /// Given a manager's standing
    /// When it goes over the wire
    /// Then both moods are snake_case words
    #[test]
    fn a_standing_serializes_as_two_words() {
        let mut manager = Manager::new(
            "m".into(),
            "A".into(),
            "B".into(),
            "1980-01-01".into(),
            "GB".into(),
        );
        manager.satisfaction = 20;
        manager.fan_approval = 85;

        assert_eq!(
            serde_json::to_value(manager_standing(&manager)).unwrap(),
            serde_json::json!({ "board": "under_threat", "fans": "adore" })
        );
    }
}
