import { naturalPositionOf } from "../../lib/positions";
import type { PlayerData } from "../../store/gameStore";

export interface KeyPlayer {
  id: string;
  name: string;
  /** His natural position. */
  position: string;
  age: number;
  ovr: number;
}

/**
 * A club's five key players: the likely eleven's best-rated, by the backend's rating
 * (`get_team_selection_ratings`). An ordering of rated figures, not a judgement of them. A player
 * the backend has not rated yet is left out.
 */
export function keyPlayers(
  xi: readonly PlayerData[],
  ratingOf: (playerId: string) => number | null,
  ageOf: (player: PlayerData) => number,
): KeyPlayer[] {
  return xi
    .flatMap((player) => {
      const ovr = ratingOf(player.id);
      return ovr === null
        ? []
        : [
            {
              id: player.id,
              name: player.match_name,
              position: naturalPositionOf(player),
              age: ageOf(player),
              ovr,
            },
          ];
    })
    .sort((left, right) => right.ovr - left.ovr)
    .slice(0, 5);
}
