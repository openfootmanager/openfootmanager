import { describe, expect, it } from "vitest";
import type { TFunction } from "i18next";

import type { FixtureData, LeagueData, PlayerData } from "../store/types";
import type { GameStateData } from "../store/gameStore";
import { createPlayer, createTeam } from "../test-utils/factories";
import {
  regionsFromCompetitions,
  likelyXi,
  sortCompetitions,
  teamCompetitions,
} from "./TeamSelection.helpers";

import type { ClubSquadRatings } from "../services/teamsService";
const t = ((key: string, opts?: { defaultValue?: string }) =>
  opts?.defaultValue ?? key) as unknown as TFunction;

function league(overrides: Partial<LeagueData> = {}): LeagueData {
  return {
    id: "league-1",
    name: "Test League",
    season: 2025,
    fixtures: [],
    standings: [],
    ...overrides,
  };
}

describe("teamCompetitions", () => {
  it("matches competitions by participant_ids", () => {
    const a = league({ id: "a", participant_ids: ["t1", "t2"] });
    const b = league({ id: "b", participant_ids: ["t3"] });

    expect(teamCompetitions("t1", [a, b])).toEqual([a]);
  });

  it("falls back to fixture membership when participant_ids is absent", () => {
    const fixtures = [{ home_team_id: "t1", away_team_id: "t9" }] as unknown as FixtureData[];
    const a = league({ id: "a", fixtures });

    expect(teamCompetitions("t1", [a])).toEqual([a]);
    expect(teamCompetitions("t5", [a])).toEqual([]);
  });
});

describe("likelyXi", () => {
  /** A club the backend rated, its players in the order given. */
  function rated(ids: string[]): ClubSquadRatings {
    return {
      team_id: "t1",
      average_rating: 70,
      players: ids.map((id, index) => ({ player_id: id, ovr: 90 - index })),
    };
  }
  const byId = (ids: string[]) =>
    new Map(ids.map((id) => [id, createPlayer({ id })] as [string, PlayerData]));

  /**
   * Given a club of fourteen, rated best first by the backend
   * When its likely eleven is read
   * Then it is the first eleven, in the backend's order
   */
  it("is the backend's first eleven, in its order", () => {
    const ids = Array.from({ length: 14 }, (_, index) => `p${index}`);

    const result = likelyXi(rated(ids), byId(ids));

    expect(result.map((player) => player.id)).toEqual(ids.slice(0, 11));
  });

  /**
   * Given a club of two
   * When its likely eleven is read
   * Then it is both
   */
  it("is the whole squad when it has fewer than eleven", () => {
    expect(likelyXi(rated(["a", "b"]), byId(["a", "b"]))).toHaveLength(2);
  });

  /**
   * Given a rated player the game state no longer lists, and a club the backend has not rated
   * When the likely eleven is read
   * Then the missing player is left out, and an unrated club has none yet
   */
  it("leaves out a player it cannot find, and has none before the backend answers", () => {
    expect(likelyXi(rated(["a", "gone"]), byId(["a"])).map((player) => player.id)).toEqual(["a"]);
    expect(likelyXi(undefined, byId(["a"]))).toEqual([]);
  });
});

describe("sortCompetitions", () => {
  it("orders by ascending priority, then by name", () => {
    const result = sortCompetitions([
      league({ id: "c", name: "Charlie", priority: 2 }),
      league({ id: "a", name: "Alpha", priority: 1 }),
      league({ id: "b", name: "Bravo", priority: 1 }),
    ]);

    expect(result.map((c) => c.id)).toEqual(["a", "b", "c"]);
  });

  it("treats missing priority as lowest precedence", () => {
    const result = sortCompetitions([
      league({ id: "none", name: "Zulu" }),
      league({ id: "first", name: "Alpha", priority: 1 }),
    ]);

    expect(result.map((c) => c.id)).toEqual(["first", "none"]);
  });
});

describe("regionsFromCompetitions", () => {
  it("groups team countries by competition-declared region, with sorted country codes", () => {
    const gameState = {
      teams: [
        createTeam({ id: "t1", country: "ES" }),
        createTeam({ id: "t2", country: "FR" }),
        createTeam({ id: "t3", country: "BR" }),
      ],
    } as unknown as GameStateData;

    const competitions = [
      league({ country_id: "ES", region_id: "europe" }),
      league({ country_id: "FR", region_id: "europe" }),
      league({ country_id: "BR", region_id: "south_america" }),
    ];

    const regions = regionsFromCompetitions(t, gameState, competitions);
    const byId = Object.fromEntries(regions.map((region) => [region.id, region.country_codes]));

    expect(byId.europe).toEqual(["ES", "FR"]);
    expect(byId.south_america).toEqual(["BR"]);
  });

  /**
   * Given a club whose country no competition declares a region for
   * When the fallback regions are built
   * Then the club lands in no region, instead of being filed under Europe by a guess.
   */
  it("files a country no competition declares under no region", () => {
    const gameState = {
      teams: [createTeam({ id: "t1", country: "ES" }), createTeam({ id: "t2", country: "NG" })],
    } as unknown as GameStateData;
    const competitions = [league({ country_id: "ES", region_id: "europe" })];

    const regions = regionsFromCompetitions(t, gameState, competitions);

    expect(regions.map((region) => region.id)).toEqual(["europe"]);
    expect(regions.flatMap((region) => region.country_codes)).not.toContain("NG");
  });
});
