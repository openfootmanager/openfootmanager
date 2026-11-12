import { fireEvent, render, screen, within } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";

import { i18nReady } from "../../i18n";
import type { KnowledgeAnswer } from "../../hooks/useKnowledgeAnswer";
import type { CareerLine } from "../../store/generated/CareerLine";
import type { CareerSeasonRow } from "../../store/generated/CareerSeasonRow";
import type { PlayerCareer } from "../../store/generated/PlayerCareer";
import { formatProfileDay } from "./PlayerProfile.helpers";
import PlayerProfileCareerTable from "./PlayerProfileCareerTable";

beforeAll(async () => {
  await i18nReady;
});

// A day as the profile writes it in English, whatever the format of the day is.
const day = (iso: string) => formatProfileDay(iso, "en");

// A translator that shows its options, so a line's club, fee and date are told from its key.
const t = (key: string, options?: Record<string, string | number>) =>
  options ? `${key} ${JSON.stringify(options)}` : key;

const season = (patch: Partial<CareerSeasonRow> & Pick<CareerSeasonRow, "season" | "club_id">) =>
  ({
    key: `${patch.season}-${patch.club_id}`,
    club_name: patch.club_id,
    appearances: 10,
    goals: 1,
    assists: 0,
    in_progress: false,
    on_loan: false,
    ...patch,
  }) satisfies CareerSeasonRow;

const seasonLine = (row: CareerSeasonRow): CareerLine => ({ kind: "season", ...row });

// The board's career (Career A), as the backend lists it: newest first, each move a line between
// the seasons it falls between.
const BOARD: PlayerCareer = {
  by_season: [
    seasonLine(
      season({
        season: 2026,
        club_id: "north",
        club_name: "Northbridge",
        appearances: 27,
        goals: 19,
        assists: 6,
        in_progress: true,
      }),
    ),
    seasonLine(
      season({
        season: 2025,
        club_id: "north",
        club_name: "Northbridge",
        appearances: 33,
        goals: 15,
        assists: 8,
      }),
    ),
    {
      kind: "move",
      key: "m-transfer",
      date: "2024-07-01",
      movement: "permanent_transfer",
      from_club_name: "Ashford",
      to_club_name: "Northbridge",
      fee: 9_500_000,
      loan_end: null,
    },
    seasonLine(
      season({
        season: 2023,
        club_id: "west",
        club_name: "Westgate",
        appearances: 14,
        goals: 8,
        assists: 3,
        on_loan: true,
      }),
    ),
    {
      kind: "move",
      key: "m-loan",
      date: "2024-01-02",
      movement: "loan_start",
      from_club_name: "Ashford",
      to_club_name: "Westgate",
      fee: null,
      loan_end: "2024-06-30",
    },
    seasonLine(
      season({
        season: 2023,
        club_id: "ash",
        club_name: "Ashford",
        appearances: 17,
        goals: 6,
        assists: 2,
      }),
    ),
    {
      kind: "move",
      key: "m-first",
      date: "2020-07-01",
      movement: "initial_contract",
      from_club_name: null,
      to_club_name: "Ashford",
      fee: null,
      loan_end: null,
    },
  ],
  by_club: [
    {
      key: "north",
      club_id: "north",
      club_name: "Northbridge",
      first_season: 2024,
      last_season: 2026,
      appearances: 60,
      goals: 34,
      assists: 14,
    },
    {
      key: "west",
      club_id: "west",
      club_name: "Westgate",
      first_season: 2023,
      last_season: 2023,
      appearances: 14,
      goals: 8,
      assists: 3,
    },
    {
      key: "ash",
      club_id: "ash",
      club_name: "Ashford",
      first_season: 2020,
      last_season: 2023,
      appearances: 17,
      goals: 6,
      assists: 2,
    },
  ],
  totals: { appearances: 91, goals: 48, assists: 19 },
};

const answered = (value: PlayerCareer | null): KnowledgeAnswer<PlayerCareer | null> => ({
  status: "answered",
  value,
});

function renderTable(
  career: KnowledgeAnswer<PlayerCareer | null>,
  onOpenTeam?: (teamId: string) => void,
) {
  return render(
    <PlayerProfileCareerTable career={career} language="en" onOpenTeam={onOpenTeam} t={t} />,
  );
}

