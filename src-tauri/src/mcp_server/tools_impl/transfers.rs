//! MCP tool implementations: transfers

use mcp_results::transfers::{
    BidMade, BidPreview, CounterOffered, FreeAgentOffered, FreeAgentPreview, LoanListingToggled,
    MarketPlayer, OfferAnswered, TransferListingToggled, TransferMarket,
};

use crate::mcp_server::context::McpContext;
use crate::mcp_server::tools_impl::helpers::{
    age_from_dob, format_position, require_game, serde_label,
};
use ofm_core::game::Game;
use ofm_core::player_knowledge::{
    passes_wage_filter, player_knowledge, KnowledgeView, PlayerKnowledge, VisibleWage,
};
use ofm_core::state::Career;
use ofm_core::transfers::TransferBidFinancialProjection;
use std::sync::Arc;

// ─── transfer_toggle_listed ────────────────────────────────────────────────

pub fn transfer_toggle_listed(
    ctx: Arc<McpContext>,
    player_id: String,
) -> Result<TransferListingToggled, String> {
    let game = require_game(&ctx.state_manager)?;
    let player_name = game
        .players
        .iter()
        .find(|p| p.id == player_id)
        .map(|p| p.match_name.clone())
        .unwrap_or_default();

    crate::commands::transfers::toggle_transfer_list_internal(
        &ctx.state_manager,
        Career::Active,
        &player_id,
    )?;

    let game = require_game(&ctx.state_manager)?;
    let is_listed = game
        .players
        .iter()
        .find(|p| p.id == player_id)
        .map(|p| p.transfer_listed)
        .unwrap_or(false);

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(TransferListingToggled {
        player_id,
        player_name,
        listed: is_listed,
    })
}

// ─── transfer_toggle_loan ──────────────────────────────────────────────────

// ─── transfer_toggle_loan ──────────────────────────────────────────────────

pub fn transfer_toggle_loan(
    ctx: Arc<McpContext>,
    player_id: String,
) -> Result<LoanListingToggled, String> {
    let game = require_game(&ctx.state_manager)?;
    let player_name = game
        .players
        .iter()
        .find(|p| p.id == player_id)
        .map(|p| p.match_name.clone())
        .unwrap_or_default();

    crate::commands::transfers::toggle_loan_list_internal(
        &ctx.state_manager,
        Career::Active,
        &player_id,
    )?;

    let game = require_game(&ctx.state_manager)?;
    let is_loaned = game
        .players
        .iter()
        .find(|p| p.id == player_id)
        .map(|p| p.loan_listed)
        .unwrap_or(false);

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(LoanListingToggled {
        player_id,
        player_name,
        listed: is_loaned,
    })
}

// ─── transfer_make_bid ──────────────────────────────────────────────────────

// ─── transfer_make_bid ──────────────────────────────────────────────────────

pub fn transfer_make_bid(
    ctx: Arc<McpContext>,
    player_id: String,
    fee: u64,
) -> Result<BidMade, String> {
    let game = require_game(&ctx.state_manager)?;
    let player = game.players.iter().find(|p| p.id == player_id);
    let player_name = player.map(|p| p.match_name.clone()).unwrap_or_default();
    // Read before the bid: an accepted one moves the player.
    let knowledge = player.map(|p| player_knowledge(&game, p));

    let response = crate::commands::transfers::make_transfer_bid_internal(
        &ctx.state_manager,
        Career::Active,
        &player_id,
        fee,
    )?;

    let bid = BidMade {
        player_name,
        fee,
        decision: serde_label(&response.decision),
        suggested_fee: suggested_fee_the_club_may_see(knowledge.as_ref(), response.suggested_fee),
        is_terminal: response.is_terminal,
        mood: serde_label(&response.feedback.mood),
        tension: response.feedback.tension,
        patience: response.feedback.patience,
        round: response.feedback.round,
    };

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(bid)
}

// ─── transfer_preview_bid ──────────────────────────────────────────────────

// ─── transfer_preview_bid ──────────────────────────────────────────────────

