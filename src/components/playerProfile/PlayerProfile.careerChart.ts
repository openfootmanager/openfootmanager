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
  /** Its height as a share of the tallest bar, 0 to 100. */
  heightPct: number;
  /** Which club band it sits on: the order a club (or its loan) first appears in. */
  bandIndex: number;
  /** The move that began this stint, when the career lists one right below it. */
  move: CareerMoveLine | null;
  /** The moves between the previous season and this one. Scaffold: empty. */
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
  bands: ChartBand[];
}

const metricValue = (row: CareerSeasonRow, metric: ChartMetric): number =>
  metric === "goals" ? row.goals : metric === "assists" ? row.assists : row.appearances;

/**
 * The backend's career as bars for a chart: a bar for each season row, oldest first, valued by the
 * chosen metric, each carrying the move that began its stint (the line the backend puts right below
 * it) and the band of its club (a loan spell is a band of its own). Heights are a share of the
 * tallest bar, a drawing scale and nothing the game decides.
 */
export function buildCareerChart(career: PlayerCareer, metric: ChartMetric): CareerChart {
  const lines = career.by_season;
  const bandOf = new Map<string, ChartBand>();
  const bars: ChartBar[] = [];

  for (let index = lines.length - 1; index >= 0; index -= 1) {
    const line = lines[index];
    if (line.kind !== "season") continue;
    const below = lines[index + 1];
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
      heightPct: 0,
      bandIndex: bandOf.get(bandKey)?.bandIndex ?? 0,
      move: below?.kind === "move" ? below : null,
      movesBefore: [],
    });
  }

  const tallest = Math.max(0, ...bars.map((bar) => bar.value));
  return {
    bars: bars.map((bar) => ({ ...bar, heightPct: tallest > 0 ? (bar.value / tallest) * 100 : 0 })),
    bands: [...bandOf.values()],
  };
}
