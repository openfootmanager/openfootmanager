import { describe, expect, it } from "vitest";

import type { CareerEntry, PlayerMovementEntry } from "../../store/gameStore";
import {
  type CareerMoveLine,
  type CareerSeasonRow,
  type CareerTimelineInput,
  buildCareerTimeline,
} from "./PlayerProfile.careerTimeline";

const TODAY = "2027-03-14";

const stint = (
  season: number,
  clubId: string,
  clubName: string,
  appearances: number,
  goals: number,
  assists: number,
): CareerEntry => ({
  season,
  team_id: clubId,
  team_name: clubName,
  appearances,
  goals,
  assists,
});

const move = (patch: Partial<PlayerMovementEntry> & Pick<PlayerMovementEntry, "date" | "kind">) =>
  patch as PlayerMovementEntry;

const input = (patch: Partial<CareerTimelineInput> = {}): CareerTimelineInput => ({
  career: [],
  movements: [],
  current: null,
  today: TODAY,
  ...patch,
});

/** A compact reading of the list: a season row as "2025 North 33", a line as "move:<kind>". */
function read(items: (CareerSeasonRow | CareerMoveLine)[]): string[] {
  return items.map((item) =>
    item.kind === "season"
      ? `${item.season} ${item.clubName}${item.inProgress ? " (now)" : ""} ${item.appearances}`
      : `move:${item.move}`,
  );
}

// Daniel's career as the board draws it (Career A): a first contract in 2020, a loan in January
// 2024, a transfer in July 2024, and the season in progress.
const BOARD_CAREER = [
  stint(2020, "ash", "Ashford", 9, 1, 0),
  stint(2021, "ash", "Ashford", 28, 9, 3),
  stint(2022, "ash", "Ashford", 34, 17, 5),
  stint(2023, "ash", "Ashford", 17, 6, 2),
  stint(2023, "west", "Westgate", 14, 8, 3),
  stint(2024, "north", "Northbridge", 36, 16, 7),
  stint(2025, "north", "Northbridge", 33, 15, 8),
];
const BOARD_MOVES = [
  move({
    date: "2020-07-01",
    kind: "initial_contract",
    to_team_id: "ash",
    to_team_name: "Ashford",
  }),
  move({
    date: "2024-01-02",
    kind: "loan_start",
    from_team_id: "ash",
    from_team_name: "Ashford",
    to_team_id: "west",
    to_team_name: "Westgate",
    loan_end_date: "2024-06-30",
  }),
  move({
    date: "2024-06-30",
    kind: "loan_return",
    from_team_id: "west",
    from_team_name: "Westgate",
    to_team_id: "ash",
    to_team_name: "Ashford",
  }),
  move({
    date: "2024-07-01",
    kind: "permanent_transfer",
    from_team_id: "ash",
    from_team_name: "Ashford",
    to_team_id: "north",
    to_team_name: "Northbridge",
    fee: 9_500_000,
  }),
];
const BOARD_NOW = {
  season: 2026,
  club: { id: "north", name: "Northbridge" },
  stats: { appearances: 27, goals: 19, assists: 6 },
  onLoan: false,
};

