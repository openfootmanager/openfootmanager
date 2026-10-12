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

export function getTeamShort(teams: TeamData[], id: string): string {
  return teams.find((team) => team.id === id)?.short_name ?? "???";
}
