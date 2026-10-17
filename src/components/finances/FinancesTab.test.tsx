import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { Profiler, type ReactElement } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import type { GameStateData, MessageData, PlayerData, TeamData } from "../../store/gameStore";
import { resolveBackendError } from "../../utils/backendI18n";
import FinancesTab from "./FinancesTab";
import { createSeasonContext } from "../../test-utils/factories";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("react-i18next", async () => {
  const { useRef } = await import("react");

  return {
    initReactI18next: {
      type: "3rdParty",
      init: () => {},
    },
    useTranslation: () => {
      // The real useTranslation consumes context and state, so it always costs
      // at least one hook slot. The mock has to do the same. A hook-free mock
      // lets a component that returns before its useState calls render *zero*
      // hooks on the guard path, and React then re-takes the mount path on the
      // next render instead of reporting the hook-count change — which silently
      // hides Rules-of-Hooks bugs from every test in this file.
      useRef(null);

      return {
        t: (key: string, params?: Record<string, string | number>) => {
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
        },
        i18n: { language: "en" },
      };
    },
  };
});

const mockedInvoke = vi.mocked(invoke);

function pendingPromise<T>(): Promise<T> {
  return new Promise(() => {});
}

/** The backend's per-facility upgrade verdicts in wire shape. */
function wireFacilityUpgrades(overrides: Record<string, unknown> = {}) {
  return [
    { facility: "Training", next_cost: 500_000, can_afford: true, block_reason: null },
    { facility: "Medical", next_cost: 250_000, can_afford: true, block_reason: null },
    { facility: "Scouting", next_cost: 750_000, can_afford: true, block_reason: null },
  ].map((upgrade) => ({ ...upgrade, ...overrides }));
}

/** A backend finance snapshot in wire shape; the tab renders exactly these figures. */
function wireSnapshot(
  overrides: Record<string, unknown> = {},
  previews: Record<string, unknown> = {},
) {
  return {
    previews,
    snapshot: {
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
      ...overrides,
    },
  };
}

async function renderFinances(ui: ReactElement) {
  const result = render(ui);
  await screen.findByText("Facilities");
  return result;
}

