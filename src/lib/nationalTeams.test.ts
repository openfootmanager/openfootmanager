import { describe, expect, it } from "vitest";

import type {
  FixtureData,
  GameStateData,
  NationalTeamData,
  PlayerData,
} from "../store/gameStore";
import {
  getNationalTeamFixtures,
  getNationalTeamName,
  getUserCalledUpPlayers,
} from "./nationalTeams";

function ntFixture(overrides: Partial<FixtureData> = {}): FixtureData {
  return {
    id: "ntf-1",
    competition_id: "international-friendlies",
    matchday: 1,
    date: "2026-09-09",
    home_team_id: "nt-eng",
    away_team_id: "nt-bra",
    competition: "InternationalNation",
    status: "Scheduled",
    result: null,
    ...overrides,
  };
}

function nationalTeam(overrides: Partial<NationalTeamData> = {}): NationalTeamData {
  return {
    id: "nt-eng",
    name: "ENG National Team",
    football_nation: "ENG",
    squad_player_ids: [],
    reputation: 500,
    fixtures: [],
    ...overrides,
  };
}

function player(overrides: Partial<PlayerData> = {}): PlayerData {
  return { id: "p1", match_name: "J. Doe", team_id: "team-1", ...overrides } as PlayerData;
}

function gameState(overrides: Partial<GameStateData> = {}): GameStateData {
  return {
    manager: { team_id: "team-1" },
    players: [],
    national_teams: [],
    ...overrides,
  } as unknown as GameStateData;
}

describe("getNationalTeamFixtures", () => {
  it("flattens fixtures across national teams", () => {
    const state = gameState({
      national_teams: [
        nationalTeam({ id: "nt-eng", fixtures: [ntFixture({ id: "a" })] }),
        nationalTeam({ id: "nt-fra", fixtures: [ntFixture({ id: "b" }), ntFixture({ id: "c" })] }),
      ],
    });

    expect(getNationalTeamFixtures(state).map((f) => f.id)).toEqual(["a", "b", "c"]);
  });

  it("returns an empty list when there are no national teams", () => {
    expect(getNationalTeamFixtures(gameState())).toEqual([]);
  });

  it("includes fixtures from national-team tournaments like the World Cup", () => {
    const state = gameState({
      national_teams: [nationalTeam({ id: "nt-eng", fixtures: [ntFixture({ id: "friendly" })] })],
      competitions: [
        {
          id: "wc-2026",
          name: "World Cup 2026",
          kind: "InternationalNation",
          season: 2026,
          fixtures: [ntFixture({ id: "wc-final" })],
          standings: [],
        },
        {
          id: "league-1",
          name: "Club League",
          kind: "League",
          season: 2026,
          fixtures: [ntFixture({ id: "club-fixture" })],
          standings: [],
        },
      ],
    } as Partial<GameStateData>);

    const ids = getNationalTeamFixtures(state).map((f) => f.id);
    expect(ids).toContain("friendly");
    expect(ids).toContain("wc-final");
    expect(ids).not.toContain("club-fixture");
  });
});

describe("getNationalTeamName", () => {
  it("resolves a nation's display name by id", () => {
    const state = gameState({
      national_teams: [nationalTeam({ id: "nt-eng", name: "ENG National Team" })],
    });

    expect(getNationalTeamName(state, "nt-eng")).toBe("ENG National Team");
  });

  it("falls back to the id when the nation is unknown", () => {
    expect(getNationalTeamName(gameState(), "nt-xyz")).toBe("nt-xyz");
  });
});

describe("getUserCalledUpPlayers", () => {
  it("returns the user's players whose nation has fixtures, even away nations", () => {
    const state = gameState({
      players: [
        player({ id: "p1", match_name: "Home Star", team_id: "team-1" }),
        player({ id: "p2", match_name: "Away Star", team_id: "team-1" }),
        player({ id: "p3", match_name: "Rival", team_id: "team-2" }),
        player({ id: "p4", match_name: "Benched", team_id: "team-1" }),
      ],
      national_teams: [
        // Fixtures live only on the home nation (single ownership).
        nationalTeam({
          id: "nt-eng",
          squad_player_ids: ["p1"],
          fixtures: [ntFixture({ home_team_id: "nt-eng", away_team_id: "nt-bra" })],
        }),
        nationalTeam({ id: "nt-bra", squad_player_ids: ["p2"], fixtures: [] }),
        // A nation with a squad but no fixtures -> p4 is not called up.
        nationalTeam({ id: "nt-ger", squad_player_ids: ["p4"], fixtures: [] }),
      ],
    });

    const calledUp = getUserCalledUpPlayers(state);

    expect(calledUp.map((c) => c.player.id).sort()).toEqual(["p1", "p2"]);
    expect(calledUp.find((c) => c.player.id === "p2")?.nationalTeamId).toBe("nt-bra");
  });

  it("returns an empty list when the manager has no club", () => {
    const state = gameState({ manager: { team_id: null } } as Partial<GameStateData>);
    expect(getUserCalledUpPlayers(state)).toEqual([]);
  });
});
