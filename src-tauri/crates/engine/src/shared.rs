use crate::types::{
    BreakSpeed, CounterPressDuration, DefensiveLine, DefensiveShape, MarkingStyle, MatchConfig,
    PlayStyle, PlayerData, PlayerRole, PressingIntensity, Side, TacticsBuildUpStyle, TacticsConfig,
    TacticsPitchWidth, Tempo,
};

// ---------------------------------------------------------------------------
// PlayerSnap — lightweight snapshot of a player to avoid borrow conflicts
// ---------------------------------------------------------------------------

#[derive(Clone)]
#[allow(dead_code)]
pub(crate) struct PlayerSnap {
    pub id: String,
    pub pace: u8,
    pub stamina: u8,
    pub strength: u8,
    pub agility: u8,
    pub passing: u8,
    pub shooting: u8,
    pub tackling: u8,
    pub dribbling: u8,
    pub defending: u8,
    pub positioning: u8,
    pub vision: u8,
    pub decisions: u8,
    pub composure: u8,
    pub aggression: u8,
    pub teamwork: u8,
    pub leadership: u8,
    pub handling: u8,
    pub reflexes: u8,
    pub aerial: u8,
    pub traits: Vec<String>,
    pub role: PlayerRole,
}

impl PlayerSnap {
    pub fn from(p: &PlayerData) -> Self {
        Self {
            id: p.id.clone(),
            pace: p.pace,
            stamina: p.stamina,
            strength: p.strength,
            agility: p.agility,
            passing: p.passing,
            shooting: p.shooting,
            tackling: p.tackling,
            dribbling: p.dribbling,
            defending: p.defending,
            positioning: p.positioning,
            vision: p.vision,
            decisions: p.decisions,
            composure: p.composure,
            aggression: p.aggression,
            teamwork: p.teamwork,
            leadership: p.leadership,
            handling: p.handling,
            reflexes: p.reflexes,
            aerial: p.aerial,
            traits: p.traits.clone(),
            role: p.role,
        }
    }

    pub fn has_trait(&self, name: &str) -> bool {
        self.traits.iter().any(|t| t == name)
    }
}

// ---------------------------------------------------------------------------
// TraitContext — which game action context we're computing a bonus for
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub(crate) enum TraitContext {
    Shooting,
    Dribbling,
    Passing,
    Tackling,
    Goalkeeping,
    Foul,
    Midfield,
}

/// Compute a multiplicative trait bonus for a specific action context.
/// Returns a modifier >= 1.0 (bonus) based on relevant traits.
pub(crate) fn trait_bonus(snap: &PlayerSnap, context: TraitContext) -> f64 {
    let mut bonus = 1.0;
    match context {
        TraitContext::Shooting => {
            if snap.has_trait("Sharpshooter") {
                bonus *= 1.08;
            }
            if snap.has_trait("CoolHead") {
                bonus *= 1.04;
            }
            if snap.has_trait("CompleteForward") {
                bonus *= 1.05;
            }
        }
        TraitContext::Dribbling => {
            if snap.has_trait("Dribbler") {
                bonus *= 1.08;
            }
            if snap.has_trait("Speedster") {
                bonus *= 1.04;
            }
            if snap.has_trait("Agile") {
                bonus *= 1.04;
            }
        }
        TraitContext::Passing => {
            if snap.has_trait("Playmaker") {
                bonus *= 1.08;
            }
            if snap.has_trait("Visionary") {
                bonus *= 1.05;
            }
            if snap.has_trait("SetPieceSpecialist") {
                bonus *= 1.03;
            }
        }
        TraitContext::Tackling => {
            if snap.has_trait("BallWinner") {
                bonus *= 1.08;
            }
            if snap.has_trait("Rock") {
                bonus *= 1.05;
            }
            if snap.has_trait("Tank") {
                bonus *= 1.04;
            }
        }
        TraitContext::Goalkeeping => {
            if snap.has_trait("SafeHands") {
                bonus *= 1.08;
            }
            if snap.has_trait("CatReflexes") {
                bonus *= 1.06;
            }
            if snap.has_trait("AerialDominance") {
                bonus *= 1.04;
            }
        }
        TraitContext::Foul => {
            if snap.has_trait("HotHead") {
                bonus *= 1.25;
            }
            if snap.has_trait("CoolHead") {
                bonus *= 0.70;
            }
        }
        TraitContext::Midfield => {
            if snap.has_trait("Engine") {
                bonus *= 1.06;
            }
            if snap.has_trait("TeamPlayer") {
                bonus *= 1.04;
            }
            if snap.has_trait("Tireless") {
                bonus *= 1.03;
            }
        }
    }
    bonus
}

