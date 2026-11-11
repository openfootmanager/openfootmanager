import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type FocusEvent,
  type KeyboardEvent,
  type ReactNode,
  type RefObject,
} from "react";

import { scrollTopToReveal, windowRange, type RowWindow } from "./VirtualList.helpers";

/** The height assumed for a scroller that reports none (a test renderer has no layout). */
const FALLBACK_VIEWPORT = 800;
/** How many rows PageUp and PageDown move. */
const PAGE_ROWS = 10;

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
  const frame = useRef<number | null>(null);
  const pendingFocus = useRef<number | null>(null);

  const [range, setRange] = useState<RowWindow>(() =>
    windowRange({
      count,
      step,
      scrollTop: 0,
      listTop: 0,
      viewportHeight: FALLBACK_VIEWPORT,
      overscan,
    }),
  );
  // The row that holds DOM focus (kept mounted), and the row that is the list's one tab stop.
  const [focusIndex, setFocusIndex] = useState<number | null>(null);
  const [activeIndex, setActiveIndex] = useState(0);

  /** What scrolls, and where the list sits in it. */
  const metrics = useCallback(() => {
    const list = listRef.current;
    if (!list) return null;
    const own = ownScrollerRef.current;
    const scroller =
      own ?? scrollParent?.current ?? list.closest<HTMLElement>("[data-scroll-container]") ?? null;
    const page = scroller ?? (document.scrollingElement as HTMLElement | null);
    const scrollTop = page?.scrollTop ?? 0;
    const viewportHeight =
      (scroller ? scroller.clientHeight : window.innerHeight) || FALLBACK_VIEWPORT;
    const listTop = own
      ? 0
      : list.getBoundingClientRect().top -
        (scroller ? scroller.getBoundingClientRect().top : 0) +
        scrollTop;
    return { scroller, page, scrollTop, viewportHeight, listTop };
  }, [scrollParent]);

  const measure = useCallback(() => {
    const m = metrics();
    if (!m) return;
    const next = windowRange({
      count,
      step,
      scrollTop: m.scrollTop,
      listTop: m.listTop,
      viewportHeight: m.viewportHeight,
      overscan,
    });
    setRange((prev) => (prev.start === next.start && prev.end === next.end ? prev : next));
  }, [metrics, count, step, overscan]);

  const revealRow = useCallback(
    (index: number) => {
      const m = metrics();
      if (!m || !m.page) return;
      const to = scrollTopToReveal({
        index,
        step,
        rowHeight,
        listTop: m.listTop,
        scrollTop: m.scrollTop,
        viewportHeight: m.viewportHeight,
      });
      if (to !== null) m.page.scrollTop = to;
      measure();
    },
    [metrics, step, rowHeight, measure],
  );

  // The list or its rows changed size: take the window again.
  useLayoutEffect(() => {
    measure();
  }, [measure]);

  // Follow the scroller.
  useEffect(() => {
    const m = metrics();
    const target: HTMLElement | Window = m?.scroller ?? window;
    const schedule = () => {
      if (frame.current !== null) return;
      frame.current = requestAnimationFrame(() => {
        frame.current = null;
        measure();
      });
    };
    target.addEventListener("scroll", schedule, { passive: true });
    window.addEventListener("resize", schedule);
    const observer =
      typeof ResizeObserver === "undefined" || !m?.scroller ? null : new ResizeObserver(schedule);
    if (observer && m?.scroller) observer.observe(m.scroller);
    return () => {
      target.removeEventListener("scroll", schedule);
      window.removeEventListener("resize", schedule);
      observer?.disconnect();
      if (frame.current !== null) cancelAnimationFrame(frame.current);
      frame.current = null;
    };
  }, [metrics, measure]);

  // A row to bring into view.
  useLayoutEffect(() => {
    if (revealIndex === null || revealIndex === undefined || revealIndex >= count) return;
    revealRow(revealIndex);
    // Only a new target moves the list: a re-render with the same one must not fight the user's scroll.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [revealIndex]);

  // What the list shows changed: back to its first row.
  const firstReset = useRef(true);
  useLayoutEffect(() => {
    if (firstReset.current) {
      firstReset.current = false;
      return;
    }
    const m = metrics();
    if (m?.page && m.scrollTop > m.listTop) m.page.scrollTop = m.listTop;
    measure();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [resetKey]);

  // Focus the row a key press asked for, once it is rendered.
  useLayoutEffect(() => {
    const wanted = pendingFocus.current;
    if (wanted === null) return;
    const row = listRef.current?.querySelector<HTMLElement>(`[data-vindex="${wanted}"]`);
    if (row) {
      pendingFocus.current = null;
      row.focus({ preventScroll: true });
    }
  });

  function moveFocusTo(index: number) {
    const clamped = Math.min(count - 1, Math.max(0, index));
    pendingFocus.current = clamped;
    setActiveIndex(clamped);
    revealRow(clamped);
  }

  function handleKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    const target = event.target as HTMLElement;
    // Only a key pressed on the row itself: inside a control in the row it is that control's.
    if (!target.hasAttribute("data-vindex")) return;
    const index = Number(target.dataset.vindex);
    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        moveFocusTo(index + 1);
        break;
      case "ArrowUp":
        event.preventDefault();
        moveFocusTo(index - 1);
        break;
      case "PageDown":
        event.preventDefault();
        moveFocusTo(index + PAGE_ROWS);
        break;
      case "PageUp":
        event.preventDefault();
        moveFocusTo(index - PAGE_ROWS);
        break;
      case "Home":
        event.preventDefault();
        moveFocusTo(0);
        break;
      case "End":
        event.preventDefault();
        moveFocusTo(count - 1);
        break;
      case "Enter":
      case " ":
        event.preventDefault();
        onActivate?.(index);
        break;
      default:
    }
  }

  function handleFocus(event: FocusEvent<HTMLDivElement>) {
    const row = (event.target as HTMLElement).closest<HTMLElement>("[data-vindex]");
    if (!row) return;
    const index = Number(row.dataset.vindex);
    setFocusIndex(index);
    setActiveIndex(index);
  }

  function handleBlur(event: FocusEvent<HTMLDivElement>) {
    if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setFocusIndex(null);
  }

  // The window, plus the focused row if it has been scrolled out of it.
  const indexes: number[] = [];
  const keepFocused = focusIndex !== null && focusIndex < count && (focusIndex < range.start || focusIndex >= range.end);
  if (keepFocused && focusIndex < range.start) indexes.push(focusIndex);
  for (let i = range.start; i < Math.min(range.end, count); i += 1) indexes.push(i);
  if (keepFocused && focusIndex >= range.end) indexes.push(focusIndex);

  // The tab stop is the active row if it is rendered, else the first rendered row.
  const tabStop = indexes.includes(activeIndex) ? activeIndex : (indexes[0] ?? 0);

  const list = (
    <div
      ref={listRef}
      role="list"
      aria-label={ariaLabel}
      onKeyDown={handleKeyDown}
      onFocus={handleFocus}
      onBlur={handleBlur}
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
          className="absolute inset-x-0 rounded-xl outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-page [&>*]:h-full"
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
