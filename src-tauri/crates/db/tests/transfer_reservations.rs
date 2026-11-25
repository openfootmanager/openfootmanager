use chrono::{TimeZone, Utc};
use db::save_manager::SaveManager;
use domain::{
    manager::Manager,
    player::{Player, PlayerAttributes, Position},
    stats::StatsState,
    team::Team,
};
use ofm_core::{clock::GameClock, game::Game, transfers::project_transfer_bid_financial_impact};

/// Given an old-format agreed deal with no separate reservation field, when saved to SQLite and read by a fresh manager, then its fee remains unavailable and cash is untouched.
#[test]
fn agreed_transfer_reservations_survive_a_fresh_sqlite_reader() {
    let mut buyer = Team::new(
        "buyer".into(),
        "Buyer".into(),
        "BUY".into(),
        "England".into(),
        "Town".into(),
        "Ground".into(),
        10_000,
    );
    buyer.finance = 7_654_321;
    buyer.transfer_budget = 3_456_789;
    let mut seller = buyer.clone();
    seller.id = "seller".into();
    let mut manager = Manager::new(
        "manager".into(),
        "Test".into(),
        "Manager".into(),
        "1980-01-01".into(),
        "England".into(),
    );
    manager.hire("buyer".into());
    let mut player = Player::new("agreed".into(), "Test".into(), "Player".into(), "2000-01-01".into(), "England".into(), Position::Forward, serde_json::from_value::<PlayerAttributes>(serde_json::json!({"pace":60, "stamina":60, "strength":60, "agility":60, "passing":60, "shooting":60, "tackling":60, "dribbling":60, "defending":60, "positioning":60, "vision":60, "decisions":60, "composure":60, "aggression":60, "teamwork":60, "leadership":60, "handling":60, "reflexes":60, "aerial":60})).unwrap());
    player.team_id = Some("seller".into());
    player.transfer_offers.push(serde_json::from_value(serde_json::json!({
        "id":"agreed-offer", "from_team_id":"buyer", "fee":876_543,
        "wage_offered":0, "status":"PendingRegistration", "date":"2026-01-12", "registration_date":"2026-07-02"
    })).unwrap());
    let mut next = player.clone();
    next.id = "next".into();
    next.transfer_offers.clear();
    let game = Game::new(
        GameClock::new(Utc.with_ymd_and_hms(2026, 1, 12, 12, 0, 0).unwrap()),
        manager,
        vec![buyer, seller],
        vec![player, next],
        vec![],
        vec![],
    );
    let dir = tempfile::tempdir().unwrap();
    let mut writer = SaveManager::init(dir.path()).unwrap();
    let save_id = writer.create_save(&game, "Reservations").unwrap();
    writer
        .save_game_with_stats(&game, &StatsState::default(), &save_id)
        .unwrap();
    drop(writer);
    let mut reader = SaveManager::init(dir.path()).unwrap();
    let loaded = reader.load_game(&save_id).unwrap();
    let preview = project_transfer_bid_financial_impact(&loaded, "next", 1).unwrap();
    assert_eq!(preview.finance_before, 7_654_321);
    assert_eq!(preview.transfer_budget_before, 2_580_246);
}
