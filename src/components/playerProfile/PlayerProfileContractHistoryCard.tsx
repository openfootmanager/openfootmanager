import type { ContractHistoryRow } from "./PlayerProfile.contractHistory";

type TranslateFn = (key: string, options?: Record<string, string | number>) => string;

interface PlayerProfileContractHistoryCardProps {
  rows: ContractHistoryRow[];
  /** Whether the club may see his wages: only its own player's contracts name one. */
  showsWages: boolean;
  t: TranslateFn;
}

/** The contracts he has had. Scaffold: the real card replaces it. */
export default function PlayerProfileContractHistoryCard(
  _props: PlayerProfileContractHistoryCardProps,
) {
  return null;
}
