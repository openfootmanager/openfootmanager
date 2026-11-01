type PlayerWithOvr = {
  ovr?: number | null;
};

export function getPlayerOvr(player: PlayerWithOvr): number {
  return player.ovr ?? 0;
}

/**
 * The average of a squad's ratings, rounded down; 0 for none. The same rule as the backend's
 * `average_rating` (ofm_core `slices::teams::projection`), so the club's own average on Home and on
 * its team card never differ by rounding.
 */
export function averageRating(ratings: number[]): number {
  if (ratings.length === 0) return 0;
  return Math.floor(ratings.reduce((total, rating) => total + rating, 0) / ratings.length);
}
