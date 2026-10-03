//! Serialize the application's save, clock and live-session lifecycle operations.
//!
//! The desktop application owns one active game. Keeping the gate here avoids
//! making the persistence crate know about a transient live-match session. The
//! gate also closes the gap between checking the session and writing a save or
//! starting a match; finish holds it until the complete result is applied.
use ofm_core::state::StateManager;
use std::sync::{Mutex, MutexGuard};

static OPERATION: Mutex<()> = Mutex::new(());

pub(crate) fn operation() -> MutexGuard<'static, ()> {
    OPERATION
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub(crate) fn ensure_idle(state: &StateManager) -> Result<(), String> {
    if state.with_live_match(|_| ()).is_some() {
        Err("be.error.liveMatch.inProgress".to_string())
    } else {
        Ok(())
    }
}

pub(crate) fn idle_operation(state: &StateManager) -> Result<MutexGuard<'static, ()>, String> {
    let guard = operation();
    ensure_idle(state)?;
    Ok(guard)
}

#[cfg(test)]
mod tests {
    use crate::{
        application::{live_match, time_advancement},
        commands::{season, time, util},
    };
    use chrono::{TimeZone, Utc};
    use domain::league::{Fixture, FixtureCompetition, FixtureStatus};
    use domain::manager::Manager;
    use domain::player::{Player, PlayerAttributes, Position};
    use domain::team::Team;
    use ofm_core::{clock::GameClock, game::Game, state::StateManager};
    use serde_json::Value;

    const KEY: &str = "be.error.liveMatch.inProgress";
    fn default_attrs() -> PlayerAttributes {
        PlayerAttributes {
            pace: 60,
            stamina: 60,
            strength: 60,
            agility: 60,
            passing: 60,
            shooting: 60,
            tackling: 60,
            dribbling: 60,
            defending: 60,
            positioning: 60,
            vision: 60,
            decisions: 60,
            composure: 60,
            aggression: 60,
            teamwork: 60,
            leadership: 60,
            handling: 60,
            reflexes: 60,
            aerial: 60,
        }
    }

    fn make_player(id: &str, name: &str, team_id: &str, position: Position) -> Player {
        let mut player = Player::new(
            id.to_string(),
            name.to_string(),
            name.to_string(),
            "2000-01-01".to_string(),
            "England".to_string(),
            position,
            default_attrs(),
        );
        player.team_id = Some(team_id.to_string());
        player
    }

    fn make_game(roster_size: usize) -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "mgr1".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("team1".to_string());

        let players: Vec<Player> = (1..=roster_size)
            .map(|idx| {
                let position = if idx == 1 {
                    Position::Goalkeeper
                } else if idx <= 5 {
                    Position::Defender
                } else if idx <= 9 {
                    Position::Midfielder
                } else {
                    Position::Forward
                };

                make_player(
                    &format!("p{}", idx),
                    &format!("Player {}", idx),
                    "team1",
                    position,
                )
            })
            .collect();

        let mut team = Team::new(
            "team1".to_string(),
            "Test FC".to_string(),
            "TST".to_string(),
            "England".to_string(),
            "Testville".to_string(),
            "Test Ground".to_string(),
            20_000,
        );
        team.starting_xi_ids = players
            .iter()
            .take(11)
            .map(|player| player.id.clone())
            .collect();

