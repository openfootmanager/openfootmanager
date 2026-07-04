// Tailwind background class per position, shared by the package editor's
// player list/preview and the tactics pitch badges so they never drift apart:
// amber for keepers, blue for defenders, green for midfielders, red for attack.
export const POSITION_COLOR: Record<string, string> = {
  Goalkeeper: "bg-amber-500",
  Defender: "bg-blue-600",
  CenterBack: "bg-blue-600",
  RightBack: "bg-blue-600",
  LeftBack: "bg-blue-600",
  RightWingBack: "bg-blue-500",
  LeftWingBack: "bg-blue-500",
  Midfielder: "bg-green-600",
  DefensiveMidfielder: "bg-green-700",
  CentralMidfielder: "bg-green-600",
  AttackingMidfielder: "bg-green-500",
  RightMidfielder: "bg-green-600",
  LeftMidfielder: "bg-green-600",
  RightWinger: "bg-red-500",
  LeftWinger: "bg-red-500",
  Forward: "bg-red-600",
  Striker: "bg-red-600",
};

export function getPositionColor(position: string): string {
  return POSITION_COLOR[position] ?? "bg-gray-600";
}
