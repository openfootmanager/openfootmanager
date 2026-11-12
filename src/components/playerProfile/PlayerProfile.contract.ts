import type { PlayerData, StaffData, TeamData } from "../../store/gameStore";

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

/** What the club-wide previews read from the game. */
export interface ClubStateInputs {
  players: PlayerData[];
  staff: StaffData[];
  teams: Pick<TeamData, "id" | "wage_budget">[];
}

/**
 * What the club-wide previews depend on, as one string. Scaffold: it still reads only the playing
 * squad; the real key replaces it.
 */
export function clubStateKey(game: ClubStateInputs, teamId: string | null): string {
  if (!teamId) return "";
  return game.players
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
