import { Check, Copy, MoreVertical, Plus, RotateCcw, Save, Search, Trash2 } from "lucide-react";
import { useEffect, useMemo, useRef, useState, type JSX } from "react";
import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";

import ContextMenu, { type ContextMenuHandle, type ContextMenuItem } from "../ContextMenu";
import { Badge, Button, Card, Select } from "../ui";
import TacticsDeleteDialog from "./TacticsDeleteDialog";
import { FORMATIONS } from "./TacticsTab.helpers";

export interface TacticsLibraryEntry {
  description: string;
  formation: string;
  id: string;
  name: string;
  playStyle: string;
  sourcePresetName?: string | null;
  type: "preset" | "custom";
}

interface TacticsCommandBarProps {
  activeTactic: TacticsLibraryEntry;
  activePlayStyle: string;
  formation: string;
  isDirty: boolean;
  /** Whether pressing Reset would change anything; the library decides, the bar renders. */
  canReset: boolean;
  onReset: () => void;
  /** Whether the active tactic is a saved custom one, the only kind that can be deleted. */
  canDelete: boolean;
  onCreateNew: () => void;
  onDelete: () => void;
  onDuplicate: () => void;
  onFormationChange: (formation: string) => void;
  onPlayStyleChange: (playStyle: string) => void;
  /** Resolves whether the save was accepted: the "Saved" cue shows only when it was. */
  onSave: () => Promise<boolean>;
  onSelectTactic: (id: string) => void;
  tacticLibrary: TacticsLibraryEntry[];
}

const SAVE_CUE_DURATION_MS = 2000;

const PLAY_STYLES = [
  "Balanced",
  "Attacking",
  "Defensive",
  "Possession",
  "Counter",
  "HighPress",
] as const;

function summarizeTactic(entry: TacticsLibraryEntry, t: TFunction): string {
  return `${entry.formation} - ${t(`common.playStyles.${entry.playStyle}`, entry.playStyle)}`;
}

