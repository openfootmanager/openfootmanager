//! The career row a season leaves on each player who played in it.

use crate::game::Game;

/// Each player with appearances gets career entries for the season: one per club he played for,
/// from his stints (`domain::career_stints`), the club he is with now for what he did since he
/// arrived. A save from before stints has none, so its whole season goes to his club.
///
/// Each row carries the days of its stint, and the day the season's stats began counting
/// (`Game::last_season_rollover_on`): the career projection places a row by them (#1402). That day
/// is read here, BEFORE the rollover overwrites it with its own (`apply_season_end_squad_turnover`
/// runs after this), so it is the start of the season that is ending, not of the next one. It is
/// `None` until a first rollover has recorded one; the projection then uses the calendar's start.
///
/// A season at no club stores no name: the screens say "free agent" in the player's language, and
/// an older save's stored English literal is read the same way.
pub(super) fn record_season_career_entries(game: &mut Game, season: u32) {
    let season_began_on = game.last_season_rollover_on.clone();
    for player in game.players.iter_mut() {
        let team_name = player
            .team_id
            .as_ref()
            .and_then(|tid| game.teams.iter().find(|t| &t.id == tid))
            .map(|t| t.name.clone())
            .unwrap_or_default();
        let team_id = player.team_id.clone().unwrap_or_default();

        let mut entries =
            player
                .season_stints
                .career_entries(season, &player.stats, &team_id, &team_name);
        for entry in &mut entries {
            entry.season_began_on = season_began_on.clone();
        }
        player.career.extend(entries);
    }
}
