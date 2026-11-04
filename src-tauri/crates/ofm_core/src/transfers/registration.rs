//! Transfer-window state and the registration gates that depend on it.
//!
//! A deal agreed while the window is shut is not a deal yet — it is an agreement waiting for
//! a registration date. These predicates are what the rest of the module asks before treating
//! a player as available.

use super::*;

pub(crate) fn has_pending_loan_registration(player: &domain::player::Player) -> bool {
    player
        .loan_offers
        .iter()
        .any(|offer| offer.status == LoanOfferStatus::PendingRegistration)
}
pub(crate) fn has_pending_transfer_registration(player: &domain::player::Player) -> bool {
    player
        .transfer_offers
        .iter()
        .any(|offer| offer.status == TransferOfferStatus::PendingRegistration)
}
pub(crate) fn player_has_active_or_pending_loan(player: &domain::player::Player) -> bool {
    player.active_loan.is_some() || has_pending_loan_registration(player)
}
/// Which of a player's two lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Listing {
    Transfer,
    Loan,
}

/// Puts one of the user's players on a list, or takes him off it.
///
/// Only a player at the user's club: a listing lowers the fee his club accepts, and an agent once
/// listed a rival club's star, then bought him a third cheaper (#1206). A player out on, or due to
/// go on, a loan cannot be put on either list, but taking him off one is always allowed, so a
/// listing made before a loan never sticks.
pub fn toggle_listing(game: &mut Game, player_id: &str, listing: Listing) -> Result<(), String> {
    let user_team_id = game
        .manager
        .team_id
        .clone()
        .ok_or_else(|| "be.error.noTeamAssigned".to_string())?;
    let player = game
        .players
        .iter_mut()
        .find(|player| player.id == player_id)
        .ok_or_else(|| "be.error.playerNotFound".to_string())?;
    if player.team_id.as_deref() != Some(user_team_id.as_str()) {
        return Err(ERR_PLAYER_NOT_OWNED_BY_USER.to_string());
    }
    let putting_on = !match listing {
        Listing::Transfer => player.transfer_listed,
        Listing::Loan => player.loan_listed,
    };
    if putting_on && player_has_active_or_pending_loan(player) {
        return Err(ERR_PLAYER_ALREADY_LOANED.to_string());
    }
    match listing {
        Listing::Transfer => player.transfer_listed = putting_on,
        Listing::Loan => player.loan_listed = putting_on,
    }
    Ok(())
}

pub(crate) fn player_has_pending_registration(player: &domain::player::Player) -> bool {
    player_has_active_or_pending_loan(player) || has_pending_transfer_registration(player)
}
/// Refuses a loan for a player who already has a move in hand, saying which: a sale agreed and
/// waiting for the window, or a loan. The one rule the three loan paths share (#1205).
pub(super) fn ensure_no_move_in_hand(player: &domain::player::Player) -> Result<(), String> {
    if has_pending_transfer_registration(player) {
        return Err(ERR_PLAYER_ALREADY_SOLD.into());
    }
    if player_has_active_or_pending_loan(player) {
        return Err(ERR_PLAYER_ALREADY_LOANED.into());
    }
    Ok(())
}
pub(crate) fn transfer_window_is_open(game: &Game) -> bool {
    matches!(
        game.season_context.transfer_window.status,
        TransferWindowStatus::Open | TransferWindowStatus::DeadlineDay
    )
}
pub(crate) fn transfer_registration_date(game: &Game) -> Result<NaiveDate, String> {
    let current_date = game.clock.current_date.date_naive();
    if transfer_window_is_open(game) {
        return Ok(current_date);
    }

    let opens_on = game
        .season_context
        .transfer_window
        .opens_on
        .as_deref()
        .ok_or(ERR_TRANSFER_WINDOW_CLOSED)?;
    let registration_date = NaiveDate::parse_from_str(opens_on, "%Y-%m-%d")
        .map_err(|_| ERR_TRANSFER_WINDOW_CLOSED.to_string())?;

    if registration_date <= current_date {
        return Err(ERR_TRANSFER_WINDOW_CLOSED.to_string());
    }

    Ok(registration_date)
}
pub(crate) fn loan_registration_date(game: &Game) -> Result<NaiveDate, String> {
    transfer_registration_date(game)
}
