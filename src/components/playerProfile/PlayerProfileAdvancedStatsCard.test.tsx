import { render, screen, within } from "@testing-library/react";
import { afterEach, beforeAll, describe, expect, it } from "vitest";

import i18n, { changeAppLanguage, i18nReady } from "../../i18n";
import type { PlayerAdvancedStatsSummary } from "./PlayerProfile.helpers";
import PlayerProfileAdvancedStatsCard from "./PlayerProfileAdvancedStatsCard";

const SUMMARY: PlayerAdvancedStatsSummary = {
  percentileEligible: true,
  peerGroup: null,
  seasonRatingRated: false,
  metrics: {
    shots: { total: 20, per90: 4, percentile: 2 },
    shotsOnTarget: { total: 10, per90: 2, percentile: null },
    passes: { completed: 80, attempted: 100, accuracy: 80, percentile: null },
    tacklesWon: { total: 9, per90: 1.8, percentile: null },
    interceptions: { total: 6, per90: 1.2, percentile: null },
    foulsCommitted: { total: 5, per90: 1, percentile: null },
  },
};

describe("PlayerProfileAdvancedStatsCard percentiles", () => {
  beforeAll(async () => {
    await i18nReady;
  });

  afterEach(async () => {
    await changeAppLanguage("en");
  });

  /**
   * Given the game language is Portuguese and a shots percentile of 2
   * When the card renders
   * Then it reads "2.º", not "2nd"
   */
  it("writes the percentile as an ordinal in the game language", async () => {
    await changeAppLanguage("pt");

    render(<PlayerProfileAdvancedStatsCard summary={SUMMARY} t={(key) => key} />);

    expect(screen.getByText("2.º")).toBeInTheDocument();
    expect(screen.queryByText("2nd")).not.toBeInTheDocument();
  });

  /**
   * Given the game language is English
   * When the same card renders
   * Then it reads "2nd"
   */
  it("keeps English suffixes in English", async () => {
    await changeAppLanguage("en");

    render(<PlayerProfileAdvancedStatsCard summary={SUMMARY} t={(key) => key} />);

    expect(screen.getByText("2nd")).toBeInTheDocument();
  });

  /**
   * Given the backend names a striker's group as the second division
   * When the card renders
   * Then the screen says which group the percentiles compare with
   */
  it("states the league and position the percentiles are measured within", () => {
    render(
      <PlayerProfileAdvancedStatsCard
        summary={{
          ...SUMMARY,
          peerGroup: {
            competitionId: "div-2",
            competitionName: "Second Division",
            competitionNameKey: null,
            countryId: null,
            position: "Striker",
          },
        }}
        t={i18n.t}
      />,
    );

    expect(screen.getByText("Compared with: Striker in Second Division")).toBeInTheDocument();
  });

  /**
   * Given a German third-tier peer group with a localized competition key
   * When the card renders in German
   * Then its group label interpolates the tier instead of using the stored English name
   */
  it("localizes the peer group's division tier", async () => {
    await changeAppLanguage("de");

    render(
      <PlayerProfileAdvancedStatsCard
        summary={{
          ...SUMMARY,
          peerGroup: {
            competitionId: "de-tier-3",
            competitionName: "Germany Division 3",
            competitionNameKey: "tournaments.competitions.tierDivision",
            countryId: "DE",
            divisionTier: 3,
            position: "Striker",
          },
        }}
        t={i18n.t}
      />,
    );

    expect(screen.getByText(/Deutschland 3\. Liga/)).toBeInTheDocument();
    expect(screen.queryByText(/Germany Division 3/)).not.toBeInTheDocument();
  });

  /**
   * Given the backend gives no peer group (a player with no league)
   * When the card renders
   * Then no group line is shown
   */
  it("shows no group line without a peer group", () => {
    render(<PlayerProfileAdvancedStatsCard summary={SUMMARY} t={i18n.t} />);

    expect(screen.queryByText(/Compared with/)).not.toBeInTheDocument();
  });
});

// A translator that shows its options, so a figure and its unit are told apart from the key.
const tShowing = (key: string, options?: Record<string, unknown>) =>
  options ? `${key} ${JSON.stringify(options)}` : key;

