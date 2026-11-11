import { describe, expect, it } from "vitest";

import type { ContractHistoryRow } from "./PlayerProfile.contractHistory";
import { buildContractTimeline } from "./PlayerProfile.contractTimeline";

const DAY = 86_400_000;
const days = (from: string, to: string) => (Date.parse(to) - Date.parse(from)) / DAY;

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

// Newest first, as the history lists them.
const TWO = [
  row({ key: "b", start: "2031-07-01", end: "2033-06-30", weeklyWage: 10_000, inForce: true }),
  row({ key: "a" }),
];
const TODAY = "2031-07-10";

describe("buildContractTimeline", () => {
  /**
   * Given a contract to 30 June 2031 and the one after it to 30 June 2033
   * When the timeline is built
   * Then the bars run oldest first on an axis of whole years (2029 to the end of 2033), each placed
   *      and sized by its days, so the second begins where the first ends
   */
  it("places each contract by its days on an axis of whole years", () => {
    const timeline = buildContractTimeline(TWO, TODAY);
    const total = days("2029-01-01", "2034-01-01");

    expect(timeline?.bars.map((bar) => bar.key)).toEqual(["a", "b"]);
    const [a, b] = timeline?.bars ?? [];
    expect(a.leftPct).toBeCloseTo((days("2029-01-01", "2029-07-01") / total) * 100, 3);
    expect(a.widthPct).toBeCloseTo((days("2029-07-01", "2031-07-01") / total) * 100, 3);
    expect(b.leftPct).toBeCloseTo(a.leftPct + a.widthPct, 3);
    expect(b.widthPct).toBeCloseTo((days("2031-07-01", "2033-07-01") / total) * 100, 3);
  });

  /**
   * Given the contract in force, which runs to 2033, and one that ran its course
   * When the timeline is built
   * Then only the contract in force is planned (its end has not come), the other is not
   */
  it("marks only a contract whose end has not come as planned", () => {
    const [a, b] = buildContractTimeline(TWO, TODAY)?.bars ?? [];

    expect(a.planned).toBe(false);
    expect(b).toMatchObject({ planned: true, inForce: true });
  });

  /**
   * Given a contract planned to 2033 that was terminated in January 2031
   * When the timeline is built
   * Then the bar ends on the day it was terminated, not on the plan, and it is not planned
   */
  it("ends a terminated contract on the day it was closed", () => {
    const timeline = buildContractTimeline(
      [row({ end: "2033-06-30", endedBy: "terminated", endedOn: "2031-01-10" })],
      TODAY,
    );
    const [bar] = timeline?.bars ?? [];
    const total = days("2029-01-01", "2034-01-01");

    expect(bar.widthPct).toBeCloseTo((days("2029-07-01", "2031-01-10") / total) * 100, 3);
    expect(bar.planned).toBe(false);
  });

  /**
   * Given a contract renewed early, so a later one took over before its scheduled end
   * When the timeline is built
   * Then the older bar ends on the day the renewal took over: the bars do not overlap
   */
  it("ends an early-renewed contract on the day its renewal took over", () => {
    const timeline = buildContractTimeline(
      [
        row({ key: "b", start: "2031-01-15", end: "2034-06-30", inForce: true }),
        row({ key: "a", end: "2032-06-30", replacedOn: "2031-01-15" }),
      ],
      TODAY,
    );
    const [a, b] = timeline?.bars ?? [];

    expect(a.leftPct + a.widthPct).toBeCloseTo(b.leftPct, 3);
  });

  /**
   * Given a contract from a save made before starts were recorded, after an earlier contract
   * When the timeline is built
   * Then its start is flagged unknown and the bar begins where the earlier one ended: never at a
   *      date it does not have
   */
  it("starts an unknown-start contract where the one before ended, and says so", () => {
    const timeline = buildContractTimeline(
      [
        row({ key: "b", start: null, end: "2033-06-30", source: "legacy_migrated", inForce: true }),
        row({ key: "a", end: "2031-06-30" }),
      ],
      TODAY,
    );
    const [a, b] = timeline?.bars ?? [];

    expect(b.startKnown).toBe(false);
    expect(b.leftPct).toBeCloseTo(a.leftPct + a.widthPct, 3);
    expect(a.startKnown).toBe(true);
  });

  /**
   * Given today, and a player whose contracts all ended before it
   * When the timeline is built
   * Then the axis still reaches today's year, and the marker falls where today does
   */
  it("keeps today on the axis even after every contract has ended", () => {
    const timeline = buildContractTimeline([row({})], "2033-03-01");
    const total = days("2029-01-01", "2034-01-01");

    expect(timeline?.todayPct).toBeCloseTo((days("2029-01-01", "2033-03-01") / total) * 100, 3);
  });

  /** Given contracts across 2029 to 2033, then there is a tick for each of those years. */
  it("ticks each year of the axis", () => {
    const timeline = buildContractTimeline(TWO, TODAY);

    expect(timeline?.ticks.map((tick) => tick.year)).toEqual([2029, 2030, 2031, 2032, 2033]);
    expect(timeline?.ticks[0].leftPct).toBe(0);
  });

  /** Given no contract, then there is no timeline. */
  it("is nothing for a player with no contract on record", () => {
    expect(buildContractTimeline([], TODAY)).toBeNull();
  });
});
