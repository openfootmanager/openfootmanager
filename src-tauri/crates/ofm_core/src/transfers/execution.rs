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

    withdraw_pending_transfer_offers(player);

    for offer in &mut player.loan_offers {
        if matches!(
            offer.status,
            LoanOfferStatus::Pending | LoanOfferStatus::PendingRegistration
        ) {
            offer.status = LoanOfferStatus::Withdrawn;
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
    withdraw_pending_transfer_offers(player);
    for offer in &mut player.loan_offers {
        if offer.id != accepted_offer_id && offer.status == LoanOfferStatus::Pending {
            offer.status = LoanOfferStatus::Withdrawn;
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
