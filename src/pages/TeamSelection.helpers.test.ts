import { describe, expect, it } from "vitest";
import type { TFunction } from "i18next";

import type { LeagueData } from "../store/types";
import type { GameStateData } from "../store/gameStore";
import { createTeam } from "../test-utils/factories";
import { regionsFromCompetitions, sortCompetitions } from "./TeamSelection.helpers";

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
