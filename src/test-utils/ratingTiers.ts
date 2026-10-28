import type { RatingTier } from "../services/squadService";

/**
 * Stands in for the backend's rating-tier table in component tests, which mock `invoke` and so
 * never reach Rust. It cuts where `ofm_core::player_tier` does (70, 77, 81); the rule itself is
 * tested in Rust, and the hook that reads the real table in `useRatingTiers.test.ts`.
 */
export function cannedRatingTier(rating: number): RatingTier {
  if (rating >= 81) return "S";
  if (rating >= 77) return "A";
  if (rating >= 70) return "B";
  return "Rest";
}

/** The module a test passes to `vi.mock("…/hooks/useRatingTiers", …)`. */
export function cannedRatingTiersModule(): {
  useRatingTiers: () => (rating: number) => RatingTier;
  resetRatingTiersCache: () => void;
} {
  return { useRatingTiers: () => cannedRatingTier, resetRatingTiersCache: () => {} };
}
