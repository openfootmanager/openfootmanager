//! Position vocabulary for the screens that colour a position badge.

/// Every position with its group, for the screens that colour or sort by group. Needs no game, so
/// the package editor reads it too.
#[tauri::command]
pub fn get_position_groups() -> Vec<ofm_core::position_group::PositionGroupEntry> {
    ofm_core::position_group::position_groups()
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::player::Position;

    /// Given no game is loaded,
    /// When the position groups are asked for,
    /// Then it is the backend's table: all 17 positions, and a wing-back is a defender.
    #[test]
    fn get_position_groups_is_the_backends_table() {
        let groups = get_position_groups();

        assert_eq!(groups.len(), 17);
        let wing_back = groups
            .iter()
            .find(|entry| entry.position == Position::LeftWingBack)
            .map(|entry| entry.group.clone());
        assert_eq!(wing_back, Some(Position::Defender));
    }
}
