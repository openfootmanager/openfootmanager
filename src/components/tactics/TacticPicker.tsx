import { Search } from "lucide-react";
import { useEffect, useId, useMemo, useRef, useState, type JSX, type KeyboardEvent } from "react";
import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";

export interface TacticsLibraryEntry {
  description: string;
  formation: string;
  id: string;
  name: string;
  playStyle: string;
  sourcePresetName?: string | null;
  type: "preset" | "custom";
}

interface TacticPickerProps {
  activeTactic: TacticsLibraryEntry;
  library: TacticsLibraryEntry[];
  /** Whether the list is open: the bar owns it, so it can shut the list when its actions menu opens. */
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSelect: (id: string) => void;
}

/** The caption above a field of the tactic bar (the bar uses it for its own fields too). */
export const FIELD_LABEL = "mb-1 block font-heading text-caption font-bold text-fg-muted";

function summarizeTactic(entry: TacticsLibraryEntry, t: TFunction): string {
  return `${entry.formation} - ${t(`common.playStyles.${entry.playStyle}`, entry.playStyle)}`;
}

function matchesQuery(entry: TacticsLibraryEntry, query: string): boolean {
  return [
    entry.name,
    entry.description,
    entry.formation,
    entry.playStyle,
    entry.sourcePresetName,
    entry.type,
  ]
    .join(" ")
    .toLowerCase()
    .includes(query);
}

/**
 * The tactic picker: the active tactic as a button, and under it a searchable list of the saved
 * tactics and the presets. The keyboard reaches all of it: the search takes focus when the list
 * opens, ArrowDown/ArrowUp walk from the search through the options, Escape shuts the list and
 * a pick or Escape returns focus to the button.
 */
