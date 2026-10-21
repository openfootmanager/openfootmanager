/**
 * The segmented look, in one place: Tabs' segmented switch and SegmentedControl both draw from it.
 * A sunken track holds the items. The selected item is raised (`quiet`) when it switches what a
 * panel shows, or filled with the brand colour (`strong`) when it is a setting the player is
 * watching, like a match's speed.
 */
export const SEGMENTED = {
  track:
    "relative min-w-0 max-w-full items-center gap-0.5 rounded-lg border border-line bg-sunken p-1",
  /** One row that scrolls when it runs out of room: Tabs, and a control sized to its options. */
  scroll: "inline-flex overflow-x-auto",
  /** The full width, wrapping onto another row rather than hiding an option off the end. */
  wrap: "flex w-full flex-wrap",
  item: "flex items-center gap-2 whitespace-nowrap rounded-md font-heading text-caption font-bold uppercase tracking-label transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-1 focus-visible:ring-offset-sunken",
  /** One line of label, the height of a compact control. */
  inline: "h-7 px-3",
  /** An icon over its label, a comfortable target (52px). */
  stacked: "min-h-13 flex-col justify-center gap-1 px-2",
  idle: "text-fg-muted hover:text-fg",
  selected: {
    quiet: "bg-panel text-fg shadow-sm ring-1 ring-line-strong",
    strong: "bg-primary-700 text-on-brand shadow-sm",
    /** Chosen but not running, like a paused match's speed: outlined, so it differs from both. */
    outlined: "bg-panel text-positive ring-2 ring-inset ring-positive-solid",
  },
} as const;
