// The game's position vocabulary, and the only place it is written down.
//
// `domain::player::Position` has four legacy bucket variants (Goalkeeper,
// Defender, Midfielder, Forward) and thirteen granular ones; `is_legacy_bucket`
// on the Rust side exists because the buckets are the older shape. Both reach
// the frontend, so anything ordering or grouping players has to handle both —
// which is why four separate copies of a four-bucket table had drifted into
// components, one of them reading a granular value and silently sorting
// nothing. Sits beside `positionColors.ts`, which keeps the colour half of the
// same vocabulary for the same reason.

export const CORE_POSITIONS = ["Goalkeeper", "Defender", "Midfielder", "Forward"] as const;

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

function normaliseKey(value: string): string {
  return value.toLowerCase().replace(/[^a-z]/g, "");
}

export function canonicalPosition(position: string): string {
  const trimmed = position.trim();
  if (!trimmed) return trimmed;

  return CANONICAL_POSITION_MAP[normaliseKey(trimmed)] || trimmed;
}

export function normalisePosition(position: string): string {
  const canonical = canonicalPosition(position);
  return POSITION_GROUPS[canonical] || canonical;
}

/**
 * Rank a position for canonical sort ordering. Unknown values sort to the end.
 */
export function positionSortRank(position: string): number {
  return POSITION_SORT_ORDER[canonicalPosition(position)] ?? 999;
}

/**
 * Rank a position by its broad line — keepers, then defence, midfield, attack.
 *
 * Normalises first, so a granular value (`CenterBack`) ranks with its group
 * rather than falling off the end. Callers wanting the finer order within a
 * line want {@link positionSortRank} instead.
 */
export function positionGroupRank(position: string): number {
  return POSITION_SORT_ORDER[normalisePosition(position)] ?? 999;
}
