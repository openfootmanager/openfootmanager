//! The group (goalkeeper, defender, midfielder, forward) each position belongs to, for the screens
//! that colour or sort by it. The rule is `Position::to_group_position`; this is its table, so the
//! frontend asks for it instead of keeping a copy.

use domain::player::Position;
use serde::Serialize;

/// One position and the group it belongs to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PositionGroupEntry {
    pub position: Position,
    pub group: Position,
}

/// Every position with its group, so a screen reads the group of any position it is given.
pub fn position_groups() -> Vec<PositionGroupEntry> {
    Position::ALL
        .into_iter()
        .map(|position| PositionGroupEntry {
            group: position.to_group_position(),
            position,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group_of(table: &[PositionGroupEntry], position: Position) -> Option<Position> {
        table
            .iter()
            .find(|entry| entry.position == position)
            .map(|entry| entry.group.clone())
    }

    /// Given the table of position groups,
    /// When it is read,
    /// Then it names every position exactly once.
    #[test]
    fn the_table_names_every_position_once() {
        let table = position_groups();

        assert_eq!(table.len(), Position::ALL.len());
        for position in Position::ALL {
            let entries = table.iter().filter(|entry| entry.position == position).count();
            assert_eq!(entries, 1, "{position:?}");
        }
    }

    /// Given the table of position groups,
    /// When a wing-back, a winger, a defensive midfielder and the keeper are read,
    /// Then each is in the group the domain rule gives it, not a group of its own.
    #[test]
    fn the_table_follows_the_domain_rule() {
        let table = position_groups();

        assert_eq!(group_of(&table, Position::RightWingBack), Some(Position::Defender));
        assert_eq!(group_of(&table, Position::LeftWinger), Some(Position::Forward));
        assert_eq!(group_of(&table, Position::DefensiveMidfielder), Some(Position::Midfielder));
        assert_eq!(group_of(&table, Position::Goalkeeper), Some(Position::Goalkeeper));
    }

    /// Given the table of position groups,
    /// When every group in it is collected,
    /// Then there are exactly the four legacy buckets, and each is its own group.
    #[test]
    fn every_group_is_one_of_the_four_buckets_and_is_its_own_group() {
        let table = position_groups();

        for entry in &table {
            assert!(entry.group.is_legacy_bucket(), "{:?}", entry.group);
        }
        for bucket in [
            Position::Goalkeeper,
            Position::Defender,
            Position::Midfielder,
            Position::Forward,
        ] {
            assert_eq!(group_of(&table, bucket.clone()), Some(bucket));
        }
    }
}
