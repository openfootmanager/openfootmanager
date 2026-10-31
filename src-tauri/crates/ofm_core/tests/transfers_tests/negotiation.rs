use super::*;

fn bid_on_a_later_day(game: &mut Game, id: &str, fee: u64) -> TransferNegotiationDecision {
    game.clock.advance_days(1);
    make_transfer_bid(game, id, fee).expect("a bid").decision
}

fn find_offer<'a>(game: &'a Game, player_id: &str, offer_id: &str) -> &'a TransferOffer {
    find_player(game, player_id)
        .transfer_offers
        .iter()
        .find(|offer| offer.id == offer_id)
        .expect("the offer is still on the player")
}

/// Given a bid the owner accepted while the window is shut, so the move waits to be registered
/// When the user bids for the same player again before the window opens
/// Then the bid is refused as not pending, and the player still carries one scheduled deal.
#[test]
fn a_player_with_a_scheduled_transfer_cannot_take_another_bid() {
    let player = make_player("player-scheduled");
    let mut game = make_game_with_player(player, vec![], 5_000_000, 5_000_000);
    game.clock.current_date = Utc.with_ymd_and_hms(2026, 12, 20, 12, 0, 0).unwrap();
    game.season_context.transfer_window.status = TransferWindowStatus::Closed;
    game.season_context.transfer_window.opens_on = Some("2027-01-01".to_string());
    let first = make_transfer_bid(&mut game, "player-scheduled", 2_000_000).expect("first bid");
    assert_eq!(first.decision, TransferNegotiationDecision::Accepted);

    let second = make_transfer_bid(&mut game, "player-scheduled", 2_000_000);

    assert_eq!(
        second.err().as_deref(),
        Some("be.error.transfers.offerNotPending")
    );
    let scheduled = find_player(&game, "player-scheduled")
        .transfer_offers
        .iter()
        .filter(|offer| offer.status == TransferOfferStatus::PendingRegistration)
        .count();
    assert_eq!(scheduled, 1);
}

/// Given a 900,000 bid from team-2 for the user's player valued at 1,000,000, and a buyer whose
///       ceiling for him is 1,200,000
/// When the user asks for 1,250,000, inside the buyer's 8% counter window but over its ceiling
/// Then the buyer counters at its ceiling, and the stored offer is rewritten to say so: the fee,
///      the round, the suggestion and the user's last ask.
#[test]
fn a_counter_inside_the_buyers_window_rewrites_the_offer_at_the_suggested_fee() {
    let mut player = make_user_player("player-counter-window");
    player
        .transfer_offers
        .push(make_pending_incoming_offer("offer-window", 900_000));
    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.teams[1].finance = 6_000_000;
    game.teams[1].transfer_budget = 3_000_000;

    let outcome = counter_offer(
        &mut game,
        "player-counter-window",
        "offer-window",
        1_250_000,
    )
    .expect("the counter is answered");

    assert_eq!(outcome.decision, TransferNegotiationDecision::CounterOffer);
    assert_eq!(outcome.suggested_fee, Some(1_200_000));
    let offer = find_offer(&game, "player-counter-window", "offer-window");
    assert_eq!(offer.status, TransferOfferStatus::Pending);
    assert_eq!(offer.fee, 1_200_000);
    assert_eq!(offer.negotiation_round, 2);
    assert_eq!(offer.suggested_counter_fee, Some(1_200_000));
    assert_eq!(offer.last_manager_fee, Some(1_250_000));
}

/// Given an owner asking 950,000 who countered a 900,000 bid
/// When the user comes back the next day with less, 870,000
/// Then the owner refuses outright: a bid that does not move forward after a counter must reach
///      94% of the asking price (893,000) to be countered again.
#[test]
fn a_lowered_bid_after_a_counter_is_refused_not_countered() {
    let mut game = negotiation_game("player-lowered");
    let first = make_transfer_bid(&mut game, "player-lowered", 900_000).expect("first bid");
    assert_eq!(first.decision, TransferNegotiationDecision::CounterOffer);

    let second = bid_on_a_later_day(&mut game, "player-lowered", 870_000);

    assert_eq!(second, TransferNegotiationDecision::Rejected);
}

