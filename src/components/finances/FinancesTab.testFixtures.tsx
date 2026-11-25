import { render, screen } from "@testing-library/react";
import type { ReactElement } from "react";

import type { GameStateData, MessageData, PlayerData, TeamData } from "../../store/gameStore";

import { createSeasonContext } from "../../test-utils/factories";

export function pendingPromise<T>(): Promise<T> {
  return new Promise(() => {});
}

/** The backend's per-facility upgrade verdicts in wire shape. */
export function wireFacilityUpgrades(overrides: Record<string, unknown> = {}) {
  return [
    { facility: "Training", next_cost: 500_000, can_afford: true, block_reason: null },
    { facility: "Medical", next_cost: 250_000, can_afford: true, block_reason: null },
    { facility: "Scouting", next_cost: 750_000, can_afford: true, block_reason: null },
  ].map((upgrade) => ({ ...upgrade, ...overrides }));
}

/** A backend finance snapshot in wire shape; the tab renders exactly these figures. */
export function wireSnapshot(
  overrides: Record<string, unknown> = {},
  previews: Record<string, unknown> = {},
) {
  return {
    previews,
    snapshot: {
      committed_transfer_fees: 0,
      available_transfer_budget: 300000,
      annual_wage_bill: 10000,
      weekly_wage_spend: 10000,
      weekly_wage_budget: 50000,
      weekly_recurring_income: 20000,
      weekly_sponsor_income: 20000,
      projected_weekly_net: 10000,
      cash_runway_weeks: null,
      wage_budget_usage_percent: 20,
      currently_in_debt: false,
      currently_over_budget: false,
      wage_budget_status: "stable",
      runway_status: "stable",
      overall_status: "stable",
      marketing_campaign_cooldown_days_remaining: 0,
      cash_flow: [],
      ...overrides,
    },
  };
}

export async function renderFinances(ui: ReactElement) {
  const result = render(ui);
  await screen.findByText("Facilities");
  return result;
}

export function createTeam(overrides: Partial<TeamData> = {}): TeamData {
  return {
    id: "team-1",
    name: "Alpha FC",
    short_name: "ALP",
    country: "BR",
    city: "Rio",
    stadium_name: "Alpha Arena",
    stadium_capacity: 50000,
    finance: 900000,
    manager_id: "manager-1",
    reputation: 50,
    wage_budget: 50000,
    transfer_budget: 300000,
    season_income: 1000000,
    season_expenses: 500000,
    formation: "4-4-2",
    play_style: "Balanced",
    training_focus: "Physical",
    training_intensity: "Medium",
    training_schedule: "Balanced",
    founded_year: 1900,
    colors: {
      primary: "#111111",
      secondary: "#ffffff",
    },
    facilities: {
      training: 2,
      medical: 1,
      scouting: 3,
    },
    starting_xi_ids: [],
    match_roles: {
      captain: null,
      vice_captain: null,
      penalty_taker: null,
      free_kick_taker: null,
      corner_taker: null,
    },
    form: [],
    history: [],
    ...overrides,
  };
}

export function createPlayer(overrides: Partial<PlayerData> = {}): PlayerData {
  return {
    id: "player-1",
    match_name: "J. Smith",
    full_name: "John Smith",
    date_of_birth: "2000-01-01",
    nationality: "BR",
    position: "Forward",
    natural_position: "Forward",
    alternate_positions: [],
    training_focus: null,
    attributes: {
      pace: 10,
      stamina: 10,
      strength: 10,
      agility: 10,
      passing: 10,
      shooting: 10,
      tackling: 10,
      dribbling: 10,
      defending: 10,
      positioning: 10,
      vision: 10,
      decisions: 10,
      composure: 10,
      aggression: 10,
      teamwork: 10,
      leadership: 10,
      handling: 10,
      reflexes: 10,
      aerial: 10,
    },
    condition: 80,
    morale: 80,
    injury: null,
    team_id: "team-1",
    retired: false,
    contract_end: null,
    wage: 1000,
    market_value: 200000,
    stats: {
      appearances: 0,
      goals: 0,
      assists: 0,
      clean_sheets: 0,
      yellow_cards: 0,
      red_cards: 0,
      avg_rating: 0,
      minutes_played: 0,
    },
    career: [],
    transfer_listed: false,
    loan_listed: false,
    transfer_offers: [],
    traits: [],
    ...overrides,
  };
}

