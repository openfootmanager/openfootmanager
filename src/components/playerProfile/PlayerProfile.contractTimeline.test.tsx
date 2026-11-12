import { fireEvent, render, screen, within } from "@testing-library/react";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import { i18nReady } from "../../i18n";
import type { GameStateData, PlayerData, PlayerMovementEntry } from "../../store/gameStore";
import { createPlayer, createSeasonContext, createTeam } from "../../test-utils/factories";
import PlayerProfile from "./PlayerProfile";

// The Contract tab draws "Contracts over time" for his own player and for a rival's (a rival's has
// no wages: the bars carry dates only). These pin that the profile mounts it on that tab at all.

const mocks = vi.hoisted(() => ({
  knowledge: { kind: "own" } as { kind: string },
  wage: { kind: "exact", weekly: 12000 } as { kind: string; weekly?: number },
}));

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
  usePlayerKnowledge: () => () => mocks.knowledge,
}));
vi.mock("../../hooks/useVisibleWage", () => ({
  useVisibleWage: () => ({ status: "answered", value: mocks.wage }),
}));
vi.mock("../../hooks/usePotentialOpinion", () => ({
  usePotentialOpinion: () => ({ status: "answered", value: null }),
}));
vi.mock("../../hooks/usePositionRatings", () => ({
  usePositionRatings: () => ({ status: "answered", value: null }),
}));

const LEDGER = [
  {
    date: "2029-07-01",
    kind: "permanent_transfer",
    from_team_id: "other",
    from_team_name: "Other FC",
    to_team_id: "team-1",
    to_team_name: "Alpha FC",
    contract: { start: "2029-07-01", end: "2031-06-30", weekly_wage: 7000, source: "transfer" },
  },
  {
    date: "2031-07-01",
    kind: "renewal",
    to_team_id: "team-1",
    to_team_name: "Alpha FC",
    contract: { start: "2031-07-01", end: "2033-06-30", weekly_wage: 12000, source: "renewal" },
  },
] as PlayerMovementEntry[];

function gameWith(player: PlayerData): GameStateData {
  return {
    season_context: createSeasonContext(),
    clock: { current_date: "2031-08-01T00:00:00Z", start_date: "2031-07-01T00:00:00Z" },
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

function openContract(player: PlayerData, isOwnClub: boolean) {
  render(
    <PlayerProfile
      player={player}
      gameState={gameWith(player)}
      isOwnClub={isOwnClub}
      onGameUpdate={() => {}}
      onSelectTeam={() => {}}
    />,
  );
  fireEvent.click(screen.getByRole("tab", { name: "Contract" }));
  return screen.getByRole("tabpanel", { name: "Contract" });
}

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  vi.mocked(invoke).mockResolvedValue(null);
});

describe("the Contract tab's timeline", () => {
  /**
   * Given his own player with two contracts on record
   * When the Contract tab is opened
   * Then the tab holds the named "Contracts over time" chart, with a bar for each contract and the
   *      weekly wage on each
   */
  it("is on the Contract tab for his own player, with wages", () => {
    mocks.knowledge = { kind: "own" };
    mocks.wage = { kind: "exact", weekly: 12000 };
    const player = createPlayer({
      id: "player-1",
      team_id: "team-1",
      wage: 12000,
      movement_history: LEDGER,
    });

    const panel = within(openContract(player, true));

    expect(panel.getByText("Contracts over time")).toBeInTheDocument();
    const chart = panel.getByRole("img", {
      name: "A timeline of his contracts, with today marked",
    });
    expect(within(chart).getAllByTitle(/Alpha FC/)).toHaveLength(2);
    expect(within(chart).getByText("€12K")).toBeInTheDocument();
    expect(within(chart).getByText("€7K")).toBeInTheDocument();
  });

  /**
   * Given a division rival's player with the same two contracts, whose wages the club may not see
   * When the Contract tab is opened
   * Then the chart is there with its bars, and no wage is on any of them
   */
  it("is on the Contract tab for a rival's player, without wages", () => {
    mocks.knowledge = { kind: "known" };
    mocks.wage = { kind: "hidden" };
    const player = createPlayer({
      id: "player-2",
      team_id: "team-2",
      wage: 12000,
      movement_history: LEDGER,
    });

    const panel = within(openContract(player, false));

    const chart = panel.getByRole("img", {
      name: "A timeline of his contracts, with today marked",
    });
    expect(within(chart).getAllByTitle(/Alpha FC/)).toHaveLength(2);
    expect(within(chart).queryByText(/€/)).toBeNull();
  });
});
