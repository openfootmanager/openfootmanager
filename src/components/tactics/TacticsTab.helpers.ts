import { calcAge, calcOvr } from "../../lib/helpers";
import { isSeniorSquadPlayer } from "../../lib/playerSquad";
import type { PlayerData } from "../../store/gameStore";
import {
  buildPitchRows,
  buildStartingXIIds,
  getPreferredPositions,
  isPlayerOutOfPosition,
  normalisePosition,
  positionCode,
  type SquadSection,
} from "../squad/SquadTab.helpers";

export const FORMATIONS = [
  "4-4-2",
  "4-3-3",
  "3-5-2",
  "4-5-1",
  "4-2-3-1",
  "3-4-3",
  "5-3-2",
  "4-1-4-1",
];

export const PLAY_STYLE_DESCRIPTION_FALLBACKS: Record<string, string> = {
  Balanced:
    "Keeps your team measured in and out of possession, with a steady shape and fewer extremes.",
  Attacking:
    "Pushes more bodies forward, creates extra support around the box, and asks your team to take more initiative.",
  Defensive:
    "Makes your team protect space first, stay compact, and reduce the risk of getting exposed behind the ball.",
  Possession:
    "Encourages your team to circulate the ball patiently, control the tempo, and look for cleaner openings.",
  Counter:
    "Invites your team to break forward quickly after regaining the ball, attacking space before the opponent resets.",
  HighPress:
    "Asks your team to close down earlier, win the ball higher up the pitch, and keep opponents under pressure.",
};

export type SortDirection = "asc" | "desc";
export type SortKey = "pos" | "name" | "age" | "condition" | "morale" | "ovr";

const POSITION_ORDER: Record<string, number> = {
  Goalkeeper: 1,
  Defender: 2,
  Midfielder: 3,
  Forward: 4,
};

interface TacticsPlayerSortContext {
  section: SquadSection;
  sortDir: SortDirection;
  sortKey: SortKey;
  xiActivePosition: Map<string, string>;
}

interface TacticsPlayerFilterContext {
  playerSearch: string;
  positionFilter: string;
  section: SquadSection;
  xiActivePosition: Map<string, string>;
}

interface ResolveStartingXiIdsOptions {
  availablePlayers: PlayerData[];
  formation: string;
  pendingStartingXiIds: string[] | null;
  playersById: Map<string, PlayerData>;
  savedStartingXiIds: string[];
}

export function buildTacticsRoster(
  players: PlayerData[],
  teamId: string,
): PlayerData[] {
  return players
    .filter(
      (player) => player.team_id === teamId && isSeniorSquadPlayer(player),
    )
    .sort((leftPlayer, rightPlayer) => {
      return (
        (POSITION_ORDER[normalisePosition(leftPlayer.position)] ?? 99) -
        (POSITION_ORDER[normalisePosition(rightPlayer.position)] ?? 99) ||
        calcOvr(rightPlayer, rightPlayer.natural_position || rightPlayer.position) -
        calcOvr(leftPlayer, leftPlayer.natural_position || leftPlayer.position)
      );
    });
}

export function resolveStartingXiIds({
  availablePlayers,
  formation,
  pendingStartingXiIds,
  playersById,
  savedStartingXiIds,
}: ResolveStartingXiIdsOptions): string[] {
  const baseIds = buildStartingXIIds(
    availablePlayers,
    savedStartingXiIds,
    formation,
  );
  const slotPositions = buildPitchRows(formation).flatMap((row) => row.positions);

  if (!pendingStartingXiIds || pendingStartingXiIds.length === 0) {
    return baseIds;
  }

  const validPendingIds = pendingStartingXiIds.filter((id) => playersById.has(id));
  const usedPlayerIds = new Set(validPendingIds);
  const fillPlayerIds: string[] = [];

  while (validPendingIds.length + fillPlayerIds.length < 11) {
    const slotPosition = slotPositions[validPendingIds.length + fillPlayerIds.length];
    const bestPlayer = availablePlayers
      .filter((player) => !usedPlayerIds.has(player.id))
      .sort(
        (leftPlayer, rightPlayer) =>
          calcOvr(rightPlayer, slotPosition) - calcOvr(leftPlayer, slotPosition),
      )[0];

    if (!bestPlayer) break;
    fillPlayerIds.push(bestPlayer.id);
    usedPlayerIds.add(bestPlayer.id);
  }

  return [...validPendingIds, ...fillPlayerIds].slice(0, 11);
}

