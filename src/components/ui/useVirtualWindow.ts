import { useCallback, useEffect, useLayoutEffect, useRef, useState, type RefObject } from "react";

import { scrollTopToReveal, windowRange, type RowWindow } from "./VirtualList.helpers";

/** The height assumed for a scroller that reports none (a renderer with no layout). */
const FALLBACK_VIEWPORT = 800;

interface Options {
  count: number;
  /** Distance from one row's top to the next. */
  step: number;
  rowHeight: number;
  overscan: number;
  listRef: RefObject<HTMLDivElement | null>;
  /** Set when the list scrolls itself (it is the scroller's only content). */
  ownScrollerRef: RefObject<HTMLDivElement | null>;
  scrollParent?: RefObject<HTMLElement | null>;
  /** A row to bring into view whenever this changes. */
  revealIndex?: number | null;
  /** Return to the first row whenever this changes. */
  resetKey?: unknown;
}

/**
 * The window of rows to render for a list inside a scroller: which rows are near the viewport, kept
 * up to date as the scroller scrolls or resizes, plus the two ways the list moves the scroller itself
 * (reveal a row, return to the top). Rendering and focus are the caller's.
 */
export function useVirtualWindow({
  count,
  step,
  rowHeight,
  overscan,
  listRef,
  ownScrollerRef,
  scrollParent,
  revealIndex,
  resetKey,
}: Options) {
  const frame = useRef<number | null>(null);
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
    // The list's top within the scroller's content: its distance from the scroller's top edge on
    // screen, plus how far the scroller has scrolled.
    const listTop = own
      ? 0
      : list.getBoundingClientRect().top -
        (scroller ? scroller.getBoundingClientRect().top : 0) +
        scrollTop;
    return { scroller, page, scrollTop, viewportHeight, listTop };
  }, [listRef, ownScrollerRef, scrollParent]);

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
      if (!m?.page) return;
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

  // A row to bring into view. Only a new target moves the list: a re-render with the same one must
  // not fight the user's scroll.
  useLayoutEffect(() => {
    if (revealIndex === null || revealIndex === undefined || revealIndex >= count) return;
    revealRow(revealIndex);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [revealIndex]);

  // What the list shows changed: back to its first row.
  const firstRender = useRef(true);
  useLayoutEffect(() => {
    if (firstRender.current) {
      firstRender.current = false;
      return;
    }
    const m = metrics();
    if (m?.page && m.scrollTop > m.listTop) m.page.scrollTop = m.listTop;
    measure();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [resetKey]);

  return { range, revealRow };
}
