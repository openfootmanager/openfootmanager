import type { PlayerData } from "../store/gameStore";
import type { PlayerSquadRole } from "../store/types";
import { calcAge } from "./helpers";

export function getPlayerSquadRole(player: Pick<PlayerData, "squad_role">): PlayerSquadRole {
  return player.squad_role === "Youth" ? "Youth" : "Senior";
}

export function isYouthAcademyPlayer(player: Pick<PlayerData, "squad_role">): boolean {
  return getPlayerSquadRole(player) === "Youth";
}

export function isSeniorSquadPlayer(player: Pick<PlayerData, "squad_role">): boolean {
  return getPlayerSquadRole(player) === "Senior";
}

export function canDelegateToYouthAcademy(
  player: Pick<PlayerData, "date_of_birth" | "squad_role">,
): boolean {
  return isSeniorSquadPlayer(player) && calcAge(player.date_of_birth) <= 21;
}

/**
 * Whether the backend lets this player be picked for the match day. Its verdict, not an injury
 * test: when fewer than eleven are healthy it fields injured players too, and the builder only
 * knows the squad it was given. Absent until the squad has been fetched, which reads as "not yet".
 */
export function isMatchDayEligible(player: Pick<PlayerData, "match_day_eligible">): boolean {
  return player.match_day_eligible === true;
}

/** Seniors remain manageable during injury; youth visibility comes from the backend call-up. */
export function isFirstTeamSquadPlayer(
  player: Pick<PlayerData, "squad_role" | "match_day_eligible">,
): boolean {
  return isSeniorSquadPlayer(player) || player.match_day_eligible === true;
}
