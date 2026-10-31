import type { AppSettings } from "../../store/settingsStore";

type SidebarWidth = AppSettings["sidebar_width"];
type SidebarBehaviour = AppSettings["sidebar_behaviour"];

/**
 * The sidebar's widths in rem, the same values as the `--spacing-sidebar-*` tokens it is drawn
 * with (a test holds the two together). Wide is the narrowest width that puts every nav label on
 * one line in all 12 locales; it was measured on the OS fallback font, and is re-measured once the
 * game bundles its Cyrillic fonts.
 */
export const SIDEBAR_REM = { rail: 3.5, compact: 13, standard: 16, wide: 17.75 } as const;

/** The narrowest the page may get beside the open sidebar before Normal starts as a rail. */
const MIN_PAGE_REM = 40;

interface SidebarState {
  width: SidebarWidth;
  behaviour: SidebarBehaviour;
  /** The player folded it (Normal only). */
  collapsed: boolean;
  /** Open over the page: hovered, focused or expanded as a drawer. */
  overlayOpen: boolean;
}

interface SidebarFrame {
  windowPx: number;
  /** The root font size, which the UI scale sets: the sidebar's rem follow it. */
  rootPx: number;
}

export interface SidebarLayout {
  /** The width the sidebar takes from the page. */
  slot: SidebarWidth | "rail";
  /** The width it is drawn at; wider than the slot when it is open over the page. */
  drawn: SidebarWidth | "rail";
  /** Drawn over the page. */
  overlay: boolean;
  /** A click outside closes it (the drawer; the hover closes when the pointer leaves). */
  closesOnOutsideClick: boolean;
  /** What the sidebar's own button does, or null when it has none. */
  toggle: "expand" | "collapse" | null;
  /** The button opens a drawer over the page instead of folding the sidebar in place. */
  opensDrawer: boolean;
}

/** Whether a window is too narrow for the open sidebar and a page of `MIN_PAGE_REM` beside it. */
function tooNarrow(width: SidebarWidth, { windowPx, rootPx }: SidebarFrame): boolean {
  return windowPx < (MIN_PAGE_REM + SIDEBAR_REM[width]) * rootPx;
}

function overlayLayout(
  state: SidebarState,
  closesOnOutsideClick: boolean,
  hasButton: boolean,
): SidebarLayout {
  const open = state.overlayOpen;
  return {
    slot: "rail",
    drawn: open ? state.width : "rail",
    overlay: open,
    closesOnOutsideClick: closesOnOutsideClick && open,
    toggle: hasButton ? (open ? "collapse" : "expand") : null,
    opensDrawer: hasButton,
  };
}

/**
 * Where the sidebar sits, from the player's settings and the window. One rule for every behaviour:
 * - Normal is open at the chosen width, as today, and folds when the player folds it; in a window
 *   too narrow for it, it rests as a rail and its button opens it over the page as a drawer.
 * - Expand on hover rests as a rail and opens over the page while hovered or focused.
 * - Expand on click rests as a rail and its button opens it over the page as a drawer.
 * - Icons only is always a rail.
 */
export function sidebarLayout(state: SidebarState, frame: SidebarFrame): SidebarLayout {
  switch (state.behaviour) {
    case "icons":
      return {
        slot: "rail",
        drawn: "rail",
        overlay: false,
        closesOnOutsideClick: false,
        toggle: null,
        opensDrawer: false,
      };
    case "hover":
      return overlayLayout(state, false, false);
    case "click":
      return overlayLayout(state, true, true);
    case "normal": {
      if (tooNarrow(state.width, frame)) return overlayLayout(state, true, true);
      const size = state.collapsed ? "rail" : state.width;
      return {
        slot: size,
        drawn: size,
        overlay: false,
        closesOnOutsideClick: false,
        toggle: state.collapsed ? "expand" : "collapse",
        opensDrawer: false,
      };
    }
  }
}
