import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import type { GameStateData, PlayerData, TeamData } from "../../store/gameStore";
import TacticsTab from "./TacticsTab";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string | Record<string, unknown>) => {
      if (key === "playerProfile.daysRemaining") {
        return `${String((fallback as Record<string, unknown> | undefined)?.count ?? "")} days remaining`;
      }
      if (key === "playerProfile.injuryDaysShort") {
        return `${String((fallback as Record<string, unknown> | undefined)?.count ?? "")}d`;
      }
      if (key.startsWith("common.injuries.")) {
        return String((fallback as Record<string, unknown> | undefined)?.defaultValue ?? key);
      }
      return typeof fallback === "string" ? fallback : key;
    },
    i18n: { language: "en" },
  }),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

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
  starting_xi_ids: [],
  form: [],
  history: [],
  ...overrides,
});

const makeGameState = (): GameStateData => {
  const players = [
    makePlayer("gk1", "Goalkeeper"),
    makePlayer("d1", "Center Back", {
      attributes: {
        pace: 50,
        stamina: 60,
        strength: 70,
        agility: 55,
        passing: 52,
        shooting: 35,
        tackling: 75,
        dribbling: 45,
        defending: 78,
        positioning: 68,
        vision: 50,
        decisions: 63,
        composure: 64,
        aggression: 71,
        teamwork: 62,
        leadership: 60,
        handling: 10,
        reflexes: 10,
        aerial: 15,
      },
    }),
    makePlayer("d2", "Defender"),
    makePlayer("d3", "Defender"),
    makePlayer("d4", "Defender"),
    makePlayer("m1", "Midfielder", {
      attributes: {
        pace: 70,
        stamina: 74,
        strength: 58,
        agility: 75,
        passing: 79,
        shooting: 66,
        tackling: 61,
        dribbling: 77,
        defending: 57,
        positioning: 72,
        vision: 80,
        decisions: 78,
        composure: 73,
        aggression: 52,
        teamwork: 81,
        leadership: 64,
        handling: 10,
        reflexes: 10,
        aerial: 10,
      },
    }),
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

const createDataTransfer = () => {
  const data = new Map<string, string>();
  return {
    effectAllowed: "move",
    dropEffect: "move",
    setData: (type: string, value: string) => {
      data.set(type, value);
    },
    getData: (type: string) => data.get(type) ?? "",
  };
};

describe("TacticsTab", () => {
  beforeEach(() => {
    localStorage.clear();
    mockedInvoke.mockReset();
    const defaultGameState = makeGameState();
    const defaultRoster = defaultGameState.players.filter(
      (p) => p.team_id === "team1",
    );
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "get_squad") return defaultRoster;
      return defaultGameState;
    });
  });

  it("renders the top tactical controls plus bench player in the left panel", () => {
    render(
      <TacticsTab
        gameState={makeGameState()}
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    expect(screen.getByText("tactics.presetTactics")).toBeInTheDocument();
    expect(screen.getByText("tactics.formation")).toBeInTheDocument();
    expect(screen.getByText("tactics.playStyle")).toBeInTheDocument();
    expect(screen.getAllByText(/preMatch\.substitutes/).length).toBeGreaterThan(
      0,
    );
    expect(screen.getByTestId("bench-player-d5")).toBeInTheDocument();
    expect(screen.getByTestId("pitch-bench-player-d5")).toBeInTheDocument();
  });

  it("shows the compact tactics toolbar across the top of the lineup workspace", () => {
    render(
      <TacticsTab
        gameState={makeGameState()}
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    expect(screen.getByText("tactics.presetTactics")).toBeInTheDocument();
    expect(screen.getByText("tactics.activePreset")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "tactics.chooseTactic" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "tactics.formation" })).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "tactics.playStyle" })).toBeInTheDocument();
  });

  it("falls back to a custom current setup when no preset matches the active tactic", () => {
    const gameState = makeGameState();
    gameState.teams = [
      makeTeam({
        formation: "4-4-2",
        play_style: "Counter",
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
    ];

    render(
      <TacticsTab
        gameState={gameState}
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    expect(
      screen.getByRole("button", { name: "tactics.chooseTactic" }),
    ).toHaveTextContent("tactics.customTactic");
    expect(
      screen.getByRole("button", { name: "tactics.chooseTactic" }),
    ).not.toHaveTextContent("balanced-control");
  });

  it("applies a preset by updating formation and play style", async () => {
    render(
      <TacticsTab
        gameState={makeGameState()}
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(
      screen.getByRole("button", { name: "tactics.chooseTactic" }),
    );
    fireEvent.click(screen.getByRole("option", { name: /high-press/i }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("set_formation", {
        formation: "3-4-3",
      });
      expect(mockedInvoke).toHaveBeenCalledWith("set_play_style", {
        playStyle: "HighPress",
      });
    });
  });

  it("shows injured bench players with injury details in the left panel", () => {
    const gameState = makeGameState();
    const injuredBenchPlayer = gameState.players.find(
      (player) => player.id === "d5",
    );
    if (injuredBenchPlayer) {
      injuredBenchPlayer.injury = {
        name: "Ankle sprain",
        days_remaining: 6,
      };
    }

    render(
      <TacticsTab
        gameState={gameState}
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    expect(screen.getByText("Ankle sprain")).toBeInTheDocument();
    expect(screen.getByText("6d")).toBeInTheDocument();
  });

  it("keeps youth academy players out of first-team tactics selection", async () => {
    const gameState = makeGameState();
    gameState.players.push(
      makePlayer("y1", "Forward", {
        full_name: "Academy Prospect",
        squad_role: "Youth",
      }),
    );
    // Override so get_squad returns the full roster including the youth player,
    // exercising the client-side isSeniorSquadPlayer filter.
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "get_squad")
        return gameState.players.filter((p) => p.team_id === "team1");
      return gameState;
    });

    render(
      <TacticsTab
        gameState={gameState}
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(screen.queryByText("Academy Prospect")).not.toBeInTheDocument();
    });
  });

  it("sends the correct starting xi order when a pitch-view bench defender is dropped onto a defensive slot", async () => {
    render(
      <TacticsTab
        gameState={makeGameState()}
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    const benchPlayer = screen.getByTestId("pitch-bench-player-d5");
    const pitchSlot = screen.getByTestId("pitch-slot-1");
    const dataTransfer = createDataTransfer();

    fireEvent.dragStart(benchPlayer, { dataTransfer });
    fireEvent.drop(pitchSlot, { dataTransfer });

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
    });
  });

  it("does not render drag handles in the lineup tables", () => {
    render(
      <TacticsTab
        gameState={makeGameState()}
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    expect(
      screen.queryByTestId("bench-player-drag-handle-d5"),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByTestId("xi-player-drag-handle-d1"),
    ).not.toBeInTheDocument();
    expect(screen.getByTestId("pitch-bench-player-d5")).toHaveAttribute(
      "draggable",
      "true",
    );
  });

  it("shows a bench player's natural position on the pitch bench cards when it differs from position", async () => {
    const gameState = makeGameState();
    gameState.players = gameState.players.map((player) =>
      player.id === "d5"
        ? {
          ...player,
          position: "Midfielder",
          natural_position: "Defender",
        }
        : player,
    );
    // Override so get_squad returns the modified players (d5 with Midfielder
    // position), ensuring the natural position display is tested post-fetch.
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "get_squad")
        return gameState.players.filter((p) => p.team_id === "team1");
      return gameState;
    });

    render(
      <TacticsTab
        gameState={gameState}
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    const benchCard = await screen.findByTestId("pitch-bench-player-d5");

    expect(
      within(benchCard).getByText("common.posAbbr.Defender"),
    ).toBeInTheDocument();
    expect(
      within(benchCard).queryByText("common.posAbbr.Midfielder"),
    ).not.toBeInTheDocument();
  });

  it("can duplicate the current setup into a custom tactic shell", () => {
    render(
      <TacticsTab
        gameState={makeGameState()}
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(
      screen.getByRole("button", { name: "tactics.duplicateTactic" }),
    );

    expect(
      screen.getByRole("button", { name: "tactics.updateTactic" }),
    ).toBeInTheDocument();
    fireEvent.click(
      screen.getByRole("button", { name: "tactics.chooseTactic" }),
    );
    expect(screen.getByRole("option", { name: /tactics.copyOfTactic/i })).toBeInTheDocument();
  });

  it("persists custom tactics across remounts", () => {
    const gameState = makeGameState();
    const { unmount } = render(
      <TacticsTab
        gameState={gameState}
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(
      screen.getByRole("button", { name: "tactics.duplicateTactic" }),
    );

    unmount();

    render(
      <TacticsTab
        gameState={gameState}
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(
      screen.getByRole("button", { name: "tactics.chooseTactic" }),
    );

    expect(
      screen.getByRole("option", { name: /tactics.copyOfTactic/i }),
    ).toBeInTheDocument();
  });

  it("does not leak custom tactics across manager or team storage scopes", () => {
    const originalState = makeGameState();
    const otherState = makeGameState();
    otherState.clock.start_date = "2026-09-01";
    otherState.manager.id = "mgr2";
    otherState.manager.team_id = "team2";
    otherState.teams = [makeTeam({ id: "team2", manager_id: "mgr2" })];
    otherState.players = otherState.players.map((player) => ({
      ...player,
      team_id: "team2",
    }));

    const { unmount } = render(
      <TacticsTab
        gameState={originalState}
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(
      screen.getByRole("button", { name: "tactics.duplicateTactic" }),
    );

    unmount();

    const secondRender = render(
      <TacticsTab
        gameState={otherState}
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(
      screen.getByRole("button", { name: "tactics.chooseTactic" }),
    );

    expect(
      screen.queryByRole("option", { name: /tactics.copyOfTactic/i }),
    ).not.toBeInTheDocument();

    secondRender.unmount();

    render(
      <TacticsTab
        gameState={originalState}
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(
      screen.getByRole("button", { name: "tactics.chooseTactic" }),
    );

    expect(
      screen.getByRole("option", { name: /tactics.copyOfTactic/i }),
    ).toBeInTheDocument();
  }, 15000);

  it("does not mark a preset as active when applying it fails", async () => {
    const gameState = makeGameState();
    mockedInvoke.mockImplementation(async (command) => {
      if (command === "set_formation") {
        throw new Error("boom");
      }

      if (command === "get_squad") {
        return gameState.players.filter((p) => p.team_id === "team1");
      }

      return gameState;
    });

    render(
      <TacticsTab
        gameState={makeGameState()}
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    expect(
      screen.getByRole("button", { name: "tactics.chooseTactic" }),
    ).toHaveTextContent("balanced-control");

    fireEvent.click(
      screen.getByRole("button", { name: "tactics.chooseTactic" }),
    );
    fireEvent.click(screen.getByRole("option", { name: /high-press/i }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("set_formation", {
        formation: "3-4-3",
      });
    });

    expect(
      screen.getByRole("button", { name: "tactics.chooseTactic" }),
    ).toHaveTextContent("balanced-control");
  });

  it("localizes the selected player position in the comparison panel", () => {
    render(
      <TacticsTab
        gameState={makeGameState()}
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    // Modal requires two players — select f1 then m1 to open comparison
    fireEvent.click(screen.getByTestId("pitch-player-f1"));
    fireEvent.click(screen.getByTestId("pitch-player-m1"));

    expect(screen.getByText("common.positions.Forward")).toBeInTheDocument();
    expect(screen.queryByText("Forward")).not.toBeInTheDocument();
  });

  it("allows selecting a bench player from the pitch view and swapping them with a starter", async () => {
    render(
      <TacticsTab
        gameState={makeGameState()}
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByTestId("pitch-bench-player-d5"));

    // Modal only opens after both players are selected
    expect(screen.queryByText("tactics.selectedPlayer")).not.toBeInTheDocument();

    fireEvent.click(screen.getByTestId("pitch-player-d2"));

    expect(mockedInvoke).not.toHaveBeenCalledWith("set_starting_xi", expect.anything());
    expect(screen.getByText("tactics.comparePlayer")).toBeInTheDocument();

    fireEvent.click(
      screen.getByRole("button", { name: "tactics.confirmSwap" }),
    );

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("set_starting_xi", {
        playerIds: [
          "gk1",
          "d1",
          "d5",
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
    });
  });

  it("uses pitch clicks for selection and swap instead of opening the player profile", async () => {
    const onSelectPlayer = vi.fn();

    render(
      <TacticsTab
        gameState={makeGameState()}
        onSelectPlayer={onSelectPlayer}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByTestId("pitch-player-d1"));

    expect(onSelectPlayer).not.toHaveBeenCalled();
    // Modal only opens after both players are selected
    expect(screen.queryByText("tactics.selectedPlayer")).not.toBeInTheDocument();

    fireEvent.click(screen.getByTestId("pitch-player-d2"));

    expect(onSelectPlayer).not.toHaveBeenCalled();
    expect(mockedInvoke).not.toHaveBeenCalledWith("set_starting_xi", expect.anything());
    expect(screen.getByText("tactics.comparePlayer")).toBeInTheDocument();

    fireEvent.click(
      screen.getByRole("button", { name: "tactics.confirmSwap" }),
    );

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("set_starting_xi", {
        playerIds: [
          "gk1",
          "d2",
          "d1",
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
    });
  });

  it("shows a comparison panel after selecting a second pitch player", () => {
    render(
      <TacticsTab
        gameState={makeGameState()}
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByTestId("pitch-player-d1"));
    fireEvent.click(screen.getByTestId("pitch-player-m1"));

    expect(screen.getByText("tactics.comparePlayer")).toBeInTheDocument();
    expect(screen.getAllByText("Player m1").length).toBeGreaterThan(0);
    expect(
      screen.getAllByText("common.attributes.vision").length,
    ).toBeGreaterThan(0);
    expect(
      screen.getByRole("button", { name: "tactics.confirmSwap" }),
    ).toBeInTheDocument();
  });

  it("clicking a starter row in the left panel selects them for swap, not opening player profile", () => {
    const onSelectPlayer = vi.fn();

    render(
      <TacticsTab
        gameState={makeGameState()}
        onSelectPlayer={onSelectPlayer}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByTestId("xi-player-d1"));

    expect(onSelectPlayer).not.toHaveBeenCalled();
    // Modal stays closed until a second player is selected
    expect(screen.queryByText("tactics.selectedPlayer")).not.toBeInTheDocument();
  });

  it("shows all starting XI players in the left panel list", () => {
    render(
      <TacticsTab
        gameState={makeGameState()}
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    expect(screen.getByTestId("xi-player-d1")).toBeInTheDocument();
    expect(screen.getByTestId("xi-player-gk1")).toBeInTheDocument();
    expect(screen.getByTestId("xi-player-f1")).toBeInTheDocument();
  });

  // A natural striker occupying the right-midfield slot (index 8 in 4-4-2).
  // Issue #272: the left panel and the pitch role picker must follow the
  // deployed slot, which is also what the backend validates roles against.
  const makeOutOfPositionGameState = (): GameStateData => {
    const gameState = makeGameState();
    gameState.players = gameState.players.map((player) =>
      player.id === "m4"
        ? { ...player, position: "Striker", natural_position: "Striker" }
        : player,
    );
    return gameState;
  };

  it("shows the deployed slot position for a starter played out of his natural position", () => {
    const gameState = makeOutOfPositionGameState();
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "get_squad")
        return gameState.players.filter((p) => p.team_id === "team1");
      return gameState;
    });

    render(
      <TacticsTab
        gameState={gameState}
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    const row = screen.getByTestId("xi-player-m4");
    expect(
      within(row).getByText("common.posAbbr.RightMidfielder"),
    ).toBeInTheDocument();
    expect(
      within(row).queryByText("common.posAbbr.Striker"),
    ).not.toBeInTheDocument();
  });

  it("offers pitch roles for the deployed slot, not the natural position", () => {
    const gameState = makeOutOfPositionGameState();
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "get_squad")
        return gameState.players.filter((p) => p.team_id === "team1");
      return gameState;
    });

    render(
      <TacticsTab
        gameState={gameState}
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    const card = screen.getByTestId("pitch-player-m4");
    fireEvent.click(within(card).getByRole("combobox"));

    // Right-midfield roles are on offer; striker-only roles are not.
    expect(
      screen.getByRole("option", { name: "InvertedWinger" }),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("option", { name: "Poacher" }),
    ).not.toBeInTheDocument();
  });

  it("does not promote an injured bench player into the starting XI", async () => {
    const gameState = makeGameState();
    gameState.players = gameState.players.map((player) =>
      player.id === "d5"
        ? {
            ...player,
            injury: { name: "Hamstring strain", days_remaining: 7 },
          }
        : player,
    );

    render(
      <TacticsTab
        gameState={gameState}
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.contextMenu(screen.getByTestId("bench-player-d5"));

    expect(
      screen.queryByRole("button", { name: "tactics.promoteToLineup" }),
    ).not.toBeInTheDocument();
    expect(mockedInvoke).not.toHaveBeenCalledWith("set_starting_xi", expect.anything());
  });

  it("does not allow swapping an injured bench player into the starting XI", () => {
    const gameState = makeGameState();
    gameState.players = gameState.players.map((player) =>
      player.id === "d5"
        ? {
            ...player,
            injury: { name: "Hamstring strain", days_remaining: 7 },
          }
        : player,
    );

    render(
      <TacticsTab
        gameState={gameState}
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByTestId("pitch-bench-player-d5"));
    fireEvent.click(screen.getByTestId("pitch-player-d2"));

    expect(
      screen.getByRole("button", { name: "tactics.confirmSwap" }),
    ).toBeDisabled();
  });

  it("offers tactics context-menu actions to promote a bench player", async () => {
    render(
      <TacticsTab
        gameState={makeGameState()}
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    const benchRow = screen.getByTestId("bench-player-d5");
    fireEvent.contextMenu(benchRow);
    fireEvent.click(
      screen.getByRole("menuitem", { name: "tactics.promoteToLineup" }),
    );

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
    });
  });

  it("offers pitch context-menu actions to move a starter to the bench", async () => {
    render(
      <TacticsTab
        gameState={makeGameState()}
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.contextMenu(screen.getByTestId("pitch-player-d1"));
    fireEvent.click(
      screen.getByRole("menuitem", { name: "tactics.moveToBench" }),
    );

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
    });
  });

  it("assigns captaincy from the tactics table context menu", async () => {
    render(
      <TacticsTab
        gameState={makeGameState()}
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.contextMenu(screen.getByTestId("xi-player-d1"));
    fireEvent.click(
      screen.getByRole("menuitem", { name: "tactics.makeCaptain" }),
    );

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("set_team_match_roles", {
        matchRoles: expect.objectContaining({
          captain: "d1",
        }),
      });
    });
  });

  it("assigns captaincy from the pitch context menu", async () => {
    render(
      <TacticsTab
        gameState={makeGameState()}
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.contextMenu(screen.getByTestId("pitch-player-d1"));
    fireEvent.click(
      screen.getByRole("menuitem", { name: "tactics.makeCaptain" }),
    );

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("set_team_match_roles", {
        matchRoles: expect.objectContaining({
          captain: "d1",
        }),
      });
    });
  });

  it("persists default set piece and team role assignments from the right panel", async () => {
    render(
      <TacticsTab
        gameState={makeGameState()}
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(
      screen.getByRole("button", { name: "tactics.autoSelectAssignments" }),
    );

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("set_team_match_roles", {
        matchRoles: expect.objectContaining({
          captain: expect.any(String),
          vice_captain: expect.any(String),
          penalty_taker: expect.any(String),
          free_kick_taker: expect.any(String),
          corner_taker: expect.any(String),
        }),
      });
    });
  });
});
