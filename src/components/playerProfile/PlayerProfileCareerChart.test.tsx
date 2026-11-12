import { act, fireEvent, render, screen, within } from "@testing-library/react";
import type { ReactElement, ReactNode } from "react";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

import { i18nReady } from "../../i18n";
import type { CareerLine } from "../../store/generated/CareerLine";
import type { CareerSeasonRow } from "../../store/generated/CareerSeasonRow";
import type { PlayerCareer } from "../../store/generated/PlayerCareer";
import { formatProfileDay } from "./PlayerProfile.helpers";
import PlayerProfileCareerChart from "./PlayerProfileCareerChart";

interface Datum {
  key: string;
  label: string;
  value: number;
  bandIndex: number;
}
interface TickProps {
  x: number;
  y: number;
  index: number;
  payload: { value: string };
}

// What the chart hands recharts, captured: jsdom has no layout, so the real chart draws nothing to
// read. The bars' data, their click handler, each bar's cell and the axis' tick renderer are what
// the screen decides; recharts draws them.
const handed = vi.hoisted(() => ({
  data: [] as Datum[],
  onClick: undefined as ((entry: { key?: string; payload?: { key: string } }) => void) | undefined,
  cells: [] as { fill?: string }[],
  tick: undefined as ((props: TickProps) => ReactElement) | undefined,
  allowDecimals: undefined as boolean | undefined,
}));

vi.mock("recharts", () => {
  const Passthrough = ({ children }: { children?: ReactNode }) => <div>{children}</div>;
  return {
    ResponsiveContainer: Passthrough,
    BarChart: (props: { data: Datum[]; children?: ReactNode }) => {
      handed.data = props.data;
      handed.cells = [];
      return <div>{props.children}</div>;
    },
    Bar: (props: { onClick?: typeof handed.onClick; children?: ReactNode }) => {
      handed.onClick = props.onClick;
      return <div>{props.children}</div>;
    },
    Cell: (props: { fill?: string }) => {
      handed.cells.push(props);
      return null;
    },
    CartesianGrid: () => null,
    LabelList: () => null,
    XAxis: (props: { tick?: typeof handed.tick }) => {
      handed.tick = props.tick;
      return null;
    },
    YAxis: (props: { allowDecimals?: boolean }) => {
      handed.allowDecimals = props.allowDecimals;
      return null;
    },
  };
});

beforeAll(async () => {
  await i18nReady;
});

// A translator that shows its options, so a figure and its unit are told from its key.
const t = (key: string, options?: Record<string, string | number>) =>
  options ? `${key} ${JSON.stringify(options)}` : key;

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
    {
      kind: "move",
      key: "m",
      date: "2024-07-01",
      movement: "permanent_transfer",
      from_club_name: "Ashford",
      to_club_name: "North",
      fee: 9_500_000,
      loan_end: null,
    },
    season({ season: 2023, club_id: "west", goals: 8, assists: 3, appearances: 14, on_loan: true }),
    season({ season: 2023, club_id: "ash", goals: 6, assists: 2, appearances: 17 }),
  ],
  by_club: [],
  totals: { appearances: 100, goals: 45, assists: 20 },
};

const readout = () =>
  within(screen.getByRole("region", { name: "playerProfile.careerChartReadout" }));

function renderChart(onOpenTeam?: (teamId: string) => void) {
  return render(
    <PlayerProfileCareerChart career={CAREER} language="en" onOpenTeam={onOpenTeam} t={t} />,
  );
}

/** Clicks a bar the way recharts reports it: the datum behind the bar. */
const clickBar = (index: number) => act(() => handed.onClick?.({ key: handed.data[index].key }));

beforeEach(() => {
  handed.data = [];
  handed.onClick = undefined;
  handed.cells = [];
  handed.tick = undefined;
});