// ---------------------------------------------------------------------------
// Play-style modifiers
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub(crate) enum PlayStylePhase {
    Midfield,
    Attack,
    Defense,
    Press,
}

pub(crate) fn play_style_modifier(
    style: PlayStyle,
    phase: PlayStylePhase,
    is_own_phase: bool,
) -> f64 {
    if !is_own_phase {
        return 1.0;
    }
    match (style, phase) {
        (PlayStyle::Attacking, PlayStylePhase::Attack) => 1.12,
        (PlayStyle::Attacking, PlayStylePhase::Defense) => 0.93,
        (PlayStyle::Defensive, PlayStylePhase::Defense) => 1.12,
        (PlayStyle::Defensive, PlayStylePhase::Attack) => 0.93,
        (PlayStyle::Possession, PlayStylePhase::Midfield) => 1.15,
        (PlayStyle::Possession, PlayStylePhase::Attack) => 0.97,
        (PlayStyle::Counter, PlayStylePhase::Attack) => 1.18,
        (PlayStyle::Counter, PlayStylePhase::Midfield) => 0.92,
        (PlayStyle::HighPress, PlayStylePhase::Press) => 1.20,
        (PlayStyle::HighPress, PlayStylePhase::Defense) => 0.95,
        _ => 1.0,
    }
}

// ---------------------------------------------------------------------------
// Role attribute modifier — applied per-player during zone resolution
// ---------------------------------------------------------------------------

/// Returns a multiplier (0.88–1.20) applied to the player's effective skill
/// calculation based on their assigned tactical role. Values reflect the
/// attribute biases described in domain::team::PlayerRole documentation.
pub(crate) fn role_attribute_modifier(role: PlayerRole, phase: PlayStylePhase) -> f64 {
    match (role, phase) {
        // Goalkeepers
        (PlayerRole::SweeperKeeper, PlayStylePhase::Defense) => 1.06,
        (PlayerRole::BallPlayingKeeper, PlayStylePhase::Midfield) => 1.06,
        // Center Backs
        (PlayerRole::Stopper, PlayStylePhase::Defense) => 1.08,
        (PlayerRole::BallPlayingCB, PlayStylePhase::Midfield) => 1.05,
        (PlayerRole::CoverCB, PlayStylePhase::Defense) => 1.05,
        // Full Backs
        (PlayerRole::AttackingFB, PlayStylePhase::Attack) => 1.08,
        (PlayerRole::AttackingFB, PlayStylePhase::Defense) => 0.93,
        (PlayerRole::DefensiveFB, PlayStylePhase::Defense) => 1.08,
        (PlayerRole::DefensiveFB, PlayStylePhase::Attack) => 0.93,
        (PlayerRole::WingBack, PlayStylePhase::Attack) => 1.10,
        (PlayerRole::WingBack, PlayStylePhase::Defense) => 0.97,
        (PlayerRole::InvertedFB, PlayStylePhase::Midfield) => 1.06,
        // Defensive Midfielders
        (PlayerRole::AnchorMan, PlayStylePhase::Defense) => 1.10,
        (PlayerRole::AnchorMan, PlayStylePhase::Attack) => 0.90,
        (PlayerRole::BallWinner, PlayStylePhase::Defense) => 1.08,
        (PlayerRole::DeepLyingPlaymaker, PlayStylePhase::Midfield) => 1.10,
        (PlayerRole::DeepLyingPlaymaker, PlayStylePhase::Attack) => 0.93,
        // Central Midfielders
        (PlayerRole::BoxToBox, PlayStylePhase::Midfield) => 1.06,
        (PlayerRole::BoxToBox, PlayStylePhase::Attack) => 1.05,
        (PlayerRole::Mezzala, PlayStylePhase::Attack) => 1.08,
        (PlayerRole::Carrilero, PlayStylePhase::Defense) => 1.06,
        // Attacking Midfielders
        (PlayerRole::AdvancedPlaymaker, PlayStylePhase::Attack) => 1.10,
        (PlayerRole::ShadowStriker, PlayStylePhase::Attack) => 1.08,
        (PlayerRole::ShadowStriker, PlayStylePhase::Defense) => 0.92,
        // Wide
        (PlayerRole::WideForward, PlayStylePhase::Attack) => 1.08,
        (PlayerRole::InsideForward, PlayStylePhase::Attack) => 1.10,
        (PlayerRole::InvertedWinger, PlayStylePhase::Midfield) => 1.08,
        // Strikers
        (PlayerRole::Poacher, PlayStylePhase::Attack) => 1.12,
        (PlayerRole::Poacher, PlayStylePhase::Defense) => 0.85,
        (PlayerRole::TargetMan, PlayStylePhase::Attack) => 1.08,
        (PlayerRole::DeepLyingForward, PlayStylePhase::Midfield) => 1.06,
        (PlayerRole::False9, PlayStylePhase::Midfield) => 1.08,
        (PlayerRole::False9, PlayStylePhase::Attack) => 1.05,
        (PlayerRole::PressingForward, PlayStylePhase::Press) => 1.15,
        (PlayerRole::CompleteForward, PlayStylePhase::Attack) => 1.10,
        (PlayerRole::CompleteForward, PlayStylePhase::Defense) => 1.03,
        _ => 1.0,
    }
}

