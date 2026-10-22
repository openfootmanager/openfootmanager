import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { ThemeProvider } from "../../context/ThemeContext";
import type { GameStateData } from "../../store/gameStore";
import MatchLive from "./MatchLive";
import type { EngineTeamData, MatchSnapshot, MinuteResult } from "./types";

const invokeMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key, i18n: { language: "en" } }),
  initReactI18next: { type: "3rdParty", init: () => {} },
}));

// The theme provider asks the system for its colour scheme; jsdom has no matchMedia.
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

const team = (id: string): EngineTeamData =>
  ({ id, name: id, formation: "4-4-2", play_style: "Balanced", players: [] }) as EngineTeamData;

const SNAPSHOT = {
  phase: "FirstHalf",
  current_minute: 44,
  home_score: 0,
  away_score: 0,
  possession: "Home",
  ball_zone: "Midfield",
  home_team: team("home"),
  away_team: team("away"),
  home_bench: [],
  away_bench: [],
  home_possession_pct: 50,
  away_possession_pct: 50,
  events: [],
  home_subs_made: 0,
  away_subs_made: 0,
  max_subs: 5,
  home_set_pieces: {
    free_kick_taker: null,
    corner_taker: null,
    penalty_taker: null,
    captain: null,
  },
  away_set_pieces: {
    free_kick_taker: null,
    corner_taker: null,
    penalty_taker: null,
    captain: null,
  },
  substitutions: [],
  allows_extra_time: false,
  home_yellows: {},
  away_yellows: {},
  sent_off: [],
} as unknown as MatchSnapshot;

const HALF_TIME: MinuteResult = {
  minute: 45,
  phase: "HalfTime",
  events: [],
  home_score: 0,
  away_score: 0,
  possession: "Home",
  ball_zone: "Midfield",
  is_finished: false,
};

const GAME = { teams: [], players: [] } as unknown as GameStateData;

function renderLive(overrides: Partial<React.ComponentProps<typeof MatchLive>> = {}) {
  const props = {
    snapshot: SNAPSHOT,
    gameState: GAME,
    userSide: "Home" as const,
    isSpectator: false,
    importantEvents: [],
    preferredSpeed: "fast" as const,
    onSnapshotUpdate: vi.fn(),
    onImportantEvent: vi.fn(),
    onHalfTime: vi.fn(),
    onFullTime: vi.fn(),
    ...overrides,
  };
  render(
    <ThemeProvider>
      <MatchLive {...props} />
    </ThemeProvider>,
  );
  return props;
}

const stepCalls = () => invokeMock.mock.calls.filter(([command]) => command === "step_live_match");

describe("MatchLive", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    invokeMock.mockReset();
    invokeMock.mockImplementation(async (command: string) =>
      command === "step_live_match" ? [HALF_TIME] : SNAPSHOT,
    );
  });
  afterEach(() => vi.useRealTimers());

  /**
   * Given a match at Fast that has just reached half time (the break shows 600ms later)
   * When Space is pressed inside that window
   * Then the match doesn't step on into the second half, and the break still shows: before the fix
   * Space resumed it and the next step started the second half before the team talk.
   */
  it("doesn't step past half time when Space is pressed before the break shows", async () => {
    const props = renderLive();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(250);
    });
    expect(stepCalls()).toHaveLength(1);

    fireEvent.keyDown(document.body, { key: " ", code: "Space" });
    fireEvent.click(screen.getByRole("button", { name: "match.step1Min" }));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(500);
    });
    expect(stepCalls()).toHaveLength(1);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(200);
    });
    expect(props.onHalfTime).toHaveBeenCalledWith("HalfTime");
    expect(stepCalls()).toHaveLength(1);
  });

  /**
   * Given the substitutions panel open over the match
   * When Space is pressed
   * Then the match isn't paused or resumed behind the dialog.
   */
  it("leaves Space alone while the substitutions panel is open", async () => {
    invokeMock.mockImplementation(async () => []);
    renderLive();
    fireEvent.click(screen.getByRole("button", { name: /match\.subs/ }));
    // The open dialog hides the rest of the screen from the accessibility tree, hence `hidden`;
    // a hidden query also counts the button's aria-hidden key hint, hence the prefix.
    const pause = { name: /^match\.pause/, hidden: true };
    expect(screen.getByRole("button", pause)).toBeInTheDocument();

    fireEvent.keyDown(document.body, { key: " ", code: "Space" });

    expect(screen.getByRole("button", pause)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /^match\.resume/, hidden: true })).toBeNull();
  });
});
