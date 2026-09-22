import { describe, expect, it } from "vitest";

import type { LeagueData } from "../store/gameStore";
import { getPromotionRelegationZones } from "./pyramid";

function division(overrides: Partial<LeagueData>): LeagueData {
  return {
    id: "div",
    name: "Division",
    season: 1,
    country_id: "ENG",
    priority: 0,
    participant_ids: [],
    rules: { format: "LeagueTable", counts_in_season_flow: true },
    fixtures: [],
    standings: [],
    ...overrides,
  } as LeagueData;
}

function clubs(prefix: string, count: number): string[] {
  return Array.from({ length: count }, (_, i) => `${prefix}-${i}`);
}

describe("getPromotionRelegationZones", () => {
  const top = division({ id: "eng-1", priority: 0, participant_ids: clubs("t", 10) });
  const second = division({ id: "eng-2", priority: 1, participant_ids: clubs("s", 10) });

  it("marks relegation slots in the top division and promotion slots below", () => {
    const state = [top, second];

    // 10-club divisions swap two each way (min(10,10)/5).
    expect(getPromotionRelegationZones(state, top)).toEqual({
      promotionSlots: 0,
      relegationSlots: 2,
    });
    expect(getPromotionRelegationZones(state, second)).toEqual({
      promotionSlots: 2,
      relegationSlots: 0,
    });
  });

  it("returns no zones for a standalone league", () => {
    const state = [top];

    expect(getPromotionRelegationZones(state, top)).toEqual({
      promotionSlots: 0,
      relegationSlots: 0,
    });
  });

  it("ignores cups and other countries when finding neighbours", () => {
    const cup = division({
      id: "eng-cup",
      priority: 2,
      rules: { format: "Knockout", counts_in_season_flow: true },
      participant_ids: clubs("c", 20),
    });
    const foreign = division({ id: "bra-1", country_id: "BRA", priority: 1 });
    const state = [top, cup, foreign];

    expect(getPromotionRelegationZones(state, top)).toEqual({
      promotionSlots: 0,
      relegationSlots: 0,
    });
  });

  it("never returns more relegation slots than the division size", () => {
    const tinyTop = division({ id: "eng-1", priority: 0, participant_ids: clubs("t", 2) });
    const tinySecond = division({ id: "eng-2", priority: 1, participant_ids: clubs("s", 2) });
    const state = [tinyTop, tinySecond];

    const zones = getPromotionRelegationZones(state, tinyTop);
    expect(zones.relegationSlots).toBeLessThanOrEqual(2);
  });

  it("swaps at least one club even for tiny divisions", () => {
    const tinyTop = division({ id: "eng-1", priority: 0, participant_ids: clubs("t", 4) });
    const tinySecond = division({ id: "eng-2", priority: 1, participant_ids: clubs("s", 4) });
    const state = [tinyTop, tinySecond];

    expect(getPromotionRelegationZones(state, tinyTop).relegationSlots).toBe(1);
    expect(getPromotionRelegationZones(state, tinySecond).promotionSlots).toBe(1);
  });

  // The frontend draws these bands from its own copy of the pyramid rules, so
  // it has to refuse the same pairings the backend refuses. Otherwise the table
  // shows a relegation zone that nothing will ever relegate anyone out of.

  it("draws no zones between the two halves of a split season", () => {
    // What create_game builds for Argentina: one division played twice, at
    // consecutive priorities, over the same twenty clubs.
    const roster = clubs("ar", 20);
    const apertura = division({
      id: "ar-d1-apertura",
      country_id: "AR",
      priority: 0,
      participant_ids: roster,
    });
    const clausura = division({
      id: "ar-d1-clausura",
      country_id: "AR",
      priority: 1,
      participant_ids: roster,
    });
    const state = [apertura, clausura];

    expect(getPromotionRelegationZones(state, apertura)).toEqual({
      promotionSlots: 0,
      relegationSlots: 0,
    });
    expect(getPromotionRelegationZones(state, clausura)).toEqual({
      promotionSlots: 0,
      relegationSlots: 0,
    });
  });

  it("ignores a neighbour that shares clubs with the division", () => {
    // A reserve or B-team table drawn from the same clubs is not a tier below.
    const first = division({
      id: "xx-1",
      country_id: "XX",
      priority: 0,
      participant_ids: clubs("x", 20),
    });
    const shadow = division({
      id: "xx-shadow",
      country_id: "XX",
      priority: 1,
      participant_ids: [...clubs("x", 5), ...clubs("y", 15)],
    });

    expect(getPromotionRelegationZones([first, shadow], first)).toEqual({
      promotionSlots: 0,
      relegationSlots: 0,
    });
  });

  it("ignores competitions that are not domestic league tiers", () => {
    const first = division({
      id: "xx-1",
      country_id: "XX",
      priority: 0,
      participant_ids: clubs("a", 20),
    });
    const regionalCup = division({
      id: "xx-regional",
      country_id: "XX",
      priority: 1,
      participant_ids: clubs("b", 10),
      scope: "Regional",
      kind: "Cup",
    });

    expect(getPromotionRelegationZones([first, regionalCup], first)).toEqual({
      promotionSlots: 0,
      relegationSlots: 0,
    });
    // And such a competition has no zones of its own.
    expect(getPromotionRelegationZones([first, regionalCup], regionalCup)).toEqual({
      promotionSlots: 0,
      relegationSlots: 0,
    });
  });

  it("draws no zones when two eligible tiers share a priority", () => {
    // The backend refuses a ladder run whose tiers do not rank distinctly, so
    // the table must not promise movement it will not make.
    const first = division({
      id: "xx-1",
      country_id: "XX",
      priority: 0,
      participant_ids: clubs("a", 20),
    });
    const peer = division({
      id: "xx-1b",
      country_id: "XX",
      priority: 0,
      participant_ids: clubs("b", 20),
    });
    const below = division({
      id: "xx-2",
      country_id: "XX",
      priority: 1,
      participant_ids: clubs("c", 20),
    });

    for (const competition of [first, peer, below]) {
      expect(getPromotionRelegationZones([first, peer, below], competition)).toEqual({
        promotionSlots: 0,
        relegationSlots: 0,
      });
    }
  });

  it("treats a numeric gap in priorities as adjacent", () => {
    // The backend sorts a country's tiers and chains neighbours by rank order,
    // not by numeric adjacency, so 0 and 2 are neighbouring divisions.
    const first = division({
      id: "xx-1",
      country_id: "XX",
      priority: 0,
      participant_ids: clubs("a", 20),
    });
    const second = division({
      id: "xx-2",
      country_id: "XX",
      priority: 2,
      participant_ids: clubs("b", 20),
    });

    expect(getPromotionRelegationZones([first, second], first)).toEqual({
      promotionSlots: 0,
      relegationSlots: 4,
    });
    expect(getPromotionRelegationZones([first, second], second)).toEqual({
      promotionSlots: 4,
      relegationSlots: 0,
    });
  });
});
