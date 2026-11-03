import { describe, expect, it } from "vitest";

import type { GameStateData, ManagerData, TeamData } from "../../store/gameStore";
import { createSeasonContext } from "../../test-utils/factories";
import { managersDirectory } from "./managerRows";

function manager(overrides: Partial<ManagerData>): ManagerData {
  return {
    id: "m",
    first_name: "Jane",
    last_name: "Doe",
    display_name: "Jane Doe",
    date_of_birth: "1980-01-01",
    nationality: "ENG",
    reputation: 70,
    satisfaction: 50,
    fan_approval: 50,
    team_id: null,
    career_stats: {
      matches_managed: 0,
      wins: 0,
      draws: 0,
      losses: 0,
      trophies: 0,
      best_finish: null,
    },
    career_history: [],
    ...overrides,
  };
}

function team(id: string, name: string, managerId: string | null, reputation: number): TeamData {
  return { id, name, manager_id: managerId, reputation } as TeamData;
}

function gameState(overrides: Partial<GameStateData>): GameStateData {
  return {
    season_context: createSeasonContext(),
    clock: { current_date: "2026-08-10T00:00:00Z", start_date: "2026-07-01T00:00:00Z" },
    manager: manager({ id: "me", display_name: "Me" }),
    teams: [],
    players: [],
    staff: [],
    messages: [],
    news: [],
    league: null,
    scouting_assignments: [],
    board_objectives: [],
    ...overrides,
  } as GameStateData;
}

describe("managersDirectory", () => {
  /**
   * Given a manager in charge of Alpha FC, with 120 matches and 2 trophies on his record, and one
   *       between jobs
   * When the directory's rows are built
   * Then each row carries the manager's name, nationality and reputation, his club as id and name
   *      (none when between jobs), and his record as matches and trophies: the fields the
   *      backend's ManagerSummary will carry, so the table reads the same row from either source.
   */
  it("gives each manager his club and his record", () => {
    const state = gameState({
      managers: [
        manager({
          id: "jane",
          display_name: "Jane Doe",
          nationality: "ENG",
          reputation: 72,
          team_id: "alpha",
          interim: true,
          career_stats: {
            matches_managed: 120,
            wins: 64,
            draws: 22,
            losses: 34,
            trophies: 2,
            best_finish: 1,
          },
        }),
        manager({ id: "marco", display_name: "Marco Rossi", nationality: "ITA", team_id: null }),
      ],
      teams: [team("alpha", "Alpha FC", "jane", 650)],
    });

    expect(managersDirectory(state).managers).toEqual([
      {
        id: "jane",
        display_name: "Jane Doe",
        nationality: "ENG",
        club: { id: "alpha", name: "Alpha FC" },
        interim: true,
        reputation: 72,
        record: { matches: 120, trophies: 2 },
      },
      {
        id: "marco",
        display_name: "Marco Rossi",
        nationality: "ITA",
        club: null,
        interim: false,
        reputation: 70,
        record: { matches: 0, trophies: 0 },
      },
    ]);
  });

  /**
   * Given a game state from before the world kept every manager (no `managers` list)
   * When the directory's rows are built
   * Then it lists the user's own manager, so the page is never empty of him.
   */
  it("falls back to the user's manager when the world lists none", () => {
    expect(managersDirectory(gameState({})).managers.map((row) => row.id)).toEqual(["me"]);
  });

  /**
   * Given one club with a manager and one without
   * When the directory's vacancies are built
   * Then only the club without a manager is a vacancy, with its name and reputation.
   */
  it("lists each club without a manager as a vacancy", () => {
    const state = gameState({
      teams: [team("alpha", "Alpha FC", "jane", 650), team("beta", "Beta United", null, 410)],
    });
    expect(managersDirectory(state).vacancies).toEqual([
      { club_id: "beta", club_name: "Beta United", club_reputation: 410 },
    ]);
  });
});
