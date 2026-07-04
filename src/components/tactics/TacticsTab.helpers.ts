import { getPlayerOvr } from "../../lib/helpers";
import { isSeniorSquadPlayer } from "../../lib/playerSquad";
import type { PlayerData } from "../../store/gameStore";
import {
  buildPitchRows,
  buildStartingXIIds,
  type PitchSlotRow,
  canonicalPosition,
  isPlayerExactForSlot,
  getPreferredPositions,
  isPlayerOutOfPosition,
  normalisePosition,
  positionCode,
  translatePositionAbbreviation,
  translatePositionLabel,
  type SquadSection,
} from "../squad/SquadTab.helpers";
export { FORMATIONS } from "../match/types";

export type TacticsLayoutMode = "balanced" | "pitch" | "analysis";
export type TacticsTableMode = "lineup" | "roles";
export type SortDirection = "asc" | "desc";
export type SortKey = "pos" | "name" | "condition" | "morale" | "ovr";

export interface TacticsPresetDefinition {
  descriptionKey: string;
  formation: string;
  id: string;
  playStyle: string;
}

export interface TacticsPitchSlot {
  index: number;
  player: PlayerData | null;
  position: string;
  rowLabel: string;
  x: number;
  y: number;
}

export interface TacticsFormationSlotOption {
  index: number;
  label: string;
  position: string;
  shortLabel: string;
}

export const TACTICS_PRESETS: TacticsPresetDefinition[] = [
  {
    id: "balanced-control",
    formation: "4-4-2",
    playStyle: "Balanced",
    descriptionKey: "tactics.presetDescriptions.balanced-control",
  },
  {
    id: "wing-play",
    formation: "4-3-3",
    playStyle: "Attacking",
    descriptionKey: "tactics.presetDescriptions.wing-play",
  },
  {
    id: "high-press",
    formation: "3-4-3",
    playStyle: "HighPress",
    descriptionKey: "tactics.presetDescriptions.high-press",
  },
  {
    id: "counter-attack",
    formation: "4-2-3-1",
    playStyle: "Counter",
    descriptionKey: "tactics.presetDescriptions.counter-attack",
  },
  {
    id: "low-block",
    formation: "5-3-2",
    playStyle: "Defensive",
    descriptionKey: "tactics.presetDescriptions.low-block",
  },
];

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

function comparePlayersForSlot(
  leftPlayer: PlayerData,
  rightPlayer: PlayerData,
  slotPosition: string,
): number {
  return (
    Number(isPlayerOutOfPosition(leftPlayer, slotPosition)) -
    Number(isPlayerOutOfPosition(rightPlayer, slotPosition)) ||
    Number(!isPlayerExactForSlot(leftPlayer, slotPosition)) -
    Number(!isPlayerExactForSlot(rightPlayer, slotPosition)) ||
    getPlayerOvr(rightPlayer) - getPlayerOvr(leftPlayer) ||
    rightPlayer.condition - leftPlayer.condition ||
    leftPlayer.full_name.localeCompare(rightPlayer.full_name)
  );
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
        getPlayerOvr(rightPlayer) - getPlayerOvr(leftPlayer)
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
      .sort((leftPlayer, rightPlayer) => comparePlayersForSlot(leftPlayer, rightPlayer, slotPosition))[0];

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
          getPlayerOvr(rightPlayer) - getPlayerOvr(leftPlayer)
        );
      case "name":
        return leftPlayer.full_name.localeCompare(rightPlayer.full_name);
      case "condition":
        return leftPlayer.condition - rightPlayer.condition;
      case "morale":
        return leftPlayer.morale - rightPlayer.morale;
      case "ovr":
        return getPlayerOvr(leftPlayer) - getPlayerOvr(rightPlayer);
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

function parseCoordinateValue(value: string): number {
  const parsed = Number.parseFloat(value);
  return Number.isFinite(parsed) ? parsed : 50;
}

