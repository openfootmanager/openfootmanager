import type { RatingTier } from "../services/squadService";

/** Where a tier's colour lands: a number's text, a bar's fill, a pill with its own background, or a
 *  tinted avatar frame. */
export type RatingTierRole = "text" | "bar" | "badge" | "hero";

/**
 * The ONLY place a rating's colour is chosen. Which tier a rating is in is the backend's
 * (`useRatingTiers`); this says how a tier looks, so no screen keeps a cut-off or a colour of its
 * own. S is gold, A the primary green, B a firm neutral, and the rest is muted.
 */
const TIER_CLASSES: Record<RatingTier, Record<RatingTierRole, string>> = {
  S: {
    text: "text-accent-500 dark:text-accent-400",
    bar: "bg-accent-500",
    badge: "bg-accent-500 text-navy-900",
    hero: "bg-accent-500/20 text-accent-400 border-accent-500/30",
  },
  A: {
    text: "text-primary-500 dark:text-primary-400",
    bar: "bg-primary-500",
    badge: "bg-primary-500 text-white",
    hero: "bg-primary-500/20 text-primary-400 border-primary-500/30",
  },
  B: {
    text: "text-gray-600 dark:text-gray-300",
    bar: "bg-gray-400 dark:bg-gray-500",
    badge: "bg-gray-200 text-gray-700 dark:bg-navy-600 dark:text-gray-200",
    hero: "bg-gray-500/20 text-gray-300 border-gray-500/30",
  },
  Rest: {
    text: "text-gray-400 dark:text-gray-500",
    bar: "bg-gray-300 dark:bg-navy-600",
    badge: "bg-gray-100 text-gray-500 dark:bg-navy-700 dark:text-gray-400",
    hero: "bg-gray-500/10 text-gray-400 border-gray-500/20",
  },
};

export function ratingTierClass(tier: RatingTier, role: RatingTierRole): string {
  return TIER_CLASSES[tier][role];
}

/** The tier's letter for a chip or caption; the rest has none, the number alone says enough. */
export function ratingTierLetter(tier: RatingTier): "S" | "A" | "B" | null {
  return tier === "Rest" ? null : tier;
}

/** Whether a tier is one of the two the game calls elite (S and A). */
export function isEliteTier(tier: RatingTier): boolean {
  return tier === "S" || tier === "A";
}
