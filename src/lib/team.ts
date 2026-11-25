import type { TeamData } from "../store/gameStore";

export function getTeamName(
  teams: TeamData[],
  id: string | null,
  unknownLabel = "Unknown",
): string {
  if (!id) {
    return "Free Agent";
  }
  return teams.find((team) => team.id === id)?.name ?? unknownLabel;
}

/** The translation key a season at no club is worded with, and the marker the backend carries it as. */
export const FREE_AGENT_TEAM_KEY = "common.freeAgent";

/** What older saves stored as the name of a season at no club: the English text itself. */
export const LEGACY_FREE_AGENT_TEAM_NAME = "Free Agent";

/**
 * A team name as stored on a career entry, an award or a news article, ready to show: a season at
 * no club stores no name (older saves stored the English literal), and is worded as the
 * translated "free agent". A club with a name keeps it whatever its id, because a package can
 * author a historical club by name alone.
 */
export function storedTeamLabel(
  teamName: string | null | undefined,
  t: (key: string) => string,
): string {
  const name = teamName?.trim() ?? "";
  const isNoClub =
    name === "" || name === LEGACY_FREE_AGENT_TEAM_NAME || name === FREE_AGENT_TEAM_KEY;
  return isNoClub ? t(FREE_AGENT_TEAM_KEY) : name;
}

export function getTeamShort(teams: TeamData[], id: string): string {
  return teams.find((team) => team.id === id)?.short_name ?? "???";
}
