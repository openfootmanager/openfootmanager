//! End-to-end tests for the game commands.
//!
//! Every test here drives a whole career opening: load or build a world, found
//! its competitions, bootstrap a manager into it, and assert on the result.
//! They live together rather than in the submodule they happen to touch first
//! because each one crosses three or more of them, and splitting them by first
//! contact would make the seams look tighter than they are.

use super::testkit::*;
use super::{
    build_game_from_world_data, game_clock_for_world, StartupOptions,
    DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
};
use domain::news::NewsCategory;
use ofm_core::career::{begin_career, CareerScope, StartPhase};

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

    begin_career(&mut game, "team1", CareerScope::default(), stats_state).unwrap();

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

/// Given a world that was generated from a seed,
/// When a new game is built from it,
/// Then the game has that seed — one number says both how the world was made and
///      how its days will go.
#[test]
fn a_new_game_keeps_the_seed_its_world_was_made_from() {
    let mut world = make_imported_baseline_world_without_staff();
    world.generation_seed = Some(0xA11C_E5ED_0000_0007);

    let game = game_from(world);

    assert_eq!(game.seed, 0xA11C_E5ED_0000_0007);
}

/// Given a world that was not generated here — an import, a package — so nothing
///       gave it a seed,
/// When a new game is built from it,
/// Then the game draws one of its own, rather than starting on 0 like every other.
#[test]
fn a_world_without_a_seed_gives_its_game_one_of_its_own() {
    let world = make_imported_baseline_world_without_staff();
    assert_eq!(world.generation_seed, None);

    let game = game_from(world);

    assert_ne!(game.seed, 0);
}

