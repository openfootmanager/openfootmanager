import type { PlayerData, ScoutedPlayer } from "../store/gameStore";

export interface ScoutedReport {
  scoutedOn: string;
  attributes: PlayerData["attributes"];
  outOfDate: boolean;
}

// ISO dates (YYYY-MM-DD) order the same as the days they name.
export function findScoutedReport(
  scoutedPlayers: ScoutedPlayer[] | undefined,
  playerId: string,
  seasonStart: string | null | undefined,
): ScoutedReport | null {
  const scouted = scoutedPlayers?.find((entry) => entry.player_id === playerId);
  if (!scouted) return null;
  return {
    scoutedOn: scouted.scouted_on,
    attributes: scouted.attributes,
    outOfDate: !!seasonStart && scouted.scouted_on < seasonStart,
  };
}
