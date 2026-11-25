import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import { i18nReady } from "../../i18n";
import type { GameStateData, PlayerData } from "../../store/gameStore";
import { createPlayer, createSeasonContext, createTeam } from "../../test-utils/factories";
import PlayerProfile from "./PlayerProfile";

// The Career tab's career is the backend's projection of the player's seasons, so the open tab asks
// again when anything it reads changes: his season stats at the same date (a fixture played over
// MCP does not move the day) and the career itself (a saved game loaded), not only the day.

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

async function answer(command: string) {
  if (command === "get_player_career") {
    return { by_season: [], by_club: [], totals: { appearances: 0, goals: 0, assists: 0 } };
  }
  if (command === "get_player_match_history") return [];
  return null;
}

const careerAsks = () =>
  vi.mocked(invoke).mock.calls.filter(([command]) => command === "get_player_career").length;

function gameWith(player: PlayerData, careerToken = 1): GameStateData {
  return {
    career_token: careerToken,
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
      career_stats: {
        matches_managed: 0,
        wins: 0,
        draws: 0,
        losses: 0,
        trophies: 0,
        best_finish: null,
      },
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

const props = { isOwnClub: true, onGameUpdate: () => {}, onSelectTeam: () => {} };

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  vi.mocked(invoke).mockImplementation(answer);
});

describe("the Career tab's career follows the player's stats and the game's career", () => {
  /**
   * Given the Career tab open with the career answered
   * When his season stats change on the same day (a fixture played over MCP; the date has not moved)
   * Then the career is asked for again, so the season in progress is not left stale
   */
  it("asks again when his season stats change at the same date", async () => {
    const player = createPlayer({ id: "player-1", team_id: "team-1" });
    const { rerender } = render(
      <PlayerProfile player={player} gameState={gameWith(player)} {...props} />,
    );
    fireEvent.click(screen.getByRole("tab", { name: "Career" }));
    await waitFor(() => expect(careerAsks()).toBe(1));

    const played = {
      ...player,
      stats: { ...player.stats, appearances: player.stats.appearances + 1 },
    };
    rerender(<PlayerProfile player={played} gameState={gameWith(played)} {...props} />);

    await waitFor(() => expect(careerAsks()).toBe(2));
  });

  /**
   * Given the same player and date under another career (a saved game loaded: the career token
   *       differs, the ids and the day do not)
   * Then the career is asked for again, never the old career's answer
   */
  it("asks again under another career token", async () => {
    const player = createPlayer({ id: "player-1", team_id: "team-1" });
    const { rerender } = render(
      <PlayerProfile player={player} gameState={gameWith(player, 1)} {...props} />,
    );
    fireEvent.click(screen.getByRole("tab", { name: "Career" }));
    await waitFor(() => expect(careerAsks()).toBe(1));

    rerender(<PlayerProfile player={player} gameState={gameWith(player, 2)} {...props} />);

    await waitFor(() => expect(careerAsks()).toBe(2));
  });

  /**
   * Given the open Career tab
   * When the game is replaced by an equal one (nothing the career reads changed)
   * Then it is not asked for again
   */
  it("does not ask again when nothing it reads has changed", async () => {
    const player = createPlayer({ id: "player-1", team_id: "team-1" });
    const { rerender } = render(
      <PlayerProfile player={player} gameState={gameWith(player)} {...props} />,
    );
    fireEvent.click(screen.getByRole("tab", { name: "Career" }));
    await waitFor(() => expect(careerAsks()).toBe(1));

    rerender(<PlayerProfile player={player} gameState={gameWith(player)} {...props} />);

    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(careerAsks()).toBe(1);
  });
});
