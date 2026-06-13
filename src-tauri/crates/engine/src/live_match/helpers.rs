use rand::{Rng, RngExt};

use crate::event::{DangerBand, FoulSeverity, GoalContext, SaveQuality};
use crate::shared::{PlayStylePhase, PlayerSnap, home_mod, play_style_modifier};
use crate::types::{PlayerData, Position, Side, TeamData};

use super::{LiveMatchState, SetPieceTakers};

// ---------------------------------------------------------------------------
// Stamina system
// ---------------------------------------------------------------------------

impl LiveMatchState {
    pub(super) fn deplete_stamina_tick(&mut self) {
        let fatigue_rate = self.config.fatigue_per_minute;
        // Iterate over all on-pitch players
        for p in self.home.players.iter().chain(self.away.players.iter()) {
            if self.sent_off.contains(&p.id) {
                continue;
            }
            let stamina_factor = p.stamina as f64 / 100.0;
            let fitness_factor = p.fitness as f64 / 100.0;
            // Higher stamina → less depletion; higher fitness → less depletion.
            // Fitness scales the base depletion more aggressively (unfit players tire much faster).
            let depletion =
                fatigue_rate * (1.0 - stamina_factor * 0.5) * (1.3 - fitness_factor * 0.6);
            if let Some(cond) = self.player_conditions.get_mut(&p.id) {
                *cond = (*cond - depletion).max(5.0);
            }
        }
    }

    /// Adjust a skill value based on the player's current in-match condition.
    pub(super) fn condition_adjusted_skill(&self, player_id: &str, base_skill: f64) -> f64 {
        let condition = self
            .player_conditions
            .get(player_id)
            .copied()
            .unwrap_or(50.0);
        // At 100% condition: full skill. At 50%: ~80% skill. At 0%: ~60% skill.
        let factor = 0.6 + 0.4 * (condition / 100.0);
        base_skill * factor
    }

    // -----------------------------------------------------------------------
    // Player selection helpers
    // -----------------------------------------------------------------------

    pub(super) fn snap_player<R: Rng>(
        &self,
        side: Side,
        preferred: Position,
        rng: &mut R,
    ) -> PlayerSnap {
        let team = self.team_ref(side);
        let available: Vec<&PlayerData> = team
            .players
            .iter()
            .filter(|p| !self.sent_off.contains(&p.id))
            .collect();

        let candidates: Vec<&PlayerData> = available
            .iter()
            .filter(|p| p.position == preferred)
            .copied()
            .collect();

        let pool = if candidates.is_empty() {
            &available
        } else {
            &candidates
        };
        if pool.is_empty() {
            return PlayerSnap::from(&team.players[0]);
        }
        PlayerSnap::from(pool[rng.random_range(0..pool.len())])
    }

    pub(super) fn snap_player_by_id(&self, player_id: &str, side: Side) -> PlayerSnap {
        let team = self.team_ref(side);
        if let Some(p) = team.players.iter().find(|p| p.id == player_id) {
            PlayerSnap::from(p)
        } else {
            PlayerSnap::from(&team.players[0])
        }
    }

    pub(super) fn pick_penalty_taker<R: Rng>(&self, side: Side, rng: &mut R) -> PlayerSnap {
        // Use designated taker if set
        if let Some(ref id) = self.set_pieces_ref(side).penalty_taker {
            let team = self.team_ref(side);
            if let Some(p) = team
                .players
                .iter()
                .find(|p| p.id == *id && !self.sent_off.contains(&p.id))
            {
                return PlayerSnap::from(p);
            }
        }
        // Fallback: pick the forward with highest shooting
        let team = self.team_ref(side);
        let mut candidates: Vec<&PlayerData> = team
            .players
            .iter()
            .filter(|p| !self.sent_off.contains(&p.id))
            .collect();
        candidates.sort_by(|a, b| b.shooting.cmp(&a.shooting));
        if let Some(p) = candidates.first() {
            PlayerSnap::from(p)
        } else {
            self.snap_player(side, Position::Forward, rng)
        }
    }

    pub(super) fn pick_goalkeeper(&self, side: Side) -> PlayerSnap {
        let team = self.team_ref(side);
        for p in &team.players {
            if p.position == Position::Goalkeeper && !self.sent_off.contains(&p.id) {
                return PlayerSnap::from(p);
            }
        }
        // No goalkeeper available — pick first available
        for p in &team.players {
            if !self.sent_off.contains(&p.id) {
                return PlayerSnap::from(p);
            }
        }
        PlayerSnap::from(&team.players[0])
    }

