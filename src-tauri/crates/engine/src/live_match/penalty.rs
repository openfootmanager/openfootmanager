use rand::{Rng, RngExt};

use crate::event::{EventType, MatchEvent};
use crate::types::{Position, Side, Zone};

use super::{LiveMatchState, MinuteResult, PenaltyShootoutState};

// ---------------------------------------------------------------------------
// Penalty shootout
// ---------------------------------------------------------------------------

impl LiveMatchState {
    pub(super) fn play_penalty_round<R: Rng>(&mut self, rng: &mut R) -> MinuteResult {
        let minute = self.current_minute;
        let mut events = Vec::new();

        // Determine which side kicks next (read-only access to penalty_state)
        let kicking_side = if self.penalty_state.home_taken <= self.penalty_state.away_taken {
            Side::Home
        } else {
            Side::Away
        };

        // Pick taker and goalkeeper (needs &self)
        let taker = self.pick_penalty_taker(kicking_side, rng);
        let gk = self.pick_goalkeeper(kicking_side.opposite());

        let shoot_skill = (taker.shooting as f64 + taker.composure as f64) / 2.0;
        let gk_skill = (gk.reflexes as f64 + gk.handling as f64) / 2.0;

        // Fatigue affects penalty accuracy in shootout
        let taker_condition = self
            .player_conditions
            .get(&taker.id)
            .copied()
            .unwrap_or(50.0);
        let fatigue_factor = (taker_condition / 100.0).clamp(0.7, 1.0);

        let conversion = (0.75 + (shoot_skill - gk_skill) / 300.0) * fatigue_factor;
        let conversion = conversion.clamp(0.55, 0.92);

        let zone = Zone::attacking_box(kicking_side);

        // Now mutate penalty_state
        let scored = rng.random_range(0.0..1.0f64) < conversion;
        if scored {
            let evt = MatchEvent::new(minute, EventType::ShootoutGoal, kicking_side, zone)
                .with_player(&taker.id);
            self.events.push(evt.clone());
            events.push(evt);
            match kicking_side {
                Side::Home => self.penalty_state.home_scored += 1,
                Side::Away => self.penalty_state.away_scored += 1,
            }
        } else {
            let evt = MatchEvent::new(minute, EventType::ShootoutMiss, kicking_side, zone)
                .with_player(&taker.id);
            self.events.push(evt.clone());
            events.push(evt);
        }

        match kicking_side {
            Side::Home => self.penalty_state.home_taken += 1,
            Side::Away => self.penalty_state.away_taken += 1,
        }

        // Level after a completed pair with the five regulation kicks taken:
        // every kick from here on is sudden death.
        {
            let ps = &mut self.penalty_state;
            if !ps.sudden_death
                && ps.home_taken >= 5
                && ps.home_taken == ps.away_taken
                && ps.home_scored == ps.away_scored
            {
                ps.sudden_death = true;
            }
        }

        // Check if shootout is decided
        let decided = self.check_penalty_decided();
        if decided {
            // The match score stays the regulation/ET score; the shootout
            // tally lives in penalty_state and is reported separately.
            self.phase = super::MatchPhase::Finished;

            let evt = MatchEvent::new(minute, EventType::FullTime, Side::Home, Zone::Midfield);
            self.events.push(evt.clone());
            events.push(evt);
        }

        MinuteResult {
            minute,
            phase: self.phase,
            events,
            home_score: self.home_score,
            away_score: self.away_score,
            possession: kicking_side,
            ball_zone: Zone::Midfield,
            is_finished: self.phase == super::MatchPhase::Finished,
        }
    }

    pub(super) fn check_penalty_decided(&self) -> bool {
        self.penalty_state.decided()
    }

