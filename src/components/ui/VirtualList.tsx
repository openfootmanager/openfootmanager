import { useRef, type ReactNode, type RefObject } from "react";

import { useListFocus } from "./useListFocus";
import { useVirtualWindow } from "./useVirtualWindow";
import { renderedIndexes } from "./VirtualList.helpers";

export interface VirtualListProps<T> {
  items: readonly T[];
  /** One row's height in px. Every row has it: the window is computed, not measured. */
  rowHeight: number;
  /** The space between rows in px. */
  gap?: number;
  itemKey: (item: T, index: number) => string | number;
  renderItem: (item: T, index: number) => ReactNode;
  /** Names the list for assistive technology. */
  "aria-label": string;
  /** Rows kept rendered beyond the viewport on each side. */
  overscan?: number;
  /**
   * What scrolls: this element, if given; else the nearest ancestor marked `data-scroll-container`;
   * else the page. Ignored when `maxHeight` makes the list its own scroller.
   */
  scrollParent?: RefObject<HTMLElement | null>;
  /** Makes the list its own scroller, at most this tall (px or any CSS length). */
  maxHeight?: number | string;
  /** Scroll this row into view when the value changes (a row a search found, a form opened). */
  revealIndex?: number | null;
  /** Return to the first row when this changes (a new search or filter). */
  resetKey?: unknown;
  /** Enter or Space on a focused row. */
  onActivate?: (index: number) => void;
  className?: string;
}

/**
 * A long list that renders only the rows near the viewport, however many it has.
 *
 * The rows are positioned by arithmetic (`index × (rowHeight + gap)`) inside a container as tall as
 * the whole list, so the scrollbar and the scroll position are those of the full list while the page
 * holds a few dozen elements. The editor used to append fifty rows per "Load more" and never remove
 * any: the full 17,098-player list put 394,000 elements on the page and a 5 GB web view behind it.
 *
 * Keyboard and assistive technology: the list is `role="list"` of `role="listitem"` rows, each
 * announcing its place in the whole list (`aria-posinset` of `aria-setsize`), with one tab stop that
 * follows focus (a roving tabindex). Arrow keys, Home, End, PageUp and PageDown move focus even to a
 * row that is not rendered yet; the focused row stays mounted while it is scrolled away so focus is
 * never dropped. Enter and Space on a row call `onActivate`.
 */
export function VirtualList<T>({
  items,
  rowHeight,
  gap = 8,
  itemKey,
  renderItem,
  "aria-label": ariaLabel,
  overscan = 6,
  scrollParent,
  maxHeight,
  revealIndex,
  resetKey,
  onActivate,
  className,
}: VirtualListProps<T>) {
  const count = items.length;
  const step = rowHeight + gap;
  const listRef = useRef<HTMLDivElement>(null);
  const ownScrollerRef = useRef<HTMLDivElement>(null);

  const { range, revealRow } = useVirtualWindow({
    count,
    step,
    rowHeight,
    overscan,
    listRef,
    ownScrollerRef,
    scrollParent,
    revealIndex,
    resetKey,
  });
  const { focusIndex, activeIndex, onKeyDown, onFocus, onBlur } = useListFocus({
    listRef,
    items,
    itemKey,
    revealRow,
    onActivate,
  });

  const indexes = renderedIndexes(range, count, focusIndex);
  // The tab stop is the active row if it is rendered, else the first rendered row.
  const tabStop = indexes.includes(activeIndex) ? activeIndex : (indexes[0] ?? 0);

  const list = (
    <div
      ref={listRef}
      role="list"
      aria-label={ariaLabel}
      onKeyDown={onKeyDown}
      onFocus={onFocus}
      onBlur={onBlur}
      className={`relative ${className ?? ""}`}
      style={{ height: Math.max(0, count * step - gap) }}
    >
      {indexes.map((index) => (
        <div
          key={itemKey(items[index], index)}
          role="listitem"
          data-vindex={index}
          aria-setsize={count}
          aria-posinset={index + 1}
          tabIndex={index === tabStop ? 0 : -1}
          className="group absolute inset-x-0 rounded-xl outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-page [&>*]:h-full"
          style={{ top: index * step, height: rowHeight }}
        >
          {renderItem(items[index], index)}
        </div>
      ))}
    </div>
  );

  if (maxHeight === undefined) return list;
  return (
    <div ref={ownScrollerRef} className="overflow-y-auto" style={{ maxHeight }}>
      {list}
    </div>
  );
}
