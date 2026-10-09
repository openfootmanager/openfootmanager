//! Incoming loan negotiations: validate a response before recording or registering it.
use super::*;
use domain::player::LoanOffer;

struct LoanTerms {
    start_date: String,
    end_date: String,
    wage_contribution_pct: u8,
    buy_option_fee: Option<u64>,
}

struct LoanAgreement {
    parent_team_id: String,
    borrower_team_id: String,
    register_immediately: bool,
    terms: LoanTerms,
}

struct LoanCounterEvaluation {
    decision: LoanOfferDecision,
    round: u8,
    wage_contribution_pct: u8,
    buy_option_fee: Option<u64>,
}

impl LoanCounterEvaluation {
    fn outcome(&self, offer_id: &str, end_date: &str) -> LoanOfferOutcome {
        let counter = self.decision == LoanOfferDecision::CounterOffer;
        loan_offer_outcome(
            self.decision.clone(),
            offer_id.to_string(),
            counter.then_some(self.wage_contribution_pct),
            counter.then(|| end_date.to_string()),
            if counter { self.buy_option_fee } else { None },
            !counter,
        )
    }
}

fn owned_player<'a>(
    players: &'a [Player],
    user_team_id: &str,
    player_id: &str,
) -> Result<&'a Player, String> {
    players
        .iter()
        .find(|player| player.id == player_id && player.team_id.as_deref() == Some(user_team_id))
        .ok_or_else(|| ERR_PLAYER_NOT_OWNED_BY_USER.to_string())
}

fn pending_offer<'a>(player: &'a Player, offer_id: &str) -> Result<&'a LoanOffer, String> {
    player
        .loan_offers
        .iter()
        .find(|offer| offer.id == offer_id && offer.status == LoanOfferStatus::Pending)
        .ok_or_else(|| ERR_OFFER_NOT_PENDING.to_string())
}

fn offer_mut<'a>(
    game: &'a mut Game,
    player_id: &str,
    offer_id: &str,
) -> Result<&'a mut LoanOffer, String> {
    game.players
        .iter_mut()
        .find(|player| player.id == player_id)
        .ok_or("be.error.playerNotFound")?
        .loan_offers
        .iter_mut()
        .find(|offer| offer.id == offer_id)
        .ok_or_else(|| ERR_OFFER_NOT_PENDING.to_string())
}

fn register_agreement(
    game: &mut Game,
    player_id: &str,
    offer_id: &str,
    agreement: &LoanAgreement,
) -> Result<(), String> {
    if agreement.register_immediately {
        execute_loan(
            game,
            player_id,
            &agreement.parent_team_id,
            &agreement.borrower_team_id,
            &agreement.terms.start_date,
            &agreement.terms.end_date,
            agreement.terms.wage_contribution_pct,
            agreement.terms.buy_option_fee,
        )?;
    } else {
        reserve_player_for_pending_loan(game, player_id, offer_id)?;
    }
    notify_loan_agreement(game, player_id, offer_id);
    Ok(())
}

fn agreed_status(register_immediately: bool) -> LoanOfferStatus {
    if register_immediately {
        LoanOfferStatus::Accepted
    } else {
        LoanOfferStatus::PendingRegistration
    }
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
    // Preserve ownership/active-loan validation before checking whether the offer is live.
    let player = owned_player(&game.players, &user_team_id, player_id)?;
    if accept && player_has_active_or_pending_loan(player) {
        return Err(ERR_PLAYER_ALREADY_LOANED.into());
    }
    let offer = pending_offer(player, offer_id)?;
    if accept {
        crate::squad_floor::ensure_departure_keeps_floor(game, player_id)?;
    }
    let current_date = game.clock.current_date.date_naive();
    let registration_date = if accept {
        loan_registration_date(game)?
    } else {
        current_date
    };
    let end_date = if accept {
        let parsed = parse_valid_loan_end_date(registration_date, &offer.end_date)?;
        validate_loan_end_before_contract(player, parsed)?;
        parsed.format("%Y-%m-%d").to_string()
    } else {
        offer.end_date.clone()
    };
    let agreement = LoanAgreement {
        parent_team_id: user_team_id,
        borrower_team_id: offer.from_team_id.clone(),
        register_immediately: registration_date == current_date,
        terms: LoanTerms {
            start_date: registration_date.format("%Y-%m-%d").to_string(),
            end_date,
            wage_contribution_pct: offer.wage_contribution_pct,
            buy_option_fee: offer.buy_option_fee,
        },
    };
    // Scheduled agreements retain their registration-day check; immediate loans need it now,
    // before the offer or any roster, wage share or movement is changed.
    if accept && agreement.register_immediately {
        validate_loan_borrower_affordability(
            game,
            &agreement.borrower_team_id,
            player,
            agreement.terms.wage_contribution_pct,
        )?;
    }
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let offer = offer_mut(game, player_id, offer_id)?;
    if accept {
        offer.status = agreed_status(agreement.register_immediately);
    } else {
        close_loan_offer(offer, LoanOfferStatus::Rejected, &today);
    }
    offer.start_date = agreement.terms.start_date.clone();
    // Arrival date stays put; rejection records its outcome in closed_on.
    if accept {
        register_agreement(game, player_id, offer_id, &agreement)?;
    }
    Ok(())
}