describe("buildCareerTimeline", () => {
  /**
   * Given the board's career: a first contract, a loan, a transfer and the season in progress
   * When the timeline is built
   * Then the seasons are newest first with each move written where it happened: the loan line
   *      between the loan club's season and the one he left, the transfer line between his first
   *      season at the new club and his last at the old one, the first contract at the bottom
   *      (a loan's return is not a line of its own: the loan line carries its end)
   */
  it("writes each move as a line where it happened, seasons newest first", () => {
    const timeline = buildCareerTimeline(
      input({ career: BOARD_CAREER, movements: BOARD_MOVES, current: BOARD_NOW }),
    );

    expect(read(timeline.bySeason)).toEqual([
      "2026 Northbridge (now) 27",
      "2025 Northbridge 33",
      "2024 Northbridge 36",
      "move:permanent_transfer",
      "2023 Westgate 14",
      "move:loan_start",
      "2023 Ashford 17",
      "2022 Ashford 34",
      "2021 Ashford 28",
      "2020 Ashford 9",
      "move:initial_contract",
    ]);
  });

  /**
   * Given the board's career
   * When the timeline is built
   * Then the transfer line carries its fee and clubs and the loan line its end date, and the loan
   *      season is the only one marked as a loan
   */
  it("carries the fee, the clubs and the loan end, and marks only the loan season", () => {
    const { bySeason } = buildCareerTimeline(
      input({ career: BOARD_CAREER, movements: BOARD_MOVES, current: BOARD_NOW }),
    );

    const transfer = bySeason.find((i) => i.kind === "move" && i.move === "permanent_transfer");
    const loan = bySeason.find((i) => i.kind === "move" && i.move === "loan_start");
    expect(transfer).toMatchObject({
      fee: 9_500_000,
      fromClubName: "Ashford",
      toClubName: "Northbridge",
      date: "2024-07-01",
    });
    expect(loan).toMatchObject({ loanEnd: "2024-06-30", toClubName: "Westgate" });
    const loanSeasons = bySeason.filter((i) => i.kind === "season" && i.onLoan);
    expect(loanSeasons.map((i) => i.kind === "season" && i.clubName)).toEqual(["Westgate"]);
  });

  /**
   * Given the board's career
   * When the timeline is built
   * Then the season in progress is flagged and the totals add every row, it included
   */
  it("flags the season in progress and sums every row into the career totals", () => {
    const timeline = buildCareerTimeline(
      input({ career: BOARD_CAREER, movements: BOARD_MOVES, current: BOARD_NOW }),
    );

    const first = timeline.bySeason[0];
    expect(first).toMatchObject({ kind: "season", inProgress: true, includesEarlierClubs: false });
    expect(timeline.totals).toEqual({ appearances: 198, goals: 91, assists: 34 });
  });

  /**
   * Given the board's career
   * When it is summed by club
   * Then each club is one row with its first and last season and its sums, newest first
   */
  it("sums the same rows by club", () => {
    const { byClub } = buildCareerTimeline(
      input({ career: BOARD_CAREER, movements: BOARD_MOVES, current: BOARD_NOW }),
    );

    expect(byClub).toEqual([
      expect.objectContaining({
        clubName: "Northbridge",
        firstSeason: 2024,
        lastSeason: 2026,
        appearances: 96,
        goals: 50,
        assists: 21,
      }),
      expect.objectContaining({
        clubName: "Westgate",
        firstSeason: 2023,
        lastSeason: 2023,
        appearances: 14,
      }),
      expect.objectContaining({
        clubName: "Ashford",
        firstSeason: 2020,
        lastSeason: 2023,
        appearances: 88,
        goals: 33,
        assists: 10,
      }),
    ]);
  });

  /**
   * Given a transfer in January of the season in progress
   * When the timeline is built
   * Then the season in progress is flagged as counting his earlier club's matches too (his season
   *      totals carry across the move), and its transfer line sits between it and the last row
   */
  it("flags a season in progress that spans a move, and puts the line below it", () => {
    const timeline = buildCareerTimeline(
      input({
        career: [stint(2025, "south", "South FC", 30, 5, 2)],
        movements: [
          move({
            date: "2027-01-10",
            kind: "permanent_transfer",
            from_team_id: "south",
            from_team_name: "South FC",
            to_team_id: "north",
            to_team_name: "North FC",
            fee: 1_000_000,
          }),
        ],
        current: {
          season: 2026,
          club: { id: "north", name: "North FC" },
          stats: { appearances: 20, goals: 4, assists: 1 },
          onLoan: false,
        },
      }),
    );

    expect(read(timeline.bySeason)).toEqual([
      "2026 North FC (now) 20",
      "move:permanent_transfer",
      "2025 South FC 30",
    ]);
    expect(timeline.bySeason[0]).toMatchObject({ includesEarlierClubs: true });
  });

  /**
   * Given a player with no club and no appearance this season
   * When the timeline is built
   * Then there is no row for the season in progress
   */
  it("draws no season in progress for a player with no club and no appearance", () => {
    const timeline = buildCareerTimeline(
      input({
        career: [stint(2025, "north", "North FC", 30, 5, 2)],
        current: {
          season: 2026,
          club: null,
          stats: { appearances: 0, goals: 0, assists: 0 },
          onLoan: false,
        },
      }),
    );

    expect(read(timeline.bySeason)).toEqual(["2025 North FC 30"]);
  });

  /**
   * Given a renewal, a move with no date and a loan's return
   * When the timeline is built
   * Then none of them is a line: a renewal is a contract matter, a move with no date cannot be
   *      placed, and the return is carried by the loan line
   */
  it("leaves out renewals, moves with no date and loan returns", () => {
    const timeline = buildCareerTimeline(
      input({
        career: [stint(2025, "north", "North FC", 30, 5, 2)],
        movements: [
          move({
            date: "2025-07-01",
            kind: "renewal",
            to_team_id: "north",
            to_team_name: "North FC",
          }),
          move({
            date: "",
            kind: "permanent_transfer",
            to_team_id: "north",
            to_team_name: "North FC",
          }),
          move({
            date: "2026-06-30",
            kind: "loan_return",
            to_team_id: "north",
            to_team_name: "North FC",
          }),
        ],
      }),
    );

    expect(read(timeline.bySeason)).toEqual(["2025 North FC 30"]);
  });

  /**
   * Given a save from before the ledger existed: seasons and no movements
   * When the timeline is built
   * Then the seasons are listed newest first with no lines
   */
  it("lists the seasons of a player with no ledger, newest first", () => {
    const timeline = buildCareerTimeline(
      input({
        career: [stint(2023, "a", "A", 10, 1, 1), stint(2024, "b", "B", 20, 2, 2)],
      }),
    );

    expect(read(timeline.bySeason)).toEqual(["2024 B 20", "2023 A 10"]);
  });

  /**
   * Given no career at all
   * When the timeline is built
   * Then every list is empty and the totals are zero
   */
  it("is empty for a player with no career", () => {
    expect(buildCareerTimeline(input())).toEqual({
      bySeason: [],
      byClub: [],
      totals: { appearances: 0, goals: 0, assists: 0 },
    });
  });
});
