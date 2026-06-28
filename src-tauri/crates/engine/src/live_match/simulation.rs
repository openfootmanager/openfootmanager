use rand::{Rng, RngExt};

use crate::event::{EventType, MatchEvent};
use crate::shared::{
    tactics_break_speed_counter, tactics_counter_press_rewin, tactics_pressing_contest,
    tactics_tempo_retention,
};
use crate::types::{Side, Zone};

use super::{LiveMatchState, MatchPhase, MinuteResult};

// ---------------------------------------------------------------------------
// Phase transitions
// ---------------------------------------------------------------------------

impl LiveMatchState {
    pub(super) fn start_match<R: Rng>(&mut self, rng: &mut R) -> MinuteResult {
        self.phase = MatchPhase::FirstHalf;
        self.current_minute = 0;
        self.ball_zone = Zone::Midfield;
        self.possession = Side::Home;
        self.first_half_stoppage = rng.random_range(0..=self.config.stoppage_time_max);

        let evt = MatchEvent::new(0, EventType::KickOff, Side::Home, Zone::Midfield);
        self.events.push(evt.clone());

        MinuteResult {
            minute: 0,
            phase: MatchPhase::FirstHalf,
            events: vec![evt],
            home_score: 0,
            away_score: 0,
            possession: Side::Home,
            ball_zone: Zone::Midfield,
            is_finished: false,
        }
    }

    pub(super) fn start_second_half<R: Rng>(&mut self, rng: &mut R) -> MinuteResult {
        self.phase = MatchPhase::SecondHalf;
        // Second half starts after halftime; use at least minute 46 but never before current_minute
        let start_min = self.current_minute.max(46);
        self.current_minute = start_min;
        self.ball_zone = Zone::Midfield;
        self.possession = Side::Away;
        self.second_half_stoppage = rng.random_range(0..=self.config.stoppage_time_max);

        let evt = MatchEvent::new(
            start_min,
            EventType::SecondHalfStart,
            Side::Away,
            Zone::Midfield,
        );
        self.events.push(evt.clone());

        MinuteResult {
            minute: start_min,
            phase: MatchPhase::SecondHalf,
            events: vec![evt],
            home_score: self.home_score,
            away_score: self.away_score,
            possession: Side::Away,
            ball_zone: Zone::Midfield,
            is_finished: false,
        }
    }

    pub(super) fn start_et_second_half<R: Rng>(&mut self, rng: &mut R) -> MinuteResult {
        self.phase = MatchPhase::ExtraTimeSecondHalf;
        let start_min = self.current_minute.max(106);
        self.current_minute = start_min;
        self.ball_zone = Zone::Midfield;
        self.possession = Side::Home;
        self.et_second_half_stoppage = rng.random_range(0..=2); // short stoppage in ET

        let evt = MatchEvent::new(
            start_min,
            EventType::SecondHalfStart,
            Side::Home,
            Zone::Midfield,
        );
        self.events.push(evt.clone());

        MinuteResult {
            minute: start_min,
            phase: MatchPhase::ExtraTimeSecondHalf,
            events: vec![evt],
            home_score: self.home_score,
            away_score: self.away_score,
            possession: Side::Home,
            ball_zone: Zone::Midfield,
            is_finished: false,
        }
    }

    pub(super) fn handle_full_time<R: Rng>(&mut self, rng: &mut R) -> MinuteResult {
        if self.allows_extra_time && self.home_score == self.away_score {
            // Go to extra time
            self.phase = MatchPhase::ExtraTimeFirstHalf;
            self.current_minute = 91;
            self.ball_zone = Zone::Midfield;
            self.possession = Side::Home;
            self.et_first_half_stoppage = rng.random_range(0..=2);

            let evt = MatchEvent::new(91, EventType::KickOff, Side::Home, Zone::Midfield);
            self.events.push(evt.clone());

            MinuteResult {
                minute: 91,
                phase: MatchPhase::ExtraTimeFirstHalf,
                events: vec![evt],
                home_score: self.home_score,
                away_score: self.away_score,
                possession: Side::Home,
                ball_zone: Zone::Midfield,
                is_finished: false,
            }
        } else {
            // Match decided in normal time
            self.phase = MatchPhase::Finished;
            self.make_result(true)
        }
    }

    pub(super) fn handle_et_end<R: Rng>(&mut self, _rng: &mut R) -> MinuteResult {
        if self.home_score == self.away_score {
            // Go to penalty shootout
            self.phase = MatchPhase::PenaltyShootout;
            self.penalty_state = super::PenaltyShootoutState::default();

            let evt = MatchEvent::new(
                self.current_minute,
                EventType::PenaltyAwarded,
                Side::Home,
                Zone::Midfield,
            );
            self.events.push(evt.clone());

            MinuteResult {
                minute: self.current_minute,
                phase: MatchPhase::PenaltyShootout,
                events: vec![evt],
                home_score: self.home_score,
                away_score: self.away_score,
                possession: self.possession,
                ball_zone: Zone::Midfield,
                is_finished: false,
            }
        } else {
            self.phase = MatchPhase::Finished;
            self.make_result(true)
        }
    }

