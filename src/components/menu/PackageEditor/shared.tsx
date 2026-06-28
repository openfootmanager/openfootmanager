import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Plus, Edit2, Trash2, ArrowLeft, CheckCircle, Loader2, X } from "lucide-react";

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
}

export function EntityListShell({
  addLabel,
  onAdd,
  emptyLabel,
  isEmpty,
  children,
  searchSlot,
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
    </div>
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
  editLabel: string;
  deleteLabel: string;
  isSelected?: boolean;
  onClick?: () => void;
}

export function EntityRow({
  title,
  subtitle,
  badge,
  onEdit,
  onDelete,
  editLabel,
  deleteLabel,
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
