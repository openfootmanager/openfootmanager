import { useTranslation } from "react-i18next";

import { useRatingTiers } from "../../hooks/useRatingTiers";
import { formatVal } from "../../lib/helpers";
import { visibleRatingText } from "../../lib/playerKnowledge";
import { naturalPositionOf } from "../../lib/positions";
import { ratingTierClass } from "../../lib/ratingTier";
import type { PlayerSortKey, PlayerSummary, PlayersPageQuery } from "../../services/playersService";
import ContextMenu, { type ContextMenuItem } from "../ContextMenu";
import PlayerPositionBadge from "../PlayerPositionBadge";
import { Badge, Card, CardBody, CountryFlag, PlayerAvatar } from "../ui";
import { SortHeaderButton } from "../ui/DataTable";
import PlayersPagination from "./PlayersPagination";

interface PlayersTableViewProps {
  items: readonly PlayerSummary[];
  query: PlayersPageQuery;
  /** A header was pressed: sorts by that column, or turns the order round if it is the sort. */
  onSort: (key: PlayerSortKey) => void;
  contextItemsFor: (summary: PlayerSummary) => ContextMenuItem[];
  onSelectPlayer: (id: string) => void;
  onSelectTeam: (id: string) => void;
  total: number;
  /** A load error is shown above the list; it must not be answered with "no matches" too. */
  fetchError: string | null;
  page: number;
  pageSize: number;
  totalPages: number;
  onPage: (page: number) => void;
}

/** The Players list as a table: sortable headers, a row per player, and the page footer. */
export default function PlayersTableView({
  items,
  query,
  onSort,
  contextItemsFor,
  onSelectPlayer,
  onSelectTeam,
  total,
  fetchError,
  page,
  pageSize,
  totalPages,
  onPage,
}: PlayersTableViewProps) {
  const { t } = useTranslation();
  const tierOf = useRatingTiers();
  return (
    <Card>
      <CardBody className="p-0">
        <div className="overflow-x-auto">
          <table className="w-full text-left border-collapse">
            <thead>
              <tr className="border-b border-line-strong bg-panel text-caption">
                <SortHeader
                  label={t("common.position")}
                  sortKey="position"
                  current={query.sort_key}
                  asc={query.sort_asc}
                  onClick={onSort}
                />
                <SortHeader
                  label={t("common.name")}
                  sortKey="name"
                  current={query.sort_key}
                  asc={query.sort_asc}
                  onClick={onSort}
                />
                <SortHeader
                  label={t("common.age")}
                  sortKey="age"
                  current={query.sort_key}
                  asc={query.sort_asc}
                  onClick={onSort}
                />
                <th
                  scope="col"
                  className="px-4 py-3 text-left text-caption font-medium text-fg-subtle"
                >
                  {t("common.nationality")}
                </th>
                <SortHeader
                  label={t("common.team")}
                  sortKey="team"
                  current={query.sort_key}
                  asc={query.sort_asc}
                  onClick={onSort}
                />
                <SortHeader
                  label={t("common.value")}
                  sortKey="value"
                  current={query.sort_key}
                  asc={query.sort_asc}
                  onClick={onSort}
                />
                <SortHeader
                  label={t("common.ovr")}
                  sortKey="ovr"
                  current={query.sort_key}
                  asc={query.sort_asc}
                  onClick={onSort}
                />
                <th
                  scope="col"
                  className="px-4 py-3 text-left text-caption font-medium text-fg-subtle"
                >
                  {t("common.status")}
                </th>
              </tr>
            </thead>
            <tbody className="divide-y divide-line">
              {items.map((summary) => {
                const contextItems = contextItemsFor(summary);

                const ovr = summary.ovr;
                const row = (
                  <tr
                    key={summary.id}
                    onClick={() => onSelectPlayer(summary.id)}
                    className="hover:bg-hover transition-colors cursor-pointer group"
                  >
                    <td className="py-2.5 px-4">
                      <PlayerPositionBadge position={naturalPositionOf(summary)} />
                    </td>
                    <td className="py-2.5 px-4">
                      <div className="flex items-center gap-3">
                        <PlayerAvatar player={summary} />
                        <span className="font-semibold text-body text-fg group-hover:text-selected transition-colors">
                          {summary.match_name}
                        </span>
                      </div>
                    </td>
                    <td className="py-2.5 px-4 text-body text-fg-muted tabular-nums">
                      {summary.age}
                    </td>
                    <td className="py-2.5 px-4 text-body text-fg-muted" title={summary.nationality}>
                      <CountryFlag
                        code={summary.nationality}
                        className="text-heading leading-none"
                      />
                    </td>
                    <td className="py-2.5 px-4">
                      {summary.team_id ? (
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation();
                            onSelectTeam(summary.team_id!);
                          }}
                          className="rounded-sm text-body text-fg-muted hover:text-fg hover:underline transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-panel"
                        >
                          {summary.team_name ?? ""}
                        </button>
                      ) : (
                        <span className="text-body text-fg-muted">{t("common.freeAgent")}</span>
                      )}
                    </td>
                    <td className="py-2.5 px-4 text-body text-fg-muted font-medium">
                      {formatVal(summary.market_value)}
                    </td>
                    <td className="py-2.5 px-4">
                      <span
                        className={`font-heading font-bold text-lead tabular-nums ${ratingTierClass(
                          ovr === null ? "Rest" : tierOf(ovr),
                          "text",
                        )}`}
                      >
                        {visibleRatingText(ovr)}
                      </span>
                    </td>
                    <td className="py-2.5 px-4">
                      {summary.transfer_listed && (
                        <Badge variant="accent" size="sm">
                          {t("transfers.transfer")}
                        </Badge>
                      )}
                      {summary.loan_listed && (
                        <Badge variant="primary" size="sm">
                          {t("transfers.loan")}
                        </Badge>
                      )}
                      {summary.injured && (
                        <Badge variant="danger" size="sm">
                          {t("common.injured")}
                        </Badge>
                      )}
                    </td>
                  </tr>
                );

                return (
                  <ContextMenu items={contextItems} key={summary.id}>
                    {row}
                  </ContextMenu>
                );
              })}
            </tbody>
          </table>
          {total === 0 && !fetchError && (
            <div className="p-8 text-center text-fg-muted text-body">{t("players.noMatch")}</div>
          )}
        </div>
        <PlayersPagination
          surface="panel"
          page={page}
          pageSize={pageSize}
          total={total}
          totalPages={totalPages}
          onPage={onPage}
        />
      </CardBody>
    </Card>
  );
}

function SortHeader({
  label,
  sortKey,
  current,
  asc,
  onClick,
}: {
  label: string;
  sortKey: PlayerSortKey;
  current: PlayerSortKey;
  asc: boolean;
  onClick: (k: PlayerSortKey) => void;
}) {
  const direction = current === sortKey ? (asc ? "asc" : "desc") : null;
  return (
    <th
      scope="col"
      aria-sort={direction ? (direction === "asc" ? "ascending" : "descending") : undefined}
      className={`text-left text-caption ${
        direction ? "font-semibold text-fg" : "font-medium text-fg-subtle"
      }`}
    >
      <SortHeaderButton
        header={label}
        direction={direction}
        onSort={() => onClick(sortKey)}
        className="px-4 py-3"
      />
    </th>
  );
}
