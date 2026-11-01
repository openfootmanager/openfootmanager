import type { PositionTable } from "../../lib/positionTable";
import { getPlayerWeeklyWageCommitment } from "../../lib/finance";
import type { GameStateData, PlayerData, TeamData } from "../../store/gameStore";

import type { ClubLeagueStanding, TeamProfileViewModel } from "./TeamProfile.types";

function sortRoster(players: PlayerData[], positions: PositionTable): PlayerData[] {
  return [...players].sort((leftPlayer, rightPlayer) => {
    return positions.lineRankOf(leftPlayer.position) - positions.lineRankOf(rightPlayer.position);
  });
}

export function buildTeamProfileViewModel(
  team: TeamData,
  gameState: GameStateData,
  leagueStanding: ClubLeagueStanding | null,
  positions: PositionTable,
): TeamProfileViewModel {
  const roster = sortRoster(
    gameState.players.filter((player) => player.team_id === team.id),
    positions,
  );

  return {
    roster,
    totalWages: gameState.players.reduce(
      (sum, player) => sum + getPlayerWeeklyWageCommitment(player, team.id),
      0,
    ),
    totalValue: roster.reduce((sum, player) => sum + player.market_value, 0),
    manager: gameState.manager.team_id === team.id ? gameState.manager : null,
    leaguePos: leagueStanding?.league_pos ?? 0,
    standings: leagueStanding?.standing ?? null,
  };
}
