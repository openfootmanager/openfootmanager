//! Envelope rollover: unrenewed clubs keep spent-down available room.

use super::*;
use domain::finance::{CashJournal, CashKind, CashPost};
use ofm_core::transfers::available_transfer_budget;

/// Given a finished user league and a foreign league still playing, whose club has spent against its limit
/// When the user's season rolls over
/// Then the unfinished club's available room stays spent-down; the cutoff does not restore the gross limit
#[test]
fn an_unfinished_foreign_club_keeps_counting_settled_spend_after_rollover() {
    let mut game = make_completed_season_game();
    game.teams.push(make_team("team3", "Brazil FC"));
    game.teams.push(make_team("team4", "Rio FC"));
    let team3_idx = game.teams.iter().position(|t| t.id == "team3").unwrap();
    game.teams[team3_idx].finance = 2_000_000;
    game.teams[team3_idx].transfer_budget = 500_000;
    game.cash_journal = CashJournal::from_vec(vec![CashPost {
        id: "bra-out".into(),
        club_id: "team3".into(),
        amount: -400_000,
        kind: CashKind::TransferFeeOut,
        date: "2026-04-01".into(),
    }]);

    let eng = League {
        id: "eng-1".to_string(),
        name: "ENG First Division".to_string(),
        country_id: Some("ENG".to_string()),
        priority: 0,
        season: 1,
        participant_ids: vec!["team1".to_string(), "team2".to_string()],
        fixtures: vec![
            make_completed_fixture("d1f1", "team1", "team2", 2, 0),
            make_completed_fixture("d1f2", "team2", "team1", 0, 1),
        ],
        standings: vec![
            make_standing("team1", 2, 0, 0, 3, 0),
            make_standing("team2", 0, 0, 2, 0, 3),
        ],
        ..Default::default()
    };
    let mut bra_open = make_completed_fixture("b1f2", "team4", "team3", 0, 0);
    bra_open.status = FixtureStatus::Scheduled;
    bra_open.result = None;
    bra_open.matchday = 2;
    bra_open.date = "2026-06-01".to_string();
    let bra = League {
        id: "bra-1".to_string(),
        name: "BRA First Division".to_string(),
        country_id: Some("BRA".to_string()),
        priority: 0,
        season: 1,
        participant_ids: vec!["team3".to_string(), "team4".to_string()],
        fixtures: vec![
            make_completed_fixture("b1f1", "team3", "team4", 1, 0),
            bra_open,
        ],
        standings: vec![
            make_standing("team3", 1, 0, 0, 1, 0),
            make_standing("team4", 0, 0, 1, 0, 1),
        ],
        ..Default::default()
    };
    game.league = Some(eng.clone());
    game.competitions = vec![eng, bra];

    let team3_before = game
        .teams
        .iter()
        .find(|team| team.id == "team3")
        .unwrap()
        .clone();
    assert_eq!(available_transfer_budget(&game, &team3_before), 100_000);

    process_end_of_season(&mut game);

    let team3 = game
        .teams
        .iter()
        .find(|team| team.id == "team3")
        .unwrap()
        .clone();
    assert_eq!(available_transfer_budget(&game, &team3), 100_000);
    assert_eq!(team3.transfer_budget, 100_000);
}
