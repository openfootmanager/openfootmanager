import { describe, expect, it } from "vitest";

import type { CareerLine } from "../../store/generated/CareerLine";
import type { CareerSeasonRow } from "../../store/generated/CareerSeasonRow";
import type { PlayerCareer } from "../../store/generated/PlayerCareer";
import { buildCareerChart } from "./PlayerProfile.careerChart";

const season = (
  patch: Partial<CareerSeasonRow> & Pick<CareerSeasonRow, "season" | "club_id">,
): CareerLine => ({
  kind: "season",
  key: `${patch.season}-${patch.club_id}`,
  club_name: patch.club_id,
  appearances: 10,
  goals: 0,
  assists: 0,
  in_progress: false,
  on_loan: false,
  ...patch,
});

const MOVE: Extract<CareerLine, { kind: "move" }> = {
  kind: "move",
  key: "m",
  date: "2024-07-01",
  movement: "permanent_transfer",
  from_club_name: "Ashford",
  to_club_name: "North",
  fee: 9_500_000,
  loan_end: null,
};

// Newest first, as the backend lists it: a transfer line sits just below the first row at the new club.
const CAREER: PlayerCareer = {
  by_season: [
    season({
      season: 2025,
      club_id: "north",
      goals: 15,
      assists: 8,
      appearances: 33,
      in_progress: true,
    }),
    season({ season: 2024, club_id: "north", goals: 16, assists: 7, appearances: 36 }),
    MOVE,
    season({ season: 2023, club_id: "west", goals: 8, assists: 3, appearances: 14, on_loan: true }),
    season({ season: 2023, club_id: "ash", goals: 6, assists: 2, appearances: 17 }),
  ],
  by_club: [],
  totals: { appearances: 100, goals: 45, assists: 20 },
};

describe("buildCareerChart", () => {
  /**
   * Given the backend's career, newest first, with a move line
   * When the chart is built for goals
   * Then there is a bar for each season row, oldest first and with no move among them, each with
   *      its goals as the value
   */
  it("makes a bar of each season row, oldest first, valued by the chosen metric", () => {
    const { bars } = buildCareerChart(CAREER, "goals");

    expect(bars.map((bar) => [bar.season, bar.clubId, bar.value])).toEqual([
      [2023, "ash", 6],
      [2023, "west", 8],
      [2024, "north", 16],
      [2025, "north", 15],
    ]);
  });

  /**
   * Given the same career
   * When the metric is assists, then appearances
   * Then the values follow the metric, and each bar still carries all three figures
   */
  it("follows the metric and keeps every figure on the bar", () => {
    const assists = buildCareerChart(CAREER, "assists").bars;
    const apps = buildCareerChart(CAREER, "appearances").bars;

    expect(assists.map((bar) => bar.value)).toEqual([2, 3, 7, 8]);
    expect(apps.map((bar) => bar.value)).toEqual([17, 14, 36, 33]);
    expect(assists[2]).toMatchObject({ goals: 16, assists: 7, appearances: 36 });
  });

  /**
   * Given a transfer line right below the first season at the new club
   * When the chart is built
   * Then that season's bar lists it among the moves before it, and the others list none
   */
  it("lists the moves between the previous season and each bar", () => {
    const { bars } = buildCareerChart(CAREER, "goals");

    expect(bars.map((bar) => bar.movesBefore.map((move) => move.key))).toEqual([[], [], ["m"], []]);
  });

  /**
   * Given club A, a loan line (to B, with no appearance for B, so no row) and a return to A
   * When the chart is built
   * Then the loan line is a move before the later A bar and nothing more is claimed: the list says
   *      when it happened in the career, not that it began that stint
   */
  it("says only where a move sits in the career, not which stint it began", () => {
    const loan: CareerLine = { ...MOVE, key: "loan", movement: "loan_start" };
    const career: PlayerCareer = {
      ...CAREER,
      by_season: [
        season({ season: 2025, club_id: "a", key: "later" }),
        loan,
        season({ season: 2025, club_id: "a", key: "first" }),
      ],
    };

    const { bars } = buildCareerChart(career, "goals");

    expect(bars.map((bar) => [bar.key, bar.movesBefore.map((move) => move.key)])).toEqual([
      ["first", []],
      ["later", ["loan"]],
    ]);
  });

  /**
   * Given two moves in a row below one season (newest listed first, as the backend lists them)
   * When the chart is built
   * Then both are before that bar, oldest first, as they happened
   */
  it("lists several moves before a bar oldest first", () => {
    const older: CareerLine = { ...MOVE, key: "older", date: "2024-01-01" };
    const newer: CareerLine = { ...MOVE, key: "newer", date: "2024-06-01" };
    const career: PlayerCareer = {
      ...CAREER,
      by_season: [
        season({ season: 2025, club_id: "a", key: "now" }),
        newer,
        older,
        season({ season: 2023, club_id: "a", key: "then" }),
      ],
    };

    const { bars } = buildCareerChart(career, "goals");

    expect(bars[1].movesBefore.map((move) => move.key)).toEqual(["older", "newer"]);
  });

  /**
   * Given a club played for in two seasons and a loan at another
   * When the chart is built
   * Then each club has one band, in the order it first appears, and a loan spell is its own band
   */
  it("gives each club one band and a loan its own", () => {
    const { bars, bands } = buildCareerChart(CAREER, "goals");

    expect(bars.map((bar) => bar.bandIndex)).toEqual([0, 1, 2, 2]);
    expect(bands.map((band) => [band.clubId, band.onLoan, band.bandIndex])).toEqual([
      ["ash", false, 0],
      ["west", true, 1],
      ["north", false, 2],
    ]);
  });

  /** Given the season in progress, then its bar says so. */
  it("marks the season in progress", () => {
    const { bars } = buildCareerChart(CAREER, "goals");

    expect(bars.map((bar) => bar.inProgress)).toEqual([false, false, false, true]);
  });

  /** Given no season row at all, then there is no chart. */
  it("is empty without seasons", () => {
    expect(
      buildCareerChart({ by_season: [], by_club: [], totals: CAREER.totals }, "goals"),
    ).toEqual({ bars: [], bands: [] });
  });
});
