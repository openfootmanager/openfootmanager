import { AlertTriangle, ChevronDown, ChevronUp, Star } from "lucide-react";
import type { JSX } from "react";
import { useTranslation } from "react-i18next";

import { calcAge, getPlayerOvr, positionBadgeVariant } from "../../lib/helpers";
import type { PlayerData } from "../../store/gameStore";
import { TraitList } from "../TraitBadge";
import { getOverallRatingClassName, type SortKey } from "./TacticsTab.helpers";
import { Badge, Card, ProgressBar } from "../ui";
import {
  getPreferredPositions,
  isPlayerOutOfPosition,
  normalisePosition,
  translatePositionAbbreviation,
  type SquadSection,
} from "../squad/SquadTab.helpers";

interface TacticsPlayerTableProps {
  emptyMessage: string;
  highlightedPlayerId: string | null;
  onSelectPlayer: (playerId: string) => void;
  players: PlayerData[];
  section: SquadSection;
  sortDir: "asc" | "desc";
  sortKey: SortKey;
  title: string;
  toggleSort: (key: SortKey) => void;
  totalCount: number;
  xiActivePosition: Map<string, string>;
}

interface SortHeaderProps {
  column: SortKey;
  label: string;
  sortDir: "asc" | "desc";
  sortKey: SortKey;
  toggleSort: (key: SortKey) => void;
}

function renderPreferredPositionMeta(
  player: PlayerData,
  translate: (key: string, options?: { defaultValue?: string }) => string,
): JSX.Element {
  return (
    <div className="flex flex-wrap items-center gap-1.5 text-xs text-gray-400 dark:text-gray-500">
      {getPreferredPositions(player).map((position, index) => (
        <Badge
          key={`${player.id}-${position}`}
          variant={index === 0 ? positionBadgeVariant(position) : "neutral"}
          size="sm"
        >
          {translatePositionAbbreviation(translate, position)}
        </Badge>
      ))}
    </div>
  );
}

function SortHeader({
  column,
  label,
  sortDir,
  sortKey,
  toggleSort,
}: SortHeaderProps): JSX.Element {
  const isActive = sortKey === column;

  return (
    <th
      className={`cursor-pointer select-none px-4 py-2.5 font-heading font-bold uppercase tracking-wider transition-colors hover:text-primary-400 ${isActive
          ? "text-primary-500 dark:text-primary-400"
          : "text-gray-500 dark:text-gray-400"
        }`}
      onClick={() => toggleSort(column)}
    >
      <div className="flex items-center gap-1">
        {label}
        {isActive ? (
          sortDir === "asc" ? (
            <ChevronUp className="h-3 w-3" />
          ) : (
            <ChevronDown className="h-3 w-3" />
          )
        ) : null}
      </div>
    </th>
  );
}

function renderTableRow(props: {
  highlightedPlayerId: string | null;
  onSelectPlayer: (playerId: string) => void;
  player: PlayerData;
  section: SquadSection;
  xiActivePosition: Map<string, string>;
}): JSX.Element {
  const {
    highlightedPlayerId,
    onSelectPlayer,
    player,
    section,
    xiActivePosition,
  } = props;
  const { t } = useTranslation();
  const activePosition =
    section === "xi"
      ? (xiActivePosition.get(player.id) ?? player.position)
      : player.natural_position || player.position;
  const normalizedActivePosition = normalisePosition(activePosition);
  const isHighlighted = highlightedPlayerId === player.id;
  const isWrongPosition =
    section === "xi" && isPlayerOutOfPosition(player, activePosition);
  const overallRating = getPlayerOvr(player);

  return (
    <tr
      key={player.id}
      data-testid={`${section}-player-${player.id}`}
      onClick={() => onSelectPlayer(player.id)}
      className={`group cursor-pointer transition-colors ${isHighlighted
          ? "bg-primary-500/10 dark:bg-primary-500/10"
          : "hover:bg-gray-50 dark:hover:bg-navy-700/50"
        }`}
    >
      <td className="px-4 py-2.5">
        <div className="flex items-center gap-1.5">
          <Badge
            variant={positionBadgeVariant(normalizedActivePosition)}
            size="sm"
          >
            {translatePositionAbbreviation(t, activePosition)}
          </Badge>
          {isWrongPosition ? (
            <span
              title={t("squad.outOfPositionTooltip")}
              className="text-amber-500"
            >
              <AlertTriangle className="h-3.5 w-3.5" />
            </span>
          ) : null}
        </div>
      </td>
      <td className="px-4 py-2.5">
        <div className="text-sm font-semibold text-gray-900 transition-colors group-hover:text-primary-600 dark:text-gray-100 dark:group-hover:text-primary-400">
          {player.full_name}
        </div>
        {renderPreferredPositionMeta(player, t)}
      </td>
      <td className="px-4 py-2.5 text-sm tabular-nums text-gray-600 dark:text-gray-400">
        {calcAge(player.date_of_birth)}
      </td>
      <td className="w-28 px-4 py-2.5">
        <ProgressBar
          value={player.condition}
          variant="auto"
          size="sm"
          showLabel
        />
      </td>
      <td className="px-4 py-2.5 text-sm tabular-nums text-gray-500 dark:text-gray-400">
        {player.morale}
      </td>
      <td className="px-4 py-2.5">
        {player.traits.length > 0 ? (
          <TraitList traits={player.traits} size="xs" max={2} />
        ) : (
          <span className="text-xs text-gray-500">—</span>
        )}
      </td>
      <td className="px-4 py-2.5">
        <span
          className={`text-base font-heading font-bold tabular-nums ${getOverallRatingClassName(
            overallRating,
          )}`}
        >
          {overallRating}
        </span>
      </td>
      <td className="px-4 py-2.5">
        <div className="flex flex-wrap items-center gap-1.5">
          {player.injury ? (
            <Badge variant="danger" size="sm">
              {t("common.injured")}
            </Badge>
          ) : (
            <span className="text-xs text-gray-500">—</span>
          )}
        </div>
      </td>
    </tr>
  );
}