pub fn transfer_preview_bid(
    ctx: Arc<McpContext>,
    player_id: String,
    fee: u64,
) -> Result<BidPreview, String> {
    let game = require_game(&ctx.state_manager)?;
    let player = game.players.iter().find(|p| p.id == player_id);
    let player_name = player.map(|p| p.match_name.clone()).unwrap_or_default();
    let wage = player.map_or(VisibleWage::Hidden, |p| {
        KnowledgeView::of(&game).of_player(p).wage
    });

    let response = crate::commands::transfers::preview_transfer_bid_financial_impact_internal(
        &ctx.state_manager,
        &player_id,
        fee,
    )?;
    Ok(bid_preview_of(
        player_name,
        fee,
        &response.projection,
        &wage,
    ))
}

/// The seller's counter-offer is the seller's valuation, which a club that does not know the
/// player has no business reading: it is withheld for an unknown player (maintainer ruling, 28 Oct:
/// hidden, not blurred). Anyone the club knows, scouted or in its division, keeps his answer.
fn suggested_fee_the_club_may_see(
    knowledge: Option<&PlayerKnowledge>,
    suggested_fee: Option<u64>,
) -> Option<u64> {
    match knowledge {
        Some(PlayerKnowledge::Unknown) | None => None,
        Some(_) => suggested_fee,
    }
}

/// The preview with its wage-derived figures only when the club may see the exact wage; the
/// difference between the wage bill before and after, and the board's verdict on it, are the
/// player's wage by another name.
fn bid_preview_of(
    player_name: String,
    fee: u64,
    p: &TransferBidFinancialProjection,
    wage: &VisibleWage,
) -> BidPreview {
    let exact = matches!(wage, VisibleWage::Exact { .. });
    BidPreview {
        player_name,
        fee,
        transfer_budget_before: p.transfer_budget_before,
        transfer_budget_after: p.transfer_budget_after,
        finance_before: p.finance_before,
        finance_after: p.finance_after,
        current_weekly_wage_spend: p.current_weekly_wage_spend,
        projected_weekly_wage_spend: exact.then_some(p.projected_weekly_wage_spend),
        weekly_wage_budget: p.weekly_wage_budget,
        projected_wage_budget_usage_pct: exact.then_some(p.projected_wage_budget_usage_pct),
        exceeds_transfer_budget: p.exceeds_transfer_budget,
        exceeds_finance: p.exceeds_finance,
        blocked_by_wage_policy: exact.then_some(p.blocked_by_wage_policy),
        incoming_weekly_wage: wage.value(),
        incoming_wage_is_approximate: matches!(wage, VisibleWage::Approximate { .. }),
    }
}

// ─── transfer_respond_to_offer ──────────────────────────────────────────────

// ─── transfer_respond_to_offer ──────────────────────────────────────────────

pub fn transfer_respond_to_offer(
    ctx: Arc<McpContext>,
    player_id: String,
    offer_id: String,
    accept: bool,
) -> Result<OfferAnswered, String> {
    crate::commands::transfers::respond_to_offer_internal(
        &ctx.state_manager,
        Career::Active,
        &player_id,
        &offer_id,
        accept,
    )?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(OfferAnswered {
        player_id,
        offer_id,
        accepted: accept,
    })
}

// ─── transfer_counter_offer ─────────────────────────────────────────────────

// ─── transfer_counter_offer ─────────────────────────────────────────────────

pub fn transfer_counter_offer(
    ctx: Arc<McpContext>,
    player_id: String,
    offer_id: String,
    requested_fee: u64,
) -> Result<CounterOffered, String> {
    let response = crate::commands::transfers::counter_offer_internal(
        &ctx.state_manager,
        Career::Active,
        &player_id,
        &offer_id,
        requested_fee,
    )?;

    let countered = CounterOffered {
        requested_fee,
        decision: serde_label(&response.decision),
        suggested_fee: response.suggested_fee,
        is_terminal: response.is_terminal,
    };

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(countered)
}

// ─── contract_propose_renewal ───────────────────────────────────────────────

