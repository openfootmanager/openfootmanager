import {
  AlertTriangle,
  ArrowDown,
  ArrowRight,
  ArrowUp,
  ChevronDown,
  ChevronUp,
  Star,
} from "lucide-react";
import type { JSX } from "react";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";

import { getPlayerOvr, positionBadgeVariant } from "../../lib/helpers";
import { getRolesForPosition } from "../../lib/playerRoles";
import type {
  PlayerData,
  TeamMatchRolesData,
} from "../../store/gameStore";
import type { PlayerRole } from "../../store/types";
import ContextMenu from "../ContextMenu";
import { Badge, Card, InjuryBadge, Select } from "../ui";
import {
  canonicalPosition,
  getPlayStyleFit,
  getSquadTacticalFit,
  getPreferredPositions,
  isPlayerOutOfPosition,
  normalisePosition,
  translatePositionAbbreviation,
  type SquadSection,
} from "../squad/SquadTab.helpers";
import {
  getOverallRatingClassName,
  type SortKey,
  type TacticsFormationSlotOption,
  type TacticsTableMode,
} from "./TacticsTab.helpers";
import { buildTacticsPlayerContextMenuItems } from "./TacticsContextMenu.helpers";

interface TacticsPlayerTableProps {
  activePlayStyle: string;
  comparePlayerId: string | null;
  emptyMessage: string;
  formation: string;
  highlightedPlayerId: string | null;
  matchRoles?: TeamMatchRolesData;
  onAssignBestFit?: (playerId: string) => void;
  onSetPlayerRole?: (playerId: string, role: PlayerRole | null) => void;
  playerRoles?: Record<string, PlayerRole>;
  onAssignMatchRole?: (
    role: keyof TeamMatchRolesData,
    playerId: string,
  ) => void;
  onAssignSlot?: (playerId: string, targetSlotIndex: number) => void;
  onClearTacticsSelection?: () => void;
  onDemoteStarter?: (playerId: string) => void;
  onPromoteBench?: (playerId: string) => void;
  onSelectPlayer: (playerId: string) => void;
  onTacticalSelect?: (playerId: string, section: SquadSection) => void;
  players: PlayerData[];
  section: SquadSection;
  slotOptions: TacticsFormationSlotOption[];
  sortDir: "asc" | "desc";
  sortKey: SortKey;
  tableMode: TacticsTableMode;
  title: string;
  toggleSort: (key: SortKey) => void;
  totalCount: number;
  xiActivePosition: Map<string, string>;
  xiSlotIndexByPlayerId: Map<string, number>;
}

interface SortHeaderProps {
  column: SortKey;
  label: string;
  sortDir: "asc" | "desc";
  sortKey: SortKey;
  toggleSort: (key: SortKey) => void;
}

interface MoraleState {
  icon: typeof ArrowUp;
  label: string;
  toneClassName: string;
}

