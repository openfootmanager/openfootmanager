import { ChevronLeft, ChevronRight, ChevronsLeft, ChevronsRight } from "lucide-react";
import { useTranslation } from "react-i18next";

interface PlayersPaginationProps {
  page: number;
  pageSize: number;
  total: number;
  totalPages: number;
  onPage: (page: number) => void;
}

/** The Players list's page footer, shared by its table and card views; drawn only past one page. */
export default function PlayersPagination({
  page,
  pageSize,
  total,
  totalPages,
  onPage,
}: PlayersPaginationProps) {
  const { t } = useTranslation();
  if (totalPages <= 1) return null;
  return (
    <div className="flex items-center justify-between px-4 py-3 border-t border-line">
      <p className="text-xs text-fg-muted font-heading">
        {t("players.showingRange", {
          from: (page - 1) * pageSize + 1,
          to: Math.min(page * pageSize, total),
          total,
        })}
      </p>
      <div className="flex items-center gap-1">
        <button
          type="button"
          onClick={() => onPage(1)}
          aria-label={t("common.firstPage")}
          disabled={page === 1}
          className="p-1.5 rounded-lg focus:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-page text-fg-muted hover:text-fg hover:bg-hover disabled:opacity-30 disabled:pointer-events-none transition-colors"
        >
          <ChevronsLeft aria-hidden="true" className="w-4 h-4" />
        </button>
        <button
          type="button"
          onClick={() => onPage(Math.max(1, page - 1))}
          aria-label={t("common.previousPage")}
          disabled={page === 1}
          className="p-1.5 rounded-lg focus:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-page text-fg-muted hover:text-fg hover:bg-hover disabled:opacity-30 disabled:pointer-events-none transition-colors"
        >
          <ChevronLeft aria-hidden="true" className="w-4 h-4" />
        </button>
        <span className="px-3 py-1 text-xs font-heading font-bold text-fg-muted">
          {page} / {totalPages}
        </span>
        <button
          type="button"
          onClick={() => onPage(Math.min(totalPages, page + 1))}
          aria-label={t("common.nextPage")}
          disabled={page === totalPages}
          className="p-1.5 rounded-lg focus:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-page text-fg-muted hover:text-fg hover:bg-hover disabled:opacity-30 disabled:pointer-events-none transition-colors"
        >
          <ChevronRight aria-hidden="true" className="w-4 h-4" />
        </button>
        <button
          type="button"
          onClick={() => onPage(totalPages)}
          aria-label={t("common.lastPage")}
          disabled={page === totalPages}
          className="p-1.5 rounded-lg focus:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-page text-fg-muted hover:text-fg hover:bg-hover disabled:opacity-30 disabled:pointer-events-none transition-colors"
        >
          <ChevronsRight aria-hidden="true" className="w-4 h-4" />
        </button>
      </div>
    </div>
  );
}
