import { useEffect, useState } from "react";

/** The narrowest the workspace may get before the sidebar gives way (round 2, layout rule L5). */
const MIN_WORKSPACE_REM = 40;
/** The expanded sidebar's width: `w-64`. */
const EXPANDED_SIDEBAR_REM = 16;

/**
 * Whether the sidebar must fold to its icon rail so the workspace keeps 40rem. In rem of the
 * scaled root, so 960 wide stays open at normal scale and folds at large and extra large.
 */
export function sidebarMustCollapse(windowWidthPx: number, rootFontPx: number): boolean {
  return windowWidthPx < (MIN_WORKSPACE_REM + EXPANDED_SIDEBAR_REM) * rootFontPx;
}

function measure(): boolean {
  const rootFontPx = Number.parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
  return sidebarMustCollapse(window.innerWidth, rootFontPx);
}

/**
 * `sidebarMustCollapse` for this window, kept current. A media query can't answer it: media
 * queries measure rem against the browser default, not the root size the UI scale sets. So it
 * listens for resizes and for the UI scale rewriting the root's inline font size.
 */
export function useSidebarMustCollapse(): boolean {
  const [mustCollapse, setMustCollapse] = useState(measure);

  useEffect(() => {
    const update = () => setMustCollapse(measure());
    window.addEventListener("resize", update);
    const observer = new MutationObserver(update);
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ["style"] });
    return () => {
      window.removeEventListener("resize", update);
      observer.disconnect();
    };
  }, []);

  return mustCollapse;
}
