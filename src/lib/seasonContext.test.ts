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
      display_name: "Jane Doe",
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

describe("seasonContext", (): void => {
  /**
   * Given a game with fixtures and a date inside the transfer window but no season_context
   * When the context is resolved
   * Then it is the neutral default (preseason, window closed, no dates): the screen does not work
   *      the season or the window out from fixtures, which is the backend's rule alone.
   */
  it("does not derive the season from fixtures when the backend sent no season_context", (): void => {
    const context = resolveSeasonContext(createGameState());

    expect(context.phase).toBe("Preseason");
    expect(context.season_start).toBeNull();
    expect(context.days_until_season_start).toBeNull();
    expect(context.transfer_window.status).toBe("Closed");
    expect(context.transfer_window.opens_on).toBeNull();
    expect(hasCompetitiveStandings(createGameState())).toBe(false);
  });

  it("prefers the backend-provided season_context when present", (): void => {
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
});

describe("season_complete", () => {
  it("is carried through normalisation, so the dashboard can trust it", () => {
    // The backend decides whether the rollover is available. The dashboard used
    // to work it out locally from `competitions[0]` — whichever competition
    // sorted first, which in a generated world is a foreign league on another
    // calendar — so an English career whose season ended in April waited on an
    // Argentine Apertura running to October and could never continue.
    const complete = resolveSeasonContext({
      season_context: {
        phase: "PostSeason",
        season_complete: true,
        season_start: null,
        season_end: null,
        days_until_season_start: null,
        transfer_window: {
          status: "Closed",
          opens_on: null,
          closes_on: null,
          days_until_opens: null,
          days_remaining: null,
        },
      },
    } as never);
    expect(complete.season_complete).toBe(true);

    const stillPlaying = resolveSeasonContext({
      season_context: { phase: "InSeason" },
    } as never);
    expect(stillPlaying.season_complete).toBe(false);
  });
});
