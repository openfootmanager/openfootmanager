import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { useState } from "react";
import { beforeEach } from "vitest";
import { describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import type { GameStateData, PlayerData, TeamData } from "../../store/gameStore";
import PlayerProfile from "./PlayerProfile";

function hasWeeklyWage(text: string, amount: number): boolean {
  const numberPortion = amount.toLocaleString();
  return text.replace(/\s+/g, "").includes(`€${numberPortion}/wk`);
}

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
      if (key === "common.back") return "Back";
      if (key === "common.contract") return "Contract";
      if (key === "common.renewContract") return "Renew Contract";
      if (key === "common.cancel") return "Cancel";
      if (key === "common.done") return "Done";
      if (key === "common.submit") return "Submit";
      if (key === "common.condition") return "Condition";
      if (key === "common.morale") return "Morale";
      if (key === "common.value") return "Value";
      if (key === "common.wage") return "Wage";
      if (key === "common.age") return "Age";
      if (key === "common.viewTeam") return "View team";
      if (key === "common.freeAgent") return "Free Agent";
      if (key === "common.unknown") return "Unknown";
      if (key === "finances.perWeekSuffix") return "/wk";
      if (key === "finances.marketValue") return "Market Value";
      if (key === "finances.contractRiskCritical") return "Critical";
      if (key === "finances.contractRiskWarning") return "Warning";
      if (key === "finances.contractRiskStable") return "Stable";
      if (key === "finances.contractExpiresOn")
        return `Expires ${params?.date}`;
      if (key === "playerProfile.contractInfo") return "Contract Info";
      if (key === "playerProfile.dateOfBirth") return "Date of Birth";
      if (key === "playerProfile.weeklyWage") return "Weekly Wage";
      if (key === "playerProfile.noContract") return "No Contract";
      if (key === "playerProfile.yearsRemaining") return "Years Remaining";
      if (key === "playerProfile.contractRisk") return "Contract Risk";
      if (key === "playerProfile.renewalTitle") return "Renew Contract";
      if (key === "playerProfile.renewalWage") return "Offered Wage";
      if (key === "playerProfile.renewalLength") return "Contract Length";
      if (key === "playerProfile.renewalLengthYears")
        return `${params?.count} years`;
      if (key === "playerProfile.renewalSubmit") return "Submit Offer";
      if (key === "playerProfile.renewalBudgetWarning")
        return "Exceeds wage budget";
      if (key === "playerProfile.renewalInvalidWage")
        return "Enter a valid weekly wage";
      if (key === "playerProfile.renewalAccepted") return "Offer accepted";
      if (key === "playerProfile.renewalRejected") return "Offer rejected";
      if (key === "playerProfile.renewalCounter")
        return `Wants more: ${params?.wage}/wk for ${params?.years} years`;
      if (key === "playerProfile.renewalBlocked")
        return "Talks are blocked after your earlier decision";
      if (key === "playerProfile.renewalCooledOff")
        return "Previous talks cooled off, so this starts as a fresh conversation.";
      if (key === "playerProfile.letContractExpire") return "Let Expire";
      if (key === "playerProfile.reopenContractTalks") return "Reopen Talks";
      if (key === "playerProfile.terminateContract") return "Terminate Now";
      if (key === "playerProfile.terminateContractTitle")
        return "Terminate Contract";
      if (key === "playerProfile.terminateContractBody")
        return `Release ${params?.name} immediately.`;
      if (key === "playerProfile.terminationSeverance") return "Severance";
      if (key === "playerProfile.projectedHealthyPlayers")
        return "Healthy players after release";
      if (key === "playerProfile.terminationUnsafe")
        return "This release would leave the squad unable to field a matchday XI.";
      if (key === "playerProfile.confirmTerminateContract")
        return "Terminate Contract";
      if (key === "playerProfile.delegateRenewal")
        return "Delegate to Assistant";
      if (key === "playerProfile.renewalDelegateMissingReport")
        return "Assistant report did not include this player.";
      if (key === "playerProfile.renewalConversationTitle")
        return "Negotiation pulse";
      if (key === "playerProfile.renewalProjectionTitle")
        return "Projected financial impact";
      if (key === "playerProfile.renewalProjectionWageBill")
        return `Weekly wage bill ${params?.before} -> ${params?.after}`;
      if (key === "playerProfile.renewalProjectionBudgetUsage")
        return `Wage budget use ${params?.before}% -> ${params?.after}%`;
      if (key === "playerProfile.renewalProjectionRunway")
        return `Cash runway ${params?.before} -> ${params?.after}`;
      if (key === "playerProfile.renewalRound")
        return `Round ${params?.count}`;
      if (key === "playerProfile.renewalPatience") return "Patience";
      if (key === "playerProfile.renewalTension") return "Tension";
      if (key === "playerProfile.renewalFeedbackFirmHeadline")
        return "They want stronger terms before moving.";
      if (key === "playerProfile.renewalFeedbackFirmDetail")
        return "The discussion is still open, but wage level and contract length need to feel clearly worthwhile from their side.";
      if (key === "playerProfile.attributes") return "Attributes";
      if (key === "playerProfile.seasonStats") return "Season Stats";
      if (key === "playerProfile.advancedStats") return "Advanced Stats";
      if (key === "playerProfile.shots") return "Shots";
      if (key === "playerProfile.shotsOnTarget") return "Shots On Target";
      if (key === "playerProfile.passes") return "Passes";
      if (key === "playerProfile.tacklesWon") return "Tackles Won";
      if (key === "playerProfile.interceptions") return "Interceptions";
      if (key === "playerProfile.foulsCommitted") return "Fouls Committed";
      if (key === "playerProfile.per90") return "Per 90";
      if (key === "playerProfile.passAccuracy") return "Pass Accuracy";
      if (key === "playerProfile.percentile") return "Percentile";
      if (key === "playerProfile.percentileUnavailable")
        return "Percentile unavailable";
      if (key === "playerProfile.recentMatches") return "Recent Matches";
      if (key === "playerProfile.vs") return "vs";
      if (key === "playerProfile.noRecentMatches") return "No recent match data";
      if (key === "playerProfile.careerHistory") return "Career History";
      if (key === "playerProfile.noCareer") return "No Career";
      if (key === "scouting.noScoutsHint") return "Hire a scout first";
      if (key === "scouting.scoutingInProgress") return "Scouting in progress";
      if (key === "scouting.scoutBtn") return "Scout";
      if (key === "finances.wagePerWeek") return "Wage/wk";
      return key;
    },
    i18n: { language: "en" },
  }),
}));

