import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { Lineups } from "./MatchPanels";
import type { MatchSnapshot } from "./types";

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
vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key, i18n: { language: "en" } }),
}));

const player = (id: string, name: string, position: string) => ({
  id,
  name,
  position,
  condition: 90,
});

// A forward listed before the keeper: the XI's own order is not the lines' order.
const snapshot = {
  home_team: {
    id: "h",
    name: "Home FC",
    formation: "4-4-2",
    players: [player("f", "Fwd Starter", "Forward"), player("g", "Keeper Starter", "Goalkeeper")],
  },
  away_team: { id: "a", name: "Away FC", formation: "4-4-2", players: [] },
  home_bench: [],
  away_bench: [],
  substitutions: [],
  home_yellows: {},
  away_yellows: {},
  sent_off: [],
} as unknown as MatchSnapshot;

describe("Lineups", () => {
  /**
   * Given the position table has not arrived, so no line is known
   * When the lineups are drawn
   * Then the starters still show, in the XI's own order and with no group headings.
   */
  it("shows the starters, ungrouped, before the position table has loaded", () => {
    positionTableState.arrived = false;
    try {
      render(<Lineups snapshot={snapshot} />);

      const names = screen.getAllByText(/Starter/).map((element) => element.textContent);
      expect(names).toEqual(["Fwd Starter", "Keeper Starter"]);
      expect(screen.queryByText("common.positionGroups.Goalkeeper")).toBeNull();
    } finally {
      positionTableState.arrived = true;
    }
  });

  /**
   * Given the table has arrived
   * When the lineups are drawn
   * Then the starters are grouped by the table's lines, keeper first, each group headed by its
   *      translated name (never an English plural built from the position code).
   */
  it("groups the starters by the table's lines once it has loaded", () => {
    render(<Lineups snapshot={snapshot} />);

    const names = screen.getAllByText(/Starter/).map((element) => element.textContent);
    expect(names).toEqual(["Keeper Starter", "Fwd Starter"]);
    expect(screen.getByText("common.positionGroups.Goalkeeper")).toBeInTheDocument();
    expect(screen.queryByText("Goalkeepers")).toBeNull();
  });
});