describe("PlayerProfileAdvancedStatsCard rows", () => {
  /**
   * Given a season's advanced stats
   * When the card renders
   * Then each of the six stats is one row of a list, led by its name, with its total and, under the
   *      name, its rate in words (per 90 minutes, or the pass accuracy)
   */
  it("draws each stat as a row with its total and its rate", () => {
    render(<PlayerProfileAdvancedStatsCard summary={SUMMARY} t={tShowing} />);

    const rows = screen.getAllByRole("listitem");
    expect(rows).toHaveLength(6);
    const shots = within(rows[0]);
    expect(shots.getByText("Shots")).toBeInTheDocument();
    expect(shots.getByText("20")).toBeInTheDocument();
    expect(shots.getByText('playerProfile.per90Value {"value":"4.00"}')).toBeInTheDocument();
    const passes = within(rows[2]);
    expect(passes.getByText("80 / 100")).toBeInTheDocument();
    expect(passes.getByText('playerProfile.accurateValue {"value":"80%"}')).toBeInTheDocument();
  });

  /**
   * Given a player with no minutes (no rate) and no attempted pass (no accuracy)
   * When the card renders
   * Then the rate says "-" for each, never "0.00 per 90" or "0% accurate"
   */
  it("shows a dash, not a made-up rate, when there is none", () => {
    render(
      <PlayerProfileAdvancedStatsCard
        summary={{
          ...SUMMARY,
          metrics: {
            ...SUMMARY.metrics,
            // A percentile is present so a dash in the row can only be the missing rate.
            shots: { total: 0, per90: null, percentile: 5 },
            passes: { completed: 0, attempted: 0, accuracy: null, percentile: 5 },
          },
        }}
        t={tShowing}
      />,
    );

    const rows = screen.getAllByRole("listitem");
    expect(within(rows[0]).queryByText(/per90Value/)).toBeNull();
    expect(within(rows[2]).queryByText(/accurateValue/)).toBeNull();
    expect(within(rows[0]).getByText("-")).toBeInTheDocument();
    expect(within(rows[2]).getByText("-")).toBeInTheDocument();
  });

  /**
   * Given a percentile of 2 for shots and none for the other stats
   * When the card renders
   * Then the row with a percentile shows its ordinal and the rows without show "-"
   */
  it("shows the backend's ordinal or a dash", () => {
    render(<PlayerProfileAdvancedStatsCard summary={SUMMARY} t={tShowing} />);

    const rows = screen.getAllByRole("listitem");
    expect(within(rows[0]).getByText("2nd")).toBeInTheDocument();
    expect(within(rows[1]).queryByText(/\d(st|nd|rd|th)$/)).toBeNull();
    expect(within(rows[1]).getByText("-")).toBeInTheDocument();
  });

  /**
   * Given a percentile of 2 for shots (the backend's number, where a high one always means he does
   *       well) and none for the other five stats
   * When the card renders
   * Then shots has a named bar at exactly 2 (the number is not recomputed or rescaled), no other
   *      row has a bar, and the bar is filled in one neutral colour: it is not painted good or bad
   */
  it("draws a named neutral bar at the backend's percentile only for a row that has one", () => {
    render(<PlayerProfileAdvancedStatsCard summary={SUMMARY} t={tShowing} />);

    const bars = screen.getAllByRole("progressbar");
    expect(bars).toHaveLength(1);
    const rows = screen.getAllByRole("listitem");
    const bar = within(rows[0]).getByRole("progressbar", {
      name: 'playerProfile.percentileBar {"stat":"Shots"}',
    });
    expect(bar).toHaveAttribute("aria-valuenow", "2");
    const fill = bar.firstElementChild?.className ?? "";
    expect(fill).toContain("bg-fg-subtle");
    expect(fill).not.toMatch(/positive|warning|danger|success|tone-/);
  });

  /**
   * Given a stat whose percentile the backend gave as 100 and one as 0
   * When the card renders
   * Then each bar carries that number as it is
   */
  it("passes the extremes through as they are", () => {
    render(
      <PlayerProfileAdvancedStatsCard
        summary={{
          ...SUMMARY,
          metrics: {
            ...SUMMARY.metrics,
            shots: { total: 20, per90: 4, percentile: 100 },
            shotsOnTarget: { total: 10, per90: 2, percentile: 0 },
          },
        }}
        t={tShowing}
      />,
    );

    const values = screen
      .getAllByRole("progressbar")
      .map((bar) => bar.getAttribute("aria-valuenow"));
    expect(values).toEqual(["100", "0"]);
  });

  /**
   * Given a season whose six stats each have their own total, rate and percentile (no two alike)
   * When the card renders
   * Then each row shows exactly its own stat's total, rate, ordinal and bar value, in the card's
   *      order: a row wired to another stat, or a percentile turned around (the backend's "a high
   *      one always means he does well" is shown as given, fouls included), would show a figure
   *      that is not its own
   */
  it.each([
    {
      row: 0,
      label: "Shots",
      total: "31",
      rate: '{"value":"3.10"}',
      kind: "per90",
      ordinal: "61st",
      bar: "61",
    },
    {
      row: 1,
      label: "Shots On Target",
      total: "12",
      rate: '{"value":"1.20"}',
      kind: "per90",
      ordinal: "62nd",
      bar: "62",
    },
    {
      row: 2,
      label: "Passes",
      total: "41 / 52",
      rate: '{"value":"78.8%"}',
      kind: "accurate",
      ordinal: "63rd",
      bar: "63",
    },
    {
      row: 3,
      label: "Tackles Won",
      total: "23",
      rate: '{"value":"2.30"}',
      kind: "per90",
      ordinal: "64th",
      bar: "64",
    },
    {
      row: 4,
      label: "Interceptions",
      total: "14",
      rate: '{"value":"1.40"}',
      kind: "per90",
      ordinal: "65th",
      bar: "65",
    },
    {
      row: 5,
      label: "Fouls Committed",
      total: "17",
      rate: '{"value":"1.70"}',
      kind: "per90",
      ordinal: "66th",
      bar: "66",
    },
  ])("wires the $label row to its own stat", ({ row, label, total, rate, kind, ordinal, bar }) => {
    render(
      <PlayerProfileAdvancedStatsCard
        summary={{
          ...SUMMARY,
          metrics: {
            shots: { total: 31, per90: 3.1, percentile: 61 },
            shotsOnTarget: { total: 12, per90: 1.2, percentile: 62 },
            passes: { completed: 41, attempted: 52, accuracy: 78.8, percentile: 63 },
            tacklesWon: { total: 23, per90: 2.3, percentile: 64 },
            interceptions: { total: 14, per90: 1.4, percentile: 65 },
            foulsCommitted: { total: 17, per90: 1.7, percentile: 66 },
          },
        }}
        t={tShowing}
      />,
    );

    const view = within(screen.getAllByRole("listitem")[row]);
    expect(view.getByText(label)).toBeInTheDocument();
    expect(view.getByText(total)).toBeInTheDocument();
    expect(
      view.getByText(`playerProfile.${kind === "per90" ? "per90Value" : "accurateValue"} ${rate}`),
    ).toBeInTheDocument();
    expect(view.getByText(ordinal)).toBeInTheDocument();
    expect(view.getByRole("progressbar")).toHaveAttribute("aria-valuenow", bar);
  });
});
