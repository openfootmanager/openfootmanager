import type { PlayerData } from "../../store/gameStore";

export interface SetPieceStats {
  score: number;
  stats: { label: string; value: number }[];
}

export function getSetPieceStatAttributeKey(label: string): string | null {
  switch (label) {
    case "SHO":
      return "shooting";
    case "COM":
      return "composure";
    case "PAS":
      return "passing";
    case "VIS":
      return "vision";
    case "LDR":
      return "leadership";
    case "TMW":
      return "teamwork";
    default:
      return null;
  }
}

export function getSetPieceStatColorClassName(value: number): string {
  if (value >= 70) {
    return "text-primary-300";
  }

  if (value >= 50) {
    return "text-gray-100";
  }

  return "text-gray-400";
}

export function getSetPieceStats(
  role: string,
  player: PlayerData,
): SetPieceStats {
  const attributes = player.attributes;

  switch (role) {
    case "penalty":
      return {
        score: Math.round((attributes.shooting + attributes.composure) / 2),
        stats: [
          { label: "SHO", value: attributes.shooting },
          { label: "COM", value: attributes.composure },
        ],
      };
    case "freekick":
      return {
        score: Math.round(
          (attributes.passing + attributes.vision + attributes.shooting / 2) /
            2.5,
        ),
        stats: [
          { label: "PAS", value: attributes.passing },
          { label: "VIS", value: attributes.vision },
          { label: "SHO", value: attributes.shooting },
        ],
      };
    case "corner":
      return {
        score: Math.round((attributes.passing + attributes.vision) / 2),
        stats: [
          { label: "PAS", value: attributes.passing },
          { label: "VIS", value: attributes.vision },
        ],
      };
    case "captain":
    case "vicecaptain":
      return {
        score: Math.round((attributes.leadership + attributes.teamwork) / 2),
        stats: [
          { label: "LDR", value: attributes.leadership },
          { label: "TMW", value: attributes.teamwork },
        ],
      };
    default:
      return { score: 0, stats: [] };
  }
}

export function roleAllowsGoalkeeper(role: string): boolean {
  return role === "captain" || role === "vicecaptain";
}