import { useMemo, useState } from "react";
import type {
  GameStateData,
  PlayerData,
  PlayerSelectionOptions,
} from "../../store/gameStore";
import { Badge, Button, Card, ProgressBar, Select, CountryFlag } from "../ui";
import {
  AlertTriangle,
  ChevronDown,
  ChevronUp,
  Repeat,
  RotateCcw,
  TimerOff,
  Trash2,
  Users,
} from "lucide-react";
import {
  calcAge,
  calcOvr,
  formatExactMoney,
  formatWeeklyAmount,
  formatVal,
  getContractRiskBadgeVariant,
  getContractRiskLevel,
  getContractYearsRemaining,
  positionBadgeVariant,
} from "../../lib/helpers";
import { canDelegateToYouthAcademy, isSeniorSquadPlayer } from "../../lib/playerSquad";
import { TraitList } from "../TraitBadge";
import { useTranslation } from "react-i18next";
import ContextMenu from "../ContextMenu";
import {
  clearContractExitIntent,
  setContractExitIntent,
} from "../../services/contractService";
import { setPlayerSquadRole } from "../../services/squadService";
import {
  toggleLoanList,
  toggleTransferList,
} from "../../services/transfersService";
import {
  buildActivePositionMap,
  buildPitchRows,
  buildPitchSlotRows,
  buildStartingXIIds,
  CORE_POSITIONS,
  getPreferredPositions,
  isPlayerOutOfPosition,
  normalisePosition,
  translatePositionAbbreviation,
} from "./SquadTab.helpers";
import {
  buildDelegateToYouthAcademyMenuItem,
  buildDividerMenuItem,
  buildToggleLoanListMenuItem,
  buildToggleTransferListMenuItem,
  buildViewProfileMenuItem,
} from "../playerActions/playerContextMenuItems";

interface SquadRosterViewProps {
  gameState: GameStateData;
  managerId: string;
  onGameUpdate?: (g: GameStateData) => void;
  onSelectPlayer: (id: string, options?: PlayerSelectionOptions) => void;
}

type FilterScope = "all" | "xi" | "bench" | "outOfPosition" | "injured";
type SortKey = "pos" | "name" | "age" | "condition" | "morale" | "ovr";

