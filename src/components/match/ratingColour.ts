type RatingBand = "unrated" | "excellent" | "good" | "average" | "poor" | "bad";

/** One set of thresholds, so a mark reads as the same band in every surface that colours it. */
function ratingBand(unrounded: number | null): RatingBand {
  if (unrounded === null) return "unrated";
  // Banded on the one-decimal mark players see: 5.96 shows as "6.0", so it must
  // not take the colour of a 5.x.
  const rating = Number(unrounded.toFixed(1));
  if (rating >= 8) return "excellent";
  if (rating >= 7) return "good";
  if (rating >= 6) return "average";
  if (rating >= 5) return "poor";
  return "bad";
}

const TEXT_TONE: Record<RatingBand, string> = {
  unrated: "text-gray-400 dark:text-gray-500",
  excellent: "text-accent-700 dark:text-accent-400",
  good: "text-green-700 dark:text-green-400",
  average: "text-gray-600 dark:text-gray-300",
  poor: "text-yellow-700 dark:text-yellow-400",
  bad: "text-red-700 dark:text-red-400",
};

// Dark enough fills for white text. No dark: variants: the pitch they sit on
// looks the same in both themes.
const BUBBLE_FILL: Record<RatingBand, string> = {
  unrated: "bg-gray-500",
  excellent: "bg-accent-700",
  good: "bg-primary-700",
  average: "bg-gray-600",
  poor: "bg-orange-700",
  bad: "bg-red-700",
};

/** Text tone for an engine mark out of ten; `null` is a player not rated yet. */
export function ratingColour(rating: number | null): string {
  return TEXT_TONE[ratingBand(rating)];
}

/** Fill for a mark's bubble on the pitch, which always carries white text. */
export function ratingBubbleColour(rating: number | null): string {
  return BUBBLE_FILL[ratingBand(rating)];
}
