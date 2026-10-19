import { type KeyboardEvent, type ReactNode, useRef } from "react";

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
const panelDomId = (prefix: string, id: string) => `${prefix}-panel-${id}`;

const LOOKS = {
  underline: {
    list: "flex items-stretch gap-1 self-stretch",
    tab: "flex items-center gap-2 border-b-2 px-3 font-heading text-body font-bold uppercase tracking-label transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-inset",
    selected: "border-primary-500 text-positive",
    idle: "border-transparent text-fg-muted hover:text-fg",
  },
  segmented: {
    list: "inline-flex items-center gap-0.5 rounded-lg border border-line bg-sunken p-0.5",
    tab: "flex h-7 items-center gap-2 rounded-md px-3 font-heading text-caption font-bold uppercase tracking-label transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-1 focus-visible:ring-offset-sunken",
    selected: "bg-panel text-fg shadow-sm ring-1 ring-line-strong",
    idle: "text-fg-muted hover:text-fg",
  },
};

/**
 * The one tab bar: an ARIA tablist with a roving tab stop. The arrow keys, Home and End move the
 * selection and the focus together, wrapping at both ends.
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
  const styles = LOOKS[look];

  function selectAndFocus(index: number) {
    const tab = tabs[(index + tabs.length) % tabs.length];
    onSelect(tab.id);
    buttons.current.get(tab.id)?.focus();
  }

  function handleKeyDown(event: KeyboardEvent<HTMLButtonElement>, index: number) {
    const target = {
      ArrowRight: index + 1,
      ArrowLeft: index - 1,
      Home: 0,
      End: tabs.length - 1,
    }[event.key];
    if (target === undefined) return;
    event.preventDefault();
    selectAndFocus(target);
  }

  return (
    <div role="tablist" aria-label={label} className={styles.list}>
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
            tabIndex={selected ? 0 : -1}
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

/** The content a tab controls, labelled by that tab. Keep every panel mounted or not, but always pass hidden. */
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
