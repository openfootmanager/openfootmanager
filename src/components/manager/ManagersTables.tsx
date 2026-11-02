import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import {
  nextSort,
  type SortDirection,
  type SortState,
  type SortValue,
  sortRows,
} from "../../lib/sortRows";
import { CountryFlag, DataTable, type DataTableColumn } from "../ui";
import type { ManagerRow, ManagersDirectory, VacancyRow } from "./managerRows";

type ManagerSortKey = "name" | "club" | "reputation" | "matches" | "trophies";

/** The directory opens on the most reputable managers, as the cards always have. */
const DEFAULT_SORT: SortState<ManagerSortKey> = { key: "reputation", direction: "desc" };

/** Names read from A; every figure, best first. */
function firstDirection(key: ManagerSortKey): SortDirection {
  return key === "name" || key === "club" ? "asc" : "desc";
}

/** What a manager sorts by in each column; one between jobs has no club, so he sorts last. */
function sortValueOf(row: ManagerRow, key: ManagerSortKey): SortValue {
  switch (key) {
    case "name":
      return row.display_name;
    case "club":
      return row.club?.name ?? null;
    case "reputation":
      return row.reputation;
    case "matches":
      return row.record.matches;
    case "trophies":
      return row.record.trophies;
  }
}

/** A club's name as a button that opens it. */
function ClubButton({ name, onOpen }: { name: string; onOpen: () => void }) {
  return (
    <button
      type="button"
      onClick={onOpen}
      className="rounded-sm text-left hover:underline focus:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-focus"
    >
      {name}
    </button>
  );
}

interface ManagersTablesProps {
  directory: ManagersDirectory;
  onSelectTeam?: (id: string) => void;
}

/**
 * The Managers directory as tables (table C): the managers, sortable, then the open jobs. A
 * manager's row opens nothing yet; his profile comes with the manager profile (M3). His club and
 * an open job's club open that club.
 */
export default function ManagersTables({ directory, onSelectTeam }: ManagersTablesProps) {
  const { t, i18n } = useTranslation();
  const [sort, setSort] = useState(DEFAULT_SORT);
  const managers = useMemo(
    () =>
      sortRows(
        directory.managers,
        (row) => sortValueOf(row, sort.key),
        sort.direction,
        i18n.language,
      ),
    [directory.managers, sort, i18n.language],
  );

  const managerColumns: DataTableColumn<ManagerRow>[] = [
    {
      key: "name",
      header: t("dashboard.manager"),
      primary: true,
      sortable: true,
      cell: (row) => <span className="font-semibold">{row.display_name}</span>,
    },
    {
      key: "club",
      header: t("managersWorld.currentClub"),
      sortable: true,
      cell: (row) =>
        row.club ? (
          <ClubButton name={row.club.name} onOpen={() => row.club && onSelectTeam?.(row.club.id)} />
        ) : (
          <span className="text-fg-muted">{t("managersWorld.unemployed")}</span>
        ),
    },
    {
      key: "reputation",
      header: t("managersWorld.reputation"),
      numeric: true,
      sortable: true,
      cell: (row) => row.reputation,
    },
    {
      key: "matches",
      header: t("managersWorld.matches"),
      numeric: true,
      sortable: true,
      cell: (row) => row.record.matches,
    },
    {
      key: "trophies",
      header: t("manager.trophies"),
      numeric: true,
      sortable: true,
      cell: (row) => row.record.trophies,
    },
  ];

  const vacancyColumns: DataTableColumn<VacancyRow>[] = [
    {
      key: "club",
      header: t("managersWorld.currentClub"),
      primary: true,
      cell: (row) => <ClubButton name={row.club_name} onOpen={() => onSelectTeam?.(row.club_id)} />,
    },
    {
      key: "reputation",
      header: t("managersWorld.reputation"),
      numeric: true,
      cell: (row) => row.club_reputation,
    },
  ];

  return (
    <div className="flex flex-col gap-5">
      <div className="overflow-hidden rounded-lg border border-line bg-panel">
        <DataTable
          label={t("managersWorld.title")}
          columns={managerColumns}
          rows={managers}
          rowKey={(row) => row.id}
          rowName={(row) => row.display_name}
          sort={sort}
          onSortChange={(key) =>
            setSort((current) =>
              nextSort(current, key as ManagerSortKey, firstDirection(key as ManagerSortKey)),
            )
          }
          primaryAside={(row) => (
            <CountryFlag
              code={row.nationality}
              locale={i18n.language}
              className="text-body leading-none"
            />
          )}
        />
      </div>
      {directory.vacancies.length > 0 ? (
        <div className="overflow-hidden rounded-lg border border-line bg-panel">
          <DataTable
            label={t("managersWorld.vacancy")}
            columns={vacancyColumns}
            rows={directory.vacancies}
            rowKey={(row) => row.club_id}
            rowName={(row) => row.club_name}
          />
        </div>
      ) : null}
    </div>
  );
}
