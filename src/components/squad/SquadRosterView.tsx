import { useMemo, useRef, useState } from "react";
import type {
  GameStateData,
  PlayerData,
  PlayerSelectionOptions,
  TeamData,
} from "../../store/gameStore";
import { Badge, Card, ProgressBar, Select, CountryFlag, PlayerAvatar, InjuryBadge } from "../ui";
import {
  AlertTriangle,
  ChevronDown,
  ChevronUp,
  MoreVertical,
  Repeat,
  RotateCcw,
  TimerOff,
  Trash2,
  Users,
} from "lucide-react";
import {
  calcAge,
  getPlayerOvr,
  getContractRiskBadgeVariant,
  getContractRiskLevel,
  getContractYearsRemaining,
  positionBadgeVariant,
} from "../../lib/helpers";
import { canDelegateToYouthAcademy, isSeniorSquadPlayer } from "../../lib/playerSquad";
import { getInjurySeverity, resolveInjuryName } from "../../lib/injury";
import { useTranslation } from "react-i18next";
import ContextMenu, { type ContextMenuHandle } from "../ContextMenu";
import {
  clearContractExitIntent,
  setContractExitIntent,
} from "../../services/contractService";
import { setPlayerSquadRole, setStartingXi } from "../../services/squadService";
import {
  toggleLoanList,
  toggleTransferList,
} from "../../services/transfersService";
import {
  buildActivePositionMap,
  buildRoleCoverageSummary,
  buildDemoteFromStartingXi,
  getBestRoleForFormation,
  getCurrentPosition,
  getPlayStyleFit,
  buildPitchRows,
  buildPitchSlotRows,
  buildPromoteToStartingXi,
  buildStartingXIIds,
  CORE_POSITIONS,
  getPreferredPositions,
  getSquadTacticalFit,
  isPlayerOutOfPosition,
  normalisePosition,
  translatePositionAbbreviation,
} from "./SquadTab.helpers";
import { findTacticsPresetBySetup } from "../tactics/TacticsTab.helpers";
import {
  buildDelegateToYouthAcademyMenuItem,
  buildDividerMenuItem,
  buildToggleLoanListMenuItem,
  buildToggleTransferListMenuItem,
  buildViewProfileMenuItem,
} from "../playerActions/playerContextMenuItems";
import {
  DEFAULT_SQUAD_LIST_SORT_STATE,
  type SquadListSortKey,
  type SquadListSortState,
} from "./SquadRosterView.state";

interface SquadRosterViewProps {
  players: PlayerData[];
  team: TeamData;
  clockDate: string;
  onSelectPlayer: (id: string, options?: PlayerSelectionOptions) => void;
  onMutationComplete?: (g: GameStateData) => void;
  sortState?: SquadListSortState;
  onSortStateChange?: (sortState: SquadListSortState) => void;
}

type FilterScope =
  | "all"
  | "xi"
  | "bench"
  | "naturalFit"
  | "needsCover"
  | "outOfPosition"
  | "injured";

