use crate::game::Game;
use domain::player::Player;

/// Returns all players (senior and youth) belonging to the given team.
/// Filtering to senior squad is done client-side.
pub fn query_squad(game: &Game, team_id: &str) -> Vec<Player> {
    game.players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some(team_id))
        .cloned()
        .collect()
}
