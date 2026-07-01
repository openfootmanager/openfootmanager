
use super::regions::{
    brazil_state_region, competition_required_region_ids, default_season_month_for_region,
    division_name, division_tier_name, division_tier_name_key, split_into_divisions,
};
use super::{
    age_on_date, apply_generated_past_history, bootstrap_team_selection,
    build_foundation_competitions, build_game_from_world_data, create_new_save,
    current_date_for_phase, ensure_international_windows, game_clock_for_world,
    load_world_data_from_path, map_save_manager_lock_error, normalize_startup_options,
    package_folder_name, parse_competition_definitions, preseason_league_year,
    preseason_season_start, rebuild_competitions_for_management_date, require_active_stats_state,
    resolve_simulation_scope, select_continental_entrants, start_date_for_year, RawStartupOptions,
    StartPhase, StartupOptions, DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
    MAX_GENERATED_HISTORY_DEPTH_YEARS,
};
use chrono::{TimeZone, Utc};
use db::save_manager::SaveManager;
use domain::{
    league::{CompetitionFormat, CompetitionScope, CompetitionType, FixtureCompetition, League},
    manager::Manager,
    news::{NewsArticle, NewsCategory},
    stats::{PlayerMatchStatsRecord, TeamMatchStatsRecord},
    world_history::{HistoricalSeasonAwardsRecord, WorldHistoryArchive},
};
use ofm_core::{
    clock::GameClock,
    game::Game,
    generator::{WorldData, WorldDataKind, WorldDataMetadata},
    season_context::refresh_game_context,
    state::StateManager,
};
use std::sync::Mutex;

fn manager_for(team_id: &str) -> Manager {
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

#[test]
fn world_cup_summer_career_stages_and_surfaces_the_tournament() {
    use ofm_core::world_cup::is_world_cup_competition;
    // A career opening in the 2026 World Cup summer.
    let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 7, 1, 12, 0, 0).unwrap());
    let mut game = Game::new(
        clock,
        manager_for("team-1"),
        vec![nation_team("team-1", "ES", 500)],
        vec![],
        vec![],
        vec![],
    );
    // A non-empty active scope so staging registers the tournament as active.
    game.active_competition_ids = vec!["dummy".to_string()];

    ensure_international_windows(&mut game);

    let world_cup = game
        .competitions
        .iter()
        .find(|competition| is_world_cup_competition(competition))
        .expect("a World Cup summer career stages the tournament");
    assert!(
        game.active_competition_ids.contains(&world_cup.id),
        "the World Cup is surfaced in the active scope"
    );
    assert!(
        game.news
            .iter()
            .any(|article| article.id.starts_with("world_cup_kickoff_")),
        "a kickoff news article is published"
    );
}

#[test]
fn rebuilding_competitions_leaves_the_world_cup_schedule_intact() {
    use ofm_core::world_cup::is_world_cup_competition;
    // A 2026 World Cup summer career, staged at the June anchor.
    let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 7, 1, 12, 0, 0).unwrap());
    let mut game = Game::new(
        clock,
        manager_for("team-1"),
        vec![nation_team("team-1", "ES", 500)],
        vec![],
        vec![],
        vec![],
    );
    game.active_competition_ids = vec!["dummy".to_string()];
    ensure_international_windows(&mut game);

    // Capture the World Cup's id and fixture dates before any re-anchoring.
    let (wc_id, before): (String, Vec<String>) = {
        let world_cup = game
            .competitions
            .iter()
            .find(|competition| is_world_cup_competition(competition))
            .expect("the World Cup is staged");
        (
            world_cup.id.clone(),
            world_cup
                .fixtures
                .iter()
                .map(|fixture| fixture.date.clone())
                .collect(),
        )
    };
    assert!(
        !before.is_empty(),
        "the staged World Cup has fixtures to protect"
    );

    // Re-anchor competitions to a February management date — the Argentina
    // mid-season scenario that previously orphaned the cup's June schedule.
    let management_date = Utc.with_ymd_and_hms(2026, 2, 1, 12, 0, 0).unwrap();
    rebuild_competitions_for_management_date(&mut game, management_date);

    let world_cup = game
        .competitions
        .iter()
        .find(|competition| competition.id == wc_id)
        .expect("the World Cup survives the re-anchor");
    let after: Vec<String> = world_cup
        .fixtures
        .iter()
        .map(|fixture| fixture.date.clone())
        .collect();
    assert_eq!(
        before, after,
        "the World Cup keeps its June schedule through a February re-anchor"
    );
    assert!(
        after
            .iter()
            .all(|date| date.starts_with("2026-06") || date.starts_with("2026-07")),
        "World Cup fixtures stay in the cup window, not pulled back to February"
    );
}

#[test]
fn non_world_cup_year_career_stages_no_tournament() {
    use ofm_core::world_cup::is_world_cup_competition;
    let clock = GameClock::new(Utc.with_ymd_and_hms(2027, 7, 1, 12, 0, 0).unwrap());
    let mut game = Game::new(
        clock,
        manager_for("team-1"),
        vec![nation_team("team-1", "ES", 500)],
        vec![],
        vec![],
        vec![],
    );

    ensure_international_windows(&mut game);

    assert!(
        !game.competitions.iter().any(is_world_cup_competition),
        "no World Cup is staged outside a cup summer"
    );
}

