use chrono::{TimeZone, Utc};
use db::save_manager::SaveManager;
use domain::{manager::Manager, stats::StatsState};
use ofm_core::{
    career::{CareerScope, begin_career},
    clock::GameClock,
    end_of_season::{advance_to_next_season, is_season_complete},
    game::Game,
    generator::{
        DefinitionSources, WorldGenConfig, generate_world_data_seeded_with,
        repair_opening_youth_academies,
    },
    turn,
};
const SEED: u64 = 11;
fn career() -> (Game, StatsState) {
    let world = generate_world_data_seeded_with(
        SEED,
        &WorldGenConfig::compact(),
        &DefinitionSources::embedded_only(),
    );
    let start = Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap();
    let club = ofm_core::world::strongest_club(&world.teams, "ENG")
        .or_else(|| world.teams.first().map(|team| team.id.as_str()))
        .expect("the world has a club")
        .to_string();
    let mut manager = Manager::new(
        "save-round-trip-manager".to_string(),
        "Save".to_string(),
        "Tester".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire(club.clone());
    let mut game = Game::new(
        GameClock::new(start),
        manager,
        world.teams,
        world.players,
        world.staff,
        vec![],
    );
    game.seed = SEED;
    game.available_staff_market_last_activity_date = Some(start.format("%Y-%m-%d").to_string());
    repair_opening_youth_academies(&mut game);
    let stats = begin_career(
        &mut game,
        &club,
        CareerScope::default(),
        StatsState::default(),
    )
    .expect("the career begins");
    (game, stats)
}

fn fixture_dates(
    game: &Game,
) -> std::collections::BTreeMap<(String, String), (String, serde_json::Value)> {
    game.competitions
        .iter()
        .flat_map(|competition| {
            competition.fixtures.iter().map(move |fixture| {
                (
                    (competition.id.clone(), fixture.id.clone()),
                    (
                        fixture.date.clone(),
                        serde_json::to_value(&fixture.postponement).unwrap(),
                    ),
                )
            })
        })
        .collect()
}

fn assert_reload_keeps_calendar(game: &Game, stats: &StatsState) {
    let before = fixture_dates(game);
    assert!(!before.is_empty());
    let messages = game
        .messages
        .iter()
        .filter(|m| m.subject_key.as_deref() == Some("be.msg.matchesMoved.subject"))
        .count();
    let saves = tempfile::tempdir().unwrap();
    let mut writer = SaveManager::init(saves.path()).unwrap();
    let id = writer
        .create_save_with_stats(game, stats, "Calendar")
        .unwrap();
    let mut reader = SaveManager::init(saves.path()).unwrap();
    let loaded = reader.load_game(&id).unwrap();
    assert_eq!(
        fixture_dates(&loaded),
        before,
        "reload must preserve every fixture date and postponement"
    );
    assert_eq!(
        loaded
            .messages
            .iter()
            .filter(|m| m.subject_key.as_deref() == Some("be.msg.matchesMoved.subject"))
            .count(),
        messages
    );
}

/// Given a new generated career, when a fresh save manager loads its save,
/// then every fixture date and postponement stays unchanged and no notice is added.
#[test]
fn a_new_career_reloads_with_the_same_calendar() {
    let (game, stats) = career();
    assert_reload_keeps_calendar(&game, &stats);
}

/// Given a career played through one season and rollover, when a fresh save manager loads it,
/// then every fixture date and postponement stays unchanged and no notice is added.
#[test]
fn a_rolled_over_career_reloads_with_the_same_calendar() {
    let (mut game, mut stats) = career();
    for _ in 0..600 {
        if is_season_complete(&game) {
            break;
        }
        turn::process_day_with_capture(&mut game, &mut |day_stats| stats.append(day_stats));
    }
    assert!(is_season_complete(&game));
    advance_to_next_season(&mut game).unwrap();
    assert_reload_keeps_calendar(&game, &stats);
}
