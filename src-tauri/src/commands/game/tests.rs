//! End-to-end tests for the game commands.
//!
//! Every test here drives a whole career opening: load or build a world, found
//! its competitions, bootstrap a manager into it, and assert on the result.
//! They live together rather than in the submodule they happen to touch first
//! because each one crosses three or more of them, and splitting them by first
//! contact would make the seams look tighter than they are.

use super::testkit::*;
use super::{
    bootstrap_team_selection, build_game_from_world_data, ensure_multi_competition_foundations,
    game_clock_for_world, resolve_simulation_scope, start_date_for_year, StartPhase,
    StartupOptions, DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
};
use chrono::{TimeZone, Utc};
use domain::{
    league::{CompetitionFormat, CompetitionScope, CompetitionType, League},
    news::NewsCategory,
};
use ofm_core::{clock::GameClock, game::Game};
use std::collections::{BTreeMap, BTreeSet};

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

/// The regression test for #555, at the shape the game actually ships.
///
/// Every earlier promotion test built its divisions by hand, and that is
/// exactly how the bug survived: hand-built divisions carry no berths, so
/// none of them ever entered the code path that a real world takes. This
/// one runs `ensure_multi_competition_foundations` — the same competition
/// plan `create_game` builds — through three consecutive rollovers and
/// checks the invariant that matters: every club is registered with exactly
/// one of its country's league tables, the divisions keep their sizes, and
/// each top flight actually turns over.
///
/// The nations are chosen to cover the shapes that broke: two-tier pyramids
/// (ENG, ES, BR), a split-season country whose two halves share one roster
/// (AR), a single-tier country (PT), Brazil's regional state cups, and a
/// continental cup every first division berths into — the shared target
/// that detached every ladder in the world.
fn production_world(start_year: i32, user_team: &str) -> Game {
    let mut teams = Vec::new();
    for (nation, count) in [("ENG", 40), ("ES", 40), ("BR", 40), ("AR", 20), ("PT", 20)] {
        for index in 0..count {
            teams.push(nation_team(
                &format!("{}-{index:02}", nation.to_lowercase()),
                nation,
                1000 - index as u32,
            ));
        }
    }
    let clock = GameClock::new(start_date_for_year(start_year).expect("valid start year"));
    let mut game = Game::new(clock, manager_for(user_team), teams, vec![], vec![], vec![]);
    ensure_multi_competition_foundations(&mut game);
    game.sync_legacy_league();
    game
}

/// (nation, club id) for every club in the world.
fn team_list(game: &Game) -> Vec<(String, String)> {
    game.teams
        .iter()
        .map(|team| (team.football_nation.clone(), team.id.clone()))
        .collect()
}

fn roster(competition: &League) -> BTreeSet<String> {
    competition.participant_ids.iter().cloned().collect()
}

