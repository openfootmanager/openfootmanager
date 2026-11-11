//! The career row a season leaves on each player who played in it.

use crate::game::Game;

/// Each player with appearances gets career entries for the season: one per club he played for,
/// from his stints (`domain::career_stints`), the club he is with now for what he did since he
/// arrived. A save from before stints has none, so its whole season goes to his club.
///
/// A season at no club stores no name: the screens say "free agent" in the player's language, and
/// an older save's stored English literal is read the same way.
pub(super) fn record_season_career_entries(game: &mut Game, season: u32) {
    for player in game.players.iter_mut() {
        let team_name = player
            .team_id
            .as_ref()
            .and_then(|tid| game.teams.iter().find(|t| &t.id == tid))
            .map(|t| t.name.clone())
            .unwrap_or_default();
        let team_id = player.team_id.clone().unwrap_or_default();

        let entries =
            player
                .season_stints
                .career_entries(season, &player.stats, &team_id, &team_name);
        player.career.extend(entries);
    }
}
