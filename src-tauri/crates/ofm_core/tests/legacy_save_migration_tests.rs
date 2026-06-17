use chrono::{TimeZone, Utc};
use domain::league::{CompetitionScope, CompetitionType, League, StandingEntry};
use domain::manager::Manager;
use domain::team::Team;
use ofm_core::clock::GameClock;
use ofm_core::game::Game;

fn team(id: &str) -> Team {
    Team::new(
        id.to_string(),
        format!("{id} FC"),
        id.to_uppercase(),
        "England".to_string(),
        "Town".to_string(),
        "Ground".to_string(),
        20_000,
    )
}

/// A save written before the multi-competition system: `league` populated,
/// `competitions` empty.
fn legacy_save() -> Game {
    let clock = GameClock::new(Utc.with_ymd_and_hms(2025, 8, 1, 12, 0, 0).unwrap());
    let mut manager = Manager::new(
        "mgr".to_string(),
        "A".to_string(),
        "B".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("home".to_string());

    let mut game = Game::new(
        clock,
        manager,
        vec![team("home"), team("away")],
        vec![],
        vec![],
        vec![],
    );

    let mut league = League::new(
        "legacy-league".to_string(),
        "Old First Division".to_string(),
        2025,
        &["home".to_string(), "away".to_string()],
    );
    let mut home_standing = StandingEntry::new("home".to_string());
    home_standing.points = 9;
    home_standing.played = 4;
    league.standings = vec![home_standing, StandingEntry::new("away".to_string())];

    game.league = Some(league);
    game.competitions = vec![];
    game.active_competition_ids = vec![];
    game
}

#[test]
fn migration_promotes_legacy_league_into_competitions() {
    let mut game = legacy_save();

    let migrated = game.migrate_legacy_league_into_competitions();

    assert!(migrated, "an old save reports that it migrated");
    assert_eq!(game.competitions.len(), 1, "the legacy league becomes a competition");
    let competition = &game.competitions[0];
    assert_eq!(competition.id, "legacy-league");
    assert_eq!(competition.kind, CompetitionType::League);
    assert_eq!(competition.scope, CompetitionScope::Domestic);

    // In-progress standings are preserved, not regenerated.
    let home = competition
        .standings
        .iter()
        .find(|entry| entry.team_id == "home")
        .expect("home standing preserved");
    assert_eq!(home.points, 9);
    assert_eq!(home.played, 4);

    // The migrated competition is registered as active, and the legacy mirror
    // is kept (and now points at the user's competition).
    assert_eq!(game.active_competition_ids, vec!["legacy-league".to_string()]);
    assert!(game.league.is_some());
    assert_eq!(game.league.as_ref().unwrap().id, "legacy-league");
}

#[test]
fn migration_is_noop_for_current_saves_with_competitions() {
    let mut game = legacy_save();
    let modern = League::new(
        "modern-league".to_string(),
        "Modern Division".to_string(),
        2026,
        &["home".to_string()],
    );
    game.competitions = vec![modern];
    game.active_competition_ids = vec!["modern-league".to_string()];

    let migrated = game.migrate_legacy_league_into_competitions();

    assert!(!migrated, "a current save is left untouched");
    assert_eq!(game.competitions.len(), 1);
    assert_eq!(game.competitions[0].id, "modern-league");
    assert_eq!(game.active_competition_ids, vec!["modern-league".to_string()]);
}
