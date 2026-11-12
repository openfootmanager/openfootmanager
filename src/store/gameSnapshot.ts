/** The two numbers a game snapshot is ordered by (`Game::career_token` and `Game::revision`). */
export interface SnapshotOrder {
  career_token?: number;
  revision?: number;
}

export function isSnapshotNotOlder(
  _held: SnapshotOrder | null | undefined,
  _incoming: SnapshotOrder,
): boolean {
  return true;
}