export function getSectionPlayerPosition(
  player: PlayerData,
  section: SquadSection,
  xiActivePosition: Map<string, string>,
): string {
  if (section === "xi") {
    return xiActivePosition.get(player.id) ?? player.position;
  }

  return player.natural_position || player.position;
}

export function sortTacticsPlayers(
  players: PlayerData[],
  context: TacticsPlayerSortContext,
): PlayerData[] {
  const { section, sortDir, sortKey, xiActivePosition } = context;
  const sortedPlayers = [...players].sort((leftPlayer, rightPlayer) => {
    const leftPosition = getSectionPlayerPosition(leftPlayer, section, xiActivePosition);
    const rightPosition = getSectionPlayerPosition(rightPlayer, section, xiActivePosition);

    switch (sortKey) {
      case "pos":
        return (
          (POSITION_ORDER[normalisePosition(leftPosition)] ?? 99) -
          (POSITION_ORDER[normalisePosition(rightPosition)] ?? 99) ||
          calcOvr(rightPlayer, rightPosition) - calcOvr(leftPlayer, leftPosition)
        );
      case "name":
        return leftPlayer.full_name.localeCompare(rightPlayer.full_name);
      case "age":
        return calcAge(leftPlayer.date_of_birth) - calcAge(rightPlayer.date_of_birth);
      case "condition":
        return leftPlayer.condition - rightPlayer.condition;
      case "morale":
        return leftPlayer.morale - rightPlayer.morale;
      case "ovr":
        return calcOvr(leftPlayer, leftPosition) - calcOvr(rightPlayer, rightPosition);
      default:
        return 0;
    }
  });

  if (sortDir === "desc") {
    return sortedPlayers.reverse();
  }

  return sortedPlayers;
}

export function matchesTacticsPlayerFilters(
  player: PlayerData,
  context: TacticsPlayerFilterContext,
): boolean {
  const { playerSearch, positionFilter, section, xiActivePosition } = context;
  const currentPosition = normalisePosition(
    getSectionPlayerPosition(player, section, xiActivePosition),
  );
  const preferredPositions = getPreferredPositions(player);
  const normalizedSearch = playerSearch.trim().toLowerCase();

  if (normalizedSearch) {
    const searchableText = [
      player.full_name,
      player.match_name,
      currentPosition,
      ...preferredPositions,
      ...preferredPositions.map(positionCode),
    ]
      .join(" ")
      .toLowerCase();

    if (!searchableText.includes(normalizedSearch)) {
      return false;
    }
  }

  if (
    positionFilter !== "All" &&
    currentPosition !== positionFilter &&
    !preferredPositions.includes(positionFilter)
  ) {
    return false;
  }

  return true;
}

export function filterAndSortTacticsPlayers(
  players: PlayerData[],
  filterContext: TacticsPlayerFilterContext,
  sortContext: TacticsPlayerSortContext,
): PlayerData[] {
  return sortTacticsPlayers(
    players.filter((player) => matchesTacticsPlayerFilters(player, filterContext)),
    sortContext,
  );
}

export function countOutOfPositionPlayers(
  startingPlayers: PlayerData[],
  xiActivePosition: Map<string, string>,
): number {
  return startingPlayers.filter((player) => {
    const currentPosition = xiActivePosition.get(player.id) ?? player.position;

    return isPlayerOutOfPosition(player, currentPosition);
  }).length;
}

export function getSelectedAndComparePlayers(
  comparePlayerId: string | null,
  playersById: Map<string, PlayerData>,
  selectedPlayerId: string | null,
): {
  comparePlayer: PlayerData | null;
  selectedPlayer: PlayerData | null;
} {
  const selectedPlayer = selectedPlayerId
    ? playersById.get(selectedPlayerId) ?? null
    : null;

  const comparePlayer =
    selectedPlayerId && comparePlayerId && selectedPlayerId !== comparePlayerId
      ? playersById.get(comparePlayerId) ?? null
      : null;

  return {
    comparePlayer,
    selectedPlayer,
  };
}

export function getOverallRatingClassName(overallRating: number): string {
  if (overallRating >= 75) {
    return "text-success-500 dark:text-success-400";
  }

  if (overallRating >= 55) {
    return "text-accent-600 dark:text-accent-400";
  }

  return "text-gray-500 dark:text-gray-400";
}
