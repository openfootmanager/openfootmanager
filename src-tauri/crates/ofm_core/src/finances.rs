use crate::game::Game;
use chrono::{Datelike, NaiveDate};
use domain::message::*;
use domain::team::{
    FinancialTransaction, FinancialTransactionKind, Sponsorship, SponsorshipBonusCriterion, Team,
};
use rand::RngExt;
use serde::Serialize;

const BOARD_SUPPORT_MIN_AMOUNT: i64 = 150_000;
const BOARD_SUPPORT_MAX_AMOUNT: i64 = 1_000_000;
const BOARD_SUPPORT_TARGET_RUNWAY_WEEKS: i64 = 8;
const BOARD_SUPPORT_SATISFACTION_PENALTY: u8 = 12;
const FINANCE_WARNING_SATISFACTION_PENALTY: u8 = 2;
const FINANCE_CRITICAL_SATISFACTION_PENALTY: u8 = 4;
const MARKETING_CAMPAIGN_COOLDOWN_DAYS: i64 = 28;
const MARKETING_CAMPAIGN_MIN_GROSS_REVENUE: i64 = 60_000;
const MARKETING_CAMPAIGN_MAX_GROSS_REVENUE: i64 = 250_000;
const MARKETING_CAMPAIGN_MIN_COST: i64 = 15_000;
const SPONSOR_PITCH_DURATION_WEEKS: u32 = 12;
const SPONSOR_PITCH_MIN_WEEKLY_AMOUNT: i64 = 40_000;
const SPONSOR_PITCH_MAX_WEEKLY_AMOUNT: i64 = 180_000;
const SPONSOR_PITCH_REPUTATION_MULTIPLIER: i64 = 120;

fn marketing_campaign_activation_description() -> String {
    ["Marketing", "campaign", "activation", "spend"].join(" ")
}

fn marketing_campaign_revenue_description() -> String {
    ["Marketing", "campaign", "merchandise", "revenue"].join(" ")
}

