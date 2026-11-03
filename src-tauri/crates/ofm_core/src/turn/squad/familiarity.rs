//! What playing out of position costs a player in a club match. The screens' familiarity words
//! (`player_rating::familiarity`) become factors on the player's match rating, one per engine
//! line, which the engine applies to the line the player is in.

use domain::player::{Player, Position};

use crate::player_rating::{Familiarity, familiarity};

/// The share of his match rating a player loses out of position (another line).
const OUT_OF_POSITION_COST: f64 = 0.10;

/// The share he loses at a position he can play (an alternate, or another position in his own
/// line): a third of the out-of-position cost.
const CAN_PLAY_COST: f64 = OUT_OF_POSITION_COST / 3.0;

/// The engine's lines, in the order of `engine::PlayerData::line_familiarity`.
const LINES: [Position; 4] = [
    Position::Goalkeeper,
    Position::Defender,
    Position::Midfielder,
    Position::Forward,
];

fn factor(familiarity: Familiarity) -> f64 {
    match familiarity {
        Familiarity::Natural => 1.0,
        Familiarity::CanPlay => 1.0 - CAN_PLAY_COST,
        Familiarity::OutOfPosition => 1.0 - OUT_OF_POSITION_COST,
    }
}

/// The factor `player` carries in each engine line. In the line of his kickoff slot it is that
/// slot's familiarity, so a right-back at left-back pays from the first minute. In any other line
/// it is his best familiarity over the line's positions: the engine moves players only between
/// lines (a substitution, a change of shape), never says where within one, and a manager moving
/// a player puts him where he fits best. A bench player has no kickoff slot.
pub(super) fn line_familiarity(player: &Player, kickoff_slot: Option<&Position>) -> [f64; 4] {
    LINES.map(|line| {
        if let Some(slot) = kickoff_slot.filter(|slot| slot.to_group_position() == line) {
            return factor(familiarity(player, slot));
        }
        Position::ALL
            .iter()
            .filter(|position| position.to_group_position() == line)
            .map(|position| factor(familiarity(player, position)))
            .fold(f64::MIN, f64::max)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::uniform_attributes;

    /// The entries of `line_familiarity`, in its order.
    const GK: usize = 0;
    const DEF: usize = 1;
    const MID: usize = 2;
    const FWD: usize = 3;

    const CAN_PLAY: f64 = 1.0 - CAN_PLAY_COST;
    const OUT_OF_POSITION: f64 = 1.0 - OUT_OF_POSITION_COST;

    fn player(natural: Position, alternates: Vec<Position>) -> Player {
        let mut player = Player::new(
            "p".into(),
            "P".into(),
            "Player".into(),
            "1995-01-01".into(),
            "ENG".into(),
            natural,
            uniform_attributes(65),
        );
        player.alternate_positions = alternates;
        player
    }

    /// Given the calibrated costs,
    /// When the factors are read,
    /// Then a player out of position keeps 0.90 of his rating and one who can play the slot
    /// keeps 1 - 0.10 / 3: the size the PR measured, the largest inside a season's noise.
    #[test]
    fn the_costs_are_a_tenth_out_of_position_and_a_third_of_that_where_he_can_play() {
        assert_eq!(factor(Familiarity::OutOfPosition), 0.90);
        assert!((factor(Familiarity::CanPlay) - (1.0 - 0.10 / 3.0)).abs() < 1e-12);
    }

    /// Given a natural right-back,
    /// When he starts at right-back,
    /// Then he carries no cost in defence.
    #[test]
    fn a_natural_player_in_his_own_slot_pays_nothing() {
        let right_back = player(Position::RightBack, vec![]);
        let lines = line_familiarity(&right_back, Some(&Position::RightBack));
        assert_eq!(lines[DEF], 1.0);
    }

    /// Given a natural right-back,
    /// When he starts at left-back,
    /// Then he carries the can-play cost in defence from the first minute, although his own
    /// position is in that line too: the kickoff slot is the one that counts.
    #[test]
    fn a_right_back_at_left_back_pays_the_can_play_cost() {
        let right_back = player(Position::RightBack, vec![]);
        let lines = line_familiarity(&right_back, Some(&Position::LeftBack));
        assert_eq!(lines[DEF], CAN_PLAY);
    }

    /// Given a natural striker,
    /// When he starts at centre-back,
    /// Then he carries the out-of-position cost in defence.
    #[test]
    fn a_striker_at_centre_back_pays_the_out_of_position_cost() {
        let striker = player(Position::Striker, vec![]);
        let lines = line_familiarity(&striker, Some(&Position::CenterBack));
        assert_eq!(lines[DEF], OUT_OF_POSITION);
    }

    /// Given a natural striker who can also play attacking midfield,
    /// When he starts on the bench,
    /// Then each line carries his best familiarity in it: none up front, can-play in midfield
    /// (his alternate), out of position in defence and in goal.
    #[test]
    fn a_bench_player_carries_his_best_familiarity_in_each_line() {
        let striker = player(Position::Striker, vec![Position::AttackingMidfielder]);
        let lines = line_familiarity(&striker, None);
        assert_eq!(lines, [OUT_OF_POSITION, OUT_OF_POSITION, CAN_PLAY, 1.0]);
    }

    /// Given a natural striker who starts at centre-back,
    /// When the engine moves him back up front (a change of shape),
    /// Then up front he pays nothing: every line but the kickoff one carries his best there.
    #[test]
    fn a_player_moved_out_of_his_kickoff_line_carries_his_best_in_the_new_one() {
        let striker = player(Position::Striker, vec![]);
        let lines = line_familiarity(&striker, Some(&Position::CenterBack));
        assert_eq!(
            (lines[DEF], lines[FWD], lines[MID]),
            (OUT_OF_POSITION, 1.0, OUT_OF_POSITION)
        );
    }

    /// Given a natural goalkeeper and a natural centre-back,
    /// When the keeper starts at centre-back and the centre-back starts in goal,
    /// Then each pays the out-of-position cost in the line he starts in.
    #[test]
    fn a_keeper_outfield_and_an_outfielder_in_goal_are_both_out_of_position() {
        let keeper = player(Position::Goalkeeper, vec![]);
        let centre_back = player(Position::CenterBack, vec![]);
        assert_eq!(
            line_familiarity(&keeper, Some(&Position::CenterBack))[DEF],
            OUT_OF_POSITION
        );
        assert_eq!(
            line_familiarity(&centre_back, Some(&Position::Goalkeeper))[GK],
            OUT_OF_POSITION
        );
    }
}
