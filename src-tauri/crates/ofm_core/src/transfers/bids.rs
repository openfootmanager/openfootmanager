//! The transfer-bid path: submitting, countering, responding to and pricing
//! transfer offers, plus the incoming-interest/feedback helpers that drive them.
//! Byte-faithful extraction from the original transfers.rs; the four public
//! entry points stay resolvable as `ofm_core::transfers::*`.

use super::*;

pub(crate) fn incoming_interest_score(
    current_date: NaiveDate,
    player: &domain::player::Player,
) -> i32 {
    let mut score = 0;

    if player.transfer_listed {
        score += 30;
    }

    if let Some(days_remaining) =
        contract_days_remaining(current_date, player.contract_end.as_deref())
    {
        if days_remaining <= 60 {
            score += 40;
        } else if days_remaining <= 180 {
            score += 25;
        } else if days_remaining <= 365 {
            score += 10;
        }
    }

    if player.market_value >= 1_000_000 {
        score += 20;
    } else if player.market_value >= 500_000 {
        score += 10;
    }

    if player.morale <= 45 {
        score += 10;
    }

    score
}

pub(crate) fn suggested_incoming_fee(
    current_date: NaiveDate,
    player: &domain::player::Player,
) -> u64 {
    let mut multiplier: f64 = if player.transfer_listed { 0.9 } else { 1.0 };

    if let Some(days_remaining) =
        contract_days_remaining(current_date, player.contract_end.as_deref())
    {
        if days_remaining <= 60 {
            multiplier -= 0.15;
        } else if days_remaining <= 180 {
            multiplier -= 0.1;
        }
    }

    if player.morale <= 45 {
        multiplier -= 0.05;
    }

    let multiplier = multiplier.clamp(0.7, 1.05);
    ((player.market_value as f64) * multiplier).round() as u64
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

pub(crate) fn expire_stale_transfer_offers(game: &mut Game) {
    let current_date = game.clock.current_date.date_naive();

    for player in &mut game.players {
        for offer in &mut player.transfer_offers {
            if offer_is_stale(current_date, offer) {
                offer.status = TransferOfferStatus::Withdrawn;
                offer.suggested_counter_fee = None;
            }
        }
    }
}

pub(crate) fn find_open_offer_from_club<'a>(
    player: &'a domain::player::Player,
    club_id: &str,
) -> Option<&'a domain::player::TransferOffer> {
    player
        .transfer_offers
        .iter()
        .find(|offer| offer.from_team_id == club_id && offer.status == TransferOfferStatus::Pending)
}

pub(crate) fn negotiation_round_from_offer(offer: Option<&domain::player::TransferOffer>) -> u8 {
    offer
        .map(|offer| offer.negotiation_round.max(1).saturating_add(1))
        .unwrap_or(1)
}

