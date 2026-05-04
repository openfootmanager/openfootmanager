import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import type {
  GameStateData,
  MessageData,
  PlayerData,
  TeamData,
} from "../../store/gameStore";
import FinancesTab from "./FinancesTab";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("react-i18next", () => ({
  initReactI18next: {
    type: "3rdParty",
    init: () => { },
  },
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "finances.facilities") return "Facilities";
      if (key === "finances.sponsors") return "Sponsors";
      if (key === "finances.activeSponsor") return "Active Sponsor";
      if (key === "finances.noActiveSponsor") return "No active sponsor";
      if (key === "finances.sponsorWeeklyValue")
        return `Weekly value: ${params?.amount}`;
      if (key === "finances.sponsorRemainingWeeks")
        return `${params?.count} weeks remaining`;
      if (key === "finances.pendingSponsorOffers") return "Pending Offers";
      if (key === "finances.noPendingSponsorOffers")
        return "No pending sponsor offers";
      if (key === "finances.cashFlow") return "Cash Flow";
      if (key === "finances.weeklyWageSpend") return "Weekly Wage Spend";
      if (key === "finances.weeklySponsorIncome")
        return "Weekly Sponsor Income";
      if (key === "finances.projectedWeeklyNet") return "Projected Weekly Net";
      if (key === "finances.cashRunway") return "Cash Runway";
      if (key === "finances.runwayWeeks")
        return `${params?.count} weeks at current pace`;
      if (key === "finances.runwayStable") return "Stable at current pace";
      if (key === "finances.wagePressure") return "Wage Pressure";
      if (key === "finances.wageBudgetUsed")
        return `${params?.percent}% of wage budget used`;
      if (key === "finances.contractRisk") return "Contract Risk";
      if (key === "finances.delegateMostRenewals")
        return "Delegate Most Renewals";
      if (key === "finances.delegateSelectedRenewals")
        return "Delegate Selected Renewals";
      if (key === "finances.selectAllAtRisk") return "Select all";
      if (key === "finances.delegatedRenewalsSummary")
        return `${params?.successes} done, ${params?.stalled} pending, ${params?.failures} failed`;
      if (key === "finances.contractRiskCritical") return "Critical";
      if (key === "finances.contractRiskWarning") return "Warning";
      if (key === "finances.contractRiskStable") return "Stable";
      if (key === "finances.contractExpiresOn")
        return `Expires ${params?.date}`;
      if (key === "finances.atRiskWages")
        return `${params?.amount}/wk at risk`;
      if (key === "finances.noContractRisks")
        return "No imminent contract risks";
      if (key === "common.renewContract") return "Renew Contract";
      if (key === "finances.facilityTraining") return "Training Facility";
      if (key === "finances.facilityMedical") return "Medical Facility";
      if (key === "finances.facilityScouting") return "Scouting Facility";
      if (key === "finances.facilityLevel") return `Level ${params?.level}`;
      if (key === "finances.upgradeFacility") return "Upgrade";
      if (key === "finances.insufficientFunds") return "Insufficient funds";
      if (key === "finances.nextUpgradeCost")
        return `Next upgrade: ${params?.amount}`;
      if (key === "finances.facilityTrainingEffect")
        return "Improves training quality";
      if (key === "finances.facilityMedicalEffect") return "Improves recovery";
      if (key === "finances.facilityScoutingEffect")
        return "Improves scouting reports";
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
  }),
}));

const mockedInvoke = vi.mocked(invoke);

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

