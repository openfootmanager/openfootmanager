import { useRatingTiers } from "../../hooks/useRatingTiers";
import { useContractRiskLevels } from "../../hooks/useContractWarningStages";
import { useId, useMemo, useRef, useState } from "react";
import type {
  GameStateData,
  PlayerData,
  PlayerSelectionOptions,
  TeamData,
} from "../../store/gameStore";
import { Badge, Card, DataTable, PositionBadges, Select } from "../ui";
import { MoreVertical, Users } from "lucide-react";
import { calcAge, getPlayerOvr } from "../../lib/helpers";
import { isFirstTeamSquadPlayer, isMatchDayEligible } from "../../lib/playerSquad";
import { useTranslation } from "react-i18next";
import ContextMenu, { type ContextMenuHandle } from "../ContextMenu";
import { clearContractExitIntent, setContractExitIntent } from "../../services/contractService";
import { setStartingXi } from "../../services/squadService";
import {
  buildActivePositionMap,
  buildRoleCoverageSummary,
  buildDemoteFromStartingXi,
  getBestRoleForFormation,
  getCurrentPosition,
  buildPitchRows,
  buildPitchSlotRows,
  buildPromoteToStartingXi,
  buildStartingXIIds,
  getPreferredPositions,
  getSquadTacticalFit,
  isPlayerOutOfPosition,
  translatePositionAbbreviation,
  translatePositionLabel,
} from "./SquadTab.helpers";
import {
  CORE_POSITIONS,
  naturalPositionOf,
  normalisePosition,
  positionGroupRank,
  positionSortRank,
} from "../../lib/positions";
import { findTacticsPresetBySetup } from "../tactics/TacticsTab.helpers";
import { SquadNameAside, squadRosterColumns } from "./SquadRosterView.columns";
import { squadRowMenuItems } from "./SquadRosterView.menu";
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
  const tierOf = useRatingTiers();
  const { t, i18n } = useTranslation();
  const [playerSearch, setPlayerSearch] = useState("");
  // Ties each visible filter label to its control, so the control is announced by that label.
  const searchInputId = useId();
  const positionFilterId = useId();
  const statusFilterId = useId();
  const [positionFilter, setPositionFilter] = useState("All");
  const [statusFilter, setStatusFilter] = useState<FilterScope>("all");
  const [localSortState, setLocalSortState] = useState<SquadListSortState>(
    DEFAULT_SQUAD_LIST_SORT_STATE,
  );
  const [contractActionPlayerId, setContractActionPlayerId] = useState<string | null>(null);
  const [contractActionError, setContractActionError] = useState<string | null>(null);
  const menuRefs = useRef<Map<string, ContextMenuHandle>>(new Map());
  const [openMenuPlayerId, setOpenMenuPlayerId] = useState<string | null>(null);

  const roster = players
    .filter((player) => isFirstTeamSquadPlayer(player))
    .sort(
      (a, b) =>
        positionGroupRank(a.position) - positionGroupRank(b.position) ||
        getPlayerOvr(b) - getPlayerOvr(a),
    );

  const playersById = useMemo(() => new Map(roster.map((player) => [player.id, player])), [roster]);
  const riskLevelOf = useContractRiskLevels(roster, clockDate);

  const available = roster.filter(isMatchDayEligible);
  const formation = team.formation || "4-4-2";
  const activePlayStyle = team.play_style || "Balanced";
  const currentPreset = findTacticsPresetBySetup(formation, activePlayStyle);
  const startingXiIds = useMemo(
    () => buildStartingXIIds(available, team.starting_xi_ids || [], formation),
    [available, team.starting_xi_ids, formation],
  );
  const pitchSlotRows = buildPitchSlotRows(buildPitchRows(formation), startingXiIds, playersById);
  const xiActivePosition = buildActivePositionMap(pitchSlotRows);
  const xiIds = useMemo(() => new Set(startingXiIds), [startingXiIds]);
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

    // Sensible starting direction per column: OVR, condition, morale default
    // to desc (higher first); everything else defaults to asc.
    const descByDefault: SquadListSortKey[] = ["ovr", "condition", "morale"];
    updateSortState({
      sortKey: key,
      sortDir: descByDefault.includes(key) ? "desc" : "asc",
    });
  };

  const isOutOfPosition = (player: PlayerData): boolean => {
    return (
      xiIds.has(player.id) &&
      isPlayerOutOfPosition(player, getCurrentPosition(player, xiActivePosition))
    );
  };

  const getTacticalFit = (player: PlayerData) => {
    return getSquadTacticalFit(player, getCurrentPosition(player, xiActivePosition));
  };

  const matchesFilters = (player: PlayerData): boolean => {
    const inXI = xiIds.has(player.id);
    const currentPos = normalisePosition(getCurrentPosition(player, xiActivePosition));
    const preferredPositions = getPreferredPositions(player);
    const search = playerSearch.trim().toLowerCase();

    if (search) {
      const searchable = [
        player.full_name,
        player.match_name,
        currentPos,
        ...preferredPositions,
        ...preferredPositions.map((position) => translatePositionAbbreviation(t, position)),
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

    // Contract-remaining rank for sorting: sooner-expiring first when asc.
    // Missing contract_end sorts last.
    const contractRank = (player: PlayerData): string => player.contract_end ?? "9999-99-99";

    const sorted = [...list].sort((a, b) => {
      switch (sortKey) {
        case "jersey":
          return (a.jersey_number ?? 999) - (b.jersey_number ?? 999);
        case "name":
          return a.full_name.localeCompare(b.full_name);
        case "pos": {
          // XI-first, then football-canonical natural_position order, then
          // OVR desc. This is the default landing view: starters cluster on
          // top, sorted by where they play; strongest at each slot rises.
          const aXi = xiIds.has(a.id) ? 0 : 1;
          const bXi = xiIds.has(b.id) ? 0 : 1;
          if (aXi !== bXi) return aXi - bXi;

          const aRank = positionSortRank(naturalPositionOf(a));
          const bRank = positionSortRank(naturalPositionOf(b));
          if (aRank !== bRank) return aRank - bRank;

          return getPlayerOvr(b) - getPlayerOvr(a);
        }
        case "age":
          return calcAge(a.date_of_birth) - calcAge(b.date_of_birth);
        case "condition":
          return a.condition - b.condition;
        case "morale":
          return a.morale - b.morale;
        case "ovr":
          return getPlayerOvr(a) - getPlayerOvr(b);
        case "contract":
          return contractRank(a).localeCompare(contractRank(b));
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
    xiIds,
  ]);

  const hasActiveFilters =
    playerSearch.trim().length > 0 || positionFilter !== "All" || statusFilter !== "all";
  const starterCount = startingXiIds.length;
  const benchCount = Math.max(roster.length - starterCount, 0);
  const naturalFitCount = roster.filter((player) => getTacticalFit(player) === "natural").length;
  const outOfPositionCount = roster.filter((player) => isOutOfPosition(player)).length;
  const injuredCount = roster.filter((player) => player.injury).length;
  const thinCoverageCount = roleCoverage.filter((coverage) => coverage.status !== "covered").length;

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
        : buildDemoteFromStartingXi(startingXiIds, available, formation, playerId);

    if (!nextXiIds || nextXiIds.join(",") === startingXiIds.join(",")) {
      return;
    }

    try {
      await persistStartingXi(nextXiIds);
    } catch (error) {
      setContractActionError(String(error));
    }
  };

  const renderPreferredPositionMeta = (player: PlayerData) => {
    const [natural, ...alternates] = getPreferredPositions(player);
    const inXI = xiIds.has(player.id);
    return (
      <PositionBadges
        // Table C: one line of up to three, the deployed position kept, the rest as "+n".
        max={3}
        oneLine
        natural={natural}
        alternates={alternates}
        deployed={inXI ? getCurrentPosition(player, xiActivePosition) : undefined}
        familiarity={inXI ? getTacticalFit(player) : undefined}
        describe={(position) => ({
          label: translatePositionAbbreviation(t, position),
          name: translatePositionLabel(t, position),
        })}
      />
    );
  };

  const columns = squadRosterColumns({
    t,
    language: i18n.language,
    tierOf,
    riskLevelOf,
    renderPositions: renderPreferredPositionMeta,
  });
  const benchHasCover = available.some((candidate) => !xiIds.has(candidate.id));
  const menuContextFor = (player: PlayerData) => ({
    t,
    inStartingXi: xiIds.has(player.id),
    canBench: benchHasCover,
    contractRisk: riskLevelOf(player.id),
    contractActionPending: contractActionPlayerId === player.id,
    onSelectPlayer,
    onMutationComplete,
    updateSquadPlanning,
    updateContractExitIntent,
  });

  return (
    <div className="flex flex-col gap-4">
      <Card>
        <div className="p-4 grid grid-cols-1 lg:grid-cols-[minmax(0,1.3fr)_220px_220px_auto] gap-3 items-end">
          <div>
            <label
              htmlFor={searchInputId}
              className="text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-2 block"
            >
              {t("common.search")}
            </label>
            <input
              id={searchInputId}
              type="text"
              value={playerSearch}
              onChange={(event) => setPlayerSearch(event.target.value)}
              placeholder={t("squad.filterPlayers")}
              className="w-full rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-800 px-3 py-2 text-sm text-gray-700 dark:text-gray-200 placeholder:text-gray-400 dark:placeholder:text-gray-500 focus:outline-none focus:ring-2 focus:ring-primary-500/30"
            />
          </div>
          <div>
            <label
              htmlFor={positionFilterId}
              className="text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-2 block"
            >
              {t("squad.pos")}
            </label>
            <Select
              id={positionFilterId}
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
            <label
              htmlFor={statusFilterId}
              className="text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-2 block"
            >
              {t("common.status")}
            </label>
            <Select
              id={statusFilterId}
              value={statusFilter}
              onChange={(event) => setStatusFilter(event.target.value as FilterScope)}
              fullWidth
            >
              <option value="all">{t("common.allPlayers")}</option>
              <option value="xi">{t("preMatch.startingXI")}</option>
              <option value="bench">{t("preMatch.substitutes")}</option>
              <option value="naturalFit">{t("squad.naturalFit")}</option>
              <option value="needsCover">{t("squad.needsCover")}</option>
              <option value="outOfPosition">{t("squad.outOfPosition")}</option>
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
            className={`px-3 py-2 rounded-lg text-xs font-heading font-bold uppercase tracking-wider transition-all ${
              hasActiveFilters
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
          <Badge variant={thinCoverageCount > 0 ? "accent" : "success"} size="sm">
            {thinCoverageCount} {t("squad.needsCover")}
          </Badge>
          <Badge variant={outOfPositionCount > 0 ? "danger" : "success"} size="sm">
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
            {filteredRoster.length} / {roster.length} {t("squad.playersLabel")}
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
              <Badge variant={thinCoverageCount > 0 ? "danger" : "success"} size="sm">
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
        <DataTable
          label={t("squad.title", { team: team.name })}
          columns={columns}
          rows={filteredRoster}
          rowKey={(player) => player.id}
          rowName={(player) => player.match_name}
          sort={{ key: sortKey, direction: sortDir }}
          onSortChange={(key) => toggleSort(key as SquadListSortKey)}
          onOpen={(player) => onSelectPlayer(player.id)}
          rowAccent={(player) =>
            xiIds.has(player.id) ? "positive" : isMatchDayEligible(player) ? null : "danger"
          }
          onRowContextMenu={(player, event) => {
            // The Menu key and Shift+F10 send a right-click at 0,0: open it at the row instead.
            const fromKeyboard = event.clientX === 0 && event.clientY === 0;
            const rect = event.currentTarget.getBoundingClientRect();
            menuRefs.current
              .get(player.id)
              ?.open(
                fromKeyboard ? rect.left : event.clientX,
                fromKeyboard ? rect.bottom : event.clientY,
              );
          }}
          primaryAside={(player) => (
            <SquadNameAside player={player} starting={xiIds.has(player.id)} t={t} />
          )}
          rowAction={{
            header: t("common.actions"),
            cell: (player) => (
              <ContextMenu
                items={squadRowMenuItems(player, menuContextFor(player))}
                ref={(handle) => {
                  if (handle) menuRefs.current.set(player.id, handle);
                  else menuRefs.current.delete(player.id);
                }}
                onOpenChange={(open) => {
                  setOpenMenuPlayerId((prev) => {
                    if (open) return player.id;
                    return prev === player.id ? null : prev;
                  });
                }}
              >
                <button
                  type="button"
                  onClick={(e) => {
                    const rect = e.currentTarget.getBoundingClientRect();
                    menuRefs.current.get(player.id)?.open(rect.left, rect.bottom + 4);
                  }}
                  aria-label={t("common.playerActions", { name: player.match_name })}
                  aria-haspopup="menu"
                  aria-expanded={openMenuPlayerId === player.id}
                  className="relative rounded-md p-1.5 text-fg-subtle transition-colors hover:text-fg focus:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-focus"
                >
                  <MoreVertical className="h-4 w-4" />
                  {player.injury || riskLevelOf(player.id) !== "stable" ? (
                    // Decoration: the injury and the contract stage are said in words in the row.
                    <span
                      aria-hidden="true"
                      className="absolute -right-0.5 -top-0.5 h-2 w-2 rounded-full bg-warning-solid"
                    />
                  ) : null}
                </button>
              </ContextMenu>
            ),
          }}
          empty={t("squad.noPlayers")}
        />
      </Card>

      {contractActionError ? (
        <div className="rounded-lg border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700 dark:border-red-900/50 dark:bg-red-950/30 dark:text-red-300">
          {contractActionError}
        </div>
      ) : null}
    </div>
  );
}
