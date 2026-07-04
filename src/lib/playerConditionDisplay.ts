/**
 * Shared tailwind text-color class for a `player.condition` value.
 *
 * Kept in one place so the same condition value looks the same in every
 * table it appears (pre-match XI / substitutes, training-groups roster).
 */
export function condColor(condition: number): string {
  if (condition >= 75) return "text-primary-400";
  if (condition >= 50) return "text-amber-400";
  return "text-red-400";
}
