import type { PlayerRole } from "../store/types";

export const ROLE_OPTIONS_BY_POSITION: Record<string, PlayerRole[]> = {
  Goalkeeper: ["Standard", "BallPlayingKeeper", "SweeperKeeper"],
  CenterBack: ["Standard", "Stopper", "CoverCB", "BallPlayingCB"],
  RightBack: ["Standard", "AttackingFB", "DefensiveFB", "InvertedFB", "WingBack"],
  LeftBack: ["Standard", "AttackingFB", "DefensiveFB", "InvertedFB", "WingBack"],
  RightWingBack: ["Standard", "WingBack", "AttackingFB"],
  LeftWingBack: ["Standard", "WingBack", "AttackingFB"],
  DefensiveMidfielder: ["Standard", "AnchorMan", "BallWinner", "DeepLyingPlaymaker", "BoxToBox"],
  CentralMidfielder: ["Standard", "BoxToBox", "Carrilero", "Mezzala", "DeepLyingPlaymaker", "AdvancedPlaymaker"],
  RightMidfielder: ["Standard", "BoxToBox", "Carrilero", "Mezzala", "WideForward", "InsideForward"],
  LeftMidfielder: ["Standard", "BoxToBox", "Carrilero", "Mezzala", "WideForward", "InsideForward"],
  AttackingMidfielder: ["Standard", "AdvancedPlaymaker", "ShadowStriker", "Mezzala"],
  RightWinger: ["Standard", "WideForward", "InsideForward", "InvertedWinger"],
  LeftWinger: ["Standard", "WideForward", "InsideForward", "InvertedWinger"],
  Striker: ["Standard", "Poacher", "TargetMan", "DeepLyingForward", "False9", "PressingForward", "CompleteForward"],
  // Broad outfield categories (reached via canonicalised/legacy positions like
  // "Defender"/"Midfielder"/"Forward") map to the union of their detailed roles,
  // so e.g. a generic Defender is never offered goalkeeper or striker roles.
  Defender: [
    "Standard", "Stopper", "CoverCB", "BallPlayingCB",
    "AttackingFB", "DefensiveFB", "InvertedFB", "WingBack",
  ],
  Midfielder: [
    "Standard", "AnchorMan", "BallWinner", "DeepLyingPlaymaker",
    "BoxToBox", "Carrilero", "Mezzala", "AdvancedPlaymaker", "ShadowStriker",
    "WideForward", "InsideForward",
  ],
  Forward: [
    "Standard", "WideForward", "InsideForward", "InvertedWinger",
    "Poacher", "TargetMan", "DeepLyingForward", "False9", "PressingForward", "CompleteForward",
  ],
};

const ALL_ROLES: PlayerRole[] = [
  "Standard", "BallPlayingKeeper", "SweeperKeeper",
  "Stopper", "CoverCB", "BallPlayingCB",
  "AttackingFB", "DefensiveFB", "InvertedFB", "WingBack",
  "AnchorMan", "BallWinner", "DeepLyingPlaymaker",
  "BoxToBox", "Carrilero", "Mezzala",
  "AdvancedPlaymaker", "ShadowStriker",
  "WideForward", "InsideForward", "InvertedWinger",
  "Poacher", "TargetMan", "DeepLyingForward", "False9", "PressingForward", "CompleteForward",
];

export function getRolesForPosition(position: string): PlayerRole[] {
  return ROLE_OPTIONS_BY_POSITION[position] ?? ALL_ROLES;
}

/**
 * Role options for a position that always include `currentRole`. A player's role
 * is a single per-player value that may have been assigned from a different
 * (slot or legacy/broad) position, so it can fall outside this position's option
 * set; including it keeps a controlled <select> in sync instead of silently
 * displaying — and on edit, overwriting with — the wrong role.
 */
export function getRoleOptions(position: string, currentRole?: string | null): PlayerRole[] {
  const base = getRolesForPosition(position);
  if (currentRole && !base.includes(currentRole as PlayerRole)) {
    return [currentRole as PlayerRole, ...base];
  }
  return base;
}
