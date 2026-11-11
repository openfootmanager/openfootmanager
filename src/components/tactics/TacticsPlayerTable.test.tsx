import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { PlayerData } from "../../store/gameStore";
import type { TacticsLineup } from "../../store/generated/TacticsLineup";
import TacticsPlayerTable, { buildTableRows } from "./TacticsPlayerTable";

vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init: () => undefined },
  useTranslation: () => ({
    t: (key: string, fallback?: string | Record<string, unknown>) => {
      if (typeof fallback === "string") return fallback;
      if (typeof fallback?.defaultValue === "string") return fallback.defaultValue;
      return key;
    },
    i18n: { language: "en" },
  }),
}));

vi.mock("../../hooks/useRatingTiers", async () => {
  const { cannedRatingTiersModule } = await import("../../test-utils/ratingTiers");
  return cannedRatingTiersModule();
});

vi.mock("../../hooks/usePositionGroups", async () => {
  const { cannedPositionsModule } = await import("../../test-utils/positionTable");
  return cannedPositionsModule();
});

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const player = (id: string, position: string, overrides: Partial<PlayerData> = {}): PlayerData =>
  ({
    id,
    match_name: id.toUpperCase(),
    full_name: `Player ${id}`,
    position,
    natural_position: position,
    alternate_positions: [],
    condition: 90,
    morale: 70,
    injury: null,
    match_day_eligible: true,
    team_id: "club",
    attributes: {},
    jersey_number: null,
    ...overrides,
  }) as unknown as PlayerData;

const gk = player("gk", "Goalkeeper", { jersey_number: 1 });
const st = player("st", "Striker");
const sub1 = player("sub1", "CenterBack");
const sub2 = player("sub2", "Striker");
const hurt = player("hurt", "Striker", {
  injury: { name: "Knock", days_remaining: 4 },
  match_day_eligible: false,
});

const lineup: TacticsLineup = {
  formation: "4-4-2",
  eleven: [
    { position: "Goalkeeper", player_id: "gk", role: "SweeperKeeper", familiarity: "natural" },
    { position: "LeftBack", player_id: "st", role: "Standard", familiarity: "out_of_position" },
  ],
  // The match ranks sub2 above sub1.
  substitutes: ["sub2", "sub1"],
  reserves: ["hurt"],
};

function renderTable(overrides: Partial<Parameters<typeof TacticsPlayerTable>[0]> = {}) {
  const handlers = {
    onTacticalSelect: vi.fn(),
    onRoleChange: vi.fn(),
    onOpenPlayerProfile: vi.fn(),
    onPromoteBench: vi.fn(),
    onDemoteStarter: vi.fn(),
    onClearSelection: vi.fn(),
    onDragStart: vi.fn(),
    onDragEnd: vi.fn(),
  };
  render(
    <TacticsPlayerTable
      starters={[gk, st]}
      others={[sub1, sub2, hurt]}
      slotPositions={["Goalkeeper", "LeftBack"]}
      lineup={lineup}
      selectedPlayerId={null}
      comparePlayerId={null}
      {...handlers}
      {...overrides}
    />,
  );
  return handlers;
}

describe("buildTableRows", () => {
  /**
   * Given the eleven, two substitutes and an injured player, and the backend's lineup
   * When the table's rows are built
   * Then the eleven come first by slot, then the substitutes in the order the match ranks them,
   *      then the reserves; each starter carries his slot's position, role and fit, the backend's.
   */
  it("orders the eleven by slot, the substitutes as the match ranks them, then the reserves", () => {
    const rows = buildTableRows(
      [gk, st],
      [sub1, sub2, hurt],
      ["Goalkeeper", "LeftBack"],
      lineup,
      {},
    );

    expect(rows.map((row) => [row.player.id, row.group])).toEqual([
      ["gk", "xi"],
      ["st", "xi"],
      ["sub2", "substitutes"],
      ["sub1", "substitutes"],
      ["hurt", "reserves"],
    ]);
    expect(rows[0]).toMatchObject({
      position: "Goalkeeper",
      role: "SweeperKeeper",
      familiarity: "natural",
    });
    expect(rows[1]).toMatchObject({ position: "LeftBack", familiarity: "out_of_position" });
  });

  /**
   * Given a starter the backend's lineup doesn't name in his slot (a change just made, not yet
   *       answered)
   * When the rows are built
   * Then his slot takes the formation's position and his own stored role, and no fit is claimed.
   */
  it("claims no fit for a slot the backend hasn't described yet", () => {
    const rows = buildTableRows([sub1], [], ["Goalkeeper"], lineup, { sub1: "Stopper" });

    expect(rows[0]).toMatchObject({ position: "Goalkeeper", role: "Stopper", familiarity: null });
  });
});