interface ConditionState {
  barClassName: string;
  label: string;
  toneClassName: string;
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
      className={`cursor-pointer select-none px-4 py-2.5 font-heading font-bold uppercase tracking-wider transition-colors hover:text-primary-400 ${
        isActive
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

function getMoraleState(
  morale: number,
  translate: (key: string, fallback?: string) => string,
): MoraleState {
  if (morale >= 90) {
    return {
      icon: ArrowUp,
      label: translate("tactics.moraleLevels.superb"),
      toneClassName: "text-success-500 dark:text-success-400",
    };
  }

  if (morale >= 75) {
    return {
      icon: ArrowUp,
      label: translate("tactics.moraleLevels.veryGood"),
      toneClassName: "text-emerald-500 dark:text-emerald-400",
    };
  }

  if (morale >= 60) {
    return {
      icon: ArrowUp,
      label: translate("tactics.moraleLevels.good"),
      toneClassName: "text-primary-500 dark:text-primary-400",
    };
  }

  if (morale >= 45) {
    return {
      icon: ArrowRight,
      label: translate("tactics.moraleLevels.okay"),
      toneClassName: "text-gray-500 dark:text-gray-400",
    };
  }

  if (morale >= 30) {
    return {
      icon: ArrowDown,
      label: translate("tactics.moraleLevels.poor"),
      toneClassName: "text-amber-500 dark:text-amber-400",
    };
  }

  return {
    icon: ArrowDown,
    label: translate("tactics.moraleLevels.veryPoor"),
    toneClassName: "text-red-500 dark:text-red-400",
  };
}

function getConditionState(
  condition: number,
  translate: (key: string, fallback?: string) => string,
): ConditionState {
  if (condition >= 90) {
    return {
      barClassName: "bg-success-500",
      label: translate("tactics.conditionLevels.ready"),
      toneClassName: "text-success-600 dark:text-success-400",
    };
  }

  if (condition >= 75) {
    return {
      barClassName: "bg-primary-500",
      label: translate("tactics.conditionLevels.sharp"),
      toneClassName: "text-primary-600 dark:text-primary-400",
    };
  }

  if (condition >= 60) {
    return {
      barClassName: "bg-accent-400",
      label: translate("tactics.conditionLevels.managing"),
      toneClassName: "text-accent-600 dark:text-accent-400",
    };
  }

  return {
    barClassName: "bg-red-500",
    label: translate("tactics.conditionLevels.risk"),
    toneClassName: "text-red-600 dark:text-red-400",
  };
}


function getStyleFitBadge(
  fit: ReturnType<typeof getPlayStyleFit>,
  translate: (key: string, fallback?: string) => string,
): { label: string; variant: "success" | "accent" | "danger" } {
  switch (fit) {
    case "strong":
      return {
        label: translate("tactics.styleFitLevels.strong"),
        variant: "success",
      };
    case "good":
      return {
        label: translate("tactics.styleFitLevels.good"),
        variant: "accent",
      };
    default:
      return {
        label: translate("tactics.styleFitLevels.risky"),
        variant: "danger",
      };
  }
}

function buildResponsibilityChips(
  playerId: string,
  matchRoles: TeamMatchRolesData | undefined,
  translate: (key: string, fallback?: string) => string,
): Array<{ label: string; variant: "primary" | "accent" | "success" | "neutral" }> {
  if (!matchRoles) {
    return [];
  }

  const responsibilities: Array<{
    label: string;
    variant: "primary" | "accent" | "success" | "neutral";
  }> = [];

  if (matchRoles.captain === playerId) {
    responsibilities.push({
      label: translate("match.captain"),
      variant: "primary",
    });
  }

  if (matchRoles.vice_captain === playerId) {
    responsibilities.push({
      label: translate("tactics.viceCaptain"),
      variant: "neutral",
    });
  }

  if (matchRoles.penalty_taker === playerId) {
    responsibilities.push({
      label: translate("match.penaltyTaker"),
      variant: "accent",
    });
  }

  if (matchRoles.free_kick_taker === playerId) {
    responsibilities.push({
      label: translate("match.freeKickTaker"),
      variant: "accent",
    });
  }

  if (matchRoles.corner_taker === playerId) {
    responsibilities.push({
      label: translate("match.cornerTaker"),
      variant: "success",
    });
  }

  return responsibilities;
}


function TacticsTableRow({
  activePlayStyle,
  comparePlayerId,
  highlightedPlayerId,
  matchRoles,
  onAssignBestFit,
  onAssignMatchRole,
  onAssignSlot,
  onClearTacticsSelection,
  onDemoteStarter,
  onPromoteBench,
  onSelectPlayer,
  onSetPlayerRole,
  onTacticalSelect,
  player,
  playerRole,
  section,
  slotOptions,
  tableMode,
  xiActivePosition,
  xiSlotIndexByPlayerId,
}: {
  activePlayStyle: string;
  comparePlayerId: string | null;
  highlightedPlayerId: string | null;
  matchRoles?: TeamMatchRolesData;
  onAssignBestFit?: (playerId: string) => void;
  onAssignMatchRole?: (
    role: keyof TeamMatchRolesData,
    playerId: string,
  ) => void;
  onAssignSlot?: (playerId: string, targetSlotIndex: number) => void;
  onClearTacticsSelection?: () => void;
  onDemoteStarter?: (playerId: string) => void;
  onPromoteBench?: (playerId: string) => void;
  onSelectPlayer: (playerId: string) => void;
  onSetPlayerRole?: (playerId: string, role: PlayerRole | null) => void;
  onTacticalSelect?: (playerId: string, section: SquadSection) => void;
  player: PlayerData;
  playerRole?: PlayerRole;
  section: SquadSection;
  slotOptions: TacticsFormationSlotOption[];
  tableMode: TacticsTableMode;
  xiActivePosition: Map<string, string>;
  xiSlotIndexByPlayerId: Map<string, number>;
}): JSX.Element {
  const { t } = useTranslation();
  const translateLabel = (key: string, fallback?: string) =>
    t(key, fallback ?? key);
  const currentSlotIndex =
    section === "xi" ? (xiSlotIndexByPlayerId.get(player.id) ?? null) : null;
  const currentSlotOption =
    currentSlotIndex != null
      ? slotOptions.find((slotOption) => slotOption.index === currentSlotIndex) ?? null
      : null;
  const activePosition =
    section === "xi"
      ? currentSlotOption?.position ?? xiActivePosition.get(player.id) ?? player.position
      : player.natural_position || player.position;
  const preferredPositions = getPreferredPositions(player);
  const visiblePreferredPositions = preferredPositions.slice(0, 3);
  const hiddenPreferredPositionCount = Math.max(0, preferredPositions.length - 3);
  const responsibilities = buildResponsibilityChips(
    player.id,
    matchRoles,
    translateLabel,
  );
  const tacticalFit = getSquadTacticalFit(player, activePosition);
  // Functions available to a player follow their current field position. If a
  // previously-assigned function is no longer valid for where they now play
  // (e.g. a striker's Poacher after being moved to defence), show Standard
  // rather than the stale attacker function.
  const roleOptions = getRolesForPosition(canonicalPosition(activePosition));
  const effectiveRole =
    playerRole && roleOptions.includes(playerRole) ? playerRole : "Standard";
  const styleFitBadge = getStyleFitBadge(
    getPlayStyleFit(player, activePlayStyle, activePosition),
    translateLabel,
  );
  const moraleState = getMoraleState(player.morale, translateLabel);
  const conditionState = getConditionState(player.condition, translateLabel);
  const overallRating = getPlayerOvr(player);
  const isHighlighted = highlightedPlayerId === player.id;
  const isComparing = comparePlayerId === player.id;
  const isWrongPosition =
    section === "xi" && isPlayerOutOfPosition(player, activePosition);
  const primaryName = player.match_name || player.full_name;
  const secondaryName =
    player.full_name && player.full_name !== primaryName ? player.full_name : null;

  const contextItems = useMemo(() => {
    return buildTacticsPlayerContextMenuItems({
      isSelected: isHighlighted,
      matchRoles,
      onAssignBestFit,
      onAssignMatchRole,
      onClearSelection: onClearTacticsSelection,
      onDemoteStarter,
      onOpenProfile: onSelectPlayer,
      onPromoteBench,
      onTacticalSelect,
      player,
      section,
      selectedPlayerId: highlightedPlayerId,
      t,
    });
  }, [
    isHighlighted,
    onAssignBestFit,
    onAssignMatchRole,
    onClearTacticsSelection,
    onDemoteStarter,
    matchRoles,
    onPromoteBench,
    onSelectPlayer,
    onTacticalSelect,
    player,
    highlightedPlayerId,
    section,
    t,
  ]);

  return (
    <ContextMenu items={contextItems}>
      <tr
        data-testid={`${section}-player-${player.id}`}
        onClick={() => onSelectPlayer(player.id)}
        className={`group cursor-pointer transition-colors ${
          isHighlighted
            ? "bg-accent-500/10 dark:bg-accent-500/10"
            : isComparing
              ? "bg-primary-500/10 dark:bg-primary-500/10"
              : "hover:bg-gray-50 dark:hover:bg-navy-700/50"
        }`}
      >
        <td className="px-4 py-3 align-top">
          {section === "xi" && currentSlotOption ? (
            <div className="space-y-1.5" onClick={(event) => event.stopPropagation()}>
              <Select
                selectSize="sm"
                variant="muted"
                value={String(currentSlotOption.index)}
                onChange={(event) => {
                  onAssignSlot?.(player.id, Number(event.target.value));
                }}
                aria-label={t("tactics.positionSelector")}
                fullWidth
                className="font-heading font-bold"
              >
                {slotOptions.map((slotOption) => (
                  <option key={slotOption.index} value={slotOption.index}>
                    {slotOption.shortLabel} - {slotOption.label}
                  </option>
                ))}
              </Select>
              {isWrongPosition ? (
                <div className="flex items-center gap-1 text-[11px] font-heading font-bold uppercase tracking-[0.16em] text-amber-500">
                  <AlertTriangle className="h-3.5 w-3.5" />
                  {t("squad.outOfPosition")}
                </div>
              ) : null}
            </div>
          ) : (
            <Badge
              variant={positionBadgeVariant(normalisePosition(activePosition))}
              size="sm"
            >
              {translatePositionAbbreviation(t, activePosition)}
            </Badge>
          )}
        </td>

        <td className="px-4 py-3 align-top">
          <div className="text-sm font-semibold text-gray-900 transition-colors group-hover:text-primary-600 dark:text-gray-100 dark:group-hover:text-primary-400">
            {primaryName}
          </div>
          {secondaryName ? (
            <div className="mt-1 truncate text-[11px] uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400">
              {secondaryName}
            </div>
          ) : null}
        </td>

        {tableMode === "lineup" ? (
          <>
            <td className="px-4 py-3 align-top">
              <div className="flex max-w-[16rem] flex-wrap gap-1.5">
                {visiblePreferredPositions.map((position, index) => (
                  <Badge
                    key={`${player.id}-${position}`}
                    variant={index === 0 ? positionBadgeVariant(position) : "neutral"}
                    size="sm"
                  >
                    {translatePositionAbbreviation(t, position)}
                  </Badge>
                ))}
                {hiddenPreferredPositionCount > 0 ? (
                  <Badge variant="neutral" size="sm">
                    +{hiddenPreferredPositionCount}
                  </Badge>
                ) : null}
              </div>
            </td>

            <td className="w-40 px-4 py-3 align-top">
              <div className="min-w-[8rem]">
                <div className="mb-1.5 flex items-center justify-between gap-2 text-[11px] font-heading font-bold uppercase tracking-[0.16em]">
                  <span className={conditionState.toneClassName}>
                    {conditionState.label}
                  </span>
                  <span className="tabular-nums text-gray-500 dark:text-gray-400">
                    {player.condition}%
                  </span>
                </div>
                <div className="h-1.5 overflow-hidden rounded-full bg-gray-200 dark:bg-navy-600">
                  <div
                    className={`h-full rounded-full ${conditionState.barClassName}`}
                    style={{ width: `${Math.max(12, player.condition)}%` }}
                  />
                </div>
              </div>
            </td>

            <td className="px-4 py-3 align-top">
              <div className={`flex items-center gap-2 text-sm font-semibold ${moraleState.toneClassName}`}>
                <moraleState.icon className="h-4 w-4" />
                <span>{moraleState.label}</span>
              </div>
              <div className="mt-1 text-[11px] uppercase tracking-[0.16em] text-gray-500 dark:text-gray-400">
                {player.morale}
              </div>
            </td>
          </>
        ) : (
          <>
            <td className="px-4 py-3 align-top">
              <div className="space-y-0.5 text-xs">
                <div>
                  <span
                    className={
                      tacticalFit === "out"
                        ? "font-medium text-red-500 dark:text-red-400"
                        : tacticalFit === "adapted"
                          ? "font-medium text-amber-500 dark:text-amber-400"
                          : "font-medium text-emerald-600 dark:text-emerald-400"
                    }
                  >
                    {t("squad.slotLabel")}: {translatePositionAbbreviation(t, activePosition)}
                  </span>
                </div>
                <div className="text-gray-500 dark:text-gray-400">
                  {t("squad.hasLabel")}: {preferredPositions.map((p) => translatePositionAbbreviation(t, p)).join(", ")}
                </div>
              </div>
            </td>

            <td className="px-4 py-3 align-top">
              <Badge variant={styleFitBadge.variant} size="sm">
                {styleFitBadge.label}
              </Badge>
              <div className="mt-1 text-[11px] uppercase tracking-[0.16em] text-gray-500 dark:text-gray-400">
                {t(`common.playStyles.${activePlayStyle}`, activePlayStyle)}
              </div>
            </td>

            <td className="px-4 py-3 align-top">
              <Select
                value={effectiveRole}
                onChange={(e) => {
                  const val = e.target.value;
                  onSetPlayerRole?.(player.id, val === "Standard" ? null : (val as PlayerRole));
                }}
              >
                {roleOptions.map((role) => (
                  <option key={role} value={role}>
                    {t(`tactics.playerRoles.${role}`, role)}
                  </option>
                ))}
              </Select>
            </td>

            <td className="px-4 py-3 align-top">
              {responsibilities.length > 0 ? (
                <div className="flex max-w-[16rem] flex-wrap gap-1.5">
                  {responsibilities.map((responsibility) => (
                    <Badge
                      key={`${player.id}-${responsibility.label}`}
                      variant={responsibility.variant}
                      size="sm"
                    >
                      {responsibility.label}
                    </Badge>
                  ))}
                </div>
              ) : (
                <span className="text-xs text-gray-500 dark:text-gray-400">
                  {t("tactics.noSpecialRoles")}
                </span>
              )}
            </td>
          </>
        )}

        <td className="px-4 py-3 align-top">
          <span
            className={`text-base font-heading font-bold tabular-nums ${getOverallRatingClassName(
              overallRating,
            )}`}
          >
            {overallRating}
          </span>
        </td>
        <td className="px-4 py-3 align-top">
          {player.injury ? (
            <InjuryBadge injury={player.injury} />
          ) : (
            <span className="text-xs text-gray-500 dark:text-gray-400">—</span>
          )}
        </td>
      </tr>
    </ContextMenu>
  );
}

export default function TacticsPlayerTable({
  activePlayStyle,
  comparePlayerId,
  emptyMessage,
  formation,
  highlightedPlayerId,
  matchRoles,
  onAssignBestFit,
  onAssignMatchRole,
  onAssignSlot,
  onClearTacticsSelection,
  onDemoteStarter,
  onPromoteBench,
  onSelectPlayer,
  onSetPlayerRole,
  onTacticalSelect,
  playerRoles,
  players,
  section,
  slotOptions,
  sortDir,
  sortKey,
  tableMode,
  title,
  toggleSort,
  totalCount,
  xiActivePosition,
  xiSlotIndexByPlayerId,
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

  return (
    <Card>
      <div className={headingClassName}>
        <h3 className={titleClassName}>
          {section === "xi" ? (
            <Star className="h-4 w-4 fill-current text-accent-400" />
          ) : null}
          {title}
        </h3>
        <div className="mt-1 flex flex-wrap items-center justify-between gap-2">
          <p className="text-xs text-gray-400">
            {players.length} / {totalCount} {t("squad.playersLabel")}
          </p>
          <div className="flex flex-wrap items-center justify-end gap-x-3 gap-y-1 text-[10px] font-heading font-bold uppercase tracking-[0.18em] text-gray-400">
            <span>
              {tableMode === "lineup"
                ? t("tactics.tableModes.lineup")
                : t("tactics.tableModes.roles")}
              {" · "}
              {formation}
            </span>
            <span>
              {t("tactics.tableInteractionHint")}
            </span>
          </div>
        </div>
      </div>

      <div className="overflow-x-auto">
        <table className="w-full min-w-[52rem] border-collapse text-left">
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
              {tableMode === "lineup" ? (
                <>
                  <th className="px-4 py-2.5 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                    {t("tactics.playablePositions")}
                  </th>
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
                </>
              ) : (
                <>
                  <th className="px-4 py-2.5 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                    {t("tactics.slotFit")}
                  </th>
                  <th className="px-4 py-2.5 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                    {t("tactics.styleFit")}
                  </th>
                  <th className="px-4 py-2.5 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                    {t("tactics.playerRoleLabel")}
                  </th>
                  <th className="px-4 py-2.5 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                    {t("tactics.responsibilities")}
                  </th>
                </>
              )}
              <SortHeader
                column="ovr"
                label={t("common.ovr")}
                sortDir={sortDir}
                sortKey={sortKey}
                toggleSort={toggleSort}
              />
              <th className="px-4 py-2.5 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                {t("common.status")}
              </th>
            </tr>
          </thead>

          <tbody className="divide-y divide-gray-100 dark:divide-navy-600">
            {players.map((player) => (
              <TacticsTableRow
                key={player.id}
                activePlayStyle={activePlayStyle}
                comparePlayerId={comparePlayerId}
                highlightedPlayerId={highlightedPlayerId}
                matchRoles={matchRoles}
                onAssignBestFit={onAssignBestFit}
                onAssignMatchRole={onAssignMatchRole}
                onAssignSlot={onAssignSlot}
                onClearTacticsSelection={onClearTacticsSelection}
                onDemoteStarter={onDemoteStarter}
                onPromoteBench={onPromoteBench}
                onSelectPlayer={onSelectPlayer}
                onSetPlayerRole={onSetPlayerRole}
                onTacticalSelect={onTacticalSelect}
                player={player}
                playerRole={playerRoles?.[player.id]}
                section={section}
                slotOptions={slotOptions}
                tableMode={tableMode}
                xiActivePosition={xiActivePosition}
                xiSlotIndexByPlayerId={xiSlotIndexByPlayerId}
              />
            ))}
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
