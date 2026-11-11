import type { ContractHistoryRow } from "./PlayerProfile.contractHistory";

type TranslateFn = (key: string, options?: Record<string, string | number>) => string;

interface PlayerProfileContractTimelineProps {
  rows: ContractHistoryRow[];
  /** Whether the club may see his wages: only its own player's contracts name one. */
  showsWages: boolean;
  /** The game's day, as `YYYY-MM-DD`. */
  today: string;
  language: string;
  t: TranslateFn;
}

/** The contracts he has had as bars over time. Scaffold: the real card replaces it. */
export default function PlayerProfileContractTimeline(_props: PlayerProfileContractTimelineProps) {
  return null;
}
