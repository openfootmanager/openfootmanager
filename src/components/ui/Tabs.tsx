import { type KeyboardEvent, type ReactNode, useEffect, useRef } from "react";

import { rovingTarget } from "./roving";
import { SEGMENTED } from "./segmented";

export interface TabItem {
  id: string;
  label: ReactNode;
  /** Shown after the label and read as part of the tab's name. */
  count?: number;
  icon?: ReactNode;
}

interface TabsProps {
  /** Ties each tab to its TabPanel; give both the same prefix (useId() works). */
  idPrefix: string;
  /** The tablist's accessible name. */
  label: string;
  tabs: TabItem[];
  selectedId: string;
  onSelect: (id: string) => void;
  /** Underline for a page's sections; segmented for a two- or three-way switch inside a panel. */
  look?: "underline" | "segmented";
}

const tabDomId = (prefix: string, id: string) => `${prefix}-tab-${id}`;

/**
 * Scrolls `list` sideways just enough to show `tab`. Not scrollIntoView: that also scrolls the page
 * when the tablist is off screen. `offsetLeft` is measured from the list, which is `relative`.
 */
function revealInList(list: HTMLElement, tab: HTMLElement) {
  const tabRight = tab.offsetLeft + tab.offsetWidth;
  if (tab.offsetLeft < list.scrollLeft) {
    list.scrollLeft = tab.offsetLeft;
  } else if (tabRight > list.scrollLeft + list.clientWidth) {
    list.scrollLeft = tabRight - list.clientWidth;
  }
}
const panelDomId = (prefix: string, id: string) => `${prefix}-panel-${id}`;

const LOOKS = {
  underline: {
    list: "relative flex min-w-0 max-w-full items-stretch gap-1 self-stretch overflow-x-auto",
    tab: "flex shrink-0 items-center gap-2 whitespace-nowrap border-b-2 px-3 font-heading text-body font-bold uppercase tracking-label transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-inset",
    selected: "border-positive text-positive",
    idle: "border-transparent text-fg-muted hover:text-fg",
  },
  segmented: {
    list: `${SEGMENTED.track} ${SEGMENTED.scroll}`,
    tab: `${SEGMENTED.item} ${SEGMENTED.inline} shrink-0`,
    selected: SEGMENTED.selected.quiet,
    idle: SEGMENTED.idle,
  },
};

/**
 * The one tab bar: an ARIA tablist with a roving tab stop. The arrow keys, Home and End move the
 * selection and the focus together, wrapping at both ends. When the tabs don't fit (long labels,
 * a large UI scale) the tablist scrolls inside its own box rather than wrapping: a second row of
 * underline tabs reads as a second tab bar, and wrapping would change the toolbar's height by
 * locale. When the selection changes, the tablist (never the page) scrolls just enough to show it.
 */
export function Tabs({
  idPrefix,
  label,
  tabs,
  selectedId,
  onSelect,
  look = "underline",
}: TabsProps) {
  const buttons = useRef(new Map<string, HTMLButtonElement>());
  const list = useRef<HTMLDivElement>(null);
  const shownId = useRef(selectedId);
  const styles = LOOKS[look];
  // A selection no tab has must not leave the tablist with no tab stop at all.
  const tabStopId = tabs.some((tab) => tab.id === selectedId) ? selectedId : tabs[0]?.id;

  useEffect(() => {
    if (shownId.current === selectedId) return;
    shownId.current = selectedId;
    const tab = buttons.current.get(selectedId);
    if (list.current && tab) revealInList(list.current, tab);
  }, [selectedId]);

  function handleKeyDown(event: KeyboardEvent<HTMLButtonElement>, index: number) {
    const target = rovingTarget(event.key, index, tabs.length);
    if (target === null) return;
    event.preventDefault();
    const tab = tabs[target];
    onSelect(tab.id);
    buttons.current.get(tab.id)?.focus();
  }

  return (
    <div ref={list} role="tablist" aria-label={label} className={styles.list}>
      {tabs.map((tab, index) => {
        const selected = tab.id === selectedId;
        return (
          <button
            key={tab.id}
            ref={(node) => {
              if (node) buttons.current.set(tab.id, node);
              else buttons.current.delete(tab.id);
            }}
            type="button"
            role="tab"
            id={tabDomId(idPrefix, tab.id)}
            aria-selected={selected}
            aria-controls={panelDomId(idPrefix, tab.id)}
            tabIndex={tab.id === tabStopId ? 0 : -1}
            onClick={() => onSelect(tab.id)}
            onKeyDown={(event) => handleKeyDown(event, index)}
            className={`${styles.tab} ${selected ? styles.selected : styles.idle}`}
          >
            {tab.icon ? <span className="[&>svg]:h-4 [&>svg]:w-4">{tab.icon}</span> : null}
            {tab.label}
            {tab.count === undefined ? null : (
              <>
                {" "}
                <span className="font-sans text-caption font-semibold tabular-nums text-fg-subtle">
                  {tab.count}
                </span>
              </>
            )}
          </button>
        );
      })}
    </div>
  );
}

interface TabPanelProps {
  idPrefix: string;
  tabId: string;
  hidden: boolean;
  children: ReactNode;
  className?: string;
}

/**
 * The content a tab controls, labelled by that tab. Keep every panel mounted and pass `hidden` to
 * all but the selected one: each tab's aria-controls points at its panel's id, which must exist.
 */
export function TabPanel({ idPrefix, tabId, hidden, children, className = "" }: TabPanelProps) {
  return (
    <div
      role="tabpanel"
      id={panelDomId(idPrefix, tabId)}
      aria-labelledby={tabDomId(idPrefix, tabId)}
      hidden={hidden}
      // biome-ignore lint/a11y/noNoninteractiveTabindex: the WAI-ARIA tabs pattern gives a text-first tabpanel the tab stop after the tab list
      tabIndex={0}
      className={className}
    >
      {children}
    </div>
  );
}