export default function TacticsCommandBar({
  activeTactic,
  activePlayStyle,
  formation,
  isDirty,
  canReset,
  onReset,
  canDelete,
  onCreateNew,
  onDelete,
  onDuplicate,
  onFormationChange,
  onPlayStyleChange,
  onSave,
  onSelectTactic,
  tacticLibrary,
}: TacticsCommandBarProps): JSX.Element {
  const { t } = useTranslation();
  const [isOpen, setIsOpen] = useState(false);
  const [search, setSearch] = useState("");
  const [showSavedCue, setShowSavedCue] = useState(false);
  const [confirmingDelete, setConfirmingDelete] = useState(false);
  const wrapperRef = useRef<HTMLDivElement | null>(null);
  const actionsMenuRef = useRef<ContextMenuHandle | null>(null);
  const [actionsOpen, setActionsOpen] = useState(false);
  const savedCueTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const filteredLibrary = useMemo(() => {
    const query = search.trim().toLowerCase();

    if (!query) {
      return tacticLibrary;
    }

    return tacticLibrary.filter((entry) =>
      [
        entry.name,
        entry.description,
        entry.formation,
        entry.playStyle,
        entry.sourcePresetName,
        entry.type,
      ]
        .join(" ")
        .toLowerCase()
        .includes(query),
    );
  }, [search, tacticLibrary]);

  const presetEntries = filteredLibrary.filter((entry) => entry.type === "preset");
  const customEntries = filteredLibrary.filter((entry) => entry.type === "custom");

  const saveLabel =
    activeTactic.type === "custom" ? t("tactics.updateTactic") : t("tactics.saveAsTactic");
  // Saving a synced custom tactic is a silent no-op — nothing has changed to
  // persist. Presets stay enabled even when isDirty is false because saving
  // there always creates a new custom tactic, which is an observable action.
  const isSaveDisabled = activeTactic.type === "custom" && !isDirty;

  useEffect(() => {
    const handlePointerDown = (event: MouseEvent) => {
      if (!wrapperRef.current?.contains(event.target as Node)) {
        setIsOpen(false);
      }
    };

    document.addEventListener("mousedown", handlePointerDown);
    return () => document.removeEventListener("mousedown", handlePointerDown);
  }, []);

  useEffect(() => {
    return () => {
      if (savedCueTimeoutRef.current) {
        clearTimeout(savedCueTimeoutRef.current);
      }
    };
  }, []);

  // The tactic and the save a cue would confirm: an answer that lands after the manager moved on
  // (another tactic, a newer save) must not confirm the wrong one.
  const activeTacticIdRef = useRef(activeTactic?.id);
  activeTacticIdRef.current = activeTactic?.id;
  const latestSaveRef = useRef(0);

  async function handleSaveClick(): Promise<void> {
    // The old cue belongs to the previous save: it goes while this one is pending.
    setShowSavedCue(false);
    if (savedCueTimeoutRef.current) {
      clearTimeout(savedCueTimeoutRef.current);
    }
    const thisSave = ++latestSaveRef.current;
    const savedTacticId = activeTactic?.id;
    // Saving a preset makes a new custom tactic and the bar moves to it; saving a custom one keeps
    // it active, so a different active tactic after the answer means the manager moved away.
    const createsATactic = activeTactic?.type !== "custom";
    const saved = await onSave();
    if (!saved || thisSave !== latestSaveRef.current) return;
    if (!createsATactic && activeTacticIdRef.current !== savedTacticId) return;
    setShowSavedCue(true);

    savedCueTimeoutRef.current = setTimeout(() => {
      setShowSavedCue(false);
    }, SAVE_CUE_DURATION_MS);
  }

  const actionItems: ContextMenuItem[] = [
    { label: t("tactics.newTactic"), icon: <Plus className="h-4 w-4" />, onClick: onCreateNew },
    {
      label: t("tactics.duplicateTactic"),
      icon: <Copy className="h-4 w-4" />,
      onClick: onDuplicate,
    },
    {
      label: t("tactics.resetTactic"),
      icon: <RotateCcw className="h-4 w-4" />,
      onClick: onReset,
      disabled: !canReset,
    },
    ...(canDelete
      ? [
          {
            label: t("tactics.deleteTactic"),
            icon: <Trash2 className="h-4 w-4" />,
            onClick: () => setConfirmingDelete(true),
            danger: true,
          },
        ]
      : []),
  ];

  const pick = (id: string) => {
    onSelectTactic(id);
    setIsOpen(false);
    setSearch("");
  };

  return (
    <Card className="overflow-visible">
      {/* One row of controls (it wraps when the window is narrow): the tactic, its formation and style,
          the actions that wait in a menu, and Save. */}
      <div
        ref={wrapperRef}
        role="group"
        aria-label={t("tactics.tacticBar")}
        data-region="tactics-bar"
        className="flex flex-wrap items-end gap-3 px-4 py-3"
      >
        <div className="relative min-w-52 flex-[2_1_14rem]">
          <span className={FIELD_LABEL}>{t("tactics.chooseTactic")}</span>
          <button
            type="button"
            aria-label={t("tactics.chooseTactic")}
            aria-expanded={isOpen}
            aria-haspopup="listbox"
            onClick={() => setIsOpen((open) => !open)}
            className="flex h-9 w-full items-center justify-between gap-3 rounded-lg border border-line-strong bg-panel px-3 text-left text-body text-fg hover:border-selected focus:outline-none focus-visible:ring-2 focus-visible:ring-focus"
          >
            <span className="min-w-0 truncate font-heading font-bold">{activeTactic.name}</span>
            <span className="shrink-0 text-caption uppercase tracking-label text-fg-muted">
              {summarizeTactic(activeTactic, t)}
            </span>
          </button>

          {isOpen ? (
            <div className="absolute left-0 right-0 top-full z-50 mt-2 rounded-xl border border-line bg-panel p-2 shadow-card">
              <div className="mb-2 flex items-center gap-2 rounded-lg border border-line bg-sunken px-3 py-2">
                <Search aria-hidden="true" className="h-4 w-4 text-fg-subtle" />
                <input
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

        <div className="min-w-32 flex-[1_1_9rem]">
          <span className={FIELD_LABEL}>{t("tactics.formation")}</span>
          <Select
            value={FORMATIONS.includes(formation) ? formation : FORMATIONS[0]}
            onChange={(e) => onFormationChange(e.target.value)}
            fullWidth
            aria-label={t("tactics.formation")}
          >
            {FORMATIONS.map((f) => (
              <option key={f} value={f}>
                {f}
              </option>
            ))}
          </Select>
        </div>

        <div className="min-w-32 flex-[1_1_10rem]">
          <span className={FIELD_LABEL}>{t("tactics.playStyle")}</span>
          <Select
            value={activePlayStyle}
            onChange={(e) => onPlayStyleChange(e.target.value)}
            fullWidth
            aria-label={t("tactics.playStyle")}
          >
            {PLAY_STYLES.map((style) => (
              // Native <option> renders text only: an icon or span child is stripped by the browser.
              <option key={style} value={style}>
                {t(`common.playStyles.${style}`, style)}
              </option>
            ))}
          </Select>
        </div>

        <ContextMenu items={actionItems} ref={actionsMenuRef} onOpenChange={setActionsOpen}>
          <button
            type="button"
            aria-label={t("tactics.tacticActions")}
            aria-haspopup="menu"
            aria-expanded={actionsOpen}
            onClick={(event) => {
              const rect = event.currentTarget.getBoundingClientRect();
              actionsMenuRef.current?.open(rect.left, rect.bottom + 4);
            }}
            className="grid h-9 w-9 shrink-0 place-items-center rounded-lg border border-line-strong bg-panel text-fg-muted hover:text-fg focus:outline-none focus-visible:ring-2 focus-visible:ring-focus"
          >
            <MoreVertical aria-hidden="true" className="h-4 w-4" />
          </button>
        </ContextMenu>

        <div className="ml-auto flex items-center gap-2">
          <Badge variant={isDirty ? "accent" : "neutral"} size="sm">
            {isDirty ? t("tactics.unsavedChanges") : t("tactics.synced")}
          </Badge>
          <Button
            type="button"
            variant="accent"
            size="sm"
            icon={showSavedCue && !isDirty ? <Check /> : <Save />}
            onClick={handleSaveClick}
            disabled={isSaveDisabled}
            className="min-w-[9.5rem]"
          >
            {showSavedCue && !isDirty ? t("tactics.tacticSaved") : saveLabel}
          </Button>
          {/* The button's own text changing is not announced; this is. */}
          <span aria-live="polite" className="sr-only">
            {showSavedCue && !isDirty ? t("tactics.tacticSaved") : ""}
          </span>
        </div>
      </div>
      {confirmingDelete ? (
        <TacticsDeleteDialog
          name={activeTactic.name}
          onCancel={() => setConfirmingDelete(false)}
          onConfirm={() => {
            setConfirmingDelete(false);
            onDelete();
          }}
        />
      ) : null}
    </Card>
  );
}

const FIELD_LABEL =
  "mb-1 block font-heading text-caption font-bold uppercase tracking-label text-fg-muted";

/** One group of the tactic picker's list (the saved tactics, or the presets). */
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
      <div className="mb-2 px-2 font-heading text-caption font-bold uppercase tracking-label text-fg-muted">
        {title}
      </div>
      <div className="space-y-1">
        {entries.map((entry) => (
          <button
            key={entry.id}
            type="button"
            role="option"
            aria-selected={entry.id === activeId}
            onClick={() => onPick(entry.id)}
            className={`w-full rounded-lg border px-3 py-2 text-left transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-focus ${
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
              <span className="shrink-0 font-heading text-caption font-bold uppercase tracking-label text-selected">
                {summarizeTactic(entry, t)}
              </span>
            </div>
          </button>
        ))}
      </div>
    </div>
  );
}
