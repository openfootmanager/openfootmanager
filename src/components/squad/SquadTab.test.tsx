import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import type {
  GameStateData,
  PlayerData,
  PlayerSelectionOptions,
  TeamData,
} from "../../store/gameStore";
import { createPlayer } from "../../test-utils/factories";
import { ratingTierClass } from "../../lib/ratingTier";
import SquadTab from "./SquadTab";
import { createSeasonContext } from "../../test-utils/factories";

// What the backend's contract warning stages read as on a badge, by player id; a test says which
// players are at risk. The hook that asks for them has its own test.
const riskLevels = vi.hoisted(() => ({
  current: {} as Record<string, "critical" | "warning" | "stable">,
}));

vi.mock("../../hooks/useContractWarningStages", () => ({
  useContractRiskLevels: () => (playerId: string) => riskLevels.current[playerId] ?? "stable",
}));

vi.mock("../../hooks/useRatingTiers", async () => {
  const { cannedRatingTiersModule } = await import("../../test-utils/ratingTiers");
  return cannedRatingTiersModule();
});

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
      if (key === "youthAcademy.delegateToYouthAcademy") return "Delegate to youth academy";
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

// Squad fixtures rate every player 60 across the board — keepers' handling and reflexes included —
// so formation fit differs only by position. The rest comes from the shared factory.
const makePlayer = (
  id: string,
  position: string,
  overrides: Partial<PlayerData> = {},
): PlayerData =>
  createPlayer({
    id,
    match_name: id.toUpperCase(),
    full_name: `Player ${id}`,
    date_of_birth: "1998-01-01",
    position,
    natural_position: position,
    attributes: { ...createPlayer().attributes, handling: 60, reflexes: 60 },
    condition: 100,
    morale: 80,
    team_id: "team1",
    wage: 1000,
    market_value: 100000,
    // The backend's verdict for a healthy senior; a test that needs another says so.
    match_day_eligible: true,
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
  starting_xi_ids: ["gk1", "d1", "d2", "d3", "d4", "m1", "m2", "m3", "m4", "f1", "f2"],
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
    season_context: createSeasonContext(),
    clock: {
      current_date: "2026-08-01",
      start_date: "2026-08-01",
    },
    manager: {
      display_name: "Test Manager",
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
        starting_xi_ids: ["gk1", "d1", "d2", "d3", "d4", "m1", "m2", "m3", "m4", "f1", "f2"],
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
    riskLevels.current = {};
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
    mockedInvoke.mockResolvedValueOnce(gameState.players.filter((p) => p.team_id === "team1"));

    render(
      <SquadTab
        gameState={gameState}
        managerId="mgr1"
        onSelectPlayer={opts.onSelectPlayer ?? vi.fn()}
        onGameUpdate={opts.onGameUpdate ?? vi.fn()}
      />,
    );
  }

  /**
   * Given a player on 30 whom the backend's squad projection rates fresh
   * When Squad lists him
   * Then his condition bar is painted in the backend's tone, not in a colour picked from 30.
   */
  it("paints each condition bar in the backend's tone", async () => {
    const state = makeGameState();
    state.players = [
      makePlayer("p1", "Forward", {
        full_name: "Rated Fresh",
        match_name: "Rated Fresh",
        condition: 30,
        condition_status: { band: "Fresh", label_key: "be.condition.fresh", tone: "good" },
      }),
    ];
    renderSquadTab(state);

    const row = await screen.findByRole("row", { name: /Rated Fresh/ });
    const bar = within(row).getByRole("progressbar", { name: "common.condition" });
    expect(bar.firstElementChild).toHaveClass("bg-tone-good-solid");
    expect(bar.firstElementChild).not.toHaveClass("bg-danger-solid");
  });

  /**
   * Given a player the backend rates tiring
   * When Squad lists him
   * Then his condition says the band's word beside the bar, so the colour isn't the only sign.
   */
  it("names each player's condition band beside the bar", async () => {
    const state = makeGameState();
    state.players = [
      makePlayer("p1", "Forward", {
        match_name: "Tiring Forward",
        condition: 64,
        condition_status: { band: "Tiring", label_key: "be.condition.tiring", tone: "fair" },
      }),
    ];
    renderSquadTab(state);

    const row = await screen.findByRole("row", { name: /Tiring Forward/ });
    expect(within(row).getByText("be.condition.tiring")).toHaveClass("text-tone-fair");
  });

  /**
   * Given the squad
   * When the roster renders
   * Then it is one table named for the squad, sorted by position by default, and its headers say
   * so (aria-sort) and ask to sort by another column.
   */
  it("draws the roster as a named table whose headers say and change its sort", async () => {
    const onSortStateChange = vi.fn();
    render(
      <SquadTab
        gameState={makeGameState()}
        managerId="mgr1"
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
        sortState={{ sortKey: "pos", sortDir: "asc" }}
        onSortStateChange={onSortStateChange}
      />,
    );

    expect(screen.getByRole("table", { name: "squad.title" })).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: /squad\.pos/ })).toHaveAttribute(
      "aria-sort",
      "ascending",
    );
    fireEvent.click(screen.getByRole("button", { name: "common.ovr" }));
    expect(onSortStateChange).toHaveBeenCalledWith({ sortKey: "ovr", sortDir: "desc" });
  });

  /**
   * Given a starter, an injured bench player and a fit bench player
   * When the roster renders
   * Then the starter's row carries the starter accent, the injured player's the can't-play one,
   * and the fit bench player's none.
   */
  it("marks starters and players who can't play with the row accent", async () => {
    const state = makeGameState();
    state.players.push(
      makePlayer("inj", "Forward", {
        match_name: "Hurt Forward",
        injury: { name: "Knee bruise", days_remaining: 10 },
        match_day_eligible: false,
      }),
    );
    renderSquadTab(state);

    expect(await screen.findByRole("row", { name: /GK1/ })).toHaveAttribute(
      "data-accent",
      "positive",
    );
    expect(screen.getByRole("row", { name: /Hurt Forward/ })).toHaveAttribute(
      "data-accent",
      "danger",
    );
    expect(screen.getByRole("row", { name: /Bench DEF/ })).not.toHaveAttribute("data-accent");
  });

  /**
   * Given a starter and an injured bench player
   * When a screen reader reads their rows
   * Then the starter's says he starts, the injured player's names his injury, and each names the
   * player's nationality: none of it hidden inside the name's open button, and none of it only a
   * colour.
   */
  it("says in words who starts, who is hurt and where each is from", async () => {
    const state = makeGameState();
    state.players.push(
      makePlayer("inj", "Forward", {
        match_name: "Hurt Forward",
        nationality: "GB",
        injury: { name: "Knee bruise", days_remaining: 10 },
        match_day_eligible: false,
      }),
    );
    renderSquadTab(state);

    const starter = await screen.findByRole("row", { name: /GK1/ });
    expect(within(starter).getByText("squad.startingXi")).toBeInTheDocument();
    const hurt = screen.getByRole("row", { name: /Hurt Forward/ });
    const injury = within(hurt).getAllByRole("img", { name: /Knee bruise/ });
    expect(injury.some((badge) => badge.closest("button") === null)).toBe(true);
    const flags = within(hurt)
      .getAllByRole("img")
      .filter((img) => img.closest("button") === null);
    expect(flags.length).toBeGreaterThanOrEqual(2);
    expect(
      within(screen.getByRole("row", { name: /Bench DEF/ })).queryByText("squad.startingXi"),
    ).toBeNull();
  });

  /**
   * Given the roster
   * When a keyboard user opens a player's menu from its button
   * Then the menu opens with its items, and Escape closes it and returns focus to the button.
   */
  it("opens a player's menu from the keyboard and gives focus back on Escape", async () => {
    renderSquadTab(makeGameState());
    const [button] = await screen.findAllByRole("button", { name: /common\.playerActions/ });
    button.focus();
    fireEvent.click(button);
    expect(await screen.findByRole("menuitem", { name: "Renew Contract" })).toBeInTheDocument();
    fireEvent.keyDown(document.activeElement ?? document.body, { key: "Escape" });
    await waitFor(() =>
      expect(screen.queryByRole("menuitem", { name: "Renew Contract" })).toBeNull(),
    );
    expect(button).toHaveFocus();
  });

  /**
   * Given two senior players rated 76 and 77, either side of the A-tier cut-off
   * When the roster renders
   * Then each rating takes the colour of its backend tier (B and A).
   */
  it("colours each rating in the roster by its backend tier", async () => {
    const state = makeGameState();
    state.players = [
      makePlayer("gk1", "Goalkeeper", { ovr: 76, match_name: "Seventy Six" }),
      makePlayer("f1", "Forward", { ovr: 77, match_name: "Seventy Seven" }),
    ];
    state.teams[0].starting_xi_ids = [];

    renderSquadTab(state);

    // Each rating is a RatingChip: the number in its tier's tone, the tier in words.
    const aTier = (await screen.findByText("77")).parentElement;
    expect(aTier).toHaveClass(...ratingTierClass("A", "badge").split(" "));
    expect(screen.getByText("76").parentElement).toHaveClass(
      ...ratingTierClass("B", "badge").split(" "),
    );
    expect(
      within(screen.getByRole("row", { name: /Seventy Seven/ })).getByText(
        "common.overallRatingTier",
      ),
    ).toBeInTheDocument();
  });

  // Given a backend-called-up youth, when Squad opens, then the manager can see him.
  it("shows called-up youth in the senior roster", async () => {
    const state = makeGameState();
    state.players = [
      makePlayer("gk1", "Goalkeeper"),
      makePlayer("called", "Forward", {
        full_name: "Called Youth",
        match_name: "Called Youth",
        squad_role: "Youth",
        match_day_eligible: true,
      }),
      makePlayer("excluded", "Forward", {
        full_name: "Excluded Youth",
        match_name: "Excluded Youth",
        squad_role: "Youth",
        match_day_eligible: false,
      }),
    ];
    renderSquadTab(state);
    expect(await screen.findByRole("row", { name: /Called Youth/ })).toBeInTheDocument();
    expect(screen.queryByRole("row", { name: /Excluded Youth/ })).not.toBeInTheDocument();
  });
  it("renders only the full roster table and not the moved tactics controls", () => {
    renderSquadTab(makeGameState());

    expect(screen.getByText("squad.title")).toBeInTheDocument();
    expect(screen.getByText("Bench DEF")).toBeInTheDocument();
    expect(screen.queryByText("What this changes")).not.toBeInTheDocument();
    expect(screen.queryByTestId("bench-player-d5")).not.toBeInTheDocument();
    expect(screen.queryByTestId("pitch-slot-1")).not.toBeInTheDocument();
    expect(screen.queryByText("squad.planStatus")).not.toBeInTheDocument();
    // Familiarity rides on the Position column's badges: no column of its own.
    expect(screen.getByRole("columnheader", { name: /squad\.pos/ })).toBeInTheDocument();
    expect(
      screen.queryByRole("columnheader", { name: "squad.formationFit" }),
    ).not.toBeInTheDocument();
    // The game does not tell the manager how well a player suits the play style: no column, no chip.
    expect(screen.queryByText(/styleFit/)).not.toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: "squad.traits" })).toBeInTheDocument();
    expect(screen.getByText(/squad.currentPlan/)).toBeInTheDocument();
    expect(screen.getByText("squad.coverageTitle")).toBeInTheDocument();
    expect(screen.getAllByText(/squad.needsCover/).length).toBeGreaterThan(0);
    expect(screen.queryByText(/squad.styleFitValues./)).not.toBeInTheDocument();
  });

  /**
   * Given a starting XI in which the goalkeeper and a forward have swapped slots
   * When the roster table renders
   * Then both are read as out of position on their Position badges, with no column of their own
   *      for it; and the unswapped XI states no such thing.
   */
  it("marks a player deployed out of position on his position badge", () => {
    renderSquadTab(makeGameState());
    expect(screen.queryAllByText(/, squad\.outOfPosition$/)).toHaveLength(0);

    cleanup();
    const swapped = makeGameState();
    swapped.teams[0].starting_xi_ids = [
      "f1",
      "d1",
      "d2",
      "d3",
      "d4",
      "m1",
      "m2",
      "m3",
      "m4",
      "gk1",
      "f2",
    ];
    renderSquadTab(swapped);

    expect(screen.getAllByText(/, squad\.outOfPosition$/)).toHaveLength(2);
    // The fixture's coarse buckets read as adapted in granular slots; only the swapped pair is warned.
    const warned = screen
      .getAllByTestId("familiarity-dot")
      .filter((dot) => dot.classList.contains("bg-warning-solid"));
    expect(warned).toHaveLength(2);
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
      match_day_eligible: false,
    });

    render(
      <SquadTab
        gameState={gameState}
        managerId="mgr1"
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    // In the keeper's own row; its details repeat the folded columns for a narrow window.
    const keeperRow = screen.getByRole("row", { name: /GK1/ });
    expect(within(keeperRow).getByText("Knee bruise")).toBeInTheDocument();
    expect(within(keeperRow).getAllByText("10d").length).toBeGreaterThan(0);
  });

  it("honors a persisted OVR sort state from the dashboard", () => {
    const gameState = makeGameState();
    gameState.players = [
      makePlayer("low", "Forward", {
        match_name: "Low OVR",
        attributes: {
          ...makePlayer("base-low", "Forward").attributes,
          pace: 30,
          shooting: 30,
          passing: 30,
          dribbling: 30,
        },
      }),
      makePlayer("high", "Forward", {
        match_name: "High OVR",
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
    riskLevels.current = { gk1: "critical" };

    renderSquadTab(gameState, { onSelectPlayer });

    const keeperRow = screen.getByRole("row", { name: /GK1/ });
    expect(within(keeperRow).getByText("Critical")).toBeInTheDocument();
    expect(within(keeperRow).getByText("Expires October 15, 2026")).toBeInTheDocument();

    const playerRow = screen.getByText("GK1").closest("tr");
    expect(playerRow).not.toBeNull();
    fireEvent.contextMenu(playerRow as HTMLTableRowElement);

    await waitFor(() => {
      expect(screen.getByRole("menuitem", { name: "Renew Contract" })).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("menuitem", { name: "Renew Contract" }));

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

    const playerRow = screen.getByText("GK1").closest("tr");
    expect(playerRow).not.toBeNull();
    fireEvent.contextMenu(playerRow as HTMLTableRowElement);

    fireEvent.click(screen.getByRole("menuitem", { name: "Let Expire" }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("set_contract_exit_intent", {
        playerId: "gk1",
        reason: "manager_squad_action",
      });
      expect(onGameUpdate).toHaveBeenCalledWith(gameState);
    });

    fireEvent.contextMenu(playerRow as HTMLTableRowElement);
    fireEvent.click(screen.getByRole("menuitem", { name: "Terminate Now" }));

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

    const playerRow = screen.getByText("GK1").closest("tr");
    expect(playerRow).not.toBeNull();
    fireEvent.contextMenu(playerRow as HTMLTableRowElement);
    fireEvent.click(screen.getByRole("menuitem", { name: "Add to Loan List" }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("toggle_loan_list", {
        playerId: "gk1",
      });
      expect(onGameUpdate).toHaveBeenCalledWith(updatedGameState);
      expect(screen.getByText("GK1").closest("tr")).toHaveTextContent("Loan");
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

    const playerRow = screen.getByText("GK1").closest("tr");
    expect(playerRow).not.toBeNull();
    fireEvent.contextMenu(playerRow as HTMLTableRowElement);
    fireEvent.click(screen.getByRole("menuitem", { name: "Reopen Talks" }));

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

    const playerRow = screen.getByText("GK1").closest("tr");
    expect(playerRow).not.toBeNull();
    fireEvent.contextMenu(playerRow as HTMLTableRowElement);
    fireEvent.click(screen.getByRole("menuitem", { name: "Delegate to youth academy" }));

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
        starting_xi_ids: ["gk1", "d5", "d2", "d3", "d4", "m1", "m2", "m3", "m4", "f1", "f2"],
      })),
    };
    const onGameUpdate = vi.fn();
    renderSquadTab(gameState, { onGameUpdate });
    mockedInvoke.mockResolvedValue(updatedGameState);

    const benchRow = screen.getByText("Bench DEF").closest("tr");
    expect(benchRow).not.toBeNull();
    fireEvent.contextMenu(benchRow as HTMLTableRowElement);
    fireEvent.click(screen.getByRole("menuitem", { name: "squad.makeStarter" }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("set_starting_xi", {
        playerIds: ["gk1", "d5", "d2", "d3", "d4", "m1", "m2", "m3", "m4", "f1", "f2"],
      });
      expect(onGameUpdate).toHaveBeenCalledWith(updatedGameState);
    });
  });
  it("names each roster filter after its visible label", async () => {
    renderSquadTab(makeGameState());

    // Each <label> has to be tied to its control, or a screen reader announces three unnamed
    // fields and these queries find nothing.
    expect(await screen.findByRole("textbox", { name: "common.search" })).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "squad.pos" })).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "common.status" })).toBeInTheDocument();
  });

  it("keeps the same column-header node when a filter changes", async () => {
    // A header that remounts throws away focus and any in-flight transition. A component declared
    // inside the render body does it — Biome's noNestedComponentDefinitions catches that case —
    // but so does anything else that changes the header's identity, such as a key derived from
    // filter state, which no lint rule sees.
    renderSquadTab(makeGameState());

    const headerBefore = await screen.findByRole("columnheader", {
      name: "squad.jerseyNumberFull",
    });
    expect(headerBefore).toHaveTextContent("squad.jerseyNumber");

    fireEvent.change(screen.getByRole("textbox", { name: "common.search" }), {
      target: { value: "z" },
    });

    await waitFor(() => {
      expect(screen.getByRole("columnheader", { name: "squad.jerseyNumberFull" })).toBe(
        headerBefore,
      );
    });
  });
});
