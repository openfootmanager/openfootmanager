import { useTranslation } from "react-i18next";

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
          {t("worldEditor.home.issues", { count: check.count })}
        </span>
      );
    case "unreadable":
      return (
        <span
          className={`${CHIP} bg-danger-tint text-danger`}
          title={t(check.errorKey, { defaultValue: t("worldEditor.home.unreadable") })}
        >
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
  const date = new Intl.DateTimeFormat(i18n.language, {
    day: "numeric",
    month: "short",
    year: "numeric",
  });
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
      cell: (row) => (row.openedAt ? date.format(new Date(row.openedAt)) : dash),
    },
    {
      key: "check",
      header: t("worldEditor.home.colCheck"),
      drawer: true,
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
            disabled={busy || row.check.kind === "unreadable"}
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
