import { describe, expect, it } from "vitest";

import {
  canDelegateToYouthAcademy,
  getPlayerSquadRole,
  isMatchDayEligible,
  isRuledOutOfMatchDay,
  isSeniorSquadPlayer,
  isYouthAcademyPlayer,
} from "./playerSquad";

describe("playerSquad", () => {
  /**
   * Given the backend's match-day verdict as yes, no, and not yet arrived
   * When a list asks whether he can be picked, and whether he is ruled out
   * Then only a yes can be picked and only a no is ruled out: an absent verdict is neither.
   */
  it("reads an absent match-day verdict as neither eligible nor ruled out", () => {
    expect(
      [true, false, undefined].map((verdict) =>
        isMatchDayEligible({ match_day_eligible: verdict }),
      ),
    ).toEqual([true, false, false]);
    expect(
      [true, false, undefined].map((verdict) =>
        isRuledOutOfMatchDay({ match_day_eligible: verdict }),
      ),
    ).toEqual([false, true, false]);
  });

  it("defaults missing squad role to senior", () => {
    expect(getPlayerSquadRole({ squad_role: undefined })).toBe("Senior");
    expect(isSeniorSquadPlayer({ squad_role: undefined })).toBe(true);
  });

  it("identifies youth academy players explicitly", () => {
    expect(isYouthAcademyPlayer({ squad_role: "Youth" })).toBe(true);
    expect(isSeniorSquadPlayer({ squad_role: "Youth" })).toBe(false);
  });

  it("only allows under-21 senior players to be delegated", () => {
    expect(
      canDelegateToYouthAcademy({
        date_of_birth: "2008-01-01",
        squad_role: "Senior",
      }),
    ).toBe(true);
    expect(
      canDelegateToYouthAcademy({
        date_of_birth: "1998-01-01",
        squad_role: "Senior",
      }),
    ).toBe(false);
    expect(
      canDelegateToYouthAcademy({
        date_of_birth: "2008-01-01",
        squad_role: "Youth",
      }),
    ).toBe(false);
  });
});