vi.mock("../../utils/backendI18n", () => ({
  resolveBackendText: (
    _key?: string,
    fallback?: string,
    _params?: Record<string, string>,
  ) => fallback ?? "",
}));

vi.mock("../../lib/countries", () => ({
  countryName: () => "England",
  isValidCountryCode: () => true,
  normaliseNationality: (value: string) => value,
  resolveCountryFlagCode: () => "GB",
}));

function createTeam(overrides: Partial<TeamData> = {}): TeamData {
  return {
    id: "team-1",
    name: "Alpha FC",
    short_name: "ALP",
    country: "GB",
    city: "London",
    stadium_name: "Alpha Ground",
    stadium_capacity: 30000,
    finance: 500000,
    manager_id: "manager-1",
    reputation: 50,
    wage_budget: 50000,
    transfer_budget: 250000,
    season_income: 0,
    season_expenses: 0,
    formation: "4-4-2",
    play_style: "Balanced",
    training_focus: "General",
    training_intensity: "Balanced",
    training_schedule: "Balanced",
    founded_year: 1900,
    colors: { primary: "#000000", secondary: "#ffffff" },
    starting_xi_ids: [],
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
    nationality: "GB",
    position: "Forward",
    natural_position: "Forward",
    alternate_positions: [],
    training_focus: null,
    attributes: {
      pace: 60,
      stamina: 60,
      strength: 60,
      agility: 60,
      passing: 60,
      shooting: 60,
      tackling: 60,
      dribbling: 60,
      defending: 60,
      positioning: 60,
      vision: 60,
      decisions: 60,
      composure: 60,
      aggression: 60,
      teamwork: 60,
      leadership: 60,
      handling: 20,
      reflexes: 20,
      aerial: 60,
    },
    condition: 80,
    morale: 75,
    injury: null,
    team_id: "team-1",
    retired: false,
    contract_end: "2026-10-15",
    wage: 12000,
    market_value: 350000,
    stats: {
      appearances: 0,
      goals: 0,
      assists: 0,
      clean_sheets: 0,
      yellow_cards: 0,
      red_cards: 0,
      avg_rating: 0,
      minutes_played: 0,
      shots: 0,
      shots_on_target: 0,
      passes_completed: 0,
      passes_attempted: 0,
      tackles_won: 0,
      interceptions: 0,
      fouls_committed: 0,
    },
    career: [],
    transfer_listed: false,
    loan_listed: false,
    transfer_offers: [],
    traits: [],
    ...overrides,
  };
}

