use crate::contract_wage_policy::{
    renewal_wage_policy_error_message, wage_policy_allows_projection,
};
use crate::finances::calc_annual_wages;
use crate::game::Game;
use chrono::{Datelike, Duration, NaiveDate};
use domain::league::CompletedTransfer;
use domain::negotiation::{NegotiationFeedback, NegotiationMood};
use domain::player::{
    ActiveLoan, LoanOfferStatus, PlayerMovementEntry, PlayerMovementKind, Position,
    TransferOfferStatus,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

mod bids;
mod consts;
mod execution;
mod registration;

pub(crate) use self::consts::*;
pub(crate) use self::registration::*;
// `pub` (not `pub(crate)`): these clusters re-export public entry points that
// external crates call as `ofm_core::transfers::*` (process_*, make_transfer_bid,
// respond_to_offer, counter_offer, project_transfer_bid_financial_impact).
pub use self::bids::*;
pub use self::execution::*;

fn loan_wage_share(player: &domain::player::Player, wage_contribution_pct: u8) -> i64 {
    (i64::from(player.wage) * i64::from(wage_contribution_pct)) / 100
}

fn validate_loan_borrower_affordability(
    game: &Game,
    borrower_team_id: &str,
    player: &domain::player::Player,
    wage_contribution_pct: u8,
) -> Result<(), String> {
    let borrower_team = game
        .teams
        .iter()
        .find(|team| team.id == borrower_team_id)
        .ok_or("be.error.teamNotFound")?;
    let projected_wage_share = loan_wage_share(player, wage_contribution_pct);
    let current_wage_bill = calc_annual_wages(game, borrower_team_id);
    let projected_wage_bill = current_wage_bill.saturating_add(projected_wage_share);

    if !wage_policy_allows_projection(borrower_team, current_wage_bill, projected_wage_bill) {
        return Err(renewal_wage_policy_error_message(borrower_team));
    }

    if borrower_team.finance < projected_wage_share {
        return Err(ERR_INSUFFICIENT_FUNDS.to_string());
    }

    Ok(())
}

/// Populate a small, deterministic opening loan market for AI clubs.
///
/// This is intended for one-time career setup/save migration, not daily market
/// maintenance. Existing listings are preserved and count toward each club's
/// target.
pub fn seed_opening_ai_loan_market(game: &mut Game) -> usize {
    let user_team_id = game.manager.team_id.as_deref();
    let current_date = game.clock.current_date.date_naive();
    let ai_teams: Vec<(String, HashSet<String>)> = game
        .teams
        .iter()
        .filter(|team| Some(team.id.as_str()) != user_team_id)
        .map(|team| {
            (
                team.id.clone(),
                team.starting_xi_ids.iter().cloned().collect(),
            )
        })
        .collect();
    let mut seeded = 0;

    for (team_id, starting_xi_ids) in ai_teams {
        let existing_listings = game
            .players
            .iter()
            .filter(|player| {
                player.team_id.as_deref() == Some(team_id.as_str())
                    && player.loan_listed
                    && !player_has_pending_registration(player)
            })
            .count();
        let listings_needed = OPENING_LOAN_LISTINGS_PER_AI_TEAM.saturating_sub(existing_listings);

        if listings_needed == 0 {
            continue;
        }

        let mut candidates: Vec<usize> = game
            .players
            .iter()
            .enumerate()
            .filter(|(_, player)| {
                player.team_id.as_deref() == Some(team_id.as_str())
                    && !player.retired
                    && !player.transfer_listed
                    && !player.loan_listed
                    && !player_has_pending_registration(player)
                    && !starting_xi_ids.contains(&player.id)
                    && player.contract_end.as_deref().is_some_and(|contract_end| {
                        NaiveDate::parse_from_str(contract_end, "%Y-%m-%d").is_ok_and(|date| {
                            date >= current_date
                                + Duration::days(MIN_OPENING_LOAN_CONTRACT_RUNWAY_DAYS)
                        })
                    })
            })
            .map(|(index, _)| index)
            .collect();

        candidates.sort_by(|left, right| {
            let left = &game.players[*left];
            let right = &game.players[*right];

            right
                .date_of_birth
                .cmp(&left.date_of_birth)
                .then_with(|| right.potential.cmp(&left.potential))
                .then_with(|| left.ovr.cmp(&right.ovr))
                .then_with(|| left.id.cmp(&right.id))
        });

        for player_index in candidates.into_iter().take(listings_needed) {
            game.players[player_index].loan_listed = true;
            seeded += 1;
        }
    }

    seeded
}

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registration_date: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LoanOfferDecision {
    Accepted,
    Rejected,
    CounterOffer,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoanOfferOutcome {
    pub decision: LoanOfferDecision,
    pub offer_id: String,
    pub suggested_wage_contribution_pct: Option<u8>,
    pub suggested_end_date: Option<String>,
    pub suggested_buy_option_fee: Option<u64>,
    pub is_terminal: bool,
}

enum PlayerImportance {
    Key,
    Regular,
    Fringe,
}

// `pub(crate)`: bids::create_incoming_user_offer takes `&MarketCandidate`, so
// the type must be at least as visible as that re-exported helper.
pub(crate) struct MarketCandidate {
    player_id: String,
    owner_team_id: String,
    score: i32,
    fee: u64,
}

/// A player worth pursuing, with buyer-independent appeal precomputed once so
/// every club can reuse it instead of re-scoring the whole world.
struct MarketTarget {
    player_id: String,
    owner_team_id: String,
    is_user_owned: bool,
    score: i32,
    fee: u64,
    /// Broad position group (0=GK, 1=DEF, 2=MID, 3=FWD), used to gate buyers
    /// that are already stacked in that area.
    position_group_index: usize,
    /// Reputation of the player's current club, used for reputation-fit gating.
    owner_reputation: u32,
    /// Clubs that already hold a pending bid (only tracked for user players,
    /// the one case where we must avoid duplicate incoming offers).
    pending_offer_clubs: HashSet<String>,
}

/// Broad position group index (0=GK, 1=DEF, 2=MID, 3=FWD) for squad-depth maths.
fn position_group_index(position: &domain::player::Position) -> usize {
    match position.to_group_position() {
        domain::player::Position::Goalkeeper => 0,
        domain::player::Position::Defender => 1,
        domain::player::Position::Midfielder => 2,
        _ => 3,
    }
}

/// Whether a club has a realistic reason to pursue a target: it isn't far below
/// the player's current club in stature, and it isn't already overloaded in the
/// player's position group.
fn buyer_has_genuine_interest(
    buyer_reputation: u32,
    owner_reputation: u32,
    buyer_position_depth: usize,
) -> bool {
    let reputation_deficit = owner_reputation as i32 - buyer_reputation as i32;
    reputation_deficit <= MAX_BUYER_REPUTATION_DEFICIT
        && buyer_position_depth < POSITION_GROUP_SURPLUS_THRESHOLD
}

/// Current squad depth per club and broad position group, computed once so the
/// market sweep doesn't re-scan every roster.
fn squad_position_depths(game: &Game) -> std::collections::HashMap<String, [usize; 4]> {
    let mut depths: std::collections::HashMap<String, [usize; 4]> =
        std::collections::HashMap::new();
    for player in &game.players {
        let Some(team_id) = player.team_id.as_deref() else {
            continue;
        };
        let slot = position_group_index(&player.natural_position);
        depths.entry(team_id.to_string()).or_default()[slot] += 1;
    }
    depths
}

struct LoanMarketCandidate {
    player_id: String,
    wage_contribution_pct: u8,
    buy_option_fee: Option<u64>,
    score: i32,
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

fn incoming_loan_interest_score(player: &domain::player::Player) -> i32 {
    if !player.loan_listed || player_has_pending_registration(player) {
        return 0;
    }

    let mut score = 45;

    if player.ovr >= 65 {
        score += 10;
    }

    if player.potential >= player.ovr.saturating_add(8) {
        score += 10;
    }

    if player.stats.appearances <= 5 {
        score += 10;
    }

    if player.morale <= 55 {
        score += 5;
    }

    score
}

fn suggested_loan_wage_contribution_pct(score: i32, player: &domain::player::Player) -> u8 {
    if score >= 70 || player.wage <= 150_000 {
        100
    } else if score >= 60 {
        75
    } else {
        50
    }
}

fn suggested_loan_buy_option_fee(player: &domain::player::Player) -> Option<u64> {
    if player.market_value == 0 {
        return None;
    }

    if player.potential >= player.ovr.saturating_add(12) && player.stats.appearances <= 3 {
        return None;
    }

    let multiplier = if player.loan_listed { 1.1 } else { 1.25 };
    Some(round_transfer_fee(
        ((player.market_value as f64) * multiplier).round() as u64,
    ))
}

fn default_loan_end_date(
    current_date: NaiveDate,
    player: &domain::player::Player,
) -> Option<String> {
    let minimum_end_date = current_date + Duration::days(30);
    let default_end_date = current_date + Duration::days(180);
    let end_date = match player.contract_end.as_deref() {
        Some(contract_end) => {
            let contract_end_date = NaiveDate::parse_from_str(contract_end, "%Y-%m-%d").ok()?;
            let latest_loan_end_date = contract_end_date - Duration::days(1);
            if latest_loan_end_date < minimum_end_date {
                return None;
            }
            std::cmp::min(default_end_date, latest_loan_end_date)
        }
        None => default_end_date,
    };

    Some(end_date.format("%Y-%m-%d").to_string())
}

fn award_leaderboard_player_ids(game: &Game) -> HashSet<String> {
    let awards = crate::season_awards::compute_season_awards(game);

    awards
        .golden_boot
        .iter()
        .chain(awards.assist_king.iter())
        .chain(awards.player_of_year.iter())
        .chain(awards.clean_sheet_king.iter())
        .chain(awards.most_appearances.iter())
        .chain(awards.young_player.iter())
        .map(|entry| entry.player_id.clone())
        .collect()
}

fn has_open_loan_offer_from_club(player: &domain::player::Player, club_id: &str) -> bool {
    player
        .loan_offers
        .iter()
        .any(|offer| offer.from_team_id == club_id && offer.status == LoanOfferStatus::Pending)
}

fn loan_offer_is_stale(current_date: NaiveDate, offer: &domain::player::LoanOffer) -> bool {
    if offer.status != LoanOfferStatus::Pending {
        return false;
    }

    let Ok(offer_date) = NaiveDate::parse_from_str(&offer.date, "%Y-%m-%d") else {
        return false;
    };

    (current_date - offer_date).num_days() >= TRANSFER_NEGOTIATION_STALE_DAYS
}

fn withdraw_pending_transfer_offers(player: &mut domain::player::Player) {
    for offer in &mut player.transfer_offers {
        if offer.status == TransferOfferStatus::Pending {
            offer.status = TransferOfferStatus::Withdrawn;
            offer.suggested_counter_fee = None;
        }
    }
}

fn expire_stale_loan_offers(game: &mut Game) {
    let current_date = game.clock.current_date.date_naive();

    for player in &mut game.players {
        for offer in &mut player.loan_offers {
            if loan_offer_is_stale(current_date, offer) {
                offer.status = LoanOfferStatus::Withdrawn;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn upsert_loan_offer(
    player: &mut domain::player::Player,
    from_team_id: &str,
    parent_team_id: &str,
    start_date: &str,
    end_date: &str,
    wage_contribution_pct: u8,
    buy_option_fee: Option<u64>,
    status: LoanOfferStatus,
    date: &str,
) -> String {
    if let Some(offer) = player.loan_offers.iter_mut().find(|offer| {
        offer.from_team_id == from_team_id && offer.status == LoanOfferStatus::Pending
    }) {
        offer.parent_team_id = parent_team_id.to_string();
        offer.start_date = start_date.to_string();
        offer.end_date = end_date.to_string();
        offer.wage_contribution_pct = wage_contribution_pct;
        offer.buy_option_fee = buy_option_fee;
        offer.last_manager_wage_contribution_pct = None;
        offer.last_manager_end_date = None;
        offer.last_manager_buy_option_fee = None;
        offer.negotiation_round = 1;
        offer.suggested_wage_contribution_pct = None;
        offer.suggested_end_date = None;
        offer.suggested_buy_option_fee = None;
        offer.status = status;
        offer.date = date.to_string();
        return offer.id.clone();
    }

    let offer_id = Uuid::new_v4().to_string();
    player.loan_offers.push(domain::player::LoanOffer {
        id: offer_id.clone(),
        from_team_id: from_team_id.to_string(),
        parent_team_id: parent_team_id.to_string(),
        start_date: start_date.to_string(),
        end_date: end_date.to_string(),
        wage_contribution_pct,
        buy_option_fee,
        last_manager_wage_contribution_pct: None,
        last_manager_end_date: None,
        last_manager_buy_option_fee: None,
        negotiation_round: 1,
        suggested_wage_contribution_pct: None,
        suggested_end_date: None,
        suggested_buy_option_fee: None,
        status,
        date: date.to_string(),
    });
    offer_id
}

pub fn evaluate_transfer_market(game: &mut Game) {
    expire_stale_transfer_offers(game);
    expire_stale_loan_offers(game);

    if !transfer_window_is_open(game) {
        return;
    }

    let user_team_id = game.manager.team_id.clone();

    let current_date = game.clock.current_date.date_naive();
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let award_leaderboards = award_leaderboard_player_ids(game);
    let team_reputation: std::collections::HashMap<String, u32> = game
        .teams
        .iter()
        .map(|team| (team.id.clone(), team.reputation))
        .collect();
    let position_depths = squad_position_depths(game);

    // In a multi-competition world only the player's active scope shops the
    // market each day; dormant clubs are handled by lighter periodic passes.
    // `None` means no scope is configured, so every club is a potential buyer.
    let active_team_ids = game.active_team_ids();
    let buyer_ids: Vec<String> = game
        .teams
        .iter()
        .filter(|team| Some(team.id.as_str()) != user_team_id.as_deref())
        .filter(|team| {
            active_team_ids
                .as_ref()
                .is_none_or(|ids| ids.contains(&team.id))
        })
        .map(|team| team.id.clone())
        .collect();
    let mut completed_ai_transfers = 0_usize;
    let mut moved_player_ids: HashSet<String> = HashSet::new();
    // New incoming offers opened to user players today, tracked to throttle the
    // inbox: at most one new club per player and a hard squad-wide ceiling.
    let mut new_offers_per_player: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut new_loan_offers_per_player: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut new_user_offers_today = 0_usize;
    let mut new_user_loan_offers_today = 0_usize;

    // A player's transfer appeal and asking fee don't depend on who's buying, so
    // score every player once and keep only the genuinely attractive targets.
    // Each club then scans this short, score-sorted list instead of the whole
    // world, turning an O(clubs × players) sweep into O(players + clubs × shortlist).
    let mut shortlist: Vec<MarketTarget> = Vec::new();
    for player in &game.players {
        let Some(owner_team_id) = player.team_id.as_deref() else {
            continue;
        };
        if player_has_pending_registration(player) {
            continue;
        }
        let mut score = incoming_interest_score(current_date, player);
        if award_leaderboards.contains(&player.id) {
            score += AWARD_LEADERBOARD_INTEREST_BONUS;
        }
        if score < 35 {
            continue;
        }
        let is_user_owned = Some(owner_team_id) == user_team_id.as_deref();
        let pending_offer_clubs: HashSet<String> = if is_user_owned {
            player
                .transfer_offers
                .iter()
                .filter(|offer| offer.status == TransferOfferStatus::Pending)
                .map(|offer| offer.from_team_id.clone())
                .collect()
        } else {
            HashSet::new()
        };
        shortlist.push(MarketTarget {
            player_id: player.id.clone(),
            owner_team_id: owner_team_id.to_string(),
            is_user_owned,
            score,
            fee: suggested_incoming_fee(current_date, player),
            position_group_index: position_group_index(&player.natural_position),
            owner_reputation: team_reputation.get(owner_team_id).copied().unwrap_or(0),
            pending_offer_clubs,
        });
    }
    // Highest appeal first; a stable sort preserves the original ordering among
    // equally appealing targets, so selection is unchanged.
    shortlist.sort_by(|a, b| b.score.cmp(&a.score));

    for buyer_id in buyer_ids {
        let Some(buyer_team) = game.teams.iter().find(|team| team.id == buyer_id).cloned() else {
            continue;
        };
        let buyer_depths = position_depths.get(&buyer_id).copied().unwrap_or([0; 4]);

        let loan_offer_player_id = if let Some(user_team_id) = user_team_id.as_deref() {
            if new_user_loan_offers_today < MAX_NEW_INCOMING_USER_OFFERS_PER_DAY {
                create_incoming_user_loan_offer_if_any(
                    game,
                    user_team_id,
                    &buyer_id,
                    &buyer_team.name,
                    &today,
                    current_date,
                    &new_loan_offers_per_player,
                )
            } else {
                None
            }
        } else {
            None
        };
        if let Some(player_id) = loan_offer_player_id.as_ref() {
            *new_loan_offers_per_player
                .entry(player_id.clone())
                .or_insert(0) += 1;
            new_user_loan_offers_today += 1;
        }

        // The list is score-sorted, so the first target clearing this club's
        // filters is its highest-appeal eligible signing.
        let chosen = shortlist.iter().find(|target| {
            if target.owner_team_id == buyer_id || moved_player_ids.contains(&target.player_id) {
                return false;
            }
            if loan_offer_player_id.as_deref() == Some(target.player_id.as_str()) {
                return false;
            }
            if target.is_user_owned {
                if target.pending_offer_clubs.contains(&buyer_id)
                    || new_user_offers_today >= MAX_NEW_INCOMING_USER_OFFERS_PER_DAY
                    || new_offers_per_player
                        .get(&target.player_id)
                        .copied()
                        .unwrap_or(0)
                        >= MAX_NEW_INCOMING_OFFERS_PER_USER_PLAYER_PER_DAY
                {
                    return false;
                }
            } else if completed_ai_transfers >= MAX_COMPLETED_AI_TRANSFERS_PER_DAY {
                return false;
            }
            // Clubs only chase players that fit their stature and a position they
            // actually need, so a single star doesn't draw the whole division.
            if !buyer_has_genuine_interest(
                buyer_team.reputation,
                target.owner_reputation,
                buyer_depths[target.position_group_index],
            ) {
                return false;
            }
            buyer_team.transfer_budget >= target.fee as i64
                && buyer_team.finance >= target.fee as i64
        });

        let Some(target) = chosen else {
            continue;
        };
        let candidate = MarketCandidate {
            player_id: target.player_id.clone(),
            owner_team_id: target.owner_team_id.clone(),
            score: target.score,
            fee: target.fee,
        };

        if Some(candidate.owner_team_id.as_str()) == user_team_id.as_deref() {
            create_incoming_user_offer(game, &candidate, &buyer_id, &buyer_team.name, &today);
            *new_offers_per_player
                .entry(candidate.player_id.clone())
                .or_insert(0) += 1;
            new_user_offers_today += 1;
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

fn create_incoming_user_loan_offer_if_any(
    game: &mut Game,
    user_team_id: &str,
    buyer_id: &str,
    buyer_name: &str,
    today: &str,
    current_date: NaiveDate,
    new_loan_offers_per_player: &std::collections::HashMap<String, usize>,
) -> Option<String> {
    let candidate = game
        .players
        .iter()
        .filter(|player| player.team_id.as_deref() == Some(user_team_id))
        .filter(|player| !has_open_loan_offer_from_club(player, buyer_id))
        .filter(|player| {
            new_loan_offers_per_player
                .get(&player.id)
                .copied()
                .unwrap_or(0)
                < MAX_NEW_INCOMING_OFFERS_PER_USER_PLAYER_PER_DAY
        })
        .filter_map(|player| {
            let score = incoming_loan_interest_score(player);
            if score >= 45 {
                default_loan_end_date(current_date, player)?;
                Some(LoanMarketCandidate {
                    player_id: player.id.clone(),
                    wage_contribution_pct: suggested_loan_wage_contribution_pct(score, player),
                    buy_option_fee: suggested_loan_buy_option_fee(player),
                    score,
                })
            } else {
                None
            }
        })
        .max_by_key(|candidate| candidate.score);

    let candidate = candidate?;
    let candidate_player_id = candidate.player_id.clone();

    let player = game
        .players
        .iter_mut()
        .find(|player| player.id == candidate_player_id)?;
    let loan_end_date = default_loan_end_date(current_date, player)?;

    let offer_id = upsert_loan_offer(
        player,
        buyer_id,
        user_team_id,
        today,
        &loan_end_date,
        candidate.wage_contribution_pct,
        candidate.buy_option_fee,
        LoanOfferStatus::Pending,
        today,
    );
    let player_name = player.full_name.clone();

    let message = crate::messages::incoming_loan_offer_message(
        &offer_id,
        &candidate_player_id,
        &player_name,
        buyer_name,
        candidate.wage_contribution_pct,
        candidate.buy_option_fee,
        &loan_end_date,
        today,
    );
    game.messages.push(message);
    Some(candidate_player_id)
}

fn should_generate_major_transfer_news(player: &domain::player::Player, fee: u64) -> bool {
    fee >= 1_000_000 || player.market_value >= 1_000_000
}

fn team_name_or_id(game: &Game, team_id: &str) -> String {
    game.teams
        .iter()
        .find(|team| team.id == team_id)
        .map(|team| team.name.clone())
        .unwrap_or_else(|| team_id.to_string())
}

fn parse_valid_loan_end_date(current_date: NaiveDate, end_date: &str) -> Result<NaiveDate, String> {
    let end_date = NaiveDate::parse_from_str(end_date, "%Y-%m-%d")
        .map_err(|_| ERR_INVALID_LOAN_END_DATE.to_string())?;
    let loan_days = (end_date - current_date).num_days();

    if !(30..=370).contains(&loan_days) {
        return Err(ERR_INVALID_LOAN_END_DATE.to_string());
    }

    Ok(end_date)
}

fn validate_loan_end_before_contract(
    player: &domain::player::Player,
    loan_end_date: NaiveDate,
) -> Result<(), String> {
    let Some(contract_end) = player.contract_end.as_deref() else {
        return Ok(());
    };
    let contract_end_date = NaiveDate::parse_from_str(contract_end, "%Y-%m-%d")
        .map_err(|_| ERR_INVALID_LOAN_END_DATE.to_string())?;

    if loan_end_date >= contract_end_date {
        return Err(ERR_INVALID_LOAN_END_DATE.to_string());
    }

    Ok(())
}

fn minimum_loan_wage_contribution_pct(
    player: &domain::player::Player,
    owner_team: &domain::team::Team,
) -> u8 {
    if owner_team.starting_xi_ids.iter().any(|id| id == &player.id) {
        return 90;
    }

    if player.stats.appearances <= 5 || player.potential >= player.ovr.saturating_add(8) {
        50
    } else if player.ovr >= 72 {
        75
    } else {
        60
    }
}

fn minimum_loan_buy_option_fee(
    player: &domain::player::Player,
    owner_team: &domain::team::Team,
) -> u64 {
    let mut multiplier: f64 = if player.loan_listed { 1.0 } else { 1.2 };

    match infer_player_importance(player, owner_team) {
        PlayerImportance::Key => multiplier += 0.25,
        PlayerImportance::Regular => multiplier += 0.1,
        PlayerImportance::Fringe => multiplier -= 0.05,
    }

    if player.potential >= player.ovr.saturating_add(10) {
        multiplier += 0.2;
    }

    if player.stats.appearances <= 3 {
        multiplier -= 0.05;
    }

    round_transfer_fee(((player.market_value as f64) * multiplier.clamp(0.85, 1.65)).round() as u64)
}

fn acceptable_loan_buy_option(
    player: &domain::player::Player,
    owner_team: &domain::team::Team,
    buy_option_fee: Option<u64>,
) -> bool {
    buy_option_fee
        .map(|fee| fee >= minimum_loan_buy_option_fee(player, owner_team))
        .unwrap_or(true)
}

fn loan_borrower_wage_ceiling(
    player: &domain::player::Player,
    borrower_team: &domain::team::Team,
    offer: &domain::player::LoanOffer,
) -> u8 {
    let mut ceiling = i16::from(offer.wage_contribution_pct);

    if player.potential >= player.ovr.saturating_add(10) {
        ceiling += 30;
    } else if player.ovr >= 72 {
        ceiling += 24;
    } else if player.potential >= player.ovr.saturating_add(6) {
        ceiling += 20;
    } else {
        ceiling += 14;
    }

    if borrower_team.finance >= 5_000_000 {
        ceiling += 8;
    }

    if player.wage <= 750_000 {
        ceiling += 6;
    }

    ceiling.clamp(i16::from(offer.wage_contribution_pct), 100) as u8
}

fn loan_borrower_buy_option_ceiling(player: &domain::player::Player) -> u64 {
    let multiplier = if player.potential >= player.ovr.saturating_add(10) {
        1.4
    } else if player.ovr >= 72 {
        1.25
    } else {
        1.15
    };

    round_transfer_fee(((player.market_value as f64) * multiplier).round() as u64)
}

fn loan_offer_outcome(
    decision: LoanOfferDecision,
    offer_id: String,
    suggested_wage_contribution_pct: Option<u8>,
    suggested_end_date: Option<String>,
    suggested_buy_option_fee: Option<u64>,
    is_terminal: bool,
) -> LoanOfferOutcome {
    LoanOfferOutcome {
        decision,
        offer_id,
        suggested_wage_contribution_pct,
        suggested_end_date,
        suggested_buy_option_fee,
        is_terminal,
    }
}

/// Submit a loan offer from the user's team for a loan-listed player.
pub fn make_loan_offer(
    game: &mut Game,
    player_id: &str,
    end_date: &str,
    wage_contribution_pct: u8,
    buy_option_fee: Option<u64>,
) -> Result<LoanOfferOutcome, String> {
    expire_stale_loan_offers(game);

    if wage_contribution_pct > 100 {
        return Err(ERR_INVALID_LOAN_WAGE_CONTRIBUTION.into());
    }

    if buy_option_fee == Some(0) {
        return Err(ERR_INVALID_LOAN_BUY_OPTION.into());
    }

    let current_date = game.clock.current_date.date_naive();
    let registration_date = loan_registration_date(game)?;
    let register_immediately = registration_date == current_date;
    let end_date = parse_valid_loan_end_date(registration_date, end_date)?;
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let start_date_string = registration_date.format("%Y-%m-%d").to_string();
    let end_date_string = end_date.format("%Y-%m-%d").to_string();

    let user_team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("be.error.noTeamAssigned")?;

    let player = game
        .players
        .iter()
        .find(|player| player.id == player_id)
        .ok_or("be.error.playerNotFound")?;

    if player.team_id.as_deref() == Some(&user_team_id) {
        return Err(ERR_CANNOT_BID_ON_OWN_PLAYER.into());
    }

    if player_has_pending_registration(player) {
        return Err(ERR_PLAYER_ALREADY_LOANED.into());
    }

    if !player.loan_listed {
        return Err(ERR_PLAYER_NOT_LOAN_LISTED.into());
    }

    validate_loan_end_before_contract(player, end_date)?;

    let owner_team_id = player.team_id.clone().ok_or(ERR_PLAYER_HAS_NO_TEAM)?;
    let owner_team = game
        .teams
        .iter()
        .find(|team| team.id == owner_team_id)
        .ok_or("be.error.teamNotFound")?;
    let minimum_contribution = minimum_loan_wage_contribution_pct(player, owner_team);
    let buy_option_accepted = acceptable_loan_buy_option(player, owner_team, buy_option_fee);
    let adjusted_minimum_contribution = if buy_option_fee.is_some() && buy_option_accepted {
        minimum_contribution.saturating_sub(10)
    } else {
        minimum_contribution
    };
    let accepted = wage_contribution_pct >= adjusted_minimum_contribution && buy_option_accepted;

    if accepted {
        validate_loan_borrower_affordability(game, &user_team_id, player, wage_contribution_pct)?;
    }

    let status = if accepted {
        if register_immediately {
            LoanOfferStatus::Accepted
        } else {
            LoanOfferStatus::PendingRegistration
        }
    } else {
        LoanOfferStatus::Rejected
    };

    let offer_id = {
        let player = game
            .players
            .iter_mut()
            .find(|player| player.id == player_id)
            .ok_or("be.error.playerNotFound")?;
        upsert_loan_offer(
            player,
            &user_team_id,
            &owner_team_id,
            &start_date_string,
            &end_date_string,
            wage_contribution_pct,
            buy_option_fee,
            status,
            &today,
        )
    };

    if accepted {
        if register_immediately {
            execute_loan(
                game,
                player_id,
                &owner_team_id,
                &user_team_id,
                &start_date_string,
                &end_date_string,
                wage_contribution_pct,
                buy_option_fee,
            )?;
        } else {
            reserve_player_for_pending_loan(game, player_id, &offer_id)?;
        }
    }

    Ok(LoanOfferOutcome {
        decision: if accepted {
            LoanOfferDecision::Accepted
        } else {
            LoanOfferDecision::Rejected
        },
        offer_id,
        suggested_wage_contribution_pct: None,
        suggested_end_date: None,
        suggested_buy_option_fee: None,
        is_terminal: true,
    })
}

/// Respond to an incoming loan offer on one of the user's players.
pub fn respond_to_loan_offer(
    game: &mut Game,
    player_id: &str,
    offer_id: &str,
    accept: bool,
) -> Result<(), String> {
    expire_stale_loan_offers(game);

    let user_team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("be.error.noTeamAssigned")?;

    let player = game
        .players
        .iter()
        .find(|player| player.id == player_id && player.team_id.as_deref() == Some(&user_team_id))
        .ok_or(ERR_PLAYER_NOT_OWNED_BY_USER)?;

    if accept && player_has_active_or_pending_loan(player) {
        return Err(ERR_PLAYER_ALREADY_LOANED.into());
    }

    let offer = player
        .loan_offers
        .iter()
        .find(|offer| offer.id == offer_id && offer.status == LoanOfferStatus::Pending)
        .ok_or(ERR_OFFER_NOT_PENDING)?;

    let from_team_id = offer.from_team_id.clone();
    let wage_contribution_pct = offer.wage_contribution_pct;
    let buy_option_fee = offer.buy_option_fee;
    let offer_end_date = offer.end_date.clone();
    let current_date = game.clock.current_date.date_naive();
    let registration_date = if accept {
        loan_registration_date(game)?
    } else {
        current_date
    };
    let register_immediately = registration_date == current_date;
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let start_date = registration_date.format("%Y-%m-%d").to_string();
    let end_date = if accept {
        let parsed_end_date = parse_valid_loan_end_date(registration_date, &offer_end_date)?;
        validate_loan_end_before_contract(player, parsed_end_date)?;
        parsed_end_date.format("%Y-%m-%d").to_string()
    } else {
        offer_end_date
    };

    if let Some(player) = game
        .players
        .iter_mut()
        .find(|player| player.id == player_id)
        && let Some(offer) = player
            .loan_offers
            .iter_mut()
            .find(|offer| offer.id == offer_id)
    {
        offer.status = if accept {
            if register_immediately {
                LoanOfferStatus::Accepted
            } else {
                LoanOfferStatus::PendingRegistration
            }
        } else {
            LoanOfferStatus::Rejected
        };
        offer.start_date = start_date.clone();
        offer.date = today.clone();
    }

    if accept {
        if register_immediately {
            execute_loan(
                game,
                player_id,
                &user_team_id,
                &from_team_id,
                &start_date,
                &end_date,
                wage_contribution_pct,
                buy_option_fee,
            )?;
        } else {
            reserve_player_for_pending_loan(game, player_id, offer_id)?;
        }
    }

    Ok(())
}

/// Counter an incoming loan offer on one of the user's players.
pub fn counter_loan_offer(
    game: &mut Game,
    player_id: &str,
    offer_id: &str,
    end_date: &str,
    wage_contribution_pct: u8,
    buy_option_fee: Option<u64>,
) -> Result<LoanOfferOutcome, String> {
    expire_stale_loan_offers(game);

    if wage_contribution_pct > 100 {
        return Err(ERR_INVALID_LOAN_WAGE_CONTRIBUTION.into());
    }

    if buy_option_fee == Some(0) {
        return Err(ERR_INVALID_LOAN_BUY_OPTION.into());
    }

    let current_date = game.clock.current_date.date_naive();
    let registration_date = loan_registration_date(game)?;
    let register_immediately = registration_date == current_date;
    let parsed_end_date = parse_valid_loan_end_date(registration_date, end_date)?;
    let requested_end_date = parsed_end_date.format("%Y-%m-%d").to_string();
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let start_date = registration_date.format("%Y-%m-%d").to_string();
    let user_team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("be.error.noTeamAssigned")?;

    let player = game
        .players
        .iter()
        .find(|player| player.id == player_id && player.team_id.as_deref() == Some(&user_team_id))
        .ok_or(ERR_PLAYER_NOT_OWNED_BY_USER)?;

    if player_has_pending_registration(player) {
        return Err(ERR_PLAYER_ALREADY_LOANED.into());
    }

    validate_loan_end_before_contract(player, parsed_end_date)?;

    let offer = player
        .loan_offers
        .iter()
        .find(|offer| offer.id == offer_id && offer.status == LoanOfferStatus::Pending)
        .ok_or(ERR_OFFER_NOT_PENDING)?;

    if offer.from_team_id == user_team_id {
        return Err(ERR_CANNOT_BID_ON_OWN_PLAYER.into());
    }

    if wage_contribution_pct < offer.wage_contribution_pct {
        return Err(ERR_LOAN_COUNTER_MUST_IMPROVE_TERMS.into());
    }

    if let (Some(current_buy_option), Some(requested_buy_option)) =
        (offer.buy_option_fee, buy_option_fee)
        && requested_buy_option < current_buy_option
    {
        return Err(ERR_LOAN_COUNTER_MUST_IMPROVE_TERMS.into());
    }

    if wage_contribution_pct == offer.wage_contribution_pct
        && requested_end_date == offer.end_date
        && buy_option_fee == offer.buy_option_fee
    {
        return Err(ERR_LOAN_COUNTER_MUST_IMPROVE_TERMS.into());
    }

    let borrower_team = game
        .teams
        .iter()
        .find(|team| team.id == offer.from_team_id)
        .ok_or("be.error.teamNotFound")?;
    let borrower_team_id = borrower_team.id.clone();
    let round = offer.negotiation_round.max(1).saturating_add(1);
    let wage_ceiling = loan_borrower_wage_ceiling(player, borrower_team, offer);
    let buy_option_ceiling = loan_borrower_buy_option_ceiling(player);
    let buy_option_accepted = buy_option_fee
        .map(|fee| fee <= buy_option_ceiling)
        .unwrap_or(true);
    let accepted = wage_contribution_pct <= wage_ceiling && buy_option_accepted;
    let counter_wage_window = wage_ceiling.saturating_add(if round >= 3 { 8 } else { 12 });
    let counter_buy_option_window = round_transfer_fee(
        ((buy_option_ceiling as f64) * if round >= 3 { 1.08 } else { 1.15 }).round() as u64,
    );
    let counterable_buy_option = buy_option_fee
        .map(|fee| fee <= counter_buy_option_window)
        .unwrap_or(true);
    let offer_id_string = offer.id.clone();

    if accepted {
        if let Some(player) = game
            .players
            .iter_mut()
            .find(|player| player.id == player_id)
            && let Some(offer) = player
                .loan_offers
                .iter_mut()
                .find(|offer| offer.id == offer_id)
        {
            offer.start_date = start_date.clone();
            offer.end_date = requested_end_date.clone();
            offer.wage_contribution_pct = wage_contribution_pct;
            offer.buy_option_fee = buy_option_fee;
            offer.last_manager_wage_contribution_pct = Some(wage_contribution_pct);
            offer.last_manager_end_date = Some(requested_end_date.clone());
            offer.last_manager_buy_option_fee = buy_option_fee;
            offer.negotiation_round = round;
            offer.suggested_wage_contribution_pct = None;
            offer.suggested_end_date = None;
            offer.suggested_buy_option_fee = None;
            offer.status = if register_immediately {
                LoanOfferStatus::Accepted
            } else {
                LoanOfferStatus::PendingRegistration
            };
            offer.date = today.clone();
        }

        if register_immediately {
            execute_loan(
                game,
                player_id,
                &user_team_id,
                &borrower_team_id,
                &start_date,
                &requested_end_date,
                wage_contribution_pct,
                buy_option_fee,
            )?;
        } else {
            reserve_player_for_pending_loan(game, player_id, offer_id)?;
        }

        return Ok(loan_offer_outcome(
            LoanOfferDecision::Accepted,
            offer_id_string,
            None,
            None,
            None,
            true,
        ));
    }

    if wage_contribution_pct <= counter_wage_window && counterable_buy_option {
        let suggested_wage_contribution_pct = wage_ceiling.max(offer.wage_contribution_pct);
        let suggested_buy_option_fee =
            buy_option_fee.map(|fee| fee.min(buy_option_ceiling).max(50_000));

        if let Some(player) = game
            .players
            .iter_mut()
            .find(|player| player.id == player_id)
            && let Some(offer) = player
                .loan_offers
                .iter_mut()
                .find(|offer| offer.id == offer_id)
        {
            offer.end_date = requested_end_date.clone();
            offer.wage_contribution_pct = suggested_wage_contribution_pct;
            offer.buy_option_fee = suggested_buy_option_fee;
            offer.last_manager_wage_contribution_pct = Some(wage_contribution_pct);
            offer.last_manager_end_date = Some(requested_end_date.clone());
            offer.last_manager_buy_option_fee = buy_option_fee;
            offer.negotiation_round = round;
            offer.suggested_wage_contribution_pct = Some(suggested_wage_contribution_pct);
            offer.suggested_end_date = Some(requested_end_date.clone());
            offer.suggested_buy_option_fee = suggested_buy_option_fee;
            offer.status = LoanOfferStatus::Pending;
            offer.date = today;
        }

        return Ok(loan_offer_outcome(
            LoanOfferDecision::CounterOffer,
            offer_id_string,
            Some(suggested_wage_contribution_pct),
            Some(requested_end_date),
            suggested_buy_option_fee,
            false,
        ));
    }

    if let Some(player) = game
        .players
        .iter_mut()
        .find(|player| player.id == player_id)
        && let Some(offer) = player
            .loan_offers
            .iter_mut()
            .find(|offer| offer.id == offer_id)
    {
        offer.last_manager_wage_contribution_pct = Some(wage_contribution_pct);
        offer.last_manager_end_date = Some(requested_end_date);
        offer.last_manager_buy_option_fee = buy_option_fee;
        offer.negotiation_round = round;
        offer.suggested_wage_contribution_pct = None;
        offer.suggested_end_date = None;
        offer.suggested_buy_option_fee = None;
        offer.status = LoanOfferStatus::Rejected;
        offer.date = today;
    }

    Ok(loan_offer_outcome(
        LoanOfferDecision::Rejected,
        offer_id_string,
        None,
        None,
        None,
        true,
    ))
}

fn complete_loan_buy_option_transfer(
    game: &mut Game,
    player_id: &str,
    buying_team_id: &str,
    parent_team_id: &str,
    fee: u64,
    notify_user: bool,
) -> Result<(), String> {
    if fee == 0 {
        return Err(ERR_INVALID_LOAN_BUY_OPTION.into());
    }

    let player_snapshot = game
        .players
        .iter()
        .find(|player| player.id == player_id)
        .cloned()
        .ok_or("be.error.playerNotFound")?;

    let loan = player_snapshot
        .active_loan
        .clone()
        .ok_or(ERR_LOAN_BUY_OPTION_NOT_AVAILABLE)?;

    if player_snapshot.team_id.as_deref() != Some(buying_team_id)
        || loan.loan_team_id != buying_team_id
        || loan.parent_team_id != parent_team_id
        || loan.buy_option_fee != Some(fee)
    {
        return Err(ERR_LOAN_BUY_OPTION_NOT_AVAILABLE.into());
    }

    let buying_team = game
        .teams
        .iter()
        .find(|team| team.id == buying_team_id)
        .ok_or("be.error.teamNotFound")?;
    let fee_i64 = i64::try_from(fee).map_err(|_| ERR_INSUFFICIENT_FUNDS.to_string())?;
    if buying_team.finance < fee_i64 {
        return Err(ERR_INSUFFICIENT_FUNDS.into());
    }
    if buying_team.transfer_budget < fee_i64 {
        return Err(ERR_TRANSFER_BUDGET_TOO_LOW.into());
    }

    if !game.teams.iter().any(|team| team.id == parent_team_id) {
        return Err("be.error.teamNotFound".into());
    }

    let from_team_name = game
        .teams
        .iter()
        .find(|team| team.id == parent_team_id)
        .map(|team| team.name.clone())
        .unwrap_or_else(|| parent_team_id.to_string());
    let to_team_name = game
        .teams
        .iter()
        .find(|team| team.id == buying_team_id)
        .map(|team| team.name.clone())
        .unwrap_or_else(|| buying_team_id.to_string());
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();

    for team in &mut game.teams {
        if team.id == buying_team_id {
            team.finance -= fee_i64;
            team.transfer_budget -= fee_i64;
        } else if team.id == parent_team_id {
            team.finance += fee_i64;
            team.remove_player_references(player_id);
        } else {
            team.remove_player_references(player_id);
        }
    }

    if let Some(player) = game
        .players
        .iter_mut()
        .find(|player| player.id == player_id)
    {
        player.team_id = Some(buying_team_id.to_string());
        player.transfer_listed = false;
        player.loan_listed = false;
        player.active_loan = None;
        player.movement_history.push(PlayerMovementEntry {
            date: today.clone(),
            kind: PlayerMovementKind::LoanToBuy,
            from_team_id: Some(parent_team_id.to_string()),
            from_team_name: Some(from_team_name.clone()),
            to_team_id: Some(buying_team_id.to_string()),
            to_team_name: Some(to_team_name.clone()),
            fee: Some(fee),
            loan_end_date: Some(loan.end_date),
        });
    }

    if should_generate_major_transfer_news(&player_snapshot, fee) {
        let article_id = format!(
            "transfer_news_{}_{}_{}_{}",
            player_id, parent_team_id, buying_team_id, today
        );
        if !game.news.iter().any(|article| article.id == article_id) {
            game.news.push(crate::news::major_transfer_article(
                &article_id,
                player_id,
                &player_snapshot.full_name,
                parent_team_id,
                &from_team_name,
                buying_team_id,
                &to_team_name,
                fee,
                &today,
            ));
        }
    }

    log_completed_transfer(
        game,
        CompletedTransfer {
            date: today.clone(),
            from_team_id: parent_team_id.to_string(),
            to_team_id: buying_team_id.to_string(),
            player_id: player_id.to_string(),
            fee,
        },
    );

    if notify_user {
        game.messages
            .push(crate::messages::loan_buy_option_exercised_message(
                player_id,
                &player_snapshot.full_name,
                fee,
                &today,
            ));
    }

    Ok(())
}

pub fn exercise_loan_buy_option(game: &mut Game, player_id: &str) -> Result<(), String> {
    expire_stale_loan_offers(game);

    if !transfer_window_is_open(game) {
        return Err(ERR_TRANSFER_WINDOW_CLOSED.into());
    }

    let user_team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("be.error.noTeamAssigned")?;

    let player_snapshot = game
        .players
        .iter()
        .find(|player| player.id == player_id)
        .cloned()
        .ok_or("be.error.playerNotFound")?;

    let loan = player_snapshot
        .active_loan
        .clone()
        .ok_or(ERR_LOAN_BUY_OPTION_NOT_AVAILABLE)?;

    if player_snapshot.team_id.as_deref() != Some(user_team_id.as_str())
        || loan.loan_team_id != user_team_id
    {
        return Err(ERR_LOAN_BUY_OPTION_NOT_AVAILABLE.into());
    }

    if !game.teams.iter().any(|team| team.id == user_team_id) {
        return Err("be.error.managedTeamNotFound".into());
    }

    let fee = loan.buy_option_fee.ok_or(ERR_NO_LOAN_BUY_OPTION)?;
    complete_loan_buy_option_transfer(
        game,
        player_id,
        &user_team_id,
        &loan.parent_team_id,
        fee,
        true,
    )
}

fn ai_should_exercise_loan_buy_option(
    player: &domain::player::Player,
    loan_team: &domain::team::Team,
    fee: u64,
    loan_minutes: u32,
    loan_appearances: u32,
) -> bool {
    if fee == 0 || loan_team.finance < fee as i64 || loan_team.transfer_budget < fee as i64 {
        return false;
    }

    let fair_option_ceiling = round_transfer_fee(((player.market_value as f64) * 1.1) as u64);
    let high_usage_ceiling = round_transfer_fee(((player.market_value as f64) * 1.25) as u64);
    let meaningful_loan_spell = loan_minutes >= 900 || loan_appearances >= 8;

    fee <= fair_option_ceiling || (meaningful_loan_spell && fee <= high_usage_ceiling)
}

fn maybe_exercise_ai_loan_buy_option(game: &mut Game, player_id: &str) -> bool {
    if !transfer_window_is_open(game) {
        return false;
    }

    let user_team_id = game.manager.team_id.as_deref();
    let Some(player_snapshot) = game
        .players
        .iter()
        .find(|player| player.id == player_id)
        .cloned()
    else {
        return false;
    };
    let Some(loan) = player_snapshot.active_loan.clone() else {
        return false;
    };
    if Some(loan.loan_team_id.as_str()) == user_team_id {
        return false;
    }
    let Some(fee) = loan.buy_option_fee else {
        return false;
    };
    let Some(loan_team) = game.teams.iter().find(|team| team.id == loan.loan_team_id) else {
        return false;
    };
    let loan_minutes = loan_development_delta(
        player_snapshot.stats.minutes_played,
        loan.loan_start_minutes,
    );
    let loan_appearances = loan_development_delta(
        player_snapshot.stats.appearances,
        loan.loan_start_appearances,
    );
    if !ai_should_exercise_loan_buy_option(
        &player_snapshot,
        loan_team,
        fee,
        loan_minutes,
        loan_appearances,
    ) {
        return false;
    }

    let notify_user = user_team_id == Some(loan.parent_team_id.as_str());
    complete_loan_buy_option_transfer(
        game,
        player_id,
        &loan.loan_team_id,
        &loan.parent_team_id,
        fee,
        notify_user,
    )
    .is_ok()
}

fn active_loan_days(loan: &ActiveLoan, current_date: NaiveDate) -> Option<i64> {
    let start_date = NaiveDate::parse_from_str(&loan.start_date, "%Y-%m-%d").ok()?;
    Some((current_date - start_date).num_days().max(0))
}

fn loan_development_report_id(player_id: &str, date: &str, final_report: bool) -> String {
    if final_report {
        format!("loan_return_report_{}_{}", player_id, date)
    } else {
        format!("loan_development_{}_{}", player_id, date)
    }
}

fn increase_attribute(value: &mut u8) -> u8 {
    if *value >= 99 {
        0
    } else {
        *value += 1;
        1
    }
}

fn improve_loan_player_attributes(player: &mut domain::player::Player) -> u8 {
    let attributes = &mut player.attributes;
    match player.natural_position.to_group_position() {
        Position::Goalkeeper => {
            increase_attribute(&mut attributes.handling)
                + increase_attribute(&mut attributes.reflexes)
                + increase_attribute(&mut attributes.aerial)
        }
        Position::Defender => {
            increase_attribute(&mut attributes.defending)
                + increase_attribute(&mut attributes.tackling)
                + increase_attribute(&mut attributes.positioning)
        }
        Position::Midfielder => {
            increase_attribute(&mut attributes.passing)
                + increase_attribute(&mut attributes.vision)
                + increase_attribute(&mut attributes.decisions)
        }
        Position::Forward => {
            increase_attribute(&mut attributes.shooting)
                + increase_attribute(&mut attributes.dribbling)
                + increase_attribute(&mut attributes.positioning)
        }
        _ => 0,
    }
}

fn apply_loan_development(
    player: &mut domain::player::Player,
    loan_team_reputation: u32,
    current_year: u32,
    loan_minutes: u32,
    loan_appearances: u32,
) -> (u8, u8, u8) {
    let ovr_before = player.ovr;
    let growth_room = player.potential.saturating_sub(player.ovr);
    if growth_room == 0 {
        return (ovr_before, player.ovr, 0);
    }

    let has_new_loan_football = loan_minutes > 0 || loan_appearances > 0;
    let mut development_cycles: u8 = if loan_minutes >= 900 || loan_appearances >= 8 {
        2
    } else if loan_minutes >= 180
        || loan_appearances >= 2
        || (has_new_loan_football && loan_team_reputation >= 650)
    {
        1
    } else {
        0
    };

    if player.injury.is_some() {
        development_cycles = development_cycles.saturating_sub(1);
    }

    let development_cycles = development_cycles.min(growth_room).min(2);
    let mut attribute_gains = 0;
    for _ in 0..development_cycles {
        attribute_gains += improve_loan_player_attributes(player);
    }

    if attribute_gains > 0 {
        crate::player_rating::refresh_player_derived(player, current_year);
    }

    (ovr_before, player.ovr, attribute_gains)
}

fn loan_development_delta(current_total: u32, reported_total: u32) -> u32 {
    if current_total >= reported_total {
        current_total - reported_total
    } else {
        current_total
    }
}

fn record_loan_development_report(game: &mut Game, player_id: &str, final_report: bool) {
    let current_date = game.clock.current_date.date_naive();
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let report_id = loan_development_report_id(player_id, &today, final_report);
    if game.messages.iter().any(|message| message.id == report_id) {
        return;
    }

    let Some(player_index) = game
        .players
        .iter()
        .position(|player| player.id == player_id)
    else {
        return;
    };
    let Some(loan) = game.players[player_index].active_loan.clone() else {
        return;
    };
    let Some(days_on_loan) = active_loan_days(&loan, current_date) else {
        return;
    };

    let loan_team = game.teams.iter().find(|team| team.id == loan.loan_team_id);
    let loan_team_name = loan_team
        .map(|team| team.name.clone())
        .unwrap_or_else(|| loan.loan_team_id.clone());
    let loan_team_reputation = loan_team.map(|team| team.reputation).unwrap_or(0);
    let current_year = current_date.year() as u32;

    let (player_name, ovr_before, ovr_after, attribute_gains) = {
        let player = &mut game.players[player_index];
        let player_name = player.full_name.clone();
        let reported_minutes = loan.development_reported_minutes;
        let reported_appearances = loan.development_reported_appearances;
        let current_minutes = player.stats.minutes_played;
        let current_appearances = player.stats.appearances;
        let loan_minutes = loan_development_delta(current_minutes, reported_minutes);
        let loan_appearances = loan_development_delta(current_appearances, reported_appearances);
        let (ovr_before, ovr_after, attribute_gains) = apply_loan_development(
            player,
            loan_team_reputation,
            current_year,
            loan_minutes,
            loan_appearances,
        );
        if let Some(active_loan) = player.active_loan.as_mut() {
            active_loan.development_reported_minutes = current_minutes;
            active_loan.development_reported_appearances = current_appearances;
        }
        (player_name, ovr_before, ovr_after, attribute_gains)
    };

    if game.manager.team_id.as_deref() == Some(loan.parent_team_id.as_str()) {
        game.messages
            .push(crate::messages::loan_development_report_message(
                &report_id,
                player_id,
                &player_name,
                &loan_team_name,
                days_on_loan,
                ovr_before,
                ovr_after,
                attribute_gains,
                final_report,
                &today,
            ));
    }
}

pub fn process_loan_development_reports(game: &mut Game) {
    let current_date = game.clock.current_date.date_naive();
    let report_player_ids: Vec<String> = game
        .players
        .iter()
        .filter_map(|player| {
            let loan = player.active_loan.as_ref()?;
            let end_date = NaiveDate::parse_from_str(&loan.end_date, "%Y-%m-%d").ok()?;
            if end_date <= current_date {
                return None;
            }

            let days_on_loan = active_loan_days(loan, current_date)?;
            if days_on_loan > 0 && days_on_loan % LOAN_DEVELOPMENT_REPORT_INTERVAL_DAYS == 0 {
                Some(player.id.clone())
            } else {
                None
            }
        })
        .collect();

    for player_id in report_player_ids {
        record_loan_development_report(game, &player_id, false);
    }
}

pub fn process_loan_returns(game: &mut Game) {
    let current_date = game.clock.current_date.date_naive();
    let returning_player_ids: Vec<String> = game
        .players
        .iter()
        .filter_map(|player| {
            let loan = player.active_loan.as_ref()?;
            let end_date = NaiveDate::parse_from_str(&loan.end_date, "%Y-%m-%d").ok()?;

            if end_date <= current_date {
                Some(player.id.clone())
            } else {
                None
            }
        })
        .collect();

    for player_id in returning_player_ids {
        record_loan_development_report(game, &player_id, true);
        if maybe_exercise_ai_loan_buy_option(game, &player_id) {
            continue;
        }

        let player_snapshot = game
            .players
            .iter()
            .find(|player| player.id == player_id)
            .cloned();
        let loan_snapshot = player_snapshot
            .as_ref()
            .and_then(|player| player.active_loan.clone());
        let movement_context = loan_snapshot.as_ref().map(|loan| {
            (
                loan.loan_team_id.clone(),
                team_name_or_id(game, &loan.loan_team_id),
                loan.parent_team_id.clone(),
                team_name_or_id(game, &loan.parent_team_id),
                loan.end_date.clone(),
            )
        });
        let resolved_jersey_number = match (&player_snapshot, &loan_snapshot) {
            (Some(snap), Some(loan)) => game
                .teams
                .iter()
                .find(|team| team.id == loan.parent_team_id)
                .and_then(|team| crate::roster::resolve_jersey_for(game, snap, team)),
            _ => None,
        };

        for team in &mut game.teams {
            team.remove_player_references(&player_id);
        }

        if let Some(player) = game
            .players
            .iter_mut()
            .find(|player| player.id == player_id)
            && let Some(loan) = player.active_loan.take()
        {
            player.team_id = Some(loan.parent_team_id);
            player.jersey_number = resolved_jersey_number;
            player.loan_listed = false;
            if let Some((
                loan_team_id,
                loan_team_name,
                parent_team_id,
                parent_team_name,
                loan_end_date,
            )) = movement_context
            {
                player.movement_history.push(PlayerMovementEntry {
                    date: game.clock.current_date.format("%Y-%m-%d").to_string(),
                    kind: PlayerMovementKind::LoanReturn,
                    from_team_id: Some(loan_team_id),
                    from_team_name: Some(loan_team_name),
                    to_team_id: Some(parent_team_id),
                    to_team_name: Some(parent_team_name),
                    fee: None,
                    loan_end_date: Some(loan_end_date),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests;
