import type { RatedTone } from "../../store/types";

/**
 * The colour classes for a step on the backend's good-to-bad scale. The backend picks the step
 * (a condition band, a tier); these only say how each step is painted, so no screen compares a
 * number against a threshold of its own.
 */
export const TONE_TEXT: Record<RatedTone, string> = {
  top: "text-tone-top",
  good: "text-tone-good",
  fair: "text-tone-fair",
  low: "text-tone-low",
  bad: "text-tone-bad",
};

/** A bar or dot filled in a step of the scale. */
export const TONE_FILL: Record<RatedTone, string> = {
  top: "bg-tone-top-solid",
  good: "bg-tone-good-solid",
  fair: "bg-tone-fair-solid",
  low: "bg-tone-low-solid",
  bad: "bg-tone-bad-solid",
};