// ─── transfer_market_browse ─────────────────────────────────────────────────

pub fn transfer_market_browse(
    ctx: Arc<McpContext>,
    position: Option<String>,
    max_price: Option<u64>,
    listed_only: Option<bool>,
) -> Result<TransferMarket, String> {
    let game = require_game(&ctx.state_manager)?;
    market_in(&game, position.as_deref(), max_price, listed_only)
}

/// The players the club does not own, each with the rating and wage the club may see
/// (`ofm_core::player_knowledge`, the rule the players list follows). `max_price` is an annual
/// figure; it filters on the wage the club can see, so a hidden wage never passes it and the filter
/// cannot be used to find out what the club may not know.
fn market_in(
    game: &Game,
    position: Option<&str>,
    max_price: Option<u64>,
    listed_only: Option<bool>,
) -> Result<TransferMarket, String> {
    game.manager
        .team_id
        .as_deref()
        .ok_or("be.error.noTeamAssigned")?;
    let max_weekly_wage =
        max_price.map(|max| u32::try_from(max / WEEKS_IN_A_YEAR).unwrap_or(u32::MAX));
    let view = KnowledgeView::of(game);

    let players = game
        .players
        .iter()
        .filter(|p| !listed_only.unwrap_or(false) || p.transfer_listed || p.loan_listed)
        .filter(|p| {
            position.is_none_or(|pos| {
                format_position(&p.position).to_lowercase() == pos.to_lowercase()
                    || format!("{:?}", p.position).to_lowercase() == pos.to_lowercase()
            })
        })
        .filter_map(|p| {
            let visibility = view.of_player(p);
            if visibility.knowledge == PlayerKnowledge::Own
                || !passes_wage_filter(&visibility.wage, max_weekly_wage)
            {
                return None;
            }
            Some(MarketPlayer {
                id: p.id.clone(),
                name: p.match_name.clone(),
                position: format_position(&p.position).to_string(),
                age: age_from_dob(&p.date_of_birth, game),
                ovr: visibility.ovr,
                team: p
                    .team_id
                    .as_deref()
                    .and_then(|tid| game.teams.iter().find(|t| t.id == tid))
                    .map(|t| t.name.clone()),
                transfer_listed: p.transfer_listed,
                loan_listed: p.loan_listed,
                wage: visibility.wage.value(),
                wage_is_approximate: matches!(visibility.wage, VisibleWage::Approximate { .. }),
            })
        })
        .collect();

    Ok(TransferMarket { players })
}

const WEEKS_IN_A_YEAR: u64 = 52;

// ─── transfer_free_agent_offer ───────────────────────────────────────────────

// ─── transfer_free_agent_offer ───────────────────────────────────────────────

pub fn transfer_free_agent_offer(
    ctx: Arc<McpContext>,
    player_id: String,
    weekly_wage: u32,
    contract_years: u32,
) -> Result<FreeAgentOffered, String> {
    let response = crate::commands::contracts::offer_free_agent_contract_internal(
        &ctx.state_manager,
        Career::Active,
        &player_id,
        weekly_wage,
        contract_years,
    )?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(FreeAgentOffered {
        weekly_wage,
        contract_years,
        outcome: serde_label(&response.outcome),
    })
}

// ─── transfer_free_agent_preview ────────────────────────────────────────────

// ─── transfer_free_agent_preview ────────────────────────────────────────────

pub fn transfer_free_agent_preview(
    ctx: Arc<McpContext>,
    player_id: String,
    weekly_wage: u32,
) -> Result<FreeAgentPreview, String> {
    let response = crate::commands::contracts::preview_free_agent_contract_impact_internal(
        &ctx.state_manager,
        &player_id,
        weekly_wage,
    )?;
    let p = &response.projection;

    Ok(FreeAgentPreview {
        weekly_wage,
        current_weekly_wage_spend: p.current_weekly_wage_spend,
        projected_weekly_wage_spend: p.projected_weekly_wage_spend,
        annual_wage_budget: p.annual_wage_budget,
        annual_soft_cap: p.annual_soft_cap,
        current_cash_runway_weeks: p.current_cash_runway_weeks,
        projected_cash_runway_weeks: p.projected_cash_runway_weeks,
        currently_over_budget: p.currently_over_budget,
        policy_allows: p.policy_allows,
    })
}