/** The cells of a row, as text. */
const cells = (row: HTMLElement) =>
  within(row)
    .getAllByRole("cell")
    .map((cell) => cell.textContent ?? "");

describe("PlayerProfileCareerTable by season", () => {
  /**
   * Given the backend's career
   * When the table is drawn
   * Then it is a named table opening on By season, with a row for each season (newest first, the
   *      backend's order) and a row for each move, and the seasons read as "2026/27"
   */
  it("opens on By season with the backend's rows in the backend's order", () => {
    renderTable(answered(BOARD));

    expect(screen.getByRole("radio", { name: "playerProfile.careerBySeason" })).toBeChecked();
    const rows = within(screen.getByRole("table", { name: "playerProfile.careerHistory" }))
      .getAllByRole("row")
      .slice(1);
    // The seasons, moves and the total row, in order.
    expect(rows).toHaveLength(BOARD.by_season.length + 1);
    // Every row's identity in order: a season row is its season and club (with its badge), a move
    // is its kind: the loan line sits between the loan season and the one he left.
    const sequence = rows.slice(0, -1).map((row) => {
      const cell = cells(row);
      if (cell.length === 1) return `move:${/careerMove\.(\w+)/.exec(cell[0])?.[1]}`;
      return `${cell[0]}|${cell[1]}`;
    });
    expect(sequence).toEqual([
      "2026/27|NorthbridgeplayerProfile.careerInProgress",
      "2025/26|Northbridge",
      "move:transferFrom",
      "2023/24|WestgateplayerProfile.careerOnLoan",
      "move:loanTo",
      "2023/24|Ashford",
      "move:initial",
    ]);
  });

  /**
   * Given a season with figures
   * When the table is drawn
   * Then its club, appearances, goals and assists are in its row
   */
  it("shows a season's club and figures", () => {
    renderTable(answered(BOARD));

    const row = screen.getByRole("row", { name: /2025\/26/ });
    expect(cells(row)).toEqual(["2025/26", "Northbridge", "33", "15", "8"]);
  });

  /**
   * Given a season in progress and a season on loan
   * When the table is drawn
   * Then the first says it is in progress and the second that it was a loan, in words, and no
   *      other row says either
   */
  it("labels the season in progress and the loan season in words", () => {
    renderTable(answered(BOARD));

    expect(screen.getAllByText("playerProfile.careerInProgress")).toHaveLength(1);
    expect(screen.getAllByText("playerProfile.careerOnLoan")).toHaveLength(1);
    const inProgress = screen.getByRole("row", { name: /2026\/27/ });
    expect(within(inProgress).getByText("playerProfile.careerInProgress")).toBeInTheDocument();
    // The season row, not the loan line that also names Westgate.
    const loan = screen.getByRole("row", { name: /^2023\/24\s*Westgate/ });
    expect(within(loan).getByText("playerProfile.careerOnLoan")).toBeInTheDocument();
  });

  /**
   * Given a transfer with a fee, a loan with an end date and an opening contract
   * When the table is drawn
   * Then each move is a line of its own spanning the table: the date, what happened and to whom,
   *      the fee and the loan's end, and the opening contract is not called a first contract
   */
  it("writes each move as a line with its date, clubs, fee and loan end", () => {
    renderTable(answered(BOARD));

    const transfer = screen.getByRole("row", { name: /playerProfile\.careerMove\.transferFrom/ });
    expect(transfer).toHaveTextContent(day("2024-07-01"));
    expect(transfer).toHaveTextContent('transferFrom {"club":"Ashford"}');
    expect(transfer).toHaveTextContent("€9,500,000");
    expect(within(transfer).getAllByRole("cell")).toHaveLength(1);
    const loan = screen.getByRole("row", { name: /playerProfile\.careerMove\.loanTo/ });
    expect(loan).toHaveTextContent(day("2024-01-02"));
    expect(loan).toHaveTextContent('loanTo {"club":"Westgate"}');
    expect(loan).toHaveTextContent(`loanUntil {"date":"${day("2024-06-30")}"}`);
    const first = screen.getByRole("row", { name: /playerProfile\.careerMove\.initial/ });
    expect(first).toHaveTextContent(day("2020-07-01"));
    expect(first).toHaveTextContent('initial {"club":"Ashford"}');
  });

  /**
   * Given the career's totals
   * When the table is drawn
   * Then the last row is the career total, summing every row the backend sent
   */
  it("ends with the career total the backend summed", () => {
    renderTable(answered(BOARD));

    const rows = screen.getAllByRole("row");
    const total = rows[rows.length - 1];
    expect(cells(total)).toEqual(["playerProfile.careerTotal", "", "91", "48", "19"]);
  });

  /**
   * Given clubs the manager can open
   * When a club's name is clicked
   * Then that club opens; and without a handler (or for a spell at no club) the name is plain text
   */
  it("makes a club a link to it, and plain text where it cannot be opened", () => {
    const open = vi.fn();
    const noClub: PlayerCareer = {
      ...BOARD,
      by_season: [seasonLine(season({ season: 2025, club_id: "", club_name: "" }))],
    };

    const { unmount } = renderTable(answered(BOARD), open);
    fireEvent.click(screen.getAllByRole("button", { name: "Northbridge" })[0]);
    expect(open).toHaveBeenCalledWith("north");
    unmount();

    renderTable(answered(noClub), open);
    expect(screen.queryByRole("button", { name: "common.freeAgent" })).toBeNull();
    expect(screen.getByText("common.freeAgent")).toBeInTheDocument();
  });
});

