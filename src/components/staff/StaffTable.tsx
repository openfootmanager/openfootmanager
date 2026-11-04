import { MoreVertical } from "lucide-react";
import { useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { calcAge, formatWeeklyWage } from "../../lib/helpers";
import { visibleRatingText } from "../../lib/playerKnowledge";
import {
  nextSort,
  type SortDirection,
  type SortState,
  type SortValue,
  sortRows,
} from "../../lib/sortRows";
import type { StaffRow } from "../../services/staffService";
import ContextMenu, { type ContextMenuHandle, type ContextMenuItem } from "../ContextMenu";
import { CountryFlag, DataTable, type DataTableColumn, RatingChip } from "../ui";

type StaffSortKey =
  | "name"
  | "role"
  | "age"
  | "rating"
  | "coaching"
  | "judgingAbility"
  | "judgingPotential"
  | "physiotherapy"
  | "wage";

type StaffAttribute = keyof StaffRow["attributes"];
const ATTRIBUTES: readonly StaffAttribute[] = [
  "coaching",
  "judgingAbility",
  "judgingPotential",
  "physiotherapy",
];

/** The staff open on their rating, the best first, as a manager reads a staff list. */
const DEFAULT_SORT: SortState<StaffSortKey> = { key: "rating", direction: "desc" };

/** Names and roles read from A, and ages from the youngest; every other figure from the best. */
function firstDirection(key: StaffSortKey): SortDirection {
  return key === "name" || key === "role" || key === "age" ? "asc" : "desc";
}

/** What a member sorts by in each column; before the backend's slice he has no rating, so last. */
function sortValueOf(
  row: StaffRow,
  key: StaffSortKey,
  roleName: (role: string) => string,
): SortValue {
  switch (key) {
    case "name":
      return row.display_name;
    case "role":
      return roleName(row.role);
    case "age":
      return calcAge(row.date_of_birth);
    case "rating":
      return row.rating ?? null;
    case "wage":
      return row.wage;
    case "coaching":
    case "judgingAbility":
    case "judgingPotential":
    case "physiotherapy":
      return row.attributes[key];
  }
}

/** A row's actions: the card's own menu, opened from a button the keyboard reaches. */
function StaffRowMenu({ row, items }: { row: StaffRow; items: ContextMenuItem[] }) {
  const { t } = useTranslation();
  const menu = useRef<ContextMenuHandle>(null);
  const [menuOpen, setMenuOpen] = useState(false);
  return (
    <ContextMenu ref={menu} items={items} onOpenChange={setMenuOpen}>
      <button
        type="button"
        onClick={(event) => {
          const rect = event.currentTarget.getBoundingClientRect();
          menu.current?.open(rect.left, rect.bottom + 4);
        }}
        aria-label={t("staff.staffActions", { name: row.display_name })}
        aria-haspopup="menu"
        aria-expanded={menuOpen}
        className="rounded-md p-1.5 text-fg-subtle transition-colors hover:text-fg focus:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-focus"
      >
        <MoreVertical aria-hidden="true" className="h-4 w-4" />
      </button>
    </ContextMenu>
  );
}

interface StaffTableProps {
  /** Names the table for a screen reader: the club's staff, or the hiring market. */
  label: string;
  rows: readonly StaffRow[];
  /** A member's actions, the very menu his card offers. */
  menuItemsFor: (row: StaffRow) => ContextMenuItem[];
}

/**
 * The Staff list as a table (table C): name, role, age, the backend's rating in its tier's tone (#589's single
 * source), the four attributes, specialization and wage, sortable through `sortRows`, and the
 * card's actions on each row.
 */
export default function StaffTable({ label, rows, menuItemsFor }: StaffTableProps) {
  const { t, i18n } = useTranslation();
  const [sort, setSort] = useState(DEFAULT_SORT);
  const roleName = (role: string) => t(`staff.roles.${role}`);
  const sorted = useMemo(
    () =>
      sortRows(
        rows,
        (row) => sortValueOf(row, sort.key, (role) => t(`staff.roles.${role}`)),
        sort.direction,
        i18n.language,
      ),
    [rows, sort, i18n.language, t],
  );

  const columns: DataTableColumn<StaffRow>[] = [
    {
      key: "name",
      header: t("common.name"),
      primary: true,
      sortable: true,
      cell: (row) => <span className="font-semibold">{row.display_name}</span>,
    },
    {
      key: "role",
      header: t("worldEditor.staffRole.label"),
      sortable: true,
      cell: (row) => roleName(row.role),
    },
    {
      key: "age",
      header: t("common.age"),
      numeric: true,
      sortable: true,
      cell: (row) => calcAge(row.date_of_birth),
    },
    {
      key: "rating",
      header: t("common.ovr"),
      numeric: true,
      sortable: true,
      // In the tone of his tier among his own role, the backend's (staff_tier), as the card shows it.
      // A rating without its tier is not painted: the chip never picks one.
      cell: (row) =>
        row.rating === undefined || row.tier === undefined ? (
          visibleRatingText(null)
        ) : (
          <RatingChip value={row.rating} tier={row.tier} />
        ),
    },
    ...ATTRIBUTES.map((attribute) => ({
      key: attribute,
      header: t(`staff.attrs.${attribute}`),
      numeric: true,
      sortable: true,
      drawer: true,
      cell: (row: StaffRow) => row.attributes[attribute],
    })),
    {
      key: "specialization",
      header: t("worldEditor.staffSpecialization.label"),
      drawer: true,
      cell: (row) => (row.specialization ? t(`staff.specializations.${row.specialization}`) : "—"),
    },
    {
      key: "wage",
      header: t("common.wage"),
      numeric: true,
      sortable: true,
      cell: (row) => (row.wage > 0 ? formatWeeklyWage(row.wage, "compact") : "—"),
    },
  ];

  return (
    <div className="overflow-hidden rounded-lg border border-line bg-panel">
      <DataTable
        label={label}
        columns={columns}
        rows={sorted}
        rowKey={(row) => row.id}
        rowName={(row) => row.display_name}
        sort={sort}
        onSortChange={(key) =>
          setSort((current) =>
            nextSort(current, key as StaffSortKey, firstDirection(key as StaffSortKey)),
          )
        }
        primaryAside={(row) => (
          <CountryFlag
            code={row.nationality}
            locale={i18n.language}
            className="text-body leading-none"
          />
        )}
        rowAction={{
          header: t("common.actions"),
          cell: (row) => <StaffRowMenu row={row} items={menuItemsFor(row)} />,
        }}
      />
    </div>
  );
}
