import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import type { ContractHistoryRow } from "./PlayerProfile.contractHistory";
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
});