describe("PlayerProfileCareerTable by club", () => {
  /**
   * Given the backend's career
   * When By club is chosen
   * Then each club is one row with the seasons it spans and the backend's sums, newest first, and
   *      the moves are not lines any more
   */
  it("sums the same rows per club, with no move lines", () => {
    renderTable(answered(BOARD));

    fireEvent.click(screen.getByRole("radio", { name: "playerProfile.careerByClub" }));

    expect(screen.getByRole("radio", { name: "playerProfile.careerByClub" })).toBeChecked();
    const rows = screen.getAllByRole("row").slice(1);
    expect(rows).toHaveLength(BOARD.by_club.length + 1);
    expect(cells(rows[0])).toEqual(["Northbridge", "2024/25 – 2026/27", "60", "34", "14"]);
    expect(cells(rows[1])).toEqual(["Westgate", "2023/24", "14", "8", "3"]);
    expect(cells(rows[2])).toEqual(["Ashford", "2020/21 – 2023/24", "17", "6", "2"]);
    expect(screen.queryByText(/careerMove/)).toBeNull();
  });

  /**
   * Given By club chosen
   * When the table is drawn
   * Then the total row is the same career total
   */
  it("keeps the same total", () => {
    renderTable(answered(BOARD));

    fireEvent.click(screen.getByRole("radio", { name: "playerProfile.careerByClub" }));

    const rows = screen.getAllByRole("row");
    expect(cells(rows[rows.length - 1])).toEqual([
      "playerProfile.careerTotal",
      "",
      "91",
      "48",
      "19",
    ]);
  });
});

describe("PlayerProfileCareerTable states", () => {
  /** Given the career still loading, then it says so and draws no table. */
  it("says it is loading", () => {
    renderTable({ status: "loading" });

    expect(screen.getByText("common.loading")).toBeInTheDocument();
    expect(screen.queryByRole("table")).toBeNull();
  });

  /**
   * Given a career that failed to load
   * When the retry is clicked
   * Then it asks again, and a failure is never drawn as an empty career
   */
  it("offers a retry for a failed load, never an empty career", () => {
    const retry = vi.fn();
    renderTable({ status: "failed", retry });

    expect(screen.queryByText("playerProfile.noCareer")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: /retry/i }));
    expect(retry).toHaveBeenCalled();
  });

  /** Given no career (the backend knows no such player, or he has none), then the card says so. */
  it("says so when there is no career", () => {
    renderTable(
      answered({ by_season: [], by_club: [], totals: { appearances: 0, goals: 0, assists: 0 } }),
    );

    expect(screen.getByText("playerProfile.noCareer")).toBeInTheDocument();
    expect(screen.queryByRole("table")).toBeNull();
    renderTable(answered(null));
    expect(screen.getAllByText("playerProfile.noCareer")).toHaveLength(2);
  });
});
