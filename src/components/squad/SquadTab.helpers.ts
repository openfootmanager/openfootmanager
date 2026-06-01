import type { PlayerData } from "../../store/gameStore";
import { getPlayerOvr } from "../../lib/helpers";

export type SquadSection = "xi" | "bench";
export type DragState = {
  playerId: string;
  from: SquadSection;
  slotIndex: number | null;
};

export type PitchRow = { label: string; y: string; positions: string[] };
export type PitchSlot = {
  index: number;
  position: string;
  player: PlayerData | null;
};
export type PitchSlotRow = PitchRow & { slots: PitchSlot[] };

export const CORE_POSITIONS = [
  "Goalkeeper",
  "Defender",
  "Midfielder",
  "Forward",
] as const;

const CANONICAL_POSITION_MAP: Record<string, string> = {
  gk: "Goalkeeper",
  goalkeeper: "Goalkeeper",
  defender: "Defender",
  def: "Defender",
  wingback: "Defender",
  midfielder: "Midfielder",
  mid: "Midfielder",
  forward: "Forward",
  fwd: "Forward",
  winger: "Forward",
  rb: "RightBack",
  rightback: "RightBack",
  cb: "CenterBack",
  centerback: "CenterBack",
  centreback: "CenterBack",
  lb: "LeftBack",
  leftback: "LeftBack",
  rwb: "RightWingBack",
  rightwingback: "RightWingBack",
  lwb: "LeftWingBack",
  leftwingback: "LeftWingBack",
  dm: "DefensiveMidfielder",
  defensivemidfielder: "DefensiveMidfielder",
  cm: "CentralMidfielder",
  centralmidfielder: "CentralMidfielder",
  am: "AttackingMidfielder",
  attackingmidfielder: "AttackingMidfielder",
  rm: "RightMidfielder",
  rightmidfielder: "RightMidfielder",
  lm: "LeftMidfielder",
  leftmidfielder: "LeftMidfielder",
  rw: "RightWinger",
  rightwinger: "RightWinger",
  lw: "LeftWinger",
  leftwinger: "LeftWinger",
  st: "Striker",
  striker: "Striker",
};

const POSITION_GROUPS: Record<string, string> = {
  Goalkeeper: "Goalkeeper",
  Defender: "Defender",
  Midfielder: "Midfielder",
  Forward: "Forward",
  RightBack: "Defender",
  CenterBack: "Defender",
  LeftBack: "Defender",
  RightWingBack: "Defender",
  LeftWingBack: "Defender",
  DefensiveMidfielder: "Midfielder",
  CentralMidfielder: "Midfielder",
  AttackingMidfielder: "Midfielder",
  RightMidfielder: "Midfielder",
  LeftMidfielder: "Midfielder",
  RightWinger: "Forward",
  LeftWinger: "Forward",
  Striker: "Forward",
};

const POSITION_LABELS: Record<string, string> = {
  Goalkeeper: "Goalkeeper",
  Defender: "Defender",
  Midfielder: "Midfielder",
  Forward: "Forward",
  RightBack: "Right Back",
  CenterBack: "Center Back",
  LeftBack: "Left Back",
  RightWingBack: "Right Wing-Back",
  LeftWingBack: "Left Wing-Back",
  DefensiveMidfielder: "Defensive Midfielder",
  CentralMidfielder: "Central Midfielder",
  AttackingMidfielder: "Attacking Midfielder",
  RightMidfielder: "Right Midfielder",
  LeftMidfielder: "Left Midfielder",
  RightWinger: "Right Winger",
  LeftWinger: "Left Winger",
  Striker: "Striker",
};

