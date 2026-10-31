import type { PlayerKnowledge } from "../services/scoutingService";

/** What the club's knowledge of players, and the wages it may see, are asked again on. */
export interface KnowledgeRevisionInputs {
  /** The career this game was installed as: another career's answers are never kept. */
  careerToken?: number;
  date: string;
  /** The club the players are at: the profile's player's, or the roster's. */
  playerClubId?: string | null;
  managerClubId?: string | null;
  /** The season's start: a rollover moves clubs between divisions, and so who is known, without
   *  moving the date. */
  seasonStart?: string | null;
  /** A profile's player's wage, last, so a renewal moves the key; none for a roster. */
  wage?: number;
}

/** The key the club's knowledge and wage answers are asked again on when it changes. */
export function knowledgeRevision(inputs: KnowledgeRevisionInputs): string {
  // JSON keeps the fields apart: a package's club ids may hold any character, a separator too.
  return JSON.stringify([
    inputs.careerToken ?? 0,
    inputs.date,
    inputs.playerClubId ?? "",
    inputs.managerClubId ?? "",
    inputs.seasonStart ?? "",
    inputs.wage ?? 0,
  ]);
}

/**
 * A rating as a list or a card writes it: the number when the club knows it, "–" when it does not.
 * The one place a hidden rating is spelled, so every screen hides it alike.
 */
export function visibleRatingText(rating: number | null): string {
  return rating === null ? "–" : String(rating);
}

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
