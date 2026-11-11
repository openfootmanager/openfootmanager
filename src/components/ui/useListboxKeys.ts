import { useState, type KeyboardEvent } from "react";

interface Options {
  /** Options in the listbox (all of them, not just the rendered window). */
  count: number;
  /** Enter on the active option. */
  onPick: (index: number) => void;
  /** Escape. */
  onEscape: () => void;
}

/**
 * The keys of a combobox's search field over its listbox: arrows move the active option and wrap at
 * both ends, Home and End jump, Enter picks the active option, Escape closes. Only the index is kept
 * here; the listbox renders it and the field names it with `aria-activedescendant`.
 */
export function useListboxKeys({ count, onPick, onEscape }: Options) {
  const [requested, setActive] = useState(0);
  // The list can shrink under the active option (the search narrowed it).
  const active = Math.min(requested, Math.max(0, count - 1));

  function onKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key === "Escape") {
      event.preventDefault();
      onEscape();
      return;
    }
    if (count === 0) return;
    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        setActive((active + 1) % count);
        break;
      case "ArrowUp":
        event.preventDefault();
        setActive((active - 1 + count) % count);
        break;
      case "Home":
        event.preventDefault();
        setActive(0);
        break;
      case "End":
        event.preventDefault();
        setActive(count - 1);
        break;
      case "Enter":
        event.preventDefault();
        onPick(active);
        break;
      default:
    }
  }

  return { active, setActive, onKeyDown };
}
