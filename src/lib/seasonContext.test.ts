import { describe, expect, it } from "vitest";

import type { GameStateData } from "../store/gameStore";
import { hasCompetitiveStandings, resolveSeasonContext } from "./seasonContext";

function createGameState(overrides: Partial<GameStateData> = {}): GameStateData {
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
    league: {
      id: "league-1",
      name: "Premier Division",
      season: 2026,
      fixtures: [
        {
          id: "fixture-1",
          matchday: 1,
          date: "2026-08-01",
          home_team_id: "team-1",
          away_team_id: "team-2",
          competition: "League",
          status: "Scheduled",
          result: null,
        },
      ],
      standings: [
        {
          team_id: "team-1",
          played: 0,
          won: 0,
          drawn: 0,
          lost: 0,
          goals_for: 0,
          goals_against: 0,
          points: 0,
        },
        {
          team_id: "team-2",
          played: 0,
          won: 0,
          drawn: 0,
          lost: 0,
          goals_for: 0,
          goals_against: 0,
          points: 0,
        },
      ],
    },
    scouting_assignments: [],
    board_objectives: [],
    ...overrides,
  };
}

describe("seasonContext", function (): void {
  it("derives preseason state when backend season_context is missing", function (): void {
    const context = resolveSeasonContext(createGameState());

    expect(context.phase).toBe("Preseason");
    expect(context.season_start).toBe("2026-08-01");
    expect(context.days_until_season_start).toBe(22);
    expect(context.transfer_window.status).toBe("Open");
    expect(hasCompetitiveStandings(createGameState())).toBe(false);
  });

  it("prefers the backend-provided season_context when present", function (): void {
    const gameState = createGameState({
      season_context: {
        phase: "InSeason",
        season_start: "2026-08-01",
        season_end: "2027-05-20",
        days_until_season_start: null,
        transfer_window: {
          status: "DeadlineDay",
          opens_on: "2026-07-02",
          closes_on: "2026-08-31",
          days_until_opens: null,
          days_remaining: 0,
        },
      },
    });

    const context = resolveSeasonContext(gameState);

    expect(context.phase).toBe("InSeason");
    expect(context.transfer_window.status).toBe("DeadlineDay");
    expect(hasCompetitiveStandings(gameState)).toBe(true);
  });

  it("ignores preseason friendlies when deriving the competitive season start", function (): void {
    const gameState = createGameState({
      league: {
        id: "league-1",
        name: "Premier Division",
        season: 2026,
        fixtures: [
          {
            id: "friendly-1",
            matchday: 0,
            date: "2026-07-15",
            home_team_id: "team-1",
            away_team_id: "team-3",
            competition: "Friendly",
            status: "Scheduled",
            result: null,
          },
          {
            id: "fixture-1",
            matchday: 1,
            date: "2026-08-01",
            home_team_id: "team-1",
            away_team_id: "team-2",
            competition: "League",
            status: "Scheduled",
            result: null,
          },
        ],
        standings: [
          {
            team_id: "team-1",
            played: 0,
            won: 0,
            drawn: 0,
            lost: 0,
            goals_for: 0,
            goals_against: 0,
            points: 0,
          },
          {
            team_id: "team-2",
            played: 0,
            won: 0,
            drawn: 0,
            lost: 0,
            goals_for: 0,
            goals_against: 0,
            points: 0,
          },
        ],
      },
    });

    const context = resolveSeasonContext(gameState);

    expect(context.phase).toBe("Preseason");
    expect(context.season_start).toBe("2026-08-01");
  });

  it("keeps the transfer window closed before the preseason opening threshold", function (): void {
    const context = resolveSeasonContext(
      createGameState({
        clock: {
          current_date: "2026-06-30T12:00:00Z",
          start_date: "2026-06-01T12:00:00Z",
        },
      }),
    );

    expect(context.transfer_window.status).toBe("Closed");
    expect(context.transfer_window.days_until_opens).toBe(2);
    expect(context.transfer_window.opens_on).toBe("2026-07-02");
    expect(context.transfer_window.closes_on).toBe("2026-08-31");
  });

  it("rolls the next transfer opening forward after the current window closes", function (): void {
    const context = resolveSeasonContext(
      createGameState({
        clock: {
          current_date: "2026-09-15T12:00:00Z",
          start_date: "2026-07-01T12:00:00Z",
        },
      }),
    );

    expect(context.transfer_window.status).toBe("Closed");
    expect(context.transfer_window.days_until_opens).toBe(290);
    expect(context.transfer_window.opens_on).toBe("2027-07-02");
    expect(context.transfer_window.closes_on).toBe("2027-08-31");
  });

  it("marks deadline day when the transfer window reaches its closing threshold", function (): void {
    const context = resolveSeasonContext(
      createGameState({
        clock: {
          current_date: "2026-08-31T12:00:00Z",
          start_date: "2026-07-01T12:00:00Z",
        },
      }),
    );

    expect(context.transfer_window.status).toBe("DeadlineDay");
    expect(context.transfer_window.days_remaining).toBe(0);
  });
});
