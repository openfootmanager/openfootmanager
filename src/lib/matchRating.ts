// Mirrors ofm_core::match_rating::is_rated: the backend stores 0 for "no rating computed".
export function isRatedMatch(rating: number): boolean {
  return Number.isFinite(rating) && rating > 0;
}