    // -----------------------------------------------------------------------
    // Rating helpers
    // -----------------------------------------------------------------------

    pub(super) fn effective_midfield(&self, side: Side) -> f64 {
        let base = self.team_ref(side).midfield_rating();
        let modifier = play_style_modifier(
            self.team_ref(side).play_style,
            PlayStylePhase::Midfield,
            true,
        );
        base * modifier * home_mod(side, &self.config)
    }

    pub(super) fn effective_press(&self, pressing_side: Side) -> f64 {
        let team = self.team_ref(pressing_side);
        let base = team.position_attr_avg(Position::Midfielder, |p| {
            ((p.stamina as u16 + p.tackling as u16 + p.pace as u16) / 3) as u8
        });
        let modifier = play_style_modifier(team.play_style, PlayStylePhase::Press, true);
        base * modifier * home_mod(pressing_side, &self.config)
    }

    // -----------------------------------------------------------------------
    // Internal accessors
    // -----------------------------------------------------------------------

    pub(super) fn team_ref(&self, side: Side) -> &TeamData {
        match side {
            Side::Home => &self.home,
            Side::Away => &self.away,
        }
    }

    pub(super) fn team_mut(&mut self, side: Side) -> &mut TeamData {
        match side {
            Side::Home => &mut self.home,
            Side::Away => &mut self.away,
        }
    }

    pub(super) fn set_pieces_ref(&self, side: Side) -> &SetPieceTakers {
        match side {
            Side::Home => &self.home_set_pieces,
            Side::Away => &self.away_set_pieces,
        }
    }

    pub(super) fn set_pieces_mut(&mut self, side: Side) -> &mut SetPieceTakers {
        match side {
            Side::Home => &mut self.home_set_pieces,
            Side::Away => &mut self.away_set_pieces,
        }
    }

    /// Classify a goal about to be scored by `side`, using the CURRENT (pre-increment) score.
    pub(super) fn goal_context(&self, side: Side) -> GoalContext {
        let (own, opp) = match side {
            Side::Home => (self.home_score, self.away_score),
            Side::Away => (self.away_score, self.home_score),
        };
        let own_new = own + 1;
        if own == 0 && opp == 0 {
            GoalContext::Opener
        } else if own_new == opp {
            GoalContext::Equaliser
        } else if own_new > opp {
            GoalContext::Extends
        } else {
            GoalContext::Consolation
        }
    }

    pub(super) fn add_goal(&mut self, side: Side) {
        match side {
            Side::Home => self.home_score += 1,
            Side::Away => self.away_score += 1,
        }
    }
}

/// Map a shooter's effective rating to a danger band for shot commentary.
pub(super) fn danger_band(shoot_rating: f64) -> DangerBand {
    if shoot_rating >= 68.0 {
        DangerBand::BigChance
    } else if shoot_rating >= 50.0 {
        DangerBand::Decent
    } else {
        DangerBand::Speculative
    }
}

/// Map a keeper's effective rating to a save-quality band.
pub(super) fn save_quality(gk_rating: f64) -> SaveQuality {
    if gk_rating >= 68.0 {
        SaveQuality::WorldClass
    } else if gk_rating >= 50.0 {
        SaveQuality::Strong
    } else {
        SaveQuality::Routine
    }
}

/// Map a fouler's aggression (0-100) to a foul-severity band.
pub(super) fn foul_severity(aggression: u8) -> FoulSeverity {
    if aggression >= 70 {
        FoulSeverity::Reckless
    } else if aggression >= 40 {
        FoulSeverity::Hard
    } else {
        FoulSeverity::Soft
    }
}

#[cfg(test)]
mod commentary_detail_tests {
    use super::*;
    use crate::event::GoalContext;

    #[test]
    fn danger_band_thresholds() {
        assert_eq!(danger_band(40.0), DangerBand::Speculative);
        assert_eq!(danger_band(49.9), DangerBand::Speculative);
        assert_eq!(danger_band(50.0), DangerBand::Decent);
        assert_eq!(danger_band(55.0), DangerBand::Decent);
        assert_eq!(danger_band(67.9), DangerBand::Decent);
        assert_eq!(danger_band(68.0), DangerBand::BigChance);
        assert_eq!(danger_band(75.0), DangerBand::BigChance);
    }

    #[test]
    fn save_quality_thresholds() {
        assert_eq!(save_quality(40.0), SaveQuality::Routine);
        assert_eq!(save_quality(49.9), SaveQuality::Routine);
        assert_eq!(save_quality(50.0), SaveQuality::Strong);
        assert_eq!(save_quality(55.0), SaveQuality::Strong);
        assert_eq!(save_quality(67.9), SaveQuality::Strong);
        assert_eq!(save_quality(68.0), SaveQuality::WorldClass);
        assert_eq!(save_quality(75.0), SaveQuality::WorldClass);
    }

