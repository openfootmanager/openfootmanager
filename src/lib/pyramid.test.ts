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
});
