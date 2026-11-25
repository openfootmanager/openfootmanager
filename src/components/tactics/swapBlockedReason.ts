import type { PlayerData } from "../../store/gameStore";
import { isMatchDayEligible, isRuledOutOfMatchDay } from "../../lib/playerSquad";
import { applyLineupSwap, type SquadSection } from "../squad/SquadTab.helpers";

/** Why the compare dialog's Confirm swap is off, when there is a reason the manager can act on. */
export type SwapBlockedReason =
  /** The backend ruled this bench player out of the match day (injured, suspended, ...). */
  | { kind: "unavailable"; playerId: string }
  /** The backend has not yet said whether this bench player can be picked. */
  | { kind: "unchecked"; playerId: string }
  /** Both players are where the swap would leave them: the eleven stays as it is. */
  | { kind: "unchanged" };

export interface SwapSide {
  player: PlayerData;
  section: SquadSection;
}

export interface SwapVerdict {
  canConfirm: boolean;
  /** Null when the swap can be confirmed, and before a second player is picked. */
  reason: SwapBlockedReason | null;
}

/**
 * Whether swapping the two picked players can be confirmed, and if not, why. The rule is the one
 * the confirm handler holds itself to: only a starter may be swapped for an eligible bench player,
 * and a swap that leaves the eleven as it was is no swap.
 */
export function swapVerdict({
  selected,
  compare,
  startingXiIds,
}: {
  selected: SwapSide | null;
  compare: SwapSide | null;
  startingXiIds: string[];
}): SwapVerdict {
  if (!selected || !compare) return { canConfirm: false, reason: null };

  for (const side of [selected, compare]) {
    if (side.section !== "bench" || isMatchDayEligible(side.player)) continue;
    const kind = isRuledOutOfMatchDay(side.player) ? "unavailable" : "unchecked";
    return { canConfirm: false, reason: { kind, playerId: side.player.id } };
  }

  const nextXiIds = applyLineupSwap(
    startingXiIds,
    { id: selected.player.id, from: selected.section },
    compare.player.id,
    compare.section,
  );
  if (!nextXiIds || nextXiIds.join(",") === startingXiIds.join(",")) {
    return { canConfirm: false, reason: { kind: "unchanged" } };
  }
  return { canConfirm: true, reason: null };
}
