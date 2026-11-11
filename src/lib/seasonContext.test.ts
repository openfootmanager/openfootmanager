import { describe, expect, it } from "vitest";

import type { GameStateData, SeasonContextData } from "../store/gameStore";
import { createSeasonContext } from "../test-utils/factories";
import { hasCompetitiveStandings, isSeasonComplete } from "./seasonContext";

const gameWith = (context: SeasonContextData): GameStateData =>
  ({ season_context: context }) as GameStateData;

describe("hasCompetitiveStandings", (): void => {
  /**
   * Given the backend's season phase
   * When the screens ask whether standings are live
   * Then preseason says no and every later phase says yes.
   */
  it("is false in preseason and true once the season is under way", (): void => {
    expect(hasCompetitiveStandings(gameWith(createSeasonContext({ phase: "Preseason" })))).toBe(
      false,
    );
    expect(hasCompetitiveStandings(gameWith(createSeasonContext({ phase: "InSeason" })))).toBe(
      true,
    );
    expect(hasCompetitiveStandings(gameWith(createSeasonContext({ phase: "PostSeason" })))).toBe(
      true,
    );
  });
});

describe("isSeasonComplete", (): void => {
  /**
   * Given the backend decides whether the rollover is available
   * When the dashboard asks
   * Then it reads that answer. It once worked it out locally from `competitions[0]`, whichever
   * competition sorted first (in a generated world a foreign league on another calendar), so an
   * English career whose season ended in April waited on an Argentine Apertura running to October
   * and could never continue.
   */
  it("reads the backend's season_complete", (): void => {
    expect(
      isSeasonComplete(
        gameWith(createSeasonContext({ phase: "PostSeason", season_complete: true })),
      ),
    ).toBe(true);
    expect(
      isSeasonComplete(
        gameWith(createSeasonContext({ phase: "InSeason", season_complete: false })),
      ),
    ).toBe(false);
  });
});
