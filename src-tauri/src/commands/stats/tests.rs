use chrono::{TimeZone, Utc};
use domain::league::{FixtureCompetition, League, StandingEntry};
use domain::manager::Manager;
use domain::player::{Player, PlayerAttributes, PlayerSeasonStats, Position};
use domain::stats::{PlayerMatchStatsRecord, StatsState, TeamMatchStatsRecord};
use domain::team::Team;
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::state::StateManager;

use super::player::{get_player_match_history_internal, get_player_stats_overview_internal};
use super::shared::{percentile_rank, AdvancedStat, StatDirection};
use super::team::{get_team_match_history_internal, get_team_stats_overview_internal};

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

fn make_player(id: &str, team_id: &str, natural_position: Position) -> Player {
    let mut player = Player::new(
        id.to_string(),
        id.to_string(),
        id.to_string(),
        "2000-01-01".to_string(),
        "England".to_string(),
        natural_position.clone(),
        default_attrs(),
    );
    player.team_id = Some(team_id.to_string());
    player.natural_position = natural_position;
    player
}

fn make_game(players: Vec<Player>) -> Game {
    let clock = GameClock::new(Utc.with_ymd_and_hms(2025, 7, 1, 0, 0, 0).unwrap());
    let mut manager = Manager::new(
        "mgr-1".to_string(),
        "Alex".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("team-1".to_string());

    let mut team = Team::new(
        "team-1".to_string(),
        "Alpha FC".to_string(),
        "ALP".to_string(),
        "England".to_string(),
        "Alpha City".to_string(),
        "Alpha Ground".to_string(),
        20_000,
    );
    team.starting_xi_ids = players.iter().map(|player| player.id.clone()).collect();

    let opponent = Team::new(
        "team-2".to_string(),
        "Bravo FC".to_string(),
        "BRV".to_string(),
        "England".to_string(),
        "Bravo City".to_string(),
        "Bravo Ground".to_string(),
        18_000,
    );

    let mut game = Game::new(
        clock,
        manager,
        vec![team, opponent],
        players,
        vec![],
        vec![],
    );
    game.competitions = vec![league("league-1", &["team-1", "team-2"])];
    game
}

fn league(id: &str, team_ids: &[&str]) -> League {
    League {
        id: id.to_string(),
        name: format!("{id} name"),
        participant_ids: team_ids.iter().map(|team| (*team).to_string()).collect(),
        standings: team_ids
            .iter()
            .map(|team| StandingEntry::new((*team).to_string()))
            .collect(),
        ..Default::default()
    }
}

fn sample_stats_state() -> StatsState {
    StatsState {
        player_matches: vec![
            PlayerMatchStatsRecord {
                fixture_id: "fixture-older".to_string(),
                season: 2025,
                matchday: 1,
                date: "2025-06-10".to_string(),
                competition: FixtureCompetition::League,
                player_id: "player-1".to_string(),
                team_id: "team-1".to_string(),
                opponent_team_id: "team-2".to_string(),
                home_team_id: "team-1".to_string(),
                away_team_id: "team-2".to_string(),
                home_goals: 2,
                away_goals: 1,
                minutes_played: 90,
                goals: 1,
                assists: 0,
                shots: 4,
                shots_on_target: 2,
                passes_completed: 20,
                passes_attempted: 24,
                tackles_won: 1,
                interceptions: 0,
                fouls_committed: 1,
                yellow_cards: 0,
                red_cards: 0,
                rating: 7.2,
            },
            PlayerMatchStatsRecord {
                fixture_id: "fixture-latest".to_string(),
                season: 2025,
                matchday: 2,
                date: "2025-06-17".to_string(),
                competition: FixtureCompetition::League,
                player_id: "player-1".to_string(),
                team_id: "team-1".to_string(),
                opponent_team_id: "team-3".to_string(),
                home_team_id: "team-3".to_string(),
                away_team_id: "team-1".to_string(),
                home_goals: 0,
                away_goals: 3,
                minutes_played: 88,
                goals: 2,
                assists: 1,
                shots: 5,
                shots_on_target: 3,
                passes_completed: 24,
                passes_attempted: 28,
                tackles_won: 2,
                interceptions: 1,
                fouls_committed: 0,
                yellow_cards: 0,
                red_cards: 0,
                rating: 8.4,
            },
            PlayerMatchStatsRecord {
                fixture_id: "fixture-other-player".to_string(),
                season: 2025,
                matchday: 2,
                date: "2025-06-17".to_string(),
                competition: FixtureCompetition::League,
                player_id: "player-2".to_string(),
                team_id: "team-1".to_string(),
                opponent_team_id: "team-3".to_string(),
                home_team_id: "team-3".to_string(),
                away_team_id: "team-1".to_string(),
                home_goals: 0,
                away_goals: 3,
                minutes_played: 90,
                goals: 0,
                assists: 0,
                shots: 1,
                shots_on_target: 0,
                passes_completed: 40,
                passes_attempted: 48,
                tackles_won: 4,
                interceptions: 3,
                fouls_committed: 2,
                yellow_cards: 1,
                red_cards: 0,
                rating: 7.0,
            },
        ],
        team_matches: vec![],
    }
}

fn sample_team_stats_state() -> StatsState {
    StatsState {
        player_matches: vec![],
        team_matches: vec![
            TeamMatchStatsRecord {
                fixture_id: "fixture-1".to_string(),
                season: 2025,
                matchday: 1,
                date: "2025-08-01".to_string(),
                competition: FixtureCompetition::League,
                team_id: "team-1".to_string(),
                opponent_team_id: "team-2".to_string(),
                home_team_id: "team-1".to_string(),
                away_team_id: "team-2".to_string(),
                goals_for: 2,
                goals_against: 0,
                possession_pct: 58,
                shots: 14,
                shots_on_target: 6,
                passes_completed: 420,
                passes_attempted: 500,
                tackles_won: 18,
                interceptions: 11,
                fouls_committed: 9,
                yellow_cards: 1,
                red_cards: 0,
            },
            TeamMatchStatsRecord {
                fixture_id: "fixture-2".to_string(),
                season: 2026,
                matchday: 1,
                date: "2026-08-01".to_string(),
                competition: FixtureCompetition::League,
                team_id: "team-1".to_string(),
                opponent_team_id: "team-2".to_string(),
                home_team_id: "team-2".to_string(),
                away_team_id: "team-1".to_string(),
                goals_for: 3,
                goals_against: 1,
                possession_pct: 62,
                shots: 16,
                shots_on_target: 7,
                passes_completed: 460,
                passes_attempted: 540,
                tackles_won: 20,
                interceptions: 13,
                fouls_committed: 10,
                yellow_cards: 2,
                red_cards: 0,
            },
        ],
    }
}

#[test]
fn get_player_stats_overview_aggregates_history_and_uses_exact_position_cohorts() {
    let mut player = make_player("player-1", "team-1", Position::Striker);
    player.stats = PlayerSeasonStats {
        appearances: 1,
        goals: 1,
        assists: 1,
        clean_sheets: 0,
        yellow_cards: 0,
        red_cards: 0,
        avg_rating: 7.0,
        minutes_played: 90,
        shots: 2,
        shots_on_target: 1,
        passes_completed: 10,
        passes_attempted: 12,
        tackles_won: 1,
        interceptions: 0,
        fouls_committed: 1,
    };
    let peer_a = make_player("player-2", "team-1", Position::Striker);
    let peer_b = make_player("player-3", "team-1", Position::Striker);
    let broad_bucket_peer = make_player("player-4", "team-1", Position::Forward);

    let state = StateManager::new();
    state.set_game(make_game(vec![player, peer_a, peer_b, broad_bucket_peer]));
    state.set_stats_state(StatsState {
        player_matches: vec![
            PlayerMatchStatsRecord {
                fixture_id: "fixture-a".to_string(),
                season: 2025,
                matchday: 1,
                date: "2025-08-01".to_string(),
                competition: FixtureCompetition::League,
                player_id: "player-1".to_string(),
                team_id: "team-1".to_string(),
                opponent_team_id: "team-2".to_string(),
                home_team_id: "team-1".to_string(),
                away_team_id: "team-2".to_string(),
                home_goals: 2,
                away_goals: 0,
                minutes_played: 90,
                goals: 1,
                assists: 1,
                shots: 6,
                shots_on_target: 3,
                passes_completed: 25,
                passes_attempted: 30,
                tackles_won: 4,
                interceptions: 2,
                fouls_committed: 1,
                yellow_cards: 0,
                red_cards: 0,
                rating: 8.1,
            },
            PlayerMatchStatsRecord {
                fixture_id: "fixture-b".to_string(),
                season: 2026,
                matchday: 1,
                date: "2026-08-01".to_string(),
                competition: FixtureCompetition::League,
                player_id: "player-1".to_string(),
                team_id: "team-1".to_string(),
                opponent_team_id: "team-2".to_string(),
                home_team_id: "team-2".to_string(),
                away_team_id: "team-1".to_string(),
                home_goals: 1,
                away_goals: 3,
                minutes_played: 90,
                goals: 2,
                assists: 0,
                shots: 4,
                shots_on_target: 2,
                passes_completed: 20,
                passes_attempted: 25,
                tackles_won: 2,
                interceptions: 1,
                fouls_committed: 2,
                yellow_cards: 0,
                red_cards: 0,
                rating: 8.4,
            },
            PlayerMatchStatsRecord {
                fixture_id: "fixture-peer-a-1".to_string(),
                season: 2025,
                matchday: 1,
                date: "2025-08-01".to_string(),
                competition: FixtureCompetition::League,
                player_id: "player-2".to_string(),
                team_id: "team-1".to_string(),
                opponent_team_id: "team-2".to_string(),
                home_team_id: "team-1".to_string(),
                away_team_id: "team-2".to_string(),
                home_goals: 1,
                away_goals: 0,
                minutes_played: 90,
                goals: 0,
                assists: 0,
                shots: 2,
                shots_on_target: 1,
                passes_completed: 18,
                passes_attempted: 24,
                tackles_won: 1,
                interceptions: 1,
                fouls_committed: 1,
                yellow_cards: 0,
                red_cards: 0,
                rating: 6.9,
            },
            PlayerMatchStatsRecord {
                fixture_id: "fixture-peer-a-2".to_string(),
                season: 2026,
                matchday: 1,
                date: "2026-08-01".to_string(),
                competition: FixtureCompetition::League,
                player_id: "player-2".to_string(),
                team_id: "team-1".to_string(),
                opponent_team_id: "team-2".to_string(),
                home_team_id: "team-2".to_string(),
                away_team_id: "team-1".to_string(),
                home_goals: 1,
                away_goals: 1,
                minutes_played: 90,
                goals: 1,
                assists: 0,
                shots: 3,
                shots_on_target: 1,
                passes_completed: 20,
                passes_attempted: 28,
                tackles_won: 2,
                interceptions: 1,
                fouls_committed: 2,
                yellow_cards: 0,
                red_cards: 0,
                rating: 7.1,
            },
            PlayerMatchStatsRecord {
                fixture_id: "fixture-peer-b-1".to_string(),
                season: 2025,
                matchday: 1,
                date: "2025-08-01".to_string(),
                competition: FixtureCompetition::League,
                player_id: "player-3".to_string(),
                team_id: "team-1".to_string(),
                opponent_team_id: "team-2".to_string(),
                home_team_id: "team-1".to_string(),
                away_team_id: "team-2".to_string(),
                home_goals: 0,
                away_goals: 0,
                minutes_played: 90,
                goals: 0,
                assists: 0,
                shots: 3,
                shots_on_target: 1,
                passes_completed: 19,
                passes_attempted: 24,
                tackles_won: 2,
                interceptions: 1,
                fouls_committed: 1,
                yellow_cards: 0,
                red_cards: 0,
                rating: 7.0,
            },
            PlayerMatchStatsRecord {
                fixture_id: "fixture-peer-b-2".to_string(),
                season: 2026,
                matchday: 1,
                date: "2026-08-01".to_string(),
                competition: FixtureCompetition::League,
                player_id: "player-3".to_string(),
                team_id: "team-1".to_string(),
                opponent_team_id: "team-2".to_string(),
                home_team_id: "team-2".to_string(),
                away_team_id: "team-1".to_string(),
                home_goals: 0,
                away_goals: 1,
                minutes_played: 90,
                goals: 0,
                assists: 0,
                shots: 4,
                shots_on_target: 2,
                passes_completed: 18,
                passes_attempted: 22,
                tackles_won: 1,
                interceptions: 1,
                fouls_committed: 1,
                yellow_cards: 0,
                red_cards: 0,
                rating: 7.2,
            },
            PlayerMatchStatsRecord {
                fixture_id: "fixture-forward-only-1".to_string(),
                season: 2025,
                matchday: 1,
                date: "2025-08-01".to_string(),
                competition: FixtureCompetition::League,
                player_id: "player-4".to_string(),
                team_id: "team-1".to_string(),
                opponent_team_id: "team-2".to_string(),
                home_team_id: "team-1".to_string(),
                away_team_id: "team-2".to_string(),
                home_goals: 1,
                away_goals: 1,
                minutes_played: 90,
                goals: 0,
                assists: 0,
                shots: 30,
                shots_on_target: 12,
                passes_completed: 40,
                passes_attempted: 45,
                tackles_won: 1,
                interceptions: 0,
                fouls_committed: 2,
                yellow_cards: 0,
                red_cards: 0,
                rating: 7.3,
            },
            PlayerMatchStatsRecord {
                fixture_id: "fixture-forward-only-2".to_string(),
                season: 2026,
                matchday: 1,
                date: "2026-08-01".to_string(),
                competition: FixtureCompetition::League,
                player_id: "player-4".to_string(),
                team_id: "team-1".to_string(),
                opponent_team_id: "team-2".to_string(),
                home_team_id: "team-2".to_string(),
                away_team_id: "team-1".to_string(),
                home_goals: 0,
                away_goals: 1,
                minutes_played: 90,
                goals: 0,
                assists: 0,
                shots: 30,
                shots_on_target: 14,
                passes_completed: 42,
                passes_attempted: 48,
                tackles_won: 1,
                interceptions: 0,
                fouls_committed: 2,
                yellow_cards: 0,
                red_cards: 0,
                rating: 7.4,
            },
        ],
        team_matches: vec![],
    });

    let overview = get_player_stats_overview_internal(&state, "player-1").unwrap();

    assert!(overview.percentile_eligible);
    assert_eq!(overview.metrics.shots.total, 10);
    assert_eq!(overview.metrics.shots.per90, Some(5.0));
    assert_eq!(overview.metrics.shots.percentile, Some(100));
    assert_eq!(overview.metrics.passes.completed, 45);
    assert_eq!(overview.metrics.passes.attempted, 55);
    assert_eq!(overview.metrics.passes.accuracy, Some(81.8));
    assert_eq!(overview.metrics.passes.percentile, Some(100));
}

#[test]
fn get_player_stats_overview_falls_back_to_current_season_totals_for_legacy_saves() {
    let mut player = make_player("player-1", "team-1", Position::Striker);
    player.stats = PlayerSeasonStats {
        appearances: 10,
        goals: 4,
        assists: 3,
        clean_sheets: 0,
        yellow_cards: 1,
        red_cards: 0,
        avg_rating: 7.2,
        minutes_played: 450,
        shots: 20,
        shots_on_target: 10,
        passes_completed: 80,
        passes_attempted: 100,
        tackles_won: 9,
        interceptions: 6,
        fouls_committed: 5,
    };
    let mut peer_a = make_player("player-2", "team-1", Position::Striker);
    peer_a.stats = PlayerSeasonStats {
        appearances: 10,
        goals: 2,
        assists: 2,
        clean_sheets: 0,
        yellow_cards: 0,
        red_cards: 0,
        avg_rating: 7.0,
        minutes_played: 450,
        shots: 10,
        shots_on_target: 5,
        passes_completed: 70,
        passes_attempted: 100,
        tackles_won: 6,
        interceptions: 4,
        fouls_committed: 3,
    };
    let mut peer_b = make_player("player-3", "team-1", Position::Striker);
    peer_b.stats = PlayerSeasonStats {
        appearances: 10,
        goals: 3,
        assists: 2,
        clean_sheets: 0,
        yellow_cards: 0,
        red_cards: 0,
        avg_rating: 7.0,
        minutes_played: 450,
        shots: 15,
        shots_on_target: 8,
        passes_completed: 75,
        passes_attempted: 100,
        tackles_won: 7,
        interceptions: 5,
        fouls_committed: 4,
    };

    let state = StateManager::new();
    state.set_game(make_game(vec![player, peer_a, peer_b]));

    let overview = get_player_stats_overview_internal(&state, "player-1").unwrap();

    assert!(overview.percentile_eligible);
    assert_eq!(overview.metrics.shots.total, 20);
    assert_eq!(overview.metrics.shots.per90, Some(4.0));
    assert_eq!(overview.metrics.passes.accuracy, Some(80.0));
    assert_eq!(overview.metrics.shots.percentile, Some(100));
}

#[test]
fn get_player_match_history_returns_latest_matches_first_with_limit() {
    let state = StateManager::new();
    state.set_stats_state(sample_stats_state());

    let history = get_player_match_history_internal(&state, "player-1", Some(1)).unwrap();

    assert_eq!(history.len(), 1);
    assert_eq!(history[0].fixture_id, "fixture-latest");
    assert_eq!(history[0].opponent_team_id, "team-3");
    assert_eq!(history[0].goals, 2);
}

/// Given a stored match rating of 0 (no rating computed) and one real rating
/// When the player's match history is read
/// Then only the real rating is flagged as rated
#[test]
fn match_history_flags_which_ratings_exist() {
    let state = StateManager::new();
    let mut stats = sample_stats_state();
    for record in stats
        .player_matches
        .iter_mut()
        .filter(|r| r.player_id == "player-1")
    {
        record.rating = if record.fixture_id == "fixture-latest" {
            0.0
        } else {
            7.2
        };
    }
    state.set_stats_state(stats);

    let history = get_player_match_history_internal(&state, "player-1", Some(10)).unwrap();

    let latest = history
        .iter()
        .find(|e| e.fixture_id == "fixture-latest")
        .unwrap();
    assert!(!latest.rated);
    assert!(history
        .iter()
        .filter(|e| e.fixture_id != "fixture-latest")
        .all(|e| e.rated));
    assert!(history.len() > 1);
}

/// Given a player whose stored season average is 0 and another whose is 7.2
/// When their stats overviews are read
/// Then only the second's season rating is flagged as rated
#[test]
fn stats_overview_flags_whether_the_season_rating_exists() {
    let mut unrated = make_player("player-1", "team-1", Position::Striker);
    unrated.stats.avg_rating = 0.0;
    let mut rated = make_player("player-2", "team-1", Position::Striker);
    rated.stats.avg_rating = 7.2;
    let state = StateManager::new();
    state.set_game(make_game(vec![unrated, rated]));

    let unrated_overview = get_player_stats_overview_internal(&state, "player-1").unwrap();
    let rated_overview = get_player_stats_overview_internal(&state, "player-2").unwrap();

    assert!(!unrated_overview.season_rating_rated);
    assert!(rated_overview.season_rating_rated);
}

/// Given match history exists for a player whose season average is 0 (a new season)
/// When the overview is read
/// Then the season rating is not flagged, whatever the older history holds
#[test]
fn stats_overview_from_history_follows_the_season_average_not_the_history() {
    let mut player = make_player("player-1", "team-1", Position::Striker);
    player.stats.avg_rating = 0.0;
    let state = StateManager::new();
    state.set_game(make_game(vec![player]));
    state.set_stats_state(sample_stats_state());

    let overview = get_player_stats_overview_internal(&state, "player-1").unwrap();

    assert!(!overview.season_rating_rated);
}

#[test]
fn get_player_match_history_returns_empty_when_stats_state_is_missing() {
    let state = StateManager::new();

    let history = get_player_match_history_internal(&state, "player-1", None).unwrap();

    assert!(history.is_empty());
}

#[test]
fn get_team_stats_overview_aggregates_totals_and_match_averages() {
    let state = StateManager::new();
    state.set_game(make_game(vec![make_player(
        "player-1",
        "team-1",
        Position::Striker,
    )]));
    state.set_stats_state(sample_team_stats_state());

    let overview = get_team_stats_overview_internal(&state, "team-1")
        .unwrap()
        .expect("expected team overview");

    assert_eq!(overview.matches_played, 2);
    assert_eq!(overview.goals_for, 5);
    assert_eq!(overview.goals_against, 1);
    assert_eq!(overview.goal_difference, 4);
    assert_eq!(overview.possession_average, Some(60.0));
    assert_eq!(overview.metrics.shots.total, 30);
    assert_eq!(overview.metrics.shots.per_match, Some(15.0));
    assert_eq!(overview.metrics.shots_on_target.total, 13);
    assert_eq!(overview.metrics.shots_on_target.per_match, Some(6.5));
    assert_eq!(overview.metrics.passes.completed, 880);
    assert_eq!(overview.metrics.passes.attempted, 1040);
    assert_eq!(overview.metrics.passes.accuracy, Some(84.6));
    assert_eq!(overview.metrics.tackles_won.per_match, Some(19.0));
}

#[test]
fn get_team_stats_overview_returns_none_without_history() {
    let state = StateManager::new();
    state.set_game(make_game(vec![make_player(
        "player-1",
        "team-1",
        Position::Striker,
    )]));

    let overview = get_team_stats_overview_internal(&state, "team-1").unwrap();

    assert!(overview.is_none());
}

#[test]
fn get_team_match_history_returns_latest_matches_first_with_limit() {
    let state = StateManager::new();
    state.set_game(make_game(vec![make_player(
        "player-1",
        "team-1",
        Position::Striker,
    )]));
    state.set_stats_state(sample_team_stats_state());

    let history = get_team_match_history_internal(&state, "team-1", Some(1)).unwrap();

    assert_eq!(history.len(), 1);
    assert_eq!(history[0].fixture_id, "fixture-2");
    assert_eq!(history[0].opponent_team_id, "team-2");
    assert_eq!(history[0].goals_for, 3);
    assert_eq!(history[0].goals_against, 1);
}

#[test]
fn get_team_match_history_returns_empty_when_stats_state_is_missing() {
    let state = StateManager::new();
    state.set_game(make_game(vec![make_player(
        "player-1",
        "team-1",
        Position::Striker,
    )]));

    let history = get_team_match_history_internal(&state, "team-1", None).unwrap();

    assert!(history.is_empty());
}

fn with_stats(mut player: Player, shots: u32, fouls: u32) -> Player {
    player.stats = PlayerSeasonStats {
        appearances: 10,
        minutes_played: 900,
        shots,
        fouls_committed: fouls,
        ..Default::default()
    };
    player
}

fn overview_of(game: Game, player_id: &str) -> super::dto::PlayerStatsOverviewDto {
    let state = StateManager::new();
    state.set_game(game);
    get_player_stats_overview_internal(&state, player_id).expect("an overview")
}

/// Given a striker in the second division, with strikers in the first division who shoot far less
/// When his shooting percentile is read
/// Then it is measured against strikers of his own league only, and the answer names that league
#[test]
fn the_peer_group_is_the_players_own_league() {
    let players = vec![
        with_stats(make_player("target", "team-3", Position::Striker), 20, 0),
        with_stats(make_player("own-low", "team-3", Position::Striker), 10, 0),
        with_stats(make_player("own-high", "team-4", Position::Striker), 30, 0),
        with_stats(make_player("far-1", "team-1", Position::Striker), 5, 0),
        with_stats(make_player("far-2", "team-2", Position::Striker), 5, 0),
        with_stats(make_player("far-3", "team-1", Position::Striker), 5, 0),
        with_stats(
            make_player("other-position", "team-3", Position::Midfielder),
            1,
            0,
        ),
    ];
    let mut game = make_game(players);
    game.competitions
        .push(league("league-2", &["team-3", "team-4"]));

    let overview = overview_of(game, "target");

    let group = overview.peer_group.expect("a peer group");
    assert_eq!(group.competition_id, "league-2");
    assert_eq!(group.position, "Striker");
    // 2 of the 3 league-2 strikers shoot no more than he does. Against the whole world it
    // would be 5 of 6.
    assert_eq!(overview.metrics.shots.percentile, Some(67));
}

/// Given two midfielders in one league, one with 10 fouls and one with 40 (and a third in between)
/// When their fouls percentiles are read
/// Then the player with fewer fouls has the higher percentile
#[test]
fn fewer_fouls_rank_higher() {
    let players = vec![
        with_stats(make_player("clean", "team-1", Position::Midfielder), 0, 10),
        with_stats(
            make_player("middling", "team-1", Position::Midfielder),
            0,
            25,
        ),
        with_stats(make_player("dirty", "team-2", Position::Midfielder), 0, 40),
    ];

    let clean = overview_of(make_game(players.clone()), "clean");
    let dirty = overview_of(make_game(players), "dirty");

    assert_eq!(clean.metrics.fouls_committed.percentile, Some(100));
    assert_eq!(dirty.metrics.fouls_committed.percentile, Some(33));
}

/// Given the stats that carry a percentile
/// When each one's direction is asked
/// Then fouls are the only stat where lower is better, from the one definition the backend ranks by
#[test]
fn every_stat_declares_its_direction() {
    for stat in AdvancedStat::ALL {
        let expected = if stat == AdvancedStat::FoulsCommitted {
            StatDirection::LowerIsBetter
        } else {
            StatDirection::HigherIsBetter
        };
        assert_eq!(stat.direction(), expected, "{stat:?}");
    }
}

/// Given the same peer values and a player in the middle
/// When ranked in each direction
/// Then higher-is-better counts the peers at or below him, and lower-is-better those at or above
#[test]
fn the_rank_follows_the_stat_direction() {
    let values = [1.0, 2.0, 3.0, 4.0];

    assert_eq!(
        percentile_rank(&values, Some(3.0), StatDirection::HigherIsBetter),
        Some(75)
    );
    assert_eq!(
        percentile_rank(&values, Some(3.0), StatDirection::LowerIsBetter),
        Some(50)
    );
}

/// Given a player at no club
/// When his overview is read
/// Then there is no peer group and no percentile, rather than a ranking against strangers
#[test]
fn a_player_without_a_league_has_no_percentiles() {
    let mut free_agent = with_stats(make_player("free", "team-1", Position::Striker), 20, 0);
    free_agent.team_id = None;
    let players = vec![
        free_agent,
        with_stats(make_player("a", "team-1", Position::Striker), 10, 0),
        with_stats(make_player("b", "team-1", Position::Striker), 30, 0),
        with_stats(make_player("c", "team-1", Position::Striker), 5, 0),
    ];

    let overview = overview_of(make_game(players), "free");

    assert!(overview.peer_group.is_none());
    assert!(!overview.percentile_eligible);
    assert_eq!(overview.metrics.shots.percentile, None);
}
