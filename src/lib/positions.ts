// How the frontend reads a position it was handed: the spelling (aliases, case, spacing) and which
// of a player's two position fields to show. What a position belongs to, and where it sorts, is the
// backend's: `get_position_groups`, read through `usePositionTable` / `createPositionTable`. Sits
// beside `positionColors.ts`, which keeps the colour half of the vocabulary.

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

function normaliseKey(value: string): string {
  return value.toLowerCase().replace(/[^a-z]/g, "");
}

/**
 * The position that describes the player themselves, for display and grouping.
 *
 * `natural_position` carries the granular value; `position` can still be the legacy bucket on a
 * player who has not been upgraded, and is the only one present in an old save.
 */
export function naturalPositionOf(player: { position: string; natural_position?: string }): string {
  return player.natural_position || player.position;
}

export function canonicalPosition(position: string): string {
  const trimmed = position.trim();
  if (!trimmed) return trimmed;

  return CANONICAL_POSITION_MAP[normaliseKey(trimmed)] || trimmed;
}
