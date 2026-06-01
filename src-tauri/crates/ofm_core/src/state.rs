use crate::game::Game;
use crate::live_match_manager::LiveMatchSession;
use domain::stats::StatsState;
use std::sync::Mutex;

fn set_option<T>(mutex: &Mutex<Option<T>>, value: T) {
    let mut lock = mutex.lock().unwrap();
    *lock = Some(value);
}

fn clear_option<T>(mutex: &Mutex<Option<T>>) {
    let mut lock = mutex.lock().unwrap();
    *lock = None;
}

fn with_option<T, F, R>(mutex: &Mutex<Option<T>>, f: F) -> Option<R>
where
    F: FnOnce(&T) -> R,
{
    let lock = mutex.lock().unwrap();
    lock.as_ref().map(f)
}

fn with_option_mut<T, F, R>(mutex: &Mutex<Option<T>>, f: F) -> Option<R>
where
    F: FnOnce(&mut T) -> R,
{
    let mut lock = mutex.lock().unwrap();
    lock.as_mut().map(f)
}

fn take_option<T>(mutex: &Mutex<Option<T>>) -> Option<T> {
    let mut lock = mutex.lock().unwrap();
    lock.take()
}

fn cloned_option<T: Clone>(mutex: &Mutex<Option<T>>) -> Option<T> {
    let lock = mutex.lock().unwrap();
    lock.clone()
}

pub struct StateManager {
    pub active_game: Mutex<Option<Game>>,
    pub active_stats: Mutex<Option<StatsState>>,
    pub live_match: Mutex<Option<LiveMatchSession>>,
    pub active_save_id: Mutex<Option<String>>,
}

impl Default for StateManager {
    fn default() -> Self {
        Self::new()
    }
}

impl StateManager {
    pub fn new() -> Self {
        Self {
            active_game: Mutex::new(None),
            active_stats: Mutex::new(None),
            live_match: Mutex::new(None),
            active_save_id: Mutex::new(None),
        }
    }

    pub fn set_game(&self, game: Game) {
        set_option(&self.active_game, game);
    }

    pub fn get_game<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&Game) -> R,
    {
        with_option(&self.active_game, f)
    }

    pub fn clear_game(&self) {
        clear_option(&self.active_game);
        clear_option(&self.active_stats);
    }

    pub fn set_stats_state(&self, stats: StatsState) {
        set_option(&self.active_stats, stats);
    }

    pub fn get_stats_state<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&StatsState) -> R,
    {
        with_option(&self.active_stats, f)
    }

    pub fn with_stats_state<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&mut StatsState) -> R,
    {
        with_option_mut(&self.active_stats, f)
    }

    pub fn clear_stats_state(&self) {
        clear_option(&self.active_stats);
    }

    pub fn append_stats_state(&self, stats: StatsState) {
        let mut lock = self.active_stats.lock().unwrap();
        match lock.as_mut() {
            Some(current) => current.append(stats),
            None => *lock = Some(stats),
        }
    }

    pub fn set_save_id(&self, id: String) {
        set_option(&self.active_save_id, id);
    }

    pub fn get_save_id(&self) -> Option<String> {
        cloned_option(&self.active_save_id)
    }

    pub fn clear_save_id(&self) {
        clear_option(&self.active_save_id);
    }

    pub fn set_live_match(&self, session: LiveMatchSession) {
        set_option(&self.live_match, session);
    }

    pub fn take_live_match(&self) -> Option<LiveMatchSession> {
        take_option(&self.live_match)
    }

    pub fn with_live_match<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&mut LiveMatchSession) -> R,
    {
        with_option_mut(&self.live_match, f)
    }
}

#[cfg(test)]
mod tests {
    use super::StateManager;
    use crate::clock::GameClock;
    use crate::game::Game;
    use crate::live_match_manager::{self, MatchMode};
    use chrono::{TimeZone, Utc};
    use domain::league::{Fixture, FixtureCompetition, FixtureStatus, League, StandingEntry};
    use domain::manager::Manager;
    use domain::player::{Player, PlayerAttributes, Position};
    use domain::team::Team;

    fn default_attrs(pos: Position) -> PlayerAttributes {
        let group = pos.to_group_position();
        let is_gk = matches!(group, Position::Goalkeeper);
        let is_def = matches!(group, Position::Defender);
        let is_fwd = matches!(group, Position::Forward);

        PlayerAttributes {
            pace: 65,
            stamina: 65,
            strength: 65,
            agility: 65,
            passing: 65,
            shooting: if is_gk { 30 } else { 65 },
            tackling: if is_gk || is_fwd { 35 } else { 65 },
            dribbling: if is_gk { 30 } else { 65 },
            defending: if is_gk {
                30
            } else if is_def {
                75
            } else {
                55
            },
            positioning: 65,
            vision: 65,
            decisions: 65,
            composure: 65,
            aggression: 50,
            teamwork: 65,
            leadership: 50,
            handling: if is_gk { 75 } else { 20 },
            reflexes: if is_gk { 75 } else { 30 },
            aerial: 60,
        }
    }

