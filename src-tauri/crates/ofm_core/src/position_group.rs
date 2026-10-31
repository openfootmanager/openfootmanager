//! The group (goalkeeper, defender, midfielder, forward) each position belongs to and where it sorts
//! in a squad list, for the screens that colour, group or sort by it. The rules are
//! `Position::to_group_position` and `Position::sort_rank`; this is their table, so the frontend asks
//! for it instead of keeping a copy.

use domain::player::Position;
use serde::Serialize;

/// One position, the group it belongs to and its place in a squad list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PositionGroupEntry {
    pub position: Position,
    pub group: Position,
    /// `Position::sort_rank`: lower sorts first.
    pub rank: u32,
}

/// Every position with its group and rank, so a screen reads both for any position it is given.
pub fn position_groups() -> Vec<PositionGroupEntry> {
    Position::ALL
        .into_iter()
        .map(|position| PositionGroupEntry {
            group: position.to_group_position(),
            rank: position.sort_rank(),
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
            let entries = table
                .iter()
                .filter(|entry| entry.position == position)
                .count();
            assert_eq!(entries, 1, "{position:?}");
        }
    }

    /// Given the table of position groups,
    /// When a wing-back, a winger, a defensive midfielder and the keeper are read,
    /// Then each is in the group the domain rule gives it, not a group of its own.
    #[test]
    fn the_table_follows_the_domain_rule() {
        let table = position_groups();

        assert_eq!(
            group_of(&table, Position::RightWingBack),
            Some(Position::Defender)
        );
        assert_eq!(
            group_of(&table, Position::LeftWinger),
            Some(Position::Forward)
        );
        assert_eq!(
            group_of(&table, Position::DefensiveMidfielder),
            Some(Position::Midfielder)
        );
        assert_eq!(
            group_of(&table, Position::Goalkeeper),
            Some(Position::Goalkeeper)
        );
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

    /// Given the table of position groups,
    /// When it is sorted by rank,
    /// Then it reads in pitch order: the keeper first, the striker last, and every defender before
    ///      any midfielder.
    #[test]
    fn the_table_carries_the_sort_rank_of_every_position() {
        let mut table = position_groups();
        for entry in &table {
            assert_eq!(entry.rank, entry.position.sort_rank(), "{:?}", entry.position);
        }

        table.sort_by_key(|entry| entry.rank);

        assert_eq!(table.first().map(|entry| entry.position.clone()), Some(Position::Goalkeeper));
        assert_eq!(table.last().map(|entry| entry.position.clone()), Some(Position::Striker));
        let last_defender = table.iter().rposition(|e| e.group == Position::Defender);
        let first_midfielder = table.iter().position(|e| e.group == Position::Midfielder);
        assert!(last_defender < first_midfielder);
    }
}
