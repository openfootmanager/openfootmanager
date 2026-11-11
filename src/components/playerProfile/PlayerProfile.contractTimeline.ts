import type { ContractHistoryRow } from "./PlayerProfile.contractHistory";

/** One contract as a bar on the timeline, placed as a share of the axis. */
export interface TimelineBar {
  key: string;
  clubName: string;
  weeklyWage: number;
  /** Where it starts and how wide it is, in percent of the axis. */
  leftPct: number;
  widthPct: number;
  /** An unknown start: the bar begins where the one before ended (or at the axis), open-edged. */
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

/**
 * The contracts he has had as bars on a time axis of whole years. Scaffold: the real model
 * replaces it.
 */
export function buildContractTimeline(
  _rows: ContractHistoryRow[],
  _today: string,
): ContractTimeline | null {
  return null;
}