// ---------------------------------------------------------------------------
// Tactics modifiers — translate TacticsConfig settings to simulation multipliers
// ---------------------------------------------------------------------------

/// Foul rate multiplier from the defensive team's pressing + marking style.
pub(crate) fn tactics_foul_modifier(tactics: &TacticsConfig) -> f64 {
    let press = match tactics.pressing_intensity {
        PressingIntensity::Aggressive => 1.25,
        PressingIntensity::Passive => 0.80,
        PressingIntensity::Medium => 1.0,
    };
    let marking = match tactics.marking_style {
        MarkingStyle::ManToMan => 1.15,
        MarkingStyle::Mixed => 1.05,
        MarkingStyle::Zonal => 1.0,
    };
    press * marking
}

/// Cross attempt probability based on the attacking team's pitch width setting.
pub(crate) fn tactics_cross_probability(tactics: &TacticsConfig) -> f64 {
    match tactics.width {
        TacticsPitchWidth::Wide => 0.72,
        TacticsPitchWidth::Narrow => 0.45,
        TacticsPitchWidth::Normal => 0.60,
    }
}

/// Shot conversion multiplier from the defending team's defensive line depth.
/// High line = more space in behind = easier for attackers to score.
pub(crate) fn tactics_defensive_conversion_mod(tactics: &TacticsConfig) -> f64 {
    match tactics.defensive_line {
        DefensiveLine::High => 1.12,
        DefensiveLine::Low => 0.92,
        DefensiveLine::VeryLow => 0.85,
        DefensiveLine::Medium => 1.0,
    }
}

/// Build-up pass success modifier based on the attacking team's build-up style.
/// Short passing = safer in own half; Long ball = riskier.
pub(crate) fn tactics_buildup_mod(tactics: &TacticsConfig) -> f64 {
    match tactics.build_up_style {
        TacticsBuildUpStyle::Short => 1.08,
        TacticsBuildUpStyle::Long => 0.88,
        TacticsBuildUpStyle::Mixed => 1.0,
    }
}

