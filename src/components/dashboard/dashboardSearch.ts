import type { GameStateData, PlayerData, TeamData } from "../../store/gameStore";

export interface DashboardSearchResults {
  matchedPlayers: PlayerData[];
  matchedTeams: TeamData[];
}

export function getDashboardSearchResults(
  gameState: GameStateData,
  query: string,
): DashboardSearchResults {
  const normalizedQuery = query.trim().toLowerCase();

  if (normalizedQuery.length < 2) {
    return {
      matchedPlayers: [],
      matchedTeams: [],
    };
  }

  return {
    matchedPlayers: gameState.players
      .filter((player) => {
        return (
          player.full_name.toLowerCase().includes(normalizedQuery) ||
          player.match_name.toLowerCase().includes(normalizedQuery)
        );
      })
      .slice(0, 5),
    matchedTeams: gameState.teams
      .filter((team) => {
        return (
          team.name.toLowerCase().includes(normalizedQuery) ||
          team.short_name.toLowerCase().includes(normalizedQuery)
        );
      })
      .slice(0, 4),
  };
}