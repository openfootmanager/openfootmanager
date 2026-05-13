import type {
  PlayerData,
  ScoutingAssignment,
  TeamData,
} from "../../store/gameStore";
import { calcOvr, getTeamName } from "../../lib/helpers";
import { normalisePosition } from "../squad/SquadTab.helpers";

interface FilterScoutablePlayersParams {
  players: PlayerData[];
  teams: TeamData[];
  myTeamId: string;
  posFilter: string;
  searchQuery: string;
}

export function filterScoutablePlayers({
  players,
  teams,
  myTeamId,
  posFilter,
  searchQuery,
}: FilterScoutablePlayersParams): PlayerData[] {
  return players
    .filter((player) => player.team_id !== myTeamId)
    .filter(
      (player) =>
        posFilter === "All" ||
        normalisePosition(player.natural_position || player.position) === posFilter,
    )
    .filter((player) => {
      if (!searchQuery) {
        return true;
      }

      const query = searchQuery.toLowerCase();

      return (
        player.full_name.toLowerCase().includes(query) ||
        player.nationality.toLowerCase().includes(query) ||
        (player.team_id &&
          getTeamName(teams, player.team_id).toLowerCase().includes(query))
      );
    })
    .sort(
      (left, right) =>
        (right.ovr ?? calcOvr(right, right.natural_position || right.position)) -
        (left.ovr ?? calcOvr(left, left.natural_position || left.position)),
    );
}

export function paginateScoutablePlayers(
  players: PlayerData[],
  page: number,
  pageSize: number,
) {
  const totalPages = Math.max(1, Math.ceil(players.length / pageSize));
  const safePage = Math.min(page, totalPages - 1);

  return {
    totalPages,
    safePage,
    players: players.slice(safePage * pageSize, (safePage + 1) * pageSize),
  };
}

export function buildAlreadyScoutingIds(assignments: ScoutingAssignment[]) {
  return new Set(assignments.map((assignment) => assignment.player_id));
}