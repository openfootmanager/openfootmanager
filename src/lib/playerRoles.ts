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
