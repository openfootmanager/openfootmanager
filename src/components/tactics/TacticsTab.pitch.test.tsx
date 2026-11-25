import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import TacticsTab from "./TacticsTab";
import { resetTacticsLineupCache } from "../../hooks/useTacticsLineupProjection";
import {
  familiarityState,
  focusableIn,
  makeGameState,
  swapBenchDefenderIn,
} from "../../test-utils/tacticsTabFixtures";

const positionTableState = vi.hoisted(() => ({ arrived: true }));

vi.mock("../../hooks/useFamiliarity", async () => {
  const { cannedFamiliarityModule } = await import("../../test-utils/familiarity");
  const { familiarityState } = await import("../../test-utils/tacticsTabFixtures");
  return cannedFamiliarityModule(
    (playerId) => familiarityState.players.get(playerId),
    () => familiarityState.arrived,
  );
});

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

vi.mock("react-i18next", () => ({
  // The error banner pulls in utils/backendI18n -> i18n/index, which registers this plugin.
  initReactI18next: { type: "3rdParty", init: () => undefined },
  useTranslation: () => ({
    // Interpolating: a key with values reads `key(name=value, ...)`, so a test can read what a
    // sentence was given, not only which sentence it was.
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
      if (typeof fallback === "string") return fallback;
      const values = Object.entries(fallback ?? {}).filter(([name]) => name !== "defaultValue");
      return values.length > 0
        ? `${key}(${values.map(([name, value]) => `${name}=${String(value)}`).join(", ")})`
        : key;
    },
    i18n: { language: "en" },
  }),
}));

vi.mock("../../hooks/useRatingTiers", async () => {
  const { cannedRatingTiersModule } = await import("../../test-utils/ratingTiers");
  return cannedRatingTiersModule();
});

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