const POSITION_CODES: Record<string, string> = {
  Goalkeeper: "GK",
  Defender: "DEF",
  Midfielder: "MID",
  Forward: "FWD",
  RightBack: "RB",
  CenterBack: "CB",
  LeftBack: "LB",
  RightWingBack: "RWB",
  LeftWingBack: "LWB",
  DefensiveMidfielder: "DM",
  CentralMidfielder: "CM",
  AttackingMidfielder: "AM",
  RightMidfielder: "RM",
  LeftMidfielder: "LM",
  RightWinger: "RW",
  LeftWinger: "LW",
  Striker: "ST",
};

function normaliseKey(value: string): string {
  return value.toLowerCase().replace(/[^a-z]/g, "");
}

export function canonicalPosition(position: string): string {
  const trimmed = position.trim();
  if (!trimmed) return trimmed;

  return CANONICAL_POSITION_MAP[normaliseKey(trimmed)] || trimmed;
}

export function parseFormationSlots(formation: string): {
  def: number;
  mid: number;
  fwd: number;
} {
  const parts = formation.split("-").map(Number);
  if (parts.length === 4) {
    return { def: parts[0], mid: parts[1] + parts[2], fwd: parts[3] };
  }
  if (parts.length === 3) {
    return { def: parts[0], mid: parts[1], fwd: parts[2] };
  }
  return { def: 4, mid: 4, fwd: 2 };
}

export function normalisePosition(position: string): string {
  const canonical = canonicalPosition(position);
  return POSITION_GROUPS[canonical] || canonical;
}

export function positionCode(position: string): string {
  const normalized = canonicalPosition(position);
  return (
    POSITION_CODES[normalized] || normalized.substring(0, 3).toUpperCase()
  );
}

export function translatePositionLabel(
  translate: (key: string, options?: { defaultValue?: string }) => string,
  position: string,
): string {
  const canonical = canonicalPosition(position);

  return translate(`common.positions.${canonical}`, {
    defaultValue: POSITION_LABELS[canonical] || canonical,
  });
}

export function translatePositionAbbreviation(
  translate: (key: string, options?: { defaultValue?: string }) => string,
  position: string,
): string {
  const normalized = canonicalPosition(position);

  return translate(`common.posAbbr.${normalized}`, {
    defaultValue: positionCode(position),
  });
}

export function getPreferredPositions(player: PlayerData): string[] {
  return [
    ...new Set(
      [
        player.natural_position || player.position,
        ...(player.alternate_positions || []),
      ]
        .filter(Boolean)
        .map(canonicalPosition),
    ),
  ];
}

