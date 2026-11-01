import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
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

// Whether the backend's position table has arrived; a test says it has not.
const positionTableState = vi.hoisted(() => ({ arrived: true }));

vi.mock("../../hooks/usePositionGroups", async () => {
  const { cannedPositionsModule } = await import("../../test-utils/positionTable");
  const { createPositionTable } = await import("../../lib/positionTable");
  const canned = cannedPositionsModule();
  const notArrived = createPositionTable([]);
  return {
    ...canned,
    usePositionTable: () => {
      const arrived = canned.usePositionTable();
      return positionTableState.arrived ? arrived : notArrived;
    },
  };
});

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
  initReactI18next: { type: "3rdParty", init: () => {} },
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
   * Given a squad numbered 1 to 11, and a bench defender with no shirt number
   * When the roster is sorted by number, up and then down
   * Then the numbered players run 1 to 11 and then 11 to 1, and the bench defender is the last row
   *      both times, not the first going down.
   */
  it("keeps a player with no shirt number at the foot of the roster in both directions", () => {
    const state = makeGameState();
    state.players = state.players.map((player, index) => ({
      ...player,
      jersey_number: player.id === "d5" ? null : index + 1,
    }));
    const roster = (sortDir: "asc" | "desc") => (
      <SquadTab
        gameState={state}
        managerId="mgr1"
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
        sortState={{ sortKey: "jersey", sortDir }}
        onSortStateChange={vi.fn()}
      />
    );
    const rowNames = () =>
      within(screen.getByRole("table", { name: "squad.title" }))
        .getAllByRole("row")
        .slice(1)
        .map((row) => within(row).getAllByRole("button")[0].getAttribute("aria-label"));

    const { rerender } = render(roster("asc"));
    expect(rowNames()[0]).toBe("GK1");
    expect(rowNames()[rowNames().length - 1]).toBe("Bench DEF");

    rerender(roster("desc"));
    expect(rowNames()[0]).toBe("F2");
    expect(rowNames()[rowNames().length - 1]).toBe("Bench DEF");
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
      "selected",
    );
    expect(screen.getByRole("row", { name: /Hurt Forward/ })).toHaveAttribute(
      "data-accent",
      "danger",
    );
    expect(screen.getByRole("row", { name: /Bench DEF/ })).not.toHaveAttribute("data-accent");
  });

  /**
   * Given the squad not yet fetched, so the roster is the game state's own players, which carry no
   *       match-day verdict
   * When the roster renders
   * Then no row carries the can't-play accent: an absent verdict means not yet, not no.
   */
  it("marks no row as unable to play before the backend's verdict arrives", () => {
    const state = makeGameState();
    state.players = state.players.map((player) => ({ ...player, match_day_eligible: undefined }));
    // The squad fetch never answers: the roster stays the one drawn before it.
    mockedInvoke.mockReturnValueOnce(new Promise(() => {}));

    render(
      <SquadTab
        gameState={state}
        managerId="mgr1"
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    expect(screen.getByRole("row", { name: /Bench DEF/ })).toBeInTheDocument();
    const redRows = screen
      .getAllByRole("row")
      .filter((row) => row.getAttribute("data-accent") === "danger");
    expect(redRows).toHaveLength(0);
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
    expect(button).toHaveAttribute("aria-expanded", "false");
    fireEvent.click(button);
    expect(await screen.findByRole("menuitem", { name: "Renew Contract" })).toBeInTheDocument();
    // The button says its menu is open while it is, and closed again after Escape.
    expect(button).toHaveAttribute("aria-expanded", "true");
    fireEvent.keyDown(document.activeElement ?? document.body, { key: "Escape" });
    await waitFor(() =>
      expect(screen.queryByRole("menuitem", { name: "Renew Contract" })).toBeNull(),
    );
    expect(button).toHaveFocus();
    expect(button).toHaveAttribute("aria-expanded", "false");
  });

  /**
   * Given a row right-clicked with the mouse
   * When its menu opens
   * Then it opens where the pointer is.
   */
  it("opens a row's menu at the pointer on a mouse right-click", async () => {
    renderSquadTab(makeGameState());
    const row = (await screen.findByText("GK1")).closest("tr") as HTMLTableRowElement;
    fireEvent.contextMenu(row, { clientX: 50, clientY: 60 });
    const menu = await screen.findByRole("menu");
    expect(menu).toHaveStyle({ left: "50px", top: "60px" });
  });

  /**
   * Given a row's menu asked for from the keyboard (the Menu key or Shift+F10 send a right-click
   * at 0,0)
   * When it opens
   * Then it opens at the row, under its left edge, not in the window's top-left corner.
   */
  it("opens a row's menu at the row on a keyboard right-click", async () => {
    renderSquadTab(makeGameState());
    const row = (await screen.findByText("GK1")).closest("tr") as HTMLTableRowElement;
    vi.spyOn(row, "getBoundingClientRect").mockReturnValue(
      DOMRect.fromRect({ x: 120, y: 260, width: 600, height: 40 }),
    );
    fireEvent.contextMenu(row, { clientX: 0, clientY: 0 });
    const menu = await screen.findByRole("menu");
    expect(menu).toHaveStyle({ left: "120px", top: "300px" });
  });

  /**
   * Given a transfer-listed player on a stable contract and a transfer-listed one whose contract
   * is critical
   * When the roster renders
   * Then each contract shows one chip, as table C does: the listing for the first, the critical
   * stage for the second (the more urgent of the two), never two chips side by side.
   */
  it("shows one contract chip, the urgent stage before a listing", async () => {
    const state = makeGameState();
    state.players[0].transfer_listed = true;
    state.players[1].transfer_listed = true;
    riskLevels.current = { d1: "critical" };
    renderSquadTab(state);

    const listed = await screen.findByRole("row", { name: /GK1/ });
    expect(within(listed).getByText("Transfer")).toBeInTheDocument();
    expect(within(listed).queryByText("finances.contractRiskStable")).toBeNull();
    const urgent = screen.getByRole("row", { name: /D1/ });
    expect(within(urgent).getByText("Critical")).toBeInTheDocument();
    expect(within(urgent).queryByText("Transfer")).toBeNull();
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
    expect(within(keeperRow).getByRole("img", { name: /Knee bruise/ })).toBeInTheDocument();
    expect(within(keeperRow).getAllByText("10d").length).toBeGreaterThan(0);
  });

  /**
   * Given an OVR sort, high to low, saved on the dashboard, and two forwards the backend rates 30
   *       and 90, the weaker listed first
   * When Squad opens
   * Then the 90 is drawn above the 30.
   */
  it("honors a persisted OVR sort state from the dashboard", () => {
    const gameState = makeGameState();
    // The rating is the backend's `ovr`; the list never derives one from attributes. Without it
    // both read 0, and the order would rest on how a tie falls.
    gameState.players = [
      makePlayer("low", "Forward", { match_name: "Low OVR", ovr: 30 }),
      makePlayer("high", "Forward", { match_name: "High OVR", ovr: 90 }),
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

  /**
   * Given a strong striker and a weak goalkeeper, both in the starting XI
   * When the squad is sorted by position (the default)
   * Then the goalkeeper's row comes first: the order is the backend's table's rank, not the OVR
   *      the two would tie-break on.
   */
  it("sorts the roster by the table's position order, not by rating", () => {
    const gameState = makeGameState();
    const weak = (id: string, position: string) =>
      makePlayer(id, position, {
        match_name: id,
        attributes: {
          ...makePlayer(`base-${id}`, position).attributes,
          pace: 30,
          shooting: 30,
          passing: 30,
          dribbling: 30,
          handling: 30,
          reflexes: 30,
        },
      });
    gameState.players = [
      makePlayer("Striker Strong", "Striker", {
        match_name: "Striker Strong",
        attributes: {
          ...makePlayer("base-strong", "Striker").attributes,
          pace: 90,
          shooting: 90,
          passing: 90,
          dribbling: 90,
        },
      }),
      weak("Keeper Weak", "Goalkeeper"),
    ];

    render(
      <SquadTab
        gameState={gameState}
        managerId="mgr1"
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
        sortState={{ sortKey: "pos", sortDir: "asc" }}
        onSortStateChange={vi.fn()}
      />,
    );

    const rowTexts = screen.getAllByRole("row").map((row) => row.textContent ?? "");
    const keeperIndex = rowTexts.findIndex((text) => text.includes("Keeper Weak"));
    const strikerIndex = rowTexts.findIndex((text) => text.includes("Striker Strong"));

    expect(keeperIndex).toBeGreaterThan(-1);
    expect(strikerIndex).toBeGreaterThan(-1);
    expect(keeperIndex).toBeLessThan(strikerIndex);
  });

  /**
   * Given a strong striker and a weak goalkeeper of the same age
   * When the squad is sorted by age, which ties
   * Then the tie keeps the roster's own order: the goalkeeper's line first, not the striker's
   *      higher rating.
   */
  it("keeps a tie in the roster's own order: line first, then rating", () => {
    const gameState = makeGameState();
    const weak = (id: string, position: string) =>
      makePlayer(id, position, {
        match_name: id,
        attributes: {
          ...makePlayer(`base-${id}`, position).attributes,
          pace: 30,
          shooting: 30,
          passing: 30,
          dribbling: 30,
          handling: 30,
          reflexes: 30,
        },
      });
    gameState.players = [
      makePlayer("Striker Strong", "Striker", {
        match_name: "Striker Strong",
        attributes: {
          ...makePlayer("base-strong", "Striker").attributes,
          pace: 90,
          shooting: 90,
          passing: 90,
          dribbling: 90,
        },
      }),
      weak("Keeper Weak", "Goalkeeper"),
    ];

    render(
      <SquadTab
        gameState={gameState}
        managerId="mgr1"
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
        sortState={{ sortKey: "age", sortDir: "asc" }}
        onSortStateChange={vi.fn()}
      />,
    );

    const rowTexts = screen.getAllByRole("row").map((row) => row.textContent ?? "");
    const keeperIndex = rowTexts.findIndex((text) => text.includes("Keeper Weak"));
    const strikerIndex = rowTexts.findIndex((text) => text.includes("Striker Strong"));

    expect(keeperIndex).toBeGreaterThan(-1);
    expect(strikerIndex).toBeGreaterThan(-1);
    expect(keeperIndex).toBeLessThan(strikerIndex);
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
    // Table C's contract column: the stage chip and the month the contract ends, on one line.
    expect(within(keeperRow).getByText(/Oct 2026/)).toBeInTheDocument();
    expect(within(keeperRow).queryByText("Expires October 15, 2026")).toBeNull();

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
        careerToken: 0,
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
        careerToken: 0,
        playerId: "gk1",
      });
      expect(onGameUpdate).toHaveBeenCalledWith(updatedGameState);
      expect(screen.getByText("GK1").closest("tr")).toHaveTextContent("Loan");
    });
  });

  /**
   * Given the backend refusing to transfer-list a player
   * When the manager lists him from the roster menu
   * Then the refusal is shown under the roster, not swallowed with the menu closing on nothing.
   */
  it("shows why a listing from the roster menu failed", async () => {
    renderSquadTab(makeGameState());
    // A backend error is a translation key: the player reads its words, never the key.
    mockedInvoke.mockRejectedValue("be.error.careerChanged");

    fireEvent.contextMenu(screen.getByText("GK1").closest("tr") as HTMLTableRowElement);
    fireEvent.click(await screen.findByRole("menuitem", { name: /transfer/i }));

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("The career changed before this could be saved.");
    expect(alert).not.toHaveTextContent("be.error");
  });

  /**
   * Given a roster showing why an earlier listing failed
   * When the manager's next action from the menu succeeds
   * Then the old error goes, so the roster never shows a failure that no longer stands.
   */
  it("clears an earlier failure once a menu action succeeds", async () => {
    const gameState = makeGameState();
    renderSquadTab(gameState);
    mockedInvoke.mockRejectedValueOnce("Listing failed");
    fireEvent.contextMenu(screen.getByText("GK1").closest("tr") as HTMLTableRowElement);
    fireEvent.click(await screen.findByRole("menuitem", { name: /transfer/i }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Listing failed");

    mockedInvoke.mockResolvedValueOnce(gameState);
    fireEvent.contextMenu(screen.getByText("GK1").closest("tr") as HTMLTableRowElement);
    fireEvent.click(await screen.findByRole("menuitem", { name: "Add to Loan List" }));
    await waitFor(() => expect(screen.queryByRole("alert")).toBeNull());
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
        careerToken: 0,
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
        careerToken: 0,
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
        starting_xi_ids: ["gk1", "d1", "d5", "d3", "d4", "m1", "m2", "m3", "m4", "f1", "f2"],
      })),
    };
    const onGameUpdate = vi.fn();
    renderSquadTab(gameState, { onGameUpdate });
    mockedInvoke.mockResolvedValue(updatedGameState);

    const benchRow = screen.getByText("Bench DEF").closest("tr");
    expect(benchRow).not.toBeNull();
    fireEvent.contextMenu(benchRow as HTMLTableRowElement);
    // d5 is a coarse "Defender", graded as a centre back as the backend reads him: he is natural at
    // the first centre-back slot (replacing d2), not at the first slot that merely adapts him.
    expect(screen.getByRole("menuitem", { name: "squad.makeStarter" })).toBeEnabled();
    fireEvent.click(screen.getByRole("menuitem", { name: "squad.makeStarter" }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("set_starting_xi", {
        careerToken: 0,
        playerIds: ["gk1", "d1", "d5", "d3", "d4", "m1", "m2", "m3", "m4", "f1", "f2"],
      });
      expect(onGameUpdate).toHaveBeenCalledWith(updatedGameState);
    });
  });
  /**
   * Given the position table has not arrived (every slot would tie, and the keeper's is first)
   * When a bench player is made a starter from the roster context menu
   * Then nothing is saved: a lineup worked out without the lines must not be written.
   */
  it("saves no lineup change before the position table has loaded", async () => {
    positionTableState.arrived = false;
    try {
      renderSquadTab(makeGameState());

      const benchRow = screen.getByText("Bench DEF").closest("tr");
      fireEvent.contextMenu(benchRow as HTMLTableRowElement);
      const makeStarter = screen.getByRole("menuitem", { name: "squad.makeStarter" });
      // Disabled, so the item says it cannot be done yet instead of silently doing nothing.
      expect(makeStarter).toBeDisabled();
      fireEvent.click(makeStarter);
      await act(async () => {});

      expect(mockedInvoke).not.toHaveBeenCalledWith("set_starting_xi", expect.anything());
    } finally {
      positionTableState.arrived = true;
    }
  });

  /**
   * Given the position table has not arrived
   * When a starter's roster menu is opened
   * Then "Send to bench" is disabled: a lineup change is not made without the lines.
   */
  it("disables Send to bench in a starter's menu until the position table has loaded", () => {
    positionTableState.arrived = false;
    try {
      renderSquadTab(makeGameState());

      const starterRow = screen.getByText("GK1").closest("tr");
      fireEvent.contextMenu(starterRow as HTMLTableRowElement);

      expect(screen.getByRole("menuitem", { name: "squad.sendToBench" })).toBeDisabled();
    } finally {
      positionTableState.arrived = true;
    }
  });

  /**
   * Given the position table loaded or not
   * When the status filter's options are opened
   * Then "needs cover" is offered once the table has loaded and not before: without the lines
   *      it would drop players it should keep.
   */
  it("offers the needs-cover filter only once the position table has loaded", () => {
    positionTableState.arrived = false;
    try {
      const { unmount } = render(
        <SquadTab
          gameState={makeGameState()}
          managerId="mgr1"
          onSelectPlayer={vi.fn()}
          onGameUpdate={vi.fn()}
        />,
      );
      fireEvent.click(screen.getByRole("combobox", { name: "common.status" }));
      expect(screen.queryByRole("option", { name: "squad.needsCover" })).toBeNull();
      expect(screen.getByRole("option", { name: "squad.naturalFit" })).toBeInTheDocument();
      unmount();
    } finally {
      positionTableState.arrived = true;
    }

    renderSquadTab(makeGameState());
    fireEvent.click(screen.getByRole("combobox", { name: "common.status" }));
    expect(screen.getByRole("option", { name: "squad.needsCover" })).toBeInTheDocument();
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
