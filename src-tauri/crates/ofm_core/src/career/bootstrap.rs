//! Putting the player's manager in charge of a club in a world that already exists.
//!
//! [`begin_career`](super::begin_career) always builds the pyramid first, so the
//! world it hands over already has competitions of its own and every career is a
//! takeover: the incumbent is replaced, and the world is left as it is.

use domain::stats::StatsState;

use crate::game::Game;

pub(super) fn takeover_club(
    game: &mut Game,
    team_id: &str,
    stats_state: StatsState,
) -> Result<StatsState, String> {
    let team = game
        .teams
        .iter()
        .find(|t| t.id == team_id)
        .ok_or("be.error.teamNotFound".to_string())?;
    let team_name = team.name.clone();

    crate::ai_hiring::seed_ai_managers(game);

    let takeover_date = game.clock.current_date.format("%Y-%m-%d").to_string();
    let incumbent_manager_id = game
        .teams
        .iter()
        .find(|candidate| candidate.id == team_id)
        .and_then(|candidate| candidate.manager_id.clone());

    if incumbent_manager_id.as_deref() != Some(game.manager.id.as_str()) {
        let fired = crate::firing::fire_ai_manager_for_team(game, team_id, &takeover_date);
        if !fired
            && let Some(team) = game
                .teams
                .iter_mut()
                .find(|candidate| candidate.id == team_id)
        {
            team.manager_id = None;
        }
        crate::job_offers::hire_manager(game, team_id, &takeover_date)?;
    }

    let staff_msg = crate::messages::staff_advice_message(&team_name, team_id, &takeover_date);
    game.messages.push(staff_msg);
    crate::player_events::generate_takeover_contract_review_message(game);
    crate::season_context::refresh_game_context(game);

    Ok(stats_state)
}