// --- Extended phase dials (tempo / shape / pressing-possession / transitions) ---
//
// These cover dimensions the original five dials don't touch. Each neutral
// (#[default]) option returns ×1.0 — and the transition rolls return 0.0 — so a
// team on its defaults leaves the simulation (and the RNG stream) unchanged.
// build_up / width / def_line / marking are intentionally NOT re-hooked here:
// they already have live effects above, and re-hooking would double-count.

/// Tempo's progression side: Direct breaks through midfield faster, Patient is
/// more measured. Applied to the attacker's midfield contest.
pub(crate) fn tactics_tempo_progression(tactics: &TacticsConfig) -> f64 {
    match tactics.tempo {
        Tempo::Direct => 1.0,
        Tempo::Patient => 0.92,
    }
}

/// Tempo's retention side: Patient circulates and holds possession longer.
/// Applied to the possessing side's weight in the per-minute possession contest.
pub(crate) fn tactics_tempo_retention(tactics: &TacticsConfig) -> f64 {
    match tactics.tempo {
        Tempo::Patient => 1.03,
        Tempo::Direct => 1.0,
    }
}

/// Pressing's ball-winning side in the per-minute possession contest: harder
/// pressing recovers the ball more often. Applied to the defending side's weight.
pub(crate) fn tactics_pressing_contest(tactics: &TacticsConfig) -> f64 {
    match tactics.pressing_intensity {
        PressingIntensity::Passive => 0.97,
        PressingIntensity::Medium => 1.0,
        PressingIntensity::Aggressive => 1.05,
    }
}

/// Pressing scales the effectiveness of the press that opposes the opponent's
/// build-up (a higher press forces more build-up turnovers).
pub(crate) fn tactics_pressing_press(tactics: &TacticsConfig) -> f64 {
    match tactics.pressing_intensity {
        PressingIntensity::Passive => 0.96,
        PressingIntensity::Medium => 1.0,
        PressingIntensity::Aggressive => 1.06,
    }
}

/// Pressing's energy cost: aggressive pressing tires a side faster. Applies only
/// to the live engine, which tracks in-match condition.
pub(crate) fn tactics_pressing_fatigue(tactics: &TacticsConfig) -> f64 {
    match tactics.pressing_intensity {
        PressingIntensity::Passive => 0.96,
        PressingIntensity::Medium => 1.0,
        PressingIntensity::Aggressive => 1.08,
    }
}

/// Defensive shape scales how hard it is to create chances against the team.
/// Applied to the defender's rating in the attacking third.
pub(crate) fn tactics_shape_modifier(tactics: &TacticsConfig) -> f64 {
    match tactics.defensive_shape {
        DefensiveShape::Stretched => 0.93,
        DefensiveShape::Normal => 1.0,
        DefensiveShape::Compact => 1.07,
    }
}

/// Counter-press duration: chance for the side that just lost the ball to win it
/// straight back at the possession flip. None ⇒ no roll (neutral, RNG-safe).
pub(crate) fn tactics_counter_press_rewin(tactics: &TacticsConfig) -> f64 {
    match tactics.counter_press_duration {
        CounterPressDuration::None => 0.0,
        CounterPressDuration::Short => 0.06,
        CounterPressDuration::Long => 0.12,
    }
}

/// Break speed: chance for the side that just won the ball to spring a fast
/// counter into its attacking third instead of resetting to midfield. Neutral
/// (Medium/Slow) ⇒ no roll; only Fast enables counters.
pub(crate) fn tactics_break_speed_counter(tactics: &TacticsConfig) -> f64 {
    match tactics.break_speed {
        BreakSpeed::Slow => 0.0,
        BreakSpeed::Medium => 0.0,
        BreakSpeed::Fast => 0.10,
    }
}

// ---------------------------------------------------------------------------
// Home advantage modifier
// ---------------------------------------------------------------------------

pub(crate) fn home_mod(side: Side, config: &MatchConfig) -> f64 {
    match side {
        Side::Home => config.home_advantage,
        Side::Away => 1.0,
    }
}

#[cfg(test)]
mod phase_modifier_tests {
    use super::*;

