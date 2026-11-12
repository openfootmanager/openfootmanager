import { Check, Copy, MoreVertical, Pencil, Plus, RotateCcw, Save, Trash2 } from "lucide-react";
import { useId, useRef, useState, type JSX } from "react";
import { useTranslation } from "react-i18next";

import ContextMenu, { type ContextMenuHandle, type ContextMenuItem } from "../ContextMenu";
import { Badge, Button, Card, Select } from "../ui";
import TacticPicker, { FIELD_LABEL, type TacticsLibraryEntry } from "./TacticPicker";
import TacticsDeleteDialog from "./TacticsDeleteDialog";
import TacticsRenameDialog from "./TacticsRenameDialog";
import { useSavedCue } from "./useSavedCue";
import { FORMATIONS } from "./TacticsTab.helpers";

interface TacticsCommandBarProps {
  activeTactic: TacticsLibraryEntry;
  activePlayStyle: string;
  formation: string;
  isDirty: boolean;
  /**
   * Whether the library's first load has settled. Save waits for it: a save asked while the load
   * is pending queues behind it and records whatever the manager changed meanwhile.
   */
  libraryLoaded: boolean;
  /** Whether pressing Reset would change anything; the library decides, the bar renders. */
  canReset: boolean;
  onReset: () => void;
  /** Whether the active tactic is a saved custom one, the only kind that can be deleted. */
  canDelete: boolean;
  /** Whether the active tactic is a saved custom one, the only kind that can be renamed. */
  canRename: boolean;
  /** Renames it: resolves null once the save accepted the name, or why it did not. */
  onRename: (name: string) => Promise<string | null>;
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

const PLAY_STYLES = [
  "Balanced",
  "Attacking",
  "Defensive",
  "Possession",
  "Counter",
  "HighPress",
] as const;

export default function TacticsCommandBar({
  activeTactic,
  activePlayStyle,
  formation,
  isDirty,
  libraryLoaded,
  canReset,
  onReset,
  canDelete,
  canRename,
  onRename,
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
  const [pickerOpen, setPickerOpen] = useState(false);
  const statusId = useId();
  const [confirmingDelete, setConfirmingDelete] = useState(false);
  const [renaming, setRenaming] = useState(false);
  const actionsMenuRef = useRef<ContextMenuHandle | null>(null);
  const [actionsOpen, setActionsOpen] = useState(false);

  const saveLabel =
    activeTactic.type === "custom" ? t("tactics.updateTactic") : t("tactics.saveAsTactic");
  // Saving a synced custom tactic is a silent no-op — nothing has changed to
  // persist. Presets stay enabled even when isDirty is false because saving
  // there always creates a new custom tactic, which is an observable action.
  const isSaveDisabled = !libraryLoaded || (activeTactic.type === "custom" && !isDirty);

  const { showSavedCue, save: handleSaveClick } = useSavedCue(activeTactic, onSave);

  // The picker's list and the actions menu are never open together: a keyboard-chosen action does not
  // produce the outside click that would shut the list.
  function handleActionsOpenChange(open: boolean): void {
    setActionsOpen(open);
    if (open) setPickerOpen(false);
  }

  const actionItems: ContextMenuItem[] = [
    { label: t("tactics.newTactic"), icon: <Plus className="h-4 w-4" />, onClick: onCreateNew },
    {
      label: t("tactics.duplicateTactic"),
      icon: <Copy className="h-4 w-4" />,
      onClick: onDuplicate,
    },
    {
      label: t("tactics.renameTactic"),
      icon: <Pencil className="h-4 w-4" />,
      onClick: () => setRenaming(true),
      disabled: !canRename,
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

  return (
    <Card className="overflow-visible">
      {/* One row of controls (it wraps when the window is narrow): the tactic, its formation and style,
          the actions that wait in a menu, and Save. */}
      <div
        role="group"
        aria-label={t("tactics.tacticBar")}
        data-region="tactics-bar"
        className="flex flex-wrap items-end gap-3 px-4 py-3"
      >
        <TacticPicker
          activeTactic={activeTactic}
          library={tacticLibrary}
          open={pickerOpen}
          onOpenChange={setPickerOpen}
          onSelect={onSelectTactic}
        />

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

        <ContextMenu
          items={actionItems}
          ref={actionsMenuRef}
          onOpenChange={handleActionsOpenChange}
        >
          <button
            type="button"
            aria-label={t("tactics.tacticActions")}
            aria-haspopup="menu"
            aria-expanded={actionsOpen}
            onClick={(event) => {
              const rect = event.currentTarget.getBoundingClientRect();
              actionsMenuRef.current?.open(rect.left, rect.bottom + 4);
            }}
            className="grid h-9 w-9 shrink-0 place-items-center rounded-lg border border-line-strong bg-panel text-fg-muted hover:text-fg focus:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-page"
          >
            <MoreVertical aria-hidden="true" className="h-4 w-4" />
          </button>
        </ContextMenu>

        <div className="ml-auto flex items-center gap-2">
          {/* Always on the page, so a change of what it says is announced. While the library loads it
              says why Save is off. */}
          <span id={statusId} aria-live="polite" className="inline-flex">
            {libraryLoaded ? (
              <Badge variant={isDirty ? "accent" : "neutral"} size="sm">
                {isDirty ? t("tactics.unsavedChanges") : t("tactics.synced")}
              </Badge>
            ) : (
              <Badge variant="neutral" size="sm">
                {t("tactics.loadingTactics")}
              </Badge>
            )}
          </span>
          <Button
            type="button"
            variant="accent"
            size="sm"
            icon={showSavedCue && !isDirty ? <Check /> : <Save />}
            onClick={handleSaveClick}
            disabled={isSaveDisabled}
            aria-describedby={libraryLoaded ? undefined : statusId}
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
      {renaming ? (
        <TacticsRenameDialog
          name={activeTactic.name}
          onRename={onRename}
          onClose={() => setRenaming(false)}
        />
      ) : null}
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
