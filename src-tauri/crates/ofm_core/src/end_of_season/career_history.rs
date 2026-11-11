//! The career row a season leaves on each player who played in it.

use crate::game::Game;

/// Each player with appearances gets one career entry for the season, from his season stats.
///
/// A season at no club stores no name: the screens say "free agent" in the player's language, and
/// an older save's stored English literal is read the same way.
pub(super) fn record_season_career_entries(game: &mut Game, season: u32) {
    for player in game.players.iter_mut() {
        if player.stats.appearances > 0 {
            let team_name = player
                .team_id
                .as_ref()
                .and_then(|tid| game.teams.iter().find(|t| &t.id == tid))
                .map(|t| t.name.clone())
                .unwrap_or_default();
            let team_id = player.team_id.clone().unwrap_or_default();

            player.career.push(domain::player::CareerEntry {
                season,
                team_id,
                team_name,
                appearances: player.stats.appearances,
                goals: player.stats.goals,
                assists: player.stats.assists,
            });
        }
    }
}
