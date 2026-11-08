import { type RefObject, useEffect, useState } from "react";

/**
 * Whether the element is at least `rem` wide, kept current as it resizes. The rem is the root's,
 * so the UI scale moves the threshold with everything else, as a container query's rem does.
 *
 * Starts true, and stays true where the browser can't measure (no ResizeObserver: a test's DOM),
 * so a screen that can't tell its width lays out as it does when there is room.
 */
export function useWidthAtLeast(ref: RefObject<HTMLElement | null>, rem: number): boolean {
  const [atLeast, setAtLeast] = useState(true);

  useEffect(() => {
    const element = ref.current;
    if (!element || typeof ResizeObserver === "undefined") return undefined;
    const measure = () => {
      const rootPx = Number.parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
      setAtLeast(element.getBoundingClientRect().width >= rem * rootPx);
    };
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    measure();
    return () => observer.disconnect();
  }, [ref, rem]);

  return atLeast;
}
