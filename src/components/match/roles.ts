// Engine-position → valid roles map (keyed by engine Position enum variant names).
// Used in PreMatchSetup role chips and ChangePlayerRole context menu.
export const ROLES_BY_ENGINE_POSITION: Record<string, string[]> = {
  Goalkeeper: ["Standard", "BallPlayingKeeper", "SweeperKeeper"],
  Defender: [
    "Standard", "Stopper", "CoverCB", "BallPlayingCB",
    "AttackingFB", "DefensiveFB", "InvertedFB", "WingBack",
  ],
  Midfielder: [
    "Standard", "AnchorMan", "BallWinner", "DeepLyingPlaymaker",
    "BoxToBox", "Carrilero", "Mezzala", "AdvancedPlaymaker",
    "ShadowStriker", "WideForward", "InsideForward", "InvertedWinger",
  ],
  Forward: [
    "Standard", "Poacher", "TargetMan", "DeepLyingForward",
    "False9", "PressingForward", "CompleteForward",
  ],
};