#[test]
fn package_folder_name_falls_back_to_the_directory_name() {
    assert_eq!(package_folder_name("/mods/My World"), "My World");
    assert_eq!(package_folder_name("turkish-league"), "turkish-league");
    // No usable component → a sensible default rather than an empty name.
    assert_eq!(package_folder_name(""), "World Package");
}

fn nation_team(id: &str, nation: &str, reputation: u32) -> domain::team::Team {
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

/// Characterization test: locks the STRUCTURE of the generated foundation
/// world (kinds, scopes, regions, countries, priorities, participant and
/// fixture counts, formats) so the Phase E "unify built-ins through the
/// resolver" refactor can prove it preserves behavior (modulo ids).
#[test]
fn foundation_competitions_structure_is_stable() {
    // A 30-club nation (→ two divisions: 20 + 10), a 6-club nation (one
    // division), and a 1-club nation (skipped). All in one region, so the
    // continental field stays under four entrants and no continental cup
    // is created — keeping the structure fully deterministic.
    let mut teams = Vec::new();
    for index in 0..30 {
        teams.push(nation_team(
            &format!("esp-{index:02}"),
            "ESP",
            1000 - index as u32,
        ));
    }
    for index in 0..6 {
        teams.push(nation_team(
            &format!("fra-{index}"),
            "FRA",
            500 - index as u32,
        ));
    }
    teams.push(nation_team("and-0", "AND", 100));

    let clock = GameClock::new(start_date_for_year(2032).unwrap());
    let manager = domain::manager::Manager::new(
        "mgr".to_string(),
        "A".to_string(),
        "B".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    let game = Game::new(clock, manager, teams, vec![], vec![], vec![]);

    let competitions = build_foundation_competitions(&game);

    type CompetitionSummary = (
        CompetitionType,
        CompetitionScope,
        Option<String>,
        Option<String>,
        usize,
        u32,
        CompetitionFormat,
    );

    let summary: Vec<CompetitionSummary> = competitions
        .iter()
        .map(|competition| {
            (
                competition.kind.clone(),
                competition.scope.clone(),
                competition.region_id.clone(),
                competition.country_id.clone(),
                competition.participant_ids.len(),
                competition.priority,
                competition.rules.format.clone(),
            )
        })
        .collect();

    let europe = || Some("europe".to_string());
    assert_eq!(
        summary,
        vec![
            (
                CompetitionType::League,
                CompetitionScope::Domestic,
                europe(),
                Some("ESP".to_string()),
                20,
                0,
                CompetitionFormat::LeagueTable
            ),
            (
                CompetitionType::League,
                CompetitionScope::Domestic,
                europe(),
                Some("ESP".to_string()),
                10,
                1,
                CompetitionFormat::LeagueTable
            ),
            (
                CompetitionType::Cup,
                CompetitionScope::Domestic,
                europe(),
                Some("ESP".to_string()),
                30,
                2,
                CompetitionFormat::Knockout
            ),
            (
                CompetitionType::League,
                CompetitionScope::Domestic,
                europe(),
                Some("FRA".to_string()),
                6,
                3,
                CompetitionFormat::LeagueTable
            ),
            (
                CompetitionType::Cup,
                CompetitionScope::Domestic,
                europe(),
                Some("FRA".to_string()),
                6,
                4,
                CompetitionFormat::Knockout
            ),
        ],
    );

    // League tables carry a full double round robin and a standings row per
    // club; the refactor must preserve both.
    let top_division = &competitions[0];
    assert_eq!(top_division.standings.len(), 20);
    assert_eq!(top_division.fixtures.len(), 20 * 19);
    assert_eq!(competitions[3].fixtures.len(), 6 * 5);

    // No continental cup for a single-region field.
    assert!(!competitions
        .iter()
        .any(|competition| competition.kind == CompetitionType::ContinentalClub));

    // Default continental berths: first division awards positions 1–4, the
    // cup awards its winner, the second division awards nothing.
    use domain::league::BerthRule;
    let top_division = &competitions[0];
    assert_eq!(top_division.berths.len(), 1);
    assert_eq!(top_division.berths[0].target, "continental-champions-cup");
    assert!(matches!(
        top_division.berths[0].rule,
        BerthRule::PositionRange { from: 1, to: 4 }
    ));
    assert!(
        competitions[1].berths.is_empty(),
        "second division awards no berth"
    );
    let cup = &competitions[2];
    assert!(matches!(
        cup.berths.first().map(|berth| &berth.rule),
        Some(BerthRule::CupWinner)
    ));
}

fn default_player_attributes() -> domain::player::PlayerAttributes {
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

fn make_bootstrap_test_game() -> Game {
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
                format!("199{}-01-01", index),
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

#[test]
fn select_continental_entrants_takes_top_clubs_per_region_by_reputation() {
    let make = |id: &str, nation: &str, reputation: u32| {
        let mut team = domain::team::Team::new(
            id.to_string(),
            id.to_string(),
            id.to_string(),
            "Country".to_string(),
            "City".to_string(),
            "Stadium".to_string(),
            10_000,
        );
        team.football_nation = nation.to_string();
        team.reputation = reputation;
        team
    };
    let teams = vec![
        make("eng-a", "GB", 900),
        make("eng-b", "GB", 800),
        make("eng-c", "GB", 700), // third in Europe -> excluded by per_region
        make("bra-a", "BR", 850),
        make("bra-b", "BR", 600),
    ];

    let entrants = select_continental_entrants(&teams, 2, 16);

    // Top two per region, ordered strongest-first across regions.
    assert_eq!(
        entrants,
        vec![
            "eng-a".to_string(),
            "bra-a".to_string(),
            "eng-b".to_string(),
            "bra-b".to_string(),
        ]
    );
}

#[test]
fn parse_competition_definitions_accepts_yaml_and_json() {
    let yaml = "\
formatVersion: 1
competitions:
  - id: tr-1
    name: Super Lig
    type: League
    scope: Domestic
    format:
      kind: LeagueTable
    participants:
      selector:
        kind: allInCountry
        country: TR
";
    let parsed = parse_competition_definitions(yaml).expect("YAML should parse");
    assert_eq!(parsed.competitions.len(), 1);
    assert_eq!(parsed.competitions[0].id, "tr-1");

    let json = r#"{"formatVersion":1,"competitions":[{"id":"tr-1","name":"Super Lig","type":"League","scope":"Domestic","format":{"kind":"LeagueTable"},"participants":{"selector":{"kind":"allInCountry","country":"TR"}}}]}"#;
    let parsed_json = parse_competition_definitions(json).expect("JSON should parse");
    assert_eq!(parsed_json.competitions[0].id, "tr-1");

    assert!(parse_competition_definitions("not: [valid").is_err());
}

fn temp_pkg_dir(tag: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("ofm-pkg-cmd-{tag}-{nanos}"));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
#[ignore = "perf harness; run: cargo test -p openfootmanager perf_baseline -- --ignored --nocapture"]
fn perf_baseline() {
    use std::time::Instant;

    let t = Instant::now();
    let world = ofm_core::generator::generate_world_data(None);
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
fn loads_a_world_from_a_package_directory() {
    let dir = temp_pkg_dir("load");
    std::fs::write(
        dir.join("confed.yaml"),
        "schema: confederation\nid: galaxy\nname: Galaxy\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("country.yaml"),
        "schema: country\nid: ZZ\nname: Zedland\nconfederation: galaxy\n",
    )
    .unwrap();
    std::fs::write(
            dir.join("teams.yaml"),
            "schema: team\nitems:\n  - { id: zed-fc, name: Zed FC, city: Zedtown, country: ZZ, colors: { primary: \"#000\", secondary: \"#fff\" } }\n  - { id: zed-utd, name: Zed United, city: Zedford, country: ZZ, colors: { primary: \"#111\", secondary: \"#fff\" } }\n",
        )
        .unwrap();

    let world =
        super::load_world_data(Some(dir.to_string_lossy().as_ref())).expect("package loads");
    assert!(world.teams.iter().any(|t| t.id == "zed-fc"));
    assert!(world.teams.iter().any(|t| t.id == "zed-utd"));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn validate_world_package_reports_problems_and_passes_clean_packages() {
    let team = "schema: team\nid: {id}\nname: {name}\ncity: X\ncountry: ES\ncolors: { primary: \"#000\", secondary: \"#fff\" }\n";

    let valid = temp_pkg_dir("valid");
    std::fs::write(
        valid.join("team.yaml"),
        team.replace("{id}", "zed-fc").replace("{name}", "Zed FC"),
    )
    .unwrap();
    let clean = super::validate_world_package(valid.to_string_lossy().to_string()).unwrap();
    assert!(clean.is_empty(), "a clean package should have no issues");

    let broken = temp_pkg_dir("broken");
    std::fs::write(
        broken.join("a.yaml"),
        team.replace("{id}", "dup").replace("{name}", "A"),
    )
    .unwrap();
    std::fs::write(
        broken.join("b.yaml"),
        team.replace("{id}", "dup").replace("{name}", "B"),
    )
    .unwrap();
    let issues = super::validate_world_package(broken.to_string_lossy().to_string()).unwrap();
    assert!(!issues.is_empty(), "a duplicate id should be reported");

    std::fs::remove_dir_all(&valid).ok();
    std::fs::remove_dir_all(&broken).ok();
}

#[test]
fn split_into_divisions_chunks_a_major_into_two_tiers() {
    let clubs: Vec<String> = (0..40).map(|i| format!("club-{i:02}")).collect();

    let divisions = split_into_divisions(&clubs, 20);

    assert_eq!(divisions.len(), 2);
    assert_eq!(divisions[0].len(), 20);
    assert_eq!(divisions[1].len(), 20);
    // Strongest tier first; the second tier starts where the first ends.
    assert_eq!(divisions[0][0], "club-00");
    assert_eq!(divisions[1][0], "club-20");
}

#[test]
fn split_into_divisions_keeps_a_single_league_at_division_size() {
    let clubs: Vec<String> = (0..20).map(|i| format!("club-{i:02}")).collect();

    let divisions = split_into_divisions(&clubs, 20);

    assert_eq!(divisions.len(), 1);
    assert_eq!(divisions[0].len(), 20);
}

#[test]
fn split_into_divisions_keeps_a_single_tier_for_small_countries() {
    let clubs: Vec<String> = (0..7).map(|i| format!("club-{i}")).collect();

    let divisions = split_into_divisions(&clubs, 20);

    assert_eq!(divisions.len(), 1);
    assert_eq!(divisions[0].len(), 7);
}

#[test]
fn split_into_divisions_folds_a_tiny_remainder_up() {
    // 25 clubs → 20 + 5; the 5-club tail folds up rather than forming a
    // tiny second division.
    let clubs: Vec<String> = (0..25).map(|i| format!("club-{i:02}")).collect();

    let divisions = split_into_divisions(&clubs, 20);

    assert_eq!(divisions.len(), 1);
    assert_eq!(divisions[0].len(), 25);
}

#[test]
fn season_start_anchor_buffers_preseason_for_non_august_calendars() {
    // An Asian (February) club: the Season-Start clock should land a
    // pre-season buffer before the first competitive fixture — not on
    // matchday one — so the player gets a pre-season with playable
    // friendlies. (Regression for Asian/Oceanian leagues, which used to
    // re-anchor straight onto their opener.)
    let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap());
    let manager = Manager::new(
        "mgr".to_string(),
        "Alex".to_string(),
        "Boss".to_string(),
        "1980-01-01".to_string(),
        "Japan".to_string(),
    );
    let mut team = domain::team::Team::new(
        "jp-1".to_string(),
        "Tokyo FC".to_string(),
        "TFC".to_string(),
        "JP".to_string(),
        "Tokyo".to_string(),
        "Stadium".to_string(),
        10_000,
    );
    team.football_nation = "JP".to_string();

    let mut game = Game::new(clock, manager, vec![team], vec![], vec![], vec![]);
    let mut league = League::new(
        "jp-league".to_string(),
        "JP League".to_string(),
        2026,
        &["jp-1".to_string()],
    );
    league.region_id = Some("asia".to_string());
    league.fixtures.push(domain::league::Fixture {
        id: "f1".to_string(),
        competition_id: "jp-league".to_string(),
        matchday: 1,
        date: "2026-02-07".to_string(),
        home_team_id: "jp-1".to_string(),
        away_team_id: "jp-2".to_string(),
        competition: FixtureCompetition::League,
        status: domain::league::FixtureStatus::Scheduled,
        result: None,
    });
    game.competitions = vec![league];

    let anchor = super::team_season_anchor(&game, "jp-1").expect("a non-August league re-anchors");
    // The first competitive fixture (Feb 7) minus the 30-day pre-season buffer.
    assert_eq!(anchor, Utc.with_ymd_and_hms(2026, 1, 8, 0, 0, 0).unwrap());
}

#[test]
fn brazil_foundations_use_the_2026_calendar_and_regional_state_series() {
    let cities = [
        "São Paulo",
        "Rio",
        "Belo Horizonte",
        "Porto Alegre",
        "Salvador",
        "Recife",
        "Curitiba",
        "Fortaleza",
        "Goiânia",
        "Santos",
        "Campinas",
        "Belém",
        "Manaus",
        "Vitória",
        "Natal",
        "Florianópolis",
        "Cuiabá",
        "Maceió",
        "Bragantino",
        "Juiz de Fora",
    ];
    let mut teams: Vec<_> = (0..40)
        .map(|index| {
            let mut team = nation_team(&format!("br-{index}"), "BR", 1000 - index);
            team.city = cities[index as usize % cities.len()].to_string();
            team
        })
        .collect();
    let clock = GameClock::new(Utc.with_ymd_and_hms(2025, 12, 15, 0, 0, 0).unwrap());
    let mut game = Game::new(
        clock,
        manager_for("br-0"),
        std::mem::take(&mut teams),
        vec![],
        vec![],
        vec![],
    );
    game.competitions = build_foundation_competitions(&game);
    ofm_core::schedule::append_south_american_preseason_friendlies(&mut game.competitions, &[]);

    let serie_a = game
        .competitions
        .iter()
        .find(|competition| competition.id == "br-d1")
        .unwrap();
    let serie_b = game
        .competitions
        .iter()
        .find(|competition| competition.id == "br-d2")
        .unwrap();
    assert_eq!(serie_a.season_start_day, 28);
    assert_eq!(serie_a.season_start_month, 1);
    assert_eq!(serie_b.season_start_day, 21);
    assert_eq!(serie_b.season_start_month, 3);
    assert!(serie_a
        .fixtures
        .iter()
        .any(|fixture| fixture.competition == FixtureCompetition::League
            && fixture.date == "2026-01-28"));
    assert!(serie_b
        .fixtures
        .iter()
        .any(|fixture| fixture.competition == FixtureCompetition::League
            && fixture.date == "2026-03-21"));
    let friendly_dates: Vec<&str> = serie_a
        .fixtures
        .iter()
        .filter(|fixture| fixture.competition == FixtureCompetition::Friendly)
        .map(|fixture| fixture.date.as_str())
        .collect();
    assert_eq!(
        friendly_dates
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>(),
        ["2025-12-21", "2025-12-28", "2026-01-04"]
            .into_iter()
            .collect()
    );

    let states: Vec<_> = game
        .competitions
        .iter()
        .filter(|competition| competition.id.starts_with("br-state-"))
        .collect();
    assert_eq!(states.len(), 4);
    assert!(states
        .iter()
        .all(|competition| !competition.rules.counts_in_season_flow
            && competition.rules.group_stage_legs == 1
            && competition.name_key.is_some()));
    for team in &game.teams {
        assert_eq!(
            states
                .iter()
                .filter(|competition| competition.participant_ids.contains(&team.id))
                .count(),
            1
        );
    }
}

#[test]
fn management_date_rebuild_preserves_authored_competition_identity() {
    let teams = vec![
        nation_team("br-a", "BR", 500),
        nation_team("br-b", "BR", 400),
    ];
    let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap());
    let mut game = Game::new(clock, manager_for("br-a"), teams, vec![], vec![], vec![]);
    let mut authored = ofm_core::schedule::generate_league(
        "Authored Brazil Championship",
        2026,
        &["br-a".to_string(), "br-b".to_string()],
        Utc.with_ymd_and_hms(2026, 1, 28, 0, 0, 0).unwrap(),
    );
    authored.id = "authored-brasileirao".to_string();
    authored.country_id = Some("BR".to_string());
    authored.region_id = Some("south-america".to_string());
    authored.season_start_month = 1;
    authored.season_start_day = 28;
    game.competitions = vec![authored];

    let anchor = Utc.with_ymd_and_hms(2025, 12, 15, 0, 0, 0).unwrap();
    game.clock.start_date = anchor;
    game.clock.current_date = anchor;
    rebuild_competitions_for_management_date(&mut game, anchor);

    let competition = game
        .competitions
        .iter()
        .find(|competition| competition.id == "authored-brasileirao")
        .unwrap();
    assert_eq!(competition.season, 2026);
    assert!(competition
        .fixtures
        .iter()
        .any(|fixture| fixture.date == "2026-01-28"));
}

#[test]
fn select_continental_entrants_caps_the_field() {
    let teams: Vec<domain::team::Team> = (0..10)
        .map(|index| {
            let mut team = domain::team::Team::new(
                format!("eng-{index}"),
                format!("Club {index}"),
                format!("C{index}"),
                "Country".to_string(),
                "City".to_string(),
                "Stadium".to_string(),
                10_000,
            );
            team.football_nation = "GB".to_string();
            team.reputation = 1000 - index as u32;
            team
        })
        .collect();

    let entrants = select_continental_entrants(&teams, 8, 4);

    assert_eq!(entrants.len(), 4);
    assert_eq!(entrants[0], "eng-0", "strongest club is seeded first");
}

#[test]
fn resolve_simulation_scope_auto_enables_required_regions_and_team_competitions() {
    let mut game = make_bootstrap_test_game();
    game.teams[0].football_nation = "BR".to_string();
    game.teams[1].football_nation = "GB".to_string();

    let mut domestic = League::new(
        "domestic-1".to_string(),
        "Brazil League".to_string(),
        2032,
        &["team1".to_string()],
    );
    domestic.region_id = Some("south-america".to_string());
    domestic.required_region_ids = vec!["south-america".to_string()];
    domestic.priority = 0;

    let mut continental = League::new(
        "continental-1".to_string(),
        "Continental Champions Cup".to_string(),
        2032,
        &["team1".to_string(), "team2".to_string()],
    );
    continental.scope = CompetitionScope::Continental;
    continental.required_region_ids = vec!["south-america".to_string(), "europe".to_string()];
    continental.priority = 1;

    game.competitions = vec![domestic.clone(), continental.clone()];

    let (active_regions, active_competitions) = resolve_simulation_scope(
        &game,
        "team1",
        Some(vec!["south-america".to_string()]),
        Some(vec![continental.id.clone()]),
    )
    .unwrap();

    assert_eq!(
        active_regions,
        vec!["europe".to_string(), "south-america".to_string()]
    );
    assert_eq!(
        active_competitions,
        vec![domestic.id.clone(), continental.id.clone()]
    );
}

#[test]
fn resolve_simulation_scope_defaults_to_team_region_when_no_scope_is_provided() {
    let mut game = make_bootstrap_test_game();
    game.teams[0].football_nation = "BR".to_string();

    let mut domestic = League::new(
        "domestic-1".to_string(),
        "Brazil League".to_string(),
        2032,
        &["team1".to_string()],
    );
    domestic.region_id = Some("south-america".to_string());
    domestic.required_region_ids = vec!["south-america".to_string()];
    domestic.priority = 0;
    game.competitions = vec![domestic.clone()];

    let (active_regions, active_competitions) =
        resolve_simulation_scope(&game, "team1", None, None).unwrap();

    assert_eq!(active_regions, vec!["south-america".to_string()]);
    assert_eq!(active_competitions, vec![domestic.id.clone()]);
}

#[test]
fn load_world_data_from_path_returns_read_file_key_when_missing() {
    let result = load_world_data_from_path("file:Z:/definitely-missing/openfootmanager-world.json");

    assert_eq!(result.unwrap_err(), "be.error.worldReadFileFailed");
}

fn sample_stats_state() -> domain::stats::StatsState {
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

fn make_imported_baseline_world_without_staff() -> WorldData {
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
            "2008-01-01",
        ));
        players.push(make_player(
            format!("{}-mid-youth", team.id),
            domain::player::Position::Midfielder,
            "2007-01-01",
        ));
        players.push(make_player(
            format!("{}-fwd-youth", team.id),
            domain::player::Position::Forward,
            "2006-01-01",
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

fn make_historical_snapshot_world() -> WorldData {
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

#[test]
fn map_save_manager_lock_error_returns_backend_key_for_poisoned_mutex() {
    let mutex = Mutex::new(());
    let _ = std::panic::catch_unwind(|| {
        let _guard = mutex.lock().unwrap();
        panic!("poison save manager mutex for test");
    });

    let result = map_save_manager_lock_error(mutex.lock());

    assert_eq!(result.unwrap_err(), "be.error.saveManagerUnavailable");
}

#[test]
fn normalize_startup_options_defaults_to_current_year_and_season_start() {
    let options = normalize_startup_options(None).unwrap();

    assert!(options.start_year >= 2020);
    assert_eq!(options.start_phase, StartPhase::SeasonStart);
    assert_eq!(
        options.history_depth_years,
        DEFAULT_GENERATED_HISTORY_DEPTH_YEARS
    );
}

#[test]
fn normalize_startup_options_rejects_years_before_2020() {
    let result = normalize_startup_options(Some(RawStartupOptions {
        start_year: Some(2019),
        start_phase: Some("seasonStart".to_string()),
        history_depth_years: None,
    }));

    assert_eq!(result.unwrap_err(), "be.error.createManager.startYearMin");
}

#[test]
fn normalize_startup_options_rejects_unknown_start_phase() {
    let result = normalize_startup_options(Some(RawStartupOptions {
        start_year: Some(2026),
        start_phase: Some("playoffs".to_string()),
        history_depth_years: None,
    }));

    assert_eq!(
        result.unwrap_err(),
        "be.error.createManager.invalidStartPhase"
    );
}

#[test]
fn normalize_startup_options_rejects_history_depths_above_maximum() {
    let result = normalize_startup_options(Some(RawStartupOptions {
        start_year: Some(2026),
        start_phase: Some("seasonStart".to_string()),
        history_depth_years: Some(MAX_GENERATED_HISTORY_DEPTH_YEARS + 1),
    }));

    assert_eq!(
        result.unwrap_err(),
        "be.error.createManager.historyDepthMax"
    );
}

#[test]
fn normalize_startup_options_accepts_custom_history_depth() {
    let options = normalize_startup_options(Some(RawStartupOptions {
        start_year: Some(2026),
        start_phase: Some("seasonStart".to_string()),
        history_depth_years: Some(24),
    }))
    .unwrap();

    assert_eq!(options.history_depth_years, 24);
}

#[test]
fn start_date_for_year_uses_selected_july_first() {
    let start_date = start_date_for_year(2032).unwrap();

    assert_eq!(start_date.to_rfc3339(), "2032-07-01T00:00:00+00:00");
}

#[test]
fn start_date_for_year_rejects_out_of_range_years() {
    let result = start_date_for_year(i32::MAX);

    assert_eq!(
        result.unwrap_err(),
        "be.error.createManager.invalidStartYear"
    );
}

#[test]
fn current_date_for_midseason_phase_is_after_start_date() {
    let current_date = current_date_for_phase(2032, StartPhase::MidSeason).unwrap();

    assert_eq!(current_date.to_rfc3339(), "2032-10-29T00:00:00+00:00");
}

#[test]
fn age_on_date_uses_selected_start_year() {
    let birth_date = chrono::NaiveDate::from_ymd_opt(2008, 1, 1).unwrap();
    let reference_date = current_date_for_phase(2038, StartPhase::SeasonStart)
        .unwrap()
        .date_naive();

    assert_eq!(age_on_date(birth_date, reference_date), 30);
}

#[test]
fn age_on_date_changes_between_season_start_and_midseason() {
    let birth_date = chrono::NaiveDate::from_ymd_opt(2008, 8, 1).unwrap();
    let season_start = current_date_for_phase(2038, StartPhase::SeasonStart)
        .unwrap()
        .date_naive();
    let midseason = current_date_for_phase(2038, StartPhase::MidSeason)
        .unwrap()
        .date_naive();

    assert_eq!(age_on_date(birth_date, season_start), 29);
    assert_eq!(age_on_date(birth_date, midseason), 30);
}

#[test]
fn age_on_date_uses_world_snapshot_date_over_startup_phase() {
    let startup_options = StartupOptions {
        start_year: 2032,
        start_phase: StartPhase::MidSeason,
        history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
    };
    let world = make_historical_snapshot_world();
    let reference_date = game_clock_for_world(&startup_options, &world.metadata)
        .unwrap()
        .current_date
        .date_naive();
    let birth_date = chrono::NaiveDate::from_ymd_opt(2001, 12, 15).unwrap();

    assert_eq!(reference_date.to_string(), "2031-11-20");
    assert_eq!(age_on_date(birth_date, reference_date), 29);
}

#[test]
fn preseason_league_setup_uses_selected_start_year_for_context() {
    let clock = GameClock::new(start_date_for_year(2032).unwrap());
    let manager = domain::manager::Manager::new(
        "mgr1".to_string(),
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
    let mut game = Game::new(clock, manager, teams, vec![], vec![], vec![]);

    let season_start = preseason_season_start(&game.clock);
    let team_ids = game
        .teams
        .iter()
        .map(|team| team.id.clone())
        .collect::<Vec<_>>();
    game.league = Some(ofm_core::schedule::generate_league(
        "Premier Division",
        preseason_league_year(&game.clock),
        &team_ids,
        season_start,
    ));
    refresh_game_context(&mut game);

    assert_eq!(
        game.clock.start_date.to_rfc3339(),
        "2032-07-01T00:00:00+00:00"
    );
    assert_eq!(game.league.as_ref().map(|league| league.season), Some(2032));
    assert_eq!(
        game.season_context.season_start.as_deref(),
        Some("2032-07-31")
    );
    assert_eq!(game.season_context.days_until_season_start, Some(30));
}

#[test]
fn apply_generated_past_history_populates_default_twelve_prior_seasons() {
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
    let players = vec![
        {
            let mut player = domain::player::Player::new(
                "player1".to_string(),
                "A. Keeper".to_string(),
                "Alex Keeper".to_string(),
                "1994-01-01".to_string(),
                "England".to_string(),
                domain::player::Position::Goalkeeper,
                domain::player::PlayerAttributes {
                    pace: 48,
                    stamina: 62,
                    strength: 64,
                    agility: 66,
                    passing: 50,
                    shooting: 20,
                    tackling: 18,
                    dribbling: 32,
                    defending: 24,
                    positioning: 68,
                    vision: 48,
                    decisions: 63,
                    composure: 61,
                    aggression: 38,
                    teamwork: 64,
                    leadership: 58,
                    handling: 76,
                    reflexes: 77,
                    aerial: 72,
                },
            );
            player.team_id = Some("team1".to_string());
            player.ovr = 68;
            player.potential = 73;
            player
        },
        {
            let mut player = domain::player::Player::new(
                "player2".to_string(),
                "A. Striker".to_string(),
                "Alex Striker".to_string(),
                "1996-01-01".to_string(),
                "England".to_string(),
                domain::player::Position::Striker,
                domain::player::PlayerAttributes {
                    pace: 72,
                    stamina: 68,
                    strength: 70,
                    agility: 71,
                    passing: 60,
                    shooting: 79,
                    tackling: 34,
                    dribbling: 73,
                    defending: 28,
                    positioning: 74,
                    vision: 62,
                    decisions: 68,
                    composure: 69,
                    aggression: 52,
                    teamwork: 64,
                    leadership: 47,
                    handling: 18,
                    reflexes: 18,
                    aerial: 61,
                },
            );
            player.team_id = Some("team1".to_string());
            player.ovr = 74;
            player.potential = 80;
            player
        },
        {
            let mut player = domain::player::Player::new(
                "player3".to_string(),
                "B. Keeper".to_string(),
                "Ben Keeper".to_string(),
                "1993-01-01".to_string(),
                "England".to_string(),
                domain::player::Position::Goalkeeper,
                domain::player::PlayerAttributes {
                    pace: 47,
                    stamina: 61,
                    strength: 63,
                    agility: 65,
                    passing: 49,
                    shooting: 19,
                    tackling: 18,
                    dribbling: 30,
                    defending: 23,
                    positioning: 67,
                    vision: 47,
                    decisions: 62,
                    composure: 60,
                    aggression: 39,
                    teamwork: 63,
                    leadership: 57,
                    handling: 75,
                    reflexes: 76,
                    aerial: 71,
                },
            );
            player.team_id = Some("team2".to_string());
            player.ovr = 67;
            player.potential = 72;
            player
        },
        {
            let mut player = domain::player::Player::new(
                "player4".to_string(),
                "B. Striker".to_string(),
                "Ben Striker".to_string(),
                "1995-01-01".to_string(),
                "England".to_string(),
                domain::player::Position::Striker,
                domain::player::PlayerAttributes {
                    pace: 71,
                    stamina: 67,
                    strength: 69,
                    agility: 70,
                    passing: 59,
                    shooting: 78,
                    tackling: 33,
                    dribbling: 72,
                    defending: 27,
                    positioning: 73,
                    vision: 61,
                    decisions: 67,
                    composure: 68,
                    aggression: 51,
                    teamwork: 63,
                    leadership: 46,
                    handling: 18,
                    reflexes: 18,
                    aerial: 60,
                },
            );
            player.team_id = Some("team2".to_string());
            player.ovr = 73;
            player.potential = 79;
            player
        },
    ];
    let mut game = Game::new(clock, manager, teams, players, staff, vec![]);

    apply_generated_past_history(
        &mut game,
        &StartupOptions {
            start_year: 2032,
            start_phase: StartPhase::SeasonStart,
            history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
        },
    );

    assert!(game.teams.iter().all(|team| team.history.len() == 12));
    assert_eq!(game.world_history.season_awards.len(), 12);
    assert!(game.players.iter().any(|player| player.career.len() == 12));
    assert!(game
        .managers
        .iter()
        .any(|manager| !manager.career_history.is_empty()));
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
    ofm_core::generator::normalize_imported_world_for_career_start(&mut world);
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
    ofm_core::generator::normalize_imported_world_for_career_start(&mut world);
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
    ofm_core::generator::normalize_imported_world_for_career_start(&mut world);
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
fn game_clock_for_world_rejects_out_of_range_snapshot_base_year() {
    let startup_options = StartupOptions {
        start_year: 2032,
        start_phase: StartPhase::MidSeason,
        history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
    };
    let mut world = make_historical_snapshot_world();
    world.metadata.base_year = Some(i32::MAX);

    let result = game_clock_for_world(&startup_options, &world.metadata);

    assert_eq!(
        result.unwrap_err(),
        "be.error.createManager.invalidStartYear"
    );
}

#[test]
fn create_new_save_persists_stats_state_on_first_save() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let saves_dir = std::env::temp_dir().join(format!("ofm-game-command-tests-{}", unique));
    std::fs::create_dir_all(&saves_dir).unwrap();
    let mut save_manager = SaveManager::init(&saves_dir).unwrap();
    let game = make_bootstrap_test_game();
    let stats_state = sample_stats_state();

    let save_id = create_new_save(&mut save_manager, &game, &stats_state, "Stats Career").unwrap();
    let loaded_stats = save_manager.load_stats_state(&save_id).unwrap();

    assert_eq!(loaded_stats.team_matches.len(), 1);
    assert_eq!(loaded_stats.player_matches.len(), 1);
    assert_eq!(loaded_stats.team_matches[0].team_id, "team1");

    std::fs::remove_dir_all(&saves_dir).unwrap();
}

#[test]
fn require_active_stats_state_returns_backend_key_when_missing() {
    let state = StateManager::new();

    let result = require_active_stats_state(&state);

    assert_eq!(result.unwrap_err(), "be.error.noActiveStatsSession");
}

#[test]
fn require_active_stats_state_clones_active_stats() {
    let state = StateManager::new();
    let stats = sample_stats_state();
    state.set_stats_state(stats.clone());

    let result = require_active_stats_state(&state).unwrap();

    assert_eq!(result.team_matches.len(), stats.team_matches.len());
    assert_eq!(result.player_matches.len(), stats.player_matches.len());
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
    ofm_core::generator::normalize_imported_world_for_career_start(&mut world);
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

#[test]
fn brazil_state_region_covers_all_standard_br_cities() {
    // All cities from STANDARD_NATIONS BR entry must map to a region so that
    // state-series competitions are generated for every club location.
    let br_cities = [
        "São Paulo",
        "Rio",
        "Belo Horizonte",
        "Porto Alegre",
        "Salvador",
        "Recife",
        "Curitiba",
        "Fortaleza",
        "Goiânia",
        "Santos",
        "Campinas",
        "Belém",
        "Manaus",
        "Vitória",
        "Natal",
        "Florianópolis",
        "Cuiabá",
        "Maceió",
        "Bragantino",
        "Juiz de Fora",
    ];
    for city in br_cities {
        assert!(
            brazil_state_region(city).is_some(),
            "brazil_state_region returned None for BR city: {city}"
        );
    }
    assert_eq!(
        brazil_state_region("Vitória"),
        Some("southeast"),
        "Vitória (ES) belongs in the southeast region, not northeast"
    );
}

#[test]
fn division_tier_name_labels_single_and_multi_division_pyramids() {
    assert_eq!(division_tier_name(0, 1), "League");
    assert_eq!(division_tier_name(0, 2), "First Division");
    assert_eq!(division_tier_name(1, 2), "Second Division");
    // Any tier below the top in a multi-division pyramid reads as "Second Division".
    assert_eq!(division_tier_name(2, 3), "Second Division");
}

#[test]
fn division_tier_name_key_mirrors_the_display_labels() {
    assert_eq!(
        division_tier_name_key(0, 1),
        "tournaments.competitions.league"
    );
    assert_eq!(
        division_tier_name_key(0, 2),
        "tournaments.competitions.firstDivision"
    );
    assert_eq!(
        division_tier_name_key(1, 2),
        "tournaments.competitions.secondDivision"
    );
    assert_eq!(
        division_tier_name_key(3, 4),
        "tournaments.competitions.secondDivision"
    );
}

#[test]
fn division_name_prefixes_the_country() {
    assert_eq!(division_name("England", 0, 1), "England League");
    assert_eq!(division_name("Spain", 0, 2), "Spain First Division");
    assert_eq!(division_name("Italy", 1, 2), "Italy Second Division");
}

#[test]
fn default_season_month_for_region_maps_known_regions_and_defaults_to_august() {
    assert_eq!(default_season_month_for_region("south-america"), 3);
    assert_eq!(default_season_month_for_region("asia"), 2);
    assert_eq!(default_season_month_for_region("oceania"), 10);
    assert_eq!(default_season_month_for_region("europe"), 8);
    assert_eq!(default_season_month_for_region("unknown-region"), 8);
}

#[test]
fn competition_required_region_ids_includes_domestic_region_and_dedups() {
    let mut league = League::new("l1".to_string(), "League One".to_string(), 2026, &[]);
    league.scope = CompetitionScope::Domestic;
    league.region_id = Some("south-america".to_string());
    league.required_region_ids = vec![
        "europe".to_string(),
        "asia".to_string(),
        "europe".to_string(),
    ];

    assert_eq!(
        competition_required_region_ids(&league),
        vec![
            "asia".to_string(),
            "europe".to_string(),
            "south-america".to_string(),
        ],
    );
}

#[test]
fn competition_required_region_ids_ignores_region_for_non_regional_scopes() {
    let mut league = League::new("l2".to_string(), "Continental Cup".to_string(), 2026, &[]);
    league.scope = CompetitionScope::Continental;
    league.region_id = Some("europe".to_string());

    assert!(competition_required_region_ids(&league).is_empty());
}
