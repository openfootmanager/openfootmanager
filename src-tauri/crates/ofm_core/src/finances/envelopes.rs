//! Board wage/transfer envelopes: permission the board grants, not cash.

use crate::game::Game;
use serde::Serialize;
use ts_rs::TS;

const ERR_TEAM_NOT_FOUND: &str = "be.error.managedTeamNotFound";
const ERR_REALLOCATION_INVALID: &str = "be.error.finance.reallocationInvalid";
const ERR_REALLOCATION_RESERVATIONS: &str = "be.error.finance.reallocationBreachesReservations";

/// One weekly wage step the board envelope controls move. Transfer permission
/// moves by 52× this amount so `wage_limit * 52 + transfer_limit` is conserved.
pub const ENVELOPE_WAGE_STEP_WEEKLY: i64 = 1_000;

/// The wage and transfer limits after a reallocation. Cash is unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
pub struct EnvelopeReallocation {
    pub wage_budget: i64,
    pub transfer_budget: i64,
}

/// Whether one envelope step is allowed, and the limits it would leave.
///
/// The Finances tab renders this and decides nothing: the same projection
/// validates `reallocate_board_envelopes`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct EnvelopeReallocationPreview {
    pub wage_delta_weekly: i64,
    pub transfer_delta: i64,
    pub next_wage_budget: i64,
    pub next_transfer_budget: i64,
    pub allowed: bool,
    pub block_reason: Option<String>,
}

/// Projected envelopes after `wage_delta_weekly`, without mutating the club.
pub fn projected_envelope_reallocation(
    game: &Game,
    club_id: &str,
    wage_delta_weekly: i64,
) -> Result<EnvelopeReallocation, String> {
    let team = game
        .teams
        .iter()
        .find(|team| team.id == club_id)
        .ok_or_else(|| ERR_TEAM_NOT_FOUND.to_string())?;
    if wage_delta_weekly == 0 {
        return Ok(EnvelopeReallocation {
            wage_budget: team.wage_budget,
            transfer_budget: team.transfer_budget,
        });
    }
    let transfer_delta = wage_delta_weekly
        .checked_mul(52)
        .ok_or_else(|| ERR_REALLOCATION_INVALID.to_string())?;
    let reserved = crate::transfers::committed_transfer_fees(game, club_id);
    let new_wage = team
        .wage_budget
        .checked_add(wage_delta_weekly)
        .ok_or_else(|| ERR_REALLOCATION_INVALID.to_string())?;
    let new_transfer = team
        .transfer_budget
        .checked_sub(transfer_delta)
        .ok_or_else(|| ERR_REALLOCATION_INVALID.to_string())?;
    if new_wage < 0 || new_transfer < 0 {
        return Err(ERR_REALLOCATION_INVALID.to_string());
    }
    if new_transfer < reserved {
        return Err(ERR_REALLOCATION_RESERVATIONS.to_string());
    }
    Ok(EnvelopeReallocation {
        wage_budget: new_wage,
        transfer_budget: new_transfer,
    })
}

/// The verdict and next limits for one envelope step. Does not mutate.
pub fn envelope_reallocation_preview(
    game: &Game,
    club_id: &str,
    wage_delta_weekly: i64,
) -> EnvelopeReallocationPreview {
    let transfer_delta = wage_delta_weekly.saturating_mul(52);
    let (next_wage_budget, next_transfer_budget) = game
        .teams
        .iter()
        .find(|team| team.id == club_id)
        .map(|team| {
            (
                team.wage_budget.saturating_add(wage_delta_weekly),
                team.transfer_budget.saturating_sub(transfer_delta),
            )
        })
        .unwrap_or((0, 0));
    match projected_envelope_reallocation(game, club_id, wage_delta_weekly) {
        Ok(next) => EnvelopeReallocationPreview {
            wage_delta_weekly,
            transfer_delta: -transfer_delta,
            next_wage_budget: next.wage_budget,
            next_transfer_budget: next.transfer_budget,
            allowed: wage_delta_weekly != 0,
            block_reason: None,
        },
        Err(reason) => EnvelopeReallocationPreview {
            wage_delta_weekly,
            transfer_delta: -transfer_delta,
            next_wage_budget,
            next_transfer_budget,
            allowed: false,
            block_reason: Some(reason),
        },
    }
}