describe("TacticsPlayerTable", () => {
  /**
   * Given the table
   * When it is drawn
   * Then it is a named table whose rows are grouped Starting eleven, Substitutes and Reserves.
   */
  it("groups the rows into the eleven, the substitutes and the reserves", () => {
    renderTable();

    const table = screen.getByRole("table", { name: "tactics.playerTable" });
    const groups = within(table)
      .getAllByRole("columnheader")
      .filter((th) => th.getAttribute("colspan"))
      .map((th) => th.textContent);
    expect(groups).toEqual(["preMatch.startingXI", "preMatch.substitutes", "tactics.reserves"]);
  });

  /**
   * Given a striker the backend says is out of position at left-back
   * When the table is drawn
   * Then his row says so in words, not only in colour; the natural keeper's row doesn't.
   */
  it("says in words when a starter is out of position", () => {
    renderTable();

    expect(
      within(screen.getByTestId("xi-row-st")).getByText("squad.outOfPosition"),
    ).toBeInTheDocument();
    expect(within(screen.getByTestId("xi-row-gk")).queryByText("squad.outOfPosition")).toBeNull();
  });

  /**
   * Given the keeper's row
   * When its role is changed to a ball-playing keeper
   * Then the table asks for that role for him; the roles offered are the keeper slot's.
   */
  it("changes a starter's role from his slot's roles", () => {
    const handlers = renderTable();

    fireEvent.click(screen.getByRole("combobox", { name: "tactics.playerRoleLabel: GK" }));
    expect(screen.queryByRole("option", { name: "tactics.playerRoles.Poacher" })).toBeNull();
    fireEvent.click(screen.getByRole("option", { name: "tactics.playerRoles.BallPlayingKeeper" }));

    expect(handlers.onRoleChange).toHaveBeenCalledWith("gk", "BallPlayingKeeper");
  });

  /**
   * Given a starter and a substitute
   * When each one's name is pressed
   * Then each is picked for a swap from his own section.
   */
  it("picks a starter or a substitute for a swap from his name", () => {
    const handlers = renderTable();

    fireEvent.click(screen.getByTestId("xi-player-gk"));
    fireEvent.click(screen.getByTestId("pitch-bench-player-sub1"));

    expect(handlers.onTacticalSelect).toHaveBeenNthCalledWith(1, "gk", "xi");
    expect(handlers.onTacticalSelect).toHaveBeenNthCalledWith(2, "sub1", "bench");
  });

  /**
   * Given a fit substitute and an injured reserve
   * When the table is drawn
   * Then the substitute can be dragged onto the pitch and the injured reserve can't, and the
   *      reserve's injury is shown by his name.
   */
  it("lets only an available player be dragged to the pitch", () => {
    renderTable();

    expect(screen.getByTestId("pitch-bench-player-sub1")).toHaveAttribute("draggable", "true");
    expect(screen.getByTestId("pitch-bench-player-hurt")).toHaveAttribute("draggable", "false");
    expect(within(screen.getByTestId("bench-player-hurt")).getByText("Knock")).toBeInTheDocument();
  });

  /**
   * Given the table, picked player and all
   * When it is drawn
   * Then the table scrolls inside itself and the picked player's row is marked.
   */
  it("scrolls inside itself and marks the picked player's row", () => {
    renderTable({ selectedPlayerId: "st" });

    expect(screen.getByRole("table").parentElement).toHaveClass("overflow-auto", "h-full");
    expect(screen.getByTestId("xi-row-st")).toHaveAttribute("data-accent", "selected");
    expect(screen.getByTestId("xi-player-st")).toHaveAttribute("aria-pressed", "true");
  });
});
