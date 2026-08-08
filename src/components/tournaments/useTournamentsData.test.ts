import { describe, expect, it, vi, beforeEach } from "vitest";
import { act, renderHook, waitFor } from "@testing-library/react";

import { fetchCompetitionsView } from "../../services/competitionsService";
import { useTournamentsData } from "./useTournamentsData";
import type { GameStateData, LeagueData } from "../../store/gameStore";
import type { CompetitionsView } from "../../services/competitionsService";

vi.mock("../../services/competitionsService", () => ({
  fetchCompetitionsView: vi.fn(),
}));

vi.mock("../../lib/seasonContext", () => ({
  resolveSeasonContext: () => ({ phase: "Preseason", season_start: null }),
}));

const mockedFetch = vi.mocked(fetchCompetitionsView);

function competition(id: string, over: Partial<LeagueData> = {}): LeagueData {
  return {
    id,
    name: id,
    season: 2026,
    fixtures: [],
    standings: [],
    participant_ids: [],
    ...over,
  } as unknown as LeagueData;
}

function gameState(over: Partial<GameStateData> = {}): GameStateData {
  return {
    clock: { current_date: "2026-08-01" },
    manager: { team_id: "team-1" },
    teams: [{ id: "team-1", name: "Alpha FC" }],
    national_teams: [],
    players: [],
    competitions: [competition("league-1")],
    active_competition_ids: ["league-1"],
    ...over,
  } as unknown as GameStateData;
}

/** A slice payload; every field overrides its gameState counterpart. */
function slice(over: Partial<CompetitionsView> = {}): CompetitionsView {
  return {
    competitions: [competition("league-1")],
    team_names: {},
    national_team_names: {},
    national_team_name_keys: {},
    player_names: {},
    world_cup_champions: [],
    manager_team_id: null,
    active_competition_ids: [],
    ...over,
  };
}

beforeEach(() => {
  mockedFetch.mockReset();
  mockedFetch.mockReturnValue(new Promise(() => {}));
});

describe("useTournamentsData", () => {
  it("names teams from gameState until the slice arrives", () => {
    const { result } = renderHook(() => useTournamentsData(gameState()));

    expect(result.current.teamNames).toEqual({ "team-1": "Alpha FC" });
    expect(result.current.userTeamId).toBe("team-1");
  });

  it("prefers the slice's names once it arrives", async () => {
    mockedFetch.mockResolvedValue(
      slice({
        team_names: { "team-1": "Alpha Football Club" },
        manager_team_id: "team-9",
      }),
    );

    const { result } = renderHook(() => useTournamentsData(gameState()));

    await waitFor(() =>
      expect(result.current.teamNames).toEqual({
        "team-1": "Alpha Football Club",
      }),
    );
    expect(result.current.userTeamId).toBe("team-9");
  });

  it("refetches the slice when the game clock advances", async () => {
    mockedFetch.mockResolvedValue(slice());

    const { rerender } = renderHook(({ state }) => useTournamentsData(state), {
      initialProps: { state: gameState() },
    });
    await waitFor(() => expect(mockedFetch).toHaveBeenCalledTimes(1));

    rerender({
      state: gameState({
        clock: { current_date: "2026-08-02" },
      } as Partial<GameStateData>),
    });

    await waitFor(() => expect(mockedFetch).toHaveBeenCalledTimes(2));
  });

  // An empty active list means "no filter applied yet", not "nothing active" —
  // filtering on it would blank the screen while the slice is still loading.
  it("treats an empty active list as every competition", () => {
    const { result } = renderHook(() =>
      useTournamentsData(
        gameState({
          competitions: [competition("a"), competition("b")],
          active_competition_ids: [],
        }),
      ),
    );

    expect(result.current.activeCompetitions.map((c) => c.id)).toEqual([
      "a",
      "b",
    ]);
  });

  it("shows only the competitions listed as active", () => {
    const { result } = renderHook(() =>
      useTournamentsData(
        gameState({
          competitions: [competition("a"), competition("b")],
          active_competition_ids: ["b"],
        }),
      ),
    );

    expect(result.current.activeCompetitions.map((c) => c.id)).toEqual(["b"]);
  });

  // With nothing selected, the competition the player is actually in wins over
  // whichever happens to sort first.
  it("opens on a competition the managed club is in", () => {
    const { result } = renderHook(() =>
      useTournamentsData(
        gameState({
          competitions: [
            competition("cup"),
            competition("league", { participant_ids: ["team-1"] }),
          ],
          active_competition_ids: ["cup", "league"],
        }),
      ),
    );

    expect(result.current.league?.id).toBe("league");
  });

  it("falls back to the first active competition when the club is in none", () => {
    const { result } = renderHook(() =>
      useTournamentsData(
        gameState({
          competitions: [competition("cup"), competition("plate")],
          active_competition_ids: ["cup", "plate"],
        }),
      ),
    );

    expect(result.current.league?.id).toBe("cup");
  });

  it("follows the competition the player picks", () => {
    const { result } = renderHook(() =>
      useTournamentsData(
        gameState({
          competitions: [competition("cup"), competition("plate")],
          active_competition_ids: ["cup", "plate"],
        }),
      ),
    );

    act(() => result.current.setSelectedCompetitionId("plate"));

    expect(result.current.league?.id).toBe("plate");
  });

  // A selection can outlive the competition it points at — a cup ends, a season
  // rolls over — and must not leave the screen pointing at nothing.
  it("moves the selection on when the chosen competition stops being active", () => {
    const { result, rerender } = renderHook(
      ({ state }) => useTournamentsData(state),
      {
        initialProps: {
          state: gameState({
            competitions: [competition("cup"), competition("plate")],
            active_competition_ids: ["cup", "plate"],
          }),
        },
      },
    );
    act(() => result.current.setSelectedCompetitionId("plate"));
    expect(result.current.league?.id).toBe("plate");

    rerender({
      state: gameState({
        competitions: [competition("cup")],
        active_competition_ids: ["cup"],
      }),
    });

    expect(result.current.league?.id).toBe("cup");
  });

  it("names the reigning champion of a World Cup's season", async () => {
    mockedFetch.mockResolvedValue(
      slice({
        competitions: [
          competition("wc", { kind: "InternationalNation" } as Partial<LeagueData>),
        ],
        active_competition_ids: ["wc"],
        world_cup_champions: [
          { year: 2026, nation_name: "Brazil" },
          { year: 2022, nation_name: "Argentina" },
        ] as CompetitionsView["world_cup_champions"],
      }),
    );

    const { result } = renderHook(() => useTournamentsData(gameState()));

    await waitFor(() =>
      expect(result.current.worldCupChampion?.nation_name).toBe("Brazil"),
    );
  });

  it("names no champion for a competition that is not a World Cup", async () => {
    mockedFetch.mockResolvedValue(
      slice({
        world_cup_champions: [
          { year: 2026, nation_name: "Brazil" },
        ] as CompetitionsView["world_cup_champions"],
      }),
    );

    const { result } = renderHook(() => useTournamentsData(gameState()));

    await waitFor(() => expect(result.current.league?.id).toBe("league-1"));
    expect(result.current.worldCupChampion).toBeNull();
  });
});