function createSponsorOfferMessage(
  overrides: Partial<MessageData> = {},
): MessageData {
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
    clock: {
      current_date: "2025-01-20T00:00:00Z",
      start_date: "2025-01-01T00:00:00Z",
    },
    manager: {
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
  });

  it("renders facility cards with levels and disables upgrades when funds are insufficient", () => {
    const gameState = createGameState({ finance: 200000 });

    render(<FinancesTab gameState={gameState} />);

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
    mockedInvoke.mockResolvedValue(updatedState);

    render(
      <FinancesTab gameState={initialState} onGameUpdate={onGameUpdate} />,
    );

    const upgradeButtons = screen.getAllByRole("button", { name: "Upgrade" });
    fireEvent.click(upgradeButtons[1]);

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("upgrade_facility", {
        facility: "Medical",
      });
    });
    expect(onGameUpdate).toHaveBeenCalledWith(updatedState);
  });

  it("renders active sponsorship and pending sponsor offers", () => {
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

    render(<FinancesTab gameState={gameState} />);

    expect(screen.getByText("Sponsors")).toBeInTheDocument();
    expect(screen.getByText("Active Sponsor")).toBeInTheDocument();
    expect(screen.getByText("Acme Corp")).toBeInTheDocument();
    expect(screen.getByText("Weekly value: €125,000")).toBeInTheDocument();
    expect(screen.getByText("8 weeks remaining")).toBeInTheDocument();
    expect(screen.getByText("Pending Offers")).toBeInTheDocument();
    expect(
      screen.getByText("Sponsorship Offer — GreenTech Industries"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Receive €100,000 in sponsorship income."),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Accept the deal" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Decline politely" }),
    ).toBeInTheDocument();
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

    mockedInvoke.mockResolvedValue({
      game: updatedState,
      effect: "Offer accepted",
    });

    render(
      <FinancesTab gameState={initialState} onGameUpdate={onGameUpdate} />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Accept the deal" }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("resolve_message_action", {
        messageId: "sponsor_2025-06-15",
        actionId: "respond",
        optionId: "accept",
      });
    });

    expect(onGameUpdate).toHaveBeenCalledWith(updatedState);
  });

  it("renders a cash-flow projection panel using wages, sponsorship income, and runway", () => {
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
      [createPlayer({ wage: 2080000, market_value: 200000 })],
    );

    render(<FinancesTab gameState={gameState} />);

    expect(screen.getByText("Cash Flow")).toBeInTheDocument();
    expect(screen.getByText("Weekly Wage Spend")).toBeInTheDocument();
    expect(screen.getByText("Weekly Sponsor Income")).toBeInTheDocument();
    expect(screen.getByText("Projected Weekly Net")).toBeInTheDocument();
    expect(screen.getByText("Cash Runway")).toBeInTheDocument();
    expect(screen.getByText("€10K/wk")).toBeInTheDocument();
    expect(screen.getByText("-€30K/wk")).toBeInTheDocument();
    expect(screen.getByText("9 weeks at current pace")).toBeInTheDocument();
  });

  it("renders wage pressure and contract risk indicators for expiring players", () => {
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

    render(
      <FinancesTab gameState={gameState} onSelectPlayer={onSelectPlayer} />,
    );

    expect(screen.getAllByText("Wage Pressure").length).toBeGreaterThan(0);
    expect(screen.getByText("130% of wage budget used")).toBeInTheDocument();
    expect(screen.getByText("Contract Risk")).toBeInTheDocument();
    expect(screen.getAllByText("Alex Critical").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Ben Warning").length).toBeGreaterThan(0);
    expect(screen.getByText("Critical")).toBeInTheDocument();
    expect(screen.getByText("Warning")).toBeInTheDocument();
    expect(screen.getByText("Expires 2025-04-30")).toBeInTheDocument();
    expect(screen.getByText("Expires 2025-10-15")).toBeInTheDocument();
    expect(screen.getByText("€1,153/wk at risk")).toBeInTheDocument();
    expect(
      screen.getAllByRole("button", { name: "Renew Contract" }),
    ).toHaveLength(2);

    fireEvent.click(
      screen.getAllByRole("button", { name: "Renew Contract" })[0],
    );

    expect(onSelectPlayer).toHaveBeenCalledWith("player-critical", {
      openRenewal: true,
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
    const initialState = createGameState(
      { wage_budget: 50000 },
      [],
      riskyPlayers,
    );
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

    mockedInvoke.mockResolvedValue({
      game: updatedState,
      report: {
        success_count: 1,
        failure_count: 0,
        stalled_count: 1,
        cases: [],
      },
    });

    render(
      <FinancesTab gameState={initialState} onGameUpdate={onGameUpdate} />,
    );

    fireEvent.click(screen.getByLabelText("Select Ben Warning"));

    fireEvent.click(
      screen.getByRole("button", { name: "Delegate Selected Renewals" }),
    );

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("delegate_renewals", {
        playerIds: ["player-critical"],
        maxWageIncreasePct: 35,
        maxContractYears: 3,
      });
    });

    expect(onGameUpdate).toHaveBeenCalledWith(updatedState);
    expect(screen.getByText("1 done, 1 pending, 0 failed")).toBeInTheDocument();
  });
});
