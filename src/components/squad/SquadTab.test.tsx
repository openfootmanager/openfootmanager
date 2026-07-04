import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import type { GameStateData, PlayerData, PlayerSelectionOptions, TeamData } from "../../store/gameStore";
import SquadTab from "./SquadTab";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string | Record<string, unknown>) => {
      if (key === "common.renewContract") return "Renew Contract";
      if (key === "playerProfile.letContractExpire") return "Let Expire";
      if (key === "playerProfile.reopenContractTalks") return "Reopen Talks";
      if (key === "playerProfile.terminateContract") return "Terminate Now";
      if (key === "squad.addToLoanList") return "Add to Loan List";
      if (key === "squad.removeFromLoanList") return "Remove from Loan List";
      if (key === "transfers.loan") return "Loan";
      if (key === "transfers.transfer") return "Transfer";
      if (key === "youthAcademy.delegateToYouthAcademy")
        return "Delegate to youth academy";
      if (key === "playerProfile.yearsRemaining") return "Years Remaining";
      if (key === "finances.contractRisk") return "Contract Risk";
      if (key === "finances.contractRiskCritical") return "Critical";
      if (key === "finances.contractRiskWarning") return "Warning";
      if (key === "finances.wagePerYear") return "Wage/yr";
      if (key === "finances.perYearSuffix") return "/yr";
      if (key === "finances.contractExpiresOn")
        return `Expires ${String((fallback as Record<string, unknown> | undefined)?.date ?? "")}`;
      if (key === "playerProfile.daysRemaining")
        return `${String((fallback as Record<string, unknown> | undefined)?.count ?? "")} days remaining`;
      if (key === "playerProfile.injuryDaysShort")
        return `${String((fallback as Record<string, unknown> | undefined)?.count ?? "")}d`;
      if (key.startsWith("common.injuries."))
        return String((fallback as Record<string, unknown> | undefined)?.defaultValue ?? key);
      return typeof fallback === "string" ? fallback : key;
    },
    i18n: { language: "en" },
  }),
}));

const makePlayer = (
  id: string,
  position: string,
  overrides: Partial<PlayerData> = {},
): PlayerData => ({
  id,
  match_name: id.toUpperCase(),
  full_name: `Player ${id}`,
  date_of_birth: "1998-01-01",
  nationality: "GB",
  position,
  natural_position: position,
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
    handling: 60,
    reflexes: 60,
    aerial: 60,
  },
  condition: 100,
  morale: 80,
  injury: null,
  team_id: "team1",
  retired: false,
  contract_end: "2027-06-30",
  wage: 1000,
  market_value: 100000,
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
});

const makeTeam = (overrides: Partial<TeamData> = {}): TeamData => ({
  id: "team1",
  name: "Test FC",
  short_name: "TFC",
  country: "England",
  city: "Test City",
  stadium_name: "Test Ground",
  stadium_capacity: 20000,
  finance: 1000000,
  manager_id: "mgr1",
  reputation: 50,
  wage_budget: 100000,
  transfer_budget: 500000,
  season_income: 0,
  season_expenses: 0,
  formation: "4-4-2",
  play_style: "Balanced",
  training_focus: "General",
  training_intensity: "Balanced",
  training_schedule: "Balanced",
  founded_year: 1900,
  colors: { primary: "#00ff00", secondary: "#ffffff" },
  starting_xi_ids: [
    "gk1",
    "d1",
    "d2",
    "d3",
    "d4",
    "m1",
    "m2",
    "m3",
    "m4",
    "f1",
    "f2",
  ],
  form: [],
  history: [],
  ...overrides,
});

