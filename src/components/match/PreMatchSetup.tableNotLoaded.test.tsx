import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import PreMatchSetup from "./PreMatchSetup";
import type { GameStateData } from "../../store/gameStore";
import type { MatchSnapshot } from "./types";

// The backend's position table has not arrived: the stand-in is empty, so `loaded` is false.
vi.mock("../../hooks/usePositionGroups", async () => {
  const { createPositionTable } = await import("../../lib/positionTable");
  const table = createPositionTable([]);
  return {
    usePositionGroups: () => () => null,
    usePositionTable: () => table,
    resetPositionGroupsCache: () => {},
  };
});
vi.mock("../../hooks/useRatingTiers", async () => {
  const { cannedRatingTiersModule } = await import("../../test-utils/ratingTiers");
  return cannedRatingTiersModule();
});
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key, i18n: { language: "en" } }),
}));
vi.mock("../../context/ThemeContext", () => ({
  useTheme: () => ({ isDark: false, toggleTheme: vi.fn() }),
}));

const engineDefender = {
  id: "h-rb",
  name: "Home RB",
  position: "Defender",
  condition: 100,
  ovr: 60,
  traits: [],
};

describe("PreMatchSetup before the position table loads", () => {
  function renderNotLoaded() {
    const snapshot = {
      home_team: {
        id: "home1",
        name: "Home FC",
        formation: "4-4-2",
        play_style: "Balanced",
        players: [engineDefender],
      },
      away_team: {
        id: "away1",
        name: "Away FC",
        formation: "4-3-3",
        play_style: "Counter",
        players: [],
      },
      home_bench: [],
      away_bench: [],
      home_set_pieces: {},
      away_set_pieces: {},
    };
    const game = {
      clock: { current_date: "2026-08-01" },
      players: [
        {
          id: "h-rb",
          full_name: "Home RB",
          match_name: "Home RB",
          position: "RightBack",
          natural_position: "RightBack",
        },
      ],
      teams: [
        {
          id: "home1",
          name: "Home FC",
          short_name: "HOM",
          colors: { primary: "#10b981", secondary: "#1a3a6b" },
        },
        {
          id: "away1",
          name: "Away FC",
          short_name: "AWY",
          colors: { primary: "#6366f1", secondary: "#1a3a6b" },
        },
      ],
    };
    render(
      <PreMatchSetup
        snapshot={snapshot as unknown as MatchSnapshot}
        gameState={game as unknown as GameStateData}
        userSide="Home"
        onStart={vi.fn()}
        onUpdateSnapshot={vi.fn()}
      />,
    );
  }

  /**
   * Given a right back in a lineup that is too short for slot alignment, and a table that has
   *       not arrived (every group unknown)
   * When the pre-match pitch draws his token
   * Then he is not drawn out of position: the red ring waits for a verdict the table can give.
   */
  it("does not draw a granular-position player out of position", () => {
    renderNotLoaded();
    const token = screen.getByRole("button", { name: "Home RB" });
    expect(token.querySelector(".ring-warning-solid")).toBeNull();
    expect(token.querySelector(".ring-1")).not.toBeNull();
  });

  /**
   * Given a table that has not arrived, so no line is known
   * When the lineup panel draws its Auto-select button
   * Then it is disabled: a pick now would ignore position and could leave no goalkeeper.
   */
  it("keeps Auto-select disabled until the table arrives", () => {
    renderNotLoaded();
    expect(screen.getByRole("button", { name: "match.autoSelectXI" })).toBeDisabled();
  });
});
