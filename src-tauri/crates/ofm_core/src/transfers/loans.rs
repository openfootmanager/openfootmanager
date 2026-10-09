//! Loans: opening talks, answering them, and everything that happens while one runs.
//!
//! A loan is a longer-lived thing than a transfer — it has a middle as well as a start and
//! an end, so the development reports, the return, and the buy option live here alongside
//! the negotiation itself.

use super::*;

mod incoming;
pub use incoming::{counter_loan_offer, respond_to_loan_offer};

pub(crate) fn loan_wage_share(player: &domain::player::Player, wage_contribution_pct: u8) -> i64 {
    (i64::from(player.wage()) * i64::from(wage_contribution_pct)) / 100
}
pub(crate) fn validate_loan_borrower_affordability(
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
    let current_wage_bill = calc_wages(game, borrower_team_id);
    let projected_wage_bill = current_wage_bill.saturating_add(projected_wage_share);

    if !wage_policy_allows_projection(borrower_team, current_wage_bill, projected_wage_bill) {
        return Err(renewal_wage_policy_error_message(borrower_team));
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
                    && player.contract_end().is_some_and(|contract_end| {
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
pub(crate) fn incoming_loan_interest_score(player: &domain::player::Player) -> i32 {
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
pub(crate) fn suggested_loan_wage_contribution_pct(
    score: i32,
    player: &domain::player::Player,
) -> u8 {
    if score >= 70 || player.wage() <= 150_000 {
        100
    } else if score >= 60 {
        75
    } else {
        50
    }
}
pub(crate) fn suggested_loan_buy_option_fee(player: &domain::player::Player) -> Option<u64> {
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
pub(crate) fn default_loan_end_date(
    current_date: NaiveDate,
    player: &domain::player::Player,
) -> Option<String> {
    let minimum_end_date = current_date + Duration::days(30);
    let default_end_date = current_date + Duration::days(180);
    let end_date = match player.contract_end() {
        Some(contract_end) => {
            let contract_end_date = NaiveDate::parse_from_str(contract_end, "%Y-%m-%d").ok()?;
            let latest_loan_end_date = contract_end_date;
            if latest_loan_end_date < minimum_end_date {
                return None;
            }
            std::cmp::min(default_end_date, latest_loan_end_date)
        }
        None => default_end_date,
    };

    Some(end_date.format("%Y-%m-%d").to_string())
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn upsert_loan_offer(
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
    // As in `upsert_transfer_offer`: an outgoing loan approach can be turned down outright, and a
    // closing status must keep the arrival date and record the ending.
    let closing = matches!(
        status,
        LoanOfferStatus::Rejected | LoanOfferStatus::Withdrawn
    );

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
        if closing {
            offer.closed_on = Some(date.to_string());
        } else {
            offer.date = date.to_string();
            offer.closed_on = None;
        }
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
        closed_on: closing.then(|| date.to_string()),
    });
    offer_id
}
pub(crate) fn create_incoming_user_loan_offer_if_any(
    game: &mut Game,
    user_team_id: &str,
    buyer_id: &str,
    buyer_name: &str,
    today: &str,
    current_date: NaiveDate,
    budget: IncomingOfferBudget<'_>,
) -> Option<String> {
    let candidate = game
        .players
        .iter()
        .filter(|player| player.team_id.as_deref() == Some(user_team_id))
        .filter(|player| {
            budget.accepts(
                &player.id,
                buyer_id,
                MAX_NEW_INCOMING_OFFERS_PER_USER_PLAYER_PER_DAY,
            )
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
pub(crate) fn parse_valid_loan_end_date(
    current_date: NaiveDate,
    end_date: &str,
) -> Result<NaiveDate, String> {
    let end_date = NaiveDate::parse_from_str(end_date, "%Y-%m-%d")
        .map_err(|_| ERR_INVALID_LOAN_END_DATE.to_string())?;
    let loan_days = (end_date - current_date).num_days();

    if !(30..=370).contains(&loan_days) {
        return Err(ERR_INVALID_LOAN_END_DATE.to_string());
    }

    Ok(end_date)
}
pub(crate) fn validate_loan_end_before_contract(
    player: &domain::player::Player,
    loan_end_date: NaiveDate,
) -> Result<(), String> {
    let Some(contract_end) = player.contract_end() else {
        return Ok(());
    };
    let contract_end_date = NaiveDate::parse_from_str(contract_end, "%Y-%m-%d")
        .map_err(|_| ERR_INVALID_LOAN_END_DATE.to_string())?;

    if loan_end_date > contract_end_date {
        return Err(ERR_INVALID_LOAN_END_DATE.to_string());
    }

    Ok(())
}
pub(crate) fn minimum_loan_wage_contribution_pct(
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
pub(crate) fn minimum_loan_buy_option_fee(
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
pub(crate) fn acceptable_loan_buy_option(
    player: &domain::player::Player,
    owner_team: &domain::team::Team,
    buy_option_fee: Option<u64>,
) -> bool {
    buy_option_fee
        .map(|fee| fee >= minimum_loan_buy_option_fee(player, owner_team))
        .unwrap_or(true)
}
pub(crate) fn loan_borrower_wage_ceiling(
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

    if player.wage() <= 750_000 {
        ceiling += 6;
    }

    ceiling.clamp(i16::from(offer.wage_contribution_pct), 100) as u8
}
pub(crate) fn loan_borrower_buy_option_ceiling(player: &domain::player::Player) -> u64 {
    let multiplier = if player.potential >= player.ovr.saturating_add(10) {
        1.4
    } else if player.ovr >= 72 {
        1.25
    } else {
        1.15
    };

    round_transfer_fee(((player.market_value as f64) * multiplier).round() as u64)
}
pub(crate) fn loan_offer_outcome(
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
        // Before the offer is marked agreed, like every other refusal here.
        crate::squad_floor::ensure_departure_keeps_floor(game, player_id)?;
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
pub(crate) fn complete_loan_buy_option_transfer(
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
    let date = game.clock.current_date.date_naive();
    // The buyer's terms, and the board's say on them, settled before any money moves.
    // The option names a fee and nothing else.
    let (buying_club, new_wage, new_contract_end) =
        buyers_contract_terms(game, &player_snapshot, buying_team_id)?;
    crate::finances::post_all(
        game,
        &[
            crate::finances::PostRequest::new(
                buying_team_id,
                -fee_i64,
                crate::finances::CashKind::TransferFeeOut,
                date,
            ),
            crate::finances::PostRequest::new(
                parent_team_id,
                fee_i64,
                crate::finances::CashKind::TransferFeeIn,
                date,
            ),
        ],
    )?;

    for team in &mut game.teams {
        if team.id == buying_team_id {
            team.transfer_budget -= fee_i64;
        } else if team.id == parent_team_id {
            // Same rule as a permanent sale: the proceeds replenish this season's transfer
            // envelope, not only the balance. The cash reaches the parent through the journal
            // post above; crediting one and not the other left a club that sold through an
            // option unable to spend what it had just earned.
            team.transfer_budget += fee_i64;
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
        // Buying the player is a new contract on the buyer's terms from today. While on
        // loan the contract stayed the parent club's, so this is the moment it changes.
        record_movement(
            player,
            PlayerMovementEntry {
                from_team_id: Some(parent_team_id.to_string()),
                from_team_name: Some(from_team_name.clone()),
                fee: Some(fee),
                loan_end_date: Some(loan.end_date),
                ..contract_entry(
                    PlayerMovementKind::LoanToBuy,
                    date,
                    &buying_club,
                    contract_record(date, new_contract_end, new_wage, ContractSource::Transfer),
                )
            },
        );
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
pub(crate) fn ai_should_exercise_loan_buy_option(
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
pub(crate) fn maybe_exercise_ai_loan_buy_option(game: &mut Game, player_id: &str) -> bool {
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
pub(crate) fn active_loan_days(loan: &ActiveLoan, current_date: NaiveDate) -> Option<i64> {
    let start_date = NaiveDate::parse_from_str(&loan.start_date, "%Y-%m-%d").ok()?;
    Some((current_date - start_date).num_days().max(0))
}
pub(crate) fn loan_development_report_id(
    player_id: &str,
    date: &str,
    final_report: bool,
) -> String {
    if final_report {
        format!("loan_return_report_{}_{}", player_id, date)
    } else {
        format!("loan_development_{}_{}", player_id, date)
    }
}
pub(crate) fn increase_attribute(value: &mut u8) -> u8 {
    if *value >= 99 {
        0
    } else {
        *value += 1;
        1
    }
}
pub(crate) fn improve_loan_player_attributes(player: &mut domain::player::Player) -> u8 {
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
pub(crate) fn apply_loan_development(
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
pub(crate) fn loan_development_delta(current_total: u32, reported_total: u32) -> u32 {
    if current_total >= reported_total {
        current_total - reported_total
    } else {
        current_total
    }
}
pub(crate) fn record_loan_development_report(game: &mut Game, player_id: &str, final_report: bool) {
    let current_date = game.clock.current_date.date_naive();
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let report_id = loan_development_report_id(player_id, &today, final_report);
    if crate::inbox::already_emitted(game, &report_id) {
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
        let report = crate::messages::loan_development_report_message(
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
        );
        crate::inbox::emit(game, report);
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
                game.team_name_or_id(&loan.loan_team_id),
                loan.parent_team_id.clone(),
                game.team_name_or_id(&loan.parent_team_id),
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
                record_movement(
                    player,
                    PlayerMovementEntry {
                        from_team_id: Some(loan_team_id),
                        from_team_name: Some(loan_team_name),
                        to_team_id: Some(parent_team_id),
                        to_team_name: Some(parent_team_name),
                        loan_end_date: Some(loan_end_date),
                        ..PlayerMovementEntry::new(
                            game.clock.current_date.format("%Y-%m-%d").to_string(),
                            PlayerMovementKind::LoanReturn,
                        )
                    },
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transfers::tests::{make_game, sample_attributes};
    use chrono::{TimeZone, Utc};

    fn contract_end_loan_game(user_is_parent: bool) -> Game {
        let mut game = make_game();
        let player = &mut game.players[0];
        player.team_id = Some(if user_is_parent { "team1" } else { "team2" }.to_string());
        player.loan_listed = true;
        player.stage_contract_end(Some("2026-04-12".to_string()));
        player.stage_wage(1_000);
        player.adopt_legacy_contract();

        // These scenarios exercise the contract boundary; both clubs can spare a player.
        for team_id in ["team1", "team2"] {
            for (position, count) in [
                (Position::Goalkeeper, 3),
                (Position::Defender, 5),
                (Position::Midfielder, 5),
                (Position::Forward, 3),
            ] {
                for index in 0..count {
                    let id = format!("depth-{team_id}-{position:?}-{index}");
                    let mut player = Player::new(
                        id.clone(),
                        id.clone(),
                        id,
                        "1998-01-01".to_string(),
                        "England".to_string(),
                        position.clone(),
                        sample_attributes(),
                    );
                    player.team_id = Some(team_id.to_string());
                    game.players.push(player);
                }
            }
        }
        game
    }

    fn add_contract_end_loan_offer(game: &mut Game, end_date: &str) {
        game.players[0].loan_offers.push(domain::player::LoanOffer {
            id: "loan-contract-end".to_string(),
            from_team_id: "team2".to_string(),
            parent_team_id: "team1".to_string(),
            start_date: "2026-01-12".to_string(),
            end_date: end_date.to_string(),
            wage_contribution_pct: 50,
            buy_option_fee: None,
            last_manager_wage_contribution_pct: None,
            last_manager_end_date: None,
            last_manager_buy_option_fee: None,
            negotiation_round: 1,
            suggested_wage_contribution_pct: None,
            suggested_end_date: None,
            suggested_buy_option_fee: None,
            status: domain::player::LoanOfferStatus::Pending,
            date: "2026-01-12".to_string(),
            closed_on: None,
        });
    }

    /// Given a short contract, when the AI proposes a loan, then it can end on the contract date.
    #[test]
    fn an_ai_loan_default_can_end_on_the_contract_date() {
        let game = contract_end_loan_game(true);
        assert_eq!(
            super::default_loan_end_date(game.clock.current_date.date_naive(), &game.players[0]),
            Some("2026-04-12".to_string()),
        );
    }

    /// Given exactly thirty contract days left, when the AI proposes a loan, then the minimum term is available.
    #[test]
    fn an_ai_loan_default_allows_exactly_thirty_contract_days() {
        let mut game = make_game();
        game.players[0].stage_contract_end(Some("2026-02-11".to_string()));
        assert_eq!(
            super::default_loan_end_date(game.clock.current_date.date_naive(), &game.players[0]),
            Some("2026-02-11".to_string()),
        );
    }

    /// Given fewer than thirty contract days left, when the AI proposes a loan, then no term is available.
    #[test]
    fn an_ai_loan_default_refuses_fewer_than_thirty_contract_days() {
        let mut game = make_game();
        game.players[0].stage_contract_end(Some("2026-02-10".to_string()));
        assert_eq!(
            super::default_loan_end_date(game.clock.current_date.date_naive(), &game.players[0]),
            None
        );
    }

    /// Given a listed player, when the user bids through the contract date, then the loan registers.
    #[test]
    fn an_outgoing_loan_can_end_on_the_contract_date() {
        let mut game = contract_end_loan_game(false);
        let outcome =
            super::make_loan_offer(&mut game, "player-award", "2026-04-12", 100, None).unwrap();
        assert_eq!(outcome.decision, super::LoanOfferDecision::Accepted);
        assert_eq!(
            game.players[0].active_loan.as_ref().unwrap().end_date,
            "2026-04-12"
        );
    }

    /// Given an incoming offer, when accepted through the contract date, then the AI borrower registers it.
    #[test]
    fn an_incoming_loan_can_end_on_the_contract_date() {
        let mut game = contract_end_loan_game(true);
        add_contract_end_loan_offer(&mut game, "2026-04-12");
        super::respond_to_loan_offer(&mut game, "player-award", "loan-contract-end", true).unwrap();
        assert_eq!(
            game.players[0].active_loan.as_ref().unwrap().end_date,
            "2026-04-12"
        );
    }

    /// Given an incoming offer, when countered through the contract date, then the accepted counter registers.
    #[test]
    fn a_countered_loan_can_end_on_the_contract_date() {
        let mut game = contract_end_loan_game(true);
        add_contract_end_loan_offer(&mut game, "2026-04-01");
        let outcome = super::counter_loan_offer(
            &mut game,
            "player-award",
            "loan-contract-end",
            "2026-04-12",
            60,
            None,
        )
        .unwrap();
        assert_eq!(outcome.decision, super::LoanOfferDecision::Accepted);
        assert_eq!(
            game.players[0].active_loan.as_ref().unwrap().end_date,
            "2026-04-12"
        );
    }

    /// Given a closed window, when a loan ending with the contract is agreed and reloaded,
    /// then it registers once when the window opens with the same end date.
    #[test]
    fn a_pending_contract_end_loan_registers_after_reload() {
        let mut game = contract_end_loan_game(false);
        game.season_context.transfer_window.status = TransferWindowStatus::Closed;
        game.season_context.transfer_window.opens_on = Some("2026-02-01".to_string());
        super::make_loan_offer(&mut game, "player-award", "2026-04-12", 100, None).unwrap();
        assert!(game.players[0].active_loan.is_none());
        let mut game: Game = serde_json::from_str(&serde_json::to_string(&game).unwrap()).unwrap();
        game.clock.current_date = Utc.with_ymd_and_hms(2026, 2, 1, 12, 0, 0).unwrap();
        game.season_context.transfer_window.status = TransferWindowStatus::Open;
        super::process_pending_loan_registrations(&mut game);
        super::process_pending_loan_registrations(&mut game);
        assert_eq!(game.players[0].team_id.as_deref(), Some("team1"));
        assert_eq!(
            game.players[0].active_loan.as_ref().unwrap().end_date,
            "2026-04-12"
        );
        assert_eq!(
            game.players[0].loan_offers[0].status,
            domain::player::LoanOfferStatus::Accepted
        );
        assert_eq!(
            game.players[0]
                .movement_history
                .iter()
                .filter(|entry| { entry.kind == domain::player::PlayerMovementKind::LoanStart })
                .count(),
            1,
        );
    }

    fn open_loan_window_and_register(game: &mut Game) {
        game.clock.current_date = Utc.with_ymd_and_hms(2026, 2, 1, 12, 0, 0).unwrap();
        game.season_context.transfer_window.status = TransferWindowStatus::Open;
        super::process_pending_loan_registrations(game);
        assert_eq!(game.players[0].team_id.as_deref(), Some("team2"));
        assert_eq!(
            game.players[0].active_loan.as_ref().unwrap().end_date,
            "2026-04-12"
        );
    }

    /// Given a closed window and an incoming loan ending with the contract,
    /// when accepted, then it waits for the window and registers with its agreed end.
    #[test]
    fn a_pending_incoming_loan_can_end_on_the_contract_date() {
        let mut game = contract_end_loan_game(true);
        add_contract_end_loan_offer(&mut game, "2026-04-12");
        game.season_context.transfer_window.status = TransferWindowStatus::Closed;
        game.season_context.transfer_window.opens_on = Some("2026-02-01".to_string());
        super::respond_to_loan_offer(&mut game, "player-award", "loan-contract-end", true).unwrap();
        assert!(game.players[0].active_loan.is_none());
        open_loan_window_and_register(&mut game);
    }

    /// Given a closed window and an incoming loan, when countered through the contract date,
    /// then the accepted counter waits for the window and registers with its agreed end.
    #[test]
    fn a_pending_countered_loan_can_end_on_the_contract_date() {
        let mut game = contract_end_loan_game(true);
        add_contract_end_loan_offer(&mut game, "2026-04-01");
        game.season_context.transfer_window.status = TransferWindowStatus::Closed;
        game.season_context.transfer_window.opens_on = Some("2026-02-01".to_string());
        let outcome = super::counter_loan_offer(
            &mut game,
            "player-award",
            "loan-contract-end",
            "2026-04-12",
            60,
            None,
        )
        .unwrap();
        assert_eq!(outcome.decision, super::LoanOfferDecision::Accepted);
        assert!(game.players[0].active_loan.is_none());
        open_loan_window_and_register(&mut game);
    }

    /// Given a saved pending loan ending beyond the contract, when registration runs,
    /// then the agreement is withdrawn and the player stays with the parent.
    #[test]
    fn a_pending_loan_cannot_outlive_the_contract() {
        let mut game = contract_end_loan_game(true);
        add_contract_end_loan_offer(&mut game, "2026-04-13");
        game.players[0].loan_offers[0].status =
            domain::player::LoanOfferStatus::PendingRegistration;
        game.players[0].loan_offers[0].start_date = "2026-02-01".to_string();
        game.clock.current_date = Utc.with_ymd_and_hms(2026, 2, 1, 12, 0, 0).unwrap();
        super::process_pending_loan_registrations(&mut game);
        assert_eq!(game.players[0].team_id.as_deref(), Some("team1"));
        assert!(game.players[0].active_loan.is_none());
        assert_eq!(
            game.players[0].loan_offers[0].status,
            domain::player::LoanOfferStatus::Withdrawn
        );
    }

    /// Given a loan running to the contract date, when the day's loan return precedes expiry,
    /// then the returned player is released from the parent club on the same date.
    #[test]
    fn a_contract_end_loan_returns_then_releases_the_player() {
        let mut game = contract_end_loan_game(false);
        super::make_loan_offer(&mut game, "player-award", "2026-04-12", 100, None).unwrap();
        game.clock.current_date = Utc.with_ymd_and_hms(2026, 4, 12, 12, 0, 0).unwrap();
        super::process_loan_returns(&mut game);
        crate::contracts::process_contract_expiries(&mut game);
        assert_eq!(
            game.players[0]
                .movement_history
                .iter()
                .filter(|entry| entry.kind == PlayerMovementKind::LoanReturn)
                .count(),
            1,
        );
        assert!(game.players[0].team_id.is_none());
        assert!(game.players[0].active_loan.is_none());
        assert!(game.players[0].contract_end().is_none());
    }

    /// Given a requested end after the contract, when bidding, then no offer or loan is created.
    #[test]
    fn an_outgoing_loan_cannot_outlive_the_contract() {
        let mut game = contract_end_loan_game(false);
        assert!(
            super::make_loan_offer(&mut game, "player-award", "2026-04-13", 100, None).is_err()
        );
        assert!(game.players[0].loan_offers.is_empty());
        assert!(game.players[0].active_loan.is_none());
    }

    /// Given an incoming offer ending after the contract, when accepted, then it remains pending.
    #[test]
    fn an_incoming_loan_cannot_outlive_the_contract() {
        let mut game = contract_end_loan_game(true);
        add_contract_end_loan_offer(&mut game, "2026-04-13");
        assert!(
            super::respond_to_loan_offer(&mut game, "player-award", "loan-contract-end", true)
                .is_err()
        );
        assert_eq!(
            game.players[0].loan_offers[0].status,
            domain::player::LoanOfferStatus::Pending
        );
        assert!(game.players[0].active_loan.is_none());
    }

    /// Given an incoming offer, when countered beyond the contract, then its terms remain unchanged.
    #[test]
    fn a_countered_loan_cannot_outlive_the_contract() {
        let mut game = contract_end_loan_game(true);
        add_contract_end_loan_offer(&mut game, "2026-04-01");
        assert!(
            super::counter_loan_offer(
                &mut game,
                "player-award",
                "loan-contract-end",
                "2026-04-13",
                60,
                None
            )
            .is_err()
        );
        assert_eq!(game.players[0].loan_offers[0].end_date, "2026-04-01");
        assert!(game.players[0].active_loan.is_none());
    }
}
