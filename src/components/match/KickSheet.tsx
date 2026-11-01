import { Check, X } from "lucide-react";
import { useTranslation } from "react-i18next";

import { type KickCell, type KickSheet as Sheet, REGULAR_ROUNDS } from "./shootoutSheet";

const RESULT_KEY: Record<KickCell["state"], string> = {
  scored: "match.shootout.scored",
  missed: "match.shootout.missed",
  toCome: "match.shootout.toCome",
  notTaken: "match.shootout.notTaken",
};

const RESULT_TONE: Record<KickCell["state"], string> = {
  scored: "text-positive",
  missed: "text-danger",
  toCome: "text-fg-subtle",
  notTaken: "text-fg-subtle",
};

/** The round's marker: a tick or a cross on its tint, or the round's number while it is to come. */
function KickMark({ cell, round }: { cell: KickCell; round: number }) {
  const base = "grid size-6 shrink-0 place-items-center rounded-full";
  if (cell.state === "scored") {
    return (
      <span aria-hidden="true" className={`${base} bg-positive-tint text-positive`}>
        <Check className="size-3.5" strokeWidth={3} />
      </span>
    );
  }
  if (cell.state === "missed") {
    return (
      <span aria-hidden="true" className={`${base} bg-danger-tint text-danger`}>
        <X className="size-3.5" strokeWidth={3} />
      </span>
    );
  }
  return (
    <span
      aria-hidden="true"
      className={`${base} border-2 border-dashed border-line-strong font-heading text-caption font-bold text-fg-subtle`}
    >
      {cell.state === "toCome" ? round : ""}
    </span>
  );
}

function KickSheetCell({ cell, round }: { cell: KickCell; round: number }) {
  const { t } = useTranslation();
  const named = cell.state === "scored" || cell.state === "missed";
  return (
    <td className="h-row border-b border-line px-3">
      <span className="flex min-w-0 items-center gap-2">
        <KickMark cell={cell} round={round} />
        {/* A long name is cut short on a narrow sheet; the tooltip keeps it whole. */}
        <span
          className="min-w-0 truncate text-fg"
          title={named ? (cell.kickerName ?? undefined) : undefined}
        >
          {named ? (cell.kickerName ?? "") : ""}
        </span>
        {/* The word stays for a screen reader where a narrow sheet folds it into the mark. */}
        <span
          className={`ml-auto shrink-0 text-caption font-semibold ${RESULT_TONE[cell.state]} sr-only @md:not-sr-only`}
        >
          {t(RESULT_KEY[cell.state])}
        </span>
      </span>
    </td>
  );
}

function SheetRow({ row, current }: { row: Sheet["rows"][number]; current: boolean }) {
  return (
    <tr aria-current={current ? "step" : undefined} className={current ? "bg-selected-tint" : ""}>
      <th
        scope="row"
        className={`h-row w-12 border-b border-l-3 border-line px-3 text-left font-heading text-lead font-bold tabular-nums ${
          current ? "border-l-selected text-selected" : "border-l-transparent text-fg-muted"
        }`}
      >
        {row.round}
      </th>
      <KickSheetCell cell={row.home} round={row.round} />
      <KickSheetCell cell={row.away} round={row.round} />
    </tr>
  );
}

/**
 * The shootout as a referee's sheet: a row per round, a column per club, each kicker and result in
 * its cell, and the sudden-death rounds under their own heading. The round under way is marked
 * with the selected role.
 */
export function KickSheet({
  sheet,
  homeName,
  awayName,
}: {
  sheet: Sheet;
  homeName: string;
  awayName: string;
}) {
  const { t } = useTranslation();
  const regular = sheet.rows.slice(0, REGULAR_ROUNDS);
  const suddenDeath = sheet.rows.slice(REGULAR_ROUNDS);
  const headerCell =
    "h-8 border-b border-line-strong px-3 text-left text-caption font-medium text-fg-subtle";

  return (
    <div className="@container w-full">
      <table className="w-full table-fixed border-separate border-spacing-0 text-body">
        <caption className="sr-only">{t("match.shootout.sheetCaption")}</caption>
        <thead>
          <tr>
            <th scope="col" className={`${headerCell} w-12`}>
              {t("match.shootout.kickColumn")}
            </th>
            <th scope="col" className={headerCell}>
              {homeName}
            </th>
            <th scope="col" className={headerCell}>
              {awayName}
            </th>
          </tr>
        </thead>
        <tbody>
          {regular.map((row) => (
            <SheetRow key={row.round} row={row} current={row.round === sheet.currentRound} />
          ))}
        </tbody>
        {suddenDeath.length > 0 && (
          <tbody>
            <tr>
              <th
                scope="rowgroup"
                colSpan={3}
                className="h-7 border-b border-line bg-sunken text-center font-heading text-caption font-bold uppercase tracking-label text-warning"
              >
                {t("match.shootout.suddenDeath")}
              </th>
            </tr>
            {suddenDeath.map((row) => (
              <SheetRow key={row.round} row={row} current={row.round === sheet.currentRound} />
            ))}
          </tbody>
        )}
      </table>
    </div>
  );
}
