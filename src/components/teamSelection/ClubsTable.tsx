import { useId, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { useRatingTiers } from "../../hooks/useRatingTiers";
import { weeklyWageAmount } from "../../lib/finance";
import { formatVal, formatWeeklyWage } from "../../lib/helpers";
import {
  nextSort,
  type SortDirection,
  type SortState,
  type SortValue,
  sortRows,
} from "../../lib/sortRows";
import type { TeamData } from "../../store/gameStore";
import { DataTable, type DataTableColumn, RatingChip, TeamLogo } from "../ui";

type ClubSortKey = "name" | "squad" | "transfers" | "wages" | "stadium";

/** The table opens strongest squad first, as the board's mock does. */
const DEFAULT_SORT: SortState<ClubSortKey> = { key: "squad", direction: "desc" };

/** A column's first direction: the names read top down; every figure, biggest first. */
function firstDirection(key: ClubSortKey): SortDirection {
  return key === "name" ? "asc" : "desc";
}

interface ClubsTableProps {
  /** The league's name, which names the card. */
  leagueName: string;
  clubs: readonly TeamData[];
  /** The league's clubs counted before a search narrows them. */
  clubCount: number;
  search: string;
  onSearchChange: (value: string) => void;
  selectedTeamId: string | null;
  onSelectTeam: (id: string) => void;
  /** The club's average as the backend rates it; null until it answers. */
  getTeamAvgOvr: (teamId: string) => number | null;
}

/**
 * The league's clubs to choose from. A club's name wraps and is never cut off, with its city under
 * it; the figures after it fold into the row's details in a narrow window. The pick is the row's
 * name button (`aria-pressed`), which a click anywhere on the row presses too. A club's squad is
 * painted in the tier the backend gives it.
 */
export default function ClubsTable({
  leagueName,
  clubs,
  clubCount,
  search,
  onSearchChange,
  selectedTeamId,
  onSelectTeam,
  getTeamAvgOvr,
}: ClubsTableProps) {
  const { t, i18n } = useTranslation();
  const tierOf = useRatingTiers();
  const headingId = useId();
  const [sort, setSort] = useState<SortState<ClubSortKey>>(DEFAULT_SORT);

  const rows = useMemo(() => {
    const sortValueOf = (team: TeamData): SortValue => {
      switch (sort.key) {
        case "name":
          return team.name;
        case "squad":
          return getTeamAvgOvr(team.id);
        case "transfers":
          return team.transfer_budget;
        case "wages":
          return team.wage_budget;
        case "stadium":
          return team.stadium_capacity;
      }
    };
    return sortRows(clubs, sortValueOf, sort.direction, i18n.language);
  }, [clubs, sort, getTeamAvgOvr, i18n.language]);

  function onSortChange(key: string) {
    const next = key as ClubSortKey;
    setSort((current) => nextSort(current, next, firstDirection(next)));
  }

  const columns: DataTableColumn<TeamData>[] = [
    {
      key: "name",
      header: t("teamSelect.table.club"),
      primary: true,
      sortable: true,
      wrap: true,
      cell: (team) => (
        <span className="flex min-w-0 items-center gap-2.5">
          <TeamLogo
            team={team}
            className="grid size-7 shrink-0 place-items-center overflow-hidden"
            imageClassName="size-7 object-contain"
          />
          <span className="flex min-w-0 flex-col">
            <span className="font-semibold leading-tight">{team.name}</span>
            <span className="text-caption text-fg-muted">{team.city}</span>
          </span>
        </span>
      ),
    },
    {
      key: "squad",
      header: t("teamSelect.squad"),
      numeric: true,
      sortable: true,
      cell: (team) => {
        const average = getTeamAvgOvr(team.id);
        return average === null ? "\u2014" : <RatingChip value={average} tier={tierOf(average)} />;
      },
    },
    {
      key: "transfers",
      header: t("teamSelect.table.transfers"),
      numeric: true,
      sortable: true,
      drawer: true,
      cell: (team) => formatVal(team.transfer_budget),
    },
    {
      key: "wages",
      header: t("teamSelect.table.wages"),
      numeric: true,
      sortable: true,
      drawer: true,
      cell: (team) => formatWeeklyWage(weeklyWageAmount(team.wage_budget), "compact"),
    },
    {
      key: "stadium",
      header: t("teamSelect.table.stadium"),
      numeric: true,
      sortable: true,
      drawer: true,
      cell: (team) => team.stadium_capacity.toLocaleString(i18n.language),
    },
  ];

  return (
    <section
      aria-labelledby={headingId}
      className="flex min-h-0 flex-col overflow-hidden rounded-xl border border-line bg-panel shadow-card"
    >
      <div className="flex min-h-11 shrink-0 flex-wrap items-center justify-between gap-x-3 gap-y-1 border-b border-line px-3.5 py-1.5">
        <h2
          id={headingId}
          className="flex flex-wrap items-baseline gap-x-2.5 font-heading text-heading font-semibold uppercase tracking-label"
        >
          {leagueName}
          <span className="font-sans text-caption font-normal normal-case tracking-normal text-fg-subtle">
            {t("teamSelect.clubCount", { n: clubCount })}
          </span>
        </h2>
        {/* Said aloud as the search narrows the list: the table changes under the typing. */}
        <p role="status" className="sr-only">
          {t("teamSelect.clubCount", { n: clubs.length })}
        </p>
        <input
          type="search"
          value={search}
          onChange={(event) => onSearchChange(event.target.value)}
          placeholder={t("teamSelect.searchClubs")}
          aria-label={t("teamSelect.searchClubs")}
          className="h-8 w-44 rounded-lg border border-line-strong bg-panel px-2.5 text-body text-fg placeholder:text-fg-subtle focus:outline-none focus-visible:ring-2 focus-visible:ring-focus"
        />
      </div>
      <div className="min-h-0 flex-1 overflow-auto">
        <DataTable
          label={t("teamSelect.table.label")}
          columns={columns}
          rows={rows}
          rowKey={(team) => team.id}
          rowName={(team) => team.name}
          sort={sort}
          onSortChange={onSortChange}
          onOpen={(team) => onSelectTeam(team.id)}
          rowAccent={(team) => (team.id === selectedTeamId ? "selected" : null)}
          rowPressed={(team) => team.id === selectedTeamId}
          empty={t("teamSelect.noClubsMatch")}
        />
      </div>
    </section>
  );
}