const makeGameState = (): GameStateData => {
  const players = [
    makePlayer("gk1", "Goalkeeper"),
    makePlayer("d1", "Center Back"),
    makePlayer("d2", "Defender"),
    makePlayer("d3", "Defender"),
    makePlayer("d4", "Defender"),
    makePlayer("m1", "Midfielder"),
    makePlayer("m2", "Midfielder"),
    makePlayer("m3", "Midfielder"),
    makePlayer("m4", "Midfielder"),
    makePlayer("f1", "Forward"),
    makePlayer("f2", "Forward"),
    makePlayer("d5", "Defender", { match_name: "Bench DEF" }),
  ];

  return {
    clock: {
      current_date: "2026-08-01",
      start_date: "2026-08-01",
    },
    manager: {
      id: "mgr1",
      first_name: "Test",
      last_name: "Manager",
      date_of_birth: "1980-01-01",
      nationality: "GB",
      reputation: 50,
      satisfaction: 50,
      fan_approval: 50,
      team_id: "team1",
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
    teams: [
      makeTeam({
        starting_xi_ids: [
          "gk1",
          "d1",
          "d2",
          "d3",
          "d4",
          "m1",
          "m2",
          "m3",
          "m4",
          "f1",
          "f2",
        ],
      }),
    ],
    players,
    staff: [],
    messages: [],
    news: [],
    league: null,
    scouting_assignments: [],
    board_objectives: [],
  };
};

describe("SquadTab", () => {
  const mockedInvoke = vi.mocked(invoke);

  beforeEach(() => {
    mockedInvoke.mockReset();
  });

  function renderSquadTab(
    gameState: GameStateData,
    opts: {
      onGameUpdate?: (g: GameStateData) => void;
      onSelectPlayer?: (id: string, options?: PlayerSelectionOptions) => void;
    } = {},
  ) {
    // Prime the initial get_squad fetch so that mockResolvedValue overrides set
    // up by mutation tests don't accidentally serve non-PlayerData[] to the squad
    // loader on mount.
    mockedInvoke.mockResolvedValueOnce(
      gameState.players.filter((p) => p.team_id === "team1"),
    );

    render(
      <SquadTab
        gameState={gameState}
        managerId="mgr1"
        onSelectPlayer={opts.onSelectPlayer ?? vi.fn()}
        onGameUpdate={opts.onGameUpdate ?? vi.fn()}
      />,
    );
  }

  it("renders only the full roster table and not the moved tactics controls", () => {
    renderSquadTab(makeGameState());

    expect(screen.getByText("squad.title")).toBeInTheDocument();
    expect(screen.getByText("Player d5")).toBeInTheDocument();
    expect(screen.queryByText("What this changes")).not.toBeInTheDocument();
    expect(screen.queryByTestId("bench-player-d5")).not.toBeInTheDocument();
    expect(screen.queryByTestId("pitch-slot-1")).not.toBeInTheDocument();
    expect(screen.queryByText("squad.planStatus")).not.toBeInTheDocument();
    expect(screen.getByText("squad.tacticalFit")).toBeInTheDocument();
    expect(screen.getByText(/squad.currentPlan/)).toBeInTheDocument();
    expect(screen.getByText("squad.coverageTitle")).toBeInTheDocument();
    expect(screen.getAllByText(/squad.needsCover/).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/squad.bestRole/).length).toBeGreaterThan(0);
    expect(
      screen.getAllByText(/squad.styleFitValues./).length,
    ).toBeGreaterThan(0);
  });

  it("shows progressive injury details in the roster", () => {
    const gameState = makeGameState();
    gameState.players[0] = makePlayer("gk1", "Goalkeeper", {
      full_name: "Injured Keeper",
      wage: 14000,
      injury: {
        name: "Knee bruise",
        days_remaining: 10,
      },
    });

    render(
      <SquadTab
        gameState={gameState}
        managerId="mgr1"
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    expect(screen.getByText("Knee bruise")).toBeInTheDocument();
    expect(screen.getByText("10d")).toBeInTheDocument();
  });

  it("honors a persisted OVR sort state from the dashboard", () => {
    const gameState = makeGameState();
    gameState.players = [
      makePlayer("low", "Forward", {
        full_name: "Low OVR",
        attributes: {
          ...makePlayer("base-low", "Forward").attributes,
          pace: 30,
          shooting: 30,
          passing: 30,
          dribbling: 30,
        },
      }),
      makePlayer("high", "Forward", {
        full_name: "High OVR",
        attributes: {
          ...makePlayer("base-high", "Forward").attributes,
          pace: 90,
          shooting: 90,
          passing: 90,
          dribbling: 90,
        },
      }),
    ];

    render(
      <SquadTab
        gameState={gameState}
        managerId="mgr1"
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
        sortState={{ sortKey: "ovr", sortDir: "desc" }}
        onSortStateChange={vi.fn()}
      />,
    );

    const rowTexts = screen.getAllByRole("row").map((row) => row.textContent ?? "");
    const highIndex = rowTexts.findIndex((text) => text.includes("High OVR"));
    const lowIndex = rowTexts.findIndex((text) => text.includes("Low OVR"));

    expect(highIndex).toBeGreaterThan(-1);
    expect(lowIndex).toBeGreaterThan(-1);
    expect(highIndex).toBeLessThan(lowIndex);
  });

  it("shows contract risk badge for expiring players and opens renewal from context menu", async () => {
    const onSelectPlayer = vi.fn();
    const gameState = makeGameState();
    gameState.clock.current_date = "2026-08-01";
    gameState.players[0].contract_end = "2026-10-15";

    renderSquadTab(gameState, { onSelectPlayer });

    expect(screen.getByText("Critical")).toBeInTheDocument();

    const playerRow = screen.getByText("Player gk1").closest("tr");
    expect(playerRow).not.toBeNull();
    fireEvent.contextMenu(playerRow as HTMLTableRowElement);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Renew Contract" })).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: "Renew Contract" }));

    expect(onSelectPlayer).toHaveBeenCalledWith("gk1", {
      openRenewal: true,
    });
  });

  it("offers contract actions from the roster context menu", async () => {
    const gameState = makeGameState();
    const onGameUpdate = vi.fn();
    const onSelectPlayer = vi.fn();
    renderSquadTab(gameState, { onGameUpdate, onSelectPlayer });
    mockedInvoke.mockResolvedValue({ game: gameState });

    const playerRow = screen.getByText("Player gk1").closest("tr");
    expect(playerRow).not.toBeNull();
    fireEvent.contextMenu(playerRow as HTMLTableRowElement);

    fireEvent.click(screen.getByRole("button", { name: "Let Expire" }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("set_contract_exit_intent", {
        playerId: "gk1",
        reason: "manager_squad_action",
      });
      expect(onGameUpdate).toHaveBeenCalledWith(gameState);
    });

    fireEvent.contextMenu(playerRow as HTMLTableRowElement);
    fireEvent.click(screen.getByRole("button", { name: "Terminate Now" }));

    expect(onSelectPlayer).toHaveBeenCalledWith("gk1", {
      openTermination: true,
    });
  });

  it("shows loan-listed status immediately after using the roster context menu", async () => {
    const gameState = makeGameState();
    const updatedGameState = {
      ...gameState,
      players: gameState.players.map((player) =>
        player.id === "gk1" ? { ...player, loan_listed: true } : player,
      ),
    };
    const onGameUpdate = vi.fn();
    renderSquadTab(gameState, { onGameUpdate });
    mockedInvoke.mockResolvedValue(updatedGameState);

    const playerRow = screen.getByText("Player gk1").closest("tr");
    expect(playerRow).not.toBeNull();
    fireEvent.contextMenu(playerRow as HTMLTableRowElement);
    fireEvent.click(screen.getByRole("button", { name: "Add to Loan List" }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("toggle_loan_list", {
        playerId: "gk1",
      });
      expect(onGameUpdate).toHaveBeenCalledWith(updatedGameState);
      expect(
        screen.getByText("Player gk1").closest("tr"),
      ).toHaveTextContent("Loan");
    });
  });

  it("reopens talks from context menu for planned-expiry players", async () => {
    const gameState = makeGameState();
    gameState.players[0].morale_core = {
      manager_trust: 50,
      renewal_state: {
        status: "Blocked",
        manager_blocked_until: null,
        last_attempt_date: "2026-08-01",
        last_assistant_attempt_date: null,
        last_outcome: "BlockedByManager",
        conversation_round: 0,
        exit_intent: {
          kind: "let_expire",
          set_on: "2026-08-01",
          reason: "manager_squad_action",
        },
      },
    };
    renderSquadTab(gameState);
    mockedInvoke.mockResolvedValue({ game: gameState });

    const playerRow = screen.getByText("Player gk1").closest("tr");
    expect(playerRow).not.toBeNull();
    fireEvent.contextMenu(playerRow as HTMLTableRowElement);
    fireEvent.click(screen.getByRole("button", { name: "Reopen Talks" }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("clear_contract_exit_intent", {
        playerId: "gk1",
      });
    });
  });

  it("delegates eligible players to the youth academy from the roster context menu", async () => {
    const gameState = makeGameState();
    gameState.players[0].date_of_birth = "2008-01-01";
    const updatedGameState = {
      ...gameState,
      players: gameState.players.map((player) =>
        player.id === "gk1" ? { ...player, squad_role: "Youth" as const } : player,
      ),
      teams: gameState.teams.map((team) => ({
        ...team,
        starting_xi_ids: team.starting_xi_ids.filter((id) => id !== "gk1"),
      })),
    };
    const onGameUpdate = vi.fn();
    renderSquadTab(gameState, { onGameUpdate });
    mockedInvoke.mockResolvedValue(updatedGameState);

    const playerRow = screen.getByText("Player gk1").closest("tr");
    expect(playerRow).not.toBeNull();
    fireEvent.contextMenu(playerRow as HTMLTableRowElement);
    fireEvent.click(
      screen.getByRole("button", { name: "Delegate to youth academy" }),
    );

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("set_player_squad_role", {
        playerId: "gk1",
        squadRole: "Youth",
      });
      expect(onGameUpdate).toHaveBeenCalledWith(updatedGameState);
    });
  });

  it("promotes a bench player into the starting xi from the roster context menu", async () => {
    const gameState = makeGameState();
    const updatedGameState = {
      ...gameState,
      teams: gameState.teams.map((team) => ({
        ...team,
        starting_xi_ids: [
          "gk1",
          "d5",
          "d2",
          "d3",
          "d4",
          "m1",
          "m2",
          "m3",
          "m4",
          "f1",
          "f2",
        ],
      })),
    };
    const onGameUpdate = vi.fn();
    renderSquadTab(gameState, { onGameUpdate });
    mockedInvoke.mockResolvedValue(updatedGameState);

    const benchRow = screen.getByText("Player d5").closest("tr");
    expect(benchRow).not.toBeNull();
    fireEvent.contextMenu(benchRow as HTMLTableRowElement);
    fireEvent.click(screen.getByRole("button", { name: "squad.makeStarter" }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("set_starting_xi", {
        playerIds: [
          "gk1",
          "d5",
          "d2",
          "d3",
          "d4",
          "m1",
          "m2",
          "m3",
          "m4",
          "f1",
          "f2",
        ],
      });
      expect(onGameUpdate).toHaveBeenCalledWith(updatedGameState);
    });
  });
});
