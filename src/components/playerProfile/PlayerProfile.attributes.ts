import type { PlayerData } from "../../store/gameStore";
import { naturalPositionOf } from "../../lib/positions";

export type PlayerAttributeKey = keyof PlayerData["attributes"];
export type AttributeGroupKey = "physical" | "technical" | "mental" | "goalkeeper";

// Single source of truth for the profile display. Every attribute in
// PlayerData["attributes"] must appear here — `satisfies Record<...>` makes
// adding an attribute to the domain a compile-time error until it's grouped.
// Order within each group follows insertion order below (ES2015+ guarantees
// `Object.entries` returns keys in insertion order for string keys).
export const ATTRIBUTE_META = {
  // Physical
  pace: "physical",
  stamina: "physical",
  strength: "physical",
  agility: "physical",
  // Aerial belongs with Physical — engine uses it for header duels for every
  // player, not just GK claims (see engine/resolution.rs & zone_resolution.rs).
  aerial: "physical",

  // Technical
  passing: "technical",
  shooting: "technical",
  tackling: "technical",
  dribbling: "technical",
  defending: "technical",

  // Mental
  positioning: "mental",
  vision: "mental",
  decisions: "mental",
  composure: "mental",
  aggression: "mental",
  teamwork: "mental",
  leadership: "mental",

  // Goalkeeper — only used in save/penalty/zone resolution for GKs. Hidden
  // for outfielders to keep the profile focused on stats that apply.
  handling: "goalkeeper",
  reflexes: "goalkeeper",
} as const satisfies Record<PlayerAttributeKey, AttributeGroupKey>;

export const GROUP_LABEL_KEY: Record<AttributeGroupKey, string> = {
  physical: "common.attrGroups.physical",
  technical: "common.attrGroups.technical",
  mental: "common.attrGroups.mental",
  goalkeeper: "common.attrGroups.goalkeeper",
};

// Shared with PlayerProfile.tsx / the radar chart so attribute grouping and
// radar keys agree on who is a goalkeeper. Prefers natural_position (same as
// primaryPosition in PlayerProfile) so temporary re-deployments don't hide GK
// attributes from a natural keeper's card.
export function isGoalkeeper(player: PlayerData): boolean {
  return naturalPositionOf(player) === "Goalkeeper";
}
