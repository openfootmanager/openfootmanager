import type { PlayerMovementEntry, TeamData } from "../../store/gameStore";

/**
 * The movement ledger with every club named. A ledger entry made when a career opens, or when an
 * old save is migrated, carries a club's id and no name; the name is the game's own, taken from its
 * teams. A name the ledger recorded stands (it is what the ledger says), and a club the game no
 * longer has is left unnamed rather than named by its id.
 */
export function withClubNames(
  movements: PlayerMovementEntry[],
  teams: Pick<TeamData, "id" | "name">[],
): PlayerMovementEntry[] {
  const names = new Map(teams.map((team) => [team.id, team.name]));
  const named = (recorded: string | null | undefined, id: string | null | undefined) =>
    recorded ?? (id ? names.get(id) : undefined) ?? recorded;
  return movements.map((entry) => ({
    ...entry,
    from_team_name: named(entry.from_team_name, entry.from_team_id),
    to_team_name: named(entry.to_team_name, entry.to_team_id),
  }));
}
