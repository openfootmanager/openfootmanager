import type { CareerMoveLine } from "../../store/generated/CareerMoveLine";
import type { CareerSeasonRow } from "../../store/generated/CareerSeasonRow";
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
  /** Which club band it sits on: the order a club (or its loan) first appears in. */
  bandIndex: number;
  /**
   * The moves the career lists between the previous season and this one, oldest first. Where they
   * sit in the career is all this says: a move is not claimed to have begun this stint (a loan to a
   * club he never played for sits before the later stint at his own).
   */
  movesBefore: CareerMoveLine[];
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
  /** The moves the career lists after its last season row (a release, a retirement). Scaffold: empty. */
  trailingMoves: CareerMoveLine[];
  bands: ChartBand[];
}

const metricValue = (row: CareerSeasonRow, metric: ChartMetric): number =>
  metric === "goals" ? row.goals : metric === "assists" ? row.assists : row.appearances;

/**
 * The backend's career as bars for a chart: a bar for each season row, oldest first, valued by the
 * chosen metric, each carrying the moves listed before it in the career and the band of its club (a
 * loan spell is a band of its own). Nothing is judged or toned here.
 */
export function buildCareerChart(career: PlayerCareer, metric: ChartMetric): CareerChart {
  const lines = career.by_season;
  const bandOf = new Map<string, ChartBand>();
  const bars: ChartBar[] = [];

  for (let index = lines.length - 1; index >= 0; index -= 1) {
    const line = lines[index];
    if (line.kind !== "season") continue;
    const movesBefore: CareerMoveLine[] = [];
    for (let below = index + 1; below < lines.length; below += 1) {
      const next = lines[below];
      if (next.kind !== "move") break;
      movesBefore.unshift(next);
    }
    const club = line.club_id || `name:${line.club_name}`;
    const bandKey = `${club}|${line.on_loan}`;
    if (!bandOf.has(bandKey)) {
      bandOf.set(bandKey, {
        key: bandKey,
        bandIndex: bandOf.size,
        clubId: line.club_id,
        clubName: line.club_name,
        onLoan: line.on_loan,
      });
    }
    bars.push({
      key: line.key,
      season: line.season,
      clubId: line.club_id,
      clubName: line.club_name,
      onLoan: line.on_loan,
      inProgress: line.in_progress,
      appearances: line.appearances,
      goals: line.goals,
      assists: line.assists,
      value: metricValue(line, metric),
      bandIndex: bandOf.get(bandKey)?.bandIndex ?? 0,
      movesBefore,
    });
  }

  return { bars, bands: [...bandOf.values()], trailingMoves: [] };
}