// ─── info_player_stats ──────────────────────────────────────────────────────

#[cfg(test)]
mod market_tests {
    use super::market_in;
    use chrono::{TimeZone, Utc};
    use domain::league::League;
    use domain::manager::Manager;
    use domain::player::{ActiveLoan, Player, PlayerAttributes, Position};
    use mcp_results::transfers::MarketPlayer;
    use ofm_core::clock::GameClock;
    use ofm_core::game::{Game, ScoutedPlayer};

    const MY_CLUB: &str = "mine";

    fn attributes(value: u8) -> PlayerAttributes {
        PlayerAttributes {
            pace: value,
            stamina: value,
            strength: value,
            agility: value,
            passing: value,
            shooting: value,
            tackling: value,
            dribbling: value,
            defending: value,
            positioning: value,
            vision: value,
            decisions: value,
            composure: value,
            aggression: value,
            teamwork: value,
            leadership: value,
            handling: value,
            reflexes: value,
            aerial: value,
        }
    }

    fn player(id: &str, team_id: Option<&str>, ovr: u8, wage: u32) -> Player {
        let mut player = Player::new(
            id.to_string(),
            id.to_string(),
            format!("{id} Full"),
            "2000-01-01".to_string(),
            "GB".to_string(),
            Position::Midfielder,
            attributes(60),
        );
        player.team_id = team_id.map(str::to_string);
        player.ovr = ovr;
        player.stage_contract(None, None, wage);
        player
    }