    #[test]
    fn foul_severity_thresholds() {
        assert_eq!(foul_severity(20), FoulSeverity::Soft);
        assert_eq!(foul_severity(39), FoulSeverity::Soft);
        assert_eq!(foul_severity(40), FoulSeverity::Hard);
        assert_eq!(foul_severity(55), FoulSeverity::Hard);
        assert_eq!(foul_severity(69), FoulSeverity::Hard);
        assert_eq!(foul_severity(70), FoulSeverity::Reckless);
        assert_eq!(foul_severity(85), FoulSeverity::Reckless);
    }

    fn make_test_player(id: &str, pos: crate::types::Position) -> crate::types::PlayerData {
        crate::types::PlayerData {
            id: id.to_string(),
            name: id.to_string(),
            position: pos,
            ovr: 70,
            condition: 90,
            fitness: 75,
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
        }
    }

    fn make_test_state() -> LiveMatchState {
        use crate::types::{MatchConfig, PlayStyle, Position, TeamData};
        let make_team = |id: &str| TeamData {
            id: id.to_string(),
            name: id.to_string(),
            formation: "4-4-2".to_string(),
            play_style: PlayStyle::Balanced,
            players: vec![
                make_test_player(&format!("{}_gk", id), Position::Goalkeeper),
                make_test_player(&format!("{}_d1", id), Position::Defender),
                make_test_player(&format!("{}_d2", id), Position::Defender),
                make_test_player(&format!("{}_d3", id), Position::Defender),
                make_test_player(&format!("{}_d4", id), Position::Defender),
                make_test_player(&format!("{}_m1", id), Position::Midfielder),
                make_test_player(&format!("{}_m2", id), Position::Midfielder),
                make_test_player(&format!("{}_m3", id), Position::Midfielder),
                make_test_player(&format!("{}_m4", id), Position::Midfielder),
                make_test_player(&format!("{}_f1", id), Position::Forward),
                make_test_player(&format!("{}_f2", id), Position::Forward),
            ],
        };
        LiveMatchState::new(
            make_team("home"),
            make_team("away"),
            MatchConfig::default(),
            vec![],
            vec![],
            false,
        )
    }

    #[test]
    fn goal_context_classifies_correctly() {
        let mut state = make_test_state();
        // 0-0, Home about to score -> Opener
        state.home_score = 0;
        state.away_score = 0;
        assert_eq!(state.goal_context(Side::Home), GoalContext::Opener);
        // 0-1, Home about to score -> Equaliser (0+1 == 1)
        state.home_score = 0;
        state.away_score = 1;
        assert_eq!(state.goal_context(Side::Home), GoalContext::Equaliser);
        // 1-0, Home about to score -> Extends (1+1 > 0)
        state.home_score = 1;
        state.away_score = 0;
        assert_eq!(state.goal_context(Side::Home), GoalContext::Extends);
        // 0-2, Home about to score -> Consolation (0+1 < 2)
        state.home_score = 0;
        state.away_score = 2;
        assert_eq!(state.goal_context(Side::Home), GoalContext::Consolation);
        // Away-side flip: 1-0, Away about to score -> Equaliser
        state.home_score = 1;
        state.away_score = 0;
        assert_eq!(state.goal_context(Side::Away), GoalContext::Equaliser);
    }

    /// Once any goal has been scored — including a penalty, which reaches the
    /// scoreboard through `add_goal` exactly like an open-play goal — no later
    /// goal can be an `Opener`. This pins the cross-path invariant that
    /// `first_goal_detail_is_opener` depends on: that test deliberately skips
    /// seeds where a `PenaltyGoal` scores first, so the penalty-before-goal
    /// case is verified here instead.
    #[test]
    fn goal_after_a_prior_goal_is_never_opener() {
        let mut state = make_test_state();
        // Simulate a converted penalty for the home side via the real scoring API.
        state.add_goal(Side::Home);
        assert_eq!(state.home_score, 1);

        // The scoreboard has moved, so neither side's next goal opens the scoring.
        assert_ne!(state.goal_context(Side::Home), GoalContext::Opener);
        assert_ne!(state.goal_context(Side::Away), GoalContext::Opener);
        // Specifically: home extends the lead, away equalises.
        assert_eq!(state.goal_context(Side::Home), GoalContext::Extends);
        assert_eq!(state.goal_context(Side::Away), GoalContext::Equaliser);
    }
}
