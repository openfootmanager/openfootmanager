/**
 * The penalty order a manager sets before a shootout, as a list of player ids. The engine fixes how
 * many of them kick; reordering only decides who, and in what order. Every function returns the
 * very same array when nothing moves, so a state update that changes nothing re-renders nothing.
 */

/** The order with the player at `from` moved to `to`, the players in between closing up. */
export function moveKicker(order: readonly string[], from: number, to: number): readonly string[] {
  if (from === to || from < 0 || to < 0 || from >= order.length || to >= order.length) {
    return order;
  }
  const moved = [...order];
  const [player] = moved.splice(from, 1);
  moved.splice(to, 0, player);
  return moved;
}

/** The order with a lifted player returned to the place he was lifted from. */
export function putBack(order: readonly string[], playerId: string, origin: number) {
  return moveKicker(order, order.indexOf(playerId), origin);
}

/** Whether the order differs from the one the engine first gave. */
export function isOrderChanged(initial: readonly string[], current: readonly string[]): boolean {
  return initial.length !== current.length || initial.some((id, index) => current[index] !== id);
}

/** The kick a place in the order takes (1-based), or null for a player who sits out. */
export function placeInOrder(index: number, kickers: number): { kick: number } | null {
  return index < kickers ? { kick: index + 1 } : null;
}
