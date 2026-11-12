import { fireEvent, render, screen, within } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";

import { i18nReady } from "../../i18n";
import type { CareerLine } from "../../store/generated/CareerLine";
import type { CareerSeasonRow } from "../../store/generated/CareerSeasonRow";
import type { PlayerCareer } from "../../store/generated/PlayerCareer";
import { formatProfileDay } from "./PlayerProfile.helpers";
import PlayerProfileCareerChart from "./PlayerProfileCareerChart";

beforeAll(async () => {
  await i18nReady;
});

// A translator that shows its options, so a bar's season, club and figure are told from its key.
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

const bars = () => screen.getAllByRole("button", { name: /playerProfile\.careerChartBar/ });
const readout = () =>
  within(screen.getByRole("region", { name: "playerProfile.careerChartReadout" }));

function renderChart(onOpenTeam?: (teamId: string) => void) {
  return render(
    <PlayerProfileCareerChart career={CAREER} language="en" onOpenTeam={onOpenTeam} t={t} />,
  );
}

describe("PlayerProfileCareerChart", () => {
  /**
   * Given the backend's career
   * When the chart is drawn
   * Then there is a bar for each season row, oldest first, each named by its season, club and
   *      figure, and the latest season is the one selected
   */
  it("draws a named bar for each season, the latest selected", () => {
    renderChart();

    const all = bars();
    expect(all).toHaveLength(4);
    expect(all[0]).toHaveAccessibleName(
      'playerProfile.careerChartBar {"season":"2023/24","club":"ash","value":6,"metric":"playerProfile.goals"}',
    );
    expect(all[3]).toHaveAttribute("aria-pressed", "true");
    expect(all[0]).toHaveAttribute("aria-pressed", "false");
  });

  /**
   * Given the chart on goals
   * When another season's bar is chosen
   * Then the readout follows it: the season, the club, and apps, goals and assists
   */
  it("shows the chosen season in the readout", () => {
    renderChart();

    fireEvent.click(bars()[1]);

    expect(bars()[1]).toHaveAttribute("aria-pressed", "true");
    expect(bars()[3]).toHaveAttribute("aria-pressed", "false");
    const view = readout();
    expect(view.getByText("2023/24")).toBeInTheDocument();
    expect(view.getByText("west")).toBeInTheDocument();
    expect(view.getByText("14")).toBeInTheDocument();
    expect(view.getByText("8")).toBeInTheDocument();
    expect(view.getByText("3")).toBeInTheDocument();
  });

  /**
   * Given the chart on goals, with the 2023/24 loan season chosen
   * When Assists, then Appearances, are chosen
   * Then the bars take that metric's figures and the chosen season stays chosen
   */
  it("switches the metric and keeps the chosen season", () => {
    renderChart();
    fireEvent.click(bars()[1]);

    fireEvent.click(screen.getByRole("radio", { name: "playerProfile.assists" }));

    expect(bars()[1]).toHaveAccessibleName(
      'playerProfile.careerChartBar {"season":"2023/24","club":"west","value":3,"metric":"playerProfile.assists"}',
    );
    expect(bars()[1]).toHaveAttribute("aria-pressed", "true");
    fireEvent.click(screen.getByRole("radio", { name: "playerProfile.apps" }));
    expect(bars()[1]).toHaveAccessibleName(
      'playerProfile.careerChartBar {"season":"2023/24","club":"west","value":14,"metric":"playerProfile.apps"}',
    );
  });

  /**
   * Given goals of 6, 8, 16 and 15
   * When the chart is drawn
   * Then each bar is drawn at its share of the tallest bar's height
   */
  it("draws each bar at its share of the tallest", () => {
    renderChart();

    const heights = bars().map((bar) =>
      Number.parseFloat(within(bar).getByRole("presentation").style.height),
    );
    expect(heights).toEqual([37.5, 50, 100, 93.75]);
  });

  /**
   * Given a transfer that began the 2024/25 stint, and a season with no move
   * When each is chosen
   * Then the readout says the move (its date, what, the fee) or that there was none this season
   */
  it("says the move of the chosen season, or that there was none", () => {
    renderChart();

    fireEvent.click(bars()[2]);
    expect(readout().getByText(new RegExp(formatProfileDay("2024-07-01", "en")))).toHaveTextContent(
      'transferFrom {"club":"Ashford"}',
    );
    expect(readout().getByText(/€9,500,000/)).toBeInTheDocument();

    fireEvent.click(bars()[3]);
    expect(readout().getByText("playerProfile.careerChartNoMove")).toBeInTheDocument();
  });

  /**
   * Given a club played for twice and a loan at another
   * When the chart is drawn
   * Then the legend names each club once, with the loan marked as one, in the order they appear
   */
  it("lists each club once in the legend, a loan as a loan", () => {
    renderChart();

    const legend = within(screen.getByRole("list", { name: "playerProfile.careerChartClubs" }));
    expect(legend.getAllByRole("listitem").map((item) => item.textContent)).toEqual([
      "ash",
      'playerProfile.careerChartLoanClub {"club":"west"}',
      "north",
    ]);
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
});
