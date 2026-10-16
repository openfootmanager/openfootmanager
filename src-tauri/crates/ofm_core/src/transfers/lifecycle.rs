//! When an offer stops being live, and what is recorded when it does.
//!
//! Expiry, rejection and withdrawal all end an offer, and each has to leave the same two
//! facts behind: the arrival date it kept, and the closure date it earned. Keeping the
//! closing helpers together with the staleness rules and the retention prune is what stops a
//! new terminal path being written that forgets one of them.

use super::*;

/// Clubs currently holding a live approach for `player`, counting both deal types.
///
/// One club is one approach, so the size of this set is also how many offers the player is
/// fielding — which is what [`MAX_PENDING_INCOMING_OFFERS_PER_USER_PLAYER`] bounds.
pub(crate) fn pending_approach_clubs(player: &domain::player::Player) -> HashSet<String> {
    player
        .transfer_offers
        .iter()
        .filter(|offer| offer.status == TransferOfferStatus::Pending)
        .map(|offer| offer.from_team_id.clone())
        .chain(
            player
                .loan_offers
                .iter()
                .filter(|offer| offer.status == LoanOfferStatus::Pending)
                .map(|offer| offer.from_team_id.clone()),
        )
        .collect()
}
/// Whether an offer closed (or, for one written before `closed_on` existed, arrived) within
/// [`REBID_COOLDOWN_DAYS`] of `today`. The one statement of that window: the incoming sweep, an
/// outgoing permanent bid and an outgoing loan bid all ask it.
pub(crate) fn within_rebid_cooldown(closed_on: Option<&str>, date: &str, today: NaiveDate) -> bool {
    NaiveDate::parse_from_str(closed_on.unwrap_or(date), "%Y-%m-%d")
        .is_ok_and(|day| (today - day).num_days() < REBID_COOLDOWN_DAYS)
}

/// Whether talks that ended on `stamp` were with the club that owns the player now.
///
/// An offer has no owner field, so a refusal is placed by date: one from before the player's last
/// arrival at his club was said by a previous owner, and the new one has not refused anybody.
pub(crate) fn heard_by_current_owner(player: &domain::player::Player, stamp: &str) -> bool {
    let day = |text: &str| NaiveDate::parse_from_str(text, "%Y-%m-%d").ok();
    match player.joined_current_club_on().and_then(day) {
        Some(joined) => day(stamp).is_none_or(|ended| ended >= joined),
        None => true,
    }
}

/// Whether a loan approach was made to the club that owns the player now. A loan offer names the
/// parent club it was made to, so no date is needed.
pub(crate) fn loan_made_to_current_owner(
    player: &domain::player::Player,
    offer: &domain::player::LoanOffer,
) -> bool {
    player.contract_club_id() == Some(offer.parent_team_id.as_str())
}

/// Clubs that were turned away recently enough that they should not be asking again.
///
/// Being told no is information a club acts on. Without this, the day after a rejection the club
/// is eligible again — it re-enters the shortlist, opens a fresh approach, and the manager rejects
/// the same suitor every day for the length of the window. Expiry counts too: talks that went cold
/// on their own are no more of an invitation to start over than a refusal is.
///
/// Read from the offers already on the player, so this costs no new saved state. `closed_on` is
/// when talks ended; offers written before that field existed fall back to their arrival date.
pub(crate) fn clubs_in_rebid_cooldown(
    player: &domain::player::Player,
    current_date: NaiveDate,
) -> HashSet<String> {
    player
        .transfer_offers
        .iter()
        .filter(|offer| {
            matches!(
                offer.status,
                TransferOfferStatus::Rejected | TransferOfferStatus::Withdrawn
            ) && within_rebid_cooldown(offer.closed_on.as_deref(), &offer.date, current_date)
                && heard_by_current_owner(player, offer.closed_on.as_deref().unwrap_or(&offer.date))
        })
        .map(|offer| offer.from_team_id.clone())
        .chain(
            player
                .loan_offers
                .iter()
                .filter(|offer| {
                    matches!(
                        offer.status,
                        LoanOfferStatus::Rejected | LoanOfferStatus::Withdrawn
                    ) && within_rebid_cooldown(
                        offer.closed_on.as_deref(),
                        &offer.date,
                        current_date,
                    ) && loan_made_to_current_owner(player, offer)
                })
                .map(|offer| offer.from_team_id.clone()),
        )
        .collect()
}

/// Whether `club_id` may open talks for a player already approached by `clubs`.
pub(crate) fn club_may_approach(clubs: &HashSet<String>, club_id: &str) -> bool {
    clubs.len() < MAX_PENDING_INCOMING_OFFERS_PER_USER_PLAYER && !clubs.contains(club_id)
}