fn game_from(world: ofm_core::generator::WorldData) -> ofm_core::game::Game {
    let startup_options = StartupOptions {
        start_year: 2032,
        start_phase: StartPhase::SeasonStart,
        history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
    };
    let clock = game_clock_for_world(&startup_options, &world.metadata).unwrap();
    let manager = domain::manager::Manager::new(
        "mgr-user".to_string(),
        "Alex".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    build_game_from_world_data(clock, manager, &startup_options, world).0
}

#[test]
fn a_new_career_starts_with_own_choices_clear_and_rivals_identity_intact() {
    use domain::team::{PlayStyle, PlayerRole, TacticsPhaseSettings};

    let mut game = make_bootstrap_test_game();
    let identity = ofm_core::ai_tactics::blueprint_for(&PlayStyle::HighPress);
    assert_ne!(identity, TacticsPhaseSettings::default());
    for team in &mut game.teams {
        team.play_style = PlayStyle::HighPress;
        team.tactics_phase = identity.clone();
        team.player_roles
            .insert(format!("{}-player-0", team.id), PlayerRole::BallWinner);
    }

    begin_career(
        &mut game,
        "team1",
        CareerScope::default(),
        domain::stats::StatsState::default(),
    )
    .unwrap();

    let own = game.teams.iter().find(|team| team.id == "team1").unwrap();
    let rival = game.teams.iter().find(|team| team.id == "team2").unwrap();
    assert_eq!(own.tactics_phase, TacticsPhaseSettings::default());
    assert!(own.player_roles.is_empty());
    assert_eq!(rival.tactics_phase, identity);
    assert_eq!(rival.player_roles.len(), 1);
}

#[test]
fn beginning_a_career_seeds_the_ai_loan_market() {
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
        player.stage_contract_end(Some("2035-06-30".to_string()));
        game.players.push(player);
    }

    begin_career(
        &mut game,
        "team1",
        CareerScope::default(),
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
            calendar: None,
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
fn beginning_a_career_preserves_an_imported_snapshot() {
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
        begin_career(&mut game, "team1", CareerScope::default(), stats_state).unwrap();

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
fn authored_group_size_reaches_the_game_built_from_a_loaded_world() {
    for (size, expected_groups, expected_fixtures) in [(Some(2), 4, 8), (None, 2, 24)] {
        let template = make_historical_snapshot_world().teams[0].clone();
        let world = ofm_core::generator::WorldData {
            teams: (0..8)
                .map(|i| {
                    let mut team = template.clone();
                    team.id = format!("authored-club-{i}");
                    team
                })
                .collect(),
            ..Default::default()
        };
        let mut json = serde_json::to_value(&world).unwrap();
        let mut format = serde_json::json!({"kind":"GroupAndKnockout"});
        if let Some(size) = size {
            format["groupSize"] = size.into();
        }
        json["competitionDefinitions"] = serde_json::json!({"competitions":[{
            "id":"authored-cup", "name":"Authored Cup", "type":"Cup", "scope":"Domestic",
            "format":format, "participants":{"explicit":world.teams.iter().map(|t| &t.id).collect::<Vec<_>>()}
        }]});
        let loaded = ofm_core::generator::load_world_from_json(&json.to_string()).unwrap();
        let manager = domain::manager::Manager::new(
            "mgr-user".into(),
            "Alex".into(),
            "Manager".into(),
            "1980-01-01".into(),
            "England".into(),
        );
        let options = StartupOptions {
            start_year: 2031,
            start_phase: StartPhase::SeasonStart,
            history_depth_years: 0,
        };
        let clock = game_clock_for_world(&options, &loaded.metadata).unwrap();
        let (game, _) = build_game_from_world_data(clock, manager, &options, loaded);
        let cup = game
            .competitions
            .iter()
            .find(|c| c.id == "authored-cup")
            .unwrap();
        assert_eq!(cup.groups.len(), expected_groups);
        assert_eq!(cup.fixtures.len(), expected_fixtures);
        assert!(cup
            .groups
            .iter()
            .all(|g| g.team_ids.len() == size.unwrap_or(4)));
    }
}

/// Given an active career whose manager has no club and no history
/// When the career stage command reads it
/// Then it answers awaitingClub, from the active game and nothing else.
#[test]
fn get_career_stage_reads_the_active_game() {
    let state = ofm_core::state::StateManager::new();
    let mut game = make_bootstrap_test_game();
    game.manager.team_id = None;
    state.set_game(game);

    assert_eq!(
        super::get_career_stage_internal(&state),
        Ok(ofm_core::career_stage::CareerStage::AwaitingClub)
    );
}

/// Given no active game session
/// When the career stage command is called
/// Then it fails with the no-session translation key.
#[test]
fn get_career_stage_without_a_session_fails_with_the_session_key() {
    let state = ofm_core::state::StateManager::new();

    assert_eq!(
        super::get_career_stage_internal(&state),
        Err("be.error.noActiveGameSession".to_string())

/// A package whose cup fields depend on how its clubs rank by reputation: six clubs rated anywhere
/// in 0..=1000, a cup of two named clubs, a cup of the top two by reputation, and a group cup of
/// everyone left. The same package as the one `ofm_core`'s package tests use.
fn package_dir_whose_cups_depend_on_reputation() -> std::path::PathBuf {
    let teams: Vec<_> = (b'a'..=b'f')
        .map(|id| {
            serde_json::json!({
                "id": char::from(id).to_string(), "name": "Club", "city": "City",
                "country": "ENG", "reputationRange": [0, 1000],
                "colors": {"primary": "#000000", "secondary": "#ffffff"}
            })
        })
        .collect();
    let competitions = serde_json::json!([
        {"id":"a-cup", "name":"A", "type":"Cup", "scope":"Domestic",
         "format":{"kind":"Knockout"}, "participants":{"explicit":["a", "b"]}},
        {"id":"b-cup", "name":"B", "type":"Cup", "scope":"Domestic",
         "format":{"kind":"Knockout"}, "participants":{"selector":{
             "kind":"topByReputation", "country":"ENG", "count":2}}},
        {"id":"c-cup", "name":"C", "type":"Cup", "scope":"Domestic",
         "format":{"kind":"GroupAndKnockout", "groupSize":2, "qualifiersPerGroup":2},
         "participants":{"selector":{"kind":"allInCountry", "country":"ENG",
             "excludeCompetitions":["a-cup", "b-cup"]}}}
    ]);
    let dir = temp_pkg_dir("reputation-cups");
    std::fs::write(
        dir.join("package.json"),
        r#"{"schema":"world","id":"reputation-cups","name":"Reputation cups","formatVersion":1,"version":"1.0.0","license":"CC0-1.0"}"#,
    )
    .unwrap();
    std::fs::create_dir_all(dir.join("teams")).unwrap();
    std::fs::write(
        dir.join("teams/teams.json"),
        serde_json::json!({"schema":"team", "items": teams}).to_string(),
    )
    .unwrap();
    std::fs::create_dir_all(dir.join("competitions")).unwrap();
    std::fs::write(
        dir.join("competitions/competitions.json"),
        serde_json::json!({"schema":"competition", "items": competitions}).to_string(),
    )
    .unwrap();
    dir
}

/// The cups of a career started from `package`, as (id, sorted participants), the way
/// `start_new_game` builds it: ranked by the package's authored reputations.
fn cup_fields_of_a_career_from(
    loaded: (ofm_core::generator::WorldData, super::AuthoredReputations),
) -> (ofm_core::game::Game, Vec<(String, Vec<String>)>) {
    let (world, authored) = loaded;
    let options = StartupOptions {
        start_year: 2031,
        start_phase: StartPhase::SeasonStart,
        history_depth_years: 0,
    };
    let clock = game_clock_for_world(&options, &world.metadata).unwrap();
    let manager = domain::manager::Manager::new(
        "mgr-user".into(),
        "Alex".into(),
        "Manager".into(),
        "1980-01-01".into(),
        "England".into(),
    );
    let (game, _) = super::build_game_from_world_data_ranked(
        clock,
        manager,
        &options,
        world,
        ofm_core::generator::ReputationBasis::Authored(&authored),
    );
    let mut cups: Vec<(String, Vec<String>)> = game
        .competitions
        .iter()
        .filter(|c| c.id.ends_with("-cup"))
        .map(|c| {
            let mut ids = c.participant_ids.clone();
            ids.sort();
            (c.id.clone(), ids)
        })
        .collect();
    cups.sort();
    (game, cups)
}

/// Given a package whose cups select and exclude clubs by reputation (#707)
/// When careers are started from it with different rolls, and one is saved and reloaded
/// Then every career has the same cups, the reload keeps them, and the rolled reputations still differ.
#[test]
fn a_package_career_keeps_the_cups_it_was_validated_with_on_every_roll_and_after_a_reload() {
    let dir = package_dir_whose_cups_depend_on_reputation();
    let (package, errors) = ofm_core::generator::load_world_package(&dir);
    assert!(errors.is_empty(), "{errors:?}");
    assert!(ofm_core::generator::validate_package(&package).is_empty());
    let sources = ofm_core::generator::DefinitionSources::embedded_only();

    // The same package reaches a career by two routes (an installed `.ofm` by id, and a
    // directory as `world_source`); both must hand the career the reputations it was validated with.
    let packages = tempfile::tempdir().unwrap();
    ofm_core::generator::export_directory_to_ofm(
        &dir,
        &packages.path().join("reputation-cups.ofm"),
    )
    .unwrap();
    let by_id = || {
        let (world, _, authored) = super::load_world_data_from_package_ids(
            packages.path(),
            &["reputation-cups".to_string()],
            Some(2031),
            None,
            &sources,
        )
        .unwrap();
        (world, authored)
    };
    let by_directory =
        || super::load_world_data(Some(dir.to_string_lossy().as_ref()), &sources).unwrap();

    for (route, load) in [
        ("package id", &by_id as &dyn Fn() -> _),
        ("directory", &by_directory as &dyn Fn() -> _),
    ] {
        assert_cups_stable_across_rolls_and_reload(route, load);
    }
    std::fs::remove_dir_all(&dir).ok();
}

fn assert_cups_stable_across_rolls_and_reload(
    route: &str,
    load: &dyn Fn() -> (ofm_core::generator::WorldData, super::AuthoredReputations),
) {
    let careers: Vec<_> = (0..40)
        .map(|_| cup_fields_of_a_career_from(load()))
        .collect();

    let distinct_fields: std::collections::BTreeSet<_> =
        careers.iter().map(|(_, cups)| cups.clone()).collect();
    assert_eq!(distinct_fields.len(), 1, "{route}: {distinct_fields:?}");
    let distinct_reputations: std::collections::BTreeSet<Vec<u32>> = careers
        .iter()
        .map(|(game, _)| game.teams.iter().map(|t| t.reputation).collect())
        .collect();
    assert!(
        distinct_reputations.len() > 1,
        "{route}: every roll gave the same reputations"
    );

    let (game, cups) = &careers[0];
    let dir = tempfile::tempdir().unwrap();
    let mut saves = db::save_manager::SaveManager::init(dir.path()).unwrap();
    let save_id = saves.create_save(game, "Package career").unwrap();
    let reloaded = saves.load_game(&save_id).unwrap();
    let mut reloaded_cups: Vec<(String, Vec<String>)> = reloaded
        .competitions
        .iter()
        .filter(|c| c.id.ends_with("-cup"))
        .map(|c| {
            let mut ids = c.participant_ids.clone();
            ids.sort();
            (c.id.clone(), ids)
        })
        .collect();
    reloaded_cups.sort();
    assert_eq!(
        &reloaded_cups, cups,
        "{route}: a reload must not change who is in the cups"
    );
}
