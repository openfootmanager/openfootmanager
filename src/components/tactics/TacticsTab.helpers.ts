import { getPlayerOvr } from "../../lib/helpers";
import { isFirstTeamSquadPlayer } from "../../lib/playerSquad";
import type { PositionTable } from "../../lib/positionTable";
import type { PlayerData } from "../../store/gameStore";
import { type PitchSlotRow, isPlayerOutOfPosition } from "../squad/SquadTab.helpers";
export { FORMATIONS } from "../match/types";

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

export function buildTacticsRoster(
  players: PlayerData[],
  teamId: string,
  positionTable: PositionTable,
): PlayerData[] {
  return players
    .filter((player) => player.team_id === teamId && isFirstTeamSquadPlayer(player))
    .sort((leftPlayer, rightPlayer) => {
      return (
        positionTable.lineRankOf(leftPlayer.position) -
          positionTable.lineRankOf(rightPlayer.position) ||
        getPlayerOvr(rightPlayer) - getPlayerOvr(leftPlayer)
      );
    });
}

export function countOutOfPositionPlayers(
  startingPlayers: PlayerData[],
  xiActivePosition: Map<string, string>,
  positionTable: PositionTable,
): number {
  return startingPlayers.filter((player) => {
    const currentPosition = xiActivePosition.get(player.id) ?? player.position;

    return isPlayerOutOfPosition(player, currentPosition, positionTable);
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
  const selectedPlayer = selectedPlayerId ? (playersById.get(selectedPlayerId) ?? null) : null;

  const comparePlayer =
    selectedPlayerId && comparePlayerId && selectedPlayerId !== comparePlayerId
      ? (playersById.get(comparePlayerId) ?? null)
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
    return Array.from(
      { length: slotCount },
      (_, index) => Math.round((10 + (index * 80) / (slotCount - 1)) * 10) / 10,
    );
  }

  return Array.from(
    { length: slotCount },
    (_, index) => Math.round(((index + 1) / (slotCount + 1)) * 100 * 10) / 10,
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

export function findTacticsPresetBySetup(
  formation: string,
  playStyle: string,
): TacticsPresetDefinition | null {
  return (
    TACTICS_PRESETS.find(
      (preset) => preset.formation === formation && preset.playStyle === playStyle,
    ) ?? null
  );
}
