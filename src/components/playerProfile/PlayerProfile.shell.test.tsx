import { fireEvent, render, screen, within } from "@testing-library/react";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import { i18nReady } from "../../i18n";
import type { GameStateData, PlayerData } from "../../store/gameStore";
import { createPlayer, createSeasonContext, createTeam } from "../../test-utils/factories";
import PlayerProfile from "./PlayerProfile";

// The shell of the profile: the name card above four tabs. What each tab holds is the business of the
// slices that fill them; these tests pin the frame, and that no fact is on the page twice.

beforeAll(async () => {
  await i18nReady;
});

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("../../hooks/useRatingTiers", async () => {
  const { cannedRatingTiersModule } = await import("../../test-utils/ratingTiers");
  return cannedRatingTiersModule();
});
vi.mock("../../hooks/useContractWarningStages", () => ({
  useContractRiskLevels: () => () => "stable",
}));
vi.mock("../../hooks/usePlayerKnowledge", () => ({
  usePlayerKnowledge: () => () => ({ kind: "own" }),
}));
vi.mock("../../hooks/useVisibleWage", () => ({
  useVisibleWage: () => ({ status: "answered", value: { kind: "exact", weekly: 12000 } }),
}));
vi.mock("../../hooks/usePotentialOpinion", () => ({
  usePotentialOpinion: () => ({ status: "answered", value: null }),
}));
vi.mock("../../hooks/usePositionRatings", () => ({
  usePositionRatings: () => ({ status: "answered", value: null }),
}));

function gameWith(player: PlayerData): GameStateData {
  return {
    season_context: createSeasonContext(),
    clock: { current_date: "2026-08-01T00:00:00Z", start_date: "2026-07-01T00:00:00Z" },
    manager: {
      display_name: "Jane Doe",
      id: "manager-1",
      first_name: "Jane",
      last_name: "Doe",
      date_of_birth: "1980-01-01",
      nationality: "GB",
      reputation: 50,
      satisfaction: 50,
      fan_approval: 50,
      team_id: "team-1",
      career_stats: { matches_managed: 0, wins: 0, draws: 0, losses: 0, trophies: 0, best_finish: null },
      career_history: [],
    },
    teams: [createTeam({ id: "team-1", name: "Alpha FC" })],
    players: [player],
    staff: [],
    messages: [],
    news: [],
    league: { id: "league-1", name: "League", season: 1, fixtures: [], standings: [] },
    scouting_assignments: [],
    board_objectives: [],
  } as unknown as GameStateData;
}

function renderProfile(overrides: Partial<PlayerData> = {}) {
  const player = createPlayer({
    id: "player-1",
    team_id: "team-1",
    condition: 92,
    morale: 75,
    market_value: 350000,
    wage: 12000,
    ...overrides,
  });
  return render(
    <PlayerProfile
      player={player}
      gameState={gameWith(player)}
      isOwnClub
      onGameUpdate={() => {}}
      onSelectTeam={() => {}}
    />,
  );
}

beforeEach(() => {
  vi.mocked(invoke).mockImplementation(async (command: string) => {
    if (command === "get_player_stats_overview") {
      return { percentileEligible: false, metrics: {} };
    }
    if (command === "get_player_match_history") return [];
    if (command === "get_scouted_report") return null;
    return null;
  });
});

describe("PlayerProfile shell", () => {
  /**
   * Given a player's profile
   * When it opens
   * Then there are four tabs, Overview, Attributes, Career and Contract, and Overview is the
   *      one showing
   */
  it("opens on the Overview of four tabs", () => {
    renderProfile();

    const tabs = within(screen.getByRole("tablist")).getAllByRole("tab");
    expect(tabs.map((tab) => tab.textContent)).toEqual([
      "Overview",
      "Attributes",
      "Career",
      "Contract",
    ]);
    expect(screen.getByRole("tab", { name: "Overview" })).toHaveAttribute("aria-selected", "true");
  });

  /**
   * Given the profile open on its Overview, the Overview tab focused
   * When the right arrow and then End are pressed
   * Then the selection follows to Attributes and then to Contract, and each tab's panel is the
   *      one showing
   */
  it("moves between the tabs from the keyboard", () => {
    renderProfile();
    const overview = screen.getByRole("tab", { name: "Overview" });
    overview.focus();

    fireEvent.keyDown(overview, { key: "ArrowRight" });
    expect(screen.getByRole("tab", { name: "Attributes" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(screen.getByRole("tabpanel", { name: "Attributes" })).toBeVisible();

    fireEvent.keyDown(screen.getByRole("tab", { name: "Attributes" }), { key: "End" });
    expect(screen.getByRole("tab", { name: "Contract" })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByRole("tabpanel", { name: "Contract" })).toBeVisible();
  });

  /**
   * Given the profile of a player of the manager's own club
   * When it opens
   * Then condition, morale, value and wage are each on the page once: the name card carries no
   *      band of them, and the Contract tab holds them (the Overview takes them in a later slice)
   */
  it("shows condition, morale, value and wage once each", () => {
    renderProfile();

    expect(screen.getAllByText("Condition")).toHaveLength(1);
    expect(screen.getAllByText("Morale")).toHaveLength(1);
    expect(screen.getAllByText(/^(Value|Market Value)$/)).toHaveLength(1);
    expect(screen.getAllByText(/^(Wage|Weekly Wage)$/)).toHaveLength(1);
  });

  /**
   * Given the profile of a player of the manager's own club
   * When it opens
   * Then the name card holds the player's name, his position badges, his club and his rating, and
   *      the Actions menu beside the name
   */
  it("puts the name, positions, club, rating and Actions in the name card", () => {
    renderProfile({ alternate_positions: ["Midfielder"] });

    const card = screen.getByRole("region", { name: "John Smith" });
    expect(within(card).getByRole("heading", { name: "John Smith" })).toBeInTheDocument();
    expect(within(card).getByRole("button", { name: "Alpha FC" })).toBeInTheDocument();
    expect(within(card).getByRole("button", { name: /Actions/ })).toBeInTheDocument();
    expect(within(card).getByText("Overall")).toBeInTheDocument();
  });
});