    pub(super) fn resolve_in_match_penalty<R: Rng>(
        &mut self,
        minute: u8,
        att_side: Side,
        rng: &mut R,
    ) -> Vec<MatchEvent> {
        let mut events = Vec::new();

        // Use designated penalty taker if set
        let taker = match self.set_pieces_ref(att_side).penalty_taker.clone() {
            Some(id) => self.snap_player_by_id(&id, att_side),
            None => self.snap_player(att_side, Position::Forward, rng),
        };
        let gk = self.snap_player(att_side.opposite(), Position::Goalkeeper, rng);

        let shoot_skill = (taker.shooting as f64 + taker.decisions as f64) / 2.0;
        let gk_skill = (gk.positioning as f64 + gk.decisions as f64) / 2.0;
        let conversion = (0.75 + (shoot_skill - gk_skill) / 300.0).clamp(0.55, 0.92);
        let zone = Zone::attacking_box(att_side);

        if rng.random_range(0.0..1.0f64) < conversion {
            // PenaltyGoal intentionally carries no EventDetail::Goal context: penalty
            // commentary uses its own base key (match.commentary.PenaltyGoal) with no
            // opener/equaliser/... sub-variants. Brace/hat-trick is still detected on
            // the frontend via goal tally, which counts PenaltyGoal events.
            let evt = MatchEvent::new(minute, EventType::PenaltyGoal, att_side, zone)
                .with_player(&taker.id);
            self.events.push(evt.clone());
            events.push(evt);
            self.add_goal(att_side);
        } else {
            let evt = MatchEvent::new(minute, EventType::PenaltyMiss, att_side, zone)
                .with_player(&taker.id);
            self.events.push(evt.clone());
            events.push(evt);
        }

        events
    }
}

impl PenaltyShootoutState {
    /// Whether the shootout has produced a winner after the kick just taken.
    /// A round is only ever decided once both sides have taken the same number
    /// of kicks — the trailing side always gets its reply.
    fn decided(&self) -> bool {
        if !self.sudden_death {
            // Normal rounds (5 each)
            let home_remaining = 5u8.saturating_sub(self.home_taken);
            let away_remaining = 5u8.saturating_sub(self.away_taken);

            // Home can't catch up even if they score all remaining
            if self.home_scored + home_remaining < self.away_scored
                && self.home_taken == self.away_taken
            {
                return true;
            }
            if self.away_scored + away_remaining < self.home_scored
                && self.away_taken == self.home_taken
            {
                return true;
            }

            // After 5 completed rounds
            self.home_taken >= 5
                && self.home_taken == self.away_taken
                && self.home_scored != self.away_scored
        } else {
            // Sudden death: after each pair, check if one side leads
            self.home_taken == self.away_taken && self.home_scored != self.away_scored
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PenaltyShootoutState;

    fn state(home_taken: u8, away_taken: u8, home_scored: u8, away_scored: u8) -> PenaltyShootoutState {
        PenaltyShootoutState {
            round: 0,
            home_taken,
            away_taken,
            home_scored,
            away_scored,
            sudden_death: false,
        }
    }

    fn sudden_death(home_taken: u8, away_taken: u8, home_scored: u8, away_scored: u8) -> PenaltyShootoutState {
        PenaltyShootoutState {
            sudden_death: true,
            ..state(home_taken, away_taken, home_scored, away_scored)
        }
    }

    // Regression: 4-4 after five rounds each, home converts its 6th kick.
    // The shootout must NOT be decided before away's reply — regardless of
    // whether the sudden-death flag has been raised.
    #[test]
    fn round_six_not_decided_before_away_replies() {
        assert!(!state(6, 5, 5, 4).decided());
        assert!(!sudden_death(6, 5, 5, 4).decided());
    }

    #[test]
    fn sudden_death_decided_after_completed_pair() {
        assert!(sudden_death(6, 6, 5, 4).decided());
        assert!(sudden_death(7, 7, 5, 6).decided());
    }

    #[test]
    fn sudden_death_continues_while_level() {
        assert!(!sudden_death(6, 6, 5, 5).decided());
    }

    #[test]
    fn regulation_decided_after_five_rounds_with_lead() {
        assert!(state(5, 5, 4, 2).decided());
        assert!(state(5, 5, 2, 4).decided());
    }

    #[test]
    fn regulation_early_decision_when_uncatchable() {
        // 1-4 after four rounds each: home's single remaining kick can't catch up.
        assert!(state(4, 4, 1, 4).decided());
        assert!(state(4, 4, 4, 1).decided());
    }

    #[test]
    fn regulation_level_after_five_rounds_not_decided() {
        assert!(!state(5, 5, 3, 3).decided());
    }
}
