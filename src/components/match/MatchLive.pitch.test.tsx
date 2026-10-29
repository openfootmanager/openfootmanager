import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { ThemeContext } from "../../context/ThemeContext";
import type { GameStateData } from "../../store/gameStore";
import { createPlayer } from "../../test-utils/factories";
import { createSnapshot } from "../../test-utils/matchFactories";
import MatchLive from "./MatchLive";

vi.mock("@tauri-apps/api/core", () => ({
  // Never settles, so the auto-step timer cannot move the match on under the test.
  invoke: vi.fn(() => new Promise(() => {})),
  convertFileSrc: vi.fn((path: string) => path),
  isTauri: vi.fn(() => false),
}));

vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init: () => {} },
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock("../../store/settingsStore", () => ({
  useSettingsStore: () => ({ settings: { match_speed: "normal" } }),
}));

const gameState = {
  teams: [],
  players: [createPlayer({ id: "Home-9", jersey_number: 9 })],
} as unknown as GameStateData;

function renderMatchLive() {
  render(
    <ThemeContext.Provider value={{ theme: "dark", toggleTheme: vi.fn(), isDark: true }}>
      <MatchLive
        snapshot={createSnapshot({ player_ratings: { "Home-9": 7.4 } })}
        gameState={gameState}
        userSide="Home"
        isSpectator={false}
        importantEvents={[]}
        onSnapshotUpdate={vi.fn()}
        onImportantEvent={vi.fn()}
        onHalfTime={vi.fn()}
        onFullTime={vi.fn()}
      />
    </ThemeContext.Provider>,
  );
}

describe("MatchLive pitch tab", () => {
  /**
   * Given a live match on the events feed, When the manager opens the Pitch tab,
   * Then both sides' formations appear with the squad's shirt numbers and live marks.
   */
  it("shows both formations with shirt numbers and live ratings on the Pitch tab", () => {
    renderMatchLive();
    expect(screen.queryByRole("region", { name: "Alpha FC" })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "match.pitch" }));

    expect(screen.getByRole("region", { name: "Alpha FC" })).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Beta United" })).toBeInTheDocument();
    expect(screen.getByText("9")).toBeInTheDocument();
    expect(screen.getByText("7.4")).toBeInTheDocument();
  });
});
