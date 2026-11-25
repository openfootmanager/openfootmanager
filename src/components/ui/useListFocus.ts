import {
  useLayoutEffect,
  useReducer,
  useRef,
  useState,
  type FocusEvent,
  type KeyboardEvent,
  type RefObject,
} from "react";

/** How many rows PageUp and PageDown move. */
const PAGE_ROWS = 10;

type Key = string | number;

interface Options<T> {
  listRef: RefObject<HTMLDivElement | null>;
  items: readonly T[];
  itemKey: (item: T, index: number) => Key;
  /** Bring a row into view (it may not be rendered yet). */
  revealRow: (index: number) => void;
  onActivate?: (index: number) => void;
}

/**
 * Keyboard and focus for a windowed list of rows.
 *
 * Focus is tracked by the row's **key**, not its index: rows are inserted and removed above the one
 * the user is on (a duplicate, an undo), and an index would then name a different record, hand the
 * focus to it, and unmount the node that has it. The key is resolved to an index on every render, so
 * the focused record stays where it is and stays mounted, wherever the list now puts it.
 *
 * One row is the list's tab stop (a roving tabindex) and follows focus; arrow keys, Home, End,
 * PageUp and PageDown move it, even to a row that is not rendered yet (the list scrolls it in, then it
 * is focused); Enter and Space on a row call `onActivate`. A key pressed inside a control in the row
 * is that control's.
 */
export function useListFocus<T>({ listRef, items, itemKey, revealRow, onActivate }: Options<T>) {
  const count = items.length;
  const [focusKey, setFocusKey] = useState<Key | null>(null);
  const [activeKey, setActiveKey] = useState<Key | null>(null);
  const pendingKey = useRef<Key | null>(null);
  const [, render] = useReducer((n: number) => n + 1, 0);
  // The index a key had last time: almost always still right, so resolving it is a comparison.
  const hints = useRef(new Map<Key, number>());

  const keyAt = (index: number) => itemKey(items[index], index);

  const indexOfKey = (key: Key | null): number => {
    if (key === null) return -1;
    const hint = hints.current.get(key);
    if (hint !== undefined && hint < count && keyAt(hint) === key) return hint;
    for (let i = 0; i < count; i += 1) {
      if (keyAt(i) === key) {
        hints.current.set(key, i);
        return i;
      }
    }
    hints.current.delete(key);
    return -1;
  };

  const focusIndex = indexOfKey(focusKey);
  const activeIndex = indexOfKey(activeKey);

  // Focus the row a key press asked for, once it is rendered.
  useLayoutEffect(() => {
    const wanted = pendingKey.current;
    if (wanted === null) return;
    const index = indexOfKey(wanted);
    const row =
      index < 0 ? null : listRef.current?.querySelector<HTMLElement>(`[data-vindex="${index}"]`);
    if (row) {
      pendingKey.current = null;
      row.focus({ preventScroll: true });
    }
  });

  function moveFocusTo(index: number) {
    const clamped = Math.min(count - 1, Math.max(0, index));
    const key = keyAt(clamped);
    pendingKey.current = key;
    setActiveKey(key);
    revealRow(clamped);
    // The key may already be the active one: still render, so the focus above is applied.
    render();
  }

  function onKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    const target = event.target as HTMLElement;
    if (!target.hasAttribute("data-vindex")) return;
    const index = Number(target.dataset.vindex);
    const moves: Record<string, number> = {
      ArrowDown: index + 1,
      ArrowUp: index - 1,
      PageDown: index + PAGE_ROWS,
      PageUp: index - PAGE_ROWS,
      Home: 0,
      End: count - 1,
    };
    if (event.key in moves) {
      event.preventDefault();
      moveFocusTo(moves[event.key]);
    } else if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      onActivate?.(index);
    }
  }

  function onFocus(event: FocusEvent<HTMLDivElement>) {
    const row = (event.target as HTMLElement).closest<HTMLElement>("[data-vindex]");
    if (!row) return;
    const key = keyAt(Number(row.dataset.vindex));
    setFocusKey(key);
    setActiveKey(key);
  }

  function onBlur(event: FocusEvent<HTMLDivElement>) {
    if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setFocusKey(null);
  }

  return { focusIndex, activeIndex, onKeyDown, onFocus, onBlur };
}
