import { describe, expect, it } from "vitest";

import {
  gameKnowledgeRevision,
  knowledgeRevision,
  visibleRatingText,
  visibleWageText,
} from "./playerKnowledge";

describe("visibleWageText", () => {
  const format = (weekly: number) => `€${weekly}/wk`;
  const t = (key: string, options?: Record<string, unknown>) => `${key} ${JSON.stringify(options)}`;

  /**
   * Given an exact, an approximate and a hidden wage
   * When each is written
   * Then the exact one is its figure, the approximate one is "about" its figure, and the hidden
   *      one is "–"
   */
  it("writes a wage as exact, about the figure, or a dash", () => {
    expect(visibleWageText({ kind: "exact", weekly: 15_843 }, format, t)).toBe("€15843/wk");
    expect(visibleWageText({ kind: "approximate", weekly: 16_000 }, format, t)).toBe(
      'playerProfile.approximateWage {"wage":"€16000/wk"}',
    );
    expect(visibleWageText({ kind: "hidden" }, format, t)).toBe("–");
  });
});

describe("visibleRatingText", () => {
  /**
   * Given a rating the club knows, and one it does not
   * When each is written for a list or a card
   * Then the known one is its number and the unknown one is "–"
   */
  it("writes a known rating as its number and an unknown one as a dash", () => {
    expect(visibleRatingText(77)).toBe("77");
    expect(visibleRatingText(null)).toBe("–");
  });
});

describe("knowledgeRevision", () => {
  const inputs = {
    careerToken: 1,
    date: "2026-09-15",
    playerClubId: "team-2",
    managerClubId: "team-1",
    seasonStart: "2026-07-01",
    wage: 12_000,
  };

  /**
   * Given a profile open on one career
   * When another career is installed under it, on the same date, with the same clubs and wage
   * Then the key moves, so the club's knowledge and the wage it may see are asked again rather
   *      than carried over from the career that is gone
   */
  it("moves when another career replaces the one the profile was opened on", () => {
    expect(knowledgeRevision({ ...inputs, careerToken: 2 })).not.toBe(knowledgeRevision(inputs));
  });

  /**
   * Given a profile open on one day
   * When the game moves on a day, nothing else changing (a scout's report can land, a report can
   *      go out of date)
   * Then the key moves, so the club's knowledge is asked again
   */
  it("moves when the game moves on a day", () => {
    expect(knowledgeRevision({ ...inputs, date: "2026-09-16" })).not.toBe(
      knowledgeRevision(inputs),
    );
  });

  /**
   * Given a profile open on a player
   * When his wage changes (a renewal), nothing else changing
   * Then the key moves, so the wage the club may see is asked again
   */
  it("moves when the player's wage changes", () => {
    expect(knowledgeRevision({ ...inputs, wage: 15_000 })).not.toBe(knowledgeRevision(inputs));
  });

  /**
   * Given a player at club "a|b" under a manager at "c"
   * When he moves to "a" and the manager to "b|c" (ids from a package may hold any character)
   * Then the key moves: the fields are kept apart, not run together with a separator
   */
  it("moves when two club ids trade characters across a separator", () => {
    expect(knowledgeRevision({ ...inputs, playerClubId: "a", managerClubId: "b|c" })).not.toBe(
      knowledgeRevision({ ...inputs, playerClubId: "a|b", managerClubId: "c" }),
    );
  });

  /**
   * Given a profile open on the last day of a season
   * When the season rolls over (promotion and relegation move clubs between divisions, and so
   *      who is known) without the date moving
   * Then the key moves, so the club's knowledge is asked again
   */
  it("moves when the season rolls over on the same date", () => {
    expect(knowledgeRevision({ ...inputs, seasonStart: "2027-07-01" })).not.toBe(
      knowledgeRevision(inputs),
    );
  });

  /**
   * Given the same career, date, clubs and wage
   * When the key is taken twice
   * Then it is the same, so nothing is asked again for no reason
   */
  it("stays put while nothing it depends on changes", () => {
    expect(knowledgeRevision({ ...inputs })).toBe(knowledgeRevision(inputs));
  });
});

describe("gameKnowledgeRevision", () => {
  const game = {
    career_token: 3,
    clock: { current_date: "2026-09-15" },
    manager: { team_id: "team-1" },
    season_context: { season_start: "2026-07-01" },
  };

  /**
   * Given the game a screen shows
   * When its knowledge revision is taken
   * Then it is the key of that game's career, date, manager's club and season start, read in one
   *      place for every screen
   */
  it("reads the career, the date, the manager's club and the season start from the game", () => {
    expect(gameKnowledgeRevision(game)).toBe(
      knowledgeRevision({
        careerToken: 3,
        date: "2026-09-15",
        managerClubId: "team-1",
        seasonStart: "2026-07-01",
      }),
    );
  });

  /**
   * Given a profile that also depends on its player's club and wage
   * When its knowledge revision is taken
   * Then the key carries those too, beside the game's four
   */
  it("adds a screen's own inputs to the game's", () => {
    expect(gameKnowledgeRevision(game, { playerClubId: "team-2", wage: 12_000 })).toBe(
      knowledgeRevision({
        careerToken: 3,
        date: "2026-09-15",
        playerClubId: "team-2",
        managerClubId: "team-1",
        seasonStart: "2026-07-01",
        wage: 12_000,
      }),
    );
  });
});