fn validate_counter_improves(
    offer: &LoanOffer,
    user_team_id: &str,
    terms: &LoanTerms,
) -> Result<(), String> {
    if offer.from_team_id == user_team_id {
        return Err(ERR_CANNOT_BID_ON_OWN_PLAYER.into());
    }
    if terms.wage_contribution_pct < offer.wage_contribution_pct {
        return Err(ERR_LOAN_COUNTER_MUST_IMPROVE_TERMS.into());
    }
    if let (Some(current), Some(requested)) = (offer.buy_option_fee, terms.buy_option_fee)
        && requested < current
    {
        return Err(ERR_LOAN_COUNTER_MUST_IMPROVE_TERMS.into());
    }
    if terms.wage_contribution_pct == offer.wage_contribution_pct
        && terms.end_date == offer.end_date
        && terms.buy_option_fee == offer.buy_option_fee
    {
        return Err(ERR_LOAN_COUNTER_MUST_IMPROVE_TERMS.into());
    }
    Ok(())
}

fn evaluate_counter(
    player: &Player,
    borrower: &Team,
    offer: &LoanOffer,
    terms: &LoanTerms,
) -> LoanCounterEvaluation {
    let round = offer.negotiation_round.max(1).saturating_add(1);
    let wage_ceiling = loan_borrower_wage_ceiling(player, borrower, offer);
    let buy_option_ceiling = loan_borrower_buy_option_ceiling(player);
    let buy_option_accepted = terms
        .buy_option_fee
        .map(|fee| fee <= buy_option_ceiling)
        .unwrap_or(true);
    if terms.wage_contribution_pct <= wage_ceiling && buy_option_accepted {
        return LoanCounterEvaluation {
            decision: LoanOfferDecision::Accepted,
            round,
            wage_contribution_pct: terms.wage_contribution_pct,
            buy_option_fee: terms.buy_option_fee,
        };
    }
    let counter_wage_window = wage_ceiling.saturating_add(if round >= 3 { 8 } else { 12 });
    let counter_buy_option_window = round_transfer_fee(
        ((buy_option_ceiling as f64) * if round >= 3 { 1.08 } else { 1.15 }).round() as u64,
    );
    let counterable_buy_option = terms
        .buy_option_fee
        .map(|fee| fee <= counter_buy_option_window)
        .unwrap_or(true);
    if terms.wage_contribution_pct <= counter_wage_window && counterable_buy_option {
        return LoanCounterEvaluation {
            decision: LoanOfferDecision::CounterOffer,
            round,
            wage_contribution_pct: wage_ceiling.max(offer.wage_contribution_pct),
            buy_option_fee: terms
                .buy_option_fee
                .map(|fee| fee.min(buy_option_ceiling).max(50_000)),
        };
    }
    LoanCounterEvaluation {
        decision: LoanOfferDecision::Rejected,
        round,
        wage_contribution_pct: terms.wage_contribution_pct,
        buy_option_fee: terms.buy_option_fee,
    }
}

