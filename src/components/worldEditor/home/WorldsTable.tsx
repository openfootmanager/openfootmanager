import { useMemo } from "react";
import { useTranslation } from "react-i18next";

import { getLocale } from "../../../lib/dateFormatting";
import { resolveBackendText } from "../../../utils/backendI18n";

import { Button, DataTable, type DataTableColumn } from "../../ui";
import type { WorldCheck, WorldRow } from "./worldRows";

const CHIP = "inline-flex h-6 items-center rounded-md px-2 text-caption font-semibold";

function KindChip({ kind }: { kind: WorldRow["kind"] }) {
  const { t } = useTranslation();
  return kind === "project" ? (
    <span className={`${CHIP} bg-info-tint text-info`}>{t("worldEditor.home.kindProject")}</span>
  ) : (
    <span className={`${CHIP} border border-line bg-sunken text-fg-muted`}>
      {t("worldEditor.home.kindInstalled")}
    </span>
  );
}

function CheckChip({ check }: { check: WorldCheck }) {
  const { t } = useTranslation();
  switch (check.kind) {
    case "clean":
      return (
        <span className={`${CHIP} bg-positive-tint text-positive`}>
          {t("worldEditor.home.noIssues")}
        </span>
      );
    case "issues":
      return (
        <span className={`${CHIP} bg-warning-tint text-warning`}>
          {t("worldEditor.issuesBadge", { count: check.count })}
        </span>
      );
    case "unreadable":
      return (
        <span className={`${CHIP} bg-danger-tint text-danger`}>
          {t("worldEditor.home.unreadable")}
        </span>
      );
    default:
      return (
        <span className={`${CHIP} border border-line bg-sunken text-fg-muted`}>
          {t("worldEditor.home.notChecked")}
        </span>
      );
  }
}

interface WorldsTableProps {
  rows: readonly WorldRow[];
  /** A world is being opened: its buttons wait. */
  busy: boolean;
  loading: boolean;
  /** Said in place of the rows when there are none. */
  empty: string;
  /** Opens a project, or an installed package as an editable copy. */
  onOpen: (row: WorldRow) => void;
}

/**
 * Your worlds as a table: the world with its place, what kind it is, its clubs and players, when it
 * was last opened and what the last check found, and the one action that opens it. An installed
 * package opens as an editable copy, so the one the player plays stays as it is.
 */
export default function WorldsTable({ rows, busy, loading, empty, onOpen }: WorldsTableProps) {
  const { t, i18n } = useTranslation();
  // The time a project was opened is an instant, shown as the player's own day: `formatDate` reads
  // the date part of its text (the UTC day), which would be a day off near midnight.
  const date = useMemo(
    () =>
      new Intl.DateTimeFormat(getLocale(i18n.language), {
        day: "numeric",
        month: "short",
        year: "numeric",
      }),
    [i18n.language],
  );
  const dash = "—";

  const columns: DataTableColumn<WorldRow>[] = [
    {
      key: "world",
      header: t("worldEditor.home.colWorld"),
      primary: true,
      wrap: true,
      cell: (row) => (
        <span className="flex min-w-0 flex-col">
          <span className="font-semibold leading-tight">{row.name}</span>
          <span className="font-mono text-caption text-fg-subtle">
            {row.kind === "installed" && row.version
              ? t("worldEditor.home.installedVersion", { version: row.version })
              : row.path}
          </span>
          {row.check.kind === "unreadable" ? (
            // Why it cannot be read, in words and always visible: the check column may be folded away.
            <span className="text-caption text-danger">
              {resolveBackendText(row.check.errorKey, t("worldEditor.home.unreadable"))}
            </span>
          ) : null}
        </span>
      ),
    },
    {
      key: "kind",
      header: t("worldEditor.home.colKind"),
      cell: (row) => <KindChip kind={row.kind} />,
    },
    {
      key: "clubs",
      header: t("worldEditor.home.colClubs"),
      numeric: true,
      cell: (row) =>
        row.check.kind === "unreadable" ? dash : row.clubs.toLocaleString(i18n.language),
    },
    {
      key: "players",
      header: t("worldEditor.home.colPlayers"),
      numeric: true,
      drawer: true,
      cell: (row) =>
        row.check.kind === "unreadable" ? dash : row.players.toLocaleString(i18n.language),
    },
    {
      key: "opened",
      header: t("worldEditor.home.colOpened"),
      drawer: true,
      cell: (row) => {
        // A record the editor wrote is trusted for its shape only: one that is not a date reads a dash.
        const opened = row.openedAt ? new Date(row.openedAt) : null;
        return opened && !Number.isNaN(opened.getTime()) ? date.format(opened) : dash;
      },
    },
    {
      key: "check",
      header: t("worldEditor.home.colCheck"),
      cell: (row) => <CheckChip check={row.check} />,
    },
  ];

  return (
    <DataTable
      label={t("worldEditor.home.tableLabel")}
      columns={columns}
      rows={rows}
      rowKey={(row) => `${row.kind}:${row.path}`}
      rowName={(row) => row.name}
      loading={loading}
      empty={empty}
      rowAction={{
        header: t("worldEditor.home.colAction"),
        cell: (row) => (
          <Button
            size="sm"
            variant="outline"
            disabled={busy}
            aria-label={t(
              row.kind === "installed"
                ? "worldEditor.home.editCopyNamed"
                : "worldEditor.home.openNamed",
              { name: row.name },
            )}
            onClick={() => onOpen(row)}
          >
            {t(row.kind === "installed" ? "worldEditor.home.editCopy" : "worldEditor.home.open")}
          </Button>
        ),
      }}
    />
  );
}