export default function TacticsPlayerTable({
  emptyMessage,
  highlightedPlayerId,
  onSelectPlayer,
  players,
  section,
  sortDir,
  sortKey,
  title,
  toggleSort,
  totalCount,
  xiActivePosition,
}: TacticsPlayerTableProps): JSX.Element {
  const { t } = useTranslation();
  const headingClassName =
    section === "xi"
      ? "rounded-t-xl border-b border-gray-100 bg-linear-to-r from-navy-700 to-navy-800 p-4 dark:border-navy-600"
      : "border-b border-gray-100 p-4 dark:border-navy-600";
  const titleClassName =
    section === "xi"
      ? "flex items-center gap-2 text-sm font-heading font-bold uppercase tracking-wide text-white"
      : "flex items-center gap-2 text-sm font-heading font-bold uppercase tracking-wide text-gray-800 dark:text-gray-200";
  const countClassName =
    section === "xi"
      ? "mt-0.5 text-xs text-gray-400"
      : "mt-0.5 text-xs text-gray-400";

  return (
    <Card>
      <div className={headingClassName}>
        <h3 className={titleClassName}>
          {section === "xi" ? (
            <Star className="h-4 w-4 fill-current text-accent-400" />
          ) : null}
          {title}
        </h3>
        <p className={countClassName}>
          {players.length} / {totalCount} {t("squad.playersLabel")}
        </p>
      </div>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-left">
          <thead>
            <tr className="border-b border-gray-200 bg-gray-50 text-xs dark:border-navy-600 dark:bg-navy-800">
              <SortHeader
                column="pos"
                label={t("squad.pos")}
                sortDir={sortDir}
                sortKey={sortKey}
                toggleSort={toggleSort}
              />
              <SortHeader
                column="name"
                label={t("common.name")}
                sortDir={sortDir}
                sortKey={sortKey}
                toggleSort={toggleSort}
              />
              <SortHeader
                column="age"
                label={t("common.age")}
                sortDir={sortDir}
                sortKey={sortKey}
                toggleSort={toggleSort}
              />
              <SortHeader
                column="condition"
                label={t("common.condition")}
                sortDir={sortDir}
                sortKey={sortKey}
                toggleSort={toggleSort}
              />
              <SortHeader
                column="morale"
                label={t("common.morale")}
                sortDir={sortDir}
                sortKey={sortKey}
                toggleSort={toggleSort}
              />
              <th className="px-4 py-2.5 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                {t("squad.traits")}
              </th>
              <SortHeader
                column="ovr"
                label={t("common.ovr")}
                sortDir={sortDir}
                sortKey={sortKey}
                toggleSort={toggleSort}
              />
              <th className="px-4 py-2.5 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                {t("common.actions")}
              </th>
            </tr>
          </thead>
          <tbody className="divide-y divide-gray-100 dark:divide-navy-600">
            {players.map((player) =>
              renderTableRow({
                highlightedPlayerId,
                onSelectPlayer,
                player,
                section,
                xiActivePosition,
              }),
            )}
          </tbody>
        </table>
        {players.length === 0 ? (
          <div className="p-6 text-center text-sm text-gray-500 dark:text-gray-400">
            {emptyMessage}
          </div>
        ) : null}
      </div>
    </Card>
  );
}