/// Every club in exactly one of its country's league tables, every division
/// still its authored size, and a split-season country's two halves still
/// running over the same clubs.
fn assert_one_league_per_club(game: &Game, sizes: &BTreeMap<String, usize>, label: &str) {
    let mut clubs_by_nation: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for team in &team_list(game) {
        clubs_by_nation
            .entry(team.0.clone())
            .or_default()
            .insert(team.1.clone());
    }
    let mut by_country: BTreeMap<String, Vec<&League>> = BTreeMap::new();
    for competition in &game.competitions {
        if competition.rules.format != CompetitionFormat::LeagueTable
            || competition.kind != CompetitionType::League
            || competition.scope != CompetitionScope::Domestic
        {
            continue;
        }
        if let Some(country) = &competition.country_id {
            by_country
                .entry(country.clone())
                .or_default()
                .push(competition);
        }
    }
    // Iterating only the countries that still have competitions means a
    // country whose leagues vanished is never checked — the test passed
    // with both Argentine leagues deleted after every rollover. Compare the
    // key sets first, so a disappearing country is itself a failure.
    let represented: BTreeSet<&String> = by_country.keys().collect();
    let expected: BTreeSet<&String> = clubs_by_nation.keys().collect();
    assert_eq!(
        represented, expected,
        "{label}: every nation must still have at least one league table"
    );

    for (country, leagues) in by_country {
        let mut union: BTreeSet<String> = BTreeSet::new();
        let split_season = ofm_core::nations::is_split_season_country(&country);
        for league in &leagues {
            let clubs = roster(league);
            assert_eq!(
                clubs.len(),
                league.participant_ids.len(),
                "{label}: {} lists a club twice: {:?}",
                league.id,
                league.participant_ids
            );
            assert_eq!(
                league.participant_ids.len(),
                sizes[&league.id],
                "{label}: {} changed size",
                league.id
            );
            if !split_season {
                assert!(
                    union.is_disjoint(&clubs),
                    "{label}: {} shares clubs with another {country} league",
                    league.id
                );
            }
            union.extend(clubs);
        }
        if split_season {
            // Both halves are the same division played twice, so they must
            // hold identical rosters rather than disjoint ones.
            let first = roster(leagues[0]);
            for league in &leagues[1..] {
                assert_eq!(
                    roster(league),
                    first,
                    "{label}: {country} halves drifted apart at {}",
                    league.id
                );
            }
        }
        assert_eq!(
            union, clubs_by_nation[&country],
            "{label}: every {country} club must be in exactly one league table"
        );
    }
}

#[test]
fn a_generated_world_promotes_and_relegates_for_three_seasons_running() {
    let mut game = production_world(2035, "eng-00");
    let (regions, competitions) =
        resolve_simulation_scope(&game, "eng-00", None, None).expect("a valid scope");
    game.active_region_ids = regions;
    game.active_competition_ids = competitions;
    let sizes: BTreeMap<String, usize> = game
        .competitions
        .iter()
        .map(|competition| (competition.id.clone(), competition.participant_ids.len()))
        .collect();
    let mut previous: BTreeMap<String, BTreeSet<String>> = game
        .competitions
        .iter()
        .filter(|competition| ["eng-d1", "es-d1"].contains(&competition.id.as_str()))
        .map(|competition| (competition.id.clone(), roster(competition)))
        .collect();

    for rollover in 1..=3 {
        let far_future = Utc.with_ymd_and_hms(2100, 1, 1, 0, 0, 0).unwrap();
        let players = game.players.clone();
        for competition in game.competitions.iter_mut() {
            ofm_core::catchup::simulate_past_fixtures(competition, &players, far_future);
        }
        let last_match_day = game
            .competitions
            .iter()
            .find(|competition| {
                competition.rules.format == CompetitionFormat::LeagueTable
                    && competition.participant_ids.iter().any(|id| id == "eng-00")
            })
            .expect("the user's division")
            .fixtures
            .iter()
            .filter(|fixture| fixture.counts_for_league_standings())
            .filter_map(|fixture| chrono::NaiveDate::parse_from_str(&fixture.date, "%Y-%m-%d").ok())
            .max()
            .expect("a played season");
        game.clock.current_date = Utc.from_utc_datetime(
            &last_match_day
                .and_hms_opt(0, 0, 0)
                .expect("midnight is a valid time"),
        ) + chrono::Duration::days(1);

        ofm_core::end_of_season::process_end_of_season(&mut game);

        let label = format!("rollover {rollover}");
        assert_one_league_per_club(&game, &sizes, &label);

        // Every domestic division that existed at kickoff must still exist.
        // Size and disjointness assertions say nothing about a league that
        // is simply gone.
        for id in sizes.keys() {
            assert!(
                game.competitions.iter().any(|c| c.id == *id),
                "{label}: competition {id} disappeared"
            );
        }

        // The user's own division has to stay in simulation scope, or the
        // day loop cannot see their fixtures and runs their match against
        // whichever competition sorts first.
        let user_division = game
            .competitions
            .iter()
            .find(|c| {
                c.rules.format == CompetitionFormat::LeagueTable
                    && c.participant_ids.iter().any(|id| id == "eng-00")
            })
            .expect("the user is registered somewhere");
        assert!(
            game.active_competition_ids.contains(&user_division.id),
            "{label}: the user's division {} is out of scope",
            user_division.id
        );

        // The point of #555: a top flight that never changes hands is the
        // bug, not a stable league.
        for id in ["eng-d1", "es-d1"] {
            let now = roster(
                game.competitions
                    .iter()
                    .find(|competition| competition.id == id)
                    .expect(id),
            );
            assert_ne!(
                previous.get(id),
                Some(&now),
                "{label}: {id} promoted and relegated nobody"
            );
            previous.insert(id.to_string(), now);
        }
    }
}

