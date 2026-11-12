import { describe, expect, it } from "vitest";

import type { CareerLine } from "../../store/generated/CareerLine";
import type { PlayerCareer } from "../../store/generated/PlayerCareer";
import { gameState } from "./fixture.generated";
import { HARNESS_PROFILES } from "./harnessPage";
import { profileCase } from "./profileCases";

// What the harness answers as a profile's career follows the backend's projection
// (`ofm_core::player_career`): a career for every active player and for an archived retiree, its
// move lines the player's own movement history's, its sums the sum of its rows.

type SeasonLine = Extract<CareerLine, { kind: "season" }>;
type MoveLine = Extract<CareerLine, { kind: "move" }>;

const seasonRows = (career: PlayerCareer) =>
  career.by_season.filter((line): line is SeasonLine => line.kind === "season");
const moveLines = (career: PlayerCareer) =>
  career.by_season.filter((line): line is MoveLine => line.kind === "move");
const clubName = (teamId: string | null | undefined) =>
  gameState.teams.find((team) => team.id === teamId)?.name ?? null;
const careerOf = (variant: (typeof HARNESS_PROFILES)[number]) =>
  profileCase(variant).career as PlayerCareer;

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
    const sum = (pick: (row: SeasonLine) => number) =>
      rows.reduce((total, row) => total + pick(row), 0);
    expect((career as PlayerCareer).totals).toEqual({
      appearances: sum((row) => row.appearances),
      goals: sum((row) => row.goals),
      assists: sum((row) => row.assists),
    });
  });

  /**
   * Given the player on loan from the club to a rival (a loan that began on 1 July 2031 and runs to
   *       30 June 2032, in his movement history)
   * When his career is read
   * Then the season in progress is at the borrower and marked a loan, the parent's seasons are not,
   *      and the loan line is the history's own (its date, the borrower, the loan's end), directly
   *      below the season it began
   */
  it("shows the loaned player's season in progress at the borrower with his loan's own line", () => {
    const { career: answer, playerPatch } = profileCase("loaned");
    const career = answer as PlayerCareer;
    const rows = seasonRows(career);
    const borrower = clubName(playerPatch?.team_id);

    const current = rows.filter((row) => row.in_progress);
    expect(current).toHaveLength(1);
    expect(current[0]).toMatchObject({ on_loan: true, club_name: borrower });
    expect(rows.filter((row) => !row.in_progress).every((row) => !row.on_loan)).toBe(true);
    const loan = moveLines(career).find((line) => line.movement === "loan_start");
    expect(loan).toMatchObject({
      date: "2031-07-01",
      loan_end: "2032-06-30",
      to_club_name: borrower,
    });
    const at = career.by_season.findIndex((line) => line.kind === "move" && line === loan);
    expect(career.by_season[at - 1]).toMatchObject({ kind: "season", in_progress: true });
    // The loan's undated opening contract is no line: a move with no date cannot be placed.
    expect(moveLines(career).some((line) => line.movement === "initial_contract")).toBe(false);
  });

  /**
   * Given a free agent signed in 2028 and released on 30 June 2031 when his contract ran out
   * When his career is read
   * Then no season is in progress, and his lines are the history's own: the release is the newest
   *      thing in it and the signing, with its fee, the oldest
   */
  it("shows a free agent's own signing and release, with no season in progress", () => {
    const career = careerOf("freeAgent");

    expect(seasonRows(career).some((row) => row.in_progress)).toBe(false);
    expect(career.by_season[0]).toMatchObject({
      kind: "move",
      movement: "released",
      date: "2031-06-30",
    });
    const last = career.by_season[career.by_season.length - 1];
    expect(last).toMatchObject({
      kind: "move",
      movement: "permanent_transfer",
      date: "2028-07-01",
      fee: 500_000,
    });
  });

  /**
   * Given the club's own player, signed from a rival in July 2029 for 1,250,000
   * When his career is read
   * Then the transfer line is between his first season at the club and his last at the old one
   */
  it("places the own player's transfer between the seasons it separates", () => {
    const career = careerOf("own");

    const at = career.by_season.findIndex((line) => line.kind === "move");
    expect(career.by_season[at]).toMatchObject({ movement: "permanent_transfer", fee: 1_250_000 });
    expect(career.by_season[at - 1]).toMatchObject({ kind: "season", season: 2029 });
    expect(career.by_season[at + 1]).toMatchObject({ kind: "season", season: 2028 });
  });

  /**
   * Given the archived retiree
   * When his career is read
   * Then it is his saved seasons and the dated line of his retirement from his archived movements,
   *      with no season in progress
   */
  it("shows the retiree's saved seasons, the dated line of his retirement and nothing in progress", () => {
    const { career, retiredPlayer } = profileCase("retired");

    const rows = seasonRows(career as PlayerCareer);
    expect(rows).toHaveLength(retiredPlayer?.career.length ?? -1);
    expect(rows.some((row) => row.in_progress)).toBe(false);
    const retired = moveLines(career as PlayerCareer).find((line) => line.movement === "retired");
    expect(retired).toMatchObject({
      date: "2031-06-30",
      from_club_name: clubName(retiredPlayer?.movement_history?.[0]?.from_team_id),
    });
    expect(retired?.from_club_name).not.toBeNull();
  });
});
