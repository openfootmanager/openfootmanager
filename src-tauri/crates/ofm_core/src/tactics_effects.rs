//! Which tactics controls change a match, for the screens to say so: one answer, from the engine
//! where the engine can give it. A control counts when a resolver reads it or the engine's AI does
//! (it chooses by it in a match it manages). A control nothing reads yet is reported here as having
//! no effect until the slice of the tactics wiring plan that wires it, which flips it in the same
//! change.

use domain::player::{Player, Position as DomainPosition};
use domain::team::PlayerRole;
use engine::TacticalDial;

use crate::turn::squad::{domain_to_engine_role, engine_line, slot_factor};

/// A control on the tactics screen.
#[derive(Debug, Clone, PartialEq)]
pub enum TacticsControl {
    /// A role for the player in a slot: whether it counts depends on the slot's line.
    Role {
        role: PlayerRole,
        slot: DomainPosition,
    },
    /// One setting of one of the nine dials.
    Dial(TacticalDial),
    Captain,
    ViceCaptain,
    PenaltyTaker,
    FreeKickTaker,
    CornerTaker,
    /// The half-time team talk.
    TeamTalk,
}

/// Whether the match plays differently for this control.
pub fn has_match_effect(control: &TacticsControl) -> bool {
    match control {
        // A role the slot does not admit is played as Standard (`turn::squad`), so it changes
        // nothing: only a role the slot admits can count.
        TacticsControl::Role { role, slot } => {
            slot.admits_role(role)
                && engine::role_has_match_effect(domain_to_engine_role(role), engine_line(slot))
        }
        TacticsControl::Dial(dial) => dial.has_match_effect(),
        TacticsControl::PenaltyTaker => true,
        // Read by nothing in the engine yet: slices 10 to 12 of the wiring plan.
        TacticsControl::Captain
        | TacticsControl::ViceCaptain
        | TacticsControl::FreeKickTaker
        | TacticsControl::CornerTaker
        | TacticsControl::TeamTalk => false,
    }
}

