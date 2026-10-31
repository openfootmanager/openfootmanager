import { describe, expect, it } from "vitest";

import { knowledgeRevision } from "./playerKnowledge";

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