fn board_support_description(season: u32) -> String {
    format!(
        "{} {}",
        ["Board", "support", "package", "for", "season"].join(" "),
        season
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FinanceHealthLevel {
    Stable,
    Watch,
    Warning,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TeamFinanceSnapshot {
    pub annual_wage_bill: i64,
    pub weekly_wage_spend: i64,
    pub weekly_wage_budget: i64,
    pub weekly_recurring_income: i64,
    pub weekly_sponsor_income: i64,
    pub projected_weekly_net: i64,
    pub cash_runway_weeks: Option<i64>,
    pub wage_budget_usage_percent: u32,
    pub currently_in_debt: bool,
    pub currently_over_budget: bool,
    pub wage_budget_status: FinanceHealthLevel,
    pub runway_status: FinanceHealthLevel,
    pub overall_status: FinanceHealthLevel,
    pub marketing_campaign_cooldown_days_remaining: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct BoardSupportResult {
    pub support_amount: i64,
    pub transfer_budget_reduction: i64,
    pub satisfaction_penalty: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SponsorPitchPreview {
    pub sponsor_name: String,
    pub weekly_amount: i64,
    pub duration_weeks: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct MarketingCampaignPreview {
    pub gross_revenue: i64,
    pub campaign_cost: i64,
    pub net_income: i64,
    pub cooldown_days: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct FinanceActionPreviews {
    pub board_support: Option<BoardSupportResult>,
    pub sponsor_pitch: Option<SponsorPitchPreview>,
    pub marketing_campaign: Option<MarketingCampaignPreview>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SponsorPitchResult {
    pub message_id: String,
    pub sponsor_name: String,
    pub weekly_amount: i64,
    pub duration_weeks: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MarketingCampaignResult {
    pub message_id: String,
    pub gross_revenue: i64,
    pub campaign_cost: i64,
    pub net_income: i64,
    pub cooldown_days: u32,
}

fn wage_budget_status(usage_percent: u32) -> FinanceHealthLevel {
    if usage_percent > 110 {
        return FinanceHealthLevel::Critical;
    }

    if usage_percent > 100 {
        return FinanceHealthLevel::Warning;
    }

    if usage_percent >= 85 {
        return FinanceHealthLevel::Watch;
    }

    FinanceHealthLevel::Stable
}

fn runway_status(balance: i64, runway_weeks: Option<i64>) -> FinanceHealthLevel {
    if balance < 0 {
        return FinanceHealthLevel::Critical;
    }

    let Some(runway_weeks) = runway_weeks else {
        return FinanceHealthLevel::Stable;
    };

    if runway_weeks <= 4 {
        return FinanceHealthLevel::Critical;
    }

    if runway_weeks <= 8 {
        return FinanceHealthLevel::Warning;
    }

    if runway_weeks <= 12 {
        return FinanceHealthLevel::Watch;
    }

    FinanceHealthLevel::Stable
}

fn most_severe_level(left: FinanceHealthLevel, right: FinanceHealthLevel) -> FinanceHealthLevel {
    fn severity(level: FinanceHealthLevel) -> u8 {
        match level {
            FinanceHealthLevel::Stable => 0,
            FinanceHealthLevel::Watch => 1,
            FinanceHealthLevel::Warning => 2,
            FinanceHealthLevel::Critical => 3,
        }
    }

    if severity(left) >= severity(right) {
        left
    } else {
        right
    }
}

fn action(id: &str, label: &str, label_key: &str, action_type: ActionType) -> MessageAction {
    MessageAction {
        id: id.to_string(),
        label: label.to_string(),
        action_type,
        resolved: false,
        label_key: Some(label_key.to_string()),
    }
}

pub fn calc_wages(game: &Game, team_id: &str) -> i64 {
    let player_wages: i64 = game
        .players
        .iter()
        .filter(|player| player.team_id.as_deref() == Some(team_id))
        .map(|player| player.wage as i64 / 52)
        .sum();

    let staff_wages: i64 = game
        .staff
        .iter()
        .filter(|staff_member| staff_member.team_id.as_deref() == Some(team_id))
        .map(|staff_member| staff_member.wage as i64 / 52)
        .sum();

    player_wages + staff_wages
}

pub fn calc_annual_wages(game: &Game, team_id: &str) -> i64 {
    let player_wages: i64 = game
        .players
        .iter()
        .filter(|player| player.team_id.as_deref() == Some(team_id))
        .map(|player| player.wage as i64)
        .sum();

    let staff_wages: i64 = game
        .staff
        .iter()
        .filter(|staff_member| staff_member.team_id.as_deref() == Some(team_id))
        .map(|staff_member| staff_member.wage as i64)
        .sum();

    player_wages + staff_wages
}

pub fn calc_cash_runway_weeks(balance: i64, projected_weekly_net: i64) -> Option<i64> {
    if projected_weekly_net >= 0 {
        return None;
    }

    Some(std::cmp::max(0, balance / projected_weekly_net.abs()))
}

pub fn calc_matchday(
    stadium_capacity: u32,
    home_match_count: i64,
    attendance_pct: f64,
    avg_ticket: f64,
) -> i64 {
    let revenue_per_match = (stadium_capacity as f64 * attendance_pct * avg_ticket) as i64;

    revenue_per_match * home_match_count
}

pub fn calc_upkeep(_team: &Team) -> i64 {
    0
}

fn estimated_weekly_matchday_income(game: &Game, team: &Team) -> i64 {
    let recent_home_match_count = count_recent_home_matches(game, &team.id);
    if recent_home_match_count == 0 {
        return 0;
    }

    calc_matchday(team.stadium_capacity, recent_home_match_count, 0.76, 20.0)
}

pub fn team_finance_snapshot(game: &Game, team_id: &str) -> Option<TeamFinanceSnapshot> {
    let team = game.teams.iter().find(|team| team.id == team_id)?;
    let annual_wage_bill = calc_annual_wages(game, team_id);
    let weekly_wage_spend = calc_wages(game, team_id);
    let weekly_wage_budget = team.wage_budget / 52;
    let current_position = current_league_position(game, team_id);
    let weekly_sponsor_income = team
        .sponsorship
        .as_ref()
        .map(|sponsorship| {
            sponsorship.base_value
                + evaluate_sponsorship_bonus(current_position, &team.form, sponsorship)
        })
        .unwrap_or(0);
    let weekly_matchday_income = estimated_weekly_matchday_income(game, team);
    let weekly_recurring_income = weekly_sponsor_income + weekly_matchday_income;
    let projected_weekly_net = weekly_recurring_income - weekly_wage_spend;
    let cash_runway_weeks = calc_cash_runway_weeks(team.finance, projected_weekly_net);
    let wage_budget_usage_percent = ((annual_wage_bill * 100) / std::cmp::max(1, team.wage_budget))
        .clamp(0, u32::MAX as i64) as u32;
    let wage_budget_status = wage_budget_status(wage_budget_usage_percent);
    let runway_status = runway_status(team.finance, cash_runway_weeks);

    Some(TeamFinanceSnapshot {
        annual_wage_bill,
        weekly_wage_spend,
        weekly_wage_budget,
        weekly_recurring_income,
        weekly_sponsor_income,
        projected_weekly_net,
        cash_runway_weeks,
        wage_budget_usage_percent,
        currently_in_debt: team.finance < 0,
        currently_over_budget: annual_wage_bill > team.wage_budget,
        wage_budget_status,
        runway_status,
        overall_status: most_severe_level(wage_budget_status, runway_status),
        marketing_campaign_cooldown_days_remaining: marketing_campaign_cooldown_days_remaining(
            team,
            game.clock.current_date.date_naive(),
        ),
    })
}

fn weekly_finance_satisfaction_penalty(snapshot: &TeamFinanceSnapshot) -> u8 {
    match snapshot.overall_status {
        FinanceHealthLevel::Critical => FINANCE_CRITICAL_SATISFACTION_PENALTY,
        FinanceHealthLevel::Warning => FINANCE_WARNING_SATISFACTION_PENALTY,
        FinanceHealthLevel::Stable | FinanceHealthLevel::Watch => 0,
    }
}

fn apply_weekly_finance_satisfaction_pressure(game: &mut Game) {
    let Some(user_team_id) = game.manager.team_id.clone() else {
        return;
    };

    let Some(snapshot) = team_finance_snapshot(game, &user_team_id) else {
        return;
    };

    let penalty = weekly_finance_satisfaction_penalty(&snapshot);
    if penalty == 0 {
        return;
    }

    game.manager.satisfaction = game.manager.satisfaction.saturating_sub(penalty);
}

fn finance_board_pressure_message(
    today: &str,
    severity: FinanceHealthLevel,
    penalty: u8,
) -> InboxMessage {
    let (body_key, priority) = match severity {
        FinanceHealthLevel::Critical => (
            "be.msg.financeBoardPressure.bodyCritical",
            MessagePriority::Urgent,
        ),
        FinanceHealthLevel::Warning => (
            "be.msg.financeBoardPressure.bodyWarning",
            MessagePriority::High,
        ),
        FinanceHealthLevel::Stable | FinanceHealthLevel::Watch => unreachable!(),
    };

    InboxMessage::new(
        format!("finance_board_pressure_{}", today),
        String::new(),
        String::new(),
        String::new(),
        today.to_string(),
    )
    .with_category(MessageCategory::BoardDirective)
    .with_priority(priority)
    .with_sender_role("")
    .with_i18n("be.msg.financeBoardPressure.subject", body_key, {
        let mut p = std::collections::HashMap::new();
        p.insert("penalty".to_string(), penalty.to_string());
        p
    })
    .with_sender_i18n("be.sender.boardOfDirectors", "be.role.chairman")
    .with_action(action(
        "view_finances",
        "",
        "be.msg.action.viewFinances",
        ActionType::NavigateTo {
            route: "/dashboard?tab=Finances".to_string(),
        },
    ))
}

fn marketing_campaign_message(
    today: &str,
    gross_revenue: i64,
    campaign_cost: i64,
    net_income: i64,
    cooldown_days: u32,
) -> InboxMessage {
    InboxMessage::new(
        format!("marketing_campaign_{}", today),
        String::new(),
        String::new(),
        String::new(),
        today.to_string(),
    )
    .with_category(MessageCategory::Finance)
    .with_priority(MessagePriority::Normal)
    .with_sender_role("")
    .with_i18n(
        "be.msg.marketingCampaign.subject",
        "be.msg.marketingCampaign.body",
        {
            let mut p = std::collections::HashMap::new();
            p.insert("grossRevenue".to_string(), gross_revenue.to_string());
            p.insert("campaignCost".to_string(), campaign_cost.to_string());
            p.insert("netIncome".to_string(), net_income.to_string());
            p.insert("days".to_string(), cooldown_days.to_string());
            p
        },
    )
    .with_sender_i18n("be.sender.commercialDirector", "be.role.commercialDirector")
    .with_action(action(
        "ack",
        "",
        "be.msg.event.ack",
        ActionType::Acknowledge,
    ))
}

fn board_support_season(game: &Game) -> u32 {
    game.league
        .as_ref()
        .map(|league| league.season)
        .unwrap_or(game.clock.current_date.year().max(0) as u32)
}

fn sponsor_pitch_available(snapshot: &TeamFinanceSnapshot) -> bool {
    snapshot.currently_over_budget
        || snapshot.currently_in_debt
        || matches!(
            snapshot.wage_budget_status,
            FinanceHealthLevel::Warning | FinanceHealthLevel::Critical
        )
        || matches!(
            snapshot.runway_status,
            FinanceHealthLevel::Warning | FinanceHealthLevel::Critical
        )
}

fn board_support_available(snapshot: &TeamFinanceSnapshot) -> bool {
    snapshot.currently_in_debt
        || matches!(
            snapshot.runway_status,
            FinanceHealthLevel::Warning | FinanceHealthLevel::Critical
        )
}

fn marketing_campaign_available(snapshot: &TeamFinanceSnapshot) -> bool {
    snapshot.currently_over_budget
        || snapshot.currently_in_debt
        || matches!(
            snapshot.wage_budget_status,
            FinanceHealthLevel::Warning | FinanceHealthLevel::Critical
        )
        || matches!(
            snapshot.runway_status,
            FinanceHealthLevel::Warning | FinanceHealthLevel::Critical
        )
}

fn most_recent_marketing_campaign_date(team: &Team) -> Option<NaiveDate> {
    team.financial_ledger
        .iter()
        .filter(|entry| entry.kind == FinancialTransactionKind::CommercialCampaign)
        .filter_map(|entry| NaiveDate::parse_from_str(&entry.date, "%Y-%m-%d").ok())
        .max()
}

fn marketing_campaign_cooldown_days_remaining(team: &Team, today: NaiveDate) -> u32 {
    let Some(last_campaign) = most_recent_marketing_campaign_date(team) else {
        return 0;
    };

    let days_since = (today - last_campaign).num_days();
    if days_since >= MARKETING_CAMPAIGN_COOLDOWN_DAYS {
        0
    } else {
        (MARKETING_CAMPAIGN_COOLDOWN_DAYS - days_since) as u32
    }
}

fn marketing_campaign_gross_revenue(team: &Team, snapshot: &TeamFinanceSnapshot) -> i64 {
    let reputation_component = (team.reputation as i64) * 250;
    let stadium_component = (team.stadium_capacity as i64) * 3;
    let pressure_component = match snapshot.overall_status {
        FinanceHealthLevel::Stable => 0,
        FinanceHealthLevel::Watch => 10_000,
        FinanceHealthLevel::Warning => 25_000,
        FinanceHealthLevel::Critical => 40_000,
    };
    let debt_bonus = if snapshot.currently_in_debt {
        20_000
    } else {
        0
    };
    let wage_pressure_bonus = if snapshot.currently_over_budget {
        15_000
    } else {
        0
    };

    (reputation_component
        + stadium_component
        + pressure_component
        + debt_bonus
        + wage_pressure_bonus)
        .clamp(
            MARKETING_CAMPAIGN_MIN_GROSS_REVENUE,
            MARKETING_CAMPAIGN_MAX_GROSS_REVENUE,
        )
}

fn marketing_campaign_cost(gross_revenue: i64) -> i64 {
    (gross_revenue / 4).max(MARKETING_CAMPAIGN_MIN_COST)
}

fn has_pending_sponsor_offer(game: &Game) -> bool {
    game.messages.iter().any(|message| {
        message.id.starts_with("sponsor_") && message.actions.iter().any(|action| !action.resolved)
    })
}

fn sponsor_pitch_message_id(game: &Game) -> String {
    format!(
        "sponsor_pitch_{}",
        game.clock.current_date.format("%Y-%m-%d")
    )
}

fn sponsor_pitch_partner(team_id: &str, current_date: chrono::DateTime<chrono::Utc>) -> String {
    const SPONSORS: [&[&str]; 8] = [
        &["Northstar", "Logistics"],
        &["Harbor", "Bank"],
        &["Crest", "Mobile"],
        &["Vertex", "Nutrition"],
        &["Iron", "Peak", "Tools"],
        &["Brightline", "Energy"],
        &["Summit", "Capital"],
        &["Evergreen", "Foods"],
    ];

    let seed = team_id
        .bytes()
        .fold(current_date.ordinal() as usize, |acc, byte| {
            acc.wrapping_mul(31).wrapping_add(byte as usize)
        });

    SPONSORS[seed % SPONSORS.len()].join(" ")
}

fn sponsor_pitch_weekly_amount(
    team: &Team,
    snapshot: &TeamFinanceSnapshot,
    current_position: Option<u32>,
) -> i64 {
    let reputation_component = team.reputation as i64 * SPONSOR_PITCH_REPUTATION_MULTIPLIER;
    let league_position_component = match current_position {
        Some(1) => 18_000,
        Some(2..=4) => 12_000,
        Some(5..=8) => 6_000,
        _ => 0,
    };
    let pressure_component = match snapshot.overall_status {
        FinanceHealthLevel::Stable => 0,
        FinanceHealthLevel::Watch => 5_000,
        FinanceHealthLevel::Warning => 15_000,
        FinanceHealthLevel::Critical => 25_000,
    };
    let wage_pressure_bonus = if snapshot.currently_over_budget {
        15_000
    } else {
        0
    };
    let debt_bonus = if snapshot.currently_in_debt {
        20_000
    } else {
        0
    };

    (SPONSOR_PITCH_MIN_WEEKLY_AMOUNT
        + reputation_component
        + league_position_component
        + pressure_component
        + wage_pressure_bonus
        + debt_bonus)
        .clamp(
            SPONSOR_PITCH_MIN_WEEKLY_AMOUNT,
            SPONSOR_PITCH_MAX_WEEKLY_AMOUNT,
        )
}

pub fn preview_board_support(game: &Game, team_id: &str) -> Result<BoardSupportResult, String> {
    let snapshot =
        team_finance_snapshot(game, team_id).ok_or("be.error.managedTeamNotFound".to_string())?;

    if !board_support_available(&snapshot) {
        return Err("be.error.finance.boardSupportUnavailable".to_string());
    }

    let season = board_support_season(game);
    let team = game
        .teams
        .iter()
        .find(|team| team.id == team_id)
        .ok_or("be.error.managedTeamNotFound".to_string())?;

    if team.financial_ledger.iter().any(|entry| {
        entry.kind == FinancialTransactionKind::BoardSupport
            && entry.description == board_support_description(season)
    }) {
        return Err("be.error.finance.boardSupportAlreadyUsed".to_string());
    }

    let reserve_target = std::cmp::max(
        snapshot.weekly_wage_spend * BOARD_SUPPORT_TARGET_RUNWAY_WEEKS,
        BOARD_SUPPORT_MIN_AMOUNT,
    );
    let support_amount = (reserve_target - team.finance)
        .max(BOARD_SUPPORT_MIN_AMOUNT)
        .min(BOARD_SUPPORT_MAX_AMOUNT);
    let transfer_budget_reduction = std::cmp::min(team.transfer_budget.max(0), support_amount / 2);

    Ok(BoardSupportResult {
        support_amount,
        transfer_budget_reduction,
        satisfaction_penalty: BOARD_SUPPORT_SATISFACTION_PENALTY,
    })
}

pub fn preview_sponsor_pitch(game: &Game, team_id: &str) -> Result<SponsorPitchPreview, String> {
    let snapshot =
        team_finance_snapshot(game, team_id).ok_or("be.error.managedTeamNotFound".to_string())?;

    if !sponsor_pitch_available(&snapshot) {
        return Err("be.error.finance.sponsorPitchUnavailable".to_string());
    }

    if has_pending_sponsor_offer(game) {
        return Err("be.error.finance.sponsorPitchPendingOffer".to_string());
    }

    let message_id = sponsor_pitch_message_id(game);
    if game.messages.iter().any(|message| message.id == message_id) {
        return Err("be.error.finance.sponsorPitchAlreadyAttemptedToday".to_string());
    }

    let team = game
        .teams
        .iter()
        .find(|team| team.id == team_id)
        .ok_or("be.error.managedTeamNotFound".to_string())?;

    if team
        .sponsorship
        .as_ref()
        .is_some_and(|sponsorship| sponsorship.remaining_weeks > 0 && sponsorship.base_value > 0)
    {
        return Err("be.error.finance.sponsorPitchActiveSponsor".to_string());
    }

    let current_position = current_league_position(game, team_id);

    Ok(SponsorPitchPreview {
        sponsor_name: sponsor_pitch_partner(team_id, game.clock.current_date),
        weekly_amount: sponsor_pitch_weekly_amount(team, &snapshot, current_position),
        duration_weeks: SPONSOR_PITCH_DURATION_WEEKS,
    })
}

pub fn preview_marketing_campaign(
    game: &Game,
    team_id: &str,
) -> Result<MarketingCampaignPreview, String> {
    let snapshot =
        team_finance_snapshot(game, team_id).ok_or("be.error.managedTeamNotFound".to_string())?;

    if !marketing_campaign_available(&snapshot) {
        return Err("be.error.finance.marketingCampaignUnavailable".to_string());
    }

    let today = game.clock.current_date.date_naive();
    let team = game
        .teams
        .iter()
        .find(|team| team.id == team_id)
        .ok_or("be.error.managedTeamNotFound".to_string())?;

    if marketing_campaign_cooldown_days_remaining(team, today) > 0 {
        return Err("be.error.finance.marketingCampaignCoolingDown".to_string());
    }

    let gross_revenue = marketing_campaign_gross_revenue(team, &snapshot);
    let campaign_cost = marketing_campaign_cost(gross_revenue);

    Ok(MarketingCampaignPreview {
        gross_revenue,
        campaign_cost,
        net_income: gross_revenue - campaign_cost,
        cooldown_days: MARKETING_CAMPAIGN_COOLDOWN_DAYS as u32,
    })
}

pub fn finance_action_previews(game: &Game, team_id: &str) -> Option<FinanceActionPreviews> {
    game.teams.iter().find(|team| team.id == team_id)?;

    Some(FinanceActionPreviews {
        board_support: preview_board_support(game, team_id).ok(),
        sponsor_pitch: preview_sponsor_pitch(game, team_id).ok(),
        marketing_campaign: preview_marketing_campaign(game, team_id).ok(),
    })
}

pub fn request_sponsor_pitch(game: &mut Game, team_id: &str) -> Result<SponsorPitchResult, String> {
    let preview = preview_sponsor_pitch(game, team_id)?;
    let message_id = sponsor_pitch_message_id(game);
    let weekly_amount = preview.weekly_amount;
    let sponsor_name = preview.sponsor_name;
    let team = game
        .teams
        .iter()
        .find(|team| team.id == team_id)
        .ok_or("be.error.managedTeamNotFound".to_string())?;
    let team_name = team.name.clone();
    let date = game.clock.current_date.format("%Y-%m-%d").to_string();

    game.messages
        .push(crate::random_events::sponsor_offer_message(
            &message_id,
            &team_name,
            &sponsor_name,
            weekly_amount as u64,
            &date,
        ));

    Ok(SponsorPitchResult {
        message_id,
        sponsor_name,
        weekly_amount,
        duration_weeks: preview.duration_weeks,
    })
}

pub fn request_marketing_campaign(
    game: &mut Game,
    team_id: &str,
) -> Result<MarketingCampaignResult, String> {
    let preview = preview_marketing_campaign(game, team_id)?;
    let today = game.clock.current_date.date_naive();
    let gross_revenue = preview.gross_revenue;
    let campaign_cost = preview.campaign_cost;
    let net_income = preview.net_income;
    let today_label = today.format("%Y-%m-%d").to_string();
    let message = marketing_campaign_message(
        &today_label,
        gross_revenue,
        campaign_cost,
        net_income,
        preview.cooldown_days,
    );
    let message_id = message.id.clone();
    let team = game
        .teams
        .iter_mut()
        .find(|team| team.id == team_id)
        .ok_or("be.error.managedTeamNotFound".to_string())?;

    team.finance += net_income;
    team.season_income += gross_revenue;
    team.season_expenses += campaign_cost;
    team.financial_ledger.push(FinancialTransaction {
        date: today_label.clone(),
        description: marketing_campaign_activation_description(),
        amount: -campaign_cost,
        kind: FinancialTransactionKind::CommercialCampaign,
    });
    team.financial_ledger.push(FinancialTransaction {
        date: today_label,
        description: marketing_campaign_revenue_description(),
        amount: gross_revenue,
        kind: FinancialTransactionKind::CommercialCampaign,
    });
    game.messages.push(message);

    Ok(MarketingCampaignResult {
        message_id,
        gross_revenue,
        campaign_cost,
        net_income,
        cooldown_days: preview.cooldown_days,
    })
}

pub fn request_board_support(game: &mut Game, team_id: &str) -> Result<BoardSupportResult, String> {
    let preview = preview_board_support(game, team_id)?;
    let season = board_support_season(game);
    let team = game
        .teams
        .iter_mut()
        .find(|team| team.id == team_id)
        .ok_or("be.error.managedTeamNotFound".to_string())?;
    let support_amount = preview.support_amount;
    let transfer_budget_reduction = preview.transfer_budget_reduction;

    team.finance += support_amount;
    team.season_income += support_amount;
    team.transfer_budget = (team.transfer_budget - transfer_budget_reduction).max(0);
    team.financial_ledger.push(FinancialTransaction {
        date: game.clock.current_date.format("%Y-%m-%d").to_string(),
        description: board_support_description(season),
        amount: support_amount,
        kind: FinancialTransactionKind::BoardSupport,
    });

    game.manager.satisfaction = game
        .manager
        .satisfaction
        .saturating_sub(preview.satisfaction_penalty);

    Ok(preview)
}

pub fn evaluate_sponsorship_bonus(
    current_position: Option<u32>,
    recent_form: &[String],
    sponsorship: &Sponsorship,
) -> i64 {
    sponsorship
        .bonus_criteria
        .iter()
        .map(|criterion| match criterion {
            SponsorshipBonusCriterion::LeaguePosition {
                max_position,
                bonus_amount,
            } => {
                if current_position.is_some_and(|position| position <= *max_position) {
                    *bonus_amount
                } else {
                    0
                }
            }
            SponsorshipBonusCriterion::UnbeatenRun {
                required_matches,
                bonus_amount,
            } => {
                if recent_form.len() >= *required_matches
                    && recent_form
                        .iter()
                        .rev()
                        .take(*required_matches)
                        .all(|result| result != "L")
                {
                    *bonus_amount
                } else {
                    0
                }
            }
        })
        .sum()
}

fn current_league_position(game: &Game, team_id: &str) -> Option<u32> {
    let league = game.league.as_ref()?;

    league
        .sorted_standings()
        .iter()
        .position(|standing| standing.team_id == team_id)
        .map(|index| index as u32 + 1)
}

fn count_recent_home_matches(game: &Game, team_id: &str) -> i64 {
    let Some(league) = &game.league else {
        return 0;
    };

    let current = game.clock.current_date.date_naive();
    let week_ago = current - chrono::Duration::days(7);

    league
        .fixtures
        .iter()
        .filter(|fixture| {
            fixture.status == domain::league::FixtureStatus::Completed
                && fixture.home_team_id == team_id
                && fixture.result.is_some()
        })
        .filter(|fixture| {
            if let Ok(date) = chrono::NaiveDate::parse_from_str(&fixture.date, "%Y-%m-%d") {
                date > week_ago && date <= current
            } else {
                false
            }
        })
        .count() as i64
}

/// Process weekly financial operations (called every Monday = weekday 0).
/// - Deduct player wages (weekly = annual / 52)
/// - Deduct staff wages
/// - Add matchday revenue for home matches played that week
/// - Check financial health and generate warnings
pub fn process_weekly_finances(game: &mut Game) {
    let weekday = game.clock.current_date.weekday().num_days_from_monday();
    if weekday != 0 {
        return; // Only process on Mondays
    }

    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let team_expenses: Vec<(String, i64)> = game
        .teams
        .iter()
        .map(|team| {
            let wages = calc_wages(game, &team.id);
            let upkeep = calc_upkeep(team);

            (team.id.clone(), wages + upkeep)
        })
        .collect();
    let team_positions: Vec<(String, Option<u32>)> = game
        .teams
        .iter()
        .map(|team| (team.id.clone(), current_league_position(game, &team.id)))
        .collect();

    for team in game.teams.iter_mut() {
        let total_expenses = team_expenses
            .iter()
            .find(|(team_id, _)| team_id == &team.id)
            .map(|(_, total)| *total)
            .unwrap_or(0);

        team.finance -= total_expenses;
        team.season_expenses += total_expenses;

        let current_position = team_positions
            .iter()
            .find(|(team_id, _)| team_id == &team.id)
            .and_then(|(_, position)| *position);

        let sponsorship_income = team
            .sponsorship
            .as_ref()
            .map(|sponsorship| {
                sponsorship.base_value
                    + evaluate_sponsorship_bonus(current_position, &team.form, sponsorship)
            })
            .unwrap_or(0);

        if sponsorship_income > 0 {
            team.finance += sponsorship_income;
            team.season_income += sponsorship_income;
        }

        if let Some(sponsorship) = team.sponsorship.as_mut() {
            sponsorship.remaining_weeks = sponsorship.remaining_weeks.saturating_sub(1);
            if sponsorship.remaining_weeks == 0 {
                team.sponsorship = None;
            }
        }
    }

    // --- Matchday income for home matches completed in last 7 days ---
    if game.league.is_some() {
        let home_match_counts: Vec<(String, i64)> = game
            .teams
            .iter()
            .map(|team| (team.id.clone(), count_recent_home_matches(game, &team.id)))
            .collect();

        for team in game.teams.iter_mut() {
            let home_count = home_match_counts
                .iter()
                .find(|(team_id, _)| team_id == &team.id)
                .map(|(_, count)| *count)
                .unwrap_or(0);

            if home_count > 0 {
                let mut rng = rand::rng();
                let attendance_pct = rng.random_range(60..=92) as f64 / 100.0;
                let avg_ticket = rng.random_range(15..=25) as f64;
                let total_revenue = calc_matchday(
                    team.stadium_capacity,
                    home_count,
                    attendance_pct,
                    avg_ticket,
                );

                team.finance += total_revenue;
                team.season_income += total_revenue;
            }
        }
    }

    // --- Financial health warnings for user's team ---
    generate_financial_warnings(game, &today);
    apply_weekly_finance_satisfaction_pressure(game);
}

/// Generate inbox messages warning about financial issues.
fn generate_financial_warnings(game: &mut Game, today: &str) {
    let user_team_id = match &game.manager.team_id {
        Some(id) => id.clone(),
        None => return,
    };

    let team = match game.teams.iter().find(|t| t.id == user_team_id) {
        Some(t) => t,
        None => return,
    };

    let existing_ids: std::collections::HashSet<String> =
        game.messages.iter().map(|m| m.id.clone()).collect();

    let mut new_messages: Vec<InboxMessage> = Vec::new();

    let snapshot = match team_finance_snapshot(game, &user_team_id) {
        Some(snapshot) => snapshot,
        None => return,
    };

    let weekly_wages = calc_wages(game, &user_team_id);
    let annual_wages = calc_annual_wages(game, &user_team_id);
    let weeks_left = snapshot.cash_runway_weeks.unwrap_or(999);

    // Critical: finances negative
    if team.finance < 0 {
        let msg_id = format!("finance_critical_{}", today);
        if !existing_ids.contains(&msg_id) {
            new_messages.push(
                InboxMessage::new(
                    msg_id,
                    String::new(),
                    String::new(),
                    String::new(),
                    today.to_string(),
                )
                .with_category(MessageCategory::Finance)
                .with_priority(MessagePriority::Urgent)
                .with_sender_role("")
                .with_i18n(
                    "be.msg.financeCritical.subject",
                    "be.msg.financeCritical.body",
                    {
                        let mut p = std::collections::HashMap::new();
                        p.insert("amount".to_string(), format_money((-team.finance) as u64));
                        p
                    },
                )
                .with_sender_i18n("be.sender.boardOfDirectors", "be.role.chairman")
                .with_action(action(
                    "view_finances",
                    "",
                    "be.msg.action.viewFinances",
                    ActionType::NavigateTo {
                        route: "/dashboard?tab=Finances".to_string(),
                    },
                )),
            );
        }
    }
    // Warning: less than 4 weeks of runway
    else if (0..4).contains(&weeks_left) {
        let msg_id = format!("finance_warning_{}", today);
        if !existing_ids.contains(&msg_id) {
            new_messages.push(
                InboxMessage::new(
                    msg_id,
                    String::new(),
                    String::new(),
                    String::new(),
                    today.to_string(),
                )
                .with_category(MessageCategory::Finance)
                .with_priority(MessagePriority::High)
                .with_sender_role("")
                .with_i18n(
                    "be.msg.financeWarning.subject",
                    "be.msg.financeWarning.body",
                    {
                        let mut p = std::collections::HashMap::new();
                        p.insert("weeklyWages".to_string(), format_money(weekly_wages as u64));
                        p.insert("weeksLeft".to_string(), weeks_left.to_string());
                        p
                    },
                )
                .with_sender_i18n("be.sender.financialDirector", "be.role.financialDirector")
                .with_action(action(
                    "view_finances",
                    "",
                    "be.msg.action.viewFinances",
                    ActionType::NavigateTo {
                        route: "/dashboard?tab=Finances".to_string(),
                    },
                )),
            );
        }
    }
    // Over budget warning: wages exceed budget
    else if annual_wages > team.wage_budget {
        let msg_id = format!("wage_over_budget_{}", today);
        if !existing_ids.contains(&msg_id) {
            new_messages.push(
                InboxMessage::new(
                    msg_id,
                    String::new(),
                    String::new(),
                    String::new(),
                    today.to_string(),
                )
                .with_category(MessageCategory::Finance)
                .with_priority(MessagePriority::Normal)
                .with_sender_role("")
                .with_i18n(
                    "be.msg.wageOverBudget.subject",
                    "be.msg.wageOverBudget.body",
                    {
                        let mut p = std::collections::HashMap::new();
                        p.insert("annualWages".to_string(), format_money(annual_wages as u64));
                        p.insert(
                            "wageBudget".to_string(),
                            format_money(team.wage_budget as u64),
                        );
                        p
                    },
                )
                .with_sender_i18n("be.sender.financialDirector", "be.role.financialDirector")
                .with_action(action(
                    "view_finances",
                    "",
                    "be.msg.action.viewFinances",
                    ActionType::NavigateTo {
                        route: "/dashboard?tab=Finances".to_string(),
                    },
                )),
            );
        }
    }

    let penalty = weekly_finance_satisfaction_penalty(&snapshot);
    if penalty > 0 {
        let msg_id = format!("finance_board_pressure_{}", today);
        if !existing_ids.contains(&msg_id) {
            new_messages.push(finance_board_pressure_message(
                today,
                snapshot.overall_status,
                penalty,
            ));
        }
    }

    game.messages.extend(new_messages);
}

fn format_money(amount: u64) -> String {
    crate::currency::format_compact_number(amount, crate::currency::DEFAULT_CURRENCY_CODE)
        .unwrap_or_else(|| amount.to_string())
}

#[cfg(test)]
mod tests {
    use super::preview_sponsor_pitch;
    use crate::clock::GameClock;
    use crate::game::Game;
    use chrono::{TimeZone, Utc};
    use domain::league::League;
    use domain::manager::Manager;
    use domain::team::Team;

    fn make_team(id: &str, name: &str) -> Team {
        let mut team = Team::new(
            id.to_string(),
            name.to_string(),
            name[..3].to_string(),
            "England".to_string(),
            "Testville".to_string(),
            format!("{} Ground", name),
            22_000,
        );
        team.reputation = 650;
        team.finance = -25_000;
        team.wage_budget = 400_000;
        team.transfer_budget = 500_000;
        team
    }

    fn make_game() -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 2, 16, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "mgr-user".to_string(),
            "Alex".to_string(),
            "Boss".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("team1".to_string());

        let mut game = Game::new(
            clock,
            manager,
            vec![make_team("team1", "Alpha FC"), make_team("team2", "Beta FC")],
            vec![],
            vec![],
            vec![],
        );

        let mut league = League::new(
            "league-1".to_string(),
            "Premier Division".to_string(),
            2026,
            &["team1".to_string(), "team2".to_string()],
        );
        league
            .standings
            .iter_mut()
            .find(|entry| entry.team_id == "team1")
            .unwrap()
            .points = 32;
        league
            .standings
            .iter_mut()
            .find(|entry| entry.team_id == "team2")
            .unwrap()
            .points = 18;
        game.league = Some(league);
        game
    }

    #[test]
    fn preview_sponsor_pitch_rewards_stronger_league_position() {
        let game = make_game();

        let leader_pitch = preview_sponsor_pitch(&game, "team1").expect("leader pitch");
        let trailing_pitch = preview_sponsor_pitch(&game, "team2").expect("trailing pitch");

        assert!(
            leader_pitch.weekly_amount > trailing_pitch.weekly_amount,
            "A stronger league position should improve sponsor pitch value when other club factors are equal"
        );
    }
}
