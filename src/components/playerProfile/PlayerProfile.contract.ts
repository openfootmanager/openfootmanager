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

/**
 * What the club-wide previews depend on, as one string: the club's own squad (who is in it, their
 * wages, contract ends, injuries and loans). A signing, a release or a renewal made while the sheet
 * is open (by the manager elsewhere, or an agent over MCP) changes it, so the wage bill and the
 * squad-safety verdict are asked again; players at other clubs and the order of the list do not.
 */
export function squadStateKey(players: PlayerData[], teamId: string | null): string {
  if (!teamId) return "";
  return players
    .filter((player) => player.team_id === teamId)
    .map((player) =>
      [
        player.id,
        player.wage,
        player.contract_end ?? "",
        player.injury ? "injured" : "",
        player.active_loan ? "loan" : "",
      ].join(":"),
    )
    .sort()
    .join("|");
}