        Game::new(clock, manager, vec![team], players, vec![], vec![])
    }

    fn make_game_with_matchday() -> Game {
        let mut game = make_game(22);
        let today = game.clock.current_date.format("%Y-%m-%d").to_string();
        let mut opponent_team = Team::new(
            "team2".to_string(),
            "Rival FC".to_string(),
            "RIV".to_string(),
            "England".to_string(),
            "Rivaltown".to_string(),
            "Rival Ground".to_string(),
            21_000,
        );
        opponent_team.starting_xi_ids = game
            .players
            .iter()
            .skip(11)
            .take(11)
            .map(|p| p.id.clone())
            .collect();
        game.teams.push(opponent_team);

        for player in game.players.iter_mut().skip(11) {
            player.team_id = Some("team2".to_string());
        }

        game.teams[0].starting_xi_ids =
            game.players.iter().take(11).map(|p| p.id.clone()).collect();
        game.league = Some(domain::league::League {
            id: "league-1".to_string(),
            name: "League".to_string(),
            season: 2025,
            fixtures: vec![Fixture {
                id: "fixture-1".to_string(),
                competition_id: "league-1".to_string(),
                matchday: 1,
                date: today,
                home_team_id: "team1".to_string(),
                away_team_id: "team2".to_string(),
                competition: FixtureCompetition::League,
                status: FixtureStatus::Scheduled,
                result: None,
            }],
            standings: vec![
                domain::league::StandingEntry::new("team1".to_string()),
                domain::league::StandingEntry::new("team2".to_string()),
            ],
            transfer_log: vec![],
            transfer_rumours: vec![],
            ..domain::league::League::default()
        });
        game.competitions = game.league.iter().cloned().collect();
        game
    }

    fn state_at_minute_thirty() -> StateManager {
        let state = StateManager::new();
        state.set_game(make_game_with_matchday());
        live_match::start_live_match_with_identity(&state, 0, "spectator", false, None, None)
            .unwrap();
        live_match::step_live_match(&state, 30).unwrap();
        let minute = live_match::get_match_snapshot(&state)
            .unwrap()
            .current_minute;
        if minute < 30 {
            live_match::step_live_match(&state, u16::from(30 - minute)).unwrap();
        }
        assert_eq!(
            live_match::get_match_snapshot(&state)
                .unwrap()
                .current_minute,
            30
        );
        state
    }

    fn checkpoint(state: &StateManager) -> Value {
        serde_json::json!({
            "game": state.get_game(|game| serde_json::to_value(game).unwrap()),
            "stats": state.get_stats_state(|stats| serde_json::to_value(stats).unwrap()),
            "snapshot": live_match::get_match_snapshot(state).unwrap(),
            "identity": state.with_live_match(|session| (session.competition_id.clone(), session.fixture_id.clone(), session.fixture_index)),
            "save_id": state.get_save_id(),
        })
    }

    macro_rules! refused_advance {
        ($name:ident, $operation:expr) => {
            /// Given a live session at minute 30, when this date route is requested,
            /// then the live-match key is returned and game, stats and session are unchanged.
            #[test]
            fn $name() {
                let state = state_at_minute_thirty();
                let before = checkpoint(&state);
                assert_eq!(($operation)(&state).err().as_deref(), Some(KEY));
                assert_eq!(checkpoint(&state), before);
            }
        };
    }
    refused_advance!(
        plain_advancing_during_a_live_match_is_refused,
        time::advance_time_internal
    );
    refused_advance!(mode_advancing_during_a_live_match_is_refused, |s| {
        time_advancement::advance_time_with_mode(s, "delegate")
    });
    refused_advance!(live_mode_advancing_during_a_live_match_is_refused, |s| {
        time_advancement::advance_time_with_mode(s, "live")
    });
    refused_advance!(
        skipping_during_a_live_match_is_refused,
        time::skip_to_match_day_internal
    );
    refused_advance!(
        one_day_advancing_during_a_live_match_is_refused,
        time::advance_one_day_internal
    );
    refused_advance!(
        event_advancing_during_a_live_match_is_refused,
        time::advance_to_next_event_internal
    );
    refused_advance!(
        season_advancing_during_a_live_match_is_refused,
        season::advance_to_next_season_internal
    );

    /// Given a live session, when another match is started, then the first session is unchanged.
    #[test]
    fn starting_a_second_match_is_refused() {
        let state = state_at_minute_thirty();
        let before = checkpoint(&state);
        assert_eq!(
            live_match::start_live_match_with_identity(&state, 0, "live", false, None, None)
                .err()
                .as_deref(),
            Some(KEY)
        );
        assert_eq!(checkpoint(&state), before);
    }

    fn save_directory() -> std::path::PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("ofm-live-save-{}-{unique}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn file_bytes(
        path: &std::path::Path,
    ) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
        let mut files = std::collections::BTreeMap::new();
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                files.extend(file_bytes(&path));
            } else {
                files.insert(path.clone(), std::fs::read(path).unwrap());
            }
        }
        files
    }

    /// Given a live session at minute 30, when save is requested, then no file changes and the session continues.
    #[test]
    fn saving_during_a_live_match_is_refused() {
        let state = state_at_minute_thirty();
        let path = save_directory();
        let mut saves = db::save_manager::SaveManager::init(&path).unwrap();
        let id = saves
            .create_save(&state.get_game(Clone::clone).unwrap(), "Live refusal")
            .unwrap();
        state.set_save_id(id);
        state.update_game(|game| game.teams[0].finance += 1234);
        let before = checkpoint(&state);
        let files = file_bytes(&path);
        let result = util::persist_active_game(&state, &mut saves);
        assert_eq!(result.err().as_deref(), Some(KEY));
        assert_eq!(file_bytes(&path), files);
        assert_eq!(checkpoint(&state), before);
        assert!(live_match::step_live_match(&state, 1).is_ok());
        drop(saves);
        std::fs::remove_dir_all(path).unwrap();
    }

    /// Given a session completed through finish, when saving, then the persisted fixture holds the full result.
    #[test]
    fn saving_after_the_match_finishes_works() {
        let state = state_at_minute_thirty();
        let path = save_directory();
        let mut saves = db::save_manager::SaveManager::init(&path).unwrap();
        let id = saves
            .create_save(&state.get_game(Clone::clone).unwrap(), "Finished match")
            .unwrap();
        state.set_save_id(id.clone());
        live_match::finish_live_match(&state).unwrap();
        util::persist_active_game(&state, &mut saves).unwrap();
        let game = saves.load_game(&id).unwrap();
        let fixture = &game.league.unwrap().fixtures[0];
        assert_eq!(fixture.status, FixtureStatus::Completed);
        assert!(fixture.result.is_some());
        assert!(state.with_live_match(|_| ()).is_none());
        drop(saves);
        std::fs::remove_dir_all(path).unwrap();
    }
    /// Given one scheduled fixture and two callers released together, when both start,
    /// then exactly one starts and the other is refused without replacing the session.
    #[test]
    fn concurrent_match_starts_keep_the_first_session() {
        let state = std::sync::Arc::new(StateManager::new());
        state.set_game(make_game_with_matchday());
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
        let handles: Vec<_> = (0..2)
            .map(|_| {
                let state = state.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    live_match::start_live_match_with_identity(
                        &state,
                        0,
                        "spectator",
                        false,
                        None,
                        None,
                    )
                })
            })
            .collect();
        barrier.wait();
        let outcomes: Vec<_> = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect();
        assert_eq!(outcomes.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(
            outcomes
                .iter()
                .find_map(|result| result.as_ref().err())
                .unwrap(),
            KEY
        );
        assert_eq!(
            live_match::get_match_snapshot(&state)
                .unwrap()
                .current_minute,
            0
        );
    }

    /// Given a full-time snapshot whose result has not yet been applied, when advancing,
    /// then the session still blocks the clock until finish is called.
    #[test]
    fn a_finished_snapshot_still_blocks_advancement_until_finish() {
        let state = state_at_minute_thirty();
        state
            .with_live_match(|session| session.run_to_completion())
            .unwrap();
        let before = checkpoint(&state);
        assert_eq!(
            time::advance_time_internal(&state).err().as_deref(),
            Some(KEY)
        );
        assert_eq!(checkpoint(&state), before);
    }
}
