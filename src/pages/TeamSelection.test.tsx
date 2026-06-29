import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { GameStateData, LeagueData } from "../store/gameStore";
import { useGameStore } from "../store/gameStore";
import { createPlayer, createTeam } from "../test-utils/factories";
import { ThemeProvider } from "../context/ThemeContext";
import TeamSelection from "./TeamSelection";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

vi.mock("react-router-dom", () => ({
  useNavigate: () => vi.fn(),
}));

vi.mock("../services/portraitService", () => ({
  prewarmManagerSquadPortraits: vi.fn(),
}));

vi.mock("../store/gameStore", () => ({ useGameStore: vi.fn() }));

vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init: () => {} },
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) =>
      params?.defaultValue !== undefined ? String(params.defaultValue) : key,
    i18n: { language: "en" },
  }),
}));

// jsdom doesn't implement matchMedia, which ThemeProvider reads on mount.
Object.defineProperty(window, "matchMedia", {
  writable: true,
  value: vi.fn().mockImplementation((query: string) => ({
    matches: query === "(prefers-color-scheme: dark)",
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
});

function league(overrides: Partial<LeagueData> = {}): LeagueData {
  return {
    id: "epl",
    name: "Premier League",
    season: 2025,
    scope: "Domestic",
    kind: "League",
    region_id: "europe",
    country_id: "GB",
    participant_ids: ["team-1"],
    priority: 1,
    fixtures: [],
    standings: [],
    ...overrides,
  };
}

function buildGameState(): GameStateData {
  return {
    teams: [createTeam({ id: "team-1", name: "Alpha FC", country: "GB" })],
    players: [createPlayer({ id: "p1", team_id: "team-1" })],
    regions: [{ id: "europe", name: "Europe", country_codes: ["GB"] }],
    competitions: [league()],
    league: null,
    manager: { first_name: "Sam", last_name: "Boss" },
  } as unknown as GameStateData;
}

describe("TeamSelection", () => {
  beforeEach(() => {
    vi.mocked(useGameStore).mockReturnValue({
      gameState: buildGameState(),
      setGameState: vi.fn(),
      setGameActive: vi.fn(),
    } as unknown as ReturnType<typeof useGameStore>);
  });

  it("renders the scope panel, the club grid, and the selected-team sidebar", () => {
    render(
      <ThemeProvider>
        <TeamSelection />
      </ThemeProvider>,
    );

    // Page chrome + scope panel
    expect(screen.getByText("teamSelect.title")).toBeInTheDocument();
    expect(screen.getByText("teamSelect.simulationScope")).toBeInTheDocument();

    // Club grid renders the team; sidebar auto-selects the first team, so the
    // club name appears in both the card and the sidebar header.
    expect(screen.getAllByText("Alpha FC").length).toBeGreaterThanOrEqual(2);

    // Confirm button reflects the auto-selected club.
    expect(screen.getByText("teamSelect.manage")).toBeInTheDocument();
  });
});