export function buildPitchRows(formation: string): PitchRow[] {
  const parts = formation
    .split("-")
    .map(Number)
    .filter((value) => !Number.isNaN(value));

  const defenderLine = (count: number): string[] => {
    switch (count) {
      case 3:
        return ["CenterBack", "CenterBack", "CenterBack"];
      case 4:
        return ["LeftBack", "CenterBack", "CenterBack", "RightBack"];
      case 5:
        return [
          "LeftWingBack",
          "CenterBack",
          "CenterBack",
          "CenterBack",
          "RightWingBack",
        ];
      default:
        return Array(count).fill("CenterBack");
    }
  };

  const midfieldLine = (count: number): string[] => {
    switch (count) {
      case 2:
        return ["CentralMidfielder", "CentralMidfielder"];
      case 3:
        return [
          "DefensiveMidfielder",
          "CentralMidfielder",
          "AttackingMidfielder",
        ];
      case 4:
        return [
          "LeftMidfielder",
          "CentralMidfielder",
          "CentralMidfielder",
          "RightMidfielder",
        ];
      case 5:
        return [
          "LeftMidfielder",
          "DefensiveMidfielder",
          "CentralMidfielder",
          "AttackingMidfielder",
          "RightMidfielder",
        ];
      default:
        return Array(count).fill("CentralMidfielder");
    }
  };

  const deepMidfieldLine = (count: number): string[] => {
    switch (count) {
      case 1:
        return ["DefensiveMidfielder"];
      case 2:
        return ["DefensiveMidfielder", "CentralMidfielder"];
      default:
        return Array(count).fill("DefensiveMidfielder");
    }
  };

  const attackingMidfieldLine = (count: number): string[] => {
    switch (count) {
      case 1:
        return ["AttackingMidfielder"];
      case 2:
        return ["AttackingMidfielder", "AttackingMidfielder"];
      case 3:
        return ["LeftMidfielder", "AttackingMidfielder", "RightMidfielder"];
      default:
        return Array(count).fill("AttackingMidfielder");
    }
  };

  const forwardLine = (count: number): string[] => {
    switch (count) {
      case 1:
        return ["Striker"];
      case 2:
        return ["Striker", "Striker"];
      case 3:
        return ["LeftWinger", "Striker", "RightWinger"];
      default:
        return Array(count).fill("Striker");
    }
  };

  if (parts.length === 4) {
    const forwardY = parts[3] === 1 ? "16%" : "20%";
    const attackingMidY = parts[3] === 1 ? "40%" : "38%";

    return [
      { label: "GK", y: "88%", positions: ["Goalkeeper"] },
      { label: "DEF", y: "70%", positions: defenderLine(parts[0]) },
      { label: "DM", y: "54%", positions: deepMidfieldLine(parts[1]) },
      { label: "AM", y: attackingMidY, positions: attackingMidfieldLine(parts[2]) },
      { label: "FWD", y: forwardY, positions: forwardLine(parts[3]) },
    ];
  }

  const slots = parseFormationSlots(formation);
  const forwardY = slots.fwd === 1 ? "18%" : "22%";
  const midfieldY = slots.fwd === 1 ? "50%" : "46%";

  return [
    { label: "GK", y: "88%", positions: ["Goalkeeper"] },
    { label: "DEF", y: "70%", positions: defenderLine(slots.def) },
    { label: "MID", y: midfieldY, positions: midfieldLine(slots.mid) },
    { label: "FWD", y: forwardY, positions: forwardLine(slots.fwd) },
  ];
}

