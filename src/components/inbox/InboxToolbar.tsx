import { CheckCheck, Trash2 } from "lucide-react";
import type { JSX } from "react";
import { useTranslation } from "react-i18next";

import { Badge, Button, Select } from "../ui";
import {
  getCategoryIcon,
  getFilterButtonClassName,
  type MessageSortOrder,
  UNREAD_FILTER,
} from "./inboxHelpers";

interface InboxToolbarProps {
  allMessagesCount: number;
  bulkSelectionEnabled: boolean;
  categories: string[];
  categoryCounts: Map<string, number>;
  categoryFilter: string | null;
  selectedMessageCount: number;
  sortOrder: MessageSortOrder;
  unreadCount: number;
  onClearOld: () => void;
  onDeleteSelected: () => void;
  onMarkAllRead: () => void;
  onShowAll: () => void;
  onShowUnread: () => void;
  onSortOrderChange: (sortOrder: MessageSortOrder) => void;
  onToggleBulkSelectionMode: () => void;
  onToggleCategory: (category: string) => void;
}

export default function InboxToolbar({
  allMessagesCount,
  bulkSelectionEnabled,
  categories,
  categoryCounts,
  categoryFilter,
  selectedMessageCount,
  sortOrder,
  unreadCount,
  onClearOld,
  onDeleteSelected,
  onMarkAllRead,
  onShowAll,
  onShowUnread,
  onSortOrderChange,
  onToggleBulkSelectionMode,
  onToggleCategory,
}: InboxToolbarProps): JSX.Element {
  const { t } = useTranslation();

  return (
    <div className="flex gap-2 mb-4 flex-wrap shrink-0">
      <button
        onClick={onShowAll}
        className={getFilterButtonClassName(!categoryFilter)}
      >
        {t("common.all")} ({allMessagesCount})
      </button>
      {unreadCount > 0 ? (
        <button
          onClick={onShowUnread}
          className={getFilterButtonClassName(categoryFilter === UNREAD_FILTER)}
        >
          {t("inbox.unread", { count: unreadCount })}
        </button>
      ) : null}
      {categories.map((category) => {
        const categoryIcon = getCategoryIcon(category);
        const count = categoryCounts.get(category) ?? 0;

        return (
          <button
            key={category}
            onClick={() => onToggleCategory(category)}
            className={getFilterButtonClassName(
              categoryFilter === category,
              "flex items-center gap-1.5",
            )}
          >
            {categoryIcon} {t(`inbox.categories.${category}`)} ({count})
          </button>
        );
      })}

      <div className="ml-auto flex flex-wrap items-center justify-end gap-2">
        <div className="flex items-center gap-2">
          <label
            htmlFor="inbox-sort-order"
            className="text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400"
          >
            {t("inbox.sortLabel")}
          </label>
          <Select
            id="inbox-sort-order"
            value={sortOrder}
            onChange={(event) =>
              onSortOrderChange(event.target.value as MessageSortOrder)
            }
            selectSize="sm"
            wrapperClassName="min-w-[170px]"
            aria-label={t("inbox.sortByDate")}
          >
            <option value="newest">{t("inbox.sortNewest")}</option>
            <option value="oldest">{t("inbox.sortOldest")}</option>
          </Select>
        </div>
        <Button
          type="button"
          variant={bulkSelectionEnabled ? "primary" : "outline"}
          size="sm"
          onClick={onToggleBulkSelectionMode}
          data-testid="inbox-toggle-selection-mode"
        >
          {bulkSelectionEnabled
            ? t("inbox.cancelSelection")
            : t("inbox.selectMessages")}
        </Button>
        {bulkSelectionEnabled ? (
          <>
            <Badge variant="neutral" size="sm">
              {t("inbox.selectedCount", {
                count: selectedMessageCount,
              })}
            </Badge>
            <Button
              type="button"
              size="sm"
              onClick={onDeleteSelected}
              disabled={selectedMessageCount === 0}
              icon={<Trash2 className="w-4 h-4" />}
              className="bg-red-500 hover:bg-red-600 active:bg-red-700 focus:ring-red-500"
              data-testid="inbox-delete-selected"
            >
              {t("inbox.deleteSelected")}
            </Button>
          </>
        ) : null}
        {unreadCount > 0 ? (
          <button
            onClick={onMarkAllRead}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-heading font-bold uppercase tracking-wider bg-white dark:bg-navy-800 text-gray-500 dark:text-gray-400 border border-gray-200 dark:border-navy-600 hover:text-primary-500 hover:border-primary-300 transition-all"
          >
            <CheckCheck className="w-3.5 h-3.5" />
            {t("inbox.markAllRead")}
          </button>
        ) : null}
        <button
          onClick={onClearOld}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-heading font-bold uppercase tracking-wider bg-white dark:bg-navy-800 text-gray-500 dark:text-gray-400 border border-gray-200 dark:border-navy-600 hover:text-red-500 hover:border-red-300 transition-all"
        >
          <Trash2 className="w-3.5 h-3.5" />
          {t("inbox.clearOld")}
        </button>
      </div>
    </div>
  );
}
