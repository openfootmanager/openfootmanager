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

    let player_snapshot = game
        .players
        .iter()
        .find(|player| player.id == player_id)
        .cloned()
        .ok_or("be.error.playerNotFound")?;
    if player_snapshot.active_loan.is_some() {
        return Err(ERR_PLAYER_ALREADY_LOANED.into());
    }

    let parent_team_name = team_name_or_id(game, parent_team_id);
    let loan_team_name = team_name_or_id(game, loan_team_id);

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
    player.movement_history.push(PlayerMovementEntry {
        date: start_date.to_string(),
        kind: PlayerMovementKind::LoanStart,
        from_team_id: Some(parent_team_id.to_string()),
        from_team_name: Some(parent_team_name.clone()),
        to_team_id: Some(loan_team_id.to_string()),
        to_team_name: Some(loan_team_name.clone()),
        fee: None,
        loan_end_date: Some(end_date.to_string()),
    });

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

/// Transfer a player between teams, adjusting finances.
pub(super) fn execute_transfer(
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
        .ok_or("be.error.playerNotFound")?;

    if player_has_active_or_pending_loan(&player_snapshot) {
        return Err(ERR_PLAYER_ALREADY_LOANED.into());
    }

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

    let resolved_jersey_number = game
        .teams
        .iter()
        .find(|team| team.id == to_team_id)
        .and_then(|team| crate::roster::resolve_jersey_for(game, &player_snapshot, team));

    // Move player
    if let Some(p) = game.players.iter_mut().find(|p| p.id == player_id) {
        p.team_id = Some(to_team_id.to_string());
        p.jersey_number = resolved_jersey_number;
        p.transfer_listed = false;
        p.loan_listed = false;
        p.movement_history.push(PlayerMovementEntry {
            date: today.clone(),
            kind: PlayerMovementKind::PermanentTransfer,
            from_team_id: Some(from_team_id.to_string()),
            from_team_name: Some(from_team_name.clone()),
            to_team_id: Some(to_team_id.to_string()),
            to_team_name: Some(to_team_name.clone()),
            fee: Some(fee),
            loan_end_date: None,
        });
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
        // Also debit the transfer budget so the cumulative envelope shrinks
        // as bids complete. `end_of_season` refills the envelope from finance
        // at the next season rollover; without this line the budget only ever
        // gated the *first* purchase and was silently uncapped thereafter.
        t.transfer_budget -= fee as i64;
        // Remove from starting XI if player was there
        if let Some(pos) = t.starting_xi_ids.iter().position(|id| id == player_id) {
            t.starting_xi_ids.remove(pos);
        }
    }

    // Credit selling team. Sales replenish the current-season transfer envelope;
    // end-of-season still recalculates next season's budget from finance.
    if let Some(t) = game.teams.iter_mut().find(|t| t.id == from_team_id) {
        t.finance += fee as i64;
        t.transfer_budget += fee as i64;
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
pub(super) fn log_completed_transfer(game: &mut Game, transfer: CompletedTransfer) {
    let target_competition_index = game
        .competitions
        .iter()
        .position(|competition| competition_contains_team(competition, &transfer.to_team_id))
        .or_else(|| {
            game.competitions.iter().position(|competition| {
                competition_contains_team(competition, &transfer.from_team_id)
            })
        })
        .or_else(|| (game.competitions.len() == 1).then_some(0));

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
pub(super) fn team_name_or_id(game: &Game, team_id: &str) -> String {
    game.teams
        .iter()
        .find(|team| team.id == team_id)
        .map(|team| team.name.clone())
        .unwrap_or_else(|| team_id.to_string())
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
        let agreement_is_valid = player_snapshot.as_ref().is_some_and(|player| {
            player.active_loan.is_none()
                && !has_pending_loan_registration(player)
                && from_team_id.is_some()
                && transfer_buyer_can_register(game, &buyer_team_id, fee)
        });

        let executed = if agreement_is_valid {
            if let Some(from_team_id) = from_team_id.as_deref() {
                if execute_transfer(game, &player_id, &buyer_team_id, from_team_id, fee).is_ok() {
                    finalize_successful_transfer_offer(game, &player_id, &offer_id).is_ok()
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

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
                offer.suggested_counter_fee = None;
            } else {
                // The agreement lapsed months after it was struck, so retention has to run from
                // the withdrawal rather than from an arrival date already outside the window.
                close_transfer_offer(offer, TransferOfferStatus::Withdrawn, &today);
            }
        }
    }
}
pub fn process_pending_loan_registrations(game: &mut Game) {
    if !transfer_window_is_open(game) {
        return;
    }

    let current_date = game.clock.current_date.date_naive();
    let today = current_date.format("%Y-%m-%d").to_string();
    let user_team_id = game.manager.team_id.clone();
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
                let borrower_can_register = user_team_id.as_deref() != Some(loan_team_id.as_str())
                    || validate_loan_borrower_affordability(
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
