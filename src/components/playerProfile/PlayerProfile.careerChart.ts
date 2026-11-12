import type { CareerMoveLine } from "../../store/generated/CareerMoveLine";
import type { PlayerCareer } from "../../store/generated/PlayerCareer";

export type ChartMetric = "goals" | "assists" | "appearances";

/** One season at one club as a bar. */
export interface ChartBar {
  key: string;
  season: number;
  clubId: string;
  clubName: string;
  onLoan: boolean;
  inProgress: boolean;
  appearances: number;
  goals: number;
  assists: number;
  /** The figure the bar shows: the chosen metric's. */
  value: number;
  /** Its height as a share of the tallest bar, 0 to 100. */
  heightPct: number;
  /** Which club band it sits on: the order a club (or its loan) first appears in. */
  bandIndex: number;
  /** The move that began this stint, when the career lists one right below it. */
  move: CareerMoveLine | null;
}

/** A club (or a loan spell at one) the bands under the bars are coloured by. */
export interface ChartBand {
  key: string;
  bandIndex: number;
  clubId: string;
  clubName: string;
  onLoan: boolean;
}

export interface CareerChart {
  /** Oldest first, the order a chart reads. */
  bars: ChartBar[];
  bands: ChartBand[];
}

/** Scaffold: the real chart model replaces it. */
export function buildCareerChart(_career: PlayerCareer, _metric: ChartMetric): CareerChart {
  return { bars: [], bands: [] };
}
