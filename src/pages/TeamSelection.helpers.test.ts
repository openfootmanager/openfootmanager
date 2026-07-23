import { describe, expect, it } from "vitest";
import type { TFunction } from "i18next";

import type { FixtureData, LeagueData } from "../store/types";
import type { GameStateData } from "../store/gameStore";
import { getPlayerOvr } from "../lib/helpers";
import { createPlayer, createTeam } from "../test-utils/factories";
import {
  buildFallbackRegions,
  competitionKindLabel,
  competitionRequiredRegions,
  competitionScopeLabel,
  likelyXi,
  sortCompetitions,
  teamCompetitions,
} from "./TeamSelection.helpers";

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

describe("competitionScopeLabel / competitionKindLabel", () => {
  it("returns null when no scope/kind is provided", () => {
    expect(competitionScopeLabel(t, undefined)).toBeNull();
    expect(competitionKindLabel(t, undefined)).toBeNull();
  });

  it("resolves the scope/kind translation with a defaultValue fallback", () => {
    expect(competitionScopeLabel(t, "Domestic")).toBe("Domestic");
    expect(competitionKindLabel(t, "Cup")).toBe("Cup");
  });
});

describe("competitionRequiredRegions", () => {
  it("adds the region_id for Domestic and Regional scopes", () => {
    expect(
      competitionRequiredRegions(
        league({ scope: "Domestic", region_id: "europe" }),
      ),
    ).toEqual(["europe"]);
    expect(
      competitionRequiredRegions(
        league({ scope: "Regional", region_id: "asia" }),
      ),
    ).toEqual(["asia"]);
  });

  it("does not add the region_id for Continental/International scopes", () => {
    expect(
      competitionRequiredRegions(
        league({ scope: "Continental", region_id: "europe" }),
      ),
    ).toEqual([]);
  });

  it("merges and de-duplicates explicit required_region_ids with the scope region, sorted", () => {
    expect(
      competitionRequiredRegions(
        league({
          scope: "Domestic",
          region_id: "europe",
          required_region_ids: ["south_america", "europe"],
        }),
      ),
    ).toEqual(["europe", "south_america"]);
  });
});

describe("teamCompetitions", () => {
  it("matches competitions by participant_ids", () => {
    const a = league({ id: "a", participant_ids: ["t1", "t2"] });
    const b = league({ id: "b", participant_ids: ["t3"] });

    expect(teamCompetitions("t1", [a, b])).toEqual([a]);
  });

  it("falls back to fixture membership when participant_ids is absent", () => {
    const fixtures = [
      { home_team_id: "t1", away_team_id: "t9" },
    ] as unknown as FixtureData[];
    const a = league({ id: "a", fixtures });

    expect(teamCompetitions("t1", [a])).toEqual([a]);
    expect(teamCompetitions("t5", [a])).toEqual([]);
  });
});

describe("likelyXi", () => {
  it("returns the top 11 players sorted by descending overall", () => {
    const players = Array.from({ length: 14 }, (_, index) =>
      createPlayer({
        id: `p${index}`,
        attributes: {
          ...createPlayer().attributes,
          shooting: 40 + index,
          passing: 40 + index,
          pace: 40 + index,
        },
      }),
    );

    const result = likelyXi(players);

    expect(result).toHaveLength(11);
    for (let i = 1; i < result.length; i += 1) {
      expect(getPlayerOvr(result[i - 1])).toBeGreaterThanOrEqual(
        getPlayerOvr(result[i]),
      );
    }
  });

  it("returns all players when fewer than 11 are provided", () => {
    const players = [createPlayer({ id: "a" }), createPlayer({ id: "b" })];
    expect(likelyXi(players)).toHaveLength(2);
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

describe("buildFallbackRegions", () => {
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

    const regions = buildFallbackRegions(t, gameState, competitions);
    const byId = Object.fromEntries(
      regions.map((region) => [region.id, region.country_codes]),
    );

    expect(byId.europe).toEqual(["ES", "FR"]);
    expect(byId.south_america).toEqual(["BR"]);
  });
});
