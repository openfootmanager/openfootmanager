import { useRef, type ReactNode } from "react";

import { useRootFontPx } from "../../hooks/useRootFontPx";
import { useVirtualWindow } from "./useVirtualWindow";
import { renderedIndexes } from "./VirtualList.helpers";

/** The DOM id of option `index` of the listbox `id`, for a search field's `aria-activedescendant`. */
export function listboxOptionId(id: string, index: number): string {
  return `${id}-option-${index}`;
}

export interface OptionState {
  active: boolean;
  selected: boolean;
}

interface VirtualListboxProps<T> {
  /** The listbox's id; option `i` is `listboxOptionId(id, i)`. */
  id: string;
  "aria-label": string;
  items: readonly T[];
  /** One option's height in rem: every option has it, so the window is computed, not measured. */
  rowHeightRem: number;
  /** The tallest the listbox gets before it scrolls (any CSS length). */
  maxHeight: string;
  /** The option the search field's `aria-activedescendant` names: kept scrolled into view and rendered. */
  activeIndex: number;
  itemKey: (item: T, index: number) => string | number;
  isSelected: (item: T) => boolean;
  onPick: (item: T) => void;
  optionClassName: (state: OptionState) => string;
  renderOption: (item: T, index: number) => ReactNode;
  /** Return to the first option when this changes (a new search). */
  resetKey?: unknown;
}

/**
 * The options of a combobox's listbox, rendered a window at a time however many there are (the club
 * picker of a 582-club package put 1,750 elements in the page each time it opened).
 *
 * Focus stays in the combobox's search field, as a combobox needs: this renders the options and takes
 * no keys. The caller moves `activeIndex` and names the active option with `aria-activedescendant`;
 * the active option is always in the window, so that id always names an element. Each option says its
 * place in the whole list (`aria-posinset` of `aria-setsize`).
 */
export function VirtualListbox<T>({
  id,
  "aria-label": ariaLabel,
  items,
  rowHeightRem,
  maxHeight,
  activeIndex,
  itemKey,
  isSelected,
  onPick,
  optionClassName,
  renderOption,
  resetKey,
}: VirtualListboxProps<T>) {
  const count = items.length;
  const rowHeight = rowHeightRem * useRootFontPx();
  const listRef = useRef<HTMLDivElement>(null);
  const scrollerRef = useRef<HTMLDivElement>(null);
  const { range } = useVirtualWindow({
    count,
    step: rowHeight,
    rowHeight,
    overscan: 4,
    listRef,
    ownScrollerRef: scrollerRef,
    revealIndex: count > 0 ? Math.min(activeIndex, count - 1) : null,
    resetKey,
  });
  const indexes = renderedIndexes(range, count, -1);

  return (
    <div
      ref={scrollerRef}
      data-scroll-container=""
      className="overflow-y-auto overscroll-contain"
      style={{ maxHeight }}
    >
      <div
        ref={listRef}
        role="listbox"
        id={id}
        aria-label={ariaLabel}
        className="relative"
        style={{ height: count * rowHeight }}
      >
        {indexes.map((index) => {
          const item = items[index];
          const selected = isSelected(item);
          return (
            <div
              key={itemKey(item, index)}
              id={listboxOptionId(id, index)}
              role="option"
              aria-selected={selected}
              aria-setsize={count}
              aria-posinset={index + 1}
              data-active={index === activeIndex}
              tabIndex={-1}
              onMouseDown={(event) => {
                // Keep focus in the search field; the pick is the click.
                event.preventDefault();
                onPick(item);
              }}
              onClick={(event) => {
                // A click with no pointer behind it is a keyboard activation.
                if (event.detail === 0) onPick(item);
              }}
              className={`absolute inset-x-0 ${optionClassName({ active: index === activeIndex, selected })}`}
              style={{ top: index * rowHeight, height: rowHeight }}
            >
              {renderOption(item, index)}
            </div>
          );
        })}
      </div>
    </div>
  );
}