/// The share of his match rating `player` plays at in `slot`: 1.0 at home, less where he can play
/// it or is out of position. The match's own figure, so a screen that shows what he plays like
/// there multiplies his natural rating by it and derives nothing itself.
pub fn match_factor(player: &Player, slot: &DomainPosition) -> f64 {
    slot_factor(player, slot)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::uniform_attributes;
    use engine::{BreakSpeed, MarkingStyle, TacticsBuildUpStyle};

    fn role(role: PlayerRole, slot: DomainPosition) -> bool {
        has_match_effect(&TacticsControl::Role { role, slot })
    }

    /// Given the roles the engine's resolvers read today,
    /// When the screen asks about them in their slots,
    /// Then each changes the match.
    #[test]
    fn a_role_the_engine_reads_in_its_slot_changes_the_match() {
        assert!(role(PlayerRole::Poacher, DomainPosition::Striker));
        assert!(role(PlayerRole::Stopper, DomainPosition::CenterBack));
        assert!(role(
            PlayerRole::DeepLyingPlaymaker,
            DomainPosition::DefensiveMidfielder
        ));
        assert!(role(
            PlayerRole::InvertedWinger,
            DomainPosition::RightMidfielder
        ));
        assert!(role(PlayerRole::InsideForward, DomainPosition::LeftWinger));
        assert!(role(PlayerRole::PressingForward, DomainPosition::Striker));
    }

    /// Given a Pressing Forward, whom the press reads up front and whom the AI's substitutions
    /// prefer for a side that presses,
    /// When the screen asks about him in each line,
    /// Then he changes the match up front and nowhere else.
    #[test]
    fn a_pressing_forward_counts_up_front_and_nowhere_else() {
        assert!(role(PlayerRole::PressingForward, DomainPosition::Striker));
        assert!(!role(
            PlayerRole::PressingForward,
            DomainPosition::AttackingMidfielder
        ));
        assert!(!role(
            PlayerRole::PressingForward,
            DomainPosition::CenterBack
        ));
    }

    /// Given a Poacher, whom the resolvers read (and penalise) in the back line,
    /// When the screen asks about him at centre-back and on the wing, slots that do not admit a
    /// Poacher,
    /// Then he does not change the match there: the match plays the role as Standard.
    #[test]
    fn a_role_the_slot_does_not_admit_has_no_match_effect() {
        assert!(!role(PlayerRole::Poacher, DomainPosition::CenterBack));
        assert!(!role(PlayerRole::Poacher, DomainPosition::LeftWinger));
        assert!(role(PlayerRole::Poacher, DomainPosition::Striker));
    }

    /// Given a role a resolver reads in one line,
    /// When the screen asks about it in a slot of each line,
    /// Then it counts where its line is consulted and not in the lines it is not.
    #[test]
    fn a_role_counts_in_the_line_of_the_slot_it_is_played_in() {
        assert!(role(PlayerRole::Stopper, DomainPosition::CenterBack));
        assert!(!role(PlayerRole::Stopper, DomainPosition::Goalkeeper));
        assert!(!role(PlayerRole::Stopper, DomainPosition::Striker));
        assert!(role(PlayerRole::Poacher, DomainPosition::Striker));
        assert!(!role(PlayerRole::Poacher, DomainPosition::Goalkeeper));
    }

    /// Given the roles no resolver reads in these slots today,
    /// When the screen asks about them,
    /// Then none changes the match, until the slice that wires it.
    #[test]
    fn a_role_no_resolver_reads_in_its_slot_has_no_match_effect() {
        for (role_, slot) in [
            (PlayerRole::SweeperKeeper, DomainPosition::Goalkeeper),
            (PlayerRole::BallPlayingKeeper, DomainPosition::Goalkeeper),
            (PlayerRole::BallPlayingCB, DomainPosition::CenterBack),
            (PlayerRole::AttackingFB, DomainPosition::RightBack),
            (PlayerRole::WingBack, DomainPosition::LeftWingBack),
            (PlayerRole::InvertedFB, DomainPosition::LeftBack),
            (PlayerRole::Mezzala, DomainPosition::CentralMidfielder),
            (
                PlayerRole::AdvancedPlaymaker,
                DomainPosition::AttackingMidfielder,
            ),
            (
                PlayerRole::ShadowStriker,
                DomainPosition::AttackingMidfielder,
            ),
            (PlayerRole::DeepLyingForward, DomainPosition::Striker),
            (PlayerRole::InvertedWinger, DomainPosition::RightWinger),
            (PlayerRole::WideForward, DomainPosition::LeftMidfielder),
        ] {
            assert!(!role(role_.clone(), slot.clone()), "{role_:?} at {slot:?}");
        }
    }

    /// Given the Standard role in any slot,
    /// When the screen asks about it,
    /// Then it counts: it is the baseline every other role is measured from.
    #[test]
    fn the_standard_role_counts_in_every_slot() {
        assert!(role(PlayerRole::Standard, DomainPosition::Goalkeeper));
        assert!(role(PlayerRole::Standard, DomainPosition::Striker));
    }

    /// Given the dials,
    /// When the screen asks about their settings,
    /// Then only Slow break speed plays like another setting; Long build-up and man marking
    /// change the match, if only by costing.
    #[test]
    fn only_slow_break_speed_has_no_match_effect() {
        assert!(!has_match_effect(&TacticsControl::Dial(
            TacticalDial::BreakSpeed(BreakSpeed::Slow)
        )));
        assert!(has_match_effect(&TacticsControl::Dial(
            TacticalDial::BreakSpeed(BreakSpeed::Fast)
        )));
        assert!(has_match_effect(&TacticsControl::Dial(
            TacticalDial::BuildUpStyle(TacticsBuildUpStyle::Long)
        )));
        assert!(has_match_effect(&TacticsControl::Dial(
            TacticalDial::MarkingStyle(MarkingStyle::ManToMan)
        )));
    }

    /// Given the match roles and the team talk,
    /// When the screen asks about them,
    /// Then the penalty taker changes the match and the rest do not yet.
    #[test]
    fn only_the_penalty_taker_of_the_match_roles_changes_the_match() {
        assert!(has_match_effect(&TacticsControl::PenaltyTaker));
        for control in [
            TacticsControl::Captain,
            TacticsControl::ViceCaptain,
            TacticsControl::FreeKickTaker,
            TacticsControl::CornerTaker,
            TacticsControl::TeamTalk,
        ] {
            assert!(!has_match_effect(&control), "{control:?}");
        }
    }

    fn natural(position: DomainPosition) -> Player {
        Player::new(
            "p".into(),
            "P".into(),
            "Player".into(),
            "1995-01-01".into(),
            "ENG".into(),
            position,
            uniform_attributes(65),
        )
    }

    const CAN_PLAY: f64 = 1.0 - 0.10 / 3.0;

    /// Given a natural centre-back,
    /// When the screen asks what he plays at at centre-back, at right-back and up front,
    /// Then it is all of him, a position he can play (1 - 0.10 / 3), and out of position (0.90),
    /// the factors the match charges.
    #[test]
    fn the_match_factor_is_what_the_match_charges_in_the_slot() {
        let centre_back = natural(DomainPosition::CenterBack);
        assert_eq!(match_factor(&centre_back, &DomainPosition::CenterBack), 1.0);
        let at_right_back = match_factor(&centre_back, &DomainPosition::RightBack);
        assert!((at_right_back - CAN_PLAY).abs() < 1e-12);
        assert_eq!(match_factor(&centre_back, &DomainPosition::Striker), 0.90);
    }

    /// Given a natural goalkeeper,
    /// When the screen asks what he plays at in goal and in the other lines,
    /// Then it is all of him in goal and 0.90 outfield; and an outfielder in goal plays at 0.90.
    #[test]
    fn the_match_factor_in_goal_is_all_of_a_goalkeeper_and_a_tenth_less_for_an_outfielder() {
        let goalkeeper = natural(DomainPosition::Goalkeeper);
        assert_eq!(match_factor(&goalkeeper, &DomainPosition::Goalkeeper), 1.0);
        assert_eq!(match_factor(&goalkeeper, &DomainPosition::Striker), 0.90);
        let midfielder = natural(DomainPosition::CentralMidfielder);
        assert_eq!(match_factor(&midfielder, &DomainPosition::Goalkeeper), 0.90);
    }

    /// Given a natural central midfielder and a natural striker,
    /// When the screen asks what each plays at in a slot of his own line, and of another,
    /// Then the same line is all of him or a position he can play, and the other line 0.90.
    #[test]
    fn the_match_factor_in_midfield_and_up_front_follows_the_line_of_the_slot() {
        let midfielder = natural(DomainPosition::CentralMidfielder);
        assert_eq!(
            match_factor(&midfielder, &DomainPosition::CentralMidfielder),
            1.0
        );
        let at_attacking_midfield = match_factor(&midfielder, &DomainPosition::AttackingMidfielder);
        assert!((at_attacking_midfield - CAN_PLAY).abs() < 1e-12);
        assert_eq!(match_factor(&midfielder, &DomainPosition::Striker), 0.90);

        let striker = natural(DomainPosition::Striker);
        assert_eq!(match_factor(&striker, &DomainPosition::Striker), 1.0);
        let on_the_wing = match_factor(&striker, &DomainPosition::LeftWinger);
        assert!((on_the_wing - CAN_PLAY).abs() < 1e-12);
        assert_eq!(
            match_factor(&striker, &DomainPosition::CentralMidfielder),
            0.90
        );
    }
}
