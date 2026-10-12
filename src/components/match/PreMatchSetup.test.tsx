import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import PreMatchSetup from "./PreMatchSetup";
import type { FixtureData, GameStateData, LeagueData } from "../../store/gameStore";
import type { MatchSnapshot } from "./types";

// Mock the few external dependencies PreMatchSetup pulls in at render time so we
// can exercise the real component tree (the opponent scout panel in particular).
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, opts?: unknown) => {
      if (key === "tournaments.competitions.nationalCup") return "Copa Nacional";
      if (typeof opts === "string") return opts;
      if (opts && typeof opts === "object" && "defaultValue" in opts) {
        return (opts as { defaultValue: string }).defaultValue;
      }
      return key;
    },
    i18n: { language: "en" },
  }),
}));
vi.mock("../../context/ThemeContext", () => ({
  useTheme: () => ({ isDark: false, toggleTheme: vi.fn() }),
}));

function enginePlayer(over: Record<string, unknown>): Record<string, unknown> {
  return {
    id: "p",
    name: "Player",
    position: "Midfielder",
    condition: 100,
    pace: 50,
    stamina: 50,
    strength: 50,
    agility: 50,
    passing: 50,
    shooting: 50,
    tackling: 50,
    dribbling: 50,
    defending: 50,
    positioning: 50,
    vision: 50,
    decisions: 50,
    composure: 50,
    aggression: 50,
    teamwork: 50,
    leadership: 50,
    handling: 50,
    reflexes: 50,
    aerial: 50,
    ovr: 60,
    traits: [],
    ...over,
  };
}

const homePlayers = [
  enginePlayer({ id: "h-gk", name: "Home GK", position: "Goalkeeper" }),
  enginePlayer({ id: "h-df", name: "Home Def", position: "Defender" }),
  enginePlayer({ id: "h-mf", name: "Home Mid", position: "Midfielder" }),
  enginePlayer({ id: "h-fw", name: "Home Fwd", position: "Forward" }),
];
const awayPlayers = [
  enginePlayer({ id: "a-gk", name: "Away GK", position: "Goalkeeper" }),
  enginePlayer({ id: "a-fw", name: "Away Fwd", position: "Forward" }),
];

const emptySetPieces = {
  free_kick_taker: null,
  corner_taker: null,
  penalty_taker: null,
  captain: null,
};

function snapshot(): Record<string, unknown> {
  return {
    home_team: {
      id: "home1",
      name: "Home FC",
      formation: "4-4-2",
      play_style: "Balanced",
      players: homePlayers,
    },
    away_team: {
      id: "away1",
      name: "Away FC",
      formation: "4-3-3", // distinct from home, so it only appears in the opponent panel
      play_style: "Counter",
      players: awayPlayers,
    },
    home_bench: [],
    away_bench: [],
    home_set_pieces: emptySetPieces,
    away_set_pieces: emptySetPieces,
  };
}

function gameState(): Record<string, unknown> {
  return {
    clock: { current_date: "2026-08-01" },
    players: [],
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
}

function renderSetup(currentFixture?: FixtureData, competitions?: LeagueData[]) {
  const state = gameState();
  if (competitions) state.competitions = competitions;
  return render(
    <PreMatchSetup
      // The two fixtures build partial objects deliberately — this test exercises the setup
      // screen, not the full match snapshot — so the cast is the honest spelling of "stands in
      // for". It goes through `unknown` because the shapes genuinely do not overlap, and the
      // two `eslint-disable` lines it used to carry were decorative: there is no ESLint here.
      snapshot={snapshot() as unknown as MatchSnapshot}
      gameState={state as unknown as GameStateData}
      currentFixture={currentFixture}
      userSide="Home"
      onStart={vi.fn()}
      onUpdateSnapshot={vi.fn()}
    />,
  );
}

describe("PreMatchSetup opponent scout panel", () => {
  /** Given a pre-match token, when its player name is hovered,
   * then its summary appears outside the clipped pitch. */
  it("shows a pre-match token summary when its name is hovered", () => {
    renderSetup();
    const control = screen.getByRole("button", { name: "Home GK" });
    fireEvent.mouseEnter(within(control).getByText("HOME GK"));
    const tooltip = screen.getByRole("tooltip");
    expect(tooltip).toHaveTextContent("squad.pitchTokenTooltip");
    expect(control).not.toContainElement(tooltip);
  });

  /** Given a pre-match token, when its player control receives keyboard focus,
   * then its summary is visible and described without replacing the player's name. */
  it("keeps the pre-match player name while describing its focused token", () => {
    renderSetup();
    const control = screen.getByRole("button", { name: "Home GK" });
    act(() => control.focus());
    expect(control).toHaveFocus();
    expect(control).toHaveAccessibleName("Home GK");
    expect(control).toHaveAccessibleDescription("squad.pitchTokenTooltip");
    expect(screen.getByRole("tooltip")).toHaveTextContent("squad.pitchTokenTooltip");
    expect(screen.getByRole("tooltip")).toHaveAttribute(
      "id",
      control.getAttribute("aria-describedby"),
    );
    act(() => control.blur());
    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
  });

  /** Given a focused player and its visible tooltip, when Escape is pressed,
   * then the tooltip closes while focus and the accessible description remain. */
  it("dismisses the focused player tooltip with Escape without moving focus", () => {
    renderSetup();
    const control = screen.getByRole("button", { name: "Home GK" });
    act(() => control.focus());
    expect(screen.getByRole("tooltip")).toBeInTheDocument();
    fireEvent.keyDown(control, { key: "Escape" });
    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
    expect(control).toHaveFocus();
    expect(control).toHaveAccessibleDescription("squad.pitchTokenTooltip");
  });

  it("scouts the opponent squad on the Opponent tab", () => {
    renderSetup();

    // Default "Your Team" view: opponent players are not listed.
    expect(screen.queryByText("Away Fwd")).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: /Away FC/ }));

    // The opponent scout panel lists the opponent's players by name.
    expect(screen.getByText("Away GK")).toBeTruthy();
    expect(screen.getByText("Away Fwd")).toBeTruthy();
  });

  it("shows the translated competition name for the current named cup fixture", () => {
    const fixture: FixtureData = {
      id: "cup-match",
      competition_id: "cup-2026",
      competition: "Cup",
      matchday: 1,
      date: "2026-08-01",
      home_team_id: "home1",
      away_team_id: "away1",
      status: "Scheduled",
      result: null,
    };
    renderSetup(fixture, [
      {
        id: "cup-2026",
        name: "National Cup",
        name_key: "tournaments.competitions.nationalCup",
        season: 2026,
        fixtures: [fixture],
        standings: [],
      },
    ]);

    expect(screen.getByText("Copa Nacional")).toBeInTheDocument();
  });
});
