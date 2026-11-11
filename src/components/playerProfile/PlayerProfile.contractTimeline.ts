import type { ContractHistoryRow } from "./PlayerProfile.contractHistory";

/** One contract as a bar on the timeline, placed as a share of the axis. */
export interface TimelineBar {
  key: string;
  clubName: string;
  weeklyWage: number;
  /** Where it starts and how wide it is, in percent of the axis. */
  leftPct: number;
  widthPct: number;
  /** An unknown start: the bar begins where the one before ended (or at the axis), never at a made-up date. */
  startKnown: boolean;
  /** Its end is a plan that has not come yet: it runs on past today. */
  planned: boolean;
  inForce: boolean;
}

export interface ContractTimeline {
  bars: TimelineBar[];
  /** Where today falls, in percent of the axis. */
  todayPct: number;
  /** A tick for each year the axis spans. */
  ticks: { year: number; leftPct: number }[];
}

const DAY_MS = 86_400_000;

const dayMs = (day: string) => Date.parse(day);
const yearStartMs = (year: number) => Date.UTC(year, 0, 1);
const yearOf = (ms: number) => new Date(ms).getUTCFullYear();

/** A contract's own end is the last day it runs: the bar reaches the end of that day. */
function scheduledEndMs(end: string): number {
  return dayMs(end) + DAY_MS;
}

/** The day each contract stops being his, oldest first; a release or a renewal cuts a plan short. */
function effectiveEnd(row: ContractHistoryRow, todayMs: number): number {
  if (row.endedOn) return dayMs(row.endedOn);
  if (row.replacedOn) return dayMs(row.replacedOn);
  return row.end ? scheduledEndMs(row.end) : todayMs;
}

/**
 * The contracts he has had as bars on a time axis of whole years, oldest first.
 *
 * A bar's end is when the contract really stopped: the day he was released or retired, the day an
 * early renewal took over, else its scheduled last day. One whose end has not come yet is planned.
 * A start that was never recorded is not guessed: the bar begins where the one before it ended
 * (the axis edge for the first) and says so. Returns null when there is nothing to draw.
 */
export function buildContractTimeline(
  rows: ContractHistoryRow[],
  today: string,
): ContractTimeline | null {
  if (rows.length === 0) return null;
  const todayMs = dayMs(today);
  const oldestFirst = [...rows].reverse();

  const ends = oldestFirst.map((row) => effectiveEnd(row, todayMs));
  const starts = oldestFirst.map((row) => (row.start ? dayMs(row.start) : null));
  const knownDays = [
    todayMs,
    ...ends.map((end) => end - 1),
    ...starts.filter((start): start is number => start !== null),
  ];
  const firstYear = yearOf(Math.min(...knownDays));
  const lastYear = yearOf(Math.max(...knownDays));
  const axisStart = yearStartMs(firstYear);
  const total = yearStartMs(lastYear + 1) - axisStart;
  const pct = (ms: number) => ((ms - axisStart) / total) * 100;

  let previousEnd = axisStart;
  const bars = oldestFirst.map((row, index): TimelineBar => {
    const startMs = starts[index] ?? previousEnd;
    const endMs = Math.max(ends[index], startMs);
    previousEnd = endMs;
    return {
      key: row.key,
      clubName: row.clubName,
      weeklyWage: row.weeklyWage,
      leftPct: pct(startMs),
      widthPct: pct(endMs) - pct(startMs),
      startKnown: starts[index] !== null,
      planned: !row.endedOn && !row.replacedOn && endMs > todayMs && row.inForce,
      inForce: row.inForce,
    };
  });

  const ticks = Array.from({ length: lastYear - firstYear + 1 }, (_, offset) => ({
    year: firstYear + offset,
    leftPct: pct(yearStartMs(firstYear + offset)),
  }));
  return { bars, todayPct: pct(todayMs), ticks };
}
