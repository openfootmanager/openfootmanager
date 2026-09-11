use std::collections::HashSet;

use db::save_manager::SaveManager;
use domain::team::Team;
use ofm_core::game::Game;
use ofm_core::state::StateManager;

const NO_ACTIVE_GAME: &str = "be.error.noActiveGameSession";
const NO_TEAM_ASSIGNED: &str = "be.error.noTeamAssigned";
const TEAM_NOT_FOUND: &str = "be.error.teamNotFound";

/// The id of the club the player currently manages.
pub fn user_team_id(game: &Game) -> Result<String, String> {
    game.manager
        .team_id
        .clone()
        .ok_or_else(|| NO_TEAM_ASSIGNED.to_string())
}

/// The club the player currently manages, borrowed for mutation.
///
/// Prefer this over hand-rolling `manager.team_id` + `teams.iter_mut().find()`.
/// Written out by hand the lookup is easy to end with `if let Some(team)`, which
/// leaves the command returning `Ok` having changed nothing when the id does not
/// match a club — a silent no-op the UI reports as success.
pub fn user_team_mut(game: &mut Game) -> Result<&mut Team, String> {
    let team_id = user_team_id(game)?;

    game.teams
        .iter_mut()
        .find(|team| team.id == team_id)
        .ok_or_else(|| TEAM_NOT_FOUND.to_string())
}

/// Mutate the active game in place and return a single clone for the response.
///
/// Replaces the pervasive `get_game(|g| g.clone())` → mutate → `set_game(clone)`
/// pattern, which deep-cloned the entire world (440 teams, ~9,680 players)
/// *twice* per command. Here the world is borrowed and mutated in place, then
/// cloned once for serialization back to the UI.
///
/// The closure performs validation **before** mutation: because it mutates the
/// live game rather than a throwaway clone, any change made before it returns
/// `Err` would persist. Existing callers already validate up front.
pub fn mutate_active_game<F>(state: &StateManager, mutate: F) -> Result<Game, String>
where
    F: FnOnce(&mut Game) -> Result<(), String>,
{
    state
        .update_game(|game| {
            mutate(game)?;
            Ok(game.clone())
        })
        .unwrap_or_else(|| Err(NO_ACTIVE_GAME.to_string()))
}

/// Snapshot dirty journal ids, persist `&Game`, then drop flushed ids on the live Game.
///
/// Every `StateManager`-owned save must go through this helper so MCP autosave
/// and Tauri `save_game` share one flush protocol. Collision on insert is a
/// no-op (`INSERT OR IGNORE`).
pub fn persist_active_game(
    state: &StateManager,
    save_manager: &mut SaveManager,
) -> Result<(), String> {
    let save_id = state
        .get_save_id()
        .filter(|id| !id.is_empty())
        .ok_or_else(|| "be.error.noActiveSaveSession".to_string())?;
    let (game, flushed) = state
        .get_game(|game| (game.clone(), game.cash_journal_dirty_ids.clone()))
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())?;
    let stats = state
        .get_stats_state(|stats| stats.clone())
        .unwrap_or_default();
    save_manager.save_game_with_stats(&game, &stats, &save_id)?;
    let flushed: HashSet<String> = flushed.into_iter().collect();
    state.update_game(|live| {
        live.cash_journal_dirty_ids
            .retain(|id| !flushed.contains(id));
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{user_team_id, user_team_mut};
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::team::Team;
    use ofm_core::clock::GameClock;
    use ofm_core::game::Game;

    fn make_game(manager_team_id: Option<&str>, teams: Vec<Team>) -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "manager-1".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );

        if let Some(team_id) = manager_team_id {
            manager.hire(team_id.to_string());
        }

        Game::new(clock, manager, teams, vec![], vec![], vec![])
    }

    fn make_team(id: &str) -> Team {
        Team::new(
            id.to_string(),
            "User FC".to_string(),
            "USR".to_string(),
            "England".to_string(),
            "London".to_string(),
            "User Ground".to_string(),
            25_000,
        )
    }

    #[test]
    fn resolves_the_managed_club() {
        let mut game = make_game(Some("team-1"), vec![make_team("team-1")]);

        assert_eq!(user_team_id(&game).expect("id"), "team-1");
        assert_eq!(user_team_mut(&mut game).expect("team").id, "team-1");
    }

    #[test]
    fn reports_an_unemployed_manager() {
        let mut game = make_game(None, vec![make_team("team-1")]);

        assert_eq!(user_team_id(&game), Err("be.error.noTeamAssigned".into()));
        assert_eq!(
            user_team_mut(&mut game).err(),
            Some("be.error.noTeamAssigned".into())
        );
    }

    #[test]
    fn reports_a_team_id_that_matches_no_club() {
        let mut game = make_game(Some("team-1"), vec![make_team("team-other")]);

        // The id resolves, so only the lookup can tell these two apart.
        assert_eq!(user_team_id(&game).expect("id"), "team-1");
        assert_eq!(
            user_team_mut(&mut game).err(),
            Some("be.error.teamNotFound".into())
        );
    }

    #[test]
    fn persist_active_game_flushes_dirty_ids_and_is_idempotent() {
        use super::persist_active_game;
        use db::save_manager::SaveManager;
        use domain::finance::CashKind;
        use ofm_core::finances::post_legacy;
        use ofm_core::state::StateManager;
        use std::fs;
        use std::time::{SystemTime, UNIX_EPOCH};

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("ofm-persist-journal-{unique}"));
        fs::create_dir_all(&path).expect("temp dir");

        let mut save_manager = SaveManager::init(&path).expect("save manager");
        let mut team = make_team("team-1");
        team.finance = 500_000;
        let game = make_game(Some("team-1"), vec![team]);
        let save_id = save_manager
            .create_save(&game, "Journal Persist")
            .expect("save");

        let state = StateManager::new();
        state.set_game(game);
        state.set_save_id(save_id.clone());

        state
            .update_game(|live| {
                post_legacy(
                    live,
                    "team-1",
                    -1_000,
                    CashKind::PlayerWages,
                    live.clock.current_date.date_naive(),
                )
                .expect("post");
            })
            .expect("update");

        persist_active_game(&state, &mut save_manager).expect("first persist");
        assert!(state
            .get_game(|live| live.cash_journal_dirty_ids.is_empty())
            .expect("live game"));

        let first = save_manager.load_game(&save_id).expect("load");
        let first_len = first.cash_journal.len();
        assert!(first_len >= 2, "seed + wages");

        persist_active_game(&state, &mut save_manager).expect("second persist");
        let second = save_manager.load_game(&save_id).expect("reload");
        assert_eq!(second.cash_journal.len(), first_len);

        state
            .update_game(|live| {
                post_legacy(
                    live,
                    "team-1",
                    250,
                    CashKind::Matchday,
                    live.clock.current_date.date_naive(),
                )
                .expect("post");
            })
            .expect("update");
        persist_active_game(&state, &mut save_manager).expect("third persist");
        let third = save_manager.load_game(&save_id).expect("reload");
        assert_eq!(third.cash_journal.len(), first_len + 1);

        let _ = fs::remove_dir_all(&path);
    }
}
