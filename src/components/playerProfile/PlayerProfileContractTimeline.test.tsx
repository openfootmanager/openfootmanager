import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import type { ContractHistoryRow } from "./PlayerProfile.contractHistory";
import { formatProfileDay } from "./PlayerProfile.helpers";
import PlayerProfileContractTimeline from "./PlayerProfileContractTimeline";

const t = (key: string, options?: Record<string, string | number>) =>
  options ? `${key} ${JSON.stringify(options)}` : key;

const row = (patch: Partial<ContractHistoryRow>): ContractHistoryRow => ({
  key: "r",
  clubName: "North FC",
  clubId: "north",
  fromClubName: null,
  start: "2029-07-01",
  end: "2031-06-30",
  weeklyWage: 7_000,
  source: "transfer",
  endedBy: null,
  endedOn: null,
  replacedOn: null,
  inForce: false,
  ...patch,
});

const ROWS = [
  row({ key: "b", start: "2031-07-01", end: "2033-06-30", weeklyWage: 10_000, inForce: true }),
  row({ key: "a" }),
];

function renderTimeline(rows: ContractHistoryRow[], showsWages = true) {
  return render(
    <PlayerProfileContractTimeline
      rows={rows}
      showsWages={showsWages}
      today="2031-07-10"
      language="en"
      t={t}
    />,
  );
}