export function getPitchRowWidth(slotCount: number): string {
  if (slotCount >= 5) return "88%";
  if (slotCount === 4) return "82%";
  if (slotCount === 3) return "68%";
  if (slotCount === 2) return "50%";
  return "28%";
}
export function getPitchSlotWidth(slotCount: number): number {
  if (slotCount >= 5) return 66;
  if (slotCount === 4) return 70;
  if (slotCount === 3) return 74;
  if (slotCount === 2) return 78;
  return 82;
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

export function buildStartingXIIds(
  available: PlayerData[],
  savedIds: string[],
  formation: string,
): string[] {
  const rows = buildPitchRows(formation);
  const slotPositions = rows.flatMap((row) => row.positions);
  const byId = new Map(available.map((player) => [player.id, player]));
  const validSavedIds: string[] = [];
  const used = new Set<string>();

  for (const id of savedIds) {
    const player = byId.get(id);
    if (player && !used.has(id)) {
      validSavedIds.push(id);
      used.add(id);
    }
  }

  if (validSavedIds.length >= 8) {
    const xi = [...validSavedIds];
    while (xi.length < 11) {
      const slotPosition = slotPositions[xi.length];
      const candidates = available.filter((player) => !used.has(player.id));
      const bestPlayer = candidates.sort((a, b) => comparePlayersForSlot(a, b, slotPosition))[0];

      if (!bestPlayer) break;
      xi.push(bestPlayer.id);
      used.add(bestPlayer.id);
    }
    return xi.slice(0, 11);
  }

  const xi: string[] = [];
  slotPositions.slice(0, 11).forEach((slotPosition) => {
    const candidates = available.filter((player) => !used.has(player.id));
    const bestPlayer = candidates.sort((a, b) => comparePlayersForSlot(a, b, slotPosition))[0];

    if (bestPlayer) {
      xi.push(bestPlayer.id);
      used.add(bestPlayer.id);
    }
  });

  return xi;
}

export function buildPitchSlotRows(
  rows: PitchRow[],
  xiIds: string[],
  playersById: Map<string, PlayerData>,
): PitchSlotRow[] {
  let slotIndex = 0;
  return rows.map((row) => ({
    ...row,
    slots: row.positions.map((position) => {
      const slot: PitchSlot = {
        index: slotIndex,
        position,
        player: playersById.get(xiIds[slotIndex]) ?? null,
      };
      slotIndex += 1;
      return slot;
    }),
  }));
}

export function buildActivePositionMap(
  pitchSlotRows: PitchSlotRow[],
): Map<string, string> {
  const map = new Map<string, string>();
  pitchSlotRows.forEach((row) => {
    row.slots.forEach((slot) => {
      if (slot.player) {
        map.set(slot.player.id, canonicalPosition(slot.position));
      }
    });
  });
  return map;
}

export function isPlayerOutOfPosition(
  player: PlayerData,
  currentPos: string,
): boolean {
  const canonicalCurrentPos = canonicalPosition(currentPos);
  const normalizedCurrentPos = normalisePosition(currentPos);
  return !getPreferredPositions(player).some(
    (position) =>
      position === canonicalCurrentPos ||
      normalisePosition(position) === normalizedCurrentPos,
  );
}

export function isPlayerExactForSlot(
  player: PlayerData,
  currentPos: string,
): boolean {
  return canonicalPosition(player.natural_position || player.position) === canonicalPosition(currentPos);
}

export function applyLineupDrop(
  currentXiIds: string[],
  dragState: DragState,
  slotIndex: number,
): string[] {
  const nextXiIds = [...currentXiIds];

  if (slotIndex < 0 || slotIndex >= nextXiIds.length) {
    return nextXiIds;
  }

  if (dragState.from === "xi") {
    const fromIndex =
      dragState.slotIndex ?? nextXiIds.indexOf(dragState.playerId);
    if (fromIndex < 0 || fromIndex === slotIndex) {
      return nextXiIds;
    }
    [nextXiIds[fromIndex], nextXiIds[slotIndex]] = [
      nextXiIds[slotIndex],
      nextXiIds[fromIndex],
    ];
    return nextXiIds;
  }

  const existingIndex = nextXiIds.indexOf(dragState.playerId);
  if (existingIndex === slotIndex) {
    return nextXiIds;
  }
  if (existingIndex >= 0) {
    nextXiIds.splice(existingIndex, 1);
    if (existingIndex < slotIndex) {
      slotIndex -= 1;
    }
    nextXiIds.splice(slotIndex, 0, dragState.playerId);
    return nextXiIds.slice(0, currentXiIds.length);
  }
  if (slotIndex >= nextXiIds.length) {
    nextXiIds.push(dragState.playerId);
  } else {
    nextXiIds[slotIndex] = dragState.playerId;
  }
  return nextXiIds.slice(0, currentXiIds.length);
}

export function applyLineupSwap(
  currentXiIds: string[],
  swapSource: { id: string; from: SquadSection },
  playerId: string,
  from: SquadSection,
): string[] | null {
  if (swapSource.from === "xi" && from === "bench") {
    return currentXiIds.map((id) => (id === swapSource.id ? playerId : id));
  }

  if (swapSource.from === "bench" && from === "xi") {
    return currentXiIds.map((id) => (id === playerId ? swapSource.id : id));
  }

  if (swapSource.from === "xi" && from === "xi") {
    const firstIndex = currentXiIds.indexOf(swapSource.id);
    const secondIndex = currentXiIds.indexOf(playerId);
    if (firstIndex < 0 || secondIndex < 0 || firstIndex === secondIndex) {
      return currentXiIds;
    }
    const nextXiIds = [...currentXiIds];
    nextXiIds[firstIndex] = playerId;
    nextXiIds[secondIndex] = swapSource.id;
    return nextXiIds;
  }

  return null;
}
