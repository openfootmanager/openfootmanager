import { useTranslation } from "react-i18next";

import type { ContractRiskLevel } from "../../lib/contractUtils";
import type { PlayerData } from "../../store/gameStore";
import type { TeamColors } from "../../store/types";
import type { ContextMenuItem } from "../ContextMenu";
import { SortControl } from "../ui";
import SquadCardGrid, { type DeployedPosition } from "./SquadCardGrid";
import type { SquadListSortKey, SquadListSortState } from "./SquadRosterView.state";

/** The fields the cards sort by: the roster table's sortable columns, named as its headers are. */
const SQUAD_SORT_OPTIONS: readonly { key: SquadListSortKey; labelKey: string }[] = [
  { key: "jersey", labelKey: "squad.jerseyNumberFull" },
  { key: "name", labelKey: "common.name" },
  { key: "pos", labelKey: "squad.pos" },
  { key: "age", labelKey: "common.age" },
  { key: "condition", labelKey: "common.condition" },
  { key: "morale", labelKey: "common.morale" },
  { key: "ovr", labelKey: "common.ovr" },
  { key: "contract", labelKey: "common.contract" },
];

interface SquadCardsViewProps {
  /** The roster as filtered and sorted for the table: the cards draw the same list. */
  players: readonly PlayerData[];
  teamName: string;
  kit: TeamColors;
  isStarter: (player: PlayerData) => boolean;
  deployedPositionOf: (player: PlayerData) => DeployedPosition | null;
  riskLevelOf: (playerId: string) => ContractRiskLevel;
  menuItemsFor: (player: PlayerData) => ContextMenuItem[];
  onSelectPlayer: (id: string) => void;
  sortKey: SquadListSortState["sortKey"];
  sortDir: SquadListSortState["sortDir"];
  /** The table header's toggle: a new field starts in its own direction, the same one flips. */
  onSort: (key: SquadListSortKey) => void;
}

/**
 * The squad roster as cards: a sort control (cards have no headers to click), then the grid, or
 * the roster's no-players line when the filters leave nobody.
 */
export default function SquadCardsView({
  players,
  sortKey,
  sortDir,
  onSort,
  ...grid
}: SquadCardsViewProps) {
  const { t } = useTranslation();
  const ascending = sortDir === "asc";
  return (
    <div className="p-4">
      <div className="mb-3 flex justify-end">
        <SortControl
          options={SQUAD_SORT_OPTIONS.map(({ key, labelKey }) => ({ key, label: t(labelKey) }))}
          sortKey={sortKey}
          ascending={ascending}
          onChange={(key, nextAscending) => {
            // The header's toggle owns each field's starting direction. Picking the field already
            // chosen again leaves the sort as it is, where a header click would flip it.
            if (key !== sortKey || nextAscending !== ascending) onSort(key);
          }}
        />
      </div>
      {players.length === 0 ? (
        <p className="p-8 text-center text-body text-fg-muted">{t("squad.noPlayers")}</p>
      ) : (
        <SquadCardGrid players={players} {...grid} />
      )}
    </div>
  );
}
