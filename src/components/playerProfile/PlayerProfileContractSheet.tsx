import type { PlayerData } from "../../store/gameStore";
import type { ContractSheetData } from "./useContractSheetData";

type TranslateFn = (key: string, options?: Record<string, string | number>) => string;

export interface PlayerProfileContractSheetProps {
  player: PlayerData;
  /** The game's date, for the years left on his contract. */
  currentDate: string;
  /** The manager holds his contract (his own player, or the parent club's player on loan). */
  isOwnContract: boolean;
  isFreeAgent: boolean;
  hasLetExpireIntent: boolean;
  /** His weekly wage when the club may see it exactly; `null` for every other state. */
  exactWeeklyWage: number | null;
  data: ContractSheetData;
  actionSubmitting: boolean;
  onOpenRenewal: () => void;
  onMarkLetExpire: () => void;
  onClearLetExpire: () => void;
  onOpenTermination: () => void;
  onOpenFreeAgentContract: () => void;
  t: TranslateFn;
}

/** The decision sheet: his contract, the club's wage bill, and what each decision does. Scaffold. */
export default function PlayerProfileContractSheet(_props: PlayerProfileContractSheetProps) {
  return null;
}