/** The Tactics screen's pitch: what it shows, what it says in words, and that it has no controls. */
describe("TacticsTab pitch", () => {
  beforeEach(() => {
    localStorage.clear();
    resetTacticsLineupCache();
    mockedInvoke.mockReset();
    const defaultGameState = makeGameState();
    const defaultRoster = defaultGameState.players.filter((p) => p.team_id === "team1");
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "get_squad") return defaultRoster;
      return defaultGameState;
    });
  });

  /** Given a tactics pitch token, when its player name is hovered,
   * then the full summary appears outside the clipped pitch. */
  it("shows a tactics token summary when its name is hovered", async () => {
    render(
      <TacticsTab gameState={makeGameState()} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
    );
    await act(async () => {}); // the squad has answered, so the summary states the fit
    const control = screen.getByTestId("pitch-player-gk1");
    fireEvent.mouseEnter(within(control).getByText("GK1"));
    const tooltip = screen.getByRole("tooltip");
    expect(tooltip).toHaveTextContent("squad.pitchTokenTooltip");
    expect(control).not.toContainElement(tooltip);
  });

  /**
   * Given the tactics screen is drawn
   * When the keeper's token on the pitch is looked at
   * Then neither it nor anything in the drawing is a control a keyboard or a screen reader could
   *      reach (the drawing is hidden from the accessibility tree, so look past that), and it is
   *      not draggable
   */
  it("shows the pitch without controls", async () => {
    render(
      <TacticsTab gameState={makeGameState()} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
    );
    await act(async () => {});

    const token = screen.getByTestId("pitch-player-gk1");
    const drawing = token.closest<HTMLElement>('[aria-hidden="true"]');
    expect(drawing).not.toBeNull();
    for (const role of ["button", "combobox", "link", "textbox"]) {
      expect(within(drawing as HTMLElement).queryByRole(role, { hidden: true })).toBeNull();
    }
    expect(focusableIn(drawing as HTMLElement)).toEqual([]);
    expect(token).not.toHaveAttribute("tabindex");
    expect(token).not.toHaveAttribute("draggable");
  });

  /**
   * Given the first team in 4-4-2 and the squad's answer in
   * When the pitch's list for a screen reader is read
   * Then each slot says its position, its player and how he fits it, with the values it was given:
   *      the keeper, 100% condition, at his natural fit
   */
  it("describes each slot in words, with its position, player and fit", async () => {
    render(
      <TacticsTab gameState={makeGameState()} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
    );
    await act(async () => {}); // the squad has answered, so the words state the fit

    const eleven = screen.getByRole("list", { name: /preMatch\.startingXI - 4-4-2/ });
    const items = within(eleven).getAllByRole("listitem");
    expect(items).toHaveLength(11);
    expect(items[0].textContent).toMatch(
      /^tactics\.slotSummary\(position=common\.positions\.\w+, /,
    );
    expect(items[0]).toHaveTextContent("player=GK1");
    expect(items[0]).toHaveTextContent(
      "squad.pitchTokenTooltip(condition=100, fit=squad.naturalFit)",
    );
  });

  /**
   * Given a first team with only ten of its eleven named
   * When the pitch's list is read
   * Then the slot with nobody in it says so, with its position, and the others still name their player
   */
  it("says an empty slot is empty, with its position", async () => {
    const gameState = makeGameState();
    gameState.teams[0].starting_xi_ids = gameState.teams[0].starting_xi_ids.slice(0, 10);
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "get_squad") return gameState.players.filter((p) => p.team_id === "team1");
      return gameState;
    });

    render(<TacticsTab gameState={gameState} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />);
    await act(async () => {});

    const items = within(screen.getByRole("list", { name: /preMatch\.startingXI/ })).getAllByRole(
      "listitem",
    );
    const empty = items.filter((item) =>
      /^tactics\.emptySlot\(position=/.test(item.textContent ?? ""),
    );
    expect(empty).toHaveLength(1);
    expect(items.filter((item) => /player=/.test(item.textContent ?? ""))).toHaveLength(10);
  });

  /**
   * Given the goalkeeper is the captain and the penalty taker
   * When the pitch is drawn
   * Then the drawing's C and PK markers are in words in his slot's entry and not in anyone else's:
   *      the drawing is hidden from a screen reader, so the list is where he learns who holds them.
   */
  it("says the match roles a starter holds in the pitch's list", async () => {
    const gameState = makeGameState();
    gameState.teams[0].match_roles = {
      captain: "gk1",
      vice_captain: null,
      penalty_taker: "gk1",
      free_kick_taker: null,
      corner_taker: null,
    };
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "get_squad") return gameState.players.filter((p) => p.team_id === "team1");
      return gameState;
    });

    render(<TacticsTab gameState={gameState} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />);
    await act(async () => {});

    const items = within(screen.getByRole("list", { name: /preMatch\.startingXI/ })).getAllByRole(
      "listitem",
    );
    expect(items[0]).toHaveTextContent("preMatch.captain");
    expect(items[0]).toHaveTextContent("preMatch.penaltyTaker");
    expect(items[1]).not.toHaveTextContent("preMatch.captain");
  });

  /**
   * Given the keeper and a centre back swapped, but neither held by the backend's projection
   * When the pitch is drawn
   * Then nobody is ringed and the count reads zero: a player with no grade is not out of position.
   */
  it("counts nobody out of position for players the projection does not hold", async () => {
    const state = makeGameState();
    state.teams[0].starting_xi_ids = [
      "d1",
      "gk1",
      "d2",
      "d3",
      "d4",
      "m1",
      "m2",
      "m3",
      "m4",
      "f1",
      "f2",
    ];
    familiarityState.players.delete("gk1");
    familiarityState.players.delete("d1");

    const { container } = render(
      <TacticsTab gameState={state} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
    );
    await act(async () => {}); // the squad has answered for this revision

    expect(screen.getByText("0 squad.outOfPosition")).toBeInTheDocument();
    expect(container.querySelectorAll(".ring-warning-solid")).toHaveLength(0);
    // And no mid-grade mark either: a player the projection does not hold is not "adapted".
    for (const id of ["d1", "gk1"]) {
      const token = screen.getByTestId(`pitch-player-${id}`);
      expect(within(token).queryByTestId("familiarity-dot")).toBeNull();
    }
  });

  /**
   * Given the unswapped XI, where the keeper stands in his own slot
   * When the pitch is drawn with the backend's familiarity
   * Then his token wears no familiarity mark: a natural slot is the quiet default.
   */
  it("marks a natural token with nothing", async () => {
    render(
      <TacticsTab gameState={makeGameState()} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
    );
    await act(async () => {}); // the squad has answered, so a mark would show

    const keeper = screen.getByTestId("pitch-player-gk1");
    expect(within(keeper).queryByTestId("familiarity-dot")).toBeNull();
  });

  /**
   * Given the keeper and a centre back swapped, with the familiarity arrived and not arrived
   * When the pitch is drawn
   * Then arrived, two tokens are ringed out of position and the badge counts them; not arrived,
   *      no token is ringed and no count is claimed: a missing grade is not a verdict.
   */
  it("rings no token and counts nothing before the familiarity has loaded", async () => {
    const swapped = () => {
      const state = makeGameState();
      state.teams[0].starting_xi_ids = [
        "d1",
        "gk1",
        "d2",
        "d3",
        "d4",
        "m1",
        "m2",
        "m3",
        "m4",
        "f1",
        "f2",
      ];
      return state;
    };
    const { container, unmount } = render(
      <TacticsTab gameState={swapped()} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
    );
    await act(async () => {}); // the squad has answered for this revision
    expect(container.querySelectorAll(".ring-warning-solid")).toHaveLength(2);
    unmount();

    familiarityState.arrived = false;
    try {
      const { container: waiting } = render(
        <TacticsTab gameState={swapped()} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
      );

      expect(waiting.querySelectorAll(".ring-warning-solid")).toHaveLength(0);
      expect(screen.queryByText(/^\d+ squad\.outOfPosition$/)).toBeNull();
      // Nor does the swapped pair wear an "adapted" mark: no grade is no verdict, not a mid one.
      for (const id of ["d1", "gk1"]) {
        const token = screen.getByTestId(`pitch-player-${id}`);
        expect(within(token).queryByTestId("familiarity-dot")).toBeNull();
      }
    } finally {
      familiarityState.arrived = true;
    }
  });

  /**
   * Given a squad with the keeper and a centre back swapped (each deployed in the other's line)
   * When the pitch is drawn with the backend's familiarity
   * Then the out-of-position count reads two, the grade the backend gives them.
   */
  it("marks a starter out of position from the backend's familiarity", async () => {
    const state = makeGameState();
    state.teams[0].starting_xi_ids = [
      "d1",
      "gk1",
      "d2",
      "d3",
      "d4",
      "m1",
      "m2",
      "m3",
      "m4",
      "f1",
      "f2",
    ];

    render(<TacticsTab gameState={state} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />);
    await act(async () => {}); // the squad has answered for this revision

    expect(screen.getByText("2 squad.outOfPosition")).toBeInTheDocument();
  });

  /**
   * Given the keeper and a centre back swapped and the familiarity arrived, but get_squad not
   *       answered for this revision
   * When the pitch is drawn
   * Then no token is ringed: the pitch states no fit while its count and its writers wait.
   */
  it("rings no token while the squad has not answered, though the familiarity has", async () => {
    const state = makeGameState();
    state.teams[0].starting_xi_ids = [
      "d1",
      "gk1",
      "d2",
      "d3",
      "d4",
      "m1",
      "m2",
      "m3",
      "m4",
      "f1",
      "f2",
    ];
    mockedInvoke.mockReset();
    mockedInvoke.mockReturnValue(new Promise(() => {}));

    const { container } = render(
      <TacticsTab gameState={state} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
    );
    await act(async () => {});

    expect(container.querySelectorAll(".ring-warning-solid")).toHaveLength(0);
    expect(screen.queryByText(/^\d+ squad\.outOfPosition$/)).toBeNull();
  });

  it("offers no dragging anywhere: the table picks and the pitch only shows", () => {
    render(
      <TacticsTab gameState={makeGameState()} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
    );

    expect(screen.queryByTestId("bench-player-drag-handle-d5")).not.toBeInTheDocument();
    expect(screen.queryByTestId("xi-player-drag-handle-d1")).not.toBeInTheDocument();
    expect(screen.getByTestId("pitch-bench-player-d5")).not.toHaveAttribute("draggable");
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
      if (command === "get_squad") return gameState.players.filter((p) => p.team_id === "team1");
      return gameState;
    });

    render(<TacticsTab gameState={gameState} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />);

    // His positions are the row's "can play" badges, natural first.
    await screen.findByTestId("pitch-bench-player-d5");
    const benchRow = screen.getByTestId("bench-player-d5");

    expect(within(benchRow).getByText("common.posAbbr.Defender")).toBeInTheDocument();
    expect(within(benchRow).queryByText("common.posAbbr.Midfielder")).not.toBeInTheDocument();
  });

  /**
   * Given a club whose saved eleven names the forward "f2", and a backend whose lineup fields
   *       the bench defender "d5" in that slot instead (as the match would, for its own reasons)
   * When the Tactics screen shows the eleven
   * Then it shows the backend's eleven, "d5" and not "f2": the screen picks no side of its own.
   */
  it("shows the eleven the backend says the match will field", async () => {
    const gameState = makeGameState();
    const roster = gameState.players.filter((p) => p.team_id === "team1");
    const eleven = ["gk1", "d1", "d2", "d3", "d4", "m1", "m2", "m3", "m4", "f1", "d5"];
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "get_squad") return roster;
      if (command === "get_tactics_lineup") {
        return {
          formation: "4-4-2",
          eleven: eleven.map((player_id) => ({
            position: "Striker",
            player_id,
            role: "Standard",
            familiarity: "natural",
          })),
          substitutes: ["f2"],
          reserves: [],
        };
      }
      return gameState;
    });

    render(<TacticsTab gameState={gameState} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />);

    expect(await screen.findByTestId("xi-player-d5")).toBeInTheDocument();
    expect(screen.queryByTestId("xi-player-f2")).not.toBeInTheDocument();
    // The pitch draws the same eleven as the table.
    expect(screen.getByTestId("pitch-player-d5")).toBeInTheDocument();
    expect(screen.queryByTestId("pitch-player-f2")).not.toBeInTheDocument();
  });

  /**
   * Given the backend's lineup has answered once, and its next answer (after a save) never comes
   * When the bench defender is swapped in for the centre back through the table
   * Then the pitch shows him at once and no longer shows the centre back: it draws the eleven the
   *      table shows, change included, and does not blank while the new answer is awaited.
   */
  it("shows a swap on the pitch before the backend's next answer arrives", async () => {
    const gameState = makeGameState();
    const roster = gameState.players.filter((p) => p.team_id === "team1");
    const saved = gameState.teams[0].starting_xi_ids ?? [];
    const lineup = {
      formation: "4-4-2",
      eleven: saved.map((player_id) => ({
        position: "Striker",
        player_id,
        role: "Standard",
        familiarity: "natural",
      })),
      substitutes: ["d5"],
      reserves: [],
    };
    let lineupAsks = 0;
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "get_squad") return roster;
      if (command === "get_tactics_lineup") {
        lineupAsks += 1;
        return lineupAsks === 1 ? lineup : new Promise(() => {});
      }
      return gameState;
    });

    const { rerender } = render(
      <TacticsTab gameState={gameState} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
    );
    await screen.findByTestId("xi-player-d1");
    expect(screen.getByTestId("pitch-player-d1")).toBeInTheDocument();

    swapBenchDefenderIn();
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("set_starting_xi", expect.anything());
    });
    // The save's result comes back: a new revision, whose lineup the backend never answers.
    const swapped = saved.map((id) => (id === "d1" ? "d5" : id));
    rerender(
      <TacticsTab
        gameState={{
          ...gameState,
          teams: [{ ...gameState.teams[0], starting_xi_ids: swapped }],
        }}
        onSelectPlayer={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );
    await act(async () => {});

    expect(lineupAsks).toBe(2);
    expect(screen.getByTestId("pitch-player-d5")).toBeInTheDocument();
    expect(screen.queryByTestId("pitch-player-d1")).not.toBeInTheDocument();
  });
});
