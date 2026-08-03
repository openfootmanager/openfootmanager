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
export type SquadTacticalFit = "natural" | "adapted" | "out";
export type SquadStyleFit = "strong" | "good" | "risky";
export type SquadRoleCoverageStatus = "covered" | "thin" | "uncovered";
export type SquadRoleCoverage = {
  role: string;
  requiredSlots: number;
  naturalStarters: number;
  benchOptions: number;
  status: SquadRoleCoverageStatus;
};

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

/**
 * Canonical football-order rank for sorting rosters by natural_position.
 * Order: GK → back line (CB → FB → WB) → midfield (DM → CM → AM → wide) → forwards (wingers → ST).
 *
 * Legacy coarse buckets sort at the *start* of their group (rank +0.5) so
 * unmigrated players stay adjacent to their granular teammates instead of
 * scattering to the end.
 */
const POSITION_SORT_ORDER: Record<string, number> = {
  Goalkeeper: 10,
  Defender: 20,
  CenterBack: 21,
  LeftBack: 22,
  RightBack: 23,
  LeftWingBack: 24,
  RightWingBack: 25,
  Midfielder: 30,
  DefensiveMidfielder: 31,
  CentralMidfielder: 32,
  AttackingMidfielder: 33,
  LeftMidfielder: 34,
  RightMidfielder: 35,
  Forward: 40,
  LeftWinger: 41,
  RightWinger: 42,
  Striker: 43,
};

/**
 * Rank a position for canonical sort ordering. Unknown values sort to the end.
 */
export function positionSortRank(position: string): number {
  return POSITION_SORT_ORDER[canonicalPosition(position)] ?? 999;
}

