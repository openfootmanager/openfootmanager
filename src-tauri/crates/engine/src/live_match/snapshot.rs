use std::collections::HashMap;

use super::{LiveMatchState, MatchPhase, MatchSnapshot, PenaltyShootoutSnapshot};

// ---------------------------------------------------------------------------
// Snapshot generation — read-only view of match state for the UI
// ---------------------------------------------------------------------------

impl LiveMatchState {
    /// Get a full snapshot of the current match state for the UI.
    pub fn snapshot(&self) -> MatchSnapshot {
        let total_poss = self.home_possession_ticks + self.away_possession_ticks;
        let home_pct = if total_poss > 0 {
            self.home_possession_ticks as f64 / total_poss as f64 * 100.0
        } else {
            50.0
        };

        // Separate yellows by side
        let mut home_yellows = HashMap::new();
        let mut away_yellows = HashMap::new();
        for (pid, count) in &self.yellows {
            if self.home.players.iter().any(|p| p.id == *pid) {
                home_yellows.insert(pid.clone(), *count);
            } else {
                away_yellows.insert(pid.clone(), *count);
            }
        }

        // Clone teams and patch in live condition values from player_conditions map
        let mut home_team = self.home.clone();
        let mut away_team = self.away.clone();
        for p in home_team
            .players
            .iter_mut()
            .chain(away_team.players.iter_mut())
        {
            if let Some(&cond) = self.player_conditions.get(&p.id) {
                p.condition = cond.round() as u8;
            }
        }

        let has_shootout_data = self.penalty_state.home_taken > 0 || self.penalty_state.away_taken > 0;
        let penalty_shootout = if self.phase == MatchPhase::PenaltyShootout || (self.phase == MatchPhase::Finished && has_shootout_data) {
            Some(PenaltyShootoutSnapshot {
                home_taken: self.penalty_state.home_taken,
                away_taken: self.penalty_state.away_taken,
                home_scored: self.penalty_state.home_scored,
                away_scored: self.penalty_state.away_scored,
                sudden_death: self.penalty_state.sudden_death,
            })
        } else {
            None
        };

        MatchSnapshot {
            phase: self.phase,
            current_minute: self.current_minute,
            home_score: self.home_score,
            away_score: self.away_score,
            possession: self.possession,
            ball_zone: self.ball_zone,
            home_team,
            away_team,
            home_bench: self.home_bench.clone(),
            away_bench: self.away_bench.clone(),
            home_possession_pct: home_pct,
            away_possession_pct: 100.0 - home_pct,
            events: self.events.clone(),
            home_subs_made: self.home_subs_made,
            away_subs_made: self.away_subs_made,
            max_subs: self.max_subs,
            home_set_pieces: self.home_set_pieces.clone(),
            away_set_pieces: self.away_set_pieces.clone(),
            substitutions: self.substitutions.clone(),
            allows_extra_time: self.allows_extra_time,
            home_yellows,
            away_yellows,
            sent_off: self.sent_off.clone(),
            penalty_shootout,
        }
    }
}
