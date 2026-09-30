//! End-to-end tests for the game commands.
//!
//! Every test here drives a whole career opening: load or build a world, found
//! its competitions, bootstrap a manager into it, and assert on the result.
//! They live together rather than in the submodule they happen to touch first
//! because each one crosses three or more of them, and splitting them by first
//! contact would make the seams look tighter than they are.

use super::testkit::*;
use super::{
    bootstrap_team_selection, build_game_from_world_data, game_clock_for_world, StartPhase,
    StartupOptions, DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
};
use domain::news::NewsCategory;

#[test]
#[ignore = "perf harness; run: cargo test -p openfootmanager perf_baseline -- --ignored --nocapture"]
fn perf_baseline() {
    use std::time::Instant;

    let t = Instant::now();
    let world = ofm_core::generator::generate_world_data(
        &ofm_core::generator::DefinitionSources::embedded_only(),
    );
    let gen = t.elapsed();
    let teams = world.teams.len();
    let players = world.players.len();

    let manager = domain::manager::Manager::new(
        "mgr-user".to_string(),
        "Alex".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    let startup_options = StartupOptions {
        start_year: 2026,
        start_phase: StartPhase::SeasonStart,
        history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
    };
    let clock = game_clock_for_world(&startup_options, &world.metadata).unwrap();

    let t = Instant::now();
    let (mut game, _stats) = build_game_from_world_data(clock, manager, &startup_options, world);
    let build = t.elapsed();

    let competitions = game.competitions.len();
    let active = game.active_competition_ids.len();

    const DAYS: u32 = 30;
    let t = Instant::now();
    for _ in 0..DAYS {
        ofm_core::turn::process_day(&mut game);
    }
    let days = t.elapsed();

    eprintln!(
        "PERF teams={teams} players={players} competitions={competitions} active_competition_ids={active}"
    );
    eprintln!("PERF world-gen         = {gen:?}");
    eprintln!("PERF build-game        = {build:?}  (foundations + history)");
    eprintln!(
        "PERF {DAYS}x process_day   = {days:?}  ({:?}/day)",
        days / DAYS
    );
}

#[test]
fn historical_snapshot_startup_preserves_league_news_history_and_stats() {
    let manager = domain::manager::Manager::new(
        "mgr-user".to_string(),
        "Alex".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    let startup_options = StartupOptions {
        start_year: 2032,
        start_phase: StartPhase::MidSeason,
        history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
    };
    let world = make_historical_snapshot_world();
    let clock = game_clock_for_world(&startup_options, &world.metadata).unwrap();

    let (game, stats_state) = build_game_from_world_data(clock, manager, &startup_options, world);

    assert_eq!(
        game.clock.start_date.to_rfc3339(),
        "2031-07-01T00:00:00+00:00"
    );
    assert_eq!(
        game.clock.current_date.to_rfc3339(),
        "2031-11-20T00:00:00+00:00"
    );
    assert_eq!(game.league.as_ref().map(|league| league.season), Some(2031));
    assert_eq!(game.news.len(), 1);
    assert_eq!(game.world_history.season_awards.len(), 1);
    assert_eq!(stats_state.team_matches.len(), 1);
    assert_eq!(stats_state.player_matches.len(), 1);
    assert!(game
        .managers
        .iter()
        .any(|manager| manager.id == "mgr-incumbent"));
}

#[test]
fn imported_roster_baseline_bootstrap_backfills_staff_market_and_opening_youth() {
    let manager = domain::manager::Manager::new(
        "mgr-user".to_string(),
        "Alex".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    let startup_options = StartupOptions {
        start_year: 2032,
        start_phase: StartPhase::SeasonStart,
        history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
    };
    let mut world = make_imported_baseline_world_without_staff();
    ofm_core::generator::normalize_imported_world_for_career_start(
        &mut world,
        startup_options.start_year as u32,
    );
    let clock = game_clock_for_world(&startup_options, &world.metadata).unwrap();

    let (game, stats_state) = build_game_from_world_data(clock, manager, &startup_options, world);

    assert!(stats_state.team_matches.is_empty());
    assert_eq!(
        game.staff
            .iter()
            .filter(|staff_member| staff_member.team_id.is_none())
            .count(),
        12
    );
    for team_id in ["team1", "team2"] {
        for role in [
            domain::staff::StaffRole::AssistantManager,
            domain::staff::StaffRole::Coach,
            domain::staff::StaffRole::Scout,
            domain::staff::StaffRole::Physio,
        ] {
            let count = game
                .staff
                .iter()
                .filter(|staff_member| {
                    staff_member.team_id.as_deref() == Some(team_id) && staff_member.role == role
                })
                .count();
            assert_eq!(count, 1);
        }
        let youth_count = game
            .players
            .iter()
            .filter(|player| {
                player.team_id.as_deref() == Some(team_id)
                    && player.squad_role == domain::player::SquadRole::Youth
            })
            .count();
        assert_eq!(youth_count, 3);
    }
    assert_eq!(
        game.available_staff_market_last_activity_date.as_deref(),
        Some("2032-07-01")
    );
}

#[test]
fn imported_roster_baseline_bootstrap_allows_ai_manager_seeding_without_imported_staff() {
    let manager = domain::manager::Manager::new(
        "mgr-user".to_string(),
        "Alex".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    let startup_options = StartupOptions {
        start_year: 2032,
        start_phase: StartPhase::SeasonStart,
        history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
    };
    let mut world = make_imported_baseline_world_without_staff();
    ofm_core::generator::normalize_imported_world_for_career_start(
        &mut world,
        startup_options.start_year as u32,
    );
    let clock = game_clock_for_world(&startup_options, &world.metadata).unwrap();
    let (mut game, stats_state) =
        build_game_from_world_data(clock, manager, &startup_options, world);

    bootstrap_team_selection(&mut game, "team1", StartPhase::SeasonStart, stats_state).unwrap();

    assert_eq!(
        game.teams
            .iter()
            .find(|team| team.id == "team1")
            .and_then(|team| team.manager_id.as_deref()),
        Some("mgr-user")
    );
    assert!(game
        .teams
        .iter()
        .filter(|team| team.id != "team1")
        .all(|team| team.manager_id.is_some()));
}

#[test]
fn bootstrap_team_selection_seeds_ai_loan_market() {
    let mut game = make_bootstrap_test_game();
    game.teams
        .iter_mut()
        .find(|team| team.id == "team2")
        .unwrap()
        .starting_xi_ids = (0..11)
        .map(|index| format!("team2-player-{index}"))
        .collect();

    for (id, date_of_birth) in [
        ("team2-loan-1", "2007-01-01"),
        ("team2-loan-2", "2006-01-01"),
        ("team2-loan-3", "2005-01-01"),
    ] {
        let mut player = domain::player::Player::new(
            id.to_string(),
            id.to_string(),
            id.to_string(),
            date_of_birth.to_string(),
            "England".to_string(),
            domain::player::Position::Midfielder,
            default_player_attributes(),
        );
        player.team_id = Some("team2".to_string());
        player.contract_end = Some("2035-06-30".to_string());
        game.players.push(player);
    }

    bootstrap_team_selection(
        &mut game,
        "team1",
        StartPhase::SeasonStart,
        domain::stats::StatsState::default(),
    )
    .unwrap();

    assert_eq!(
        game.players
            .iter()
            .filter(|player| { player.team_id.as_deref() == Some("team2") && player.loan_listed })
            .count(),
        2
    );
    assert!(game
        .players
        .iter()
        .filter(|player| player.team_id.as_deref() == Some("team1"))
        .all(|player| !player.loan_listed));
}

#[test]
fn imported_historical_snapshot_preserves_state_while_backfilling_staff() {
    let manager = domain::manager::Manager::new(
        "mgr-user".to_string(),
        "Alex".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    let startup_options = StartupOptions {
        start_year: 2032,
        start_phase: StartPhase::MidSeason,
        history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
    };
    let mut world = make_historical_snapshot_world();
    world.staff.clear();
    let original_news_len = world.news.len();
    let original_season = world.league.as_ref().map(|league| league.season);
    let original_awards = world.world_history.season_awards.len();
    ofm_core::generator::normalize_imported_world_for_career_start(
        &mut world,
        startup_options.start_year as u32,
    );
    let clock = game_clock_for_world(&startup_options, &world.metadata).unwrap();

    let (game, stats_state) = build_game_from_world_data(clock, manager, &startup_options, world);

    assert_eq!(
        game.league.as_ref().map(|league| league.season),
        original_season
    );
    assert_eq!(game.news.len(), original_news_len);
    assert_eq!(game.world_history.season_awards.len(), original_awards);
    assert_eq!(stats_state.team_matches.len(), 1);
    assert_eq!(
        game.staff
            .iter()
            .filter(|staff_member| staff_member.team_id.is_none())
            .count(),
        12
    );
    for team_id in ["team1", "team2"] {
        let has_assistant = game.staff.iter().any(|staff_member| {
            staff_member.team_id.as_deref() == Some(team_id)
                && staff_member.role == domain::staff::StaffRole::AssistantManager
        });
        assert!(has_assistant);
    }
}

#[test]
fn embedded_competition_definitions_replace_the_auto_built_competitions() {
    use ofm_core::generator::{
        CompetitionDefinition, CompetitionDefinitionFile, FormatDef, ParticipantSpec,
    };

    let manager = domain::manager::Manager::new(
        "mgr-user".to_string(),
        "Alex".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    let startup_options = StartupOptions {
        start_year: 2032,
        start_phase: StartPhase::MidSeason,
        history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
    };
    let mut world = make_historical_snapshot_world();
    let team_ids: Vec<String> = world.teams.iter().map(|t| t.id.clone()).collect();
    assert!(team_ids.len() >= 2);
    world.competition_definitions = Some(CompetitionDefinitionFile {
        format_version: 1,
        competitions: vec![CompetitionDefinition {
            id: "custom-league".to_string(),
            name: "Custom League".to_string(),
            r#type: domain::league::CompetitionType::League,
            scope: domain::league::CompetitionScope::Domestic,
            region_id: None,
            country_id: None,
            required_region_ids: vec![],
            priority: 0,
            format: FormatDef {
                kind: domain::league::CompetitionFormat::LeagueTable,
                legs: None,
                group_size: None,
                qualifiers_per_group: None,
                best_third_qualifiers: None,
            },
            participants: ParticipantSpec {
                explicit: Some(team_ids.clone()),
                selector: None,
            },
            berths: Vec::new(),
            season_start_month: None,
            season_start_day: None,
            name_key: None,
            logo: None,
        }],
    });
    let clock = game_clock_for_world(&startup_options, &world.metadata).unwrap();

    let (game, _stats) = build_game_from_world_data(clock, manager, &startup_options, world);

    let custom = game
        .competitions
        .iter()
        .find(|c| c.id == "custom-league")
        .expect("authored competition replaces the auto-built ones");
    assert_eq!(custom.participant_ids, team_ids);
    assert!(
        game.competitions.iter().all(|c| c.id == "custom-league"
            || c.kind == domain::league::CompetitionType::InternationalNation),
        "no auto-generated club competitions when definitions are supplied"
    );
}

#[test]
fn bootstrap_team_selection_preserves_existing_snapshot_state() {
    let manager = domain::manager::Manager::new(
        "mgr-user".to_string(),
        "Alex".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    let startup_options = StartupOptions {
        start_year: 2032,
        start_phase: StartPhase::MidSeason,
        history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
    };
    let world = make_historical_snapshot_world();
    let clock = game_clock_for_world(&startup_options, &world.metadata).unwrap();
    let (mut game, stats_state) =
        build_game_from_world_data(clock, manager, &startup_options, world);

    let updated_stats =
        bootstrap_team_selection(&mut game, "team1", StartPhase::MidSeason, stats_state).unwrap();

    assert_eq!(game.league.as_ref().map(|league| league.season), Some(2031));
    assert_eq!(updated_stats.team_matches.len(), 1);
    assert_eq!(updated_stats.player_matches.len(), 1);
    assert_eq!(
        game.teams
            .iter()
            .find(|team| team.id == "team1")
            .and_then(|team| team.manager_id.as_deref()),
        Some("mgr-user")
    );
    assert!(game
        .news
        .iter()
        .any(|article| article.category == NewsCategory::ManagerialChange));
}

#[test]
fn bootstrap_team_selection_midseason_populates_half_season_state() {
    let mut game = make_bootstrap_test_game();

    let stats_state = bootstrap_team_selection(
        &mut game,
        "team1",
        StartPhase::MidSeason,
        domain::stats::StatsState::default(),
    )
    .unwrap();

    let league = game.league.as_ref().unwrap();
    let completed = league
        .fixtures
        .iter()
        .filter(|fixture| {
            fixture.counts_for_league_standings()
                && fixture.status == domain::league::FixtureStatus::Completed
                && (fixture.home_team_id == "team1" || fixture.away_team_id == "team1")
        })
        .count();
    let scheduled = league
        .fixtures
        .iter()
        .filter(|fixture| {
            fixture.counts_for_league_standings()
                && (fixture.home_team_id == "team1" || fixture.away_team_id == "team1")
        })
        .count();
    let team_standing = league
        .standings
        .iter()
        .find(|entry| entry.team_id == "team1")
        .unwrap();

    assert_eq!(completed, scheduled / 2);
    assert!(!stats_state.team_matches.is_empty());
    assert!(!stats_state.player_matches.is_empty());
    assert_eq!(team_standing.played as usize, completed);
    assert!(game
        .news
        .iter()
        .any(|article| article.category == domain::news::NewsCategory::ManagerialChange));
    assert!(game.news.iter().any(|article| {
        matches!(
            article.category,
            domain::news::NewsCategory::MatchReport
                | domain::news::NewsCategory::LeagueRoundup
                | domain::news::NewsCategory::StandingsUpdate
        )
    }));
}

/// Regression test for issue #225: verifies that bootstrap_team_selection followed by
/// upgrade_game_player_identities converts generic bucket positions
/// (Defender/Midfielder/Forward) to granular positions (LeftBack/CentralMidfielder/etc.).
/// select_team calls both in sequence; it cannot be called directly here because it
/// requires Tauri App state, so this test exercises the same in-memory operations.
#[test]
fn bootstrap_and_upgrade_sets_granular_positions() {
    let startup_options = StartupOptions {
        start_year: 2032,
        start_phase: StartPhase::SeasonStart,
        history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
    };
    let mut world = make_imported_baseline_world_without_staff();
    ofm_core::generator::normalize_imported_world_for_career_start(
        &mut world,
        startup_options.start_year as u32,
    );
    let clock = game_clock_for_world(&startup_options, &world.metadata).unwrap();
    let manager = domain::manager::Manager::new(
        "mgr-user".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    let (mut game, stats_state) =
        build_game_from_world_data(clock, manager, &startup_options, world);

    // All generated players start with generic (legacy-bucket) positions
    let outfield_before: Vec<_> = game
        .players
        .iter()
        .filter(|p| p.position != domain::player::Position::Goalkeeper)
        .collect();
    assert!(
        outfield_before
            .iter()
            .all(|p| p.natural_position.is_legacy_bucket()),
        "generated players should all start with generic (legacy-bucket) natural_position"
    );

    bootstrap_team_selection(&mut game, "team1", StartPhase::SeasonStart, stats_state).unwrap();
    ofm_core::player_identity::upgrade_game_player_identities(&mut game);

    // After upgrade, outfield players on team1 should have granular natural_position
    let outfield_after: Vec<_> = game
        .players
        .iter()
        .filter(|p| {
            p.team_id.as_deref() == Some("team1")
                && p.position != domain::player::Position::Goalkeeper
        })
        .collect();
    assert!(
        !outfield_after.is_empty(),
        "team1 should have outfield players"
    );
    assert!(
        outfield_after
            .iter()
            .all(|p| !p.natural_position.is_legacy_bucket()),
        "outfield players on the selected team should have granular natural_position after upgrade"
    );
}
