use rand::{Rng, RngExt};

use crate::event::{EventType, MatchEvent};
use crate::types::{Position, Side, Zone};

use super::{LiveMatchState, MinuteResult};

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
            let evt = MatchEvent::new(minute, EventType::PenaltyGoal, kicking_side, zone)
                .with_player(&taker.id);
            self.events.push(evt.clone());
            events.push(evt);
            match kicking_side {
                Side::Home => self.penalty_state.home_scored += 1,
                Side::Away => self.penalty_state.away_scored += 1,
            }
        } else {
            let evt = MatchEvent::new(minute, EventType::PenaltyMiss, kicking_side, zone)
                .with_player(&taker.id);
            self.events.push(evt.clone());
            events.push(evt);
        }

        match kicking_side {
            Side::Home => self.penalty_state.home_taken += 1,
            Side::Away => self.penalty_state.away_taken += 1,
        }

        // Check if shootout is decided
        let decided = self.check_penalty_decided();
        if decided {
            // Add penalty goals to score
            self.home_score += self.penalty_state.home_scored;
            self.away_score += self.penalty_state.away_scored;
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
        let ps = &self.penalty_state;

        if !ps.sudden_death {
            // Normal rounds (5 each)
            let home_remaining = 5u8.saturating_sub(ps.home_taken);
            let away_remaining = 5u8.saturating_sub(ps.away_taken);

            // Home can't catch up even if they score all remaining
            if ps.home_scored + home_remaining < ps.away_scored && ps.home_taken == ps.away_taken {
                return true;
            }
            if ps.away_scored + away_remaining < ps.home_scored && ps.away_taken == ps.home_taken {
                return true;
            }

            // After 5 rounds each
            if ps.home_taken >= 5 && ps.away_taken >= 5 && ps.home_scored != ps.away_scored {
                return true;
            }
            // If equal after 5 rounds, we enter sudden death on next step
            // (handled by setting sudden_death flag)

            false
        } else {
            // Sudden death: after each pair, check if one side leads
            ps.home_taken == ps.away_taken && ps.home_scored != ps.away_scored
        }
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