    /// My club and "rival" play in division one; "faraway" plays in division two. Besides my own
    /// player and one I loaned out, the market has a division rival (rating 71, wage 12 345), a
    /// player my scout reported on (wage 15 843), a stranger in division two and a free agent.
    fn world(manager_club: Option<&str>) -> Game {
        let mut manager = Manager::new(
            "m1".to_string(),
            "A".to_string(),
            "B".to_string(),
            "1980-01-01".to_string(),
            "GB".to_string(),
        );
        manager.team_id = manager_club.map(str::to_string);
        let mut loaned_out = player("loaned-out", Some("faraway"), 80, 9_000);
        loaned_out.active_loan = Some(ActiveLoan {
            parent_team_id: MY_CLUB.to_string(),
            loan_team_id: "faraway".to_string(),
            start_date: "2026-07-01".to_string(),
            end_date: "2027-06-30".to_string(),
            wage_contribution_pct: 100,
            buy_option_fee: None,
            loan_start_minutes: 0,
            loan_start_appearances: 0,
            development_reported_minutes: 0,
            development_reported_appearances: 0,
        });
        let mut rival = player("rival-man", Some("rival"), 71, 12_345);
        rival.transfer_listed = true;
        let players = vec![
            player("own", Some(MY_CLUB), 75, 5_000),
            loaned_out,
            rival,
            player("scouted", Some("faraway"), 66, 15_843),
            player("stranger", Some("faraway"), 90, 20_000),
            player("free", None, 55, 3_000),
        ];
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 9, 15, 12, 0, 0).unwrap());
        let mut game = Game::new(clock, manager, vec![], players, vec![], vec![]);
        let division = |id: &str, clubs: &[&str]| {
            let clubs: Vec<String> = clubs.iter().map(|club| club.to_string()).collect();
            League::new(id.to_string(), id.to_string(), 2026, &clubs)
        };
        game.competitions = vec![
            division("division-one", &[MY_CLUB, "rival"]),
            division("division-two", &["faraway", "other"]),
        ];
        game.scouted_players.push(ScoutedPlayer {
            player_id: "scouted".to_string(),
            scouted_on: "2026-09-01".to_string(),
            attributes: attributes(60),
        });
        game
    }

    fn ids(market: &mcp_results::transfers::TransferMarket) -> Vec<&str> {
        market.players.iter().map(|p| p.id.as_str()).collect()
    }

    fn row<'a>(market: &'a mcp_results::transfers::TransferMarket, id: &str) -> &'a MarketPlayer {
        market.players.iter().find(|p| p.id == id).expect("listed")
    }

    /// Given a market with a division rival, a scouted player, a stranger, a free agent and two
    /// players the club owns (one loaned out)
    /// When the market is browsed with no filter
    /// Then the club's own players are not listed, the rival shows his rating but no wage, the
    ///      scouted player no rating and a rounded wage, and the others nothing but who they are.
    #[test]
    fn the_market_shows_each_player_as_much_as_the_club_knows() {
        let market = market_in(&world(Some(MY_CLUB)), None, None, None).expect("a market");

        assert_eq!(ids(&market), ["rival-man", "scouted", "stranger", "free"]);
        let rival = row(&market, "rival-man");
        assert_eq!(
            (rival.ovr, rival.wage, rival.wage_is_approximate),
            (Some(71), None, false)
        );
        let scouted = row(&market, "scouted");
        assert_eq!(
            (scouted.ovr, scouted.wage, scouted.wage_is_approximate),
            (None, Some(16_000), true)
        );
        for hidden in ["stranger", "free"] {
            let unknown = row(&market, hidden);
            assert_eq!((unknown.ovr, unknown.wage), (None, None), "{hidden}");
        }
    }

    /// Given an annual price ceiling
    /// When the market is browsed
    /// Then only a player whose wage the club can see and whose weekly wage fits (52 weeks) passes;
    ///      a hidden wage never does, however high the ceiling, so the filter is no wage oracle.
    #[test]
    fn the_price_filter_never_matches_a_wage_the_club_cannot_see() {
        let game = world(Some(MY_CLUB));

        let roomy = market_in(&game, None, Some(1_000_000), None).expect("a market");
        let unlimited = market_in(&game, None, Some(u64::MAX), None).expect("a market");
        let tight = market_in(&game, None, Some(100_000), None).expect("a market");

        assert_eq!(ids(&roomy), ["scouted"]);
        assert_eq!(ids(&unlimited), ["scouted"]);
        assert!(ids(&tight).is_empty(), "{:?}", ids(&tight));
    }

    /// Given a listed rival and a position nobody plays
    /// When the market is browsed for listed players, and for goalkeepers
    /// Then the first finds the rival alone and the second finds nobody.
    #[test]
    fn the_listing_and_position_filters_still_apply() {
        let game = world(Some(MY_CLUB));

        let listed = market_in(&game, None, None, Some(true)).expect("a market");
        let keepers = market_in(&game, Some("Goalkeeper"), None, None).expect("a market");
        let midfielders = market_in(&game, Some("Midfielder"), None, None).expect("a market");

        assert_eq!(ids(&listed), ["rival-man"]);
        assert!(ids(&keepers).is_empty());
        assert_eq!(midfielders.players.len(), 4);
    }

    /// Given a manager without a club
    /// When the market is browsed
    /// Then it says there is no team, as before.
    #[test]
    fn a_manager_without_a_club_has_no_market() {
        let refused = market_in(&world(None), None, None, None);

        assert_eq!(refused.err().as_deref(), Some("be.error.noTeamAssigned"));
    }
}

#[cfg(test)]
mod bid_answer_tests {
    use super::{bid_preview_of, suggested_fee_the_club_may_see};
    use ofm_core::player_knowledge::{PlayerKnowledge, VisibleWage};
    use ofm_core::transfers::TransferBidFinancialProjection;

    fn projection() -> TransferBidFinancialProjection {
        TransferBidFinancialProjection {
            transfer_budget_before: 5_000_000,
            transfer_budget_after: 4_000_000,
            finance_before: 9_000_000,
            finance_after: 8_000_000,
            annual_wage_bill_before: 2_600_000,
            annual_wage_bill_after: 3_000_000,
            annual_wage_budget: 3_500_000,
            current_weekly_wage_spend: 50_000,
            projected_weekly_wage_spend: 57_692,
            weekly_wage_budget: 67_307,
            incoming_player_weekly_wage: 7_692,
            projected_wage_budget_usage_pct: 86,
            exceeds_transfer_budget: false,
            exceeds_finance: false,
            blocked_by_wage_policy: true,
            pending_registration_date: None,
        }
    }

