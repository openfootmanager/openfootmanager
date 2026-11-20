use chrono::{TimeZone, Utc};
use db::game_database::GameDatabase;
use db::repositories::meta_repo;
use db::save_manager::SaveManager;
use domain::{
    finance::{CashJournal, CashKind, CashPost},
    manager::Manager,
    stats::StatsState,
    team::Team,
};
use ofm_core::{
    clock::GameClock,
    game::Game,
    transfers::{available_transfer_budget, settled_transfer_spend},
};

fn remaining_after_purchase_game() -> Game {
    let mut buyer = Team::new(
        "buyer".into(),
        "Buyer".into(),
        "BUY".into(),
        "England".into(),
        "Town".into(),
        "Ground".into(),
        10_000,
    );
    buyer.finance = 3_500_000;
    // Pre-v10 remaining after a 1.5m purchase from a 3m limit.
    buyer.transfer_budget = 1_500_000;
    let mut seller = buyer.clone();
    seller.id = "seller".into();
    seller.finance = 6_500_000;
    seller.transfer_budget = 4_500_000;
    let mut manager = Manager::new(
        "manager".into(),
        "Test".into(),
        "Manager".into(),
        "1980-01-01".into(),
        "England".into(),
    );
    manager.hire("buyer".into());
    let mut game = Game::new(
        GameClock::new(Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap()),
        manager,
        vec![buyer, seller],
        vec![],
        vec![],
        vec![],
    );
    game.cash_journal = CashJournal::from_vec(vec![
        CashPost {
            id: "buy-open".into(),
            club_id: "buyer".into(),
            amount: 5_000_000,
            kind: CashKind::OpeningBalance,
            date: "2026-07-01".into(),
        },
        CashPost {
            id: "buy-out".into(),
            club_id: "buyer".into(),
            amount: -1_500_000,
            kind: CashKind::TransferFeeOut,
            date: "2026-08-01".into(),
        },
        CashPost {
            id: "sell-open".into(),
            club_id: "seller".into(),
            amount: 5_000_000,
            kind: CashKind::OpeningBalance,
            date: "2026-07-01".into(),
        },
        CashPost {
            id: "sell-in".into(),
            club_id: "seller".into(),
            amount: 1_500_000,
            kind: CashKind::TransferFeeIn,
            date: "2026-08-01".into(),
        },
    ]);
    game
}

/// Given a format-9 save that stored remaining after settlement
/// When it is loaded by a format-10 build
/// Then each club's transfer_budget is lifted to the season limit once
#[test]
fn a_pre_v10_save_lifts_remaining_to_the_season_limit() {
    let game = remaining_after_purchase_game();
    let dir = tempfile::tempdir().unwrap();
    let mut writer = SaveManager::init(dir.path()).unwrap();
    let save_id = writer.create_save(&game, "Envelope").unwrap();
    writer
        .save_game_with_stats(&game, &StatsState::default(), &save_id)
        .unwrap();
    drop(writer);

    let db_path = dir.path().join(format!("{save_id}.db"));
    {
        let db = GameDatabase::open(&db_path).unwrap();
        db.conn()
            .execute(
                "UPDATE game_meta SET save_format_version = 9 WHERE id = 'singleton'",
                [],
            )
            .unwrap();
    }

    let mut reader = SaveManager::init(dir.path()).unwrap();
    let loaded = reader.load_game(&save_id).unwrap();
    let buyer = loaded.teams.iter().find(|team| team.id == "buyer").unwrap();
    let seller = loaded
        .teams
        .iter()
        .find(|team| team.id == "seller")
        .unwrap();
    assert_eq!(buyer.transfer_budget, 3_000_000);
    assert_eq!(seller.transfer_budget, 3_000_000);
    assert_eq!(settled_transfer_spend(&loaded, "buyer"), 1_500_000);
    assert_eq!(settled_transfer_spend(&loaded, "seller"), -1_500_000);
    assert_eq!(available_transfer_budget(&loaded, buyer), 1_500_000);
    assert_eq!(available_transfer_budget(&loaded, seller), 4_500_000);

    let db = GameDatabase::open(&db_path).unwrap();
    let meta = meta_repo::load_meta(db.conn()).unwrap().unwrap();
    assert_eq!(
        meta.save_format_version,
        meta_repo::CURRENT_SAVE_FORMAT_VERSION
    );
}

/// Given a format-10 save that already stores the season limit
/// When it is loaded
/// Then transfer_budget is not lifted again
#[test]
fn a_v10_save_does_not_lift_the_limit_again() {
    let mut game = remaining_after_purchase_game();
    game.teams[0].transfer_budget = 3_000_000;
    game.teams[1].transfer_budget = 3_000_000;
    let dir = tempfile::tempdir().unwrap();
    let mut writer = SaveManager::init(dir.path()).unwrap();
    let save_id = writer.create_save(&game, "Envelope v10").unwrap();
    writer
        .save_game_with_stats(&game, &StatsState::default(), &save_id)
        .unwrap();
    drop(writer);

    let mut reader = SaveManager::init(dir.path()).unwrap();
    let loaded = reader.load_game(&save_id).unwrap();
    let buyer = loaded.teams.iter().find(|team| team.id == "buyer").unwrap();
    let seller = loaded
        .teams
        .iter()
        .find(|team| team.id == "seller")
        .unwrap();
    assert_eq!(buyer.transfer_budget, 3_000_000);
    assert_eq!(seller.transfer_budget, 3_000_000);
    assert_eq!(available_transfer_budget(&loaded, buyer), 1_500_000);
    assert_eq!(available_transfer_budget(&loaded, seller), 4_500_000);
}
