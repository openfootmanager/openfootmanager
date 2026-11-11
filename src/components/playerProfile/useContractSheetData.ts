import type { PlayerData } from "../../store/gameStore";
import type { ContractTerminationPreviewData } from "../../services/contractService";
import type { RenewalProjection } from "./PlayerProfile.renewal";

/** An answer the Contract tab loads, and where it stands. */
export type SheetAnswer<T> =
  | { status: "idle" }
  | { status: "loading" }
  | { status: "answered"; value: T }
  | { status: "failed" };

export interface ContractSheetData {
  /** What releasing him would cost and leave: his own player with a contract, not on loan. */
  termination: SheetAnswer<ContractTerminationPreviewData>;
  /** The club's wage bill and budget, with his current wage in it: his own player's. */
  projection: SheetAnswer<RenewalProjection>;
  /** Asks again for whatever failed. */
  retry: () => void;
}

interface UseContractSheetDataArgs {
  player: PlayerData;
  /** The Contract tab is the one open: nothing is asked for before. */
  active: boolean;
  /** The manager holds his contract: only then is there anything to ask. */
  ownsContract: boolean;
  /** Changes when the game changes under the sheet (an action taken, the day moved on). */
  refreshKey: string;
}

/**
 * The two answers the decision sheet shows before a click. Scaffold: the real hook replaces it.
 */
export function useContractSheetData(_args: UseContractSheetDataArgs): ContractSheetData {
  return { termination: { status: "idle" }, projection: { status: "idle" }, retry: () => {} };
}
