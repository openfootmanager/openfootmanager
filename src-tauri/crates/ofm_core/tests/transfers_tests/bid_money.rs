use super::*;

/// Given a selling club whose cash would overflow on receiving the fee, in an open window
/// When the user bids enough for the club to accept
/// Then the bid is refused before anything is agreed: the player stays, carries no accepted
///      offer, and neither club's money moves
#[test]
fn a_fee_the_seller_cannot_book_is_refused_before_the_deal_is_agreed() {
    let player = make_player("player-overflow");
    let mut game = make_game_with_player(player, vec![], 5_000_000, 3_000_000);
    game.teams[1].finance = i64::MAX - 1_000;
    let money_before: Vec<(i64, i64)> = game
        .teams
        .iter()
        .map(|team| (team.finance, team.transfer_budget))
        .collect();

    let result = make_transfer_bid(&mut game, "player-overflow", 2_000_000);

    assert_eq!(
        result.err().as_deref(),
        Some("be.error.finance.amountOverflow")
    );
    let player = game
        .players
        .iter()
        .find(|player| player.id == "player-overflow")
        .unwrap();
    assert_eq!(player.team_id.as_deref(), Some("team-2"));
    assert!(
        player
            .transfer_offers
            .iter()
            .all(|offer| offer.status != TransferOfferStatus::Accepted),
        "an agreement was recorded for a deal whose money could not be booked"
    );
    let money_after: Vec<(i64, i64)> = game
        .teams
        .iter()
        .map(|team| (team.finance, team.transfer_budget))
        .collect();
    assert_eq!(money_after, money_before);
}

/// Given a club with 2,000,000 to spend on transfers and 5,000,000 in the bank
/// When bids of 1,000,000, exactly 2,000,000, 3,000,000, exactly 5,000,000 and 6,000,000 are previewed
/// Then the preview says a fee exceeds the transfer budget only above 2,000,000, and exceeds
///      the club's cash only above 5,000,000
#[test]
fn a_bid_preview_flags_a_fee_only_when_it_is_more_than_the_club_has() {
    let player = make_player("player-preview-flags");
    let game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);

    let flags: Vec<(u64, bool, bool)> = [1_000_000, 2_000_000, 3_000_000, 5_000_000, 6_000_000]
        .into_iter()
        .map(|fee| {
            let projection = ofm_core::transfers::project_transfer_bid_financial_impact(
                &game,
                "player-preview-flags",
                fee,
            )
            .expect("the preview is informational");
            (
                fee,
                projection.exceeds_transfer_budget,
                projection.exceeds_finance,
            )
        })
        .collect();

    assert_eq!(
        flags,
        vec![
            (1_000_000, false, false),
            (2_000_000, false, false),
            (3_000_000, true, false),
            (5_000_000, true, false),
            (6_000_000, true, true),
        ]
    );
}

/// Given a club whose wage budget leaves no room for the player's standard wage, and the same
///       club with a budget to spare, both able to pay a fee the seller would take
/// When a bid for him is previewed in each case
/// Then the preview flags the wage only where the board refuses it when the deal is agreed, by the
///      same rule that refusal uses
#[test]
fn a_bid_preview_flags_a_wage_the_board_would_refuse() {
    const FEE: u64 = 50_000_000;
    let player = make_player("player-wage-flag");
    let mut game = make_game_with_player(player, vec![], 100_000_000, 100_000_000);
    let preview = |game: &Game| {
        ofm_core::transfers::project_transfer_bid_financial_impact(game, "player-wage-flag", FEE)
            .expect("the preview is informational")
    };

    game.teams[0].wage_budget = 1_000_000_000;
    let roomy = preview(&game);
    let bill = roomy.current_weekly_wage_spend;
    game.teams[0].wage_budget = bill;
    let tight = preview(&game);

    assert!(!roomy.blocked_by_wage_policy);
    assert!(
        tight.blocked_by_wage_policy,
        "bill {bill}, incoming {}",
        tight.incoming_player_weekly_wage
    );
    let refusal = make_transfer_bid(&mut game, "player-wage-flag", FEE).err();
    assert!(
        refusal
            .as_deref()
            .is_some_and(|error| error.starts_with("be.error.contracts.boardWagePolicy")),
        "the flagged deal is the one the board refuses once it is agreed: {refusal:?}"
    );
}

fn users_offer(
    id: &str,
    status: TransferOfferStatus,
    round: u8,
    date: &str,
    closed_on: Option<&str>,
) -> TransferOffer {
    TransferOffer {
        id: id.to_string(),
        from_team_id: "team-1".to_string(),
        fee: 1_000_000,
        wage_offered: 0,
        last_manager_fee: Some(1_000_000),
        negotiation_round: round,
        suggested_counter_fee: None,
        status,
        date: date.to_string(),
        registration_date: None,
        closed_on: closed_on.map(str::to_string),
        registration_failure_reason: None,
    }
}

/// The screen's view of the user's negotiation over a player holding one offer from the user's
/// club, on 2026-08-01.
fn negotiation_over(offer: TransferOffer) -> ofm_core::transfers::BidNegotiationStatus {
    let mut player = make_player("player-talks");
    player.transfer_offers.push(offer);
    let game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    ofm_core::transfers::bid_negotiation_status(&game, "player-talks").expect("a status")
}

/// Given the user has a live offer, a refusal from 5 days ago at round 2, a refusal from 40 days
///       ago, and a withdrawn offer
/// When the negotiation the next bid would continue is asked for in each case
/// Then the live offer and the recent refusal continue; the old refusal and the withdrawn offer
///      do not, because the engine would start those over at round 1
#[test]
fn the_status_names_the_negotiation_the_next_bid_continues() {
    let live = negotiation_over(users_offer(
        "live",
        TransferOfferStatus::Pending,
        2,
        "2026-07-30",
        None,
    ));
    let recent = negotiation_over(users_offer(
        "recent",
        TransferOfferStatus::Rejected,
        2,
        "2026-07-20",
        Some("2026-07-27"),
    ));
    let old = negotiation_over(users_offer(
        "old",
        TransferOfferStatus::Rejected,
        3,
        "2026-06-01",
        Some("2026-06-21"),
    ));
    let withdrawn = negotiation_over(users_offer(
        "withdrawn",
        TransferOfferStatus::Withdrawn,
        2,
        "2026-07-20",
        Some("2026-07-27"),
    ));

    assert_eq!(live.offer_id.as_deref(), Some("live"));
    assert!(live.continues_talks && live.talks_resume_on.is_none());
    assert_eq!(recent.offer_id.as_deref(), Some("recent"));
    assert!(recent.continues_talks && recent.talks_resume_on.is_none());
    assert_eq!(old.offer_id, None);
    assert_eq!(withdrawn.offer_id, None);
}

/// Given talks the owner has broken off (a refusal at round 4, closed 3 days ago)
/// When the status is asked for
/// Then the negotiation is named, it does not continue, and it says the day talks can resume
///      (30 days after it closed), the same day the bid itself would be refused until
#[test]
fn broken_off_talks_say_when_they_can_resume() {
    let status = negotiation_over(users_offer(
        "broken",
        TransferOfferStatus::Rejected,
        4,
        "2026-07-20",
        Some("2026-07-29"),
    ));

    assert_eq!(status.offer_id.as_deref(), Some("broken"));
    assert!(!status.continues_talks);
    assert_eq!(status.talks_resume_on.as_deref(), Some("2026-08-28"));
}