describe("PlayerProfileContractTimeline", () => {
  /**
   * Given two contracts he has had, the latest in force
   * When the card is drawn for his own player
   * Then it is one named picture of the contracts over time, with a bar per contract, each titled
   *      with its club and span and carrying its weekly wage, and a marker for today
   */
  it("draws a bar for each contract with its wage, and a marker for today", () => {
    renderTimeline(ROWS);

    expect(screen.getByText("playerProfile.contractSheet.overTime")).toBeInTheDocument();
    expect(
      screen.getByRole("img", { name: "playerProfile.contractSheet.overTimeLabel" }),
    ).toBeInTheDocument();
    expect(screen.getAllByTitle(/North FC/)).toHaveLength(2);
    expect(screen.getByText("€10K")).toBeInTheDocument();
    expect(screen.getByText("€7K")).toBeInTheDocument();
    expect(screen.getByText("playerProfile.contractSheet.today")).toBeInTheDocument();
    expect(screen.getByText("playerProfile.contractSheet.overTimeNote")).toBeInTheDocument();
  });

  /**
   * Given a contract whose end has not come yet, and one that has
   * When the card is drawn
   * Then the planned one has a dashed edge and the other does not
   */
  it("draws a planned end with a dashed edge", () => {
    renderTimeline(ROWS);

    // Oldest first: the contract that ran its course, then the one in force.
    const [over, planned] = screen.getAllByTitle(/North FC/);
    expect(planned.className).toContain("border-dashed");
    expect(over.className).not.toContain("border-dashed");
  });

  /**
   * Given a rival's contracts, whose wages the club may not see
   * When the card is drawn
   * Then the bars stay (their dates are public) and no wage is on any of them
   */
  it("leaves the wages off when the club may not see them", () => {
    renderTimeline(ROWS, false);

    expect(screen.getAllByTitle(/North FC/)).toHaveLength(2);
    expect(screen.queryByText(/€/)).toBeNull();
    expect(screen.getByText("playerProfile.contractSheet.overTimeNoteNoWage")).toBeInTheDocument();
  });

  /**
   * Given a contract from a save made before starts were recorded
   * When the card is drawn
   * Then its bar is open at the start (no left border), so it does not claim a date it lacks
   */
  it("leaves the start of an unknown-start contract open", () => {
    renderTimeline([row({ key: "u", start: null, source: "legacy_migrated", inForce: true })]);

    expect(screen.getByTitle(/North FC/).className).toContain("border-l-0");
  });

  /** Given no contract on record, then the card is not drawn. */
  it("is not drawn for a player with no contract on record", () => {
    const { container } = renderTimeline([]);

    expect(container).toBeEmptyDOMElement();
  });

  /**
   * Given a contract scheduled to 30 June 2033 that was terminated on 10 January 2031
   * When the card is drawn
   * Then its tooltip and the chart's accessible description give the day it really ended, never the
   *      scheduled end the bar does not reach
   */
  it("says the day a contract really ended, not the day it was scheduled to", () => {
    renderTimeline([
      row({ end: "2033-06-30", endedBy: "terminated", endedOn: "2031-01-10", weeklyWage: 7_000 }),
    ]);

    const actual = formatProfileDay("2031-01-10", "en");
    const scheduled = formatProfileDay("2033-06-30", "en");
    const bar = screen.getByTitle(/North FC/);
    expect(bar.title).toContain(actual);
    expect(bar.title).not.toContain(scheduled);
    const chart = screen.getByRole("img", { name: "playerProfile.contractSheet.overTimeLabel" });
    expect(chart).toHaveAccessibleDescription(expect.stringContaining(actual));
    expect(chart).not.toHaveAccessibleDescription(expect.stringContaining(scheduled));
  });

  /**
   * Given a contract renewed early, so a later one took over on 15 January 2031 before its
   *       scheduled end of 30 June 2032
   * When the card is drawn
   * Then the older contract's description ends on the day the renewal took over
   */
  it("ends an early-renewed contract's description on the day the renewal took over", () => {
    renderTimeline([
      row({ key: "b", start: "2031-01-15", end: "2034-06-30", inForce: true }),
      row({ key: "a", end: "2032-06-30", replacedOn: "2031-01-15" }),
    ]);

    const [older] = screen.getAllByTitle(/North FC/);
    expect(older.title).toContain(formatProfileDay("2031-01-15", "en"));
    expect(older.title).not.toContain(formatProfileDay("2032-06-30", "en"));
  });

  /**
   * Given two contracts and today, on an axis of whole years from 2029 to the end of 2033
   * When the card is drawn
   * Then each bar is placed and sized by its days (left and width in percent of the axis) and the
   *      Today label sits at today's share of the axis, so removing the placement shows
   */
  it("places each bar and the Today label by their days on the axis", () => {
    renderTimeline(ROWS);

    const share = (from: string, to: string) =>
      ((Date.parse(to) - Date.parse(from)) /
        (Date.parse("2034-01-01") - Date.parse("2029-01-01"))) *
      100;
    const [older, newer] = screen.getAllByTitle(/North FC/);
    const left = (element: HTMLElement) => Number.parseFloat(element.style.left);
    const width = (element: HTMLElement) => Number.parseFloat(element.style.width);
    expect(left(older)).toBeCloseTo(share("2029-01-01", "2029-07-01"), 2);
    expect(width(older)).toBeCloseTo(share("2029-07-01", "2031-07-01"), 2);
    expect(left(newer)).toBeCloseTo(share("2029-01-01", "2031-07-01"), 2);
    expect(width(newer)).toBeCloseTo(share("2031-07-01", "2033-07-01"), 2);
    const today = screen.getByText("playerProfile.contractSheet.today");
    expect(left(today)).toBeCloseTo(share("2029-01-01", "2031-07-10"), 2);
  });

  /**
   * Given a contract that has ended and one in force, for a club that may not see wages (so a bar
   *       carries no text, and its outline is the only signal)
   * When the card is drawn
   * Then the historical bar is outlined in the subtle foreground token (about 5:1 on the panel in
   *      both themes), not the faint line token (about 1.5:1)
   */
  it("outlines a historical bar in a token that reaches 3:1 on the track", () => {
    renderTimeline(ROWS, false);

    const [older] = screen.getAllByTitle(/North FC/);
    expect(older.className).toContain("border-fg-subtle");
    expect(older.className).not.toContain("border-line");
  });
});