export default function TacticPicker({
  activeTactic,
  library,
  open,
  onOpenChange,
  onSelect,
}: TacticPickerProps): JSX.Element {
  const { t } = useTranslation();
  const [search, setSearch] = useState("");
  const wrapperRef = useRef<HTMLDivElement | null>(null);
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  const searchRef = useRef<HTMLInputElement | null>(null);
  const activeNameId = useId();

  const filtered = useMemo(() => {
    const query = search.trim().toLowerCase();
    return query ? library.filter((entry) => matchesQuery(entry, query)) : library;
  }, [search, library]);
  const presetEntries = filtered.filter((entry) => entry.type === "preset");
  const customEntries = filtered.filter((entry) => entry.type === "custom");

  useEffect(() => {
    if (!open) {
      setSearch("");
      return undefined;
    }
    searchRef.current?.focus();
    const handlePointerDown = (event: MouseEvent) => {
      if (!wrapperRef.current?.contains(event.target as Node)) onOpenChange(false);
    };
    document.addEventListener("mousedown", handlePointerDown);
    return () => document.removeEventListener("mousedown", handlePointerDown);
  }, [open, onOpenChange]);

  function close(): void {
    onOpenChange(false);
    triggerRef.current?.focus();
  }

  function handleKeyDown(event: KeyboardEvent<HTMLDivElement>): void {
    if (!open) return;
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      close();
      return;
    }
    if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
    const stops: HTMLElement[] = [
      ...(searchRef.current ? [searchRef.current] : []),
      ...(wrapperRef.current?.querySelectorAll<HTMLElement>('[role="option"]') ?? []),
    ];
    const at = stops.findIndex((stop) => stop === document.activeElement);
    const next =
      event.key === "ArrowDown" ? Math.min(at + 1, stops.length - 1) : Math.max(at - 1, 0);
    event.preventDefault();
    stops[next]?.focus();
  }

  function pick(id: string): void {
    onSelect(id);
    close();
  }

  return (
    // The keydown handler only routes keys from the controls inside it (search, options).
    // biome-ignore lint/a11y/noStaticElementInteractions: a key listener on the picker's wrapper; every control inside is focusable
    <div ref={wrapperRef} onKeyDown={handleKeyDown} className="relative min-w-52 flex-[2_1_14rem]">
      <span className={FIELD_LABEL}>{t("tactics.chooseTactic")}</span>
      <button
        ref={triggerRef}
        type="button"
        aria-label={t("tactics.chooseTactic")}
        aria-describedby={activeNameId}
        aria-expanded={open}
        aria-haspopup="listbox"
        onClick={() => onOpenChange(!open)}
        className="flex h-9 w-full items-center justify-between gap-3 rounded-lg border border-line-strong bg-panel px-3 text-left text-body text-fg hover:border-selected focus:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-page"
      >
        {/* Both keep their own width until the button is too narrow; then the summary gives up twice as much, so the name stays readable. */}
        <span id={activeNameId} className="flex min-w-0 flex-1 items-center justify-between gap-3">
          <span className="min-w-0 shrink truncate font-heading font-bold">
            {activeTactic.name}
          </span>
          <span className="min-w-0 shrink-[2] truncate text-right text-caption text-fg-muted">
            {summarizeTactic(activeTactic, t)}
          </span>
        </span>
      </button>

      {open ? (
        <div className="absolute left-0 right-0 top-full z-50 mt-2 rounded-xl border border-line bg-panel p-2 shadow-card">
          <div className="mb-2 flex items-center gap-2 rounded-lg border border-line bg-sunken px-3 py-2">
            <Search aria-hidden="true" className="h-4 w-4 text-fg-subtle" />
            <input
              ref={searchRef}
              type="text"
              value={search}
              onChange={(event) => setSearch(event.target.value)}
              aria-label={t("tactics.searchTactics")}
              placeholder={t("tactics.searchTactics")}
              className="w-full bg-transparent text-body text-fg outline-none placeholder:text-fg-subtle"
            />
          </div>
          <div
            role="listbox"
            aria-label={t("tactics.chooseTactic")}
            className="max-h-80 space-y-3 overflow-y-auto p-1"
          >
            {customEntries.length > 0 ? (
              <TacticPickerGroup
                title={t("tactics.myTactics")}
                entries={customEntries}
                activeId={activeTactic.id}
                onPick={pick}
                t={t}
              />
            ) : null}
            <TacticPickerGroup
              title={t("tactics.presets")}
              entries={presetEntries}
              activeId={activeTactic.id}
              onPick={pick}
              t={t}
            />
          </div>
        </div>
      ) : null}
    </div>
  );
}

/** One group of the picker's list (the saved tactics, or the presets). */
function TacticPickerGroup({
  title,
  entries,
  activeId,
  onPick,
  t,
}: {
  title: string;
  entries: TacticsLibraryEntry[];
  activeId: string;
  onPick: (id: string) => void;
  t: TFunction;
}): JSX.Element {
  return (
    <div>
      <div className="mb-2 px-2 font-heading text-caption font-bold text-fg-muted">{title}</div>
      <div className="space-y-1">
        {entries.map((entry) => (
          <button
            key={entry.id}
            type="button"
            role="option"
            aria-selected={entry.id === activeId}
            onClick={() => onPick(entry.id)}
            // Inset ring: the list scrolls inside a box that would clip an offset ring.
            className={`w-full rounded-lg border px-3 py-2 text-left transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-focus ${
              entry.id === activeId
                ? "border-selected bg-selected-tint"
                : "border-transparent bg-sunken hover:border-line hover:bg-hover"
            }`}
          >
            <div className="flex items-start justify-between gap-3">
              <div className="min-w-0">
                <div className="truncate font-heading text-body font-bold text-fg">
                  {entry.name}
                </div>
                <div className="mt-1 text-caption text-fg-muted">{entry.description}</div>
              </div>
              <span className="shrink-0 font-heading text-caption font-bold text-selected">
                {summarizeTactic(entry, t)}
              </span>
            </div>
          </button>
        ))}
      </div>
    </div>
  );
}
