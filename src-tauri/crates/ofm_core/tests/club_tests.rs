use chrono::{TimeZone, Utc};
use domain::manager::Manager;
use domain::team::{Facilities, FacilityType, Team};
use ofm_core::clock::GameClock;
use ofm_core::club;
use ofm_core::game::Game;

fn make_team(id: &str, name: &str) -> Team {
    let mut team = Team::new(
        id.to_string(),
        name.to_string(),
        name[..3].to_string(),
        "England".to_string(),
        "London".to_string(),
        "Stadium".to_string(),
        40_000,
    );
    team.finance = 2_000_000;
    team
}

fn make_game() -> Game {
    let date = Utc.with_ymd_and_hms(2025, 6, 16, 12, 0, 0).unwrap();
    let clock = GameClock::new(date);
    let mut manager = Manager::new(
        "mgr1".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("team1".to_string());

    let team1 = make_team("team1", "Test FC");

    Game::new(clock, manager, vec![team1], vec![], vec![], vec![])
}

#[test]
fn upgrade_facility_deducts_funds_and_increments_level() {
    let mut game = make_game();
    let initial_finance = game.teams[0].finance;

    let cost = club::upgrade_facility(&mut game.teams[0], FacilityType::Medical).unwrap();

    assert_eq!(cost, 250_000);
    assert_eq!(game.teams[0].finance, initial_finance - cost);
    assert_eq!(game.teams[0].facilities.medical, 2);
}

#[test]
fn upgrade_facility_rejects_when_funds_are_insufficient() {
    let mut game = make_game();
    game.teams[0].finance = 100_000;
    game.teams[0].facilities = Facilities {
        training: 1,
        medical: 1,
        scouting: 1,
    };

    let result = club::upgrade_facility(&mut game.teams[0], FacilityType::Training);

    assert_eq!(
        result.unwrap_err(),
        "be.error.finance.facilityUpgradeInsufficientFunds?amount=250000"
    );
    assert_eq!(game.teams[0].finance, 100_000);
    assert_eq!(game.teams[0].facilities.training, 1);
}
