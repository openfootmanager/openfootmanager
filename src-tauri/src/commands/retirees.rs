use std::sync::Arc;

use domain::player::Player;
use ofm_core::hall_of_fame::{HallOfFameLegend, PastChampion};
use ofm_core::state::StateManager;
use tauri::State;

const NO_ACTIVE_GAME: &str = "be.error.noActiveGameSession";
const PLAYER_NOT_FOUND: &str = "be.error.playerNotFound";

pub fn get_hall_of_fame_legends_internal(
    state: &StateManager,
) -> Result<Vec<HallOfFameLegend>, String> {
    state
        .get_game(ofm_core::hall_of_fame::hall_of_fame_legends)
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())
}

pub fn get_hall_of_fame_champions_internal(
    state: &StateManager,
) -> Result<Vec<PastChampion>, String> {
    state
        .get_game(ofm_core::hall_of_fame::hall_of_fame_champions)
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())
}

pub fn get_retired_player_internal(
    state: &StateManager,
    player_id: &str,
) -> Result<Player, String> {
    state
        .get_game(|game| ofm_core::retirees::find_retiree(game, player_id))
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())?
        .ok_or_else(|| PLAYER_NOT_FOUND.to_string())
}

#[tauri::command]
pub fn get_hall_of_fame_legends(
    state: State<'_, Arc<StateManager>>,
) -> Result<Vec<HallOfFameLegend>, String> {
    get_hall_of_fame_legends_internal(&state)
}

/// Every league title in the world's past, latest first: the backend names the champions, so no
/// screen reads "first place" as a title.
#[tauri::command]
pub fn get_hall_of_fame_champions(
    state: State<'_, Arc<StateManager>>,
) -> Result<Vec<PastChampion>, String> {
    get_hall_of_fame_champions_internal(&state)
}

#[tauri::command]
pub fn get_retired_player(
    state: State<'_, Arc<StateManager>>,
    player_id: String,
) -> Result<Player, String> {
    get_retired_player_internal(&state, &player_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::season::tests::completed_checkpoint;
    use domain::retired_player::RetiredPlayer;

    fn state_with_an_archived_veteran() -> StateManager {
        let mut game = completed_checkpoint();
        let mut veteran = game.players[0].clone();
        veteran.id = "p-veteran".into();
        veteran.full_name = "Ada Old".into();
        veteran.retired = true;
        veteran.career = vec![domain::player::CareerEntry {
            season: 2038,
            team_id: "home".into(),
            team_name: "Home FC".into(),
            appearances: 30,
            goals: 3,
            assists: 4,
        }];
        game.retired_players
            .push(RetiredPlayer::from_player(&veteran, "2039-06-30", 2039));
        let state = StateManager::new();
        state.set_game(game);
        state
    }

    /// Given an archived veteran, when the Hall of Fame is requested, then he is listed.
    #[test]
    fn the_hall_of_fame_lists_an_archived_veteran() {
        let legends = get_hall_of_fame_legends_internal(&state_with_an_archived_veteran()).unwrap();

        assert_eq!(legends.len(), 1);
        assert_eq!(legends[0].full_name, "Ada Old");
    }

    /// Given a game with a club that won a title in the past, when the champions are requested,
    /// then that title is listed with its club and season.
    #[test]
    fn the_hall_of_fame_lists_a_past_title() {
        let mut game = completed_checkpoint();
        game.teams[0].history.push(domain::team::TeamSeasonRecord {
            season: 2038,
            league_position: 1,
            played: 38,
            won: 30,
            drawn: 4,
            lost: 4,
            goals_for: 80,
            goals_against: 30,
            competition_id: String::new(),
        });
        let state = StateManager::new();
        state.set_game(game);

        let champions = get_hall_of_fame_champions_internal(&state).unwrap();

        assert_eq!(champions.len(), 1);
        assert_eq!(champions[0].season, 2038);
    }

    /// Given no game, when the champions are requested, then the session error comes back.
    #[test]
    fn the_champions_need_a_game() {
        let error = get_hall_of_fame_champions_internal(&StateManager::new()).unwrap_err();

        assert_eq!(error, "be.error.noActiveGameSession");
    }

    /// Given no game, when the Hall of Fame is requested, then the session error comes back.
    #[test]
    fn the_hall_of_fame_needs_a_game() {
        let error = get_hall_of_fame_legends_internal(&StateManager::new()).unwrap_err();

        assert_eq!(error, "be.error.noActiveGameSession");
    }

    /// Given an archived veteran, when his profile is requested, then it comes back as a retired player.
    #[test]
    fn an_archived_veteran_has_a_profile() {
        let player =
            get_retired_player_internal(&state_with_an_archived_veteran(), "p-veteran").unwrap();

        assert!(player.retired);
        assert_eq!(player.full_name, "Ada Old");
    }

    /// Given an active player, when asked for as a retiree, then he is not found.
    #[test]
    fn an_active_player_is_not_a_retiree() {
        let state = state_with_an_archived_veteran();
        let active_id = state.get_game(|game| game.players[0].id.clone()).unwrap();

        let error = get_retired_player_internal(&state, &active_id).unwrap_err();

        assert_eq!(error, "be.error.playerNotFound");
    }
}