pub(crate) fn transfer_negotiation_metrics(
    round: u8,
    stalled: bool,
    respected_signal: bool,
) -> (u8, u8) {
    let mut tension = 34_i16 + (i16::from(round.saturating_sub(1)) * 16);
    let mut patience = 82_i16 - (i16::from(round.saturating_sub(1)) * 18);

    if stalled {
        tension += 12;
        patience -= 12;
    }

    if respected_signal {
        tension -= 8;
        patience += 8;
    }

    (tension.clamp(20, 90) as u8, patience.clamp(18, 86) as u8)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn upsert_transfer_offer(
    player: &mut domain::player::Player,
    from_team_id: &str,
    fee: u64,
    status: TransferOfferStatus,
    date: &str,
    last_manager_fee: Option<u64>,
    negotiation_round: u8,
    suggested_counter_fee: Option<u64>,
    registration_date: Option<String>,
) -> String {
    if let Some(offer) = player.transfer_offers.iter_mut().find(|offer| {
        offer.from_team_id == from_team_id && offer.status == TransferOfferStatus::Pending
    }) {
        offer.fee = fee;
        offer.status = status;
        offer.date = date.to_string();
        offer.last_manager_fee = last_manager_fee;
        offer.negotiation_round = negotiation_round;
        offer.suggested_counter_fee = suggested_counter_fee;
        offer.registration_date = registration_date;
        return offer.id.clone();
    }

    let offer_id = Uuid::new_v4().to_string();
    player.transfer_offers.push(domain::player::TransferOffer {
        id: offer_id.clone(),
        from_team_id: from_team_id.to_string(),
        fee,
        wage_offered: 0,
        last_manager_fee,
        negotiation_round,
        suggested_counter_fee,
        status,
        date: date.to_string(),
        registration_date,
    });
    offer_id
}

pub(crate) fn create_incoming_user_offer(
    game: &mut Game,
    candidate: &MarketCandidate,
    buyer_id: &str,
    buyer_name: &str,
    today: &str,
) {
    let (player_name, interested_clubs) = {
        let Some(player) = game
            .players
            .iter_mut()
            .find(|player| player.id == candidate.player_id)
        else {
            return;
        };

        player.transfer_offers.push(domain::player::TransferOffer {
            id: Uuid::new_v4().to_string(),
            from_team_id: buyer_id.to_string(),
            fee: candidate.fee,
            wage_offered: 0,
            last_manager_fee: None,
            negotiation_round: 1,
            suggested_counter_fee: None,
            status: TransferOfferStatus::Pending,
            date: today.to_string(),
            registration_date: None,
        });

        // Distinct clubs currently holding a live bid — the figure the digest
        // reports ("N clubs interested").
        let interested_clubs = player
            .transfer_offers
            .iter()
            .filter(|offer| offer.status == TransferOfferStatus::Pending)
            .map(|offer| offer.from_team_id.clone())
            .collect::<HashSet<String>>()
            .len();

        (player.full_name.clone(), interested_clubs)
    };

    // One updating thread per player rather than a fresh message per club, so
    // repeat interest never floods the inbox.
    let message = crate::messages::transfer_interest_digest_message(
        &candidate.player_id,
        &player_name,
        interested_clubs,
        buyer_name,
        candidate.fee,
        today,
    );
    if let Some(existing) = game
        .messages
        .iter_mut()
        .find(|existing| existing.id == message.id)
    {
        *existing = message;
    } else {
        game.messages.push(message);
    }
}

pub(crate) fn buyer_counter_offer_ceiling(
    current_date: NaiveDate,
    player: &domain::player::Player,
    current_offer_fee: u64,
    buyer_team: &domain::team::Team,
) -> u64 {
    let baseline_fee = suggested_incoming_fee(current_date, player).max(current_offer_fee);
    let ceiling = ((baseline_fee as f64) * 1.2).round() as u64;
    ceiling
        .min(buyer_team.transfer_budget.max(0) as u64)
        .min(buyer_team.finance.max(0) as u64)
}

pub(crate) fn transfer_outcome(
    decision: TransferNegotiationDecision,
    suggested_fee: Option<u64>,
    is_terminal: bool,
    registration_date: Option<String>,
    feedback: NegotiationFeedback,
) -> TransferNegotiationOutcome {
    TransferNegotiationOutcome {
        decision,
        suggested_fee,
        is_terminal,
        registration_date,
        feedback,
    }
}

pub fn project_transfer_bid_financial_impact(
    game: &Game,
    player_id: &str,
    fee: u64,
) -> Result<TransferBidFinancialProjection, String> {
    let user_team_id = game
        .manager
        .team_id
        .clone()
        .ok_or_else(|| "be.error.noTeamAssigned".to_string())?;

    let player = game
        .players
        .iter()
        .find(|player| player.id == player_id)
        .ok_or_else(|| "be.error.playerNotFound".to_string())?;

    if player.team_id.as_deref() == Some(user_team_id.as_str()) {
        return Err(ERR_CANNOT_BID_ON_OWN_PLAYER.to_string());
    }

    if player_has_pending_registration(player) {
        return Err(ERR_PLAYER_ALREADY_LOANED.to_string());
    }

    let team = game
        .teams
        .iter()
        .find(|team| team.id == user_team_id)
        .ok_or_else(|| "be.error.managedTeamNotFound".to_string())?;

    let annual_wage_bill_before = calc_annual_wages(game, &team.id);
    let annual_wage_bill_after = annual_wage_bill_before + player.wage as i64;
    let projected_wage_budget_usage_pct = if team.wage_budget > 0 {
        ((annual_wage_bill_after as f64 / team.wage_budget as f64) * 100.0).round() as i64
    } else {
        0
    };

    let transfer_budget_after = team.transfer_budget - fee as i64;
    let finance_after = team.finance - fee as i64;

    Ok(TransferBidFinancialProjection {
        transfer_budget_before: team.transfer_budget,
        transfer_budget_after,
        finance_before: team.finance,
        finance_after,
        annual_wage_bill_before,
        annual_wage_bill_after,
        annual_wage_budget: team.wage_budget,
        projected_wage_budget_usage_pct,
        exceeds_transfer_budget: transfer_budget_after < 0,
        exceeds_finance: finance_after < 0,
    })
}

/// Submit a transfer bid from user's team for a player.
/// The AI evaluates the bid and can accept, reject, or counter based on club context.
pub fn make_transfer_bid(
    game: &mut Game,
    player_id: &str,
    fee: u64,
) -> Result<TransferNegotiationOutcome, String> {
    expire_stale_transfer_offers(game);

    let current_date = game.clock.current_date.date_naive();
    let registration_date = transfer_registration_date(game)?;
    let register_immediately = registration_date == current_date;
    let registration_date_string = registration_date.format("%Y-%m-%d").to_string();

    let user_team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("be.error.noTeamAssigned")?;

    let player = game
        .players
        .iter()
        .find(|p| p.id == player_id)
        .ok_or("be.error.playerNotFound")?;

    if player.team_id.as_deref() == Some(&user_team_id) {
        return Err(ERR_CANNOT_BID_ON_OWN_PLAYER.into());
    }

    if player_has_active_or_pending_loan(player) {
        return Err(ERR_PLAYER_ALREADY_LOANED.into());
    }

    if has_pending_transfer_registration(player) {
        return Err(ERR_OFFER_NOT_PENDING.into());
    }

    let owner_team_id = player.team_id.clone().ok_or(ERR_PLAYER_HAS_NO_TEAM)?;

    let my_team = game
        .teams
        .iter()
        .find(|t| t.id == user_team_id)
        .ok_or("be.error.managedTeamNotFound")?;

    let fee_i64 = i64::try_from(fee).map_err(|_| ERR_INSUFFICIENT_FUNDS.to_string())?;

    if my_team.finance < fee_i64 {
        return Err(ERR_INSUFFICIENT_FUNDS.into());
    }

    if my_team.transfer_budget < fee_i64 {
        return Err(ERR_TRANSFER_BUDGET_TOO_LOW.into());
    }

    let owner_team = game
        .teams
        .iter()
        .find(|t| t.id == owner_team_id)
        .ok_or("be.error.teamNotFound")?;

    let buyer_team = my_team;

    let threshold = minimum_acceptable_fee(current_date, player, owner_team, buyer_team);
    let date = game.clock.current_date.format("%Y-%m-%d").to_string();
    let existing_offer = find_open_offer_from_club(player, &user_team_id);
    let previous_fee = existing_offer.map(|offer| offer.fee);
    let previous_counter_fee = existing_offer.and_then(|offer| offer.suggested_counter_fee);
    let round = negotiation_round_from_offer(existing_offer);
    let respected_signal = previous_counter_fee
        .map(|counter| fee >= counter.saturating_mul(95) / 100)
        .unwrap_or(false);
    let stalled = previous_fee
        .map(|previous| fee <= previous.saturating_add(50_000))
        .unwrap_or(false);
    let concession = if respected_signal {
        ((threshold as f64) * 0.04).round() as u64
    } else if round >= 3 && !stalled {
        ((threshold as f64) * 0.02).round() as u64
    } else {
        0
    };
    let adjusted_threshold = threshold.saturating_sub(concession);
    let counter_floor_ratio = if round >= 2 && stalled {
        0.94
    } else if round >= 3 {
        0.92
    } else {
        0.88
    };
    let counter_floor = ((adjusted_threshold as f64) * counter_floor_ratio).round() as u64;
    let openness_score = player_move_openness_score(current_date, player, owner_team, buyer_team);
    let (tension, patience) = transfer_negotiation_metrics(round, stalled, respected_signal);

    if fee >= adjusted_threshold {
        let status = if register_immediately {
            TransferOfferStatus::Accepted
        } else {
            TransferOfferStatus::PendingRegistration
        };
        let registration_date = (!register_immediately).then_some(registration_date_string.clone());
        let offer_id = if let Some(p) = game.players.iter_mut().find(|p| p.id == player_id) {
            upsert_transfer_offer(
                p,
                &user_team_id,
                fee,
                status,
                &date,
                Some(fee),
                round,
                None,
                registration_date,
            )
        } else {
            return Err("be.error.playerNotFound".into());
        };

        if register_immediately {
            execute_transfer(game, player_id, &user_team_id, &owner_team_id, fee)?;
            finalize_successful_transfer_offer(game, player_id, &offer_id)?;

            let player_name = game
                .players
                .iter()
                .find(|p| p.id == player_id)
                .map(|p| p.full_name.clone())
                .unwrap_or_default();

            let msg = crate::messages::transfer_complete_message(&player_name, fee, &date);
            game.messages.push(msg);
        } else {
            reserve_player_for_pending_transfer(game, player_id, &offer_id)?;
        }

        return Ok(transfer_outcome(
            TransferNegotiationDecision::Accepted,
            None,
            true,
            (!register_immediately).then_some(registration_date_string.clone()),
            build_transfer_feedback(
                if register_immediately {
                    "transfers.transferFeedbackAcceptedHeadline"
                } else {
                    "transfers.transferFeedbackScheduledHeadline"
                },
                if register_immediately {
                    "transfers.transferFeedbackAcceptedDetail"
                } else {
                    "transfers.transferFeedbackScheduledDetail"
                },
                NegotiationMood::Positive,
                tension.saturating_sub(8),
                patience.saturating_add(6).min(90),
                round,
                &[("fee", fee.to_string()), ("date", registration_date_string)],
            ),
        ));
    }

    if fee >= counter_floor {
        let suggested_fee = round_transfer_fee(adjusted_threshold);
        if let Some(p) = game.players.iter_mut().find(|p| p.id == player_id) {
            upsert_transfer_offer(
                p,
                &user_team_id,
                fee,
                TransferOfferStatus::Pending,
                &date,
                Some(fee),
                round,
                Some(suggested_fee),
                None,
            );
        }

        return Ok(transfer_outcome(
            TransferNegotiationDecision::CounterOffer,
            Some(suggested_fee),
            false,
            None,
            build_transfer_feedback(
                "transfers.transferFeedbackCounterHeadline",
                "transfers.transferFeedbackCounterDetail",
                if openness_score >= 45 {
                    NegotiationMood::Firm
                } else {
                    NegotiationMood::Tense
                },
                if openness_score >= 45 {
                    tension.saturating_sub(6)
                } else {
                    tension.saturating_add(6).min(90)
                },
                if openness_score >= 45 {
                    patience.saturating_add(4).min(90)
                } else {
                    patience.saturating_sub(4)
                },
                round,
                &[("fee", suggested_fee.to_string())],
            ),
        ));
    }

    if let Some(p) = game.players.iter_mut().find(|p| p.id == player_id) {
        upsert_transfer_offer(
            p,
            &user_team_id,
            fee,
            TransferOfferStatus::Rejected,
            &date,
            Some(fee),
            round,
            None,
            None,
        );
    }

    Ok(transfer_outcome(
        TransferNegotiationDecision::Rejected,
        None,
        true,
        None,
        build_transfer_feedback(
            "transfers.transferFeedbackRejectedHeadline",
            "transfers.transferFeedbackRejectedDetail",
            NegotiationMood::Guarded,
            tension.saturating_add(10).min(92),
            patience.saturating_sub(14),
            round,
            &[("fee", round_transfer_fee(adjusted_threshold).to_string())],
        ),
    ))
}

/// Respond to an incoming transfer offer on one of user's players.
pub fn respond_to_offer(
    game: &mut Game,
    player_id: &str,
    offer_id: &str,
    accept: bool,
) -> Result<(), String> {
    expire_stale_transfer_offers(game);

    let user_team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("be.error.noTeamAssigned")?;

    let player = game
        .players
        .iter()
        .find(|p| p.id == player_id && p.team_id.as_deref() == Some(&user_team_id))
        .ok_or(ERR_PLAYER_NOT_OWNED_BY_USER)?;

    if accept && player_has_active_or_pending_loan(player) {
        return Err(ERR_PLAYER_ALREADY_LOANED.into());
    }

    if accept && has_pending_transfer_registration(player) {
        return Err(ERR_OFFER_NOT_PENDING.into());
    }

    let offer = player
        .transfer_offers
        .iter()
        .find(|o| o.id == offer_id && o.status == TransferOfferStatus::Pending)
        .ok_or(ERR_OFFER_NOT_PENDING)?;

    let from_team_id = offer.from_team_id.clone();
    let fee = offer.fee;
    let current_date = game.clock.current_date.date_naive();
    let registration_date = if accept {
        transfer_registration_date(game)?
    } else {
        current_date
    };
    let register_immediately = registration_date == current_date;
    let registration_date_string = registration_date.format("%Y-%m-%d").to_string();
    let owner_team = game
        .teams
        .iter()
        .find(|team| team.id == user_team_id)
        .ok_or("be.error.managedTeamNotFound")?;
    let buyer_team = game
        .teams
        .iter()
        .find(|team| team.id == from_team_id)
        .ok_or("be.error.teamNotFound")?;
    let openness_score = player_move_openness_score(current_date, player, owner_team, buyer_team);

    // Update offer status
    if let Some(p) = game.players.iter_mut().find(|p| p.id == player_id)
        && let Some(o) = p.transfer_offers.iter_mut().find(|o| o.id == offer_id)
    {
        o.status = if accept {
            if register_immediately {
                TransferOfferStatus::Accepted
            } else {
                TransferOfferStatus::PendingRegistration
            }
        } else {
            TransferOfferStatus::Rejected
        };
        o.registration_date = if accept && !register_immediately {
            Some(registration_date_string.clone())
        } else {
            None
        };
    }

    if accept {
        if register_immediately {
            execute_transfer(game, player_id, &from_team_id, &user_team_id, fee)?;
            finalize_successful_transfer_offer(game, player_id, offer_id)?;
        } else {
            reserve_player_for_pending_transfer(game, player_id, offer_id)?;
        }
    } else if let Some(player) = game
        .players
        .iter_mut()
        .find(|player| player.id == player_id)
    {
        apply_blocked_move_consequences(player, openness_score);
    }

    Ok(())
}

pub fn counter_offer(
    game: &mut Game,
    player_id: &str,
    offer_id: &str,
    requested_fee: u64,
) -> Result<TransferNegotiationOutcome, String> {
    expire_stale_transfer_offers(game);

    let user_team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("be.error.noTeamAssigned")?;

    let player = game
        .players
        .iter()
        .find(|p| p.id == player_id && p.team_id.as_deref() == Some(&user_team_id))
        .ok_or(ERR_PLAYER_NOT_OWNED_BY_USER)?;

    if player_has_active_or_pending_loan(player) {
        return Err(ERR_PLAYER_ALREADY_LOANED.into());
    }

    if has_pending_transfer_registration(player) {
        return Err(ERR_OFFER_NOT_PENDING.into());
    }

    let offer = player
        .transfer_offers
        .iter()
        .find(|offer| offer.id == offer_id && offer.status == TransferOfferStatus::Pending)
        .ok_or(ERR_OFFER_NOT_PENDING)?;

    if requested_fee <= offer.fee {
        return Err(ERR_COUNTER_OFFER_MUST_EXCEED_CURRENT.into());
    }

    let buyer_team = game
        .teams
        .iter()
        .find(|team| team.id == offer.from_team_id)
        .ok_or("be.error.teamNotFound")?;

    let buyer_team_id = buyer_team.id.clone();
    let current_date = game.clock.current_date.date_naive();
    let registration_date = transfer_registration_date(game)?;
    let register_immediately = registration_date == current_date;
    let registration_date_string = registration_date.format("%Y-%m-%d").to_string();
    let round = offer.negotiation_round.max(1).saturating_add(1);
    let respected_signal = offer
        .suggested_counter_fee
        .map(|suggested| requested_fee <= suggested.saturating_add(50_000))
        .unwrap_or(false);
    let stalled = requested_fee > offer.fee.saturating_add(175_000);
    let (tension, patience) = transfer_negotiation_metrics(round, stalled, respected_signal);
    let counter_ceiling = buyer_counter_offer_ceiling(current_date, player, offer.fee, buyer_team);
    let budget_cap =
        (buyer_team.transfer_budget.max(0) as u64).min(buyer_team.finance.max(0) as u64);
    let goodwill_margin = if respected_signal { 50_000 } else { 0 };
    let accepted = requested_fee
        <= counter_ceiling
            .saturating_add(goodwill_margin)
            .min(budget_cap);
    let counter_window =
        ((counter_ceiling as f64) * if round >= 3 && stalled { 1.03 } else { 1.08 }).round() as u64;
    let date = game.clock.current_date.format("%Y-%m-%d").to_string();

    if let Some(player) = game
        .players
        .iter_mut()
        .find(|player| player.id == player_id)
        && let Some(offer) = player
            .transfer_offers
            .iter_mut()
            .find(|offer| offer.id == offer_id)
    {
        if accepted {
            offer.fee = requested_fee;
            offer.status = if register_immediately {
                TransferOfferStatus::Accepted
            } else {
                TransferOfferStatus::PendingRegistration
            };
            offer.last_manager_fee = Some(requested_fee);
            offer.negotiation_round = round;
            offer.suggested_counter_fee = None;
            offer.registration_date = if register_immediately {
                None
            } else {
                Some(registration_date_string.clone())
            };
        } else if requested_fee > counter_window {
            offer.status = TransferOfferStatus::Rejected;
            offer.last_manager_fee = Some(requested_fee);
            offer.negotiation_round = round;
            offer.suggested_counter_fee = None;
            offer.registration_date = None;
        }
        offer.date = date.clone();
    }

    if accepted {
        if register_immediately {
            execute_transfer(
                game,
                player_id,
                &buyer_team_id,
                &user_team_id,
                requested_fee,
            )?;
            finalize_successful_transfer_offer(game, player_id, offer_id)?;
        } else {
            reserve_player_for_pending_transfer(game, player_id, offer_id)?;
        }
        return Ok(transfer_outcome(
            TransferNegotiationDecision::Accepted,
            None,
            true,
            (!register_immediately).then_some(registration_date_string.clone()),
            build_transfer_feedback(
                if register_immediately {
                    "transfers.transferFeedbackAcceptedHeadline"
                } else {
                    "transfers.transferFeedbackScheduledHeadline"
                },
                if register_immediately {
                    "transfers.transferFeedbackAcceptedDetail"
                } else {
                    "transfers.transferFeedbackScheduledDetail"
                },
                NegotiationMood::Positive,
                tension.saturating_sub(8),
                patience.saturating_add(8).min(92),
                round,
                &[
                    ("fee", requested_fee.to_string()),
                    ("date", registration_date_string),
                ],
            ),
        ));
    }

    if requested_fee <= counter_window {
        let suggested_fee = round_transfer_fee(counter_ceiling);
        if let Some(player) = game
            .players
            .iter_mut()
            .find(|player| player.id == player_id)
            && let Some(offer) = player
                .transfer_offers
                .iter_mut()
                .find(|offer| offer.id == offer_id)
        {
            offer.fee = suggested_fee;
            offer.status = TransferOfferStatus::Pending;
            offer.last_manager_fee = Some(requested_fee);
            offer.negotiation_round = round;
            offer.suggested_counter_fee = Some(suggested_fee);
            offer.registration_date = None;
            offer.date = date;
        }

        return Ok(transfer_outcome(
            TransferNegotiationDecision::CounterOffer,
            Some(suggested_fee),
            false,
            None,
            build_transfer_feedback(
                "transfers.transferFeedbackCounterHeadline",
                "transfers.transferFeedbackCounterDetail",
                NegotiationMood::Firm,
                tension,
                patience,
                round,
                &[("fee", suggested_fee.to_string())],
            ),
        ));
    }

    Ok(transfer_outcome(
        TransferNegotiationDecision::Rejected,
        None,
        true,
        None,
        build_transfer_feedback(
            "transfers.transferFeedbackRejectedHeadline",
            "transfers.transferFeedbackRejectedDetail",
            NegotiationMood::Tense,
            tension.saturating_add(10).min(92),
            patience.saturating_sub(12),
            round,
            &[("fee", round_transfer_fee(counter_ceiling).to_string())],
        ),
    ))
}

pub(crate) fn round_transfer_fee(value: u64) -> u64 {
    if value == 0 {
        return 0;
    }

    value.div_ceil(50_000) * 50_000
}

pub(crate) fn build_transfer_feedback(
    headline_key: &str,
    detail_key: &str,
    mood: NegotiationMood,
    tension: u8,
    patience: u8,
    round: u8,
    params: &[(&str, String)],
) -> NegotiationFeedback {
    NegotiationFeedback {
        mood,
        headline_key: headline_key.to_string(),
        detail_key: Some(detail_key.to_string()),
        tension,
        patience,
        round,
        params: params
            .iter()
            .map(|(key, value)| ((*key).to_string(), value.clone()))
            .collect(),
    }
}
