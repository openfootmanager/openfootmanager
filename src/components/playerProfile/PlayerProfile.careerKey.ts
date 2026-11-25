import type { PlayerData } from "../../store/gameStore";

/**
 * What the profile's career answer depends on, as one string: the career the game holds (its
 * token, so a saved game loaded over the same ids and dates is another career), the day, his club,
 * the number of his closed seasons and of his moves, and his season stats (a fixture played over
 * MCP changes them without the day moving, and the season in progress is built from them).
 */
export function careerRevision(
  careerToken: number,
  player: PlayerData,
  currentDate: string,
): string {
  const { appearances, goals, assists, minutes_played } = player.stats;
  return [
    careerToken,
    currentDate,
    player.team_id ?? "",
    player.career.length,
    (player.movement_history ?? []).length,
    appearances,
    goals,
    assists,
    minutes_played,
  ].join("|");
}