    fn make_player(id: &str, name: &str, team_id: &str, position: Position) -> Player {
        let mut player = Player::new(
            id.to_string(),
            name.to_string(),
            format!("Full {}", name),
            "1995-01-01".to_string(),
            "GB".to_string(),
            position.clone(),
            default_attrs(position),
        );
        player.team_id = Some(team_id.to_string());
        player.morale = 70;
        player.condition = 90;
        player
    }

    fn make_team(id: &str, name: &str) -> Team {
        Team::new(
            id.to_string(),
            name.to_string(),
            name[..3].to_string(),
            "England".to_string(),
            "London".to_string(),
            "Stadium".to_string(),
            40_000,
        )
    }

    fn make_squad(team_id: &str) -> Vec<Player> {
        let mut players = Vec::new();

        for idx in 0..2 {
            players.push(make_player(
                &format!("{}_gk{}", team_id, idx),
                &format!("GK{}", idx),
                team_id,
                Position::Goalkeeper,
            ));
        }

        for idx in 0..7 {
            players.push(make_player(
                &format!("{}_def{}", team_id, idx),
                &format!("Def{}", idx),
                team_id,
                Position::Defender,
            ));
        }

        for idx in 0..7 {
            players.push(make_player(
                &format!("{}_mid{}", team_id, idx),
                &format!("Mid{}", idx),
                team_id,
                Position::Midfielder,
            ));
        }

        for idx in 0..6 {
            players.push(make_player(
                &format!("{}_fwd{}", team_id, idx),
                &format!("Fwd{}", idx),
                team_id,
                Position::Forward,
            ));
        }

        players
    }

    fn make_game_with_fixture() -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "mgr1".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("team1".to_string());

        let team1 = make_team("team1", "Test FC");
        let team2 = make_team("team2", "Rival FC");

        let mut players = make_squad("team1");
        players.extend(make_squad("team2"));

        let fixture = Fixture {
            id: "fix1".to_string(),
            matchday: 1,
            date: "2025-06-15".to_string(),
            home_team_id: "team1".to_string(),
            away_team_id: "team2".to_string(),
            competition: FixtureCompetition::League,
            status: FixtureStatus::Scheduled,
            result: None,
        };

        let league = League {
            id: "league1".to_string(),
            name: "Test League".to_string(),
            season: 1,
            fixtures: vec![fixture],
            standings: vec![
                StandingEntry::new("team1".to_string()),
                StandingEntry::new("team2".to_string()),
            ],
            transfer_log: vec![],
            transfer_rumours: vec![],
        };

        let mut game = Game::new(clock, manager, vec![team1, team2], players, vec![], vec![]);
        game.league = Some(league);
        game
    }

    #[test]
    fn game_lifecycle_supports_set_get_and_clear() {
        let state = StateManager::new();
        assert_eq!(state.get_game(|game| game.teams.len()), None);

        state.set_game(make_game_with_fixture());

        assert_eq!(
            state.get_game(|game| game.manager.team_id.clone()),
            Some(Some("team1".to_string()))
        );

        state.clear_game();

        assert_eq!(state.get_game(|game| game.teams.len()), None);
    }

    #[test]
    fn save_id_lifecycle_supports_set_get_and_clear() {
        let state = StateManager::new();
        assert_eq!(state.get_save_id(), None);

        state.set_save_id("save-1".to_string());
        assert_eq!(state.get_save_id(), Some("save-1".to_string()));

        state.clear_save_id();
        assert_eq!(state.get_save_id(), None);
    }

    #[test]
    fn live_match_lifecycle_supports_mutation_and_take() {
        let state = StateManager::new();
        let game = make_game_with_fixture();
        let session =
            live_match_manager::create_live_match(&game, 0, MatchMode::Spectator, false).unwrap();

        assert!(state.take_live_match().is_none());

        state.set_live_match(session);

        let mode = state
            .with_live_match(|live_match| {
                live_match.mode = MatchMode::Instant;
                live_match.mode
            })
            .unwrap();
        assert_eq!(mode, MatchMode::Instant);

        let taken = state.take_live_match().unwrap();
        assert_eq!(taken.mode, MatchMode::Instant);
        assert!(state.take_live_match().is_none());
        assert!(state.with_live_match(|_| ()).is_none());
    }
}
