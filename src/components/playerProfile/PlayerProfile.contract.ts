import type { PlayerData } from "../../store/gameStore";

/** Why a contract cannot be ended now, when it cannot. */
export type TerminationBlock = "no_contract" | "on_loan";

/**
 * What stops the manager releasing this player outright, if anything. Both follow the backend's own
 * refusals (`playerHasNoActiveContract`, `playerOnActiveLoan`), so the sheet and the actions menu
 * say so before a click instead of after it. The missing contract is named first.
 */
export function terminationBlockedBy(player: PlayerData): TerminationBlock | null {
  if (!player.contract_end) return "no_contract";
  if (player.active_loan) return "on_loan";
  return null;
}
