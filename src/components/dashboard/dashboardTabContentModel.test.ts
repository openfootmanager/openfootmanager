import { describe, expect, it, vi } from "vitest";

import type { GameStateData } from "../../store/gameStore";
import { createDashboardTabContentModel } from "./dashboardTabContentModel";

function createGameState(): GameStateData {
  return {
    clock: {
      current_date: "2026-07-10T12:00:00Z",
      start_date: "2026-07-01T12:00:00Z",
    },
    manager: {
      id: "manager-1",
      first_name: "Jane",
      last_name: "Doe",
      date_of_birth: "1980-01-01",
      nationality: "England",
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
    teams: [],
    players: [],
    staff: [],
    messages: [],
    news: [],
    league: null,
    scouting_assignments: [],
    board_objectives: [],
  };
}

describe("dashboardTabContentModel", function (): void {
  it("derives the manager id and preserves handler references", function (): void {
    const onSelectPlayer = vi.fn();
    const onSelectTeam = vi.fn();
    const onGameUpdate = vi.fn();
    const onNavigate = vi.fn();

    const model = createDashboardTabContentModel({
      activeTab: "Squad",
      gameState: createGameState(),
      seasonComplete: false,
      visitedOnboardingTabs: new Set<string>(["Squad"]),
      initialMessageId: "message-1",
      handlers: {
        onSelectPlayer,
        onSelectTeam,
        onGameUpdate,
        onNavigate,
      },
    });

    expect(model.managerId).toBe("manager-1");
    expect(model.activeTab).toBe("Squad");
    expect(model.initialMessageId).toBe("message-1");
    expect(model.visitedOnboardingTabs.has("Squad")).toBe(true);
    expect(model.handlers.onSelectPlayer).toBe(onSelectPlayer);
    expect(model.handlers.onSelectTeam).toBe(onSelectTeam);
    expect(model.handlers.onGameUpdate).toBe(onGameUpdate);
    expect(model.handlers.onNavigate).toBe(onNavigate);
  });
});
