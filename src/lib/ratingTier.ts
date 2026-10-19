import type { RatingTier } from "../services/squadService";

/** Where a tier's colour lands: a number's text, a bar's fill, a pill with its own background, or a
 *  tinted avatar frame. */
export type RatingTierRole = "text" | "bar" | "badge" | "hero";

/**
 * The ONLY place a rating's colour is chosen. Which tier a rating is in is the backend's
 * (`useRatingTiers`); this says how a tier looks, so no screen keeps a cut-off or a colour of its
 * own. The design system's rated-value tones do the painting (`tone-top` … `tone-fair`), which
 * change value in dark mode and high contrast by themselves; the rest is plain and muted.
 */
const TIER_CLASSES: Record<RatingTier, Record<RatingTierRole, string>> = {
  S: {
    text: "text-tone-top",
    bar: "bg-tone-top-solid",
    badge: "bg-tone-top-tint text-tone-top",
    hero: "bg-tone-top-tint text-tone-top border-tone-top-solid",
  },
  A: {
    text: "text-tone-good",
    bar: "bg-tone-good-solid",
    badge: "bg-tone-good-tint text-tone-good",
    hero: "bg-tone-good-tint text-tone-good border-tone-good-solid",
  },
  B: {
    text: "text-tone-fair",
    bar: "bg-tone-fair-solid",
    badge: "bg-tone-fair-tint text-tone-fair",
    hero: "bg-tone-fair-tint text-tone-fair border-tone-fair-solid",
  },
  Rest: {
    text: "text-fg-muted",
    bar: "bg-line-strong",
    badge: "bg-sunken text-fg-muted",
    hero: "bg-sunken text-fg-muted border-line",
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
