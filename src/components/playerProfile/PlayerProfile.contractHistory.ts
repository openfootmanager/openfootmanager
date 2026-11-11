import type { PlayerMovementEntry } from "../../store/gameStore";
import type { ContractSource, ReleaseReason } from "../../store/types";

/** One contract the player has had, as the Contract tab lists it. */
export interface ContractHistoryRow {
  /** Stable across renders: the ledger entry's date, kind and place. */
  key: string;
  /** The club the contract is with; empty when the ledger names none. */
  clubName: string;
  /** `null` is an honest unknown (a save from before starts were recorded), never a date. */
  start: string | null;
  end: string | null;
  /** Per week. */
  weeklyWage: number;
  source: ContractSource;
  /** The release that closed it, when one did and said why; `null` for a contract that was renewed or is in force. */
  endedBy: ReleaseReason | null;
  /** The day a later contract took over from it before its own end (a renewal signed early). */
  replacedOn: string | null;
  /** The contract he is on now: the last one, not closed by a release, and not yet past its end. */
  inForce: boolean;
}

/**
 * The contracts a player has had, newest first, from his movement ledger. Scaffold: the real
 * builder replaces it.
 */
export function buildContractHistory(
  _movements: PlayerMovementEntry[],
  _today: string,
): ContractHistoryRow[] {
  return [];
}
