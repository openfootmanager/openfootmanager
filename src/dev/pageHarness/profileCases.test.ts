import { describe, expect, it } from "vitest";

import type { PlayerCareer } from "../../store/generated/PlayerCareer";
import { HARNESS_PROFILES } from "./harnessPage";
import { profileCase } from "./profileCases";

const seasonRows = (career: PlayerCareer) =>
  career.by_season.flatMap((line) => (line.kind === "season" ? [line] : []));

describe("the harness's profile cases answer a career the way the backend does", () => {
  /**
   * Given every state of the profile the harness can open on an active player or a retiree
   * When its career answer is read
   * Then there is one for each state, and its totals are the sum of its season rows: the backend
   *      answers a career for every active player and for an archived retiree, never null
   */
  it.each(HARNESS_PROFILES)("has a career whose totals sum its rows: %s", (variant) => {
    const { career } = profileCase(variant);

    expect(career).not.toBeNull();
    const rows = seasonRows(career as PlayerCareer);
    const sum = (pick: (row: (typeof rows)[number]) => number) =>
      rows.reduce((total, row) => total + pick(row), 0);
    expect((career as PlayerCareer).totals).toEqual({
      appearances: sum((row) => row.appearances),
      goals: sum((row) => row.goals),
      assists: sum((row) => row.assists),
    });
  });

  /**
   * Given the player on loan from the club to a rival
   * When his career is read
   * Then the season in progress is at the rival and is marked a loan, and the parent's closed
   *      seasons are not
   */
  it("shows the loaned player's season in progress at the borrower, on loan", () => {
    const rows = seasonRows(profileCase("loaned").career as PlayerCareer);

    const current = rows.filter((row) => row.in_progress);
    expect(current).toHaveLength(1);
    expect(current[0].on_loan).toBe(true);
    expect(rows.filter((row) => !row.in_progress).every((row) => !row.on_loan)).toBe(true);
    expect(current[0].club_name).not.toBe(rows.find((row) => !row.in_progress)?.club_name);
  });

  /**
   * Given a free agent
   * When his career is read
   * Then no season is in progress: he has no club to be playing for
   */
  it("shows a free agent with no season in progress", () => {
    const rows = seasonRows(profileCase("freeAgent").career as PlayerCareer);

    expect(rows.length).toBeGreaterThan(0);
    expect(rows.some((row) => row.in_progress)).toBe(false);
  });

  /**
   * Given the archived retiree
   * When his career is read
   * Then it is his saved seasons only, with no season in progress
   */
  it("shows the retiree's saved seasons and nothing in progress", () => {
    const { career, retiredPlayer } = profileCase("retired");

    const rows = seasonRows(career as PlayerCareer);
    expect(rows).toHaveLength(retiredPlayer?.career.length ?? -1);
    expect(rows.some((row) => row.in_progress)).toBe(false);
  });
});
