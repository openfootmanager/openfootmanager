use crate::finances::calc_annual_wages;
use crate::game::Game;
use chrono::NaiveDate;
use domain::league::CompletedTransfer;
use domain::negotiation::{NegotiationFeedback, NegotiationMood};
use domain::player::TransferOfferStatus;
use domain::season::TransferWindowStatus;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

const TRANSFER_NEGOTIATION_STALE_DAYS: i64 = 14;
const MAX_COMPLETED_AI_TRANSFERS_PER_DAY: usize = 2;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransferNegotiationDecision {
    Accepted,
    Rejected,
    CounterOffer,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransferNegotiationOutcome {
    pub decision: TransferNegotiationDecision,
    pub suggested_fee: Option<u64>,
    pub is_terminal: bool,
    pub feedback: NegotiationFeedback,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransferBidFinancialProjection {
    pub transfer_budget_before: i64,
    pub transfer_budget_after: i64,
    pub finance_before: i64,
    pub finance_after: i64,
    pub annual_wage_bill_before: i64,
    pub annual_wage_bill_after: i64,
    pub annual_wage_budget: i64,
    pub projected_wage_budget_usage_pct: i64,
    pub exceeds_transfer_budget: bool,
    pub exceeds_finance: bool,
}

enum PlayerImportance {
    Key,
    Regular,
    Fringe,
}

struct MarketCandidate {
    player_id: String,
    owner_team_id: String,
    score: i32,
    fee: u64,
}

fn contract_days_remaining(current_date: NaiveDate, contract_end: Option<&str>) -> Option<i64> {
    let contract_end = contract_end?;
    let contract_end_date = NaiveDate::parse_from_str(contract_end, "%Y-%m-%d").ok()?;
    Some((contract_end_date - current_date).num_days())
}

fn infer_player_importance(
    player: &domain::player::Player,
    owner_team: &domain::team::Team,
) -> PlayerImportance {
    if owner_team.starting_xi_ids.iter().any(|id| id == &player.id) {
        return PlayerImportance::Key;
    }

    if player.market_value >= 1_500_000 {
        return PlayerImportance::Regular;
    }

    PlayerImportance::Fringe
}

fn minimum_acceptable_fee(
    current_date: NaiveDate,
    player: &domain::player::Player,
    owner_team: &domain::team::Team,
    buyer_team: &domain::team::Team,
) -> u64 {
    let mut multiplier: f64 = if player.transfer_listed { 0.8 } else { 1.2 };

    if let Some(days_remaining) =
        contract_days_remaining(current_date, player.contract_end.as_deref())
    {
        if days_remaining <= 60 {
            multiplier -= 0.25;
        } else if days_remaining <= 180 {
            multiplier -= 0.15;
        } else if days_remaining <= 365 {
            multiplier -= 0.05;
        }
    }

    match infer_player_importance(player, owner_team) {
        PlayerImportance::Key => multiplier += 0.2,
        PlayerImportance::Regular => multiplier += 0.1,
        PlayerImportance::Fringe => {}
    }

    if player.morale <= 40 {
        multiplier -= 0.05;
    }

    let openness_score = player_move_openness_score(current_date, player, owner_team, buyer_team);
    if openness_score >= 60 {
        multiplier -= 0.20;
    } else if openness_score >= 40 {
        multiplier -= 0.10;
    }

    let multiplier = multiplier.clamp(0.55, 1.6);
    ((player.market_value as f64) * multiplier).round() as u64
}

fn player_move_openness_score(
    current_date: NaiveDate,
    player: &domain::player::Player,
    owner_team: &domain::team::Team,
    buyer_team: &domain::team::Team,
) -> i32 {
    let mut score = 0;

    if player.morale <= 45 {
        score += 20;
    } else if player.morale <= 60 {
        score += 10;
    }

    if player.stats.appearances <= 2 {
        score += 15;
    } else if player.stats.appearances <= 5 {
        score += 8;
    }

    if let Some(days_remaining) =
        contract_days_remaining(current_date, player.contract_end.as_deref())
    {
        if days_remaining <= 180 {
            score += 20;
        } else if days_remaining <= 365 {
            score += 10;
        }
    }

    let reputation_gap = buyer_team.reputation as i32 - owner_team.reputation as i32;
    if reputation_gap >= 200 {
        score += 25;
    } else if reputation_gap >= 75 {
        score += 15;
    }

    if player.transfer_listed {
        score += 10;
    }

    score
}

fn apply_blocked_move_consequences(player: &mut domain::player::Player, openness_score: i32) {
    if openness_score < 40 {
        return;
    }

    let morale_drop = if openness_score >= 60 { 10 } else { 6 };
    player.morale = (i16::from(player.morale) - morale_drop).clamp(0, 100) as u8;
    player.morale_core.manager_trust =
        (i16::from(player.morale_core.manager_trust) - 5).clamp(0, 100) as u8;
    player.morale_core.unresolved_issue = Some(domain::player::PlayerIssue {
        category: domain::player::PlayerIssueCategory::Contract,
        severity: if openness_score >= 60 { 75 } else { 60 },
    });
}

fn incoming_interest_score(current_date: NaiveDate, player: &domain::player::Player) -> i32 {
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

fn suggested_incoming_fee(current_date: NaiveDate, player: &domain::player::Player) -> u64 {
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

fn has_open_incoming_offer_from_club(player: &domain::player::Player, club_id: &str) -> bool {
    player
        .transfer_offers
        .iter()
        .any(|offer| offer.from_team_id == club_id && offer.status == TransferOfferStatus::Pending)
}

fn offer_is_stale(current_date: NaiveDate, offer: &domain::player::TransferOffer) -> bool {
    if offer.status != TransferOfferStatus::Pending {
        return false;
    }

    let Ok(offer_date) = NaiveDate::parse_from_str(&offer.date, "%Y-%m-%d") else {
        return false;
    };

    (current_date - offer_date).num_days() >= TRANSFER_NEGOTIATION_STALE_DAYS
}

fn expire_stale_transfer_offers(game: &mut Game) {
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

fn find_open_offer_from_club<'a>(
    player: &'a domain::player::Player,
    club_id: &str,
) -> Option<&'a domain::player::TransferOffer> {
    player
        .transfer_offers
        .iter()
        .find(|offer| offer.from_team_id == club_id && offer.status == TransferOfferStatus::Pending)
}

fn negotiation_round_from_offer(offer: Option<&domain::player::TransferOffer>) -> u8 {
    offer
        .map(|offer| offer.negotiation_round.max(1).saturating_add(1))
        .unwrap_or(1)
}

fn transfer_negotiation_metrics(round: u8, stalled: bool, respected_signal: bool) -> (u8, u8) {
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

fn upsert_transfer_offer(
    player: &mut domain::player::Player,
    from_team_id: &str,
    fee: u64,
    status: TransferOfferStatus,
    date: &str,
    last_manager_fee: Option<u64>,
    negotiation_round: u8,
    suggested_counter_fee: Option<u64>,
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
    });
    offer_id
}

fn transfer_window_is_open(game: &Game) -> bool {
    matches!(
        game.season_context.transfer_window.status,
        TransferWindowStatus::Open | TransferWindowStatus::DeadlineDay
    )
}

pub fn evaluate_transfer_market(game: &mut Game) {
    expire_stale_transfer_offers(game);

    if !transfer_window_is_open(game) {
        return;
    }

    let user_team_id = game.manager.team_id.clone();

    let current_date = game.clock.current_date.date_naive();
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();

    let buyer_ids: Vec<String> = game
        .teams
        .iter()
        .filter(|team| Some(team.id.as_str()) != user_team_id.as_deref())
        .map(|team| team.id.clone())
        .collect();
    let mut completed_ai_transfers = 0_usize;
    let mut moved_player_ids: HashSet<String> = HashSet::new();

    for buyer_id in buyer_ids {
        let Some(buyer_team) = game.teams.iter().find(|team| team.id == buyer_id).cloned() else {
            continue;
        };

        let mut chosen: Option<MarketCandidate> = None;

        for player in &game.players {
            let Some(owner_team_id) = player.team_id.as_deref() else {
                continue;
            };

            if owner_team_id == buyer_id || moved_player_ids.contains(&player.id) {
                continue;
            }

            let is_user_owned = Some(owner_team_id) == user_team_id.as_deref();
            if !is_user_owned && completed_ai_transfers >= MAX_COMPLETED_AI_TRANSFERS_PER_DAY {
                continue;
            }

            if is_user_owned && has_open_incoming_offer_from_club(player, &buyer_id) {
                continue;
            }

            let score = incoming_interest_score(current_date, player);
            if score < 35 {
                continue;
            }

            let fee = suggested_incoming_fee(current_date, player);
            if buyer_team.transfer_budget < fee as i64 || buyer_team.finance < fee as i64 {
                continue;
            }

            if chosen
                .as_ref()
                .is_none_or(|candidate| score > candidate.score)
            {
                chosen = Some(MarketCandidate {
                    player_id: player.id.clone(),
                    owner_team_id: owner_team_id.to_string(),
                    score,
                    fee,
                });
            }
        }

        let Some(candidate) = chosen else {
            continue;
        };

        if Some(candidate.owner_team_id.as_str()) == user_team_id.as_deref() {
            create_incoming_user_offer(game, &candidate, &buyer_id, &buyer_team.name, &today);
            continue;
        }

        if candidate.score <= 60 || completed_ai_transfers >= MAX_COMPLETED_AI_TRANSFERS_PER_DAY {
            continue;
        }

        if execute_transfer(
            game,
            &candidate.player_id,
            &buyer_id,
            &candidate.owner_team_id,
            candidate.fee,
        )
        .is_ok()
        {
            moved_player_ids.insert(candidate.player_id);
            completed_ai_transfers += 1;
        }
    }
}

pub fn generate_incoming_transfer_offers(game: &mut Game) {
    evaluate_transfer_market(game);
}

fn create_incoming_user_offer(
    game: &mut Game,
    candidate: &MarketCandidate,
    buyer_id: &str,
    buyer_name: &str,
    today: &str,
) {
    let Some(player) = game
        .players
        .iter_mut()
        .find(|player| player.id == candidate.player_id)
    else {
        return;
    };

    let offer_id = Uuid::new_v4().to_string();

    player.transfer_offers.push(domain::player::TransferOffer {
        id: offer_id.clone(),
        from_team_id: buyer_id.to_string(),
        fee: candidate.fee,
        wage_offered: 0,
        last_manager_fee: None,
        negotiation_round: 1,
        suggested_counter_fee: None,
        status: TransferOfferStatus::Pending,
        date: today.to_string(),
    });

    let player_name = player.full_name.clone();
    let message = crate::messages::incoming_transfer_offer_message(
        &offer_id,
        &candidate.player_id,
        &player_name,
        buyer_name,
        candidate.fee,
        today,
    );
    game.messages.push(message);
}

fn buyer_counter_offer_ceiling(
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

fn should_generate_major_transfer_news(player: &domain::player::Player, fee: u64) -> bool {
    fee >= 1_000_000 || player.market_value >= 1_000_000
}

fn transfer_outcome(
    decision: TransferNegotiationDecision,
    suggested_fee: Option<u64>,
    is_terminal: bool,
    feedback: NegotiationFeedback,
) -> TransferNegotiationOutcome {
    TransferNegotiationOutcome {
        decision,
        suggested_fee,
        is_terminal,
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
        .ok_or_else(|| "No user team".to_string())?;

    let player = game
        .players
        .iter()
        .find(|player| player.id == player_id)
        .ok_or_else(|| "Player not found".to_string())?;

    if player.team_id.as_deref() == Some(user_team_id.as_str()) {
        return Err("Cannot bid on your own player".to_string());
    }

    let team = game
        .teams
        .iter()
        .find(|team| team.id == user_team_id)
        .ok_or_else(|| "User team not found".to_string())?;

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

    if !transfer_window_is_open(game) {
        return Err("Transfer window is closed".into());
    }

    let user_team_id = game.manager.team_id.clone().ok_or("No user team")?;

    let player = game
        .players
        .iter()
        .find(|p| p.id == player_id)
        .ok_or("Player not found")?;

    if player.team_id.as_deref() == Some(&user_team_id) {
        return Err("Cannot bid on your own player".into());
    }

    let owner_team_id = player.team_id.clone().ok_or("Player has no team")?;

    let my_team = game
        .teams
        .iter()
        .find(|t| t.id == user_team_id)
        .ok_or("User team not found")?;

    if (my_team.finance as u64) < fee {
        return Err("Insufficient funds".into());
    }

    if my_team.transfer_budget < fee as i64 {
        return Err("Transfer budget too low".into());
    }

    let owner_team = game
        .teams
        .iter()
        .find(|t| t.id == owner_team_id)
        .ok_or("Owner team not found")?;

    let buyer_team = my_team;

    let current_date = game.clock.current_date.date_naive();

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
        if let Some(p) = game.players.iter_mut().find(|p| p.id == player_id) {
            upsert_transfer_offer(
                p,
                &user_team_id,
                fee,
                TransferOfferStatus::Accepted,
                &date,
                Some(fee),
                round,
                None,
            );
        }

        // Execute transfer
        execute_transfer(game, player_id, &user_team_id, &owner_team_id, fee)?;

        // Generate message
        let player_name = game
            .players
            .iter()
            .find(|p| p.id == player_id)
            .map(|p| p.full_name.clone())
            .unwrap_or_default();

        let msg = crate::messages::transfer_complete_message(&player_name, fee, &date);
        game.messages.push(msg);

        return Ok(transfer_outcome(
            TransferNegotiationDecision::Accepted,
            None,
            true,
            build_transfer_feedback(
                "transfers.transferFeedbackAcceptedHeadline",
                "transfers.transferFeedbackAcceptedDetail",
                NegotiationMood::Positive,
                tension.saturating_sub(8),
                patience.saturating_add(6).min(90),
                round,
                &[("fee", fee.to_string())],
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
            );
        }

        return Ok(transfer_outcome(
            TransferNegotiationDecision::CounterOffer,
            Some(suggested_fee),
            false,
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
        );
    }

    Ok(transfer_outcome(
        TransferNegotiationDecision::Rejected,
        None,
        true,
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

    if accept && !transfer_window_is_open(game) {
        return Err("Transfer window is closed".into());
    }

    let user_team_id = game.manager.team_id.clone().ok_or("No user team")?;

    let player = game
        .players
        .iter()
        .find(|p| p.id == player_id && p.team_id.as_deref() == Some(&user_team_id))
        .ok_or("Player not found or not yours")?;

    let offer = player
        .transfer_offers
        .iter()
        .find(|o| o.id == offer_id && o.status == TransferOfferStatus::Pending)
        .ok_or("Offer not found or not pending")?;

    let from_team_id = offer.from_team_id.clone();
    let fee = offer.fee;
    let current_date = game.clock.current_date.date_naive();
    let owner_team = game
        .teams
        .iter()
        .find(|team| team.id == user_team_id)
        .ok_or("User team not found")?;
    let buyer_team = game
        .teams
        .iter()
        .find(|team| team.id == from_team_id)
        .ok_or("Buying team not found")?;
    let openness_score = player_move_openness_score(current_date, player, owner_team, buyer_team);

    // Update offer status
    if let Some(p) = game.players.iter_mut().find(|p| p.id == player_id)
        && let Some(o) = p.transfer_offers.iter_mut().find(|o| o.id == offer_id)
    {
        o.status = if accept {
            TransferOfferStatus::Accepted
        } else {
            TransferOfferStatus::Rejected
        };
    }

    if accept {
        execute_transfer(game, player_id, &from_team_id, &user_team_id, fee)?;
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

    if !transfer_window_is_open(game) {
        return Err("Transfer window is closed".into());
    }

    let user_team_id = game.manager.team_id.clone().ok_or("No user team")?;

    let player = game
        .players
        .iter()
        .find(|p| p.id == player_id && p.team_id.as_deref() == Some(&user_team_id))
        .ok_or("Player not found or not yours")?;

    let offer = player
        .transfer_offers
        .iter()
        .find(|offer| offer.id == offer_id && offer.status == TransferOfferStatus::Pending)
        .ok_or("Offer not found or not pending")?;

    if requested_fee <= offer.fee {
        return Err("Counter offer must exceed current offer".into());
    }

    let buyer_team = game
        .teams
        .iter()
        .find(|team| team.id == offer.from_team_id)
        .ok_or("Buying team not found")?;

    let buyer_team_id = buyer_team.id.clone();
    let current_date = game.clock.current_date.date_naive();
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
            offer.status = TransferOfferStatus::Accepted;
            offer.last_manager_fee = Some(requested_fee);
            offer.negotiation_round = round;
            offer.suggested_counter_fee = None;
        } else if requested_fee > counter_window {
            offer.status = TransferOfferStatus::Rejected;
            offer.last_manager_fee = Some(requested_fee);
            offer.negotiation_round = round;
            offer.suggested_counter_fee = None;
        }
        offer.date = date.clone();
    }

    if accepted {
        execute_transfer(
            game,
            player_id,
            &buyer_team_id,
            &user_team_id,
            requested_fee,
        )?;
        return Ok(transfer_outcome(
            TransferNegotiationDecision::Accepted,
            None,
            true,
            build_transfer_feedback(
                "transfers.transferFeedbackAcceptedHeadline",
                "transfers.transferFeedbackAcceptedDetail",
                NegotiationMood::Positive,
                tension.saturating_sub(8),
                patience.saturating_add(8).min(92),
                round,
                &[("fee", requested_fee.to_string())],
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
            offer.date = date;
        }

        return Ok(transfer_outcome(
            TransferNegotiationDecision::CounterOffer,
            Some(suggested_fee),
            false,
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

fn round_transfer_fee(value: u64) -> u64 {
    if value == 0 {
        return 0;
    }

    ((value + 49_999) / 50_000) * 50_000
}

fn build_transfer_feedback(
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

/// Transfer a player between teams, adjusting finances.
fn execute_transfer(
    game: &mut Game,
    player_id: &str,
    to_team_id: &str,
    from_team_id: &str,
    fee: u64,
) -> Result<(), String> {
    let player_snapshot = game
        .players
        .iter()
        .find(|player| player.id == player_id)
        .cloned()
        .ok_or("Player not found")?;
    let from_team_name = game
        .teams
        .iter()
        .find(|team| team.id == from_team_id)
        .map(|team| team.name.clone())
        .unwrap_or_else(|| from_team_id.to_string());
    let to_team_name = game
        .teams
        .iter()
        .find(|team| team.id == to_team_id)
        .map(|team| team.name.clone())
        .unwrap_or_else(|| to_team_id.to_string());
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let departing_starter_ids: Vec<String> = game
        .teams
        .iter()
        .find(|team| team.id == from_team_id)
        .filter(|team| team.starting_xi_ids.iter().any(|id| id == player_id))
        .map(|team| {
            team.starting_xi_ids
                .iter()
                .filter(|id| id.as_str() != player_id)
                .cloned()
                .collect()
        })
        .unwrap_or_default();

    // Move player
    if let Some(p) = game.players.iter_mut().find(|p| p.id == player_id) {
        p.team_id = Some(to_team_id.to_string());
        p.transfer_listed = false;
        p.loan_listed = false;
        // Remove from any starting XI
    }

    if !departing_starter_ids.is_empty() {
        for player in &mut game.players {
            if player.team_id.as_deref() == Some(from_team_id)
                && departing_starter_ids.iter().any(|id| id == &player.id)
            {
                player.morale = (i16::from(player.morale) - 4).clamp(0, 100) as u8;
            }
        }
    }

    // Debit buying team
    if let Some(t) = game.teams.iter_mut().find(|t| t.id == to_team_id) {
        t.finance -= fee as i64;
        // Remove from starting XI if player was there
        if let Some(pos) = t.starting_xi_ids.iter().position(|id| id == player_id) {
            t.starting_xi_ids.remove(pos);
        }
    }

    // Credit selling team
    if let Some(t) = game.teams.iter_mut().find(|t| t.id == from_team_id) {
        t.finance += fee as i64;
        // Remove from starting XI
        if let Some(pos) = t.starting_xi_ids.iter().position(|id| id == player_id) {
            t.starting_xi_ids.remove(pos);
        }
    }

    if should_generate_major_transfer_news(&player_snapshot, fee) {
        let article_id = format!(
            "transfer_news_{}_{}_{}_{}",
            player_id, from_team_id, to_team_id, today
        );
        if !game.news.iter().any(|article| article.id == article_id) {
            game.news.push(crate::news::major_transfer_article(
                &article_id,
                player_id,
                &player_snapshot.full_name,
                from_team_id,
                &from_team_name,
                to_team_id,
                &to_team_name,
                fee,
                &today,
            ));
        }
    }

    if let Some(league) = &mut game.league {
        league.transfer_log.push(CompletedTransfer {
            date: today,
            from_team_id: from_team_id.to_string(),
            to_team_id: to_team_id.to_string(),
            player_id: player_id.to_string(),
            fee,
        });
    }

    Ok(())
}
