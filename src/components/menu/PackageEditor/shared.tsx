import { useId, useState } from "react";
import { useTranslation } from "react-i18next";
import { Plus, Copy, Download, Edit2, Trash2, ArrowLeft, CheckCircle, Loader2, X } from "lucide-react";
import { Button } from "../../ui/Button";
import { ENTITY_LIST_PAGE_SIZE } from "./entityList.helpers";

// ---------------------------------------------------------------------------
// EntityListShell
// ---------------------------------------------------------------------------

interface EntityListShellProps {
  addLabel: string;
  onAdd: () => void;
  emptyLabel: string;
  isEmpty: boolean;
  children: React.ReactNode;
  searchSlot?: React.ReactNode;
  /** Sits under the rows: the match count, and the button that reveals more. */
  footerSlot?: React.ReactNode;
}

export function EntityListShell({
  addLabel,
  onAdd,
  emptyLabel,
  isEmpty,
  children,
  searchSlot,
  footerSlot,
}: EntityListShellProps) {
  return (
    <div className="flex flex-col gap-2">
      <button
        onClick={onAdd}
        className="flex items-center justify-center gap-2 w-full py-2.5 border border-dashed border-gray-300 dark:border-navy-500 rounded-xl text-sm text-gray-500 dark:text-gray-400 hover:text-primary-500 dark:hover:text-primary-400 hover:border-primary-400 dark:hover:border-primary-500 transition-colors"
      >
        <Plus className="w-4 h-4" />
        <span className="font-heading font-bold uppercase tracking-wider">{addLabel}</span>
      </button>

      {searchSlot && <div>{searchSlot}</div>}

      {isEmpty && (
        <p className="text-xs text-gray-400 dark:text-gray-500 text-center py-4">{emptyLabel}</p>
      )}

      <div className="flex flex-col gap-2">{children}</div>

      {footerSlot}
    </div>
  );
}

// ---------------------------------------------------------------------------
// EntityListFooter
// ---------------------------------------------------------------------------

interface EntityListFooterProps {
  /** Rows currently rendered. */
  shown: number;
  /** Rows the search and filters left, rendered or not. */
  matches: number;
  /** Whether this section holds any records at all. */
  hasRecords: boolean;
  onLoadMore: () => void;
}

/**
 * The line under an entity list: how much of it you are looking at, and the
 * button that shows more. Also the place a fruitless search is reported —
 * without it the panel just went blank, since the shell's empty message only
 * covers a section with no records in it at all.
 */
export function EntityListFooter({ shown, matches, hasRecords, onLoadMore }: EntityListFooterProps) {
  const { t } = useTranslation();
  const countId = useId();

  if (!hasRecords) {
    return null;
  }

  const everythingShown = shown >= matches;

  // Kept mounted, disabled, once the list has run out: revealing the last
  // page from the keyboard would otherwise unmount the focused button and
  // drop focus to the top of the document, hundreds of rows above.
  const wasCapped = matches > ENTITY_LIST_PAGE_SIZE;

  return (
    <div className="flex flex-col gap-2 pt-1">
      {wasCapped && (
        <Button
          type="button"
          variant="outline"
          size="sm"
          // `aria-disabled`, not `disabled`: a disabled element is not
          // focusable, so disabling the button the user just pressed hands
          // focus back to the document — the same problem as unmounting it.
          // This keeps it in the tab order and announced as unavailable.
          className={`w-full ${everythingShown ? "opacity-50 cursor-not-allowed" : ""}`}
          onClick={everythingShown ? undefined : onLoadMore}
          aria-disabled={everythingShown}
          aria-describedby={countId}
        >
          {t("common.loadMore")}
        </Button>
      )}
      {/*
        One live region for all three messages, so revealing a page or
        searching into nothing is announced rather than silently changing
        the rows underneath.
      */}
      <p
        id={countId}
        role="status"
        className={`text-xs text-gray-500 dark:text-gray-400 ${matches === 0 ? "text-center py-4" : "text-right"}`}
      >
        {matches === 0
          ? t("common.noResults")
          : everythingShown
            ? t("common.nResults", { count: matches })
            : t("worldEditor.showingEntries", { shown, total: matches })}
      </p>
    </div>
  );
}

// ---------------------------------------------------------------------------
// ExportCsvButton
// ---------------------------------------------------------------------------

/** Sits beside a list's search box and writes that entity type out as CSV. */
export function ExportCsvButton({ onClick }: { onClick: () => void }) {
  const { t } = useTranslation();
  return (
    <button
      type="button"
      onClick={onClick}
      title={t("worldEditor.exportCsv")}
      aria-label={t("worldEditor.exportCsv")}
      className="flex items-center justify-center p-1.5 rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 text-gray-400 dark:text-gray-500 hover:text-primary-500 dark:hover:text-primary-400 hover:border-primary-400 dark:hover:border-primary-500 transition-colors flex-shrink-0"
    >
      <Download className="w-3.5 h-3.5" />
    </button>
  );
}

