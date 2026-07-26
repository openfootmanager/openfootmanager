import { act, fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import MatchLive from "./MatchLive";
import type { GameStateData } from "../../store/gameStore";
import type { MatchSnapshot } from "./types";
import { ThemeProvider } from "../../context/ThemeContext";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("../../store/settingsStore", () => ({
  useSettingsStore: () => ({
    settings: { match_speed: "normal" },
  }),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

let desktopControlsMatches = false;
let mediaQueryChangeListeners: Array<(event: MediaQueryListEvent) => void> = [];

Object.defineProperty(window, "matchMedia", {
  writable: true,
  value: vi.fn().mockImplementation((query: string) => ({
    matches: query === "(min-width: 1024px)" ? desktopControlsMatches : false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(
      (_event: string, cb: (event: MediaQueryListEvent) => void) => {
        mediaQueryChangeListeners.push(cb);
      },
    ),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
});

function setDesktopControls(matches: boolean): void {
  desktopControlsMatches = matches;
  const event = { matches } as MediaQueryListEvent;
  for (const cb of mediaQueryChangeListeners) cb(event);
}

function makeSnapshot(): MatchSnapshot {
  return {
    phase: "Finished",
    current_minute: 90,
    home_score: 1,
    away_score: 0,
    possession: "Home",
    ball_zone: "Midfield",
    home_team: {
      id: "team1",
      name: "Alpha FC",
      formation: "4-4-2",
      play_style: "Balanced",
      players: [],
    },
    away_team: {
      id: "team2",
      name: "Beta FC",
      formation: "4-3-3",
      play_style: "Balanced",
      players: [],
    },
    home_bench: [],
    away_bench: [],
    home_possession_pct: 55,
    away_possession_pct: 45,
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
  };
}

const gameState = {
  teams: [],
  players: [],
} as unknown as GameStateData;

function renderMatchLive() {
  return render(
    <ThemeProvider>
      <MatchLive
        snapshot={makeSnapshot()}
        gameState={gameState}
        userSide="Home"
        isSpectator={false}
        importantEvents={[]}
        onSnapshotUpdate={vi.fn()}
        onImportantEvent={vi.fn()}
        onHalfTime={vi.fn()}
        onFullTime={vi.fn()}
      />
    </ThemeProvider>,
  );
}

beforeEach(() => {
  desktopControlsMatches = false;
  mediaQueryChangeListeners = [];
});

describe("MatchLive controls panel mounting", () => {
  it("mounts the controls panel exactly once as a static aside on desktop", () => {
    desktopControlsMatches = true;
    renderMatchLive();

    expect(screen.getAllByText("match.simSpeed")).toHaveLength(1);
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(screen.queryByTestId("match-controls-open")).toBeNull();
  });

  it("mounts no controls panel below lg until the drawer is opened", () => {
    renderMatchLive();

    expect(screen.queryByText("match.simSpeed")).toBeNull();
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("mounts the controls panel exactly once inside the drawer when opened", () => {
    renderMatchLive();

    fireEvent.click(screen.getByTestId("match-controls-open"));

    expect(screen.getAllByText("match.simSpeed")).toHaveLength(1);
    const dialog = screen.getByRole("dialog");
    expect(dialog.getAttribute("aria-modal")).toBe("true");
    // useModalOverlay moved focus into the freshly mounted drawer.
    expect(dialog.contains(document.activeElement)).toBe(true);
  });

  it("closes the drawer on Escape and restores focus to the trigger", () => {
    renderMatchLive();

    fireEvent.click(screen.getByTestId("match-controls-open"));
    expect(screen.getByRole("dialog")).toBeTruthy();

    fireEvent.keyDown(document, { key: "Escape" });
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(screen.getByTestId("match-controls-open")).toHaveFocus();
  });

  it("closes the drawer and releases the scroll lock when crossing into the desktop layout", () => {
    renderMatchLive();

    fireEvent.click(screen.getByTestId("match-controls-open"));
    expect(screen.getByRole("dialog")).toBeTruthy();
    expect(document.body.style.overflow).toBe("hidden");

    act(() => setDesktopControls(true));

    expect(screen.queryByRole("dialog")).toBeNull();
    expect(document.body.style.overflow).toBe("");
  });
});