describe("PlayerProfileCareerChart", () => {
  /**
   * Given the backend's career
   * When the chart is drawn
   * Then recharts is handed a bar for each season, oldest first, valued by goals, labelled by its
   *      season, with the band of its club, and counts in whole numbers only
   */
  it("hands recharts a bar for each season, oldest first, valued by the metric", () => {
    renderChart();

    expect(handed.data.map((d) => [d.label, d.value, d.bandIndex])).toEqual([
      ["2023/24", 6, 0],
      ["2023/24", 8, 1],
      ["2024/25", 16, 2],
      ["2025/26", 15, 2],
    ]);
    expect(handed.allowDecimals).toBe(false);
  });

  /**
   * Given the chart just drawn
   * When it is read
   * Then the latest season is the chosen bar: its cell alone has the chosen fill
   */
  it("fills the chosen bar apart from the others, the latest at first", () => {
    renderChart();

    const fills = handed.cells.map((cell) => cell.fill);
    expect(new Set(fills.slice(0, 3)).size).toBe(1);
    expect(fills[3]).not.toBe(fills[0]);
  });

  /**
   * Given the chart on its latest season
   * When a bar is clicked (recharts hands back the bar's datum, or the datum as `payload`)
   * Then the chosen fill and the readout follow it: the season, the club, apps, goals and assists
   */
  it("follows a clicked bar in the fill and in the readout", () => {
    renderChart();

    clickBar(1);

    const fills = handed.cells.map((cell) => cell.fill);
    expect(fills[1]).not.toBe(fills[0]);
    expect(fills[1]).not.toBe(fills[3]);
    const view = readout();
    expect(view.getByText("2023/24")).toBeInTheDocument();
    expect(view.getByText("west")).toBeInTheDocument();
    expect(view.getByText("14")).toBeInTheDocument();
    expect(view.getByText("8")).toBeInTheDocument();
    expect(view.getByText("3")).toBeInTheDocument();
    act(() => handed.onClick?.({ payload: { key: handed.data[0].key } }));
    expect(readout().getByText("ash")).toBeInTheDocument();
  });

  /**
   * Given the chart on goals, with the 2023/24 loan season chosen
   * When Assists, then Appearances, are chosen
   * Then the bars take that metric's figures and the chosen season stays chosen
   */
  it("switches the metric and keeps the chosen season", () => {
    renderChart();
    clickBar(1);

    fireEvent.click(screen.getByRole("radio", { name: "playerProfile.assists" }));
    expect(handed.data.map((d) => d.value)).toEqual([2, 3, 7, 8]);
    expect(readout().getByText("west")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("radio", { name: "playerProfile.apps" }));
    expect(handed.data.map((d) => d.value)).toEqual([17, 14, 36, 33]);
    expect(readout().getByText("west")).toBeInTheDocument();
  });

  /**
   * Given the bars are not focusable (they are drawn by recharts)
   * When the keyboard's previous and next season buttons are used
   * Then the chosen season steps back and forward through the bars, the buttons stop at the ends,
   *      and the readout announces the change
   */
  it("steps through the seasons with buttons a keyboard can reach", () => {
    renderChart();
    const previous = screen.getByRole("button", { name: "playerProfile.careerChartPrevious" });
    const next = screen.getByRole("button", { name: "playerProfile.careerChartNext" });

    expect(next).toBeDisabled();
    fireEvent.click(previous);
    expect(readout().getByText("2024/25")).toBeInTheDocument();
    fireEvent.click(previous);
    fireEvent.click(previous);
    expect(readout().getByText("ash")).toBeInTheDocument();
    expect(previous).toBeDisabled();
    fireEvent.click(next);
    expect(readout().getByText("west")).toBeInTheDocument();
    expect(
      screen.getByRole("region", { name: "playerProfile.careerChartReadout" }),
    ).toHaveAttribute("aria-live", "polite");
  });

  /**
   * Given a transfer that sits between the 2023/24 seasons and the 2024/25 one, and a season with
   *       no move before it
   * When each is chosen
   * Then the readout lists the moves before the season (their date, what, the fee) under its
   *      heading, or says there was none before it: chronology only, no claim about which stint a
   *      move began
   */
  it("lists the moves before the chosen season, or says there were none", () => {
    renderChart();

    clickBar(2);
    expect(readout().getByText("playerProfile.careerChartMovesSince")).toBeInTheDocument();
    expect(readout().getByText(new RegExp(formatProfileDay("2024-07-01", "en")))).toHaveTextContent(
      'transferFrom {"club":"Ashford"}',
    );
    expect(readout().getByText(/€9,500,000/)).toBeInTheDocument();

    clickBar(3);
    expect(readout().getByText("playerProfile.careerChartNoMoveSince")).toBeInTheDocument();
    expect(readout().queryByText("playerProfile.careerChartMovesSince")).toBeNull();
  });

  /**
   * Given a club played for twice and a loan at another
   * When the chart is drawn
   * Then the legend names each club once under the number its ticks carry, with the loan marked as
   *      one, in the order they appear: a club is never told apart by colour alone
   */
  it("lists each club once in the legend, numbered, a loan as a loan", () => {
    renderChart();

    const legend = within(screen.getByRole("list", { name: "playerProfile.careerChartClubs" }));
    expect(legend.getAllByRole("listitem").map((item) => item.textContent)).toEqual([
      "1 ash",
      '2 playerProfile.careerChartLoanClub {"club":"west"}',
      "3 north",
    ]);
  });

  /**
   * Given the axis' tick renderer recharts is handed
   * When it draws the tick of each bar
   * Then the tick is the bar's season with a band under it in its club's colour and the club's
   *      number, and bars of one club share both while a loan has its own
   */
  it("draws each axis tick as the season with its club's band and number under it", () => {
    renderChart();
    const band = (index: number) => {
      const { container } = render(
        <svg role="img" aria-label="tick">
          {handed.tick?.({ x: 10, y: 20, index, payload: { value: handed.data[index].label } })}
        </svg>,
      );
      const texts = [...container.querySelectorAll("text")].map((text) => text.textContent);
      return {
        season: texts[0],
        number: texts[1],
        fill: container.querySelector("rect")?.getAttribute("fill"),
      };
    };

    const [ash, west, north24, north25] = [0, 1, 2, 3].map(band);

    expect(ash.season).toBe("2023/24");
    expect(north25.season).toBe("2025/26");
    expect(north24.fill).toBe(north25.fill);
    expect(new Set([ash.fill, west.fill, north24.fill]).size).toBe(3);
    expect(ash.fill).toMatch(/^var\(--color-/);
    // The number is the legend's: the same club has the same one on every tick, a loan its own.
    expect([ash, west, north24, north25].map((tick) => tick.number)).toEqual(["1", "2", "3", "3"]);
  });

  /**
   * Given the selected season's club and a way to open clubs
   * When the club in the readout is clicked
   * Then that club opens
   */
  it("opens the club of the chosen season", () => {
    const open = vi.fn();
    renderChart(open);

    fireEvent.click(
      within(screen.getByRole("region", { name: "playerProfile.careerChartReadout" })).getByRole(
        "button",
        { name: "north" },
      ),
    );

    expect(open).toHaveBeenCalledWith("north");
  });

  /**
   * Given the loan season chosen
   * When the readout is read
   * Then it says the season was a loan, in words, as the table does; a season at the club's own
   *      does not
   */
  it("says in the readout when the chosen season was a loan", () => {
    renderChart();

    clickBar(1);
    expect(readout().getByText("playerProfile.careerOnLoan")).toBeInTheDocument();
    clickBar(3);
    expect(readout().queryByText("playerProfile.careerOnLoan")).toBeNull();
  });

  /**
   * Given a season at no club (the backend sends an empty club name for it)
   * When the chart is drawn
   * Then the legend names it the way the readout does (the free-agent label), never an unnamed
   *      swatch
   */
  it("names a spell at no club in the legend", () => {
    render(
      <PlayerProfileCareerChart
        career={{
          ...CAREER,
          by_season: [season({ season: 2025, club_id: "", club_name: "" })],
        }}
        language="en"
        t={t}
      />,
    );

    const legend = within(screen.getByRole("list", { name: "playerProfile.careerChartClubs" }));
    expect(legend.getAllByRole("listitem").map((item) => item.textContent)).toEqual([
      "1 common.freeAgent",
    ]);
  });

  /**
   * Given a career with moves but no season row at all
   * When the chart is drawn
   * Then the chart says there is no data yet, not a blank card
   */
  it("shows the empty state for a career with no season", () => {
    render(
      <PlayerProfileCareerChart
        career={{ ...CAREER, by_season: [CAREER.by_season[2]] }}
        language="en"
        t={t}
      />,
    );

    expect(screen.getByText("No data yet")).toBeInTheDocument();
    expect(screen.queryByRole("radiogroup")).toBeNull();
  });

  /**
   * Given the first bar of a career that begins with an opening contract, and one with none
   * When each is chosen
   * Then the readout words the moves as "since joining" (the first bar has no previous spell to
   *      count from), and says none since joining when there is none
   */
  it("words the first bar's moves as since joining", () => {
    renderChart();
    // The first bar (Ashford 2023/24) has no move listed below it.
    clickBar(0);
    expect(readout().getByText("playerProfile.careerChartNoMoveFirst")).toBeInTheDocument();

    render(
      <PlayerProfileCareerChart
        career={{
          ...CAREER,
          by_season: [
            season({ season: 2025, club_id: "a", key: "only" }),
            {
              kind: "move",
              key: "first",
              date: "2024-07-01",
              movement: "initial_contract",
              from_club_name: null,
              to_club_name: "a",
              fee: null,
              loan_end: null,
            },
          ],
        }}
        language="en"
        t={t}
      />,
    );
    expect(screen.getAllByText("playerProfile.careerChartMovesFirst")).toHaveLength(1);
  });

  /**
   * Given two 2025 rows at club A with a loan in October 2025 between them (a loan to a club he
   *       never played for, so it has no row of its own)
   * When the later row is chosen
   * Then the loan is listed as a move since the previous spell, not as one "before this season":
   *      it happened inside the season
   */
  it("words a mid-season move as since the previous spell", () => {
    render(
      <PlayerProfileCareerChart
        career={{
          ...CAREER,
          by_season: [
            season({ season: 2025, club_id: "a", key: "later" }),
            {
              kind: "move",
              key: "loan",
              date: "2025-10-01",
              movement: "loan_start",
              from_club_name: "A",
              to_club_name: "B",
              fee: null,
              loan_end: "2026-01-05",
            },
            season({ season: 2025, club_id: "a", key: "first" }),
          ],
        }}
        language="en"
        t={t}
      />,
    );

    // The latest bar is chosen at first: it is the later 2025 row.
    const view = readout();
    expect(view.getByText("playerProfile.careerChartMovesSince")).toBeInTheDocument();
    expect(view.getByText(/loanTo/)).toBeInTheDocument();
    expect(view.queryByText(/before this season/i)).toBeNull();
  });
});