/// Move weekly wage permission by `wage_delta_weekly`. Conserves
/// `wage_limit * 52 + transfer_limit`. Does not post cash.
///
/// A positive delta raises the weekly wage cap and cuts the transfer limit by
/// 52× that amount. The new transfer limit must still cover agreed
/// `PendingRegistration` fees.
pub fn reallocate_board_envelopes(
    game: &mut Game,
    club_id: &str,
    wage_delta_weekly: i64,
) -> Result<EnvelopeReallocation, String> {
    let result = projected_envelope_reallocation(game, club_id, wage_delta_weekly)?;
    if wage_delta_weekly != 0 {
        let team = game
            .teams
            .iter_mut()
            .find(|team| team.id == club_id)
            .ok_or_else(|| ERR_TEAM_NOT_FOUND.to_string())?;
        team.wage_budget = result.wage_budget;
        team.transfer_budget = result.transfer_budget;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::player::{Player, PlayerAttributes, Position, TransferOffer, TransferOfferStatus};
    use domain::team::Team;

    fn attrs() -> PlayerAttributes {
        PlayerAttributes {
            pace: 65,
            stamina: 65,
            strength: 65,
            agility: 65,
            passing: 65,
            shooting: 65,
            tackling: 65,
            dribbling: 65,
            defending: 65,
            positioning: 65,
            vision: 65,
            decisions: 65,
            composure: 65,
            aggression: 50,
            teamwork: 65,
            leadership: 50,
            handling: 20,
            reflexes: 30,
            aerial: 60,
        }
    }

    fn make_game() -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "mgr".to_string(),
            "A".to_string(),
            "B".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("buyer".to_string());
        let mut buyer = Team::new(
            "buyer".to_string(),
            "Buyer".to_string(),
            "BUY".to_string(),
            "England".to_string(),
            "London".to_string(),
            "Ground".to_string(),
            20_000,
        );
        buyer.finance = 5_000_000;
        buyer.wage_budget = 200_000;
        buyer.transfer_budget = 2_000_000;
        Game::new(clock, manager, vec![buyer], vec![], vec![], vec![])
    }

    fn pending_registration(from_team_id: &str, fee: u64) -> TransferOffer {
        TransferOffer {
            id: "offer-1".to_string(),
            from_team_id: from_team_id.to_string(),
            fee,
            wage_offered: 0,
            last_manager_fee: None,
            negotiation_round: 1,
            suggested_counter_fee: None,
            status: TransferOfferStatus::PendingRegistration,
            date: "2026-08-01".to_string(),
            registration_date: None,
            closed_on: None,
            registration_failure_reason: None,
        }
    }

    fn listed_player(offer: TransferOffer) -> Player {
        let mut player = Player::new(
            "p1".to_string(),
            "Sold".to_string(),
            "Sold Player".to_string(),
            "1998-01-01".to_string(),
            "GB".to_string(),
            Position::Midfielder,
            attrs(),
        );
        player.team_id = Some("seller".to_string());
        player.transfer_offers.push(offer);
        player
    }

    /// Given wage and transfer limits, when weekly wage permission is raised,
    /// then the transfer limit falls by 52× that amount and cash is untouched.
    #[test]
    fn reallocation_conserves_season_permission_without_touching_cash() {
        let mut game = make_game();
        let finance = game.teams[0].finance;
        let journal_len = game.cash_journal.len();
        let permission = game.teams[0].wage_budget * 52 + game.teams[0].transfer_budget;

        let result = reallocate_board_envelopes(&mut game, "buyer", 10_000).unwrap();

        assert_eq!(game.teams[0].finance, finance);
        assert_eq!(game.cash_journal.len(), journal_len);
        assert_eq!(game.teams[0].wage_budget, 210_000);
        assert_eq!(game.teams[0].transfer_budget, 2_000_000 - 520_000);
        assert_eq!(result.wage_budget, 210_000);
        assert_eq!(result.transfer_budget, 1_480_000);
        assert_eq!(
            game.teams[0].wage_budget * 52 + game.teams[0].transfer_budget,
            permission
        );
    }

    /// Given permission moved onto wages, when it is moved back, then both limits return.
    #[test]
    fn reallocation_can_return_permission_to_transfers() {
        let mut game = make_game();
        reallocate_board_envelopes(&mut game, "buyer", 10_000).unwrap();
        reallocate_board_envelopes(&mut game, "buyer", -10_000).unwrap();
        assert_eq!(game.teams[0].wage_budget, 200_000);
        assert_eq!(game.teams[0].transfer_budget, 2_000_000);
    }

    /// Given an agreed purchase sitting against the transfer limit, when a wage
    /// raise would leave less permission than that fee, then the move is refused
    /// and the envelopes stay put.
    #[test]
    fn reallocation_rejects_a_cut_that_would_breach_reservations() {
        let mut game = make_game();
        game.players
            .push(listed_player(pending_registration("buyer", 1_800_000)));
        let wage = game.teams[0].wage_budget;
        let transfer = game.teams[0].transfer_budget;

        let err = reallocate_board_envelopes(&mut game, "buyer", 10_000).unwrap_err();

        assert_eq!(err, ERR_REALLOCATION_RESERVATIONS);
        assert_eq!(game.teams[0].wage_budget, wage);
        assert_eq!(game.teams[0].transfer_budget, transfer);
        assert_eq!(game.teams[0].finance, 5_000_000);
    }

    /// Given a live talk that is not yet agreed, when envelopes are moved, then
    /// that pending fee does not count as reserved.
    #[test]
    fn a_pending_talk_does_not_block_reallocation() {
        let mut game = make_game();
        let mut offer = pending_registration("buyer", 1_800_000);
        offer.status = TransferOfferStatus::Pending;
        game.players.push(listed_player(offer));

        reallocate_board_envelopes(&mut game, "buyer", 10_000).unwrap();

        assert_eq!(game.teams[0].wage_budget, 210_000);
        assert_eq!(game.teams[0].transfer_budget, 1_480_000);
    }

    /// Given a transfer limit smaller than 52× the wage step, when the wage cap
    /// is raised, then the move is refused as invalid.
    #[test]
    fn reallocation_rejects_a_move_that_would_take_a_limit_below_zero() {
        let mut game = make_game();
        game.teams[0].transfer_budget = 500_000;
        let err = reallocate_board_envelopes(&mut game, "buyer", 10_000).unwrap_err();
        assert_eq!(err, ERR_REALLOCATION_INVALID);
        assert_eq!(game.teams[0].transfer_budget, 500_000);
        assert_eq!(game.teams[0].wage_budget, 200_000);
    }

    /// Given a wage cut larger than the wage cap, when envelopes are moved, then
    /// the wage limit is not allowed to go negative.
    #[test]
    fn reallocation_rejects_a_wage_cut_below_zero() {
        let mut game = make_game();
        game.teams[0].wage_budget = 5_000;
        let err = reallocate_board_envelopes(&mut game, "buyer", -10_000).unwrap_err();
        assert_eq!(err, ERR_REALLOCATION_INVALID);
        assert_eq!(game.teams[0].wage_budget, 5_000);
    }

    /// Given a zero delta, when envelopes are reallocated, then the limits are returned unchanged.
    #[test]
    fn reallocation_of_zero_is_a_no_op() {
        let mut game = make_game();
        let result = reallocate_board_envelopes(&mut game, "buyer", 0).unwrap();
        assert_eq!(result.wage_budget, 200_000);
        assert_eq!(result.transfer_budget, 2_000_000);
    }

    /// Given an unknown club, when envelopes are reallocated, then the managed-team key is returned.
    #[test]
    fn reallocation_reports_an_unknown_club() {
        let mut game = make_game();
        let err = reallocate_board_envelopes(&mut game, "nobody", 1_000).unwrap_err();
        assert_eq!(err, ERR_TEAM_NOT_FOUND);
    }

    /// Given reserved fees a one-step wage raise would uncover
    /// When the reallocation preview is read
    /// Then raise-wage is refused with the reservations key and the club is unchanged.
    #[test]
    fn preview_refuses_a_wage_raise_that_would_breach_reservations() {
        let mut game = make_game();
        game.teams[0].transfer_budget = 300_000;
        game.players
            .push(listed_player(pending_registration("buyer", 300_000)));
        let wage = game.teams[0].wage_budget;
        let transfer = game.teams[0].transfer_budget;

        let preview = envelope_reallocation_preview(&game, "buyer", ENVELOPE_WAGE_STEP_WEEKLY);

        assert!(!preview.allowed);
        assert_eq!(
            preview.block_reason.as_deref(),
            Some(ERR_REALLOCATION_RESERVATIONS)
        );
        assert_eq!(preview.wage_delta_weekly, ENVELOPE_WAGE_STEP_WEEKLY);
        assert_eq!(preview.transfer_delta, -ENVELOPE_WAGE_STEP_WEEKLY * 52);
        assert_eq!(preview.next_wage_budget, wage + ENVELOPE_WAGE_STEP_WEEKLY);
        assert_eq!(
            preview.next_transfer_budget,
            transfer - ENVELOPE_WAGE_STEP_WEEKLY * 52
        );
        assert_eq!(game.teams[0].wage_budget, wage);
        assert_eq!(game.teams[0].transfer_budget, transfer);
        assert_eq!(game.teams[0].finance, 5_000_000);
    }

    /// Given a wage cap smaller than one step
    /// When the transfer-raise preview is read
    /// Then it is refused as invalid and the envelopes stay put.
    #[test]
    fn preview_refuses_a_transfer_raise_that_would_take_wages_below_zero() {
        let mut game = make_game();
        game.teams[0].wage_budget = 500;

        let preview = envelope_reallocation_preview(&game, "buyer", -ENVELOPE_WAGE_STEP_WEEKLY);

        assert!(!preview.allowed);
        assert_eq!(
            preview.block_reason.as_deref(),
            Some(ERR_REALLOCATION_INVALID)
        );
        assert_eq!(game.teams[0].wage_budget, 500);
    }
}
