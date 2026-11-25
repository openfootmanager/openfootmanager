import type { TeamDef } from "./types";

/**
 * Club id → display name, so a list row can name a player's or staff member's
 * club without scanning the team array. The lists resolved it with a `find`
 * inside the row map, which is O(rows × teams) on every render — noticeable
 * once a package holds a full pyramid rather than a sample league.
 *
 * Records with no id are skipped: a half-typed team would otherwise claim the
 * empty club id that "no club" uses.
 */
export function buildTeamNameMap(teams: TeamDef[] | undefined): Map<string, string> {
  const names = new Map<string, string>();
  teams?.forEach((team) => {
    if (team.id) {
      names.set(team.id, team.name);
    }
  });
  return names;
}