/// Brazil is the one shipped nation whose divisions run on different
/// calendars — Série A opens 28 January, Série B on 21 March — so at the
/// first division's last matchday the second still has eight rounds to
/// play. The rollover used to fire anyway, the ladder skipped the whole
/// country for want of a finished lower tier, and Série B was never
/// regenerated: promotion happened every *other* season, and Série B
/// skipped a calendar year each time it did.
///
/// A Brazilian career is the whole point of this test. An English one
/// cannot see the bug, because both English divisions finish together.
#[test]
fn a_brazilian_career_promotes_and_relegates_every_season() {
    let mut game = production_world(2035, "br-00");
    let (regions, competitions) =
        resolve_simulation_scope(&game, "br-00", None, None).expect("a valid scope");
    game.active_region_ids = regions;
    game.active_competition_ids = competitions;

    for rollover in 1..=3 {
        let user_last = game
            .competitions
            .iter()
            .find(|competition| {
                competition.rules.format == CompetitionFormat::LeagueTable
                    && competition.participant_ids.iter().any(|id| id == "br-00")
            })
            .expect("the user's division")
            .fixtures
            .iter()
            .filter(|fixture| fixture.counts_for_league_standings())
            .filter_map(|fixture| chrono::NaiveDate::parse_from_str(&fixture.date, "%Y-%m-%d").ok())
            .max()
            .expect("a scheduled season");

        // Advance in steps the way a player does, until the game itself
        // says the season is over. If the gate let the rollover through
        // while Série B was mid-season, this would stop at the first step.
        let mut cutoff = Utc.from_utc_datetime(
            &user_last
                .and_hms_opt(0, 0, 0)
                .expect("midnight is a valid time"),
        ) + chrono::Duration::days(1);
        for _ in 0..40 {
            let players = game.players.clone();
            for competition in game.competitions.iter_mut() {
                ofm_core::catchup::simulate_past_fixtures(competition, &players, cutoff);
            }
            game.clock.current_date = cutoff;
            if ofm_core::end_of_season::is_season_complete(&game) {
                break;
            }
            cutoff += chrono::Duration::days(14);
        }
        assert!(
            ofm_core::end_of_season::is_season_complete(&game),
            "rollover {rollover}: the Brazilian season never completed"
        );

        let roster_before = by_competition_id(&game, "br-d1").participant_ids.clone();
        let second_tier_season_before = by_competition_id(&game, "br-d2").season;

        ofm_core::end_of_season::process_end_of_season(&mut game);

        assert_ne!(
            by_competition_id(&game, "br-d1").participant_ids,
            roster_before,
            "rollover {rollover}: Série A promoted and relegated nobody"
        );
        assert_eq!(
            by_competition_id(&game, "br-d2").season,
            second_tier_season_before + 1,
            "rollover {rollover}: Série B must advance exactly one season, not skip a year"
        );
    }
}

fn by_competition_id<'a>(game: &'a Game, id: &str) -> &'a League {
    game.competitions
        .iter()
        .find(|competition| competition.id == id)
        .unwrap_or_else(|| panic!("{id} should exist"))
}