function createGameState(player: PlayerData): GameStateData {
  return {
    clock: {
      current_date: "2026-08-01T00:00:00Z",
      start_date: "2026-07-01T00:00:00Z",
    },
    manager: {
      id: "manager-1",
      first_name: "Jane",
      last_name: "Doe",
      date_of_birth: "1980-01-01",
      nationality: "GB",
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
    teams: [createTeam()],
    players: [player],
    staff: [],
    messages: [],
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

function createAdvancedStatsSummary() {
  return {
    percentileEligible: false,
    metrics: {
      shots: { total: 0, per90: null, percentile: null },
      shotsOnTarget: { total: 0, per90: null, percentile: null },
      passes: {
        completed: 0,
        attempted: 0,
        accuracy: null,
        percentile: null,
      },
      tacklesWon: { total: 0, per90: null, percentile: null },
      interceptions: { total: 0, per90: null, percentile: null },
      foulsCommitted: { total: 0, per90: null, percentile: null },
    },
  };
}

function defaultInvokeResponse(command: string) {
  if (command === "get_player_stats_overview") {
    return createAdvancedStatsSummary();
  }

  if (command === "get_player_match_history") {
    return [];
  }

  return createGameState(createPlayer());
}

function RenewalHarness({ initialPlayer }: { initialPlayer?: PlayerData }) {
  const [gameState, setGameState] = useState<GameStateData>(
    createGameState(initialPlayer ?? createPlayer()),
  );

  return (
    <PlayerProfile
      player={gameState.players[0]}
      gameState={gameState}
      isOwnClub
      onClose={vi.fn()}
      onGameUpdate={setGameState}
    />
  );
}

describe("PlayerProfile contract surfaces", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    vi.mocked(invoke).mockImplementation(async (command: string) =>
      defaultInvokeResponse(command),
    );
  });

  it("renders expiry date, years remaining, and contract risk for the selected player", () => {
    const player = createPlayer();
    const gameState = createGameState(player);

    render(
      <PlayerProfile
        player={player}
        gameState={gameState}
        isOwnClub
        onClose={vi.fn()}
      />,
    );

    expect(screen.getByText("Contract Info")).toBeInTheDocument();
    expect(screen.getByText("Expires 2026-10-15")).toBeInTheDocument();
    expect(screen.getByText("Years Remaining")).toBeInTheDocument();
    expect(screen.getByText("Contract Risk")).toBeInTheDocument();
    expect(screen.getByText("Critical")).toBeInTheDocument();
    expect(
      screen.getAllByText((_, element) =>
        hasWeeklyWage(element?.textContent ?? "", 230),
      ).length,
    ).toBeGreaterThan(0);
  });

  it("allows selecting the player's team from the hero header", () => {
    const player = createPlayer();
    const gameState = createGameState(player);
    const onSelectTeam = vi.fn();

    render(
      <PlayerProfile
        player={player}
        gameState={gameState}
        isOwnClub
        onClose={vi.fn()}
        onSelectTeam={onSelectTeam}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Alpha FC" }));

    expect(onSelectTeam).toHaveBeenCalledWith("team-1");
  });

  it("offers a context menu action to open the player's team", () => {
    const player = createPlayer();
    const gameState = createGameState(player);
    const onSelectTeam = vi.fn();

    render(
      <PlayerProfile
        player={player}
        gameState={gameState}
        isOwnClub
        onClose={vi.fn()}
        onSelectTeam={onSelectTeam}
      />,
    );

    fireEvent.contextMenu(screen.getByTestId("player-profile-team-link"));
    fireEvent.click(screen.getByRole("button", { name: "View team" }));

    expect(onSelectTeam).toHaveBeenCalledWith("team-1");
  });

  it("loads advanced stats from the backend overview query", async () => {
    const player = createPlayer({
      stats: {
        appearances: 10,
        goals: 4,
        assists: 3,
        clean_sheets: 0,
        yellow_cards: 1,
        red_cards: 0,
        avg_rating: 7.2,
        minutes_played: 450,
        shots: 20,
        shots_on_target: 10,
        passes_completed: 80,
        passes_attempted: 100,
        tackles_won: 9,
        interceptions: 6,
        fouls_committed: 5,
      },
    });
    const peerA = createPlayer({
      id: "player-2",
      match_name: "A. Peer",
      full_name: "Alpha Peer",
      stats: {
        ...player.stats,
        shots: 10,
        shots_on_target: 5,
        passes_completed: 70,
        passes_attempted: 100,
        tackles_won: 6,
        interceptions: 4,
        fouls_committed: 3,
      },
    });
    const peerB = createPlayer({
      id: "player-3",
      match_name: "B. Peer",
      full_name: "Bravo Peer",
      stats: {
        ...player.stats,
        shots: 15,
        shots_on_target: 8,
        passes_completed: 75,
        passes_attempted: 100,
        tackles_won: 7,
        interceptions: 5,
        fouls_committed: 4,
      },
    });
    const gameState = {
      ...createGameState(player),
      players: [player, peerA, peerB],
    };

    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === "get_player_stats_overview") {
        return {
          percentileEligible: true,
          metrics: {
            shots: { total: 33, per90: 6.6, percentile: 88 },
            shotsOnTarget: { total: 14, per90: 2.8, percentile: 81 },
            passes: {
              completed: 144,
              attempted: 180,
              accuracy: 80,
              percentile: 77,
            },
            tacklesWon: { total: 15, per90: 3, percentile: 72 },
            interceptions: { total: 11, per90: 2.2, percentile: 70 },
            foulsCommitted: { total: 8, per90: 1.6, percentile: 41 },
          },
        };
      }

      if (command === "get_player_match_history") {
        return [];
      }

      return defaultInvokeResponse(command);
    });

    render(
      <PlayerProfile
        player={player}
        gameState={gameState}
        isOwnClub
        onClose={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("get_player_stats_overview", {
        playerId: "player-1",
      });
      expect(screen.getByText("Advanced Stats")).toBeInTheDocument();
      expect(screen.getByText("Shots")).toBeInTheDocument();
      expect(screen.getAllByText("33").length).toBeGreaterThan(0);
      expect(screen.getAllByText("80%").length).toBeGreaterThan(0);
      expect(screen.getByText("88th")).toBeInTheDocument();
    });
  });

  it("loads and renders recent player match history", async () => {
    const player = createPlayer({
      stats: {
        appearances: 10,
        goals: 4,
        assists: 3,
        clean_sheets: 0,
        yellow_cards: 1,
        red_cards: 0,
        avg_rating: 7.2,
        minutes_played: 450,
        shots: 20,
        shots_on_target: 10,
        passes_completed: 80,
        passes_attempted: 100,
        tackles_won: 9,
        interceptions: 6,
        fouls_committed: 5,
      },
    });
    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === "get_player_match_history") {
        return [
          {
            fixture_id: "fixture-1",
            date: "2025-06-17",
            competition: "League",
            matchday: 2,
            opponent_team_id: "team-3",
            opponent_name: "Bravo FC",
            team_goals: 3,
            opponent_goals: 0,
            minutes_played: 88,
            goals: 2,
            assists: 1,
            shots: 5,
            shots_on_target: 3,
            rating: 8.4,
          },
        ];
      }

      return defaultInvokeResponse(command);
    });

    render(
      <PlayerProfile
        player={player}
        gameState={createGameState(player)}
        isOwnClub
        onClose={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("get_player_match_history", {
        playerId: "player-1",
        limit: 5,
      });
      expect(screen.getByText("Recent Matches")).toBeInTheDocument();
      expect(screen.getByText("Bravo FC")).toBeInTheDocument();
      expect(screen.getByText("3-0")).toBeInTheDocument();
      expect(screen.getByText("8.4")).toBeInTheDocument();
    });
  });

  it("validates renewal offers before submission", async () => {
    vi.mocked(invoke).mockImplementation(
      async (command: string, payload?: any) => {
        if (command === "preview_renewal_financial_impact") {
          const offered = Number(payload?.weeklyWage ?? 0);
          return {
            projection: {
              current_annual_wage_bill: 24000,
              projected_annual_wage_bill: 24000 - 12000 + offered,
              annual_wage_budget: 50000,
              annual_soft_cap: 55000,
              current_weekly_wage_spend: 461,
              projected_weekly_wage_spend: Math.round(
                (24000 - 12000 + offered) / 52,
              ),
              current_cash_runway_weeks: 1084,
              projected_cash_runway_weeks: 500,
              currently_over_budget: false,
              policy_allows: offered <= 55000,
            },
          };
        }

        return defaultInvokeResponse(command);
      },
    );

    render(<RenewalHarness />);

    fireEvent.click(screen.getByRole("button", { name: "Renew Contract" }));

    fireEvent.change(screen.getByLabelText("Offered Wage"), {
      target: { value: "0" },
    });

    expect(screen.getByText("Enter a valid weekly wage")).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("Offered Wage"), {
      target: { value: "60000" },
    });

    await waitFor(() => {
      expect(screen.getByText("Exceeds wage budget")).toBeInTheDocument();
      expect(
        screen.getByRole("button", { name: "Submit Offer" }),
      ).toBeDisabled();
      expect(screen.getByText("Projected financial impact")).toBeInTheDocument();
    });
  });

  it("submits a renewal offer and refreshes contract data when accepted", async () => {
    const updatedPlayer = createPlayer({
      contract_end: "2029-08-01",
      wage: 15000,
    });
    const updatedGame = createGameState(updatedPlayer);

    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === "propose_renewal") {
        return {
          outcome: "accepted",
          game: updatedGame,
          suggested_wage: null,
          suggested_years: null,
          session_status: "agreed",
          is_terminal: true,
        };
      }

      return defaultInvokeResponse(command);
    });

    render(<RenewalHarness />);

    fireEvent.click(screen.getByRole("button", { name: "Renew Contract" }));
    fireEvent.change(screen.getByLabelText("Offered Wage"), {
      target: { value: "15000" },
    });
    fireEvent.change(screen.getByLabelText("Contract Length"), {
      target: { value: "3" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Submit Offer" }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("propose_renewal", {
        playerId: "player-1",
        weeklyWage: 15000,
        contractYears: 3,
      });
    });

    await waitFor(() => {
      expect(screen.getByText("Offer accepted")).toBeInTheDocument();
      expect(screen.getByText("Expires 2029-08-01")).toBeInTheDocument();
      expect(
        screen.getAllByText((_, element) =>
          hasWeeklyWage(element?.textContent ?? "", 288),
        ).length,
      ).toBeGreaterThan(0);
      expect(screen.getByText("Stable")).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Done" })).toBeInTheDocument();
      expect(
        screen.queryByRole("button", { name: "Submit Offer" }),
      ).not.toBeInTheDocument();
    });
  });

  it("shows a rejected state when the renewal offer is turned down", async () => {
    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === "propose_renewal") {
        return {
          outcome: "rejected",
          game: createGameState(createPlayer()),
          suggested_wage: null,
          suggested_years: null,
          session_status: "stalled",
          is_terminal: false,
        };
      }

      return defaultInvokeResponse(command);
    });

    render(<RenewalHarness />);

    fireEvent.click(screen.getByRole("button", { name: "Renew Contract" }));
    fireEvent.change(screen.getByLabelText("Offered Wage"), {
      target: { value: "12000" },
    });
    fireEvent.change(screen.getByLabelText("Contract Length"), {
      target: { value: "2" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Submit Offer" }));

    await waitFor(() => {
      expect(screen.getByText("Offer rejected")).toBeInTheDocument();
    });
  });

  it("shows improved terms when the player wants more", async () => {
    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === "propose_renewal") {
        return {
          outcome: "counter_offer",
          game: createGameState(createPlayer()),
          suggested_wage: 16000,
          suggested_years: 4,
          session_status: "open",
          is_terminal: false,
          feedback: {
            mood: "firm",
            headline_key: "playerProfile.renewalFeedbackFirmHeadline",
            detail_key: "playerProfile.renewalFeedbackFirmDetail",
            tension: 58,
            patience: 64,
            round: 1,
            params: {},
          },
        };
      }

      return defaultInvokeResponse(command);
    });

    render(<RenewalHarness />);

    fireEvent.click(screen.getByRole("button", { name: "Renew Contract" }));
    fireEvent.change(screen.getByLabelText("Offered Wage"), {
      target: { value: "13000" },
    });
    fireEvent.change(screen.getByLabelText("Contract Length"), {
      target: { value: "2" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Submit Offer" }));

    await waitFor(() => {
      expect(
        screen.getByText("Wants more: €16,000/wk for 4 years"),
      ).toBeInTheDocument();
      expect(screen.getByText("Negotiation pulse")).toBeInTheDocument();
      expect(
        screen.getByText("They want stronger terms before moving."),
      ).toBeInTheDocument();
      expect(screen.getByText("Round 1")).toBeInTheDocument();
      expect(screen.getByText("Patience")).toBeInTheDocument();
      expect(screen.getByText("Tension")).toBeInTheDocument();
    });
  });

  it("shows a cooled-off notice when stale talks reset before a new offer", async () => {
    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === "propose_renewal") {
        return {
          outcome: "counter_offer",
          game: createGameState(createPlayer()),
          suggested_wage: 15500,
          suggested_years: 3,
          session_status: "open",
          is_terminal: false,
          cooled_off: true,
          feedback: {
            mood: "calm",
            headline_key: "playerProfile.renewalFeedbackCalmHeadline",
            detail_key: "playerProfile.renewalFeedbackCalmDetail",
            tension: 34,
            patience: 76,
            round: 1,
            params: {},
          },
        };
      }

      return defaultInvokeResponse(command);
    });

    render(<RenewalHarness />);

    fireEvent.click(screen.getByRole("button", { name: "Renew Contract" }));
    fireEvent.change(screen.getByLabelText("Offered Wage"), {
      target: { value: "13500" },
    });
    fireEvent.change(screen.getByLabelText("Contract Length"), {
      target: { value: "2" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Submit Offer" }));

    await waitFor(() => {
      expect(
        screen.getByText(
          "Previous talks cooled off, so this starts as a fresh conversation.",
        ),
      ).toBeInTheDocument();
      expect(screen.getByText("Round 1")).toBeInTheDocument();
    });
  });

  it("can delegate a single renewal attempt to the assistant", async () => {
    const delegatedPlayer = createPlayer({
      contract_end: "2029-08-01",
      wage: 14000,
    });
    const updatedGame = createGameState(delegatedPlayer);

    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === "delegate_renewals") {
        return {
          game: updatedGame,
          report: {
            success_count: 1,
            failure_count: 0,
            stalled_count: 0,
            cases: [
              {
                player_id: "player-1",
                player_name: "John Smith",
                status: "successful",
                agreed_wage: 14000,
                agreed_years: 3,
                note: "I was able to close this one without needing you to step in.",
              },
            ],
          },
        };
      }

      return defaultInvokeResponse(command);
    });

    render(<RenewalHarness />);

    fireEvent.click(screen.getByRole("button", { name: "Renew Contract" }));
    fireEvent.click(
      screen.getByRole("button", { name: "Delegate to Assistant" }),
    );

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("delegate_renewals", {
        playerIds: ["player-1"],
        maxWageIncreasePct: 35,
        maxContractYears: 3,
      });
    });

    await waitFor(() => {
      expect(screen.getByText("Offer accepted")).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Done" })).toBeInTheDocument();
    });
  });

  it("shows a localized error when the assistant report omits the player", async () => {
    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === "delegate_renewals") {
        return {
          game: createGameState(createPlayer()),
          report: {
            success_count: 0,
            failure_count: 0,
            stalled_count: 0,
            cases: [],
          },
        };
      }

      return defaultInvokeResponse(command);
    });

    render(<RenewalHarness />);

    fireEvent.click(screen.getByRole("button", { name: "Renew Contract" }));
    fireEvent.click(
      screen.getByRole("button", { name: "Delegate to Assistant" }),
    );

    await waitFor(() => {
      expect(
        screen.getByText("Assistant report did not include this player."),
      ).toBeInTheDocument();
    });
  });

  it("marks a contract to expire and can reopen talks", async () => {
    const markedPlayer = createPlayer({
      morale_core: {
        manager_trust: 50,
        renewal_state: {
          status: "blocked",
          manager_blocked_until: null,
          last_attempt_date: "2026-08-01",
          last_assistant_attempt_date: null,
          last_outcome: "BlockedByManager",
          conversation_round: 0,
          exit_intent: {
            kind: "let_expire",
            set_on: "2026-08-01",
            reason: "manager_profile_action",
          },
        },
      },
    });

    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === "set_contract_exit_intent") {
        return { game: createGameState(markedPlayer) };
      }

      if (command === "clear_contract_exit_intent") {
        return { game: createGameState(createPlayer()) };
      }

      return defaultInvokeResponse(command);
    });

    render(<RenewalHarness />);

    fireEvent.click(screen.getByRole("button", { name: "Let Expire" }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("set_contract_exit_intent", {
        playerId: "player-1",
        reason: "manager_profile_action",
      });
      expect(screen.getByRole("button", { name: "Reopen Talks" })).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: "Reopen Talks" }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("clear_contract_exit_intent", {
        playerId: "player-1",
      });
      expect(screen.getByRole("button", { name: "Let Expire" })).toBeInTheDocument();
    });
  });

  it("previews and confirms immediate contract termination", async () => {
    const releasedPlayer = createPlayer({
      team_id: null,
      contract_end: null,
      wage: 0,
    });

    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === "preview_contract_termination") {
        return {
          preview: {
            player_id: "player-1",
            player_name: "J. Smith",
            severance_cost: 132000,
            squad_safety: {
              team_id: "team-1",
              projected_roster_size: 11,
              healthy_players: 11,
              healthy_goalkeepers: 1,
              effective_xi_size: 11,
              can_field_matchday_squad: true,
              missing_reasons: [],
            },
          },
        };
      }

      if (command === "terminate_contract_now") {
        return {
          game: createGameState(releasedPlayer),
          severance_cost: 132000,
          squad_safety: {
            team_id: "team-1",
            projected_roster_size: 11,
            healthy_players: 11,
            healthy_goalkeepers: 1,
            effective_xi_size: 11,
            can_field_matchday_squad: true,
            missing_reasons: [],
          },
        };
      }

      return defaultInvokeResponse(command);
    });

    render(<RenewalHarness />);

    fireEvent.click(screen.getByRole("button", { name: "Terminate Now" }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("preview_contract_termination", {
        playerId: "player-1",
      });
      expect(screen.getByText("Severance")).toBeInTheDocument();
      expect(screen.getByText("11/11")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: "Terminate Contract" }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("terminate_contract_now", {
        playerId: "player-1",
      });
      expect(screen.getByText("No Contract")).toBeInTheDocument();
    });
  });
});
