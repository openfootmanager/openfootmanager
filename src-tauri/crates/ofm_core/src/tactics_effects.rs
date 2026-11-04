//! Which tactics controls change a match, for the screens to say so: one answer, from the engine
//! where the engine can give it. A control the engine does not read yet is reported here until the
//! slice that wires it (`drafts/tactics-match/wire-unread-controls.md`), which flips it in the same
//! change.

use domain::player::{Player, Position as DomainPosition};
use domain::team::PlayerRole;
use engine::{Position, TacticalDial};

use crate::turn::squad::{domain_to_engine_role, line_familiarity};

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
        TacticsControl::Role { role, slot } => {
            engine::role_has_match_effect(domain_to_engine_role(role), engine_line(slot))
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
/// it or is out of position. The match's own figure (`turn::squad::familiarity`), so a screen that
/// shows what he plays like there multiplies his natural rating by it and derives nothing itself.
pub fn match_factor(player: &Player, slot: &DomainPosition) -> f64 {
    let line = match engine_line(slot) {
        Position::Goalkeeper => 0,
        Position::Defender => 1,
        Position::Midfielder => 2,
        Position::Forward => 3,
    };
    line_familiarity(player, Some(slot))[line]
}

/// The engine line a slot plays in.
fn engine_line(slot: &DomainPosition) -> Position {
    match slot.to_group_position() {
        DomainPosition::Goalkeeper => Position::Goalkeeper,
        DomainPosition::Defender => Position::Defender,
        DomainPosition::Forward => Position::Forward,
        _ => Position::Midfielder,
    }
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
            (PlayerRole::PressingForward, DomainPosition::Striker),
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

    /// Given a natural centre-back,
    /// When the screen asks what he plays at at centre-back, at right-back and up front,
    /// Then it is all of him, a position he can play (1 - 0.10 / 3), and out of position (0.90),
    /// the factors the match charges.
    #[test]
    fn the_match_factor_is_what_the_match_charges_in_the_slot() {
        let centre_back = Player::new(
            "p".into(),
            "P".into(),
            "Player".into(),
            "1995-01-01".into(),
            "ENG".into(),
            DomainPosition::CenterBack,
            uniform_attributes(65),
        );
        assert_eq!(match_factor(&centre_back, &DomainPosition::CenterBack), 1.0);
        let at_right_back = match_factor(&centre_back, &DomainPosition::RightBack);
        assert!((at_right_back - (1.0 - 0.10 / 3.0)).abs() < 1e-12);
        assert_eq!(match_factor(&centre_back, &DomainPosition::Striker), 0.90);
    }
}
