/** The two numbers a game snapshot is ordered by (`Game::career_token` and `Game::revision`). */
export interface SnapshotOrder {
  career_token?: number;
  revision?: number;
}

/**
 * Whether `incoming` may replace `held`: it is of a later career, or of the same career at a
 * revision that is not older. A refetch started before a newer game was shown can resolve after it;
 * this is what keeps the older snapshot out (#1394).
 *
 * The career token is handed out by the backend, one higher for every game it installs, so it is
 * monotonic for as long as the app runs, and the store starts empty again on a reload: nothing is
 * held then, and any snapshot applies. A revision counts the writes of ONE career and starts at 0
 * for the next, so it is only ever compared within the same token. Equal revisions are the same
 * game, so they apply (a repeated refetch is idempotent). A number a state does not carry (one
 * built before the fields existed, a test stub) reads as 0.
 */
export function isSnapshotNotOlder(
  held: SnapshotOrder | null | undefined,
  incoming: SnapshotOrder,
): boolean {
  if (!held) return true;
  const heldToken = held.career_token ?? 0;
  const incomingToken = incoming.career_token ?? 0;
  if (incomingToken !== heldToken) return incomingToken > heldToken;
  return (incoming.revision ?? 0) >= (held.revision ?? 0);
}
