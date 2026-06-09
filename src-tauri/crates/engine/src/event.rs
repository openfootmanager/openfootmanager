use crate::types::{Side, Zone};
use serde::{Deserialize, Serialize};

/// A single event that occurred during the match.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchEvent {
    pub minute: u8,
    pub event_type: EventType,
    pub side: Side,
    pub zone: Zone,
    /// ID of the primary player involved (scorer, passer, fouler, etc.).
    pub player_id: Option<String>,
    /// ID of a secondary player (assist provider, fouled player, etc.).
    pub secondary_player_id: Option<String>,
    /// Optional engine-derived qualifier for richer commentary. `None` for
    /// events that carry no extra colour.
    #[serde(default)]
    pub detail: Option<EventDetail>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    // --- Structural events ---
    KickOff,
    HalfTime,
    SecondHalfStart,
    FullTime,

    // --- Possession & passing ---
    PassCompleted,
    PassIntercepted,

    // --- Attacking ---
    Dribble,
    DribbleTackled,
    Cross,

    // --- Shooting ---
    ShotOnTarget,
    ShotOffTarget,
    ShotBlocked,
    ShotSaved,
    Goal,
    PenaltyAwarded,
    PenaltyGoal,
    PenaltyMiss,

    // --- Defending ---
    Tackle,
    Interception,
    Clearance,

    // --- Fouls & discipline ---
    Foul,
    YellowCard,
    RedCard,
    SecondYellow,

    // --- Set pieces ---
    Corner,
    FreeKick,

    // --- Other ---
    Injury,
    GoalKick,
    Substitution,
}

/// Truthful, engine-derived qualifiers used to colour commentary.
/// Every variant carries only values the engine already computes, so prose
/// built from it never claims something that was not simulated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventDetail {
    Shot { danger: DangerBand },
    Save { quality: SaveQuality },
    Foul { severity: FoulSeverity },
    Goal { context: GoalContext },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DangerBand {
    Speculative,
    Decent,
    BigChance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SaveQuality {
    Routine,
    Strong,
    WorldClass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FoulSeverity {
    Soft,
    Hard,
    Reckless,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GoalContext {
    Opener,
    Equaliser,
    Extends,
    Consolation,
}

impl MatchEvent {
    pub fn new(minute: u8, event_type: EventType, side: Side, zone: Zone) -> Self {
        Self {
            minute,
            event_type,
            side,
            zone,
            player_id: None,
            secondary_player_id: None,
            detail: None,
        }
    }

    pub fn with_player(mut self, player_id: &str) -> Self {
        self.player_id = Some(player_id.to_string());
        self
    }

    pub fn with_secondary(mut self, player_id: &str) -> Self {
        self.secondary_player_id = Some(player_id.to_string());
        self
    }

    pub fn with_detail(mut self, detail: EventDetail) -> Self {
        self.detail = Some(detail);
        self
    }

    pub fn is_goal(&self) -> bool {
        matches!(self.event_type, EventType::Goal | EventType::PenaltyGoal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Side, Zone};

    #[test]
    fn new_event_has_no_detail() {
        let evt = MatchEvent::new(10, EventType::Goal, Side::Home, Zone::AwayBox);
        assert!(evt.detail.is_none());
    }

    #[test]
    fn with_detail_attaches_and_round_trips_through_serde() {
        let evt = MatchEvent::new(10, EventType::Goal, Side::Home, Zone::AwayBox)
            .with_player("p1")
            .with_detail(EventDetail::Goal {
                context: GoalContext::Equaliser,
            });
        let json = serde_json::to_string(&evt).unwrap();
        let back: MatchEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(
            back.detail,
            Some(EventDetail::Goal {
                context: GoalContext::Equaliser
            })
        );
    }
}