    // -----------------------------------------------------------------------
    // Core minute simulation
    // -----------------------------------------------------------------------

    pub(super) fn play_minute<R: Rng>(&mut self, rng: &mut R) -> MinuteResult {
        self.current_minute += 1;
        let minute = self.current_minute;

        // Track possession
        match self.possession {
            Side::Home => self.home_possession_ticks += 1,
            Side::Away => self.away_possession_ticks += 1,
        }

        // Deplete stamina for all on-pitch players
        self.deplete_stamina_tick();

        // Simulate 1-3 actions per minute
        let mut minute_events = Vec::new();
        let actions = rng.random_range(1..=3u8);
        for _ in 0..actions {
            let new_events = self.resolve_action(minute, rng);
            minute_events.extend(new_events);
        }

        // Possession contest. Tempo (retention) and pressing (ball-winning)
        // weight the battle here; neutral dials are ×1.0 and the transition
        // rolls only fire for non-neutral dials, so a default-vs-default match is
        // byte-identical to the pre-dial engine.
        let poss_side = self.possession;
        let def_side = poss_side.opposite();
        let poss_tactics = self.team_ref(poss_side).tactics.clone();
        let def_tactics = self.team_ref(def_side).tactics.clone();
        let mid_att =
            self.effective_midfield(poss_side) * tactics_tempo_retention(&poss_tactics);
        let mid_def =
            self.effective_midfield(def_side) * tactics_pressing_contest(&def_tactics);
        let retain = mid_att / (mid_att + mid_def);
        if rng.random_range(0.0..1.0f64) > retain {
            // Counter-press: the side losing the ball may win it straight back.
            let rewin = tactics_counter_press_rewin(&poss_tactics);
            if rewin > 0.0 && rng.random_range(0.0..1.0f64) < rewin {
                // Ball retained; possession and zone unchanged.
            } else {
                self.possession = def_side;
                // Break speed: the winner may spring a fast counter forward.
                let breakaway = tactics_break_speed_counter(&def_tactics);
                if breakaway > 0.0 && rng.random_range(0.0..1.0f64) < breakaway {
                    self.ball_zone = Zone::attacking_third(def_side);
                } else {
                    self.ball_zone = Zone::Midfield;
                }
            }
        }

        // Record ball zone for AI zone-pressure tracking (cap at 10)
        if self.recent_zones.len() >= 10 {
            self.recent_zones.pop_front();
        }
        self.recent_zones.push_back(self.ball_zone);

        // Check for phase transitions
        let transition_events = self.check_phase_end(minute, rng);
        minute_events.extend(transition_events);

        MinuteResult {
            minute,
            phase: self.phase,
            events: minute_events,
            home_score: self.home_score,
            away_score: self.away_score,
            possession: self.possession,
            ball_zone: self.ball_zone,
            is_finished: self.phase == MatchPhase::Finished,
        }
    }

    fn check_phase_end<R: Rng>(&mut self, minute: u8, _rng: &mut R) -> Vec<MatchEvent> {
        let mut events = Vec::new();
        match self.phase {
            MatchPhase::FirstHalf => {
                if minute >= 45 + self.first_half_stoppage {
                    self.phase = MatchPhase::HalfTime;
                    let evt =
                        MatchEvent::new(minute, EventType::HalfTime, Side::Home, Zone::Midfield);
                    self.events.push(evt.clone());
                    events.push(evt);
                }
            }
            MatchPhase::SecondHalf => {
                if minute >= 90 + self.second_half_stoppage {
                    self.phase = MatchPhase::FullTime;
                    let evt =
                        MatchEvent::new(minute, EventType::FullTime, Side::Home, Zone::Midfield);
                    self.events.push(evt.clone());
                    events.push(evt);
                }
            }
            MatchPhase::ExtraTimeFirstHalf => {
                if minute >= 105 + self.et_first_half_stoppage {
                    self.phase = MatchPhase::ExtraTimeHalfTime;
                    let evt =
                        MatchEvent::new(minute, EventType::HalfTime, Side::Home, Zone::Midfield);
                    self.events.push(evt.clone());
                    events.push(evt);
                }
            }
            MatchPhase::ExtraTimeSecondHalf => {
                if minute >= 120 + self.et_second_half_stoppage {
                    self.phase = MatchPhase::ExtraTimeEnd;
                    let evt =
                        MatchEvent::new(minute, EventType::FullTime, Side::Home, Zone::Midfield);
                    self.events.push(evt.clone());
                    events.push(evt);
                }
            }
            _ => {}
        }
        events
    }

    pub(super) fn make_result(&self, _is_finished: bool) -> MinuteResult {
        MinuteResult {
            minute: self.current_minute,
            phase: self.phase,
            events: Vec::new(),
            home_score: self.home_score,
            away_score: self.away_score,
            possession: self.possession,
            ball_zone: self.ball_zone,
            is_finished: true,
        }
    }
}