    fn scouted() -> PlayerKnowledge {
        PlayerKnowledge::Scouted {
            scouted_on: "2026-09-01".to_string(),
            current: true,
        }
    }

    /// Given a counter-offer of 900 000 from the seller
    /// When the club asks what fee it may see, for each kind of knowledge
    /// Then an unknown player's (and an unresolvable one's) is withheld and everyone the club
    ///      knows keeps it.
    #[test]
    fn the_sellers_counter_is_withheld_for_an_unknown_player_only() {
        let counter = Some(900_000);

        assert_eq!(
            suggested_fee_the_club_may_see(Some(&PlayerKnowledge::Unknown), counter),
            None
        );
        assert_eq!(suggested_fee_the_club_may_see(None, counter), None);
        for knowledge in [PlayerKnowledge::Own, PlayerKnowledge::Known, scouted()] {
            assert_eq!(
                suggested_fee_the_club_may_see(Some(&knowledge), counter),
                counter,
                "{knowledge:?}"
            );
        }
        assert_eq!(
            suggested_fee_the_club_may_see(Some(&PlayerKnowledge::Known), None),
            None
        );
    }

    /// Given a hidden wage (an unknown player, or a rival the club only knows)
    /// When the bid preview is built
    /// Then nothing in it gives the wage away: no projected wage bill, usage or policy verdict,
    ///      and no wage of his own; the fee-side figures are untouched.
    #[test]
    fn a_hidden_wage_leaves_the_preview_without_any_wage_derived_figure() {
        let preview = bid_preview_of(
            "P".to_string(),
            1_000_000,
            &projection(),
            &VisibleWage::Hidden,
        );

        assert_eq!(preview.projected_weekly_wage_spend, None);
        assert_eq!(preview.projected_wage_budget_usage_pct, None);
        assert_eq!(preview.blocked_by_wage_policy, None);
        assert_eq!(
            (
                preview.incoming_weekly_wage,
                preview.incoming_wage_is_approximate
            ),
            (None, false)
        );
        assert_eq!(preview.transfer_budget_after, 4_000_000);
        assert_eq!(preview.current_weekly_wage_spend, 50_000);
    }

    /// Given a wage a scout reported on (rounded)
    /// When the bid preview is built
    /// Then the rounded wage is shown and flagged, and the exact derived figures still are not.
    #[test]
    fn a_scouted_wage_shows_as_the_rounded_figure_only() {
        let wage = VisibleWage::Approximate { weekly: 7_700 };

        let preview = bid_preview_of("P".to_string(), 1_000_000, &projection(), &wage);

        assert_eq!(
            (
                preview.incoming_weekly_wage,
                preview.incoming_wage_is_approximate
            ),
            (Some(7_700), true)
        );
        assert_eq!(preview.projected_weekly_wage_spend, None);
        assert_eq!(preview.blocked_by_wage_policy, None);
    }

    /// Given a wage the club may see exactly
    /// When the bid preview is built
    /// Then the projected bill, usage and policy verdict are the projection's own.
    #[test]
    fn an_exact_wage_keeps_every_figure() {
        let wage = VisibleWage::Exact { weekly: 7_692 };

        let preview = bid_preview_of("P".to_string(), 1_000_000, &projection(), &wage);

        assert_eq!(preview.projected_weekly_wage_spend, Some(57_692));
        assert_eq!(preview.projected_wage_budget_usage_pct, Some(86));
        assert_eq!(preview.blocked_by_wage_policy, Some(true));
        assert_eq!(
            (
                preview.incoming_weekly_wage,
                preview.incoming_wage_is_approximate
            ),
            (Some(7_692), false)
        );
    }
}
