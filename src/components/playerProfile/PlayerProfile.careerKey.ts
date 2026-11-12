import type { PlayerData } from "../../store/gameStore";

/**
 * What the profile's career answer depends on, as one string. Scaffold: the key as it was (the day,
 * his club and the number of his seasons and moves); the real revision replaces it.
 */
export function careerRevision(
  _careerToken: number,
  player: PlayerData,
  currentDate: string,
): string {
  return [
    currentDate,
    player.team_id ?? "",
    player.career.length,
    (player.movement_history ?? []).length,
  ].join("|");
}
