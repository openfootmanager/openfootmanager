//! Notices for agreed deals, using the stored final terms and the inbox sent-ledger.

use super::*;
use domain::message::{InboxMessage, MessageCategory, MessageContext, MessagePriority};
use std::collections::HashMap;

pub(super) fn notify_transfer_agreement(
    game: &mut Game,
    player_id: &str,
    offer_id: &str,
    seller_team_id: &str,
) {
    let Some(user_team_id) = game.manager.team_id.as_deref() else {
        return;
    };
    let Some(player) = game.players.iter().find(|player| player.id == player_id) else {
        return;
    };
    let Some(offer) = player
        .transfer_offers
        .iter()
        .find(|offer| offer.id == offer_id)
    else {
        return;
    };
    if user_team_id != seller_team_id && user_team_id != offer.from_team_id {
        return;
    }
    let pending = match offer.status {
        TransferOfferStatus::PendingRegistration => true,
        TransferOfferStatus::Accepted => false,
        _ => return,
    };
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let mut message =
        crate::messages::transfer_complete_message(&player.full_name, offer.fee, &today);
    let phase = if pending { "agreed" } else { "complete" };
    message.id = format!("transfer_{phase}_{player_id}_{offer_id}_{user_team_id}");
    message.context = MessageContext {
        player_id: Some(player_id.to_string()),
        team_id: Some(user_team_id.to_string()),
        ..Default::default()
    };
    message
        .i18n_params
        .insert("buyer".into(), game.team_name_or_id(&offer.from_team_id));
    message
        .i18n_params
        .insert("seller".into(), game.team_name_or_id(seller_team_id));
    if pending {
        let Some(start) = offer.registration_date.as_ref() else {
            return;
        };
        message.subject_key = Some("be.msg.transferAgreed.subject".into());
        message.body_key = Some("be.msg.transferAgreed.body".into());
        message.i18n_params.insert("start".into(), start.clone());
    } else if user_team_id == seller_team_id {
        message.body_key = Some("be.msg.transferComplete.bodySold".into());
    }
    crate::inbox::emit(game, message);
}

pub(super) fn notify_loan_agreement(game: &mut Game, player_id: &str, offer_id: &str) {
    let Some(user_team_id) = game.manager.team_id.as_deref() else {
        return;
    };
    let Some(player) = game.players.iter().find(|player| player.id == player_id) else {
        return;
    };
    let Some(offer) = player.loan_offers.iter().find(|offer| offer.id == offer_id) else {
        return;
    };
    if !matches!(
        offer.status,
        LoanOfferStatus::Accepted | LoanOfferStatus::PendingRegistration
    ) || (user_team_id != offer.parent_team_id && user_team_id != offer.from_team_id)
    {
        return;
    }
    let mut params = HashMap::from([
        ("player".into(), player.full_name.clone()),
        ("parent".into(), game.team_name_or_id(&offer.parent_team_id)),
        ("borrower".into(), game.team_name_or_id(&offer.from_team_id)),
        ("start".into(), offer.start_date.clone()),
        ("end".into(), offer.end_date.clone()),
        (
            "contribution".into(),
            offer.wage_contribution_pct.to_string(),
        ),
    ]);
    let body_key = if let Some(fee) = offer.buy_option_fee {
        params.insert("fee".into(), fee.to_string());
        "be.msg.loanAgreed.bodyWithOption"
    } else {
        "be.msg.loanAgreed.bodyNoOption"
    };
    let message = InboxMessage::new(
        format!("loan_agreed_{player_id}_{offer_id}_{user_team_id}"),
        String::new(),
        String::new(),
        String::new(),
        game.clock.current_date.format("%Y-%m-%d").to_string(),
    )
    .with_category(MessageCategory::Transfer)
    .with_priority(MessagePriority::Normal)
    .with_i18n("be.msg.loanAgreed.subject", body_key, params)
    .with_sender_i18n("be.sender.transferCommittee", "be.role.directorOfFootball")
    .with_context(MessageContext {
        player_id: Some(player_id.to_string()),
        team_id: Some(user_team_id.to_string()),
        ..Default::default()
    });
    crate::inbox::emit(game, message);
}

#[cfg(test)]
mod tests {
    use super::super::tests::{acceptance_game, assert_acceptance_notice, incoming_loan};
    use super::*;

    /// Given a deleted deferred agreement notice, when the save reloads and the same agreement is announced, then the ledger keeps it deleted.
    #[test]
    fn a_deleted_transfer_agreement_stays_deleted_after_reload() {
        let mut game = acceptance_game("team2", true);
        make_transfer_bid(&mut game, "player-award", 2_000_001).unwrap();
        assert_acceptance_notice(&game, "be.msg.transferAgreed.body", "2026-07-02");
        let id = game.messages[0].id.clone();
        let offer_id = game.players[0].transfer_offers[0].id.clone();
        game.messages.clear();
        let mut loaded: Game =
            serde_json::from_str(&serde_json::to_string(&game).unwrap()).unwrap();
        notify_transfer_agreement(&mut loaded, "player-award", &offer_id, "team2");
        assert!(loaded.messages.is_empty());
        assert!(loaded.emitted_events.contains(&id));
    }

    /// Given a deleted loan agreement notice, when the save reloads and the same final terms are announced, then the ledger keeps it deleted.
    #[test]
    fn a_deleted_loan_agreement_stays_deleted_after_reload() {
        let mut game = acceptance_game("team1", true);
        let offer_id = incoming_loan(&mut game);
        respond_to_loan_offer(&mut game, "player-award", &offer_id, true).unwrap();
        assert_acceptance_notice(&game, "be.msg.loanAgreed.bodyWithOption", "2026-07-02");
        let id = game.messages[0].id.clone();
        game.messages.clear();
        let mut loaded: Game =
            serde_json::from_str(&serde_json::to_string(&game).unwrap()).unwrap();
        notify_loan_agreement(&mut loaded, "player-award", &offer_id);
        assert!(loaded.messages.is_empty());
        assert!(loaded.emitted_events.contains(&id));
    }

    /// Given agreed loan terms between AI clubs, when notification is requested, then an unrelated manager receives no message.
    #[test]
    fn an_unrelated_manager_receives_no_loan_agreement() {
        let mut game = acceptance_game("team1", true);
        let id = incoming_loan(&mut game);
        respond_to_loan_offer(&mut game, "player-award", &id, true).unwrap();
        game.messages.clear();
        game.manager.team_id = Some("unrelated".into());
        notify_loan_agreement(&mut game, "player-award", &id);
        assert!(game.messages.is_empty());
    }
}
