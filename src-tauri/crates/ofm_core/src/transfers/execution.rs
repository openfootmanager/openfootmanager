//! Carrying out a transfer or a loan once its terms are agreed.
//!
//! The negotiation that reaches those terms is still in the parent module. By
//! the time anything here runs the deal is settled, and these functions only
//! move the player, the money and the paperwork.

use super::*;

pub(super) fn round_transfer_fee(value: u64) -> u64 {
    if value == 0 {
        return 0;
    }

    value.div_ceil(50_000) * 50_000
}

pub(super) fn build_transfer_feedback(
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

// The eight parameters are the agreed loan terms, and every caller has them as
// separate values already — validated field by field during negotiation, or
// read off a `LoanOffer`. Bundling them into a terms struct here would only
// move the assembly to the call sites. It becomes worth doing if `LoanOffer`
// ever grows a shared terms type the negotiation code can pass straight
// through.
#[allow(clippy::too_many_arguments)]
pub(super) fn execute_loan(
    game: &mut Game,
    player_id: &str,
    parent_team_id: &str,
    loan_team_id: &str,
    start_date: &str,
    end_date: &str,
    wage_contribution_pct: u8,
    buy_option_fee: Option<u64>,
) -> Result<(), String> {
    if parent_team_id == loan_team_id {
        return Err(ERR_CANNOT_BID_ON_OWN_PLAYER.into());
    }
    // First, before anything moves: an agreement struck while the club had
    // players to spare can fall due after it has lost some.
    crate::squad_floor::ensure_departure_keeps_floor(game, player_id)?;

    let player_snapshot = game
        .players
        .iter()
        .find(|player| player.id == player_id)
        .cloned()
        .ok_or("be.error.playerNotFound")?;
    if player_snapshot.active_loan.is_some() {
        return Err(ERR_PLAYER_ALREADY_LOANED.into());
    }

    let parent_team_name = game.team_name_or_id(parent_team_id);
    let loan_team_name = game.team_name_or_id(loan_team_id);

    let resolved_jersey_number = game
        .teams
        .iter()
        .find(|team| team.id == loan_team_id)
        .and_then(|team| crate::roster::resolve_jersey_for(game, &player_snapshot, team));

    for team in &mut game.teams {
        team.remove_player_references(player_id);
    }

    // Captured before the player is borrowed mutably below.
    let closed_on = game.clock.current_date.format("%Y-%m-%d").to_string();

    let player = game
        .players
        .iter_mut()
        .find(|player| player.id == player_id)
        .ok_or("be.error.playerNotFound")?;

    player.team_id = Some(loan_team_id.to_string());
    player.jersey_number = resolved_jersey_number;
    player.transfer_listed = false;
    player.loan_listed = false;
    player.active_loan = Some(ActiveLoan {
        parent_team_id: parent_team_id.to_string(),
        loan_team_id: loan_team_id.to_string(),
        start_date: start_date.to_string(),
        end_date: end_date.to_string(),
        wage_contribution_pct,
        buy_option_fee,
        loan_start_minutes: player.stats.minutes_played,
        loan_start_appearances: player.stats.appearances,
        development_reported_minutes: player.stats.minutes_played,
        development_reported_appearances: player.stats.appearances,
    });
    // A loan is a movement, not a contract: he stays on his parent club's agreement.
    record_movement(
        player,
        PlayerMovementEntry {
            from_team_id: Some(parent_team_id.to_string()),
            from_team_name: Some(parent_team_name.clone()),
            to_team_id: Some(loan_team_id.to_string()),
            to_team_name: Some(loan_team_name.clone()),
            loan_end_date: Some(end_date.to_string()),
            ..PlayerMovementEntry::new(start_date, PlayerMovementKind::LoanStart)
        },
    );

    withdraw_pending_transfer_offers(player, &closed_on);

    for offer in &mut player.loan_offers {
        if matches!(
            offer.status,
            LoanOfferStatus::Pending | LoanOfferStatus::PendingRegistration
        ) {
            super::close_loan_offer(offer, LoanOfferStatus::Withdrawn, &closed_on);
        }
    }

    let article_id = format!(
        "loan_news_{}_{}_{}_{}",
        player_id, parent_team_id, loan_team_id, start_date
    );
    if !game.news.iter().any(|article| article.id == article_id) {
        game.news.push(crate::news::loan_move_article(
            &article_id,
            player_id,
            &player_snapshot.full_name,
            parent_team_id,
            &parent_team_name,
            loan_team_id,
            &loan_team_name,
            end_date,
            start_date,
        ));
    }

    Ok(())
}

pub(super) fn reserve_player_for_pending_loan(
    game: &mut Game,
    player_id: &str,
    accepted_offer_id: &str,
) -> Result<(), String> {
    let closed_on = game.clock.current_date.format("%Y-%m-%d").to_string();
    let player = game
        .players
        .iter_mut()
        .find(|player| player.id == player_id)
        .ok_or("be.error.playerNotFound")?;

    if player.active_loan.is_some() {
        return Err(ERR_PLAYER_ALREADY_LOANED.into());
    }

    player.transfer_listed = false;
    player.loan_listed = false;
    withdraw_pending_transfer_offers(player, &closed_on);
    for offer in &mut player.loan_offers {
        if offer.id != accepted_offer_id && offer.status == LoanOfferStatus::Pending {
            super::close_loan_offer(offer, LoanOfferStatus::Withdrawn, &closed_on);
        }
    }

    Ok(())
}

pub(super) fn reserve_player_for_pending_transfer(
    game: &mut Game,
    player_id: &str,
    _accepted_offer_id: &str,
) -> Result<(), String> {
    let player = game
        .players
        .iter_mut()
        .find(|player| player.id == player_id)
        .ok_or("be.error.playerNotFound")?;

    if player_has_active_or_pending_loan(player) {
        return Err(ERR_PLAYER_ALREADY_LOANED.into());
    }

    Ok(())
}

pub(super) fn transfer_buyer_can_register(game: &Game, buyer_team_id: &str, fee: u64) -> bool {
    let Ok(fee_i64) = i64::try_from(fee) else {
        return false;
    };

    game.teams
        .iter()
        .find(|team| team.id == buyer_team_id)
        .is_some_and(|team| team.finance >= fee_i64 && team.transfer_budget >= fee_i64)
}

pub(super) fn ensure_transfer_cash_postable(
    game: &Game,
    buyer_id: &str,
    seller_id: &str,
    fee: u64,
) -> Result<(), String> {
    let fee_i64 = i64::try_from(fee).map_err(|_| "be.error.finance.amountOverflow".to_string())?;
    let date = game.clock.current_date.date_naive();
    crate::finances::validate_posts(
        game,
        &[
            crate::finances::PostRequest::new(
                buyer_id,
                -fee_i64,
                crate::finances::CashKind::TransferFeeOut,
                date,
            ),
            crate::finances::PostRequest::new(
                seller_id,
                fee_i64,
                crate::finances::CashKind::TransferFeeIn,
                date,
            ),
        ],
    )
}

/// The contract a club would give a player it buys: the buying club, his wage and the
/// day it ends. Its standard terms, the ones a renewal is judged by. The board has not
/// been asked: [`buyers_contract_terms`] asks it, and a preview that only wants to show
/// the figures uses this.
pub(super) fn buyers_standard_terms(
    game: &Game,
    player: &Player,
    buyer_team_id: &str,
) -> Result<(Team, u32, NaiveDate), String> {
    let buyer = game
        .teams
        .iter()
        .find(|team| team.id == buyer_team_id)
        .cloned()
        .ok_or("be.error.teamNotFound")?;
    // No bid carries a wage yet (every `wage_offered` is 0), so none is passed; when one
    // does, it is passed here.
    let (wage, end) =
        standard_contract_terms(player, &buyer, game.clock.current_date.date_naive(), 0)
            .ok_or(ERR_UNABLE_TO_CALCULATE_CONTRACT_END_DATE)?;
    Ok((buyer, wage, end))
}

/// Whether the board would let `buyer` pay `player` his standard wage, for the sweep
/// that asks it of many players for one buyer: the buyer's bill and senior counts come
/// in already worked out, so nothing here scans the world.
pub(super) fn buyer_can_pay_standard_wage(
    player: &Player,
    buyer: &Team,
    facts: &BuyerWageFacts,
    today: NaiveDate,
) -> bool {
    standard_contract_terms(player, buyer, today, 0)
        .is_some_and(|(wage, _)| facts.purchase_verdict(buyer, player, wage).permits())
}

/// [`buyers_standard_terms`], if the board lets the buyer pay them, through the one
/// wage rule. `Err` is the board's refusal, in the same words a renewal gets.
pub(super) fn buyers_contract_terms(
    game: &Game,
    player: &Player,
    buyer_team_id: &str,
) -> Result<(Team, u32, NaiveDate), String> {
    let (buyer, wage, end) = buyers_standard_terms(game, player, buyer_team_id)?;
    if !purchase_wage_policy_verdict(game, &buyer, player, wage).permits() {
        return Err(renewal_wage_policy_error_message(&buyer));
    }
    Ok((buyer, wage, end))
}

/// Whether `buyer_team_id` could be given the contract it would need to buy this
/// player. For the checks made before a deal is agreed, so that refusing afterwards
/// cannot leave one agreed with the player still at his club.
pub(super) fn ensure_buyer_can_pay_standard_wage(
    game: &Game,
    player_id: &str,
    buyer_team_id: &str,
) -> Result<(), String> {
    let player = game
        .players
        .iter()
        .find(|player| player.id == player_id)
        .ok_or("be.error.playerNotFound")?;
    buyers_contract_terms(game, player, buyer_team_id).map(|_| ())
}

/// Transfer a player between teams, adjusting finances.
pub(super) fn execute_transfer(
    game: &mut Game,
    player_id: &str,
    to_team_id: &str,
    from_team_id: &str,
    fee: u64,
) -> Result<(), String> {
    // First, before anything moves: an agreement struck while the club had
    // players to spare can fall due after it has lost some.
    crate::squad_floor::ensure_departure_keeps_floor(game, player_id)?;
    let player_snapshot = game
        .players
        .iter()
        .find(|player| player.id == player_id)
        .cloned()
        .ok_or("be.error.playerNotFound")?;

    if player_has_active_or_pending_loan(&player_snapshot) {
        return Err(ERR_PLAYER_ALREADY_LOANED.into());
    }

    let from_team_name = game.team_name_or_id(from_team_id);
    let to_team_name = game.team_name_or_id(to_team_id);
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

    let resolved_jersey_number = game
        .teams
        .iter()
        .find(|team| team.id == to_team_id)
        .and_then(|team| crate::roster::resolve_jersey_for(game, &player_snapshot, team));

    // The buyer's terms, and the board's say on them, settled before any money moves so
    // that a refusal here cannot leave a half-done transfer.
    let (buying_team, new_wage, new_contract_end) =
        buyers_contract_terms(game, &player_snapshot, to_team_id)?;

    let fee_i64 = i64::try_from(fee).map_err(|_| "be.error.finance.amountOverflow".to_string())?;
    let date = game.clock.current_date.date_naive();
    crate::finances::post_all(
        game,
        &[
            crate::finances::PostRequest::new(
                to_team_id,
                -fee_i64,
                crate::finances::CashKind::TransferFeeOut,
                date,
            ),
            crate::finances::PostRequest::new(
                from_team_id,
                fee_i64,
                crate::finances::CashKind::TransferFeeIn,
                date,
            ),
        ],
    )?;

    // Move player
    if let Some(p) = game.players.iter_mut().find(|p| p.id == player_id) {
        p.team_id = Some(to_team_id.to_string());
        p.jersey_number = resolved_jersey_number;
        p.transfer_listed = false;
        p.loan_listed = false;
        // Joining a club is signing with it: a new contract on the buyer's terms from
        // the day the move happens. Nothing of the seller's contract is carried over.
        record_movement(
            p,
            PlayerMovementEntry {
                from_team_id: Some(from_team_id.to_string()),
                from_team_name: Some(from_team_name.clone()),
                fee: Some(fee),
                ..contract_entry(
                    PlayerMovementKind::PermanentTransfer,
                    date,
                    &buying_team,
                    contract_record(date, new_contract_end, new_wage, ContractSource::Transfer),
                )
            },
        );
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

    // Envelope (transfer_budget) still mutates here. Cash is posted above.
    // Transfer fees do not change season income/expenses (same as develop).
    if let Some(t) = game.teams.iter_mut().find(|t| t.id == to_team_id) {
        t.transfer_budget -= fee_i64;
        if let Some(pos) = t.starting_xi_ids.iter().position(|id| id == player_id) {
            t.starting_xi_ids.remove(pos);
        }
    }

    if let Some(t) = game.teams.iter_mut().find(|t| t.id == from_team_id) {
        t.transfer_budget += fee_i64;
        // A departing player gives up every role, not just his place in the XI. Trimming the XI by
        // hand here used to leave him as the old club's captain and penalty taker.
        t.remove_player_references(player_id);
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

    // Route through the competition log rather than writing straight to `game.league`.
    // `Game::sync_legacy_league` replaces that field with a clone of a competition, so a record
    // written only there is discarded the next time it runs, not merely misfiled. The loan
    // buy-option path has always gone through here.
    log_completed_transfer(
        game,
        CompletedTransfer {
            date: today,
            from_team_id: from_team_id.to_string(),
            to_team_id: to_team_id.to_string(),
            player_id: player_id.to_string(),
            fee,
        },
    );

    Ok(())
}

pub(super) fn finalize_successful_transfer_offer(
    game: &mut Game,
    player_id: &str,
    accepted_offer_id: &str,
) -> Result<(), String> {
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let player = game
        .players
        .iter_mut()
        .find(|player| player.id == player_id)
        .ok_or("be.error.playerNotFound")?;

    for offer in &mut player.transfer_offers {
        if offer.id != accepted_offer_id && offer.status == TransferOfferStatus::Pending {
            close_transfer_offer(offer, TransferOfferStatus::Withdrawn, &today);
        }
    }

    for offer in &mut player.loan_offers {
        if offer.status == LoanOfferStatus::Pending {
            close_loan_offer(offer, LoanOfferStatus::Withdrawn, &today);
        }
    }

    Ok(())
}
pub(super) fn competition_contains_team(
    competition: &domain::league::League,
    team_id: &str,
) -> bool {
    competition
        .participant_ids
        .iter()
        .any(|participant_id| participant_id == team_id)
        || competition
            .standings
            .iter()
            .any(|entry| entry.team_id == team_id)
}
/// Files a completed move in exactly one competition's transfer log, which is what the save and
/// the transfer screens read. `game.league` is a mirror rebuilt from `game.competitions`, so it is
/// the last resort and only serves a legacy save that has no competitions yet.
pub(super) fn log_completed_transfer(game: &mut Game, transfer: CompletedTransfer) {
    // A deal the user's club is part of goes in the user's own competition, because that is the one
    // `sync_legacy_league` copies into `game.league` — the only transfer log the news roundup and
    // the world transfer tab read. Picking by club order instead lets the two disagree: a sale to
    // another division files under the buyer, and a club playing a cup listed before its league
    // files under the cup. Either way the record exists and nothing ever shows it.
    let user_is_involved =
        game.manager.team_id.as_deref().is_some_and(|team_id| {
            team_id == transfer.from_team_id || team_id == transfer.to_team_id
        });

    let target_competition_index = user_is_involved
        .then(|| game.user_competition_index())
        .flatten()
        .or_else(|| {
            game.competitions.iter().position(|competition| {
                competition_contains_team(competition, &transfer.to_team_id)
            })
        })
        .or_else(|| {
            game.competitions.iter().position(|competition| {
                competition_contains_team(competition, &transfer.from_team_id)
            })
        })
        // Neither club plays anywhere the world knows about. Any competition keeps the record;
        // `game.league` does not, so reaching for it here would lose the move on the next sync.
        .or_else(|| (!game.competitions.is_empty()).then_some(0));

    if let Some(index) = target_competition_index {
        game.competitions[index].transfer_log.push(transfer);
        game.sync_legacy_league();
    } else if let Some(league) = &mut game.league {
        league.transfer_log.push(transfer);
    }
}
pub(super) fn should_generate_major_transfer_news(
    player: &domain::player::Player,
    fee: u64,
) -> bool {
    fee >= 1_000_000 || player.market_value >= 1_000_000
}
pub fn process_pending_transfer_registrations(game: &mut Game) {
    if !transfer_window_is_open(game) {
        return;
    }

    let current_date = game.clock.current_date.date_naive();
    let today = current_date.format("%Y-%m-%d").to_string();
    let user_team_id = game.manager.team_id.clone();
    type DueTransferRegistration = (String, String, String, u64);

    let due_registrations: Vec<DueTransferRegistration> = game
        .players
        .iter()
        .flat_map(|player| {
            player.transfer_offers.iter().filter_map(|offer| {
                if offer.status != TransferOfferStatus::PendingRegistration {
                    return None;
                }

                let registration_date = offer.registration_date.as_deref()?;
                let registration_date =
                    NaiveDate::parse_from_str(registration_date, "%Y-%m-%d").ok()?;
                if registration_date > current_date {
                    return None;
                }

                Some((
                    player.id.clone(),
                    offer.id.clone(),
                    offer.from_team_id.clone(),
                    offer.fee,
                ))
            })
        })
        .collect();

    for (player_id, offer_id, buyer_team_id, fee) in due_registrations {
        let player_snapshot = game
            .players
            .iter()
            .find(|player| player.id == player_id)
            .cloned();
        let from_team_id = player_snapshot.as_ref().and_then(|player| {
            player
                .team_id
                .as_deref()
                .filter(|team_id| *team_id != buyer_team_id)
                .map(str::to_string)
        });
        use domain::player::TransferRegistrationFailureReason as Failure;
        let registration_result = (|| {
            let player = player_snapshot.as_ref().ok_or(Failure::PlayerUnavailable)?;
            if player_has_active_or_pending_loan(player) {
                return Err(Failure::LoanConflict);
            }
            let from_team_id = from_team_id.as_deref().ok_or(Failure::PlayerUnavailable)?;
            if !transfer_buyer_can_register(game, &buyer_team_id, fee) {
                return Err(Failure::InsufficientFunds);
            }
            execute_transfer(game, &player_id, &buyer_team_id, from_team_id, fee)
                .map_err(|_| Failure::RegistrationBlocked)?;
            finalize_successful_transfer_offer(game, &player_id, &offer_id)
                .map_err(|_| Failure::RegistrationBlocked)
        })();
        let failure_reason = registration_result.err();
        let executed = failure_reason.is_none();

        if executed && user_team_id.as_deref() == Some(buyer_team_id.as_str()) {
            let player_name = game
                .players
                .iter()
                .find(|player| player.id == player_id)
                .map(|player| player.full_name.clone())
                .unwrap_or_default();
            game.messages
                .push(crate::messages::transfer_complete_message(
                    &player_name,
                    fee,
                    &today,
                ));
        }

        if let Some(player) = game
            .players
            .iter_mut()
            .find(|player| player.id == player_id)
            && let Some(offer) = player
                .transfer_offers
                .iter_mut()
                .find(|offer| offer.id == offer_id)
        {
            if executed {
                offer.status = TransferOfferStatus::Accepted;
                offer.registration_date = Some(today.clone());
                offer.registration_failure_reason = None;
                offer.suggested_counter_fee = None;
            } else {
                // The agreement lapsed months after it was struck, so retention has to run from
                // the withdrawal rather than from an arrival date already outside the window.
                close_transfer_offer(offer, TransferOfferStatus::Withdrawn, &today);
                offer.registration_failure_reason = failure_reason;
            }
        }

        if let Some(reason) = failure_reason
            && let Some(player) = player_snapshot.as_ref()
            && let Some(offer) = player
                .transfer_offers
                .iter()
                .find(|offer| offer.id == offer_id)
            && let Some(user_club_id) = user_team_id.as_deref()
            && (user_club_id == buyer_team_id || Some(user_club_id) == player.contract_club_id())
        {
            let message = crate::messages::transfer_registration_failed_message(
                offer,
                player,
                user_club_id,
                &game.team_name_or_id(&buyer_team_id),
                reason,
                &today,
            );
            crate::inbox::emit(game, message);
        }
    }
}
pub fn process_pending_loan_registrations(game: &mut Game) {
    if !transfer_window_is_open(game) {
        return;
    }

    let current_date = game.clock.current_date.date_naive();
    let today = current_date.format("%Y-%m-%d").to_string();
    type DueLoanRegistration = (String, String, String, String, String, u8, Option<u64>);

    let due_registrations: Vec<DueLoanRegistration> = game
        .players
        .iter()
        .flat_map(|player| {
            player.loan_offers.iter().filter_map(|offer| {
                if offer.status != LoanOfferStatus::PendingRegistration {
                    return None;
                }

                let start_date = NaiveDate::parse_from_str(&offer.start_date, "%Y-%m-%d").ok()?;
                if start_date > current_date {
                    return None;
                }

                Some((
                    player.id.clone(),
                    offer.id.clone(),
                    offer.parent_team_id.clone(),
                    offer.from_team_id.clone(),
                    offer.end_date.clone(),
                    offer.wage_contribution_pct,
                    offer.buy_option_fee,
                ))
            })
        })
        .collect();

    for (
        player_id,
        offer_id,
        parent_team_id,
        loan_team_id,
        end_date,
        wage_contribution_pct,
        buy_option_fee,
    ) in due_registrations
    {
        let agreement_is_valid = game
            .players
            .iter()
            .find(|player| player.id == player_id)
            .is_some_and(|player| {
                // Every borrower, not only the user's club, as `transfer_buyer_can_register`
                // does for permanent moves: finances can sink between agreement and window.
                let borrower_can_register = validate_loan_borrower_affordability(
                    game,
                    &loan_team_id,
                    player,
                    wage_contribution_pct,
                )
                .is_ok();

                player.team_id.as_deref() == Some(&parent_team_id)
                    && player.active_loan.is_none()
                    && borrower_can_register
                    && NaiveDate::parse_from_str(&end_date, "%Y-%m-%d")
                        .ok()
                        .is_some_and(|loan_end_date| {
                            loan_end_date > current_date
                                && validate_loan_end_before_contract(player, loan_end_date).is_ok()
                        })
            });

        let executed = agreement_is_valid
            && execute_loan(
                game,
                &player_id,
                &parent_team_id,
                &loan_team_id,
                &today,
                &end_date,
                wage_contribution_pct,
                buy_option_fee,
            )
            .is_ok();

        if let Some(player) = game
            .players
            .iter_mut()
            .find(|player| player.id == player_id)
            && let Some(offer) = player
                .loan_offers
                .iter_mut()
                .find(|offer| offer.id == offer_id)
        {
            if executed {
                offer.status = LoanOfferStatus::Accepted;
                offer.start_date = today.clone();
                // `execute_loan` withdraws every live loan offer on the player, including this
                // one, so the agreement arrives here carrying a closure stamp it did not earn.
                offer.closed_on = None;
            } else {
                // See the permanent path above: retention runs from the withdrawal.
                close_loan_offer(offer, LoanOfferStatus::Withdrawn, &today);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transfers::tests::{make_game, sample_attributes};

    fn pending_transfer_game(user_is_seller: bool) -> Game {
        let mut game = make_game();
        let seller = if user_is_seller { "team1" } else { "team2" };
        let buyer = if user_is_seller { "team2" } else { "team1" };
        game.players[0].team_id = Some(seller.to_string());
        game.players[0].adopt_legacy_contract();
        game.players[0].transfer_offers.push(
            serde_json::from_value(serde_json::json!({
                "id": "registration-offer", "from_team_id": buyer, "fee": 800_000,
                "wage_offered": 0, "status": "PendingRegistration", "date": "2025-12-01",
                "registration_date": "2026-01-12"
            }))
            .unwrap(),
        );
        for (position, count) in [
            (Position::Goalkeeper, 3),
            (Position::Defender, 5),
            (Position::Midfielder, 5),
            (Position::Forward, 3),
        ] {
            for index in 0..count {
                let id = format!("depth-{position:?}-{index}");
                let mut player = Player::new(
                    id.clone(),
                    id.clone(),
                    id,
                    "1998-01-01".to_string(),
                    "England".to_string(),
                    position.clone(),
                    sample_attributes(),
                );
                player.team_id = Some(seller.to_string());
                game.players.push(player);
            }
        }
        game
    }

    fn assert_registration_failure(game: &Game, reason: &str) {
        let offer = &game.players[0].transfer_offers[0];
        assert_eq!(offer.status, TransferOfferStatus::Withdrawn);
        assert_eq!(offer.closed_on.as_deref(), Some("2026-01-12"));
        assert_eq!(
            serde_json::to_value(offer).unwrap()["registration_failure_reason"],
            reason
        );
        let messages: Vec<_> = game
            .messages
            .iter()
            .filter(|message| {
                message.subject_key.as_deref() == Some("be.msg.transferRegistrationFailed.subject")
            })
            .collect();
        assert_eq!(messages.len(), 1);
        assert_eq!(
            messages[0].context.player_id.as_deref(),
            Some("player-award")
        );
        assert_eq!(
            messages[0].context.team_id.as_deref(),
            game.manager.team_id.as_deref()
        );
        assert_eq!(messages[0].i18n_params["player"], "Golden Boot");
        assert_eq!(
            messages[0].i18n_params["buyer"],
            game.team_name_or_id(&offer.from_team_id)
        );
        assert!(messages[0].body_key.as_deref().unwrap().ends_with(reason));
    }

    /// Given an agreed transfer and reduced cash, when registration is due, then the buyer gets a funds explanation without being charged.
    #[test]
    fn a_buyer_with_insufficient_cash_is_told_why_registration_failed() {
        let mut game = pending_transfer_game(false);
        game.teams[0].finance = 799_999;
        process_pending_transfer_registrations(&mut game);
        assert_registration_failure(&game, "InsufficientFunds");
        assert_eq!(game.players[0].team_id.as_deref(), Some("team2"));
        assert_eq!(game.teams[0].finance, 799_999);
        assert_eq!(game.teams[0].transfer_budget, 5_000_000);
    }

    /// Given an agreed transfer and reduced transfer budget, when registration is due, then the buyer gets a funds explanation.
    #[test]
    fn a_buyer_with_insufficient_budget_is_told_why_registration_failed() {
        let mut game = pending_transfer_game(false);
        game.teams[0].transfer_budget = 799_999;
        process_pending_transfer_registrations(&mut game);
        assert_registration_failure(&game, "InsufficientFunds");
        assert_eq!(game.teams[0].finance, 5_000_000);
        assert_eq!(game.teams[0].transfer_budget, 799_999);
    }

    /// Given competing due agreements, when the first consumes the remaining budget, then the second fails with an explicit funds reason.
    #[test]
    fn competing_registrations_explain_the_deal_that_cannot_be_funded() {
        let mut game = pending_transfer_game(false);
        let mut other = game.players[0].clone();
        other.id = "second-player".to_string();
        other.transfer_offers[0].id = "second-offer".to_string();
        game.players.push(other);
        game.teams[0].transfer_budget = 1_000_000;
        process_pending_transfer_registrations(&mut game);
        assert_eq!(
            game.players[0].transfer_offers[0].status,
            TransferOfferStatus::Accepted
        );
        assert_eq!(game.teams[0].transfer_budget, 200_000);
        let failed = game.players.last().unwrap();
        assert_eq!(failed.team_id.as_deref(), Some("team2"));
        assert_eq!(
            serde_json::to_value(&failed.transfer_offers[0]).unwrap()["registration_failure_reason"],
            "InsufficientFunds"
        );
        assert_eq!(
            game.messages
                .iter()
                .filter(|m| m.subject_key.as_deref()
                    == Some("be.msg.transferRegistrationFailed.subject"))
                .count(),
            1
        );
    }

    /// Given an agreed transfer and an active loan, when registration is due, then the manager gets a loan-conflict explanation.
    #[test]
    fn an_active_loan_explains_a_failed_transfer_registration() {
        let mut game = pending_transfer_game(false);
        game.players[0].active_loan = Some(ActiveLoan {
            parent_team_id: "team2".to_string(),
            loan_team_id: "team1".to_string(),
            start_date: "2026-01-01".to_string(),
            end_date: "2026-05-01".to_string(),
            wage_contribution_pct: 50,
            buy_option_fee: None,
            loan_start_minutes: 0,
            loan_start_appearances: 0,
            development_reported_minutes: 0,
            development_reported_appearances: 0,
        });
        game.players[0].team_id = Some("team1".to_string());
        process_pending_transfer_registrations(&mut game);
        assert_registration_failure(&game, "LoanConflict");
        assert!(game.players[0].active_loan.is_some());
    }

    /// Given an agreed transfer and a pending loan, when registration is due, then the manager gets a loan-conflict explanation.
    #[test]
    fn a_pending_loan_explains_a_failed_transfer_registration() {
        let mut game = pending_transfer_game(false);
        game.players[0].loan_offers.push(
            serde_json::from_value(serde_json::json!({
                "id":"pending-loan", "from_team_id":"team1", "parent_team_id":"team2",
                "start_date":"2026-01-12", "end_date":"2026-05-01", "wage_contribution_pct":50,
                "status":"PendingRegistration", "date":"2025-12-01"
            }))
            .unwrap(),
        );
        process_pending_transfer_registrations(&mut game);
        assert_registration_failure(&game, "LoanConflict");
    }

    /// Given a player no longer at a selling club, when registration is due, then the manager gets an availability explanation.
    #[test]
    fn an_unavailable_player_explains_a_failed_transfer_registration() {
        let mut game = pending_transfer_game(false);
        game.players[0].team_id = None;
        process_pending_transfer_registrations(&mut game);
        assert_registration_failure(&game, "PlayerUnavailable");
        assert!(game.players[0].team_id.is_none());
    }

    /// Given a seller whose squad can no longer spare the player, when execution refuses registration, then the manager is notified without a debit.
    #[test]
    fn a_final_registration_refusal_is_not_silent() {
        let mut game = pending_transfer_game(false);
        game.players.truncate(1);
        process_pending_transfer_registrations(&mut game);
        assert_registration_failure(&game, "RegistrationBlocked");
        assert_eq!(game.players[0].team_id.as_deref(), Some("team2"));
        assert_eq!(game.teams[0].finance, 5_000_000);
    }

    /// Given the user's agreed sale to an unaffordable AI buyer, when registration is due, then the seller also receives the failure explanation.
    #[test]
    fn the_user_selling_club_is_told_why_registration_failed() {
        let mut game = pending_transfer_game(true);
        game.teams[1].transfer_budget = 0;
        process_pending_transfer_registrations(&mut game);
        assert_registration_failure(&game, "InsufficientFunds");
    }

    /// Given an AI-to-AI agreement, when registration cannot be funded, then the reason is retained without messaging an unrelated manager.
    #[test]
    fn an_ai_registration_failure_keeps_its_reason_without_an_unrelated_message() {
        let mut game = pending_transfer_game(false);
        let mut unrelated_team = game.teams[0].clone();
        unrelated_team.id = "team3".to_string();
        game.teams.push(unrelated_team);
        game.manager.team_id = Some("team3".to_string());
        game.teams[0].finance = 0;
        process_pending_transfer_registrations(&mut game);
        assert_eq!(
            serde_json::to_value(&game.players[0].transfer_offers[0]).unwrap()["registration_failure_reason"],
            "InsufficientFunds"
        );
        assert!(game.messages.is_empty());
    }

    /// Given a failed agreement saved and reloaded, when registration runs again after its inbox message is deleted, then its reason survives and no duplicate is emitted.
    #[test]
    fn a_failed_registration_survives_reload_without_duplicate_notifications() {
        let mut game = pending_transfer_game(false);
        game.teams[0].finance = 0;
        process_pending_transfer_registrations(&mut game);
        assert_registration_failure(&game, "InsufficientFunds");
        game.messages.clear();
        let mut loaded: Game =
            serde_json::from_str(&serde_json::to_string(&game).unwrap()).unwrap();
        process_pending_transfer_registrations(&mut loaded);
        assert!(loaded.messages.is_empty());
        assert_eq!(
            serde_json::to_value(&loaded.players[0].transfer_offers[0]).unwrap()["registration_failure_reason"],
            "InsufficientFunds"
        );
    }

    /// Given an affordable agreement read from JSON without the new reason field, when registration succeeds, then no failure reason or failure notification is added.
    #[test]
    fn a_successful_registration_has_no_failure_reason() {
        let mut game = pending_transfer_game(false);
        process_pending_transfer_registrations(&mut game);
        assert_eq!(
            game.players[0].transfer_offers[0].status,
            TransferOfferStatus::Accepted
        );
        assert!(serde_json::to_value(&game.players[0].transfer_offers[0]).unwrap()["registration_failure_reason"].is_null());
        assert_eq!(game.teams[0].finance, 4_200_000);
        assert_eq!(game.messages.len(), 1);
        assert_eq!(
            game.messages[0].subject_key.as_deref(),
            Some("be.msg.transferComplete.subject")
        );
    }

    /// Given a future registration date, when today's registration sweep runs, then the agreement stays pending and no failure is reported.
    #[test]
    fn a_future_agreement_is_not_reported_as_a_registration_failure() {
        let mut game = pending_transfer_game(false);
        game.players[0].transfer_offers[0].registration_date = Some("2026-01-13".to_string());
        game.teams[0].finance = 0;
        process_pending_transfer_registrations(&mut game);
        assert_eq!(
            game.players[0].transfer_offers[0].status,
            TransferOfferStatus::PendingRegistration
        );
        assert!(game.messages.is_empty());
    }

    /// Given a closed window, when the registration sweep runs, then the agreement stays pending without a failure notice.
    #[test]
    fn a_closed_window_does_not_report_a_registration_failure() {
        let mut game = pending_transfer_game(false);
        game.season_context.transfer_window.status = TransferWindowStatus::Closed;
        game.teams[0].finance = 0;
        process_pending_transfer_registrations(&mut game);
        assert_eq!(
            game.players[0].transfer_offers[0].status,
            TransferOfferStatus::PendingRegistration
        );
        assert!(game.messages.is_empty());
    }
}