function createTeam(overrides: Partial<TeamData> = {}): TeamData {
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

function createPlayer(overrides: Partial<PlayerData> = {}): PlayerData {
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

function createSponsorOfferMessage(overrides: Partial<MessageData> = {}): MessageData {
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

function createGameState(
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

describe("FinancesTab facilities", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
    mockedInvoke.mockImplementation((command) => {
      if (command === "get_finance_snapshot") {
        return Promise.resolve(wireSnapshot());
      }

      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });
  });

  it("renders facility cards with levels and disables upgrades when funds are insufficient", async () => {
    const gameState = createGameState({ finance: 200000 });
    mockedInvoke.mockResolvedValue(
      wireSnapshot({}, { facility_upgrades: wireFacilityUpgrades({ can_afford: false }) }),
    );

    await renderFinances(<FinancesTab gameState={gameState} />);

    expect(screen.getByText("Facilities")).toBeInTheDocument();
    expect(screen.getByText("Training Facility")).toBeInTheDocument();
    expect(screen.getByText("Medical Facility")).toBeInTheDocument();
    expect(screen.getByText("Scouting Facility")).toBeInTheDocument();
    expect(screen.getByText("Level 2")).toBeInTheDocument();
    expect(screen.getByText("Level 1")).toBeInTheDocument();
    expect(screen.getByText("Level 3")).toBeInTheDocument();

    const upgradeButtons = screen.getAllByRole("button", { name: "Upgrade" });
    expect(upgradeButtons).toHaveLength(3);
    expect(upgradeButtons[0]).toBeDisabled();
    expect(upgradeButtons[1]).toBeDisabled();
    expect(upgradeButtons[2]).toBeDisabled();
    expect(screen.getAllByText("Insufficient funds")).toHaveLength(3);
  });

  it("invokes facility upgrade and publishes the updated game state", async () => {
    const initialState = createGameState();
    const updatedState = createGameState({
      finance: 650000,
      facilities: {
        training: 2,
        medical: 2,
        scouting: 3,
      },
      season_expenses: 750000,
    });
    const onGameUpdate = vi.fn();
    mockedInvoke.mockImplementation((command) => {
      if (command === "get_finance_snapshot") {
        return Promise.resolve(wireSnapshot({}, { facility_upgrades: wireFacilityUpgrades() }));
      }

      if (command === "upgrade_facility") {
        return Promise.resolve(updatedState);
      }

      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    await renderFinances(<FinancesTab gameState={initialState} onGameUpdate={onGameUpdate} />);

    const upgradeButtons = screen.getAllByRole("button", { name: "Upgrade" });
    fireEvent.click(upgradeButtons[1]);

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("upgrade_facility", {
        careerToken: 0,
        facility: "Medical",
      });
    });
    expect(onGameUpdate).toHaveBeenCalledWith(updatedState);
  });

  it("renders active sponsorship and pending sponsor offers", async () => {
    const gameState = createGameState(
      {
        sponsorship: {
          sponsor_name: "Acme Corp",
          base_value: 125000,
          remaining_weeks: 8,
          bonus_criteria: [],
        },
      },
      [createSponsorOfferMessage()],
    );

    await renderFinances(<FinancesTab gameState={gameState} />);

    expect(screen.getByText("Sponsors")).toBeInTheDocument();
    expect(screen.getByText("Active Sponsor")).toBeInTheDocument();
    expect(screen.getByText("Acme Corp")).toBeInTheDocument();
    expect(screen.getByText("Weekly value: €125,000")).toBeInTheDocument();
    expect(screen.getByText("8 weeks remaining")).toBeInTheDocument();
    expect(screen.getByText("Pending Offers")).toBeInTheDocument();
    expect(screen.getByText("Sponsorship Offer — GreenTech Industries")).toBeInTheDocument();
    expect(screen.getByText("Receive €100,000 in sponsorship income.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Accept the deal" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Decline politely" })).toBeInTheDocument();
  });

  it("accepts a sponsor offer through resolve_message_action and publishes the updated state", async () => {
    const initialState = createGameState({}, [createSponsorOfferMessage()]);
    const updatedState = createGameState(
      {
        sponsorship: {
          sponsor_name: "GreenTech Industries",
          base_value: 100000,
          remaining_weeks: 12,
          bonus_criteria: [],
        },
      },
      [],
    );
    const onGameUpdate = vi.fn();

    mockedInvoke.mockImplementation((command) => {
      if (command === "get_finance_snapshot") {
        return Promise.resolve(wireSnapshot());
      }

      if (command === "resolve_message_action") {
        return Promise.resolve({
          game: updatedState,
          effect: "Offer accepted",
        });
      }

      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    await renderFinances(<FinancesTab gameState={initialState} onGameUpdate={onGameUpdate} />);

    fireEvent.click(screen.getByRole("button", { name: "Accept the deal" }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("resolve_message_action", {
        careerToken: 0,
        messageId: "sponsor_2025-06-15",
        actionId: "respond",
        optionId: "accept",
      });
    });

    expect(onGameUpdate).toHaveBeenCalledWith(updatedState);
  });

  it("requests a sponsor pitch for a pressured club and publishes the updated state", async () => {
    const initialState = createGameState(
      { wage_budget: 50000 },
      [],
      [createPlayer({ wage: 5200000 })],
    );
    const updatedState = createGameState(
      { wage_budget: 50000 },
      [createSponsorOfferMessage()],
      [createPlayer({ wage: 5200000 })],
    );
    const onGameUpdate = vi.fn();

    mockedInvoke.mockImplementation((command) => {
      if (command === "get_finance_snapshot") {
        return Promise.resolve({
          snapshot: {
            annual_wage_bill: 5200000,
            weekly_wage_spend: 100000,
            weekly_wage_budget: 962,
            weekly_recurring_income: 0,
            weekly_sponsor_income: 0,
            projected_weekly_net: -100000,
            cash_runway_weeks: 9,
            wage_budget_usage_percent: 10400,
            currently_in_debt: false,
            currently_over_budget: true,
            wage_budget_status: "critical",
            runway_status: "watch",
            overall_status: "critical",
            marketing_campaign_cooldown_days_remaining: 0,
          },
          previews: {
            sponsor_pitch: {
              sponsor_name: "Summit Capital",
              weekly_amount: 85000,
              duration_weeks: 12,
            },
          },
        });
      }

      if (command === "request_sponsor_pitch") {
        return Promise.resolve({
          game: updatedState,
          result: {
            message_id: "sponsor_pitch_2025-01-20",
            sponsor_name: "Summit Capital",
            weekly_amount: 85000,
            duration_weeks: 12,
          },
        });
      }

      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    await renderFinances(<FinancesTab gameState={initialState} onGameUpdate={onGameUpdate} />);

    fireEvent.click(screen.getByRole("button", { name: "Pitch Sponsor" }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("request_sponsor_pitch", { careerToken: 0 });
    });

    expect(onGameUpdate).toHaveBeenCalledWith(updatedState);
    // The preview and the result summary read the same, so both are on screen.
    expect(screen.getAllByText("Summit Capital will pay €85,000 for 12 weeks")).toHaveLength(2);
  });

  it("launches a marketing campaign for a pressured club and publishes the updated state", async () => {
    const initialState = createGameState(
      { wage_budget: 50000, finance: -30000 },
      [],
      [createPlayer({ wage: 5200000 })],
    );
    const updatedState = createGameState(
      { wage_budget: 50000, finance: 82500 },
      [],
      [createPlayer({ wage: 5200000 })],
    );
    const onGameUpdate = vi.fn();

    mockedInvoke.mockImplementation((command) => {
      if (command === "get_finance_snapshot") {
        return Promise.resolve({
          snapshot: {
            annual_wage_bill: 5200000,
            weekly_wage_spend: 100000,
            weekly_wage_budget: 962,
            weekly_recurring_income: 0,
            weekly_sponsor_income: 0,
            projected_weekly_net: -100000,
            cash_runway_weeks: 3,
            wage_budget_usage_percent: 10400,
            currently_in_debt: true,
            currently_over_budget: true,
            wage_budget_status: "critical",
            runway_status: "critical",
            overall_status: "critical",
            marketing_campaign_cooldown_days_remaining: 0,
          },
          previews: {
            marketing_campaign: {
              gross_revenue: 150000,
              campaign_cost: 50000,
              net_income: 100000,
              cooldown_days: 28,
            },
          },
        });
      }

      if (command === "request_marketing_campaign") {
        return Promise.resolve({
          game: updatedState,
          result: {
            message_id: "marketing_campaign_2025-06-16",
            gross_revenue: 150000,
            campaign_cost: 37500,
            net_income: 112500,
            cooldown_days: 28,
          },
        });
      }

      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    await renderFinances(<FinancesTab gameState={initialState} onGameUpdate={onGameUpdate} />);

    fireEvent.click(screen.getByRole("button", { name: "Launch Campaign" }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("request_marketing_campaign", { careerToken: 0 });
    });

    expect(onGameUpdate).toHaveBeenCalledWith(updatedState);
    expect(
      screen.getByText(
        (_, node) =>
          node?.textContent ===
          "Campaign netted €112,500 after €37,500 in spend (€150,000 gross). Cooldown: 28 days",
      ),
    ).toBeInTheDocument();
  });

  it("renders recovery previews from the backend finance snapshot", async () => {
    const initialState = createGameState(
      { wage_budget: 50000, finance: -30000 },
      [],
      [createPlayer({ wage: 5200000 })],
    );

    mockedInvoke.mockImplementation((command) => {
      if (command === "get_finance_snapshot") {
        return Promise.resolve({
          snapshot: {
            annual_wage_bill: 5200000,
            weekly_wage_spend: 100000,
            weekly_wage_budget: 962,
            weekly_recurring_income: 0,
            weekly_sponsor_income: 0,
            projected_weekly_net: -100000,
            cash_runway_weeks: 3,
            wage_budget_usage_percent: 10400,
            currently_in_debt: true,
            currently_over_budget: true,
            wage_budget_status: "critical",
            runway_status: "critical",
            overall_status: "critical",
            marketing_campaign_cooldown_days_remaining: 0,
          },
          previews: {
            board_support: {
              support_amount: 150000,
              transfer_budget_reduction: 75000,
              satisfaction_penalty: 12,
            },
            sponsor_pitch: {
              sponsor_name: "Summit Capital",
              weekly_amount: 85000,
              duration_weeks: 12,
            },
            marketing_campaign: {
              gross_revenue: 150000,
              campaign_cost: 37500,
              net_income: 112500,
              cooldown_days: 28,
            },
          },
        });
      }

      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    await renderFinances(<FinancesTab gameState={initialState} />);

    await waitFor(() => {
      expect(
        screen.getByText(
          "Board could inject €150,000, cut transfer budget by €75,000, confidence -12",
        ),
      ).toBeInTheDocument();
    });

    expect(screen.getByText("Summit Capital will pay €85,000 for 12 weeks")).toBeInTheDocument();
    expect(
      screen.getByText(
        (_, node) =>
          node?.textContent ===
          "Campaign netted €112,500 after €37,500 in spend (€150,000 gross). Cooldown: 28 days",
      ),
    ).toBeInTheDocument();
  });

  it("disables the marketing campaign action while the campaign is cooling down", async () => {
    const initialState = createGameState(
      { wage_budget: 50000, finance: -30000 },
      [],
      [createPlayer({ wage: 5200000 })],
    );

    mockedInvoke.mockImplementation((command) => {
      if (command === "get_finance_snapshot") {
        return Promise.resolve({
          snapshot: {
            annual_wage_bill: 5200000,
            weekly_wage_spend: 100000,
            weekly_wage_budget: 962,
            weekly_recurring_income: 0,
            weekly_sponsor_income: 0,
            projected_weekly_net: -100000,
            cash_runway_weeks: 3,
            wage_budget_usage_percent: 10400,
            currently_in_debt: true,
            currently_over_budget: true,
            wage_budget_status: "critical",
            runway_status: "critical",
            overall_status: "critical",
            marketing_campaign_cooldown_days_remaining: 9,
          },
        });
      }

      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    await renderFinances(<FinancesTab gameState={initialState} />);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Launch Campaign" })).toBeDisabled();
    });

    expect(screen.getByText("Marketing campaign available again in 9 days")).toBeInTheDocument();
  });

  /**
   * Given the backend blocks every facility upgrade because the club is in financial distress,
   * and, as the control, the same club state with nothing blocked,
   * when the Finances tab shows the facilities,
   * then the blocked upgrades are disabled and say why, and the unblocked ones are enabled.
   * The rule (which statuses block) is Rust's and is tested in finances_tests.rs; this tab only
   * presents the verdict the previews carry.
   */
  it.each([
    { blockReason: "be.error.finance.facilityUpgradeCritical", blocked: true },
    { blockReason: null, blocked: false },
  ])(
    "presents the backend's facility-upgrade block (blocked: $blocked)",
    async ({ blockReason, blocked }) => {
      const gameState = createGameState({ finance: 1000000 });
      mockedInvoke.mockImplementation((command) => {
        if (command === "get_finance_snapshot") {
          return Promise.resolve(
            wireSnapshot(
              { runway_status: "warning", overall_status: "warning" },
              { facility_upgrades: wireFacilityUpgrades({ block_reason: blockReason }) },
            ),
          );
        }

        return Promise.reject(new Error(`Unexpected command: ${command}`));
      });

      await renderFinances(<FinancesTab gameState={gameState} />);

      const upgradeButtons = await screen.findAllByRole("button", { name: "Upgrade" });
      expect(upgradeButtons).toHaveLength(3);
      await waitFor(() => {
        for (const button of upgradeButtons) {
          if (blocked) expect(button).toBeDisabled();
          else expect(button).toBeEnabled();
        }
      });
      const reason = resolveBackendError("be.error.finance.facilityUpgradeCritical");
      expect(screen.queryAllByText(reason)).toHaveLength(blocked ? 3 : 0);
    },
  );

  it("renders a cash-flow projection panel using wages, sponsorship income, and runway", async () => {
    const gameState = createGameState(
      {
        finance: 280000,
        sponsorship: {
          sponsor_name: "Acme Corp",
          base_value: 10000,
          remaining_weeks: 8,
          bonus_criteria: [],
        },
      },
      [],
      [createPlayer({ wage: 40000, market_value: 200000 })],
    );

    mockedInvoke.mockResolvedValue(
      wireSnapshot({
        weekly_wage_spend: 40000,
        weekly_sponsor_income: 10000,
        projected_weekly_net: -30000,
        cash_runway_weeks: 9,
      }),
    );

    await renderFinances(<FinancesTab gameState={gameState} />);

    expect(screen.getByText("Cash Flow")).toBeInTheDocument();
    expect(screen.getByText("Weekly Wage Spend")).toBeInTheDocument();
    expect(screen.getByText("Weekly Sponsor Income")).toBeInTheDocument();
    expect(screen.getByText("Projected Weekly Net")).toBeInTheDocument();
    expect(screen.getByText("Cash Runway")).toBeInTheDocument();
    expect(screen.getByText("€10K/wk")).toBeInTheDocument();
    expect(screen.getByText("-€30K/wk")).toBeInTheDocument();
    expect(screen.getByText("9 weeks at current pace")).toBeInTheDocument();
  });

  it("renders wage pressure and contract risk indicators for expiring players", async () => {
    const onSelectPlayer = vi.fn();
    const gameState = createGameState(
      {
        wage_budget: 50000,
      },
      [],
      [
        createPlayer({
          id: "player-critical",
          full_name: "Alex Critical",
          wage: 35000,
          contract_end: "2025-04-30",
        }),
        createPlayer({
          id: "player-warning",
          full_name: "Ben Warning",
          wage: 25000,
          contract_end: "2025-10-15",
        }),
        createPlayer({
          id: "player-stable",
          full_name: "Carl Stable",
          wage: 5000,
          contract_end: "2027-06-30",
        }),
      ],
    );

    mockedInvoke.mockResolvedValue(
      wireSnapshot({ weekly_wage_spend: 60000, wage_budget_usage_percent: 130 }),
    );

    await renderFinances(<FinancesTab gameState={gameState} onSelectPlayer={onSelectPlayer} />);

    expect(screen.getAllByText("Wage Pressure").length).toBeGreaterThan(0);
    expect(screen.getByText("130% of wage budget used")).toBeInTheDocument();
    expect(screen.getByText("Contract Risk")).toBeInTheDocument();
    expect(screen.getAllByText("Alex Critical").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Ben Warning").length).toBeGreaterThan(0);
    expect(screen.getByText("Critical")).toBeInTheDocument();
    expect(screen.getByText("Warning")).toBeInTheDocument();
    expect(screen.getByText("Expires 2025-04-30")).toBeInTheDocument();
    expect(screen.getByText("Expires 2025-10-15")).toBeInTheDocument();
    expect(screen.getByText("€60,000/wk at risk")).toBeInTheDocument();
    expect(screen.getAllByRole("button", { name: "Renew Contract" })).toHaveLength(2);

    fireEvent.click(screen.getAllByRole("button", { name: "Renew Contract" })[0]);

    expect(onSelectPlayer).toHaveBeenCalledWith("player-critical", {
      openRenewal: true,
    });
  });

  /**
   * Given a squad with two players whose contracts are at risk
   * When the finance panel first appears on screen
   * Then both are already selected, so a click that lands before any effect has run cannot flip
   * the default (the delegate test below clicks the moment "Facilities" shows).
   */
  it("shows every at-risk player selected from the first frame they are on screen", async () => {
    const riskyPlayers = [
      createPlayer({
        id: "player-critical",
        full_name: "Alex Critical",
        contract_end: "2025-04-30",
      }),
      createPlayer({ id: "player-warning", full_name: "Ben Warning", contract_end: "2025-10-15" }),
    ];
    mockedInvoke.mockImplementation((command) =>
      command === "get_finance_snapshot"
        ? Promise.resolve(wireSnapshot())
        : Promise.reject(new Error(`Unexpected command: ${command}`)),
    );
    // A Profiler's onRender runs while a commit is being made, after the DOM has changed and before
    // that commit's passive effects, whatever the scheduler does. Reading the checkbox there sees
    // the first frame by construction rather than by how React happens to order its tasks.
    let checkedOnFirstAppearance: boolean | null = null;
    const readFirstFrame = () => {
      const box = document.querySelector<HTMLInputElement>(
        'input[aria-label="Select Ben Warning"]',
      );
      if (box && checkedOnFirstAppearance === null) checkedOnFirstAppearance = box.checked;
    };

    await renderFinances(
      <Profiler id="finances" onRender={readFirstFrame}>
        <FinancesTab gameState={createGameState({ wage_budget: 50000 }, [], riskyPlayers)} />
      </Profiler>,
    );

    expect(checkedOnFirstAppearance).toBe(true);
  });

  /**
   * Given two at-risk players, both selected
   * When "Select all" is pressed twice
   * Then the first press clears the selection and the second selects everyone again.
   */
  it("toggles every at-risk player off and back on with Select all", async () => {
    const riskyPlayers = [
      createPlayer({
        id: "player-critical",
        full_name: "Alex Critical",
        contract_end: "2025-04-30",
      }),
      createPlayer({ id: "player-warning", full_name: "Ben Warning", contract_end: "2025-10-15" }),
    ];
    mockedInvoke.mockImplementation((command) =>
      command === "get_finance_snapshot"
        ? Promise.resolve(wireSnapshot())
        : Promise.reject(new Error(`Unexpected command: ${command}`)),
    );
    await renderFinances(
      <FinancesTab gameState={createGameState({ wage_budget: 50000 }, [], riskyPlayers)} />,
    );
    const critical = screen.getByLabelText("Select Alex Critical");
    const warning = screen.getByLabelText("Select Ben Warning");

    fireEvent.click(screen.getByRole("button", { name: "Select all" }));
    expect(critical).not.toBeChecked();
    expect(warning).not.toBeChecked();

    fireEvent.click(screen.getByRole("button", { name: "Select all" }));
    expect(critical).toBeChecked();
    expect(warning).toBeChecked();
  });

  /**
   * Given two at-risk players, both selected
   * When Ben is switched off and then on again
   * Then the delegate call carries both players.
   */
  it("delegates a player who was switched off and then on again", async () => {
    const riskyPlayers = [
      createPlayer({
        id: "player-critical",
        full_name: "Alex Critical",
        contract_end: "2025-04-30",
      }),
      createPlayer({ id: "player-warning", full_name: "Ben Warning", contract_end: "2025-10-15" }),
    ];
    mockedInvoke.mockImplementation((command) => {
      if (command === "get_finance_snapshot") return Promise.resolve(wireSnapshot());
      if (command === "delegate_renewals") {
        return Promise.resolve({
          game: createGameState({ wage_budget: 50000 }, [], riskyPlayers),
          report: { success_count: 2, failure_count: 0, stalled_count: 0, cases: [] },
        });
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });
    await renderFinances(
      <FinancesTab gameState={createGameState({ wage_budget: 50000 }, [], riskyPlayers)} />,
    );

    fireEvent.click(screen.getByLabelText("Select Ben Warning"));
    fireEvent.click(screen.getByLabelText("Select Ben Warning"));
    fireEvent.click(screen.getByRole("button", { name: "Delegate Selected Renewals" }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("delegate_renewals", {
        careerToken: 0,
        playerIds: ["player-critical", "player-warning"],
        maxWageIncreasePct: 35,
        maxContractYears: 3,
      });
    });
  });

  it("delegates only the selected risky renewals to the assistant and publishes the updated state", async () => {
    const riskyPlayers = [
      createPlayer({
        id: "player-critical",
        full_name: "Alex Critical",
        wage: 35000,
        contract_end: "2025-04-30",
      }),
      createPlayer({
        id: "player-warning",
        full_name: "Ben Warning",
        wage: 25000,
        contract_end: "2025-10-15",
      }),
    ];
    const initialState = createGameState({ wage_budget: 50000 }, [], riskyPlayers);
    const updatedState = createGameState(
      { wage_budget: 50000 },
      [],
      [
        createPlayer({
          id: "player-critical",
          full_name: "Alex Critical",
          wage: 36000,
          contract_end: "2028-01-20",
        }),
        createPlayer({
          id: "player-warning",
          full_name: "Ben Warning",
          wage: 25000,
          contract_end: "2025-10-15",
        }),
      ],
    );
    const onGameUpdate = vi.fn();

    mockedInvoke.mockImplementation((command) => {
      if (command === "get_finance_snapshot") {
        return Promise.resolve(wireSnapshot());
      }

      if (command === "delegate_renewals") {
        return Promise.resolve({
          game: updatedState,
          report: {
            success_count: 1,
            failure_count: 0,
            stalled_count: 1,
            cases: [],
          },
        });
      }

      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    await renderFinances(<FinancesTab gameState={initialState} onGameUpdate={onGameUpdate} />);

    fireEvent.click(screen.getByLabelText("Select Ben Warning"));

    fireEvent.click(screen.getByRole("button", { name: "Delegate Selected Renewals" }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("delegate_renewals", {
        careerToken: 0,
        playerIds: ["player-critical"],
        maxWageIncreasePct: 35,
        maxContractYears: 3,
      });
    });

    expect(onGameUpdate).toHaveBeenCalledWith(updatedState);
    expect(screen.getByText("1 done, 1 pending, 0 failed")).toBeInTheDocument();
  });
});

describe("FinancesTab team guard", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
    mockedInvoke.mockImplementation((command) => {
      if (command === "get_finance_snapshot") {
        return Promise.resolve(wireSnapshot());
      }

      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });
  });

  it("shows the no-team message when the manager has no club", () => {
    const gameState = createGameState();
    gameState.manager.team_id = null;

    render(<FinancesTab gameState={gameState} />);

    expect(screen.getByText("No team")).toBeInTheDocument();
  });

  it("survives the manager gaining a team without remounting", async () => {
    const unemployed = createGameState();
    unemployed.manager.team_id = null;

    const { rerender } = render(<FinancesTab gameState={unemployed} />);
    expect(screen.getByText("No team")).toBeInTheDocument();

    // Same component instance, team now resolves. When the guard sat above the
    // hooks this rerender threw "Rendered more hooks than during the previous
    // render" because the hook count went from 1 to 11.
    rerender(<FinancesTab gameState={createGameState()} />);

    expect(screen.queryByText("No team")).not.toBeInTheDocument();
    expect(await screen.findByText("Facilities")).toBeInTheDocument();
  });
});

describe("FinancesTab backend snapshot", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
  });

  /**
   * Given the backend snapshot has not arrived
   * When the tab renders
   * Then it shows a placeholder and no figures, rather than a locally projected guess.
   */
  it("shows a placeholder until the backend snapshot arrives", () => {
    mockedInvoke.mockImplementation(() => pendingPromise());

    render(<FinancesTab gameState={createGameState()} />);

    expect(screen.getByText("common.loading")).toBeInTheDocument();
    expect(screen.queryByText("Facilities")).not.toBeInTheDocument();
  });

  /**
   * Given a rendered tab and a club whose state then changes
   * When the next snapshot is still in flight
   * Then the previous backend figures stay on screen instead of flipping to another source.
   */
  it("keeps the last backend figures while a newer snapshot is loading", async () => {
    let calls = 0;
    mockedInvoke.mockImplementation((command) => {
      if (command !== "get_finance_snapshot") {
        return Promise.reject(new Error(`Unexpected command: ${command}`));
      }
      calls += 1;
      return calls === 1
        ? Promise.resolve(wireSnapshot({ weekly_sponsor_income: 777_000 }))
        : pendingPromise();
    });

    const { rerender } = render(<FinancesTab gameState={createGameState()} />);
    await screen.findByText("Facilities");
    const sponsorFigure = screen.getByText(/777/);

    rerender(<FinancesTab gameState={createGameState({ finance: 123_456 })} />);

    await waitFor(() => expect(calls).toBe(2));
    expect(screen.getByText("Facilities")).toBeInTheDocument();
    expect(sponsorFigure).toBeInTheDocument();
    expect(screen.queryByText("common.loading")).not.toBeInTheDocument();
  });

  /**
   * Given the backend snapshot fails to load
   * When the player presses Retry
   * Then the snapshot is requested again and the tab renders.
   */
  it("offers a retry when the snapshot cannot be loaded", async () => {
    let calls = 0;
    mockedInvoke.mockImplementation((command) => {
      if (command !== "get_finance_snapshot") {
        return Promise.reject(new Error(`Unexpected command: ${command}`));
      }
      calls += 1;
      return calls === 1 ? Promise.reject(new Error("boom")) : Promise.resolve(wireSnapshot());
    });
    vi.spyOn(console, "error").mockImplementation(() => {});

    render(<FinancesTab gameState={createGameState()} />);

    expect(await screen.findByRole("alert")).toHaveTextContent("finances.snapshotLoadFailed");
    fireEvent.click(screen.getByRole("button", { name: "common.retry" }));

    expect(await screen.findByText("Facilities")).toBeInTheDocument();
  });

  /**
   * Given the backend offers board support and every facility upgrade
   * When the club state changes and the next snapshot is still in flight
   * Then no action is enabled: a verdict about the older state must not authorise anything.
   */
  it("never enables an action from previews that describe an older club state", async () => {
    let calls = 0;
    mockedInvoke.mockImplementation((command) => {
      if (command !== "get_finance_snapshot") {
        return Promise.reject(new Error(`Unexpected command: ${command}`));
      }
      calls += 1;
      return calls === 1
        ? Promise.resolve(
            wireSnapshot(
              {},
              {
                facility_upgrades: wireFacilityUpgrades(),
                board_support: {
                  support_amount: 1,
                  transfer_budget_reduction: 1,
                  satisfaction_penalty: 1,
                },
              },
            ),
          )
        : pendingPromise();
    });
    const { rerender } = render(<FinancesTab gameState={createGameState()} />);
    await screen.findByText("Facilities");
    const fresh = screen.getAllByRole("button", { name: "Upgrade" });
    expect(fresh.every((button) => !(button as HTMLButtonElement).disabled)).toBe(true);
    expect(screen.getByRole("button", { name: "finances.requestBoardSupport" })).toBeEnabled();

    rerender(<FinancesTab gameState={createGameState({ finance: 123_456 })} />);

    await waitFor(() => expect(calls).toBe(2));
    const stale = screen.getAllByRole("button", { name: "Upgrade" });
    expect(stale.every((button) => (button as HTMLButtonElement).disabled)).toBe(true);
    expect(screen.getByRole("button", { name: "finances.requestBoardSupport" })).toBeDisabled();
  });

  /**
   * Given a snapshot retained for one club
   * When the manager's club changes and the new club's snapshot has not arrived
   * Then the other club's figures are not shown.
   */
  it("does not render another club's retained snapshot", async () => {
    mockedInvoke.mockImplementationOnce(() =>
      Promise.resolve(wireSnapshot({ weekly_sponsor_income: 777_000 })),
    );
    mockedInvoke.mockImplementation(() => pendingPromise());
    const { rerender } = render(<FinancesTab gameState={createGameState()} />);
    await screen.findByText(/777/);

    const otherClub = createGameState({ id: "team-2", name: "Beta FC" });
    otherClub.manager.team_id = "team-2";
    rerender(<FinancesTab gameState={otherClub} />);

    expect(screen.queryByText(/777/)).not.toBeInTheDocument();
    expect(screen.getByText("common.loading")).toBeInTheDocument();
  });

  /**
   * Given figures on screen and a refetch that fails
   * When the tab renders
   * Then the figures stay and the failure shows with Retry, and a club change clears it.
   */
  it("shows a failed refetch beside the retained figures and clears it on a club change", async () => {
    let calls = 0;
    mockedInvoke.mockImplementation((command) => {
      if (command !== "get_finance_snapshot") {
        return Promise.reject(new Error(`Unexpected command: ${command}`));
      }
      calls += 1;
      if (calls === 1) return Promise.resolve(wireSnapshot({ weekly_sponsor_income: 777_000 }));
      if (calls === 2) return Promise.reject(new Error("boom"));
      return pendingPromise();
    });
    vi.spyOn(console, "error").mockImplementation(() => {});
    const { rerender } = render(<FinancesTab gameState={createGameState()} />);
    await screen.findByText(/777/);

    rerender(<FinancesTab gameState={createGameState({ finance: 123_456 })} />);

    expect(await screen.findByRole("alert")).toHaveTextContent("finances.snapshotLoadFailed");
    expect(screen.getByText(/777/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "common.retry" })).toBeInTheDocument();

    rerender(<FinancesTab gameState={createGameState({ finance: 99 })} />);

    await waitFor(() => expect(screen.queryByRole("alert")).not.toBeInTheDocument());
  });

  /**
   * Given figures on screen
   * When a live match finishes: the team's form changes, but the game date does not move
   * Then the snapshot is fetched again, because sponsorship and matchday figures changed.
   */
  it("refetches when a finished match changes the form on an unchanged date", async () => {
    let calls = 0;
    mockedInvoke.mockImplementation((command) => {
      if (command !== "get_finance_snapshot") {
        return Promise.reject(new Error(`Unexpected command: ${command}`));
      }
      calls += 1;
      return Promise.resolve(wireSnapshot());
    });
    const { rerender } = render(<FinancesTab gameState={createGameState({ form: [] })} />);
    await waitFor(() => expect(calls).toBe(1));

    rerender(<FinancesTab gameState={createGameState({ form: ["W"] })} />);

    await waitFor(() => expect(calls).toBe(2));
  });

  /**
   * Given figures on screen
   * When a fixture of the club completes on an unchanged date and the form is unchanged
   * Then the snapshot is fetched again: standings and matchday income moved.
   */
  it("refetches when the club's completed fixtures change on an unchanged date", async () => {
    let calls = 0;
    mockedInvoke.mockImplementation((command) => {
      if (command !== "get_finance_snapshot") {
        return Promise.reject(new Error(`Unexpected command: ${command}`));
      }
      calls += 1;
      return Promise.resolve(wireSnapshot());
    });
    const before = createGameState();
    const after = createGameState();
    after.league = {
      ...after.league!,
      fixtures: [
        {
          id: "fx-1",
          date: "2025-01-20",
          home_team_id: "team-1",
          away_team_id: "team-2",
          status: "Completed",
          result: null,
        } as unknown as NonNullable<GameStateData["league"]>["fixtures"][number],
      ],
    };
    const { rerender } = render(<FinancesTab gameState={before} />);
    await waitFor(() => expect(calls).toBe(1));

    rerender(<FinancesTab gameState={after} />);

    await waitFor(() => expect(calls).toBe(2));
  });
});
