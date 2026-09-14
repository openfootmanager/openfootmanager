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
    let cooling = |closed_on: Option<&str>, date: &str| {
        NaiveDate::parse_from_str(closed_on.unwrap_or(date), "%Y-%m-%d")
            .is_ok_and(|day| (current_date - day).num_days() < REBID_COOLDOWN_DAYS)
    };

    player
        .transfer_offers
        .iter()
        .filter(|offer| {
            matches!(
                offer.status,
                TransferOfferStatus::Rejected | TransferOfferStatus::Withdrawn
            ) && cooling(offer.closed_on.as_deref(), &offer.date)
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
                    ) && cooling(offer.closed_on.as_deref(), &offer.date)
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
