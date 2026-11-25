/** Shared class names of the dashboard header and its search. */

export const FOCUS_RING =
  "focus:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-panel";

// Inside a menu the ring is inset: an offset ring would be clipped by the menu's edge.
export const MENU_FOCUS =
  "focus:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-focus";
export const MENU_ROW = `flex w-full items-center gap-2 px-3 py-2 text-left transition-colors hover:bg-hover ${MENU_FOCUS}`;
