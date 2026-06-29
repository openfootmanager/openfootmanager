import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import PreMatchSetup from "./PreMatchSetup";

// Mock the few external dependencies PreMatchSetup pulls in at render time so we
// can exercise the real component tree (the opponent scout panel in particular).
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, opts?: unknown) => {
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
    pace: 50, stamina: 50, strength: 50, agility: 50, passing: 50, shooting: 50,
    tackling: 50, dribbling: 50, defending: 50, positioning: 50, vision: 50,
    decisions: 50, composure: 50, aggression: 50, teamwork: 50, leadership: 50,
    handling: 50, reflexes: 50, aerial: 50, ovr: 60, traits: [],
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
      { id: "home1", name: "Home FC", short_name: "HOM", colors: { primary: "#10b981", secondary: "#1a3a6b" } },
      { id: "away1", name: "Away FC", short_name: "AWY", colors: { primary: "#6366f1", secondary: "#1a3a6b" } },
    ],
  };
}

function renderSetup() {
  return render(
    <PreMatchSetup
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      snapshot={snapshot() as any}
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      gameState={gameState() as any}
      userSide="Home"
      onStart={vi.fn()}
      onUpdateSnapshot={vi.fn()}
    />,
  );
}

describe("PreMatchSetup opponent scout panel", () => {
  it("scouts the opponent squad on the Opponent tab", () => {
    renderSetup();

    // Default "Your Team" view: opponent players are not listed.
    expect(screen.queryByText("Away Fwd")).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: /Away FC/ }));

    // The opponent scout panel lists the opponent's players by name.
    expect(screen.getByText("Away GK")).toBeTruthy();
    expect(screen.getByText("Away Fwd")).toBeTruthy();
  });
});
