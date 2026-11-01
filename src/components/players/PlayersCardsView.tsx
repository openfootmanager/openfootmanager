import { useTranslation } from "react-i18next";

import { useRatingTiers } from "../../hooks/useRatingTiers";
import type { PlayerSummary, PlayersPageQuery } from "../../services/playersService";
import type { TeamData } from "../../store/gameStore";
import type { ContextMenuItem } from "../ContextMenu";
import { SortControl } from "../ui";
import PlayersCardGrid from "./PlayersCardGrid";
import { PLAYER_SORT_OPTIONS } from "./playerSortOptions";
import PlayersPagination from "./PlayersPagination";

interface PlayersCardsViewProps {
  items: readonly PlayerSummary[];
  query: PlayersPageQuery;
  /** The sort the control writes: the same query the table's headers write. */
  onSortChange: (key: PlayersPageQuery["sort_key"], ascending: boolean) => void;
  teams: TeamData[];
  contextItemsFor: (summary: PlayerSummary) => ContextMenuItem[];
  onSelectPlayer: (id: string) => void;
  onSelectTeam: (id: string) => void;
  total: number;
  fetchError: string | null;
  page: number;
  pageSize: number;
  totalPages: number;
  onPage: (page: number) => void;
}

/** The Players list as cards: a sort control (cards have no headers), the grid, and the page footer. */
export default function PlayersCardsView({
  items,
  query,
  onSortChange,
  teams,
  contextItemsFor,
  onSelectPlayer,
  onSelectTeam,
  total,
  fetchError,
  page,
  pageSize,
  totalPages,
  onPage,
}: PlayersCardsViewProps) {
  const { t } = useTranslation();
  const tierOf = useRatingTiers();
  return (
    <>
      <div className="mb-3 flex justify-end">
        <SortControl
          options={PLAYER_SORT_OPTIONS.map(({ key, labelKey }) => ({
            key,
            label: t(labelKey),
          }))}
          sortKey={query.sort_key}
          ascending={query.sort_asc}
          defaultAscending={(key) => key === "name"}
          onChange={onSortChange}
        />
      </div>
      {total === 0 ? (
        fetchError ? null : (
          <div className="p-8 text-center text-fg-muted text-sm">{t("players.noMatch")}</div>
        )
      ) : (
        <PlayersCardGrid
          items={items}
          tierOf={tierOf}
          kitOf={(teamId) => teams.find((tm) => tm.id === teamId)?.colors ?? null}
          contextItemsFor={contextItemsFor}
          onSelectPlayer={onSelectPlayer}
          onSelectTeam={onSelectTeam}
        />
      )}
      <PlayersPagination
        page={page}
        pageSize={pageSize}
        total={total}
        totalPages={totalPages}
        onPage={onPage}
      />
    </>
  );
}