export function createSponsorOfferMessage(overrides: Partial<MessageData> = {}): MessageData {
  return {
    id: "sponsor_2025-06-15",
    subject: "Sponsorship Offer — GreenTech Industries",
    body: "GreenTech Industries want to sponsor your club.",
    sender: "Commercial Director",
    sender_role: "Commercial Director",
    date: "2025-06-15",
    read: false,
    category: "Finance",
    priority: "Normal",
    actions: [
      {
        id: "respond",
        label: "Respond",
        action_type: {
          ChooseOption: {
            options: [
              {
                id: "accept",
                label: "Accept the deal",
                description: "Receive €100,000 in sponsorship income.",
              },
              {
                id: "decline",
                label: "Decline politely",
                description: "Turn down the offer.",
              },
            ],
          },
        },
        resolved: false,
      },
    ],
    context: {
      team_id: null,
      player_id: null,
      fixture_id: null,
      match_result: null,
    },
    ...overrides,
  };
}

export function createGameState(
  teamOverrides: Partial<TeamData> = {},
  messages: MessageData[] = [],
  players: PlayerData[] = [createPlayer()],
): GameStateData {
  return {
    season_context: createSeasonContext(),
    clock: {
      current_date: "2025-01-20T00:00:00Z",
      start_date: "2025-01-01T00:00:00Z",
    },
    manager: {
      display_name: "Jane Doe",
      id: "manager-1",
      first_name: "Jane",
      last_name: "Doe",
      date_of_birth: "1980-01-01",
      nationality: "BR",
      reputation: 50,
      satisfaction: 50,
      fan_approval: 50,
      team_id: "team-1",
      career_stats: {
        matches_managed: 0,
        wins: 0,
        draws: 0,
        losses: 0,
        trophies: 0,
        best_finish: null,
      },
      career_history: [],
    },
    teams: [createTeam(teamOverrides)],
    players,
    staff: [],
    messages,
    news: [],
    league: {
      id: "league-1",
      name: "League",
      season: 1,
      fixtures: [],
      standings: [],
    },
    scouting_assignments: [],
    board_objectives: [],
  };
}
export const translateFinanceTestKey = (key: string, params?: Record<string, string | number>) => {
  if (key === "finances.facilities") return "Facilities";
  if (key === "finances.sponsors") return "Sponsors";
  if (key === "finances.activeSponsor") return "Active Sponsor";
  if (key === "finances.noActiveSponsor") return "No active sponsor";
  if (key === "finances.sponsorWeeklyValue") return `Weekly value: ${params?.amount}`;
  if (key === "finances.sponsorRemainingWeeks") return `${params?.count} weeks remaining`;
  if (key === "finances.pendingSponsorOffers") return "Pending Offers";
  if (key === "finances.noPendingSponsorOffers") return "No pending sponsor offers";
  if (key === "finances.pitchSponsor") return "Pitch Sponsor";
  if (key === "finances.sponsorPitchDescription")
    return "Ask the commercial team to chase a short-term sponsor deal.";
  if (key === "finances.marketingCampaign") return "Marketing Campaign";
  if (key === "finances.launchMarketingCampaign") return "Launch Campaign";
  if (key === "finances.marketingCampaignDescription")
    return "Push a one-off merchandise and outreach campaign for immediate cash.";
  if (key === "finances.marketingCampaignUnavailable")
    return "Marketing campaigns are reserved for clubs under wage or cash pressure.";
  if (key === "finances.marketingCampaignCoolingDown")
    return `Marketing campaign available again in ${params?.days} days`;
  if (key === "finances.marketingCampaignSummary")
    return `Campaign netted ${params?.netIncome} after ${params?.cost} in spend (${params?.grossRevenue} gross). Cooldown: ${params?.days} days`;
  if (key === "finances.sponsorPitchUnavailable")
    return "Sponsor pitches are reserved for clubs under wage or cash pressure.";
  if (key === "finances.sponsorPitchActiveSponsor")
    return "An active sponsorship is already in place.";
  if (key === "finances.sponsorPitchPendingOffer") return "Review the pending offer first.";
  if (key === "finances.sponsorPitchSummary")
    return `${params?.sponsor} will pay ${params?.amount} for ${params?.weeks} weeks`;
  if (key === "finances.boardSupportSummary")
    return `Board could inject ${params?.amount}, cut transfer budget by ${params?.transferBudgetReduction}, confidence -${params?.satisfactionPenalty}`;
  if (key === "finances.cashFlow") return "Cash Flow";
  if (key === "finances.weeklyWageSpend") return "Weekly Wage Spend";
  if (key === "finances.weeklySponsorIncome") return "Weekly Sponsor Income";
  if (key === "finances.projectedWeeklyNet") return "Projected Weekly Net";
  if (key === "finances.cashRunway") return "Cash Runway";
  if (key === "finances.runwayWeeks") return `${params?.count} weeks at current pace`;
  if (key === "finances.runwayStable") return "Stable at current pace";
  if (key === "finances.wagePressure") return "Wage Pressure";
  if (key === "finances.wageBudgetUsed") return `${params?.percent}% of wage budget used`;
  if (key === "finances.contractRisk") return "Contract Risk";
  if (key === "finances.delegateMostRenewals") return "Delegate Most Renewals";
  if (key === "finances.delegateSelectedRenewals") return "Delegate Selected Renewals";
  if (key === "finances.selectAllAtRisk") return "Select all";
  if (key === "finances.delegatedRenewalsSummary")
    return `${params?.successes} done, ${params?.stalled} pending, ${params?.failures} failed`;
  if (key === "finances.contractRiskCritical") return "Critical";
  if (key === "finances.contractRiskWarning") return "Warning";
  if (key === "finances.contractRiskStable") return "Stable";
  if (key === "finances.contractExpiresOn") return `Expires ${params?.date}`;
  if (key === "finances.atRiskWages") return `${params?.amount}/wk at risk`;
  if (key === "finances.noContractRisks") return "No imminent contract risks";
  if (key === "finances.selectRiskPlayer") return `Select ${params?.player}`;
  if (key === "common.renewContract") return "Renew Contract";
  if (key === "finances.facilityTraining") return "Training Facility";
  if (key === "finances.facilityMedical") return "Medical Facility";
  if (key === "finances.facilityScouting") return "Scouting Facility";
  if (key === "finances.facilityLevel") return `Level ${params?.level}`;
  if (key === "finances.upgradeFacility") return "Upgrade";
  if (key === "finances.insufficientFunds") return "Insufficient funds";
  if (key === "finances.nextUpgradeCost") return `Next upgrade: ${params?.amount}`;
  if (key === "finances.facilityTrainingEffect") return "Improves training quality";
  if (key === "finances.facilityMedicalEffect") return "Improves recovery";
  if (key === "finances.facilityScoutingEffect") return "Improves scouting reports";
  if (key === "finances.overview") return "Overview";
  if (key === "finances.wageBill") return "Wage Bill";
  if (key === "finances.weeklyTotal") return "Weekly Total";
  if (key === "finances.budget") return "Budget";
  if (key === "finances.underBudget") return "Under budget";
  if (key === "finances.overBudget") return "Over budget";
  if (key === "finances.payroll") return "Payroll";
  if (key === "finances.squadValue") return "Squad Value";
  if (key === "finances.clubBalance") return "Club Balance";
  if (key === "finances.wageBudget") return "Wage Budget";
  if (key === "finances.transferBudget") return "Transfer Budget";
  if (key === "finances.seasonIncome") return "Season Income";
  if (key === "finances.seasonExpenses") return "Season Expenses";
  if (key === "finances.perWeekSuffix") return "/wk";
  if (key === "finances.wagePerWeek") return "Wage/wk";
  if (key === "finances.marketValue") return "Market Value";
  if (key === "finances.until") return `Until ${params?.year}`;
  if (key === "common.player") return "Player";
  if (key === "common.position") return "Position";
  if (key === "common.contract") return "Contract";
  if (key === "common.noTeam") return "No team";
  return key;
};
