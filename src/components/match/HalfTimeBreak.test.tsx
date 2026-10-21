import { render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { ThemeProvider } from "../../context/ThemeContext";
import HalfTimeBreak from "./HalfTimeBreak";
import { makeGameState, makeSnapshot } from "./matchScreenFixtures";
import type { MatchEvent } from "./types";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

Object.defineProperty(window, "matchMedia", {
  writable: true,
  value: vi.fn().mockImplementation((query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
});

vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init: () => {} },
  useTranslation: () => ({ t: (key: string) => key }),
}));

const awayGoal: MatchEvent = {
  minute: 31,
  event_type: "Goal",
  side: "Away",
  zone: "Midfield",
  player_id: null,
  secondary_player_id: null,
};

describe("HalfTimeBreak", () => {
  /**
   * Given an away club called Beta FC whose short name is BTA, not its name's first three letters
   * When the half-time break lists a first-half goal of the away side
   * Then the event row is badged with the club's short name, as the score header shows it.
   */
  it("badges a first-half event with its club's short name", () => {
    const snapshot = makeSnapshot();
    const gameState = makeGameState();
    gameState.teams = gameState.teams.map((team) =>
      team.id === snapshot.away_team.id ? { ...team, short_name: "BTA" } : team,
    );

    render(
      <ThemeProvider>
        <HalfTimeBreak
          snapshot={snapshot}
          gameState={gameState}
          userSide="Home"
          isSpectator
          importantEvents={[awayGoal]}
          onResume={() => {}}
          onUpdateSnapshot={() => {}}
        />
      </ThemeProvider>,
    );

    // The score header also shows the short name (in the crest), so read the first-half card alone.
    const eventsCard = screen.getByText("match.firstHalfEvents").parentElement as HTMLElement;
    expect(within(eventsCard).getByText("BTA")).toBeInTheDocument();
  });
});
