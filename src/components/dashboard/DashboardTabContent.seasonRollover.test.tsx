import { act, fireEvent, render, screen } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

import { i18nReady } from "../../i18n";
import { isSeasonComplete } from "../../lib/seasonContext";
import type { GameStateData } from "../../store/gameStore";
import { createAwards, createEndOfSeasonGame } from "../../test-utils/endOfSeason";
import { createSeasonContext } from "../../test-utils/factories";
import DashboardTabContent from "./DashboardTabContent";
import { createDashboardTabContentModel } from "./dashboardTabContentModel";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

vi.mock("../home/HomeTab", () => ({ default: () => <div>Home Tab Mock</div> }));
vi.mock("../squad/SquadTab", () => ({ default: () => <div>Squad Tab Mock</div> }));

beforeAll(async () => {
  await i18nReady;
});

/** The finished season, and the game the rollover returns: the new season has not started. */
function finishedSeason(): GameStateData {
  return {
    ...createEndOfSeasonGame(),
    season_context: createSeasonContext({ season_complete: true }),
  };
}
function nextSeason(): GameStateData {
  return {
    ...createEndOfSeasonGame(),
    season_context: createSeasonContext({ season_complete: false }),
  };
}

/**
 * The dashboard as the real one drives this: the game lives in state, `seasonComplete` is read from
 * it on every render (as Dashboard.tsx does), and the screen's `onGameUpdate` replaces it.
 */
function Dashboard({ startOn = "Home" }: { startOn?: string }) {
  const [gameState, setGameState] = useState(finishedSeason());
  const [activeTab, setActiveTab] = useState(startOn);
  return (
    <>
      <button type="button" onClick={() => setActiveTab("Squad")}>
        go to squad
      </button>
      <button type="button" onClick={() => setActiveTab("Home")}>
        go to home
      </button>
      <DashboardTabContent
        viewModel={createDashboardTabContentModel({
          activeTab,
          gameState,
          seasonComplete: isSeasonComplete(gameState),
          visitedOnboardingTabs: new Set<string>(),
          initialMessageId: null,
          handlers: {
            onSelectPlayer: vi.fn(),
            onSelectTeam: vi.fn(),
            onGameUpdate: setGameState,
            onNavigate: vi.fn(),
          },
        })}
      />
    </>
  );
}

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  const result = {
    game: nextSeason(),
    summary: {
      season: 6,
      league_name: "Premier League",
      champion_id: "team-1",
      champion_name: "Alpha FC",
      user_position: 1,
      user_points: 83,
      user_won: 25,
      user_drawn: 8,
      user_lost: 5,
      user_goals_for: 72,
      user_goals_against: 30,
      golden_boot_player: "Victor Vale",
      golden_boot_goals: 24,
      poty_player: "Victor Vale",
      poty_rating: 7.8,
      total_teams: 2,
      season_awards: createAwards(),
    },
  };
  vi.mocked(invoke).mockImplementation(async (command) =>
    command === "get_followed_division" ? finishedSeason().league : result,
  );
});

describe("the season rollover, on the dashboard", () => {
  /** Given no followed league, when rollover completes, then skip the ceremony and show the next season. */
  it("skips the ceremony without a followed league", async () => {
    vi.mocked(invoke).mockImplementation(async (command) =>
      command === "get_followed_division"
        ? null
        : {
            game: nextSeason(),
            summary: { season: 6, league_name: "", season_awards: createAwards() },
          },
    );
    render(<Dashboard />);
    fireEvent.click(screen.getByRole("button", { name: "Start Next Season" }));
    expect(await screen.findByText("Season 7")).toBeInTheDocument();
    expect(screen.queryByText("Awards Ceremony")).not.toBeInTheDocument();
    expect(screen.queryByText(/schedule.*released/i)).not.toBeInTheDocument();
  });
  /**
   * Given a finished season on Home, and a rollover whose new game says the season is not complete
   * When the player starts the next season
   * Then the awards ceremony shows (the new game does not swap Home in underneath it), then the
   *      "Season 7" screen, and only "Continue to dashboard" there returns to Home.
   */
  it("shows the awards ceremony and the new-season screen, then Home", async () => {
    render(<Dashboard />);

    fireEvent.click(screen.getByRole("button", { name: "Start Next Season" }));

    expect(await screen.findByText("Awards Ceremony")).toBeInTheDocument();
    expect(screen.queryByText("Home Tab Mock")).not.toBeInTheDocument();

    // The ceremony's own button, then the new season's (its label has a capital D).
    fireEvent.click(screen.getByRole("button", { name: "Continue to dashboard" }));
    expect(await screen.findByText("Season 7")).toBeInTheDocument();
    expect(screen.queryByText("Home Tab Mock")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Continue to Dashboard" }));
    expect(await screen.findByText("Home Tab Mock")).toBeInTheDocument();
  });

  /**
   * Given the ceremony showing
   * When the player visits another tab and comes back to Home
   * Then the ceremony is still there, not the review of a season that is over.
   */
  it("keeps the ceremony when the player leaves Home and returns", async () => {
    render(<Dashboard />);
    fireEvent.click(screen.getByRole("button", { name: "Start Next Season" }));
    await screen.findByText("Awards Ceremony");

    fireEvent.click(screen.getByRole("button", { name: "go to squad" }));
    // The tab is lazy-loaded.
    expect(await screen.findByText("Squad Tab Mock")).toBeInTheDocument();
    await act(async () => fireEvent.click(screen.getByRole("button", { name: "go to home" })));

    expect(screen.getByText("Awards Ceremony")).toBeInTheDocument();
  });
});
