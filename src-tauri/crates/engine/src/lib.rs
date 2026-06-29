pub mod ai;
pub mod engine;
pub mod event;
pub mod live_match;
pub mod report;
pub(crate) mod shared;
pub mod types;

// Re-export key types for convenience
pub use engine::simulate;
pub use engine::simulate_with_rng;
pub use event::{EventType, MatchEvent};
pub use live_match::{
    LiveMatchState, MatchCommand, MatchPhase, MatchSnapshot, MinuteResult,
    PenaltyShootoutSnapshot, SetPieceTakers, SubstitutionRecord,
};
pub use report::{GoalDetail, GoalSource, MatchReport, PlayerMatchStats, TeamStats};
pub use types::{
    BreakSpeed, CounterPressDuration, DefensiveLine, DefensiveShape, MarkingStyle, MatchConfig,
    PlayStyle, PlayerData, PlayerRole, Position, PressingIntensity, Side, TacticsBuildUpStyle,
    TacticsConfig, TacticsPitchWidth, Tempo, TeamData, Zone,
};
