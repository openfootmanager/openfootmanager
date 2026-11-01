import { useMemo } from "react";
import { useTranslation } from "react-i18next";

import { formatVal } from "../../lib/helpers";
import { visibleRatingText } from "../../lib/playerKnowledge";
import { type SortDirection, type SortState, type SortValue, sortRows } from "../../lib/sortRows";
import type { TeamCard } from "../../services/teamsService";
import { Badge, CountryFlag, DataTable, type DataTableColumn, TeamLogo } from "../ui";

export type TeamsSortKey =
  | "position"
  | "name"
  | "points"
  | "avgOvr"
  | "reputation"
  | "squad"
  | "value";

/** The directory opens in league order, as the backend sends it. */
export const TEAMS_DEFAULT_SORT: SortState<TeamsSortKey> = { key: "position", direction: "asc" };

/** A column's first direction: the table and the name read top down; every figure, best first. */
export function teamsSortFirstDirection(key: TeamsSortKey): SortDirection {
  return key === "position" || key === "name" ? "asc" : "desc";
}

/**
 * What a club sorts by in each column. An unranked club (position 0) and one whose average the
 * manager doesn't know have none, so they sort last.
 */
function sortValueOf(card: TeamCard, key: TeamsSortKey): SortValue {
  switch (key) {
    case "position":
      return card.league_pos > 0 ? card.league_pos : null;
    case "name":
      return card.team.name;
    case "points":
      return card.standing?.points ?? null;
    case "avgOvr":
      return card.avg_ovr;
    case "reputation":
      return card.team.reputation;
    case "squad":
      return card.roster_size;
    case "value":
      return card.total_value;
  }
}

interface TeamsLeagueTableProps {
  /** The league's name, which names the table for a screen reader. */
  label: string;
  cards: readonly TeamCard[];
  userTeamId: string | null;
  sort: SortState<TeamsSortKey>;
  onSort: (key: TeamsSortKey) => void;
  onSelectTeam: (id: string) => void;
}

/**
 * One league's clubs as a table (table C): position, name, the standing, the squad's average
 * (withheld for a club the manager doesn't know), reputation, squad size and value. No wage bill or
 * balance: a rival's finances are hidden.
 */
export default function TeamsLeagueTable({
  label,
  cards,
  userTeamId,
  sort,
  onSort,
  onSelectTeam,
}: TeamsLeagueTableProps) {
  const { t, i18n } = useTranslation();
  const rows = useMemo(
    () => sortRows(cards, (card) => sortValueOf(card, sort.key), sort.direction),
    [cards, sort],
  );

  const columns: DataTableColumn<TeamCard>[] = [
    {
      key: "position",
      header: "#",
      headerLabel: t("common.position"),
      numeric: true,
      sortable: true,
      cell: (card) => (card.league_pos > 0 ? card.league_pos : "—"),
    },
    {
      key: "name",
      header: t("common.team"),
      primary: true,
      sortable: true,
      cell: (card) => (
        <span className="flex min-w-0 items-center gap-2">
          <TeamLogo
            team={card.team}
            className="grid h-6 w-6 shrink-0 place-items-center overflow-hidden"
            imageClassName="h-6 w-6 object-contain"
          />
          <span className="truncate font-semibold">{card.team.name}</span>
        </span>
      ),
    },
    {
      key: "played",
      header: t("common.played"),
      numeric: true,
      cell: (card) => card.standing?.played ?? "—",
    },
    {
      key: "points",
      header: t("common.pts"),
      numeric: true,
      sortable: true,
      cell: (card) => card.standing?.points ?? "—",
    },
    {
      key: "avgOvr",
      header: t("teams.avgOvr"),
      numeric: true,
      sortable: true,
      cell: (card) => visibleRatingText(card.avg_ovr),
    },
    {
      key: "reputation",
      header: t("teams.rep"),
      numeric: true,
      sortable: true,
      cell: (card) => card.team.reputation,
    },
    {
      key: "squad",
      header: t("teams.squad"),
      numeric: true,
      sortable: true,
      drawer: true,
      cell: (card) => card.roster_size,
    },
    {
      key: "value",
      header: t("common.value"),
      numeric: true,
      sortable: true,
      drawer: true,
      cell: (card) => formatVal(card.total_value),
    },
  ];

  return (
    <DataTable
      label={label}
      columns={columns}
      rows={rows}
      rowKey={(card) => card.team.id}
      rowName={(card) => card.team.name}
      sort={sort}
      onSortChange={(key) => onSort(key as TeamsSortKey)}
      onOpen={(card) => onSelectTeam(card.team.id)}
      rowAccent={(card) => (card.team.id === userTeamId ? "selected" : null)}
      primaryAside={(card) => (
        <>
          <CountryFlag
            code={card.team.country}
            locale={i18n.language}
            className="text-body leading-none"
          />
          {card.team.id === userTeamId ? (
            <Badge variant="accent" size="sm">
              {t("teams.yourTeam")}
            </Badge>
          ) : null}
        </>
      )}
    />
  );
}
