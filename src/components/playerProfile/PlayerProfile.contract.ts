import type { PlayerData } from "../../store/gameStore";

/** Why a contract cannot be ended now, when it cannot. */
export type TerminationBlock = "no_contract" | "on_loan";

/**
 * What stops the manager releasing this player outright, if anything. Scaffold: the real rule
 * replaces it.
 */
export function terminationBlockedBy(_player: PlayerData): TerminationBlock | null {
  return null;
}
