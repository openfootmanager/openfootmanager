import type { PlayerMovementEntry, TeamData } from "../../store/gameStore";

/**
 * The movement ledger with every club named. A ledger entry made when a career opens, or when an
 * old save is migrated, carries a club's id and no name; the name is the game's own. Scaffold.
 */
export function withClubNames(
  movements: PlayerMovementEntry[],
  _teams: Pick<TeamData, "id" | "name">[],
): PlayerMovementEntry[] {
  return movements;
}