function getSlotXCoordinates(slotCount: number): number[] {
  // Rows of five (back/mid fives in 4-5-1, 3-5-2, 5-4-1…) get pushed toward
  // the touchlines so neighbouring markers don't overlap; smaller rows keep
  // the centered even spread.
  if (slotCount >= 5) {
    return Array.from({ length: slotCount }, (_, index) =>
      Math.round((10 + (index * 80) / (slotCount - 1)) * 10) / 10,
    );
  }

  return Array.from({ length: slotCount }, (_, index) =>
    Math.round((((index + 1) / (slotCount + 1)) * 100) * 10) / 10,
  );
}

export function buildTacticsPitchSlots(rows: PitchSlotRow[]): TacticsPitchSlot[] {
  return rows.flatMap((row) => {
    const rowY = parseCoordinateValue(row.y);
    const rowXCoordinates = getSlotXCoordinates(row.slots.length);

    return row.slots.map((slot, slotIndex) => ({
      index: slot.index,
      player: slot.player,
      position: slot.position,
      rowLabel: row.label,
      x: rowXCoordinates[slotIndex] ?? 50,
      y: rowY,
    }));
  });
}

function getDuplicatedSlotShortLabel(
  position: string,
  duplicateIndex: number,
  duplicateCount: number,
): string {
  const canonical = canonicalPosition(position);

  if (canonical === "CenterBack") {
    if (duplicateCount === 2) return duplicateIndex === 0 ? "LCB" : "RCB";
    if (duplicateCount === 3) {
      return ["LCB", "CB", "RCB"][duplicateIndex] ?? "CB";
    }
  }

  if (canonical === "CentralMidfielder") {
    if (duplicateCount === 2) return duplicateIndex === 0 ? "LCM" : "RCM";
    if (duplicateCount === 3) {
      return ["LCM", "CM", "RCM"][duplicateIndex] ?? "CM";
    }
  }

  if (canonical === "Striker") {
    if (duplicateCount === 2) return duplicateIndex === 0 ? "LS" : "RS";
    if (duplicateCount === 3) {
      return ["LF", "ST", "RF"][duplicateIndex] ?? "ST";
    }
  }

  return `${positionCode(position)} ${duplicateIndex + 1}`;
}

function getDuplicatedSlotLabel(
  translate: (key: string) => string,
  position: string,
  duplicateIndex: number,
  duplicateCount: number,
): string {
  const positionLabel = translatePositionLabel(translate, position);

  if (duplicateCount === 2) {
    return duplicateIndex === 0
      ? `${translate("common.left")} ${positionLabel}`
      : `${translate("common.right")} ${positionLabel}`;
  }

  if (duplicateCount === 3) {
    const descriptors = [
      translate("common.left"),
      translate("common.center"),
      translate("common.right"),
    ];

    return `${descriptors[duplicateIndex] ?? duplicateIndex + 1} ${positionLabel}`;
  }

  return `${positionLabel} ${duplicateIndex + 1}`;
}

export function buildFormationSlotOptions(
  formation: string,
  translate: (key: string) => string,
): TacticsFormationSlotOption[] {
  const positions = buildPitchRows(formation).flatMap((row) => row.positions);
  const duplicateCounts = new Map<string, number>();
  const duplicateIndexes = new Map<string, number>();

  positions.forEach((position) => {
    duplicateCounts.set(position, (duplicateCounts.get(position) ?? 0) + 1);
  });

  return positions.map((position, index) => {
    const duplicateIndex = duplicateIndexes.get(position) ?? 0;
    const duplicateCount = duplicateCounts.get(position) ?? 1;
    duplicateIndexes.set(position, duplicateIndex + 1);

    if (duplicateCount === 1) {
      return {
        index,
        label: translatePositionLabel(translate, position),
        position,
        shortLabel: translatePositionAbbreviation(translate, position),
      };
    }

    return {
      index,
      label: getDuplicatedSlotLabel(
        translate,
        position,
        duplicateIndex,
        duplicateCount,
      ),
      position,
      shortLabel: getDuplicatedSlotShortLabel(
        position,
        duplicateIndex,
        duplicateCount,
      ),
    };
  });
}

export function findTacticsPresetBySetup(
  formation: string,
  playStyle: string,
): TacticsPresetDefinition | null {
  return (
    TACTICS_PRESETS.find(
      (preset) =>
        preset.formation === formation && preset.playStyle === playStyle,
    ) ?? null
  );
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