export default function SquadRosterView({
  players,
  team,
  clockDate,
  onSelectPlayer,
  onMutationComplete,
  sortState,
  onSortStateChange,
}: SquadRosterViewProps) {
  const { t } = useTranslation();
  const [playerSearch, setPlayerSearch] = useState("");
  const [positionFilter, setPositionFilter] = useState("All");
  const [statusFilter, setStatusFilter] = useState<FilterScope>("all");
  const [localSortState, setLocalSortState] = useState<SquadListSortState>(
    DEFAULT_SQUAD_LIST_SORT_STATE,
  );
  const [contractActionPlayerId, setContractActionPlayerId] = useState<
    string | null
  >(null);
  const [contractActionError, setContractActionError] = useState<string | null>(
    null,
  );
  const menuRefs = useRef<Map<string, ContextMenuHandle>>(new Map());

  const posOrder: Record<string, number> = {
    Goalkeeper: 1,
    Defender: 2,
    Midfielder: 3,
    Forward: 4,
  };

  const roster = players
    .filter((player) => isSeniorSquadPlayer(player))
    .sort(
      (a, b) =>
        (posOrder[normalisePosition(a.position)] || 99) -
        (posOrder[normalisePosition(b.position)] || 99) ||
        getPlayerOvr(b) - getPlayerOvr(a),
    );

  const playersById = useMemo(
    () => new Map(roster.map((player) => [player.id, player])),
    [roster],
  );

  const available = roster.filter((player) => !player.injury);
  const formation = team.formation || "4-4-2";
  const activePlayStyle = team.play_style || "Balanced";
  const currentPreset = findTacticsPresetBySetup(formation, activePlayStyle);
  const startingXiIds = buildStartingXIIds(
    available,
    team.starting_xi_ids || [],
    formation,
  );
  const pitchSlotRows = buildPitchSlotRows(
    buildPitchRows(formation),
    startingXiIds,
    playersById,
  );
  const xiActivePosition = buildActivePositionMap(pitchSlotRows);
  const xiIds = new Set(startingXiIds);
  const roleCoverage = useMemo(
    () => buildRoleCoverageSummary(available, startingXiIds, formation),
    [available, startingXiIds, formation],
  );
  const rolesNeedingCover = useMemo(
    () =>
      new Set(
        roleCoverage
          .filter((coverage) => coverage.status !== "covered")
          .map((coverage) => coverage.role),
      ),
    [roleCoverage],
  );
  const activeSortState = sortState ?? localSortState;
  const sortKey = activeSortState.sortKey;
  const sortDir = activeSortState.sortDir;

  const updateSortState = (nextSortState: SquadListSortState) => {
    if (onSortStateChange) {
      onSortStateChange(nextSortState);
      return;
    }

    setLocalSortState(nextSortState);
  };

  const toggleSort = (key: SquadListSortKey) => {
    if (sortKey === key) {
      updateSortState({
        sortKey,
        sortDir: sortDir === "asc" ? "desc" : "asc",
      });
      return;
    }

    updateSortState({
      sortKey: key,
      sortDir: key === "ovr" ? "desc" : "asc",
    });
  };

  const isOutOfPosition = (player: PlayerData): boolean => {
    return (
      xiIds.has(player.id) &&
      isPlayerOutOfPosition(player, getCurrentPosition(player, xiActivePosition))
    );
  };

  const getTacticalFit = (player: PlayerData) => {
    return getSquadTacticalFit(
      player,
      getCurrentPosition(player, xiActivePosition),
    );
  };

  const matchesFilters = (player: PlayerData): boolean => {
    const inXI = xiIds.has(player.id);
    const currentPos = normalisePosition(
      getCurrentPosition(player, xiActivePosition),
    );
    const preferredPositions = getPreferredPositions(player);
    const search = playerSearch.trim().toLowerCase();

    if (search) {
      const searchable = [
        player.full_name,
        player.match_name,
        currentPos,
        ...preferredPositions,
        ...preferredPositions.map((position) =>
          translatePositionAbbreviation(t, position),
        ),
      ]
        .join(" ")
        .toLowerCase();
      if (!searchable.includes(search)) return false;
    }

    if (
      positionFilter !== "All" &&
      currentPos !== positionFilter &&
      !preferredPositions.includes(positionFilter)
    ) {
      return false;
    }

    switch (statusFilter) {
      case "xi":
        return inXI;
      case "bench":
        return !inXI;
      case "naturalFit":
        return getTacticalFit(player) === "natural";
      case "needsCover":
        return rolesNeedingCover.has(getBestRoleForFormation(player, formation));
      case "outOfPosition":
        return isOutOfPosition(player);
      case "injured":
        return Boolean(player.injury);
      default:
        return true;
    }
  };

  const filteredRoster = useMemo(() => {
    const list = roster.filter((player) => matchesFilters(player));
    const sorted = [...list].sort((a, b) => {
      const getPos = (player: PlayerData) =>
        normalisePosition(getCurrentPosition(player, xiActivePosition));

      switch (sortKey) {
        case "pos": {
          const aOvr = getPlayerOvr(a);
          const bOvr = getPlayerOvr(b);

          return (
            (posOrder[getPos(a)] || 99) - (posOrder[getPos(b)] || 99) ||
            bOvr - aOvr
          );
        }
        case "name":
          return a.full_name.localeCompare(b.full_name);
        case "age":
          return calcAge(a.date_of_birth) - calcAge(b.date_of_birth);
        case "condition":
          return a.condition - b.condition;
        case "morale":
          return a.morale - b.morale;
        case "ovr": {
          const aOvr = getPlayerOvr(a);
          const bOvr = getPlayerOvr(b);

          return aOvr - bOvr;
        }
        default:
          return 0;
      }
    });

    return sortDir === "desc" ? sorted.reverse() : sorted;
  }, [
    formation,
    playerSearch,
    positionFilter,
    roleCoverage,
    roster,
    sortDir,
    sortKey,
    startingXiIds,
    statusFilter,
    t,
    xiActivePosition,
  ]);

  const hasActiveFilters =
    playerSearch.trim().length > 0 ||
    positionFilter !== "All" ||
    statusFilter !== "all";
  const starterCount = startingXiIds.length;
  const benchCount = Math.max(roster.length - starterCount, 0);
  const naturalFitCount = roster.filter(
    (player) => getTacticalFit(player) === "natural",
  ).length;
  const outOfPositionCount = roster.filter((player) =>
    isOutOfPosition(player),
  ).length;
  const injuredCount = roster.filter((player) => player.injury).length;
  const thinCoverageCount = roleCoverage.filter(
    (coverage) => coverage.status !== "covered",
  ).length;

  const persistStartingXi = async (playerIds: string[]): Promise<void> => {
    const updated = await setStartingXi(playerIds);
    onMutationComplete?.(updated);
  };

  const updateContractExitIntent = async (
    playerId: string,
    shouldLetExpire: boolean,
  ): Promise<void> => {
    setContractActionPlayerId(playerId);
    setContractActionError(null);

    try {
      const result = shouldLetExpire
        ? await setContractExitIntent(playerId, "manager_squad_action")
        : await clearContractExitIntent(playerId);
      onMutationComplete?.(result.game);
    } catch (error) {
      setContractActionError(String(error));
    } finally {
      setContractActionPlayerId(null);
    }
  };

  const updateSquadPlanning = async (
    playerId: string,
    action: "promote" | "demote",
  ): Promise<void> => {
    const nextXiIds =
      action === "promote"
        ? buildPromoteToStartingXi(startingXiIds, playersById, formation, playerId)
        : buildDemoteFromStartingXi(
            startingXiIds,
            available,
            formation,
            playerId,
          );

    if (!nextXiIds || nextXiIds.join(",") === startingXiIds.join(",")) {
      return;
    }

    try {
      await persistStartingXi(nextXiIds);
    } catch (error) {
      setContractActionError(String(error));
    }
  };

  const renderPreferredPositionMeta = (player: PlayerData) => (
    <div className="text-xs text-gray-400 dark:text-gray-500 flex items-center gap-1.5 flex-wrap">
      <CountryFlag code={player.nationality} className="text-sm leading-none" />
      {getPreferredPositions(player).map((position, index) => (
        <Badge
          key={`${player.id}-${position}`}
          variant={index === 0 ? positionBadgeVariant(position) : "neutral"}
          size="sm"
        >
          {translatePositionAbbreviation(t, position)}
        </Badge>
      ))}
    </div>
  );

  const renderRoleAndStyleMeta = (player: PlayerData) => {
    const bestRole = getBestRoleForFormation(player, formation);
    const currentPos = getCurrentPosition(player, xiActivePosition);
    const styleFit = getPlayStyleFit(player, activePlayStyle, currentPos);

    return (
      <div className="mt-1 flex flex-wrap items-center gap-1.5 text-[11px] text-gray-500 dark:text-gray-400">
        <span className="font-medium">
          {t("squad.bestRole")}: {translatePositionAbbreviation(t, bestRole)}
        </span>
        <Badge
          variant={
            styleFit === "strong"
              ? "success"
              : styleFit === "good"
                ? "accent"
                : "danger"
          }
          size="sm"
        >
          {t(`squad.styleFitValues.${styleFit}`)}
        </Badge>
      </div>
    );
  };

  const SortHeader = ({ col, label }: { col: SquadListSortKey; label: string }) => (
    <th
      className={`py-2.5 px-4 font-heading font-bold uppercase tracking-wider cursor-pointer select-none hover:text-primary-400 transition-colors ${sortKey === col ? "text-primary-500 dark:text-primary-400" : "text-gray-500 dark:text-gray-400"}`}
      onClick={() => toggleSort(col)}
    >
      <div className="flex items-center gap-1">
        {label}
        {sortKey === col ? (
          sortDir === "asc" ? (
            <ChevronUp className="w-3 h-3" />
          ) : (
            <ChevronDown className="w-3 h-3" />
          )
        ) : null}
      </div>
    </th>
  );

  return (
    <div className="flex flex-col gap-4">
      <Card>
        <div className="p-4 grid grid-cols-1 lg:grid-cols-[minmax(0,1.3fr)_220px_220px_auto] gap-3 items-end">
          <div>
            <label className="text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-2 block">
              {t("common.search")}
            </label>
            <input
              type="text"
              value={playerSearch}
              onChange={(event) => setPlayerSearch(event.target.value)}
              placeholder={t("squad.filterPlayers")}
              className="w-full rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-800 px-3 py-2 text-sm text-gray-700 dark:text-gray-200 placeholder:text-gray-400 focus:outline-none focus:ring-2 focus:ring-primary-500/30"
            />
          </div>
          <div>
            <label className="text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-2 block">
              {t("squad.pos")}
            </label>
            <Select
              value={positionFilter}
              onChange={(event) => setPositionFilter(event.target.value)}
              fullWidth
            >
              <option value="All">{t("common.all")}</option>
              {CORE_POSITIONS.map((position) => (
                <option key={position} value={position}>
                  {translatePositionAbbreviation(t, position)}
                </option>
              ))}
            </Select>
          </div>
          <div>
            <label className="text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-2 block">
              {t("common.status")}
            </label>
            <Select
              value={statusFilter}
              onChange={(event) =>
                setStatusFilter(event.target.value as FilterScope)
              }
              fullWidth
            >
              <option value="all">
                {t("common.allPlayers")}
              </option>
              <option value="xi">
                {t("preMatch.startingXI")}
              </option>
              <option value="bench">
                {t("preMatch.substitutes")}
              </option>
              <option value="naturalFit">
                {t("squad.naturalFit")}
              </option>
              <option value="needsCover">
                {t("squad.needsCover")}
              </option>
              <option value="outOfPosition">
                {t("squad.outOfPosition")}
              </option>
              <option value="injured">{t("common.injured")}</option>
            </Select>
          </div>
          <button
            type="button"
            onClick={() => {
              setPlayerSearch("");
              setPositionFilter("All");
              setStatusFilter("all");
            }}
            disabled={!hasActiveFilters}
            className={`px-3 py-2 rounded-lg text-xs font-heading font-bold uppercase tracking-wider transition-all ${hasActiveFilters
              ? "bg-gray-100 dark:bg-navy-700 text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-navy-600"
              : "bg-gray-100 dark:bg-navy-700 text-gray-400 cursor-not-allowed"
              }`}
          >
            {t("common.clear")}
          </button>
        </div>
        <div className="px-4 pb-4 flex flex-wrap gap-2">
          <Badge variant="primary" size="sm">
            {starterCount} {t("squad.starter")}
          </Badge>
          <Badge variant="neutral" size="sm">
            {benchCount} {t("squad.benchOption")}
          </Badge>
          <Badge variant="success" size="sm">
            {naturalFitCount} {t("squad.naturalFit")}
          </Badge>
          <Badge
            variant={thinCoverageCount > 0 ? "accent" : "success"}
            size="sm"
          >
            {thinCoverageCount} {t("squad.needsCover")}
          </Badge>
          <Badge
            variant={outOfPositionCount > 0 ? "danger" : "success"}
            size="sm"
          >
            {outOfPositionCount} {t("squad.outOfPosition")}
          </Badge>
          <Badge variant={injuredCount > 0 ? "danger" : "neutral"} size="sm">
            {injuredCount} {t("common.injured")}
          </Badge>
          <Badge variant="primary" size="sm">
            {filteredRoster.length} {t("squad.playersLabel")}
          </Badge>
        </div>
      </Card>

      <Card>
        <div className="p-4 border-b border-gray-100 dark:border-navy-600 bg-linear-to-r from-navy-700 to-navy-800 rounded-t-xl">
          <h3 className="text-sm font-heading font-bold text-white uppercase tracking-wide flex items-center gap-2">
            <Users className="w-4 h-4 text-accent-400" />
            {t("squad.title", { team: team.name })}
          </h3>
          <p className="text-xs text-gray-400 mt-0.5">
            {filteredRoster.length} / {roster.length}{" "}
            {t("squad.playersLabel")}
          </p>
          <p className="text-xs text-gray-400 mt-1">
            {t("squad.currentPlan")}: {formation} /{" "}
            {currentPreset
              ? t(`tactics.presetNames.${currentPreset.id}`, currentPreset.id)
              : t(`common.playStyles.${activePlayStyle}`, activePlayStyle)}
          </p>
          <div className="mt-3 flex flex-col gap-2">
            <div className="flex flex-wrap items-center gap-2">
              <span className="text-xs font-heading font-bold uppercase tracking-wider text-gray-300">
                {t("squad.coverageTitle")}
              </span>
              <Badge
                variant={thinCoverageCount > 0 ? "danger" : "success"}
                size="sm"
              >
                {thinCoverageCount > 0
                  ? t("squad.coverageNeedsAttention", {
                    count: thinCoverageCount,
                  })
                  : t("squad.coverageStable")}
              </Badge>
            </div>
            <div className="flex flex-wrap gap-2">
              {roleCoverage.map((coverage) => (
                <Badge
                  key={coverage.role}
                  variant={
                    coverage.status === "covered"
                      ? "success"
                      : coverage.status === "thin"
                        ? "accent"
                        : "danger"
                  }
                  size="sm"
                  className="gap-1"
                >
                  <span>{translatePositionAbbreviation(t, coverage.role)}</span>
                  <span>
                    {t("squad.coverageBadge", {
                      starters: coverage.naturalStarters,
                      required: coverage.requiredSlots,
                      bench: coverage.benchOptions,
                    })}
                  </span>
                </Badge>
              ))}
            </div>
          </div>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full text-left border-collapse">
            <thead>
              <tr className="bg-gray-50 dark:bg-navy-800 border-b border-gray-200 dark:border-navy-600 text-xs">
                <th className="py-2.5 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                  #
                </th>
                <SortHeader col="pos" label={t("squad.pos")} />
                <SortHeader col="name" label={t("common.name")} />
                <th className="py-2.5 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                  {t("squad.tacticalFit")}
                </th>
                <SortHeader col="age" label={t("common.age")} />
                <SortHeader col="condition" label={t("common.condition")} />
                <SortHeader col="morale" label={t("common.morale")} />
                <th className="py-2.5 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                  {t("common.contract")}
                </th>
                <th className="py-2.5 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                  {t("common.actions")}
                </th>
                <SortHeader col="ovr" label={t("common.ovr")} />
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100 dark:divide-navy-600">
              {filteredRoster.map((player) => {
                const inXI = xiIds.has(player.id);
                const currentPos = getCurrentPosition(player, xiActivePosition);
                const ovr = getPlayerOvr(player);
                const age = calcAge(player.date_of_birth);
                const wrongPos = inXI && isOutOfPosition(player);
                const tacticalFit = getTacticalFit(player);
                const contractRiskLevel = getContractRiskLevel(
                  player.contract_end,
                  clockDate,
                );
                const contractRiskLabel =
                  contractRiskLevel === "critical"
                    ? t("finances.contractRiskCritical")
                    : contractRiskLevel === "warning"
                      ? t("finances.contractRiskWarning")
                      : t("finances.contractRiskStable");
                const hasLetExpireIntent =
                  player.morale_core?.renewal_state?.exit_intent?.kind ===
                  "let_expire";
                const isContractActionSubmitting =
                  contractActionPlayerId === player.id;

                const injurySeverity = player.injury
                  ? getInjurySeverity(player.injury.days_remaining)
                  : null;
                const injuryDotClass = injurySeverity === "major" ? "bg-red-500"
                  : injurySeverity === "serious" ? "bg-orange-400"
                  : injurySeverity === "moderate" ? "bg-amber-400"
                  : injurySeverity === "minor" ? "bg-yellow-400"
                  : null;
                const rowBorderClass = player.injury
                  ? (injurySeverity === "major" || injurySeverity === "serious"
                    ? "border-l-2 border-l-red-500"
                    : "border-l-2 border-l-amber-400")
                  : contractRiskLevel === "critical"
                    ? "border-l-2 border-l-orange-500"
                    : contractRiskLevel === "warning"
                      ? "border-l-2 border-l-yellow-400"
                      : "";
                const hasUrgentItems = Boolean(player.injury) || contractRiskLevel !== "stable";

                const contextItems = [
                  ...(player.injury ? [
                    {
                      type: "label" as const,
                      label: `${resolveInjuryName(player.injury.name, t)} — ${t("playerProfile.injuryDaysShort", { count: player.injury.days_remaining })}`,
                      icon: <AlertTriangle className="w-3.5 h-3.5" />,
                    },
                    buildDividerMenuItem(),
                  ] : []),
                  buildViewProfileMenuItem(t, () => onSelectPlayer(player.id)),
                  inXI
                    ? {
                        label: t("squad.sendToBench"),
                        icon: <RotateCcw className="w-4 h-4" />,
                        disabled:
                          available.filter((candidate) => !xiIds.has(candidate.id))
                            .length === 0,
                        onClick: () => {
                          void updateSquadPlanning(player.id, "demote");
                        },
                      }
                    : {
                        label: t("squad.makeStarter"),
                        icon: <Users className="w-4 h-4" />,
                        disabled: Boolean(player.injury),
                        onClick: () => {
                          void updateSquadPlanning(player.id, "promote");
                        },
                      },
                  buildDividerMenuItem(),
                  {
                    label: t("common.renewContract"),
                    icon: <Repeat className="w-4 h-4" />,
                    urgent: contractRiskLevel !== "stable",
                    disabled: !player.contract_end,
                    onClick: () =>
                      onSelectPlayer(player.id, {
                        openRenewal: true,
                      }),
                  },
                  hasLetExpireIntent
                    ? {
                      label: t("playerProfile.reopenContractTalks"),
                      icon: <RotateCcw className="w-4 h-4" />,
                      disabled:
                        !player.contract_end || isContractActionSubmitting,
                      onClick: () => {
                        void updateContractExitIntent(player.id, false);
                      },
                    }
                    : {
                      label: t("playerProfile.letContractExpire"),
                      icon: <TimerOff className="w-4 h-4" />,
                      disabled:
                        !player.contract_end || isContractActionSubmitting,
                      onClick: () => {
                        void updateContractExitIntent(player.id, true);
                      },
                    },
                  {
                    label: t("playerProfile.terminateContract"),
                    icon: <Trash2 className="w-4 h-4" />,
                    danger: true,
                    disabled: !player.contract_end,
                    onClick: () =>
                      onSelectPlayer(player.id, {
                        openTermination: true,
                      }),
                  },
                  buildDividerMenuItem(),
                  buildToggleTransferListMenuItem(
                    t,
                    player.transfer_listed,
                    async () => {
                      try {
                        const updated = await toggleTransferList(player.id);
                        onMutationComplete?.(updated);
                      } catch {
                        return;
                      }
                    },
                  ),
                  buildToggleLoanListMenuItem(t, player.loan_listed, async () => {
                    try {
                      const updated = await toggleLoanList(player.id);
                      onMutationComplete?.(updated);
                    } catch {
                      return;
                    }
                  }),
                  ...(canDelegateToYouthAcademy(player)
                    ? [
                      buildDelegateToYouthAcademyMenuItem(t, async () => {
                        try {
                          const updated = await setPlayerSquadRole(
                            player.id,
                            "Youth",
                          );
                          onMutationComplete?.(updated);
                        } catch {
                          return;
                        }
                      }),
                    ]
                    : []),
                ];

                return (
                  <ContextMenu
                    items={contextItems}
                    key={player.id}
                    ref={(handle) => {
                      if (handle) menuRefs.current.set(player.id, handle);
                      else menuRefs.current.delete(player.id);
                    }}
                  >
                    <tr
                      onClick={() => onSelectPlayer(player.id)}
                      className={`hover:bg-gray-50 dark:hover:bg-navy-700/50 transition-colors group cursor-pointer ${rowBorderClass}`}
                    >
                      <td className="py-2.5 px-4 tabular-nums text-sm font-medium text-gray-600 dark:text-gray-400">
                        {player.jersey_number ?? "—"}
                      </td>
                      <td className="py-2.5 px-4">
                        <div className="flex items-center gap-1.5">
                          <Badge
                            variant={positionBadgeVariant(currentPos)}
                            size="sm"
                          >
                            {translatePositionAbbreviation(t, currentPos)}
                          </Badge>
                          {wrongPos ? (
                            <span
                              className="text-amber-500"
                              title={t("squad.outOfPositionTooltip")}
                            >
                              <AlertTriangle className="w-3.5 h-3.5" />
                            </span>
                          ) : null}
                        </div>
                      </td>
                      <td className="py-2.5 px-4">
                        <div className="flex items-center gap-3">
                          <PlayerAvatar player={player} />
                          <div className="min-w-0">
                            <div className="flex items-center gap-1.5 font-semibold text-sm text-gray-900 dark:text-gray-100 group-hover:text-primary-600 dark:group-hover:text-primary-400 transition-colors">
                              {injuryDotClass && (
                                <span className={`inline-block h-1.5 w-1.5 shrink-0 rounded-full ${injuryDotClass}`} />
                              )}
                              {player.full_name}
                            </div>
                            {renderPreferredPositionMeta(player)}
                            {renderRoleAndStyleMeta(player)}
                            {player.transfer_listed || player.loan_listed ? (
                              <div className="mt-1 flex flex-wrap gap-1">
                                {player.transfer_listed ? (
                                  <Badge variant="accent" size="sm">
                                    {t("transfers.transfer")}
                                  </Badge>
                                ) : null}
                                {player.loan_listed ? (
                                  <Badge variant="primary" size="sm">
                                    {t("transfers.loan")}
                                  </Badge>
                                ) : null}
                              </div>
                            ) : null}
                            {player.injury ? (
                              <InjuryBadge injury={player.injury} />
                            ) : null}
                          </div>
                        </div>
                      </td>
                      <td className="py-2.5 px-4">
                        {inXI ? (
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
                                {t("squad.slotLabel")}: {translatePositionAbbreviation(t, currentPos)}
                              </span>
                            </div>
                            <div className="text-gray-500 dark:text-gray-400">
                              {t("squad.hasLabel")}: {getPreferredPositions(player).map((p) => translatePositionAbbreviation(t, p)).join(", ")}
                            </div>
                          </div>
                        ) : (
                          <div className="text-xs text-gray-500 dark:text-gray-400">
                            <span className="font-medium">{t("squad.bestRole")}:</span>{" "}
                            {translatePositionAbbreviation(t, getBestRoleForFormation(player, formation))}
                          </div>
                        )}
                      </td>
                      <td className="py-2.5 px-4 text-sm text-gray-600 dark:text-gray-400 tabular-nums">
                        {age}
                      </td>
                      <td className="py-2.5 px-4 w-28">
                        <ProgressBar
                          value={player.condition}
                          variant="auto"
                          size="sm"
                          showLabel
                        />
                      </td>
                      <td className="py-2.5 px-4 text-sm text-gray-500 dark:text-gray-400 tabular-nums">
                        {player.morale}
                      </td>
                      <td className="py-2.5 px-4 text-xs text-gray-600 dark:text-gray-400">
                        <div className="space-y-1">
                          <div className="flex items-center gap-1.5">
                            <span className="font-medium text-gray-700 dark:text-gray-300">
                              {getContractYearsRemaining(player.contract_end, clockDate)}
                            </span>
                            <Badge variant={getContractRiskBadgeVariant(contractRiskLevel)} size="sm">
                              {contractRiskLabel}
                            </Badge>
                          </div>
                          <div className="text-gray-500 dark:text-gray-400">
                            {player.contract_end
                              ? t("finances.contractExpiresOn", { date: player.contract_end })
                              : "—"}
                          </div>
                        </div>
                      </td>
                      <td className="py-2.5 px-4" onClick={(e) => e.stopPropagation()}>
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation();
                            const rect = e.currentTarget.getBoundingClientRect();
                            menuRefs.current.get(player.id)?.open(rect.left, rect.bottom + 4);
                          }}
                          className="relative rounded-md p-1.5 text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 transition-colors"
                        >
                          <MoreVertical className="h-4 w-4" />
                          {hasUrgentItems && (
                            <span className="absolute -right-0.5 -top-0.5 h-2 w-2 rounded-full bg-amber-400" />
                          )}
                        </button>
                      </td>
                      <td className="py-2.5 px-4 text-right">
                        <span
                          className={`font-heading font-bold text-sm ${ovr >= 80
                            ? "text-primary-500"
                            : ovr >= 55
                              ? "text-accent-600 dark:text-accent-400"
                              : "text-gray-500 dark:text-gray-400"
                            }`}
                        >
                          {ovr}
                        </span>
                      </td>
                    </tr>
                  </ContextMenu>
                );
              })}
            </tbody>
          </table>
          {filteredRoster.length === 0 ? (
            <div className="p-8 text-center text-gray-500 dark:text-gray-400 font-heading uppercase tracking-wider text-sm">
              {t("squad.noPlayers")}
            </div>
          ) : null}
        </div>
      </Card>

      {contractActionError ? (
        <div className="rounded-lg border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700 dark:border-red-900/50 dark:bg-red-950/30 dark:text-red-300">
          {contractActionError}
        </div>
      ) : null}
    </div>
  );
}
