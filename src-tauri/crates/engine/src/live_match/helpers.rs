use rand::Rng;

use crate::event::{DangerBand, FoulSeverity, GoalContext, SaveQuality};
use crate::shared::{
    PlayStylePhase, PlayerSnap, home_mod, play_style_modifier, tactics_pressing_fatigue,
    tactics_pressing_press,
};
use crate::types::{PlayerData, Position, Side, TeamData};

/// The ground each missing player adds to what the rest of a side must cover when it defends.
const SHORT_HANDED_COST: f64 = 0.7;

use super::{LiveMatchState, SetPieceTakers};

// ---------------------------------------------------------------------------
// Stamina system
// ---------------------------------------------------------------------------

impl LiveMatchState {
    pub(super) fn deplete_stamina_tick(&mut self) {
        let base_rate = self.config.fatigue_per_minute;
        // Aggressive pressing tires a side faster; neutral (Medium) is ×1.0.
        let home_rate = base_rate * tactics_pressing_fatigue(&self.home.tactics);
        let away_rate = base_rate * tactics_pressing_fatigue(&self.away.tactics);
        // Iterate over all on-pitch players, each with their team's fatigue rate.
        let players = self
            .home
            .players
            .iter()
            .map(|p| (p, home_rate))
            .chain(self.away.players.iter().map(|p| (p, away_rate)));
        for (p, fatigue_rate) in players {
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
        crate::shared::snap_from_squad(&team.players, &self.sent_off, preferred, rng)
            .unwrap_or_else(PlayerSnap::nobody)
    }

    pub(super) fn snap_player_by_id(&self, player_id: &str, side: Side) -> PlayerSnap {
        let team = self.team_ref(side);
        team.players
            .iter()
            .find(|player| player.id == player_id)
            .or_else(|| team.players.first())
            .map(PlayerSnap::from)
            .unwrap_or_else(PlayerSnap::nobody)
    }

    pub(super) fn pick_goalkeeper(&self, side: Side) -> PlayerSnap {
        if let Some(keeper) = self.keeper_on_the_pitch(side) {
            return PlayerSnap::from(keeper);
        }
        let team = self.team_ref(side);
        // No goalkeeper available — pick first available
        for p in &team.players {
            if !self.sent_off.contains(&p.id) {
                return PlayerSnap::from(p);
            }
        }
        // Everyone is sent off, or there is nobody at all. `create_live_match`
        // refuses a side with no players, so the latter should not reach here —
        // but a blank name in an event is a bug worth reporting, and indexing
        // an empty squad is a window that stops responding.
        team.players
            .first()
            .map(PlayerSnap::from)
            .unwrap_or_else(PlayerSnap::nobody)
    }

    // -----------------------------------------------------------------------
    // Rating helpers
    // -----------------------------------------------------------------------

    pub(super) fn effective_midfield(&self, side: Side) -> f64 {
        let base = self.team_ref(side).midfield_rating_on_the_pitch(
            &|id| !self.is_on_the_pitch(side, id),
            self.midfield_slots(side),
        );
        let modifier = play_style_modifier(
            self.team_ref(side).play_style,
            PlayStylePhase::Midfield,
            true,
        );
        base * modifier * home_mod(side, &self.config)
    }

    pub(super) fn effective_press(&self, pressing_side: Side) -> f64 {
        let team = self.team_ref(pressing_side);
        let base = team.line_strength(
            Position::Midfielder,
            |p| ((p.stamina as u16 + p.tackling as u16 + p.pace as u16) / 3) as u8,
            &|id| !self.is_on_the_pitch(pressing_side, id),
            self.midfield_slots(pressing_side),
        );
        let modifier = play_style_modifier(team.play_style, PlayStylePhase::Press, true);
        base * modifier
            * self.short_handed_cover(pressing_side)
            * tactics_pressing_press(&team.tactics)
            * home_mod(pressing_side, &self.config)
    }

    /// The midfield slots in `side`'s current shape.
    fn midfield_slots(&self, side: Side) -> usize {
        Self::parse_formation(&self.team_ref(side).formation).1
    }

    /// How well a side short of players still covers the ground when it defends: each player it
    /// is missing adds `SHORT_HANDED_COST` to the ground the rest must cover. It weakens the
    /// side's press and tackles and leaves the chances against it cleaner; its attacking is left
    /// alone, as the literature finds a side down to ten concedes far more but scores about as
    /// often (`tests/short_handed_tests.rs`).
    pub(super) fn short_handed_cover(&self, side: Side) -> f64 {
        let missing = self
            .team_ref(side)
            .players
            .iter()
            .filter(|p| !self.is_on_the_pitch(side, &p.id))
            .count();
        1.0 / (1.0 + SHORT_HANDED_COST * missing as f64)
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
    /// This side's goals, then the opponent's.
    pub(crate) fn score_for(&self, side: Side) -> (u8, u8) {
        match side {
            Side::Home => (self.home_score, self.away_score),
            Side::Away => (self.away_score, self.home_score),
        }
    }

    /// Whether `player_id` is on `side`'s pitch: in the side's players and not sent off. A
    /// substituted-off player is no longer in the side's players. The one test for being on the
    /// pitch: a player who has left it leaves an empty slot in their line and their side
    /// short-handed, so any other way of leaving it belongs here.
    pub(super) fn is_on_the_pitch(&self, side: Side, player_id: &str) -> bool {
        !self.sent_off.contains(player_id)
            && self
                .team_ref(side)
                .players
                .iter()
                .any(|player| player.id == player_id)
    }

    /// The goalkeeper this side still has on the pitch: one who has not been
    /// sent off. The one answer to "is anybody in goal", for choosing who
    /// faces a shot, for putting a keeper on, and for the AI deciding it must.
    pub(crate) fn keeper_on_the_pitch(&self, side: Side) -> Option<&PlayerData> {
        self.team_ref(side)
            .players
            .iter()
            .find(|p| p.position == Position::Goalkeeper && !self.sent_off.contains(&p.id))
    }

    pub(super) fn goal_context(&self, side: Side) -> GoalContext {
        let (own, opp) = self.score_for(side);
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
            injury_risk: 1.0,
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
            role: crate::types::PlayerRole::Standard,
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
            tactics: crate::types::TacticsConfig::default(),
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

#[cfg(test)]
mod empty_squad_tests {
    use crate::live_match::LiveMatchState;
    use crate::types::{MatchConfig, PlayStyle, Side, TacticsConfig, TeamData};

    fn empty_team(id: &str) -> TeamData {
        TeamData {
            id: id.to_string(),
            name: id.to_string(),
            formation: "4-4-2".to_string(),
            play_style: PlayStyle::Balanced,
            tactics: TacticsConfig::default(),
            players: vec![],
        }
    }

    /// The batch engine refuses a match with an empty side, but the live,
    /// spectator and delegated paths build `LiveMatchState` directly and never
    /// reach that guard. Goalkeeper selection ended by indexing `players[0]`,
    /// so a shootout with an empty XI panicked out of the Tauri command and
    /// left the window with no response.
    #[test]
    fn picking_a_goalkeeper_from_nobody_does_not_panic() {
        let state = LiveMatchState::new(
            empty_team("home"),
            empty_team("away"),
            MatchConfig::default(),
            vec![],
            vec![],
            true,
        );

        let keeper = state.pick_goalkeeper(Side::Home);

        assert!(keeper.id.is_empty(), "nobody is available to keep goal");
    }
}

#[cfg(test)]
mod short_handed_line_tests {
    use crate::live_match::LiveMatchState;
    use crate::shared::{PlayStylePhase, home_mod, play_style_modifier, tactics_pressing_press};
    use crate::types::{
        MatchConfig, PlayStyle, PlayerData, PlayerRole, Position, Side, TacticsConfig, TeamData,
    };

    fn player(id: &str, position: Position) -> PlayerData {
        PlayerData {
            id: id.to_string(),
            name: id.to_string(),
            position,
            ovr: 70,
            condition: 100,
            fitness: 100,
            injury_risk: 1.0,
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

    /// A 4-4-2 whose midfielders, `{prefix}_m1` .. `{prefix}_m4`, have stamina 60, tackling 70
    /// and pace 80, so a press that read the wrong mix of them would not come to 70.
    fn side(prefix: &str) -> TeamData {
        let mut players = vec![player(&format!("{prefix}_gk"), Position::Goalkeeper)];
        players.extend((1..=4).map(|n| player(&format!("{prefix}_d{n}"), Position::Defender)));
        players.extend((1..=4).map(|n| {
            let mut midfielder = player(&format!("{prefix}_m{n}"), Position::Midfielder);
            midfielder.stamina = 60;
            midfielder.pace = 80;
            midfielder
        }));
        players.extend((1..=2).map(|n| player(&format!("{prefix}_f{n}"), Position::Forward)));
        TeamData {
            id: prefix.to_string(),
            name: prefix.to_string(),
            formation: "4-4-2".to_string(),
            play_style: PlayStyle::Balanced,
            tactics: TacticsConfig::default(),
            players,
        }
    }

    fn kick_off() -> LiveMatchState {
        LiveMatchState::new(
            side("h"),
            side("a"),
            MatchConfig::default(),
            vec![],
            vec![],
            false,
        )
    }

    /// Given a full side whose midfielders have stamina 60, tackling 70 and pace 80,
    /// When its press is read,
    /// Then it is the midfield's average of the three, 70, times the side's press modifiers.
    #[test]
    fn the_press_reads_its_midfielders_stamina_tackling_and_pace() {
        let state = kick_off();
        let modifiers = play_style_modifier(PlayStyle::Balanced, PlayStylePhase::Press, true)
            * tactics_pressing_press(&TacticsConfig::default())
            * home_mod(Side::Home, &MatchConfig::default());
        assert!((state.effective_press(Side::Home) - 70.0 * modifiers).abs() < 1e-9);
    }

    /// Given a full four-man midfield,
    /// When one midfielder is sent off,
    /// Then the side's midfield is three quarters of what it was: his slot is empty.
    #[test]
    fn a_midfielder_sent_off_leaves_the_midfield_three_quarters_strong() {
        let mut state = kick_off();
        let full = state.effective_midfield(Side::Home);
        state.test_send_off("h_m1");
        let ratio = state.effective_midfield(Side::Home) / full;
        assert!(
            (ratio - 0.75).abs() < 1e-12,
            "the midfield kept {ratio} of its strength"
        );
    }

    /// Given a full four-man midfield,
    /// When one midfielder is sent off,
    /// Then the side's press keeps three quarters for his empty slot, and of that the cover a
    /// side one short keeps, 1 / (1 + 0.7).
    #[test]
    fn a_midfielder_sent_off_costs_the_press_his_slot_and_the_sides_cover() {
        let mut state = kick_off();
        let full = state.effective_press(Side::Home);
        state.test_send_off("h_m1");
        let ratio = state.effective_press(Side::Home) / full;
        assert!(
            (ratio - 0.75 / 1.7).abs() < 1e-12,
            "the press kept {ratio} of its strength"
        );
    }
}