// ---------------------------------------------------------------------------
// EntityRow
// ---------------------------------------------------------------------------

interface EntityRowProps {
  title: string;
  subtitle?: string;
  badge?: React.ReactNode;
  onEdit: () => void;
  onDelete: () => void;
  /** Omit to hide the duplicate action for entity types that don't support it. */
  onDuplicate?: () => void;
  editLabel: string;
  deleteLabel: string;
  duplicateLabel?: string;
  isSelected?: boolean;
  onClick?: () => void;
}

export function EntityRow({
  title,
  subtitle,
  badge,
  onEdit,
  onDelete,
  onDuplicate,
  editLabel,
  deleteLabel,
  duplicateLabel,
  isSelected,
  onClick,
}: EntityRowProps) {
  const { t } = useTranslation();
  const [confirming, setConfirming] = useState(false);

  function handleDeleteClick(e: React.MouseEvent) {
    e.stopPropagation();
    setConfirming(true);
  }

  function handleConfirmDelete(e: React.MouseEvent) {
    e.stopPropagation();
    setConfirming(false);
    onDelete();
  }

  function handleCancelDelete(e: React.MouseEvent) {
    e.stopPropagation();
    setConfirming(false);
  }

  return (
    <div
      className={`flex items-center gap-3 p-3 rounded-xl border transition-colors ${
        isSelected
          ? "border-primary-400 dark:border-primary-500 bg-primary-50 dark:bg-primary-500/10"
          : "border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 hover:border-gray-300 dark:hover:border-navy-500"
      } ${onClick ? "cursor-pointer" : ""}`}
      onClick={confirming ? undefined : onClick}
    >
      {badge}
      <div className="flex-1 min-w-0">
        <p className="font-heading font-bold text-sm uppercase tracking-wide text-gray-800 dark:text-gray-200 truncate">
          {title}
        </p>
        {subtitle && (
          <p className="text-[10px] text-gray-400 dark:text-gray-500">{subtitle}</p>
        )}
      </div>
      {confirming ? (
        <div className="flex items-center gap-1 flex-shrink-0">
          <button
            onClick={handleConfirmDelete}
            className="p-1 rounded-md bg-red-500 text-white hover:bg-red-600 transition-colors"
            title={t("common.confirmDelete")}
          >
            <Trash2 className="w-3.5 h-3.5" />
          </button>
          <button
            onClick={handleCancelDelete}
            className="p-1 rounded-md text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 transition-colors"
            title={t("common.cancel")}
          >
            <X className="w-3.5 h-3.5" />
          </button>
        </div>
      ) : (
        <>
          <button
            onClick={(e) => { e.stopPropagation(); onEdit(); }}
            className="text-gray-400 hover:text-primary-500 transition-colors flex-shrink-0"
            title={editLabel}
          >
            <Edit2 className="w-4 h-4" />
          </button>
          {onDuplicate && (
            <button
              onClick={(e) => { e.stopPropagation(); onDuplicate(); }}
              className="text-gray-400 hover:text-primary-500 transition-colors flex-shrink-0"
              title={duplicateLabel}
              aria-label={duplicateLabel}
            >
              <Copy className="w-4 h-4" />
            </button>
          )}
          <button
            onClick={handleDeleteClick}
            className="text-gray-400 hover:text-red-500 transition-colors flex-shrink-0"
            title={deleteLabel}
          >
            <Trash2 className="w-4 h-4" />
          </button>
        </>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// EntityFormShell
// ---------------------------------------------------------------------------

interface EntityFormShellProps {
  title: string;
  onBack: () => void;
  onSave: () => void;
  isBusy: boolean;
  saveDisabled?: boolean;
  saveLabel: string;
  children: React.ReactNode;
}

export function EntityFormShell({
  title,
  onBack,
  onSave,
  isBusy,
  saveDisabled,
  saveLabel,
  children,
}: EntityFormShellProps) {
  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center gap-2 mb-2">
        <button
          onClick={onBack}
          className="text-gray-400 hover:text-gray-700 dark:hover:text-white transition-colors p-1 rounded-lg hover:bg-gray-100 dark:hover:bg-navy-600"
        >
          <ArrowLeft className="w-5 h-5" />
        </button>
        <h2 className="text-xl font-heading font-bold uppercase tracking-wide text-gray-900 dark:text-white">
          {title}
        </h2>
      </div>

      <div className="flex flex-col gap-3">{children}</div>

      <button
        onClick={onSave}
        disabled={isBusy || saveDisabled}
        className="w-full py-3 bg-gradient-to-r from-primary-500 to-primary-600 hover:from-primary-600 hover:to-primary-700 text-white rounded-xl font-heading font-bold uppercase tracking-wide transition-all disabled:opacity-60 disabled:cursor-not-allowed flex items-center justify-center gap-2"
      >
        {isBusy ? <Loader2 className="w-4 h-4 animate-spin" /> : <CheckCircle className="w-4 h-4" />}
        {saveLabel}
      </button>
    </div>
  );
}
