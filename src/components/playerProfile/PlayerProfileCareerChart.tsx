import type { PlayerCareer } from "../../store/generated/PlayerCareer";

type TranslateFn = (key: string, options?: Record<string, string | number>) => string;

interface PlayerProfileCareerChartProps {
  career: PlayerCareer;
  language: string;
  /** Opens a club by its id; without it the club is plain text. */
  onOpenTeam?: (teamId: string) => void;
  t: TranslateFn;
}

/** The career as a bar for each season. Scaffold: the real chart replaces it. */
export default function PlayerProfileCareerChart(_props: PlayerProfileCareerChartProps) {
  return null;
}