/// Given an owner asking 950,000 who refused an 800,000 bid
/// When the user comes back the next day with a real improvement, 860,000
/// Then the owner counters: a second bid that moves forward only has to reach 88% of the asking
///      price (836,000), not the 94% a stalled one needs.
#[test]
fn an_improved_bid_after_a_refusal_is_countered() {
    let mut game = negotiation_game("player-improved");
    let first = make_transfer_bid(&mut game, "player-improved", 800_000).expect("first bid");
    assert_eq!(first.decision, TransferNegotiationDecision::Rejected);

    let second = bid_on_a_later_day(&mut game, "player-improved", 860_000);

    assert_eq!(second, TransferNegotiationDecision::CounterOffer);
}

/// Given an owner asking 950,000 who refused bids of 700,000 and 760,000
/// When the third bid, 820,000, improves again
/// Then the owner concedes only 2% (asking 931,000, so 857,000 to be countered) and refuses it:
///      the concession is a discount, not the whole price.
#[test]
fn a_third_improving_bid_short_of_the_two_percent_concession_is_refused() {
    let mut game = negotiation_game("player-third-short");
    make_transfer_bid(&mut game, "player-third-short", 700_000).expect("first bid");
    bid_on_a_later_day(&mut game, "player-third-short", 760_000);

    let third = bid_on_a_later_day(&mut game, "player-third-short", 820_000);

    assert_eq!(third, TransferNegotiationDecision::Rejected);
    assert_eq!(
        find_player(&game, "player-third-short").team_id.as_deref(),
        Some("team-2")
    );
}

/// Given an owner asking 950,000 who refused bids of 700,000 and 760,000
/// When the third bid, 940,000, improves again and clears the 2% concession (931,000)
/// Then the owner accepts it and the player moves.
#[test]
fn a_third_improving_bid_inside_the_two_percent_concession_is_accepted() {
    let mut game = negotiation_game("player-third-inside");
    make_transfer_bid(&mut game, "player-third-inside", 700_000).expect("first bid");
    bid_on_a_later_day(&mut game, "player-third-inside", 760_000);

    let third = bid_on_a_later_day(&mut game, "player-third-inside", 940_000);

    assert_eq!(third, TransferNegotiationDecision::Accepted);
    assert_eq!(
        find_player(&game, "player-third-inside").team_id.as_deref(),
        Some("team-1")
    );
}

/// Given an owner asking 950,000 who refused 800,000 and then countered 870,000 at 950,000
/// When the third bid, 925,000, comes within 5% of that counter
/// Then the owner meets it: respecting the counter earns a 4% concession (asking 912,000), more
///      than the 2% (931,000) a merely improving third bid earns, so the bid is accepted
#[test]
fn a_third_bid_that_respects_the_counter_earns_the_four_percent_concession() {
    let mut game = negotiation_game("player-respected");
    make_transfer_bid(&mut game, "player-respected", 800_000).expect("first bid");
    let second = bid_on_a_later_day(&mut game, "player-respected", 870_000);
    assert_eq!(second, TransferNegotiationDecision::CounterOffer);

    let third = bid_on_a_later_day(&mut game, "player-respected", 925_000);

    assert_eq!(third, TransferNegotiationDecision::Accepted);
}

/// Given a user club with a starting eleven of its own players
/// When it buys a player who is not in that eleven
/// Then its eleven is exactly as it was: only the seller's side loses a starter.
#[test]
fn buying_a_player_leaves_the_buyers_starting_eleven_alone() {
    let mut player = make_player("player-arriving");
    player.stage_contract_end(Some("2026-08-31".to_string()));
    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    let buyers_eleven: Vec<String> = game
        .players
        .iter()
        .filter(|player| player.team_id.as_deref() == Some("team-1"))
        .take(11)
        .map(|player| player.id.clone())
        .collect();
    assert_eq!(buyers_eleven.len(), 11);
    game.teams[0].starting_xi_ids = buyers_eleven.clone();

    let outcome = make_transfer_bid(&mut game, "player-arriving", 1_000_000).expect("a bid");

    assert_eq!(outcome.decision, TransferNegotiationDecision::Accepted);
    assert_eq!(game.teams[0].starting_xi_ids, buyers_eleven);
}
