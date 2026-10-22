import type { PlayerKnowledge } from "../services/scoutingService";

/** The club sees his live rating and attributes: he is its own, or plays in its division. */
export function knowsPlayerRating(knowledge: PlayerKnowledge): boolean {
  return knowledge.kind === "own" || knowledge.kind === "known";
}

/** Condition, morale and the exact wage are his club's business: only the club's own player shows them. */
export function knowsPrivateState(knowledge: PlayerKnowledge): boolean {
  return knowledge.kind === "own";
}

// Traits the backend derives from potential rather than from attributes (domain `PlayerTrait`,
// `ofm_core::player_rating::qualifies_for_wonderkid`): showing one tells the potential.
const POTENTIAL_DERIVED_TRAITS: readonly string[] = ["Wonderkid"];

/** The traits the club may see: a potential-derived one only when it knows his rating. */
export function visibleTraits(traits: string[], knowledge: PlayerKnowledge): string[] {
  return knowsPlayerRating(knowledge)
    ? traits
    : traits.filter((trait) => !POTENTIAL_DERIVED_TRAITS.includes(trait));
}
