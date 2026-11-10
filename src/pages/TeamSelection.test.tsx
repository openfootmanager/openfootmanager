import { fireEvent, render, screen, within } from "@testing-library/react";
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

vi.mock("../hooks/useRatingTiers", async () => {
  const { cannedRatingTiersModule } = await import("../test-utils/ratingTiers");
  return cannedRatingTiersModule();
});

vi.mock("../hooks/usePositionGroups", async () => {
  const { cannedPositionsModule } = await import("../test-utils/positionTable");
  return cannedPositionsModule();
});

vi.mock("../services/portraitService", () => ({
  prewarmManagerSquadPortraits: vi.fn(),
}));

vi.mock("../store/gameStore", () => ({ useGameStore: vi.fn() }));

// One `t` for every render, as react-i18next gives: a fresh function each time would make anything
// memoised on it (the derived regions) a new value on every render.
const translate = vi.hoisted(
  () => (key: string, params?: Record<string, string | number>) =>
    params?.defaultValue !== undefined ? String(params.defaultValue) : key,
);
const i18nState = vi.hoisted(() => ({ language: "en" }));

vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init: () => {} },
  useTranslation: () => ({ t: translate, i18n: i18nState }),
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
    teams: [
      createTeam({ id: "team-1", name: "Alpha FC", short_name: "ALP", country: "GB" }),
      createTeam({ id: "team-2", name: "Bravo Town", short_name: "BRV", country: "GB" }),
    ],
    players: [createPlayer({ id: "p1", team_id: "team-1" })],
    competitions: [league({ participant_ids: ["team-1", "team-2"] })],
    league: null,
    clock: { current_date: "2026-07-01", start_date: "2026-07-01" },
    manager: { first_name: "Sam", last_name: "Boss" },
  } as unknown as GameStateData;
}

function renderPage() {
  render(
    <ThemeProvider>
      <TeamSelection />
    </ThemeProvider>,
  );
}

describe("TeamSelection", () => {
  beforeEach(() => {
    vi.mocked(useGameStore).mockReturnValue({
      gameState: buildGameState(),
      setGameState: vi.fn(),
      setGameActive: vi.fn(),
    } as unknown as ReturnType<typeof useGameStore>);
  });

  /**
   * Given a world with two clubs in one league
   * When the screen opens
   * Then the steps, the country and league rail and the clubs' table are drawn; no club is chosen,
   *      so the card says so and Manage is disabled; and the simulation settings are closed.
   */
  it("opens with no club chosen and Manage disabled", () => {
    renderPage();

    expect(screen.getByRole("heading", { level: 1, name: "teamSelect.title" })).toBeInTheDocument();
    expect(screen.getByRole("list", { name: "teamSelect.steps.label" })).toBeInTheDocument();
    expect(screen.getByRole("navigation", { name: "teamSelect.rail.label" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Alpha FC" })).toHaveAttribute(
      "aria-pressed",
      "false",
    );
    expect(screen.getByRole("button", { name: "Bravo Town" })).toBeInTheDocument();

    expect(screen.getByRole("button", { name: "teamSelect.manageNone" })).toBeDisabled();
    const card = screen.getByRole("complementary", { name: "teamSelect.card.label" });
    expect(within(card).getByText("teamSelect.card.emptyTitle")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "teamSelect.simulation" })).toHaveAttribute(
      "aria-expanded",
      "false",
    );
  });

  /**
   * Given the screen with no club chosen
   * When a club is pressed in the table
   * Then Manage names that club and is enabled, and the chosen-club card shows it.
   */
  it("enables Manage and shows the card once a club is chosen", () => {
    renderPage();

    fireEvent.click(screen.getByRole("button", { name: "Alpha FC" }));

    expect(screen.getByRole("button", { name: "Alpha FC" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.getByRole("button", { name: "teamSelect.manage" })).toBeEnabled();
    const card = screen.getByRole("complementary", { name: "teamSelect.card.label" });
    expect(within(card).getByRole("heading", { name: "Alpha FC" })).toBeInTheDocument();
    expect(within(card).queryByText("teamSelect.card.emptyTitle")).toBeNull();
  });

  /**
   * Given the simulation settings closed
   * When the header's Simulation button is pressed
   * Then the scope panel opens, so the regions and competitions stay reachable until they move
   *      into a dialog.
   */
  it("opens the simulation settings from the header", () => {
    renderPage();

    fireEvent.click(screen.getByRole("button", { name: "teamSelect.simulation" }));

    expect(screen.getByRole("button", { name: "teamSelect.simulation" })).toHaveAttribute(
      "aria-expanded",
      "true",
    );
    expect(screen.getByText("teamSelect.simulationScope")).toBeInTheDocument();
  });
});