const GROUP_ROLE_PREFERENCES: Record<string, string[]> = {
  Goalkeeper: ["Goalkeeper"],
  Defender: ["CenterBack", "LeftBack", "RightBack", "LeftWingBack", "RightWingBack"],
  Midfielder: [
    "CentralMidfielder",
    "DefensiveMidfielder",
    "AttackingMidfielder",
    "LeftMidfielder",
    "RightMidfielder",
  ],
  Forward: ["Striker", "LeftWinger", "RightWinger"],
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

/**
 * The position a player is currently in: their assigned pitch slot when they are
 * in the starting XI (`xiActivePosition` only holds XI players), otherwise their
 * natural position. `player.position` is only a last-resort fallback because on
 * legacy saves it can still hold a coarse bucket.
 */
export function getCurrentPosition(
  player: PlayerData,
  xiActivePosition: Map<string, string>,
): string {
  return (
    xiActivePosition.get(player.id) || player.natural_position || player.position
  );
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

/**
 * Order candidates for a pitch slot, best first.
 *
 * Players who can play the slot at all lead: it is one of their preferred
 * positions, or at least in the same position group. Among those, a player
 * whose natural position *is* the slot goes ahead of one merely covering it.
 * Then the higher rating, then the fresher legs, with name as a stable
 * tiebreak so the order never wobbles between renders.
 *
 * Squad and Tactics both auto-fill from this, so the precedence is a shared
 * contract, not a local preference: changing it moves players in Squad's
 * best-fit assignment and Tactics' auto-pick at the same time. Change it
 * deliberately, or not at all.
 */
export function comparePlayersForSlot(
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

/**
 * Front-end mirror of the backend's `deployed_position`
 * (ofm_core::player_rating): the granular slot a player occupies, derived from
 * the team's formation and starting-XI order. Returns null for players outside
 * the starting XI; callers fall back to the natural position, matching how the
 * backend validates `set_player_role`.
 */
export function getDeployedPosition(
  team: { formation: string; starting_xi_ids: string[] },
  playerId: string,
): string | null {
  const slotIndex = team.starting_xi_ids.indexOf(playerId);
  if (slotIndex < 0) return null;
  const slots = buildPitchRows(team.formation).flatMap((row) => row.positions);
  return slots[slotIndex] ?? null;
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

export function getSquadTacticalFit(
  player: PlayerData,
  currentPos: string,
): SquadTacticalFit {
  if (isPlayerExactForSlot(player, currentPos)) {
    return "natural";
  }

  if (isPlayerOutOfPosition(player, currentPos)) {
    return "out";
  }

  return "adapted";
}

function averageAttributes(
  player: PlayerData,
  attributeKeys: Array<keyof PlayerData["attributes"]>,
): number {
  return (
    attributeKeys.reduce((total, key) => total + player.attributes[key], 0) /
    attributeKeys.length
  );
}

export function getBestRoleForFormation(
  player: PlayerData,
  formation: string,
): string {
  const formationSlots = buildPitchRows(formation).flatMap((row) => row.positions);
  // Best role is an ability recommendation, so it is driven by the player's
  // natural position (not player.position, which is only a coarse bucket).
  const naturalPosition = canonicalPosition(player.natural_position || player.position);
  const alternatePositions = (player.alternate_positions || []).map(canonicalPosition);

  // 1. The player's natural position has an exact slot in this formation.
  if (formationSlots.includes(naturalPosition)) {
    return naturalPosition;
  }

  // 2. The closest role within the player's primary position group. This is
  //    preferred over tangential alternates so a DM in a DM-less formation
  //    resolves to CM, never to an unrelated alternate such as RB.
  const playerGroup = normalisePosition(naturalPosition);
  const groupRolePreferences = GROUP_ROLE_PREFERENCES[playerGroup] ?? [];
  const preferredGroupRole = groupRolePreferences.find((position) =>
    formationSlots.includes(position),
  );
  if (preferredGroupRole) {
    return preferredGroupRole;
  }

  // 3. An alternate position the player can play that exists in the formation.
  const exactAlternateRole = alternatePositions.find((position) =>
    formationSlots.includes(position),
  );
  if (exactAlternateRole) {
    return exactAlternateRole;
  }

  // 4. Fallbacks: any slot in the player's group, else first preferred position.
  const firstGroupRole = formationSlots.find(
    (position) => normalisePosition(position) === playerGroup,
  );

  return firstGroupRole ?? getPreferredPositions(player)[0] ?? naturalPosition;
}

// Penalty (in attribute points) applied to the style-fit score based on how
// naturally the player suits their current field position. Tunable.
const STYLE_FIT_POSITION_PENALTY: Record<SquadTacticalFit, number> = {
  natural: 0,
  adapted: 8,
  out: 18,
};

export function getPlayStyleFit(
  player: PlayerData,
  playStyle: string,
  currentPos?: string,
): SquadStyleFit {
  const attributeScore = (() => {
    switch (playStyle) {
      case "Attacking":
        return averageAttributes(player, ["shooting", "dribbling", "pace", "passing"]);
      case "Defensive":
        return averageAttributes(player, ["defending", "tackling", "positioning", "strength"]);
      case "Possession":
        return averageAttributes(player, ["passing", "vision", "decisions", "composure"]);
      case "Counter":
        return averageAttributes(player, ["pace", "dribbling", "passing", "positioning"]);
      case "HighPress":
        return averageAttributes(player, ["stamina", "aggression", "teamwork", "pace", "tackling"]);
      default:
        return averageAttributes(player, ["decisions", "teamwork", "composure", "stamina"]);
    }
  })();

  // A player asked to execute a style out of position is a worse fit regardless
  // of raw attributes (e.g. a striker shoehorned at centre-back). When no
  // position is supplied the score is attribute-only as before.
  const positionPenalty = currentPos
    ? STYLE_FIT_POSITION_PENALTY[getSquadTacticalFit(player, currentPos)]
    : 0;
  const fitScore = attributeScore - positionPenalty;

  if (fitScore >= 72) {
    return "strong";
  }

  if (fitScore >= 58) {
    return "good";
  }

  return "risky";
}

export function buildRoleCoverageSummary(
  availablePlayers: PlayerData[],
  currentXiIds: string[],
  formation: string,
): SquadRoleCoverage[] {
  const slotPositions = buildPitchRows(formation).flatMap((row) => row.positions);
  const requiredByRole = new Map<string, number>();
  const naturalStartersByRole = new Map<string, number>();
  const benchOptionsByRole = new Map<string, number>();
  const xiSet = new Set(currentXiIds);
  const playersById = new Map(
    availablePlayers.map((player) => [player.id, player] as const),
  );

  slotPositions.forEach((role) => {
    requiredByRole.set(role, (requiredByRole.get(role) ?? 0) + 1);
  });

  currentXiIds.forEach((playerId, slotIndex) => {
    const player = playersById.get(playerId);
    const slotPosition = slotPositions[slotIndex];

    if (!player || !slotPosition) {
      return;
    }

    if (getSquadTacticalFit(player, slotPosition) === "natural") {
      naturalStartersByRole.set(
        slotPosition,
        (naturalStartersByRole.get(slotPosition) ?? 0) + 1,
      );
    }
  });

  availablePlayers
    .filter((player) => !xiSet.has(player.id))
    .forEach((player) => {
      const preferredPositions = new Set(getPreferredPositions(player));
      requiredByRole.forEach((_, role) => {
        if (preferredPositions.has(role)) {
          benchOptionsByRole.set(role, (benchOptionsByRole.get(role) ?? 0) + 1);
        }
      });
    });

  return [...requiredByRole.entries()]
    .map(([role, requiredSlots]) => {
      const naturalStarters = naturalStartersByRole.get(role) ?? 0;
      const benchOptions = benchOptionsByRole.get(role) ?? 0;
      const status: SquadRoleCoverageStatus =
        naturalStarters === 0 && benchOptions === 0
          ? "uncovered"
          : naturalStarters < requiredSlots || benchOptions === 0
            ? "thin"
            : "covered";

      return {
        role,
        requiredSlots,
        naturalStarters,
        benchOptions,
        status,
      };
    })
    .sort((leftRole, rightRole) => {
      const statusOrder: Record<SquadRoleCoverageStatus, number> = {
        uncovered: 0,
        thin: 1,
        covered: 2,
      };

      return (
        statusOrder[leftRole.status] - statusOrder[rightRole.status] ||
        leftRole.role.localeCompare(rightRole.role)
      );
    });
}

function getFitScore(player: PlayerData, currentPos: string): number {
  const fit = getSquadTacticalFit(player, currentPos);

  if (fit === "natural") return 2;
  if (fit === "adapted") return 1;
  return 0;
}

export function buildPromoteToStartingXi(
  currentXiIds: string[],
  playersById: Map<string, PlayerData>,
  formation: string,
  playerId: string,
): string[] | null {
  if (currentXiIds.includes(playerId)) {
    return currentXiIds;
  }

  const promotedPlayer = playersById.get(playerId);

  if (!promotedPlayer) {
    return null;
  }

  if (promotedPlayer.injury) {
    return null;
  }

  const slotPositions = buildPitchRows(formation).flatMap((row) => row.positions);

  const targetSlotIndex = slotPositions
    .map((slotPosition, slotIndex) => {
      const incumbentId = currentXiIds[slotIndex];
      const incumbentPlayer = incumbentId ? playersById.get(incumbentId) ?? null : null;

      return {
        incumbentPlayer,
        slotIndex,
        slotPosition,
      };
    })
    .sort((leftSlot, rightSlot) => {
      const promotedLeftScore = getFitScore(promotedPlayer, leftSlot.slotPosition);
      const promotedRightScore = getFitScore(promotedPlayer, rightSlot.slotPosition);
      const incumbentLeftScore = leftSlot.incumbentPlayer
        ? getFitScore(leftSlot.incumbentPlayer, leftSlot.slotPosition)
        : -1;
      const incumbentRightScore = rightSlot.incumbentPlayer
        ? getFitScore(rightSlot.incumbentPlayer, rightSlot.slotPosition)
        : -1;
      const incumbentLeftOvr = leftSlot.incumbentPlayer
        ? getPlayerOvr(leftSlot.incumbentPlayer)
        : -1;
      const incumbentRightOvr = rightSlot.incumbentPlayer
        ? getPlayerOvr(rightSlot.incumbentPlayer)
        : -1;
      const incumbentLeftCondition = leftSlot.incumbentPlayer?.condition ?? -1;
      const incumbentRightCondition = rightSlot.incumbentPlayer?.condition ?? -1;

      return (
        promotedRightScore - promotedLeftScore ||
        incumbentLeftScore - incumbentRightScore ||
        incumbentLeftOvr - incumbentRightOvr ||
        incumbentLeftCondition - incumbentRightCondition ||
        leftSlot.slotIndex - rightSlot.slotIndex
      );
    })[0]?.slotIndex;

  if (targetSlotIndex == null || targetSlotIndex < 0) {
    return null;
  }

  const nextXiIds = [...currentXiIds];
  nextXiIds[targetSlotIndex] = playerId;
  return nextXiIds;
}

export function buildAssignStartingXiSlot(
  currentXiIds: string[],
  playerId: string,
  targetSlotIndex: number,
): string[] | null {
  const currentSlotIndex = currentXiIds.indexOf(playerId);

  if (currentSlotIndex < 0) {
    return null;
  }

  if (targetSlotIndex < 0 || targetSlotIndex >= currentXiIds.length) {
    return currentXiIds;
  }

  if (currentSlotIndex === targetSlotIndex) {
    return currentXiIds;
  }

  const nextXiIds = [...currentXiIds];
  [nextXiIds[currentSlotIndex], nextXiIds[targetSlotIndex]] = [
    nextXiIds[targetSlotIndex],
    nextXiIds[currentSlotIndex],
  ];

  return nextXiIds;
}

export function buildAssignBestFitSlot(
  currentXiIds: string[],
  playersById: Map<string, PlayerData>,
  formation: string,
  playerId: string,
): string[] | null {
  const currentSlotIndex = currentXiIds.indexOf(playerId);

  if (currentSlotIndex < 0) {
    return null;
  }

  const slotPositions = buildPitchRows(formation).flatMap((row) => row.positions);
  const player = playersById.get(playerId);

  if (!player) {
    return null;
  }

  const bestSlotIndex = slotPositions
    .map((slotPosition, slotIndex) => ({
      fitScore: getFitScore(player, slotPosition),
      slotIndex,
    }))
    .sort((leftSlot, rightSlot) => {
      return (
        rightSlot.fitScore - leftSlot.fitScore ||
        leftSlot.slotIndex - rightSlot.slotIndex
      );
    })[0]?.slotIndex;

  if (bestSlotIndex == null) {
    return currentXiIds;
  }

  return buildAssignStartingXiSlot(currentXiIds, playerId, bestSlotIndex);
}

export function buildDemoteFromStartingXi(
  currentXiIds: string[],
  availablePlayers: PlayerData[],
  formation: string,
  playerId: string,
): string[] | null {
  const slotIndex = currentXiIds.indexOf(playerId);

  if (slotIndex < 0) {
    return currentXiIds;
  }

  const slotPosition = buildPitchRows(formation).flatMap((row) => row.positions)[slotIndex];
  const benchCandidates = availablePlayers.filter(
    (player) => !currentXiIds.includes(player.id) && player.id !== playerId,
  );
  const replacement = benchCandidates.sort((leftPlayer, rightPlayer) =>
    comparePlayersForSlot(leftPlayer, rightPlayer, slotPosition),
  )[0];

  if (!replacement) {
    return null;
  }

  const nextXiIds = [...currentXiIds];
  nextXiIds[slotIndex] = replacement.id;
  return nextXiIds;
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