/// What today's sweep has already sent to the user's squad, and who is not welcome to add to it.
///
/// The three limits answer different questions. `new_today` resets every day and throttles
/// arrivals; `approach_clubs` is the standing queue and bounds how many offers face the manager at
/// once; `cooled_clubs` is memory of refusals, and is deliberately *not* counted against the queue
/// — a club sitting out its cooldown should not also occupy a slot someone else could use.
pub(crate) struct IncomingOfferBudget<'a> {
    pub(crate) new_today: &'a std::collections::HashMap<String, usize>,
    pub(crate) approach_clubs: &'a std::collections::HashMap<String, HashSet<String>>,
    pub(crate) cooled_clubs: &'a std::collections::HashMap<String, HashSet<String>>,
}

impl IncomingOfferBudget<'_> {
    pub(crate) fn accepts(&self, player_id: &str, club_id: &str, per_day_limit: usize) -> bool {
        self.new_today.get(player_id).copied().unwrap_or(0) < per_day_limit
            && self
                .cooled_clubs
                .get(player_id)
                .is_none_or(|clubs| !clubs.contains(club_id))
            && self
                .approach_clubs
                .get(player_id)
                .is_none_or(|clubs| club_may_approach(clubs, club_id))
    }
}
pub(crate) fn offer_is_stale(
    current_date: NaiveDate,
    offer: &domain::player::TransferOffer,
) -> bool {
    if offer.status != TransferOfferStatus::Pending {
        return false;
    }

    let Ok(offer_date) = NaiveDate::parse_from_str(&offer.date, "%Y-%m-%d") else {
        return false;
    };

    (current_date - offer_date).num_days() >= TRANSFER_NEGOTIATION_STALE_DAYS
}
pub(crate) fn loan_offer_is_stale(
    current_date: NaiveDate,
    offer: &domain::player::LoanOffer,
) -> bool {
    if offer.status != LoanOfferStatus::Pending {
        return false;
    }

    let Ok(offer_date) = NaiveDate::parse_from_str(&offer.date, "%Y-%m-%d") else {
        return false;
    };

    (current_date - offer_date).num_days() >= TRANSFER_NEGOTIATION_STALE_DAYS
}
/// Moves a transfer offer to a terminal status and records when talks ended.
///
/// Every path that rejects or withdraws an offer goes through here so `closed_on` cannot be
/// forgotten at one of them — `date` is the arrival date and is rewritten whenever a club
/// re-opens talks, so it can never answer "when did this close".
pub(crate) fn close_transfer_offer(
    offer: &mut domain::player::TransferOffer,
    status: TransferOfferStatus,
    today: &str,
) {
    offer.status = status;
    offer.suggested_counter_fee = None;
    offer.registration_failure_reason = None;
    offer.closed_on = Some(today.to_string());
}
/// Loan counterpart of [`close_transfer_offer`].
pub(crate) fn close_loan_offer(
    offer: &mut domain::player::LoanOffer,
    status: LoanOfferStatus,
    today: &str,
) {
    offer.status = status;
    offer.closed_on = Some(today.to_string());
}
pub(crate) fn expire_stale_transfer_offers(game: &mut Game) {
    let current_date = game.clock.current_date.date_naive();
    let today = current_date.format("%Y-%m-%d").to_string();

    for player in &mut game.players {
        for offer in &mut player.transfer_offers {
            if offer_is_stale(current_date, offer) {
                close_transfer_offer(offer, TransferOfferStatus::Withdrawn, &today);
            }
        }
    }
}
pub(crate) fn withdraw_pending_transfer_offers(player: &mut domain::player::Player, today: &str) {
    for offer in &mut player.transfer_offers {
        if offer.status == TransferOfferStatus::Pending {
            close_transfer_offer(offer, TransferOfferStatus::Withdrawn, today);
        }
    }
}
pub(crate) fn expire_stale_loan_offers(game: &mut Game) {
    let current_date = game.clock.current_date.date_naive();
    let today = current_date.format("%Y-%m-%d").to_string();

    for player in &mut game.players {
        for offer in &mut player.loan_offers {
            if loan_offer_is_stale(current_date, offer) {
                close_loan_offer(offer, LoanOfferStatus::Withdrawn, &today);
            }
        }
    }
}
/// Drops rejected and withdrawn offers once they age out of the retention window.
///
/// Terminal offers were never removed, so they accumulated on the player for the life of the
/// save — both bloating the linear scans every lookup does and leaving the UI with an unbounded
/// history list. Offers written before `closed_on` existed fall back to their arrival date, so
/// an existing save's backlog drains too.
pub(crate) fn prune_closed_offers(game: &mut Game) {
    let current_date = game.clock.current_date.date_naive();

    let aged_out = |closed_on: Option<&str>, date: &str| {
        let stamp = closed_on.unwrap_or(date);
        NaiveDate::parse_from_str(stamp, "%Y-%m-%d")
            .is_ok_and(|day| (current_date - day).num_days() >= CLOSED_OFFER_RETENTION_DAYS)
    };

    for player in &mut game.players {
        player.transfer_offers.retain(|offer| {
            !matches!(
                offer.status,
                TransferOfferStatus::Rejected | TransferOfferStatus::Withdrawn
            ) || !aged_out(offer.closed_on.as_deref(), &offer.date)
        });
        player.loan_offers.retain(|offer| {
            !matches!(
                offer.status,
                LoanOfferStatus::Rejected | LoanOfferStatus::Withdrawn
            ) || !aged_out(offer.closed_on.as_deref(), &offer.date)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transfers::tests::sample_attributes;
    use domain::player::{
        LoanOffer, Player, PlayerMovementEntry, PlayerMovementKind, Position, TransferOffer,
    };

    fn day(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap()
    }

    /// A player at "new-club" whom "club-x" was turned away from on `refused_on`, and who arrived
    /// at "new-club" on `arrived_on`.
    fn player_refused_then_moved(refused_on: &str, arrived_on: &str) -> Player {
        let mut player = Player::new(
            "p1".into(),
            "P One".into(),
            "Player One".into(),
            "1995-01-01".into(),
            "England".into(),
            Position::Forward,
            sample_attributes(),
        );
        player.team_id = Some("new-club".into());
        player.transfer_offers.push(TransferOffer {
            id: "offer-1".into(),
            from_team_id: "club-x".into(),
            fee: 1_000_000,
            wage_offered: 0,
            last_manager_fee: None,
            negotiation_round: 4,
            suggested_counter_fee: None,
            status: TransferOfferStatus::Rejected,
            date: refused_on.into(),
            registration_date: None,
            registration_failure_reason: None,
            closed_on: Some(refused_on.into()),
        });
        player
            .record_movement(PlayerMovementEntry {
                from_team_id: Some("old-club".into()),
                to_team_id: Some("new-club".into()),
                ..PlayerMovementEntry::new(arrived_on, PlayerMovementKind::PermanentTransfer)
            })
            .unwrap();
        player
    }

    /// Given a club the previous owner turned away, and the player since sold to another club
    /// When the clubs still in a rebid cooldown are listed
    /// Then that club is not among them: the new owner has refused nobody.
    #[test]
    fn a_refusal_by_the_previous_owner_does_not_hold_a_club_in_cooldown() {
        let player = player_refused_then_moved("2026-08-05", "2026-08-10");

        assert!(clubs_in_rebid_cooldown(&player, day("2026-08-12")).is_empty());
    }

    /// Given the same refusal, but made after the player arrived at his club
    /// When the clubs still in a rebid cooldown are listed
    /// Then the club is held: the current owner said no.
    #[test]
    fn a_refusal_by_the_current_owner_holds_a_club_in_cooldown() {
        let player = player_refused_then_moved("2026-08-11", "2026-08-10");

        let cooling = clubs_in_rebid_cooldown(&player, day("2026-08-12"));

        assert_eq!(cooling, HashSet::from(["club-x".to_string()]));
    }

    /// Given a refusal stamped on the very day the player arrived
    /// When the clubs still in a rebid cooldown are listed
    /// Then it counts as the new owner's: the day of the sale is the new owner's day.
    #[test]
    fn a_refusal_on_the_day_of_arrival_counts_as_the_new_owners() {
        let player = player_refused_then_moved("2026-08-10", "2026-08-10");

        let cooling = clubs_in_rebid_cooldown(&player, day("2026-08-12"));

        assert_eq!(cooling, HashSet::from(["club-x".to_string()]));
    }

    /// A player at "new-club" whom "club-x" asked to borrow from `parent`, and was turned away.
    fn player_refused_a_loan_approach(parent: &str) -> Player {
        let mut player = player_refused_then_moved("2026-08-05", "2026-08-10");
        player.transfer_offers.clear();
        player.loan_offers.push(LoanOffer {
            id: "loan-1".into(),
            from_team_id: "club-x".into(),
            parent_team_id: parent.into(),
            start_date: "2026-09-01".into(),
            end_date: "2027-06-30".into(),
            wage_contribution_pct: 50,
            buy_option_fee: None,
            last_manager_wage_contribution_pct: None,
            last_manager_end_date: None,
            last_manager_buy_option_fee: None,
            negotiation_round: 4,
            suggested_wage_contribution_pct: None,
            suggested_end_date: None,
            suggested_buy_option_fee: None,
            status: LoanOfferStatus::Rejected,
            date: "2026-08-11".into(),
            closed_on: Some("2026-08-11".into()),
        });
        player
    }

    /// Given a loan approach the previous owner ("old-club") turned down, and the player since sold
    /// to "new-club"
    /// When the clubs still in a rebid cooldown are listed
    /// Then the club is not among them: the approach was made to someone else.
    #[test]
    fn a_loan_refusal_by_the_previous_owner_does_not_hold_a_club_in_cooldown() {
        let player = player_refused_a_loan_approach("old-club");

        assert!(clubs_in_rebid_cooldown(&player, day("2026-08-12")).is_empty());
    }

    /// Given the same refusal, made to the club that owns the player now
    /// When the clubs still in a rebid cooldown are listed
    /// Then the club is held.
    #[test]
    fn a_loan_refusal_by_the_current_owner_holds_a_club_in_cooldown() {
        let player = player_refused_a_loan_approach("new-club");

        let cooling = clubs_in_rebid_cooldown(&player, day("2026-08-12"));

        assert_eq!(cooling, HashSet::from(["club-x".to_string()]));
    }
}
