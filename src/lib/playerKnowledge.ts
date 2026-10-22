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

/**
 * His potential is a scout's opinion: the club has one for its own player and for a player it has
 * scouted. Knowing a rival's rating from sharing his division is not enough.
 */
function knowsPotential(knowledge: PlayerKnowledge): boolean {
  return knowledge.kind === "own" || knowledge.kind === "scouted";
}

/** The traits the club may see: a potential-derived one only when it has a view of his potential. */
export function visibleTraits(traits: string[], knowledge: PlayerKnowledge): string[] {
  return knowsPotential(knowledge)
    ? traits
    : traits.filter((trait) => !POTENTIAL_DERIVED_TRAITS.includes(trait));
}
