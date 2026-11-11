import type { PlayerData } from "../../store/gameStore";

/** What the archive kept of how a player's career ended: the day, and the club he left from. */
export interface Retirement {
  /** The retirement movement's date; null when the archive holds no such movement. */
  retiredOn: string | null;
  /** The club he left from; null when neither the movement nor his career names one. */
  lastClubName: string | null;
}

/**
 * How a retired player's career ended, read from his retirement movement and, for the club, his
 * latest season; null for a player who has not retired. A person archived without either still
 * has a `Retirement`, with nothing in it: he has retired, and the page says so without inventing
 * a date or a club.
 */
export function retirementOf(player: PlayerData): Retirement | null {
  if (!player.retired) return null;
  const movement = [...(player.movement_history ?? [])]
    .reverse()
    .find((entry) => entry.kind === "retired");
  const latestSeason = player.career?.length ? player.career[player.career.length - 1] : null;
  return {
    retiredOn: movement?.date ?? null,
    lastClubName: movement?.from_team_name ?? latestSeason?.team_name ?? null,
  };
}
