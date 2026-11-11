import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { PlayerData } from "../../store/gameStore";
import type { TacticsLineup } from "../../store/generated/TacticsLineup";
import TacticsPlayerTable from "./TacticsPlayerTable";

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

describe("TacticsPlayerTable", () => {
  /**
   * Given the table
   * When it is drawn
   * Then it is a named table whose rows are grouped Starting eleven, Substitutes and Reserves.
   */
  it("groups the rows into the eleven, the substitutes and the reserves", () => {
    renderTable();

    const table = screen.getByRole("table", { name: "tactics.playerTable" });
    // Each section is a row group of its own, opened by a row-group header (found by role and name).
    const groupOf = (name: string) => {
      const header = within(table).getByRole("rowheader", { name });
      return screen.getAllByRole("rowgroup").find((group) => group.contains(header));
    };
    expect(
      within(groupOf("preMatch.startingXI") as HTMLElement).getByTestId("xi-row-gk"),
    ).toBeTruthy();
    expect(
      within(groupOf("preMatch.substitutes") as HTMLElement).getByTestId("bench-player-sub1"),
    ).toBeTruthy();
    expect(
      within(groupOf("tactics.reserves") as HTMLElement).getByTestId("bench-player-hurt"),
    ).toBeTruthy();
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
   * Given a striker deployed at left-back (his slot's position is LeftBack, his own is Striker)
   * When his role is changed
   * Then the roles offered are the left-back slot's, not a striker's: wing-back is offered and the
   *      poacher is not, and the table asks for the wing-back role for him.
   */
  it("offers a starter the roles of the slot he stands in, not of his own position", () => {
    const handlers = renderTable();

    fireEvent.click(screen.getByRole("combobox", { name: "tactics.playerRoleLabel: ST" }));
    expect(screen.queryByRole("option", { name: "tactics.playerRoles.Poacher" })).toBeNull();
    fireEvent.click(screen.getByRole("option", { name: "tactics.playerRoles.WingBack" }));

    expect(handlers.onRoleChange).toHaveBeenCalledWith("st", "WingBack");
  });

  /**
   * Given a fit substitute's row
   * When its actions button is pressed and "Promote to lineup" is chosen from the menu it opens
   * Then the table asks for that substitute to be promoted; the menu also offers his profile.
   */
  it("opens a row's menu from its actions button and runs the item chosen", () => {
    const handlers = renderTable();

    const button = within(screen.getByTestId("bench-player-sub1")).getByRole("button", {
      name: "common.playerActions",
    });
    expect(button).toHaveAttribute("aria-expanded", "false");
    fireEvent.click(button);

    expect(button).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByRole("menuitem", { name: "squad.viewProfile" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("menuitem", { name: "tactics.promoteToLineup" }));
    expect(handlers.onPromoteBench).toHaveBeenCalledWith("sub1");
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