fn record_counter(
    offer: &mut LoanOffer,
    agreement: &LoanAgreement,
    evaluation: &LoanCounterEvaluation,
    today: &str,
) {
    let terms = &agreement.terms;
    offer.last_manager_wage_contribution_pct = Some(terms.wage_contribution_pct);
    offer.last_manager_end_date = Some(terms.end_date.clone());
    offer.last_manager_buy_option_fee = terms.buy_option_fee;
    offer.negotiation_round = evaluation.round;
    offer.suggested_wage_contribution_pct = None;
    offer.suggested_end_date = None;
    offer.suggested_buy_option_fee = None;
    match evaluation.decision {
        LoanOfferDecision::Accepted => {
            offer.start_date = terms.start_date.clone();
            offer.end_date = terms.end_date.clone();
            offer.wage_contribution_pct = terms.wage_contribution_pct;
            offer.buy_option_fee = terms.buy_option_fee;
            offer.status = agreed_status(agreement.register_immediately);
            // Agreement retains the arrival date, matching direct acceptance.
        }
        LoanOfferDecision::CounterOffer => {
            offer.end_date = terms.end_date.clone();
            offer.wage_contribution_pct = evaluation.wage_contribution_pct;
            offer.buy_option_fee = evaluation.buy_option_fee;
            offer.suggested_wage_contribution_pct = Some(evaluation.wage_contribution_pct);
            offer.suggested_end_date = Some(terms.end_date.clone());
            offer.suggested_buy_option_fee = evaluation.buy_option_fee;
            offer.status = LoanOfferStatus::Pending;
            offer.closed_on = None;
            offer.date = today.to_string();
        }
        LoanOfferDecision::Rejected => close_loan_offer(offer, LoanOfferStatus::Rejected, today),
    }
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
    let parsed_end_date = parse_valid_loan_end_date(registration_date, end_date)?;
    let terms = LoanTerms {
        start_date: registration_date.format("%Y-%m-%d").to_string(),
        end_date: parsed_end_date.format("%Y-%m-%d").to_string(),
        wage_contribution_pct,
        buy_option_fee,
    };
    let user_team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("be.error.noTeamAssigned")?;
    // Preserve the original contract/reservation error order before resolving a pending offer.
    let player = owned_player(&game.players, &user_team_id, player_id)?;
    if player_has_pending_registration(player) {
        return Err(ERR_PLAYER_ALREADY_LOANED.into());
    }
    validate_loan_end_before_contract(player, parsed_end_date)?;
    let offer = pending_offer(player, offer_id)?;
    validate_counter_improves(offer, &user_team_id, &terms)?;
    let borrower = game
        .teams
        .iter()
        .find(|team| team.id == offer.from_team_id)
        .ok_or("be.error.teamNotFound")?;
    let evaluation = evaluate_counter(player, borrower, offer, &terms);
    let agreement = LoanAgreement {
        parent_team_id: user_team_id,
        borrower_team_id: borrower.id.clone(),
        register_immediately: registration_date == current_date,
        terms,
    };
    if evaluation.decision == LoanOfferDecision::Accepted {
        crate::squad_floor::ensure_departure_keeps_floor(game, player_id)?;
        if agreement.register_immediately {
            validate_loan_borrower_affordability(
                game,
                &agreement.borrower_team_id,
                player,
                agreement.terms.wage_contribution_pct,
            )?;
        }
    }
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    record_counter(
        offer_mut(game, player_id, offer_id)?,
        &agreement,
        &evaluation,
        &today,
    );
    if evaluation.decision == LoanOfferDecision::Accepted {
        register_agreement(game, player_id, offer_id, &agreement)?;
    }
    Ok(evaluation.outcome(offer_id, &agreement.terms.end_date))
}
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use domain::player::LoanOffer;

    const PLAYER_ID: &str = "player-award";
    const OFFER_ID: &str = "incoming-affordability";
    const END_DATE: &str = "2026-06-30";

    fn incoming_loan_game(borrower_budget: i64) -> Game {
        let mut game = crate::transfers::tests::make_game();
        game.teams[1].wage_budget = borrower_budget;
        game.players[0].stage_wage(100_000);
        game.players[0].loan_listed = true;
        game.players[0].loan_offers.push(LoanOffer {
            id: OFFER_ID.to_string(),
            from_team_id: "team2".to_string(),
            parent_team_id: "team1".to_string(),
            start_date: "2026-01-12".to_string(),
            end_date: END_DATE.to_string(),
            wage_contribution_pct: 50,
            buy_option_fee: None,
            last_manager_wage_contribution_pct: None,
            last_manager_end_date: None,
            last_manager_buy_option_fee: None,
            negotiation_round: 1,
            suggested_wage_contribution_pct: None,
            suggested_end_date: None,
            suggested_buy_option_fee: None,
            status: LoanOfferStatus::Pending,
            date: "2026-01-10".to_string(),
            closed_on: None,
        });
        // The parent retains one more senior in each group than the floor after departure;
        // these unpaid depth players cannot affect the affordability scenario.
        for team_id in ["team1", "team2"] {
            for (group, floor) in crate::squad_floor::MIN_PLAYERS_PER_GROUP {
                for index in 0..=floor {
                    let id = format!("depth-{team_id}-{group:?}-{index}");
                    let mut player = Player::new(
                        id.clone(),
                        id.clone(),
                        id,
                        "1996-01-01".to_string(),
                        "England".to_string(),
                        group.clone(),
                        game.players[0].attributes.clone(),
                    );
                    player.team_id = Some(team_id.to_string());
                    player.stage_contract_end(Some("2031-06-30".to_string()));
                    player.stage_wage(0);
                    game.players.push(player);
                }
            }
        }
        game
    }

    fn game_snapshot(game: &Game) -> serde_json::Value {
        serde_json::to_value(game).unwrap()
    }

    fn accept_counter(game: &mut Game) -> Result<LoanOfferOutcome, String> {
        counter_loan_offer(game, PLAYER_ID, OFFER_ID, END_DATE, 75, None)
    }

    fn assert_registered(game: &Game, share: u8) {
        let player = &game.players[0];
        assert_eq!(player.team_id.as_deref(), Some("team2"));
        assert_eq!(player.loan_offers[0].status, LoanOfferStatus::Accepted);
        let loan = player.active_loan.as_ref().unwrap();
        assert_eq!(loan.parent_team_id, "team1");
        assert_eq!(loan.loan_team_id, "team2");
        assert_eq!(loan.wage_contribution_pct, share);
        assert_eq!(loan.start_date, "2026-01-12");
        assert_eq!(loan.end_date, END_DATE);
    }

    /// Given a fresh AI loan offer and a collapsed borrower budget, when it is accepted in an open window, then registration is refused before any state changes.
    #[test]
    fn accepting_an_unaffordable_ai_loan_leaves_the_game_unchanged() {
        let mut game = incoming_loan_game(100_000);
        game.teams[1].wage_budget = 10_000;
        let before = game_snapshot(&game);
        let error = respond_to_loan_offer(&mut game, PLAYER_ID, OFFER_ID, true).unwrap_err();
        assert_eq!(
            error,
            "be.error.transfers.loanBorrowerCannotAffordWages?budget=10000"
        );
        assert_eq!(game_snapshot(&game), before);
    }

    /// Given an affordable original share but an unaffordable final counter share, when the AI accepts the counter in an open window, then the final terms are refused atomically.
    #[test]
    fn an_accepted_counter_checks_the_final_borrower_wage_share() {
        let mut game = incoming_loan_game(60_000);
        validate_loan_borrower_affordability(&game, "team2", &game.players[0], 50).unwrap();
        let before = game_snapshot(&game);
        let error = accept_counter(&mut game).unwrap_err();
        assert_eq!(
            error,
            "be.error.transfers.loanBorrowerCannotAffordWages?budget=60000"
        );
        assert_eq!(game_snapshot(&game), before);
    }

    /// Given enough wage room for the AI borrower, when an incoming loan is accepted in an open window, then its original terms register normally.
    #[test]
    fn accepting_an_affordable_ai_loan_registers_its_terms() {
        let mut game = incoming_loan_game(100_000);
        respond_to_loan_offer(&mut game, PLAYER_ID, OFFER_ID, true).unwrap();
        assert_registered(&game, 50);
        assert_eq!(calc_wages(&game, "team2"), 50_000);
        assert_eq!(calc_wages(&game, "team1"), 50_000);
    }

    /// Given enough wage room for the final counter, when the AI accepts it, then registration and the negotiation record retain those exact terms.
    #[test]
    fn an_affordable_accepted_counter_registers_and_records_its_terms() {
        let mut game = incoming_loan_game(100_000);
        let outcome = accept_counter(&mut game).unwrap();
        assert_eq!(outcome.decision, LoanOfferDecision::Accepted);
        assert_registered(&game, 75);
        let offer = &game.players[0].loan_offers[0];
        assert_eq!(offer.last_manager_wage_contribution_pct, Some(75));
        assert_eq!(offer.negotiation_round, 2);
        assert_eq!(offer.date, "2026-01-10");
        assert_eq!(calc_wages(&game, "team2"), 75_000);
    }

    /// Given existing borrower loan wages that consume its remaining room, when another incoming loan is accepted, then the borrower cannot overcommit.
    #[test]
    fn accepting_a_loan_counts_the_borrowers_existing_loan_wages() {
        let mut game = incoming_loan_game(40_000);
        game.players[0].stage_wage(30_000);
        let mut loaned = game.players[1].clone();
        loaned.id = "existing-loanee".to_string();
        loaned.team_id = Some("team2".to_string());
        loaned.stage_wage(60_000);
        loaned.active_loan = Some(ActiveLoan {
            parent_team_id: "team1".to_string(),
            loan_team_id: "team2".to_string(),
            start_date: "2026-01-01".to_string(),
            end_date: END_DATE.to_string(),
            wage_contribution_pct: 50,
            buy_option_fee: None,
            loan_start_minutes: 0,
            loan_start_appearances: 0,
            development_reported_minutes: 0,
            development_reported_appearances: 0,
        });
        game.players.push(loaned);
        assert_eq!(calc_wages(&game, "team2"), 30_000);
        let before = game_snapshot(&game);
        let error = respond_to_loan_offer(&mut game, PLAYER_ID, OFFER_ID, true).unwrap_err();
        assert_eq!(
            error,
            "be.error.transfers.loanBorrowerCannotAffordWages?budget=40000"
        );
        assert_eq!(game_snapshot(&game), before);
    }

    /// Given a borrower already over budget but within the existing legacy grace for this loan, when an incoming offer is accepted, then the shared policy still permits it.
    #[test]
    fn accepting_a_loan_preserves_the_existing_legacy_wage_grace() {
        let mut game = incoming_loan_game(10_000);
        game.players[0].stage_wage(40_000);
        game.players.last_mut().unwrap().stage_wage(30_000);
        respond_to_loan_offer(&mut game, PLAYER_ID, OFFER_ID, true).unwrap();
        assert_registered(&game, 50);
        assert_eq!(calc_wages(&game, "team2"), 50_000);
    }

    /// Given a zero-cost incoming loan and no borrower wage budget, when it is accepted, then the borrower can register without an invented minimum share.
    #[test]
    fn accepting_a_zero_share_loan_preserves_the_existing_policy() {
        let mut game = incoming_loan_game(0);
        game.players[0].loan_offers[0].wage_contribution_pct = 0;
        respond_to_loan_offer(&mut game, PLAYER_ID, OFFER_ID, true).unwrap();
        assert_registered(&game, 0);
        assert_eq!(calc_wages(&game, "team2"), 0);
    }

    /// Given an insolvent borrower whose pending offer is unwanted, when the parent rejects it, then rejection does not require affordability.
    #[test]
    fn an_unaffordable_borrowers_offer_can_still_be_rejected() {
        let mut game = incoming_loan_game(0);
        respond_to_loan_offer(&mut game, PLAYER_ID, OFFER_ID, false).unwrap();
        assert_eq!(
            game.players[0].loan_offers[0].status,
            LoanOfferStatus::Rejected
        );
        assert_eq!(
            game.players[0].loan_offers[0].closed_on.as_deref(),
            Some("2026-01-12")
        );
        assert!(game.players[0].active_loan.is_none());
    }

    /// Given an insolvent borrower and a counter outside its bargaining ceiling, when talks continue, then affordability does not prevent a nonregistered counterproposal.
    #[test]
    fn an_unaffordable_borrower_can_receive_a_counterproposal() {
        let mut game = incoming_loan_game(0);
        let outcome =
            counter_loan_offer(&mut game, PLAYER_ID, OFFER_ID, END_DATE, 90, None).unwrap();
        assert_eq!(outcome.decision, LoanOfferDecision::CounterOffer);
        assert_eq!(
            game.players[0].loan_offers[0].status,
            LoanOfferStatus::Pending
        );
        assert!(game.players[0].active_loan.is_none());
    }

    /// Given a closed window and a currently unaffordable AI borrower, when terms are agreed, then registration remains deferred and later rechecks the actual budget.
    #[test]
    fn an_ai_loan_can_be_agreed_before_its_registration_budget_is_checked() {
        let mut game = incoming_loan_game(10_000);
        game.season_context.transfer_window.status = TransferWindowStatus::Closed;
        game.season_context.transfer_window.opens_on = Some("2026-01-15".to_string());
        respond_to_loan_offer(&mut game, PLAYER_ID, OFFER_ID, true).unwrap();
        assert_eq!(
            game.players[0].loan_offers[0].status,
            LoanOfferStatus::PendingRegistration
        );
        assert!(game.players[0].active_loan.is_none());
        game.clock.current_date = Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap();
        game.season_context.transfer_window.status = TransferWindowStatus::Open;
        process_pending_loan_registrations(&mut game);
        assert_eq!(
            game.players[0].loan_offers[0].status,
            LoanOfferStatus::Withdrawn
        );
        assert_eq!(game.players[0].team_id.as_deref(), Some("team1"));
        assert!(game.players[0].active_loan.is_none());
    }

    /// Given a closed window and unaffordable final counter terms, when the AI agrees, then the scheduled path still owns its later budget check.
    #[test]
    fn an_accepted_ai_counter_can_wait_for_the_registration_budget_check() {
        let mut game = incoming_loan_game(10_000);
        game.season_context.transfer_window.status = TransferWindowStatus::Closed;
        game.season_context.transfer_window.opens_on = Some("2026-01-15".to_string());
        assert_eq!(
            accept_counter(&mut game).unwrap().decision,
            LoanOfferDecision::Accepted
        );
        assert_eq!(
            game.players[0].loan_offers[0].status,
            LoanOfferStatus::PendingRegistration
        );
        game.clock.current_date = Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap();
        game.season_context.transfer_window.status = TransferWindowStatus::Open;
        process_pending_loan_registrations(&mut game);
        assert_eq!(
            game.players[0].loan_offers[0].status,
            LoanOfferStatus::Withdrawn
        );
        assert!(game.players[0].active_loan.is_none());
    }

    /// Given a user borrower with the same overcommitted budget, when making a loan offer, then its already-guarded route still refuses the deal.
    #[test]
    fn the_user_borrower_remains_subject_to_the_existing_wage_check() {
        let mut game = incoming_loan_game(100_000);
        game.teams[0].wage_budget = 10_000;
        game.players[0].team_id = Some("team2".to_string());
        game.players[0].loan_offers.clear();
        let before = game_snapshot(&game);
        let error = make_loan_offer(&mut game, PLAYER_ID, END_DATE, 75, None).unwrap_err();
        assert_eq!(
            error,
            "be.error.transfers.loanBorrowerCannotAffordWages?budget=10000"
        );
        assert_eq!(game_snapshot(&game), before);
    }

    /// Given a fresh offer whose borrowing club no longer exists, when it is accepted, then missing-club validation refuses it before recording an agreement.
    #[test]
    fn a_missing_borrower_is_refused_before_acceptance_changes_the_offer() {
        let mut game = incoming_loan_game(100_000);
        game.teams.retain(|team| team.id != "team2");
        let before = game_snapshot(&game);
        assert_eq!(
            respond_to_loan_offer(&mut game, PLAYER_ID, OFFER_ID, true).unwrap_err(),
            "be.error.teamNotFound"
        );
        assert_eq!(game_snapshot(&game), before);
    }

    /// Given a reloaded nondefault loan offer and borrower budget, when the parent accepts it, then save/load does not bypass the shared affordability rule.
    #[test]
    fn accepting_a_reloaded_ai_loan_rechecks_affordability() {
        let game = incoming_loan_game(10_000);
        let mut restored: Game =
            serde_json::from_str(&serde_json::to_string(&game).unwrap()).unwrap();
        assert_eq!(restored.players[0].wage(), 100_000);
        assert_eq!(restored.players[0].loan_offers[0].wage_contribution_pct, 50);
        let before = game_snapshot(&restored);
        assert_eq!(
            respond_to_loan_offer(&mut restored, PLAYER_ID, OFFER_ID, true).unwrap_err(),
            "be.error.transfers.loanBorrowerCannotAffordWages?budget=10000"
        );
        assert_eq!(game_snapshot(&restored), before);
    }

    /// Given a reloaded offer that can fund its original share but not the final counter, when the AI agrees to the counter, then saved data cannot bypass validation.
    #[test]
    fn an_accepted_reloaded_ai_counter_rechecks_the_final_wage_share() {
        let game = incoming_loan_game(60_000);
        let mut restored: Game =
            serde_json::from_str(&serde_json::to_string(&game).unwrap()).unwrap();
        let before = game_snapshot(&restored);
        assert_eq!(
            accept_counter(&mut restored).unwrap_err(),
            "be.error.transfers.loanBorrowerCannotAffordWages?budget=60000"
        );
        assert_eq!(game_snapshot(&restored), before);
    }
}
