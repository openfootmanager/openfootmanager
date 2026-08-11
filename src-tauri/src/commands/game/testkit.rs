//! Test fixtures shared across the game-command submodules.
//!
//! These build whole worlds — a bootstrap-ready `Game`, an imported roster
//! baseline, a historical snapshot — and they are what several clusters of
//! tests have in common. They live here rather than in any one submodule so
//! that the peels of `game.rs` can take their tests with them without either
//! duplicating a fixture or reaching sideways into a sibling.
//!
//! Reach them as `use crate::commands::game::testkit::*;` — an absolute path,
//! so it survives a submodule being nested further later on.

use domain::{
    league::{FixtureCompetition, League},
    manager::Manager,
    news::{NewsArticle, NewsCategory},
    stats::{PlayerMatchStatsRecord, TeamMatchStatsRecord},
    world_history::{HistoricalSeasonAwardsRecord, WorldHistoryArchive},
};
use ofm_core::{
    clock::GameClock,
    game::Game,
    generator::{WorldData, WorldDataKind, WorldDataMetadata},
};

use super::start_date_for_year;

pub(super) fn manager_for(team_id: &str) -> Manager {
    let mut manager = Manager::new(
        "mgr".to_string(),
        "A".to_string(),
        "B".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire(team_id.to_string());
    manager
}

pub(super) fn nation_team(id: &str, nation: &str, reputation: u32) -> domain::team::Team {
    let mut team = domain::team::Team::new(
        id.to_string(),
        id.to_string(),
        id.to_string(),
        nation.to_string(),
        "City".to_string(),
        "Stadium".to_string(),
        10_000,
    );
    team.football_nation = nation.to_string();
    team.reputation = reputation;
    team
}

pub(super) fn default_player_attributes() -> domain::player::PlayerAttributes {
    domain::player::PlayerAttributes {
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
        aggression: 50,
        teamwork: 60,
        leadership: 50,
        handling: 20,
        reflexes: 20,
        aerial: 60,
    }
}

pub(super) fn make_bootstrap_test_game() -> Game {
    let clock = GameClock::new(start_date_for_year(2032).unwrap());
    let manager = domain::manager::Manager::new(
        "mgr-user".to_string(),
        "Alex".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    let teams = vec![
        domain::team::Team::new(
            "team1".to_string(),
            "Alpha FC".to_string(),
            "AFC".to_string(),
            "England".to_string(),
            "London".to_string(),
            "Alpha Park".to_string(),
            20_000,
        ),
        domain::team::Team::new(
            "team2".to_string(),
            "Beta FC".to_string(),
            "BFC".to_string(),
            "England".to_string(),
            "Manchester".to_string(),
            "Beta Park".to_string(),
            22_000,
        ),
    ];
    let staff = vec![
        {
            let mut staff = domain::staff::Staff::new(
                "staff1".to_string(),
                "Pat".to_string(),
                "Coach".to_string(),
                "1978-01-01".to_string(),
                domain::staff::StaffRole::AssistantManager,
                domain::staff::StaffAttributes {
                    coaching: 70,
                    judging_ability: 65,
                    judging_potential: 64,
                    physiotherapy: 40,
                },
            );
            staff.nationality = "England".to_string();
            staff.team_id = Some("team1".to_string());
            staff
        },
        {
            let mut staff = domain::staff::Staff::new(
                "staff2".to_string(),
                "Lee".to_string(),
                "Coach".to_string(),
                "1979-01-01".to_string(),
                domain::staff::StaffRole::AssistantManager,
                domain::staff::StaffAttributes {
                    coaching: 72,
                    judging_ability: 66,
                    judging_potential: 65,
                    physiotherapy: 39,
                },
            );
            staff.nationality = "England".to_string();
            staff.team_id = Some("team2".to_string());
            staff
        },
    ];

    let mut players = Vec::new();
    for team_id in ["team1", "team2"] {
        for index in 0..11 {
            let position = if index == 0 {
                domain::player::Position::Goalkeeper
            } else if index < 5 {
                domain::player::Position::Defender
            } else if index < 8 {
                domain::player::Position::Midfielder
            } else {
                domain::player::Position::Forward
            };
            let mut player = domain::player::Player::new(
                format!("{}-player-{}", team_id, index),
                format!("{} P{}", team_id, index),
                format!("{} Player {}", team_id, index),
                // 1990..=2000. The obvious `format!("199{index}")` breaks at the
                // eleventh player of each side, which parses as the year 19910.
                format!("{}-01-01", 1990 + index),
                "England".to_string(),
                position,
                default_player_attributes(),
            );
            player.team_id = Some(team_id.to_string());
            player.ovr = 62 + index as u8;
            player.potential = 68 + index as u8;
            players.push(player);
        }
    }

    Game::new(clock, manager, teams, players, staff, vec![])
}

pub(super) fn temp_pkg_dir(tag: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("ofm-pkg-cmd-{tag}-{nanos}"));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

pub(super) fn sample_stats_state() -> domain::stats::StatsState {
    domain::stats::StatsState {
        player_matches: vec![PlayerMatchStatsRecord {
            fixture_id: "fixture-1".to_string(),
            season: 2031,
            matchday: 12,
            date: "2031-11-20".to_string(),
            competition: FixtureCompetition::League,
            player_id: "team1-player-0".to_string(),
            team_id: "team1".to_string(),
            opponent_team_id: "team2".to_string(),
            home_team_id: "team1".to_string(),
            away_team_id: "team2".to_string(),
            home_goals: 2,
            away_goals: 1,
            minutes_played: 90,
            goals: 1,
            assists: 0,
            shots: 4,
            shots_on_target: 2,
            passes_completed: 30,
            passes_attempted: 35,
            tackles_won: 1,
            interceptions: 1,
            fouls_committed: 0,
            yellow_cards: 0,
            red_cards: 0,
            rating: 7.5,
        }],
        team_matches: vec![TeamMatchStatsRecord {
            fixture_id: "fixture-1".to_string(),
            season: 2031,
            matchday: 12,
            date: "2031-11-20".to_string(),
            competition: FixtureCompetition::League,
            team_id: "team1".to_string(),
            opponent_team_id: "team2".to_string(),
            home_team_id: "team1".to_string(),
            away_team_id: "team2".to_string(),
            goals_for: 2,
            goals_against: 1,
            possession_pct: 53,
            shots: 11,
            shots_on_target: 6,
            passes_completed: 310,
            passes_attempted: 360,
            tackles_won: 15,
            interceptions: 9,
            fouls_committed: 7,
            yellow_cards: 1,
            red_cards: 0,
        }],
    }
}

pub(super) fn make_imported_baseline_world_without_staff() -> WorldData {
    let teams = vec![
        domain::team::Team::new(
            "team1".to_string(),
            "Alpha FC".to_string(),
            "AFC".to_string(),
            "England".to_string(),
            "London".to_string(),
            "Alpha Park".to_string(),
            20_000,
        ),
        domain::team::Team::new(
            "team2".to_string(),
            "Beta FC".to_string(),
            "BFC".to_string(),
            "England".to_string(),
            "Manchester".to_string(),
            "Beta Park".to_string(),
            22_000,
        ),
    ];

    let mut players = Vec::new();
    for team in &teams {
        let make_player = |id: String, position: domain::player::Position, date_of_birth: &str| {
            let mut player = domain::player::Player::new(
                id.clone(),
                format!("{id} Match"),
                format!("{id} Full"),
                date_of_birth.to_string(),
                "England".to_string(),
                position,
                default_player_attributes(),
            );
            player.team_id = Some(team.id.clone());
            player.ovr = 62;
            player.potential = 68;
            player
        };

        players.push(make_player(
            format!("{}-gk", team.id),
            domain::player::Position::Goalkeeper,
            "1998-01-01",
        ));
        players.push(make_player(
            format!("{}-def-youth", team.id),
            domain::player::Position::Defender,
            "2014-01-01",
        ));
        players.push(make_player(
            format!("{}-mid-youth", team.id),
            domain::player::Position::Midfielder,
            "2013-01-01",
        ));
        players.push(make_player(
            format!("{}-fwd-youth", team.id),
            domain::player::Position::Forward,
            "2012-01-01",
        ));
        for index in 0..8 {
            players.push(make_player(
                format!("{}-senior-{index}", team.id),
                domain::player::Position::Defender,
                "1997-01-01",
            ));
        }
    }

    WorldData {
        name: "Imported Baseline".to_string(),
        description: "No staff import".to_string(),
        teams,
        players,
        staff: vec![],
        managers: vec![],
        league: None,
        news: vec![],
        stats: domain::stats::StatsState::default(),
        world_history: WorldHistoryArchive::default(),
        metadata: WorldDataMetadata::default(),
        ..Default::default()
    }
}

pub(super) fn make_historical_snapshot_world() -> WorldData {
    let base_game = make_bootstrap_test_game();
    let mut league = League::new(
        "league-1".to_string(),
        "Premier Division".to_string(),
        2031,
        &["team1".to_string(), "team2".to_string()],
    );
    league.standings = vec![
        domain::league::StandingEntry {
            team_id: "team1".to_string(),
            played: 12,
            won: 7,
            drawn: 3,
            lost: 2,
            goals_for: 18,
            goals_against: 10,
            points: 24,
        },
        domain::league::StandingEntry {
            team_id: "team2".to_string(),
            played: 12,
            won: 5,
            drawn: 2,
            lost: 5,
            goals_for: 14,
            goals_against: 15,
            points: 17,
        },
    ];

    let mut incumbent = domain::manager::Manager::new(
        "mgr-incumbent".to_string(),
        "Jordan".to_string(),
        "Incumbent".to_string(),
        "1974-01-01".to_string(),
        "England".to_string(),
    );
    incumbent.hire("team1".to_string());

    let mut teams = base_game.teams.clone();
    teams[0].manager_id = Some(incumbent.id.clone());

    let mut archive = WorldHistoryArchive::default();
    archive.record_season_awards(HistoricalSeasonAwardsRecord {
        season: 2030,
        golden_boot: None,
        assist_king: None,
        player_of_year: None,
        clean_sheet_king: None,
        most_appearances: None,
        young_player: None,
        manager_of_season: None,
    });

    WorldData {
        name: "Historical Snapshot".to_string(),
        description: "Season already underway".to_string(),
        teams,
        players: base_game.players,
        staff: base_game.staff,
        managers: vec![incumbent],
        competitions: Vec::new(),
        competition_definitions: None,
        national_teams: Vec::new(),
        regions: Vec::new(),
        default_active_regions: Vec::new(),
        default_active_competitions: Vec::new(),
        league: Some(league),
        news: vec![NewsArticle::new(
            "news-1".to_string(),
            "Season underway".to_string(),
            "The campaign has begun.".to_string(),
            "World Feed".to_string(),
            "2031-11-20".to_string(),
            NewsCategory::StandingsUpdate,
        )],
        stats: sample_stats_state(),
        world_history: archive,
        metadata: WorldDataMetadata {
            format_version: 2,
            world_id: "historical-snapshot".to_string(),
            kind: WorldDataKind::HistoricalSnapshot,
            base_year: Some(2031),
            snapshot_date: Some("2031-11-20T00:00:00Z".to_string()),
        },
        extra_translations: std::collections::HashMap::new(),
        build_notices: Vec::new(),
    }
}
