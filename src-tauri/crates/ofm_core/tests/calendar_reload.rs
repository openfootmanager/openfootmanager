use chrono::{TimeZone, Utc};
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

/// Given a freshly generated career, when its calendar is checked as on load,
/// then no fixtures move or disappear and no postponement is announced.
#[test]
fn a_new_career_needs_no_calendar_repair() {
    let (mut game, _) = career();
    let before = serde_json::to_value(&game.competitions).unwrap();
    let messages = game.messages.len();
    let changed = ofm_core::calendar_repair::repair_loaded_calendar(&mut game);
    assert_eq!(changed, 0, "the generated calendar must already be settled");
    assert_eq!(serde_json::to_value(&game.competitions).unwrap(), before);
    assert_eq!(game.messages.len(), messages);
}

/// Given a career played through a season and rollover, when its calendar is checked as on load,
/// then no fixtures move or disappear and no postponement is announced.
#[test]
fn a_rolled_over_career_needs_no_calendar_repair() {
    let (mut game, mut stats) = career();
    for _ in 0..600 {
        if is_season_complete(&game) {
            break;
        }
        turn::process_day_with_capture(&mut game, &mut |day_stats| stats.append(day_stats));
    }
    assert!(is_season_complete(&game));
    advance_to_next_season(&mut game).unwrap();
    let before = serde_json::to_value(&game.competitions).unwrap();
    let messages = game.messages.len();
    let changed = ofm_core::calendar_repair::repair_loaded_calendar(&mut game);
    assert_eq!(changed, 0, "the rollover calendar must already be settled");
    assert_eq!(serde_json::to_value(&game.competitions).unwrap(), before);
    assert_eq!(game.messages.len(), messages);
}