export default function SquadRosterView({
  gameState,
  managerId,
  onGameUpdate,
  onSelectPlayer,
}: SquadRosterViewProps) {
  const { t } = useTranslation();
  const weeklySuffix = t("finances.perWeekSuffix");
  const myTeam = gameState.teams.find((team) => team.manager_id === managerId);
  const [playerSearch, setPlayerSearch] = useState("");
  const [positionFilter, setPositionFilter] = useState("All");
  const [statusFilter, setStatusFilter] = useState<FilterScope>("all");
  const [sortKey, setSortKey] = useState<SortKey>("pos");
  const [sortDir, setSortDir] = useState<"asc" | "desc">("asc");
  const [contractActionPlayerId, setContractActionPlayerId] = useState<
    string | null
  >(null);
  const [contractActionError, setContractActionError] = useState<string | null>(
    null,
  );

  if (!myTeam) {
    return (
      <p className="text-gray-500 dark:text-gray-400">
        {t("common.unemployed")}
      </p>
    );
  }

  const posOrder: Record<string, number> = {
    Goalkeeper: 1,
    Defender: 2,
    Midfielder: 3,
    Forward: 4,
  };

  const roster = gameState.players
    .filter(
      (player) => player.team_id === myTeam.id && isSeniorSquadPlayer(player),
    )
    .sort(
      (a, b) =>
        (posOrder[normalisePosition(a.position)] || 99) -
        (posOrder[normalisePosition(b.position)] || 99) ||
        (b.ovr ?? calcOvr(b, b.natural_position || b.position)) -
        (a.ovr ?? calcOvr(a, a.natural_position || a.position)),
    );

  const playersById = useMemo(
    () => new Map(roster.map((player) => [player.id, player])),
    [roster],
  );

  const available = roster.filter((player) => !player.injury);
  const formation = myTeam.formation || "4-4-2";
  const startingXiIds = buildStartingXIIds(
    available,
    myTeam.starting_xi_ids || [],
    formation,
  );
  const pitchSlotRows = buildPitchSlotRows(
    buildPitchRows(formation),
    startingXiIds,
    playersById,
  );
  const xiActivePosition = buildActivePositionMap(pitchSlotRows);
  const xiIds = new Set(startingXiIds);

  const toggleSort = (key: SortKey) => {
    if (sortKey === key) {
      setSortDir((current) => (current === "asc" ? "desc" : "asc"));
      return;
    }
    setSortKey(key);
    setSortDir(key === "ovr" ? "desc" : "asc");
  };

  const isOutOfPosition = (player: PlayerData): boolean => {
    const currentPos = xiActivePosition.get(player.id) || player.position;
    return xiIds.has(player.id) && isPlayerOutOfPosition(player, currentPos);
  };

  const matchesFilters = (player: PlayerData): boolean => {
    const inXI = xiIds.has(player.id);
    const currentPos = inXI
      ? normalisePosition(xiActivePosition.get(player.id) || player.position)
      : normalisePosition(player.position);
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
      const getPos = (player: PlayerData) => {
        if (xiIds.has(player.id)) {
          return normalisePosition(
            xiActivePosition.get(player.id) || player.position,
          );
        }
        return normalisePosition(player.position);
      };

      switch (sortKey) {
        case "pos": {
          const aPosition = xiIds.has(a.id)
            ? xiActivePosition.get(a.id) || a.position
            : a.natural_position || a.position;
          const bPosition = xiIds.has(b.id)
            ? xiActivePosition.get(b.id) || b.position
            : b.natural_position || b.position;

          const aOvr = xiIds.has(a.id)
            ? calcOvr(a, aPosition)
            : (a.ovr ?? calcOvr(a, aPosition));
          const bOvr = xiIds.has(b.id)
            ? calcOvr(b, bPosition)
            : (b.ovr ?? calcOvr(b, bPosition));

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
          const aOvr = xiIds.has(a.id)
            ? calcOvr(a, xiActivePosition.get(a.id) || a.position)
            : (a.ovr ?? calcOvr(a, a.natural_position || a.position));
          const bOvr = xiIds.has(b.id)
            ? calcOvr(b, xiActivePosition.get(b.id) || b.position)
            : (b.ovr ?? calcOvr(b, b.natural_position || b.position));

          return aOvr - bOvr;
        }
        default:
          return 0;
      }
    });

    return sortDir === "desc" ? sorted.reverse() : sorted;
  }, [roster, sortKey, sortDir, playerSearch, positionFilter, statusFilter]);

  const hasActiveFilters =
    playerSearch.trim().length > 0 ||
    positionFilter !== "All" ||
    statusFilter !== "all";
  const outOfPositionCount = roster.filter((player) =>
    isOutOfPosition(player),
  ).length;
  const injuredCount = roster.filter((player) => player.injury).length;

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
      onGameUpdate?.(result.game);
    } catch (error) {
      setContractActionError(String(error));
    } finally {
      setContractActionPlayerId(null);
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

  const SortHeader = ({ col, label }: { col: SortKey; label: string }) => (
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
    <div className="max-w-6xl mx-auto flex flex-col gap-4">
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
            {t("squad.title", { team: myTeam.name })}
          </h3>
          <p className="text-xs text-gray-400 mt-0.5">
            {filteredRoster.length} / {roster.length}{" "}
            {t("squad.playersLabel")}
          </p>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full text-left border-collapse">
            <thead>
              <tr className="bg-gray-50 dark:bg-navy-800 border-b border-gray-200 dark:border-navy-600 text-xs">
                <SortHeader col="pos" label={t("squad.pos")} />
                <SortHeader col="name" label={t("common.name")} />
                <SortHeader col="age" label={t("common.age")} />
                <SortHeader col="condition" label={t("common.condition")} />
                <SortHeader col="morale" label={t("common.morale")} />
                <th className="py-2.5 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                  {t("squad.traits")}
                </th>
                <th className="py-2.5 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                  {t("common.value")}
                </th>
                <th className="py-2.5 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                  {t("finances.wagePerWeek")}
                </th>
                <th className="py-2.5 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                  {t("playerProfile.yearsRemaining")}
                </th>
                <th className="py-2.5 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                  {t("finances.contractRisk")}
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
                const currentPos = inXI
                  ? xiActivePosition.get(player.id) || player.position
                  : player.natural_position || player.position;
                // For XI players show position-specific OVR (includes out-of-position penalty).
                // For bench/non-XI players use the backend natural-position OVR.
                const ovr = inXI
                  ? calcOvr(player, currentPos)
                  : (player.ovr ?? calcOvr(player, currentPos));
                const age = calcAge(player.date_of_birth);
                const wrongPos = inXI && isOutOfPosition(player);
                const contractRiskLevel = getContractRiskLevel(
                  player.contract_end,
                  gameState.clock.current_date,
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

                const contextItems = [
                  buildViewProfileMenuItem(t, () => onSelectPlayer(player.id)),
                  {
                    label: t("common.renewContract"),
                    icon: <Repeat className="w-4 h-4" />,
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
                        onGameUpdate?.(updated);
                      } catch {
                        return;
                      }
                    },
                  ),
                  buildToggleLoanListMenuItem(t, player.loan_listed, async () => {
                    try {
                      const updated = await toggleLoanList(player.id);
                      onGameUpdate?.(updated);
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
                          onGameUpdate?.(updated);
                        } catch {
                          return;
                        }
                      }),
                    ]
                    : []),
                ];

                return (
                  <ContextMenu items={contextItems} key={player.id}>
                    <tr
                      onClick={() => onSelectPlayer(player.id)}
                      className="hover:bg-gray-50 dark:hover:bg-navy-700/50 transition-colors group cursor-pointer"
                    >
                      <td className="py-2.5 px-4">
                        <div className="flex items-center gap-1.5">
                          {inXI ? (
                            <span
                              className="w-1.5 h-1.5 rounded-full bg-primary-500"
                              title={t("preMatch.startingXI")}
                            />
                          ) : null}
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
                        <div className="font-semibold text-sm text-gray-900 dark:text-gray-100 group-hover:text-primary-600 dark:group-hover:text-primary-400 transition-colors">
                          {player.full_name}
                        </div>
                        {renderPreferredPositionMeta(player)}
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
                      <td className="py-2.5 px-4">
                        {player.traits && player.traits.length > 0 ? (
                          <TraitList traits={player.traits} size="xs" max={2} />
                        ) : (
                          <span className="text-xs text-gray-500">—</span>
                        )}
                      </td>
                      <td className="py-2.5 px-4 text-xs text-gray-600 dark:text-gray-400 font-medium">
                        {formatVal(player.market_value)}
                      </td>
                      <td className="py-2.5 px-4 text-xs text-gray-600 dark:text-gray-400 font-medium whitespace-nowrap">
                        {formatWeeklyAmount(
                          formatExactMoney(player.wage),
                          weeklySuffix,
                        )}
                      </td>
                      <td className="py-2.5 px-4 text-xs text-gray-600 dark:text-gray-400">
                        <div className="space-y-1">
                          <div className="font-medium text-gray-700 dark:text-gray-300">
                            {getContractYearsRemaining(
                              player.contract_end,
                              gameState.clock.current_date,
                            )}
                          </div>
                          <div>
                            {player.contract_end
                              ? t("finances.contractExpiresOn", {
                                date: player.contract_end,
                              })
                              : "—"}
                          </div>
                        </div>
                      </td>
                      <td className="py-2.5 px-4">
                        <Badge
                          variant={getContractRiskBadgeVariant(
                            contractRiskLevel,
                          )}
                          size="sm"
                        >
                          {contractRiskLabel}
                        </Badge>
                      </td>
                      <td className="py-2.5 px-4">
                        {player.contract_end &&
                          contractRiskLevel !== "stable" ? (
                          <Button
                            size="sm"
                            variant="outline"
                            onClick={(event) => {
                              event.stopPropagation();
                              onSelectPlayer(player.id, {
                                openRenewal: true,
                              });
                            }}
                          >
                            {t("common.renewContract")}
                          </Button>
                        ) : (
                          <span className="text-xs text-gray-500 dark:text-gray-400">
                            —
                          </span>
                        )}
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