    fn cfg(f: impl FnOnce(&mut TacticsConfig)) -> TacticsConfig {
        let mut c = TacticsConfig::default();
        f(&mut c);
        c
    }

    /// The load-bearing invariant: a default TacticsConfig must leave every new
    /// dial neutral (×1.0 for ratings, 0.0 for the probabilistic transitions),
    /// so default teams simulate byte-identically to the pre-dial engine.
    #[test]
    fn default_config_is_fully_neutral() {
        let d = TacticsConfig::default();
        assert_eq!(tactics_tempo_progression(&d), 1.0);
        assert_eq!(tactics_tempo_retention(&d), 1.0);
        assert_eq!(tactics_pressing_contest(&d), 1.0);
        assert_eq!(tactics_pressing_press(&d), 1.0);
        assert_eq!(tactics_pressing_fatigue(&d), 1.0);
        assert_eq!(tactics_shape_modifier(&d), 1.0);
        assert_eq!(tactics_counter_press_rewin(&d), 0.0);
        assert_eq!(tactics_break_speed_counter(&d), 0.0);
    }

    #[test]
    fn tempo_directions() {
        // Direct is neutral; Patient progresses slower but retains more.
        assert!(tactics_tempo_progression(&cfg(|c| c.tempo = Tempo::Patient)) < 1.0);
        assert_eq!(tactics_tempo_progression(&cfg(|c| c.tempo = Tempo::Direct)), 1.0);
        assert!(tactics_tempo_retention(&cfg(|c| c.tempo = Tempo::Patient)) > 1.0);
        assert_eq!(tactics_tempo_retention(&cfg(|c| c.tempo = Tempo::Direct)), 1.0);
    }

    #[test]
    fn pressing_directions_monotonic() {
        let passive = cfg(|c| c.pressing_intensity = PressingIntensity::Passive);
        let medium = cfg(|c| c.pressing_intensity = PressingIntensity::Medium);
        let aggressive = cfg(|c| c.pressing_intensity = PressingIntensity::Aggressive);
        for f in [
            tactics_pressing_contest,
            tactics_pressing_press,
            tactics_pressing_fatigue,
        ] {
            assert!(f(&passive) < f(&medium), "passive should be < medium");
            assert!(f(&medium) < f(&aggressive), "medium should be < aggressive");
            assert_eq!(f(&medium), 1.0, "medium must be neutral");
        }
    }

    #[test]
    fn shape_directions_monotonic() {
        let stretched = cfg(|c| c.defensive_shape = DefensiveShape::Stretched);
        let normal = cfg(|c| c.defensive_shape = DefensiveShape::Normal);
        let compact = cfg(|c| c.defensive_shape = DefensiveShape::Compact);
        assert!(tactics_shape_modifier(&stretched) < 1.0);
        assert_eq!(tactics_shape_modifier(&normal), 1.0);
        assert!(tactics_shape_modifier(&compact) > 1.0);
    }

    #[test]
    fn transition_dials_are_probabilities_with_neutral_zero() {
        // Counter-press: None rolls nothing; Long > Short > 0.
        assert_eq!(tactics_counter_press_rewin(&cfg(|c| c.counter_press_duration = CounterPressDuration::None)), 0.0);
        let short = tactics_counter_press_rewin(&cfg(|c| c.counter_press_duration = CounterPressDuration::Short));
        let long = tactics_counter_press_rewin(&cfg(|c| c.counter_press_duration = CounterPressDuration::Long));
        assert!(0.0 < short && short < long && long < 1.0);
        // Break speed: only Fast rolls; Slow and Medium are no-ops.
        assert_eq!(tactics_break_speed_counter(&cfg(|c| c.break_speed = BreakSpeed::Slow)), 0.0);
        assert_eq!(tactics_break_speed_counter(&cfg(|c| c.break_speed = BreakSpeed::Medium)), 0.0);
        let fast = tactics_break_speed_counter(&cfg(|c| c.break_speed = BreakSpeed::Fast));
        assert!(0.0 < fast && fast < 1.0);
    }
}
