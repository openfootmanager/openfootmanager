import { useRatingTiers } from "../../hooks/useRatingTiers";
import { visibleRatingText } from "../../lib/playerKnowledge";
import { ratingTierClass } from "../../lib/ratingTier";
import { useCallback, useState } from "react";
import type { GameStateData, PlayerSelectionOptions } from "../../store/gameStore";
import { getErrorMessage, resolveTranslatedErrorMessage } from "../../utils/errorMessage";
import { Card, CardBody, Badge, Select, CountryFlag, PlayerAvatar } from "../ui";
import ContextMenu from "../ContextMenu";
import PlayerListFilters from "./PlayerListFilters";
import {
  Filter,
  ArrowUpDown,
  ChevronLeft,
  ChevronRight,
  ChevronsLeft,
  ChevronsRight,
} from "lucide-react";
import { formatVal } from "../../lib/helpers";
import { useTranslation } from "react-i18next";
import { buildAlreadyScoutingIds } from "../scouting/ScoutingTab.model";
import { calculateAvailableScouts } from "../scouting/ScoutingTab.helpers";
import { sendScout } from "../../services/scoutingService";
import { toggleLoanList, toggleTransferList } from "../../services/transfersService";
import type { PlayerSortKey, PlayersPageQuery } from "../../services/playersService";
import { usePlayersPage } from "../../hooks/usePlayersPage";
import {
  buildDividerMenuItem,
  buildOfferFreeAgentContractMenuItem,
  buildMakeTransferBidMenuItem,
  buildScoutPlayerMenuItem,
  buildToggleLoanListMenuItem,
  buildToggleTransferListMenuItem,
  buildViewProfileMenuItem,
  buildViewTeamMenuItem,
} from "../playerActions/playerContextMenuItems";
import FreeAgentContractModal from "../transfers/FreeAgentContractModal";
import TransferBidModal from "../transfers/TransferBidModal";
import { useFreeAgentContractFlow } from "../transfers/useFreeAgentContractFlow";
import { useTransferBidFlow } from "../transfers/useTransferBidFlow";
import { naturalPositionOf } from "../../lib/positions";
import PlayerPositionBadge from "../PlayerPositionBadge";

interface PlayersListTabProps {
  gameState: GameStateData;
  onGameUpdate?: (game: GameStateData) => void;
  onSelectPlayer: (id: string, options?: PlayerSelectionOptions) => void;
  onSelectTeam: (id: string) => void;
}

const PAGE_SIZE = 30;

const DEFAULT_QUERY: PlayersPageQuery = {
  search: null,
  position: null,
  positions: [],
  age_min: null,
  age_max: null,
  ovr_min: null,
  ovr_max: null,
  value_min: null,
  value_max: null,
  in_budget: false,
  scope: "all",
  team_id: null,
  status: "all",
  sort_key: "ovr",
  sort_asc: false,
  page: 1,
  page_size: PAGE_SIZE,
};

export default function PlayersListTab({
  gameState,
  onGameUpdate,
  onSelectPlayer,
  onSelectTeam,
}: PlayersListTabProps) {
  const { t } = useTranslation();
  const tierOf = useRatingTiers();
  const {
    query,
    patchQuery,
    slice,
    error: fetchError,
    refetch: refetchSlice,
  } = usePlayersPage(DEFAULT_QUERY, t);
  const [sendingPlayerId, setSendingPlayerId] = useState<string | null>(null);
  const [scoutError, setScoutError] = useState<string | null>(null);
  const managerTeamId = gameState.manager.team_id ?? "";

  const handleGameUpdate = useCallback(
    (game: GameStateData) => {
      onGameUpdate?.(game);
      refetchSlice();
    },
    [onGameUpdate, refetchSlice],
  );

  const findFullPlayer = useCallback(
    (id: string) => gameState.players.find((p) => p.id === id),
    [gameState.players],
  );
  const {
    bidTarget,
    bidAmount,
    setBidAmount,
    bidResult,
    bidLoading,
    bidFeedback,
    bidProjection,
    bidFee,
    activeBidOffer,
    myTeam,
    hasExistingOffer,
    talksResumeOn,
    bidSubmitDisabled,
    openBidNegotiation,
    closeBidNegotiation,
    handleMakeBid,
  } = useTransferBidFlow({
    gameState,
    onGameUpdate: handleGameUpdate,
  });
  const {
    freeAgentTarget,
    contractWage,
    setContractWage,
    contractLength,
    setContractLength,
    contractFeedback,
    contractProjection,
    contractSubmitting,
    contractSubmitDisabled,
    contractStatusMessage,
    contractStatusClassName,
    openFreeAgentContract,
    closeFreeAgentContract,
    submitFreeAgentContract,
  } = useFreeAgentContractFlow({
    gameState,
    onGameUpdate: handleGameUpdate,
  });
  const scouts = gameState.staff.filter(
    (staffMember) => staffMember.role === "Scout" && staffMember.team_id === managerTeamId,
  );
  const scoutingAssignments = gameState.scouting_assignments || [];
  const allScoutingAssignments = [
    ...scoutingAssignments,
    ...(gameState.youth_scouting_assignments || []),
  ];
  const availableScouts = calculateAvailableScouts(scouts, allScoutingAssignments);
  const alreadyScoutingIds = buildAlreadyScoutingIds(scoutingAssignments);

  const handleScoutPlayer = async (playerId: string): Promise<void> => {
    if (availableScouts.length === 0) {
      setScoutError(null);
      return;
    }

    const scout = availableScouts[0];
    setScoutError(null);
    setSendingPlayerId(playerId);

    try {
      const updated = await sendScout(scout.id, playerId);
      setScoutError(null);
      handleGameUpdate(updated);
    } catch (error) {
      console.error("Failed to send scout:", error);
      setScoutError(resolveTranslatedErrorMessage(getErrorMessage(error), t));
    } finally {
      setSendingPlayerId(null);
    }
  };

  const handleSort = (key: PlayerSortKey) => {
    if (query.sort_key === key) {
      patchQuery({ sort_asc: !query.sort_asc });
    } else {
      patchQuery({ sort_key: key, sort_asc: key === "name" });
    }
  };

  const items = slice?.items ?? [];
  const total = slice?.total ?? 0;
  const page = query.page;
  const pageSize = slice?.page_size ?? query.page_size;
  const totalPages = Math.max(1, Math.ceil(total / pageSize));

  return (
    <div>
      <PlayerListFilters
        query={query}
        onChange={patchQuery}
        statusOptions={["all", "transfer", "loan"]}
        trailing={
          <Select
            value={query.team_id ?? ""}
            onChange={(e) => patchQuery({ team_id: e.target.value || null })}
            selectSize="sm"
            className="min-w-44 font-heading font-bold uppercase tracking-wider"
          >
            <option value="">{t("players.allTeams")}</option>
            {gameState.teams.map((tm) => (
              <option key={tm.id} value={tm.id}>
                {tm.name}
              </option>
            ))}
          </Select>
        }
      />

      <p className="text-xs text-gray-400 dark:text-gray-500 mb-3 font-heading uppercase tracking-wider">
        <Filter className="w-3.5 h-3.5 inline mr-1 -mt-0.5" />
        {t("players.nPlayersFound", { count: total })}
      </p>

      {fetchError ? (
        <p
          role="alert"
          className="mb-3 text-xs font-heading font-bold uppercase tracking-wider text-red-500"
        >
          {fetchError}
        </p>
      ) : null}

      {scoutError ? (
        <p
          role="alert"
          className="mb-3 text-xs font-heading font-bold uppercase tracking-wider text-red-500"
        >
          {scoutError}
        </p>
      ) : null}

      {/* Players table */}
      <Card>
        <CardBody className="p-0">
          <div className="overflow-x-auto">
            <table className="w-full text-left border-collapse">
              <thead>
                <tr className="bg-gray-50 dark:bg-navy-800 border-b border-gray-200 dark:border-navy-600 text-xs">
                  <SortHeader
                    label={t("common.position")}
                    sortKey="position"
                    current={query.sort_key}
                    asc={query.sort_asc}
                    onClick={handleSort}
                  />
                  <SortHeader
                    label={t("common.name")}
                    sortKey="name"
                    current={query.sort_key}
                    asc={query.sort_asc}
                    onClick={handleSort}
                  />
                  <SortHeader
                    label={t("common.age")}
                    sortKey="age"
                    current={query.sort_key}
                    asc={query.sort_asc}
                    onClick={handleSort}
                  />
                  <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                    {t("common.nationality")}
                  </th>
                  <SortHeader
                    label={t("common.team")}
                    sortKey="team"
                    current={query.sort_key}
                    asc={query.sort_asc}
                    onClick={handleSort}
                  />
                  <SortHeader
                    label={t("common.value")}
                    sortKey="value"
                    current={query.sort_key}
                    asc={query.sort_asc}
                    onClick={handleSort}
                  />
                  <SortHeader
                    label={t("common.ovr")}
                    sortKey="ovr"
                    current={query.sort_key}
                    asc={query.sort_asc}
                    onClick={handleSort}
                  />
                  <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                    {t("common.status")}
                  </th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100 dark:divide-navy-600">
                {items.map((summary) => {
                  const scoutState = alreadyScoutingIds.has(summary.id)
                    ? "already-assigned"
                    : sendingPlayerId === summary.id
                      ? "busy"
                      : availableScouts.length === 0
                        ? "unavailable"
                        : "ready";
                  const contextItems = [
                    buildViewProfileMenuItem(t, () => onSelectPlayer(summary.id)),
                    ...(summary.team_id
                      ? [
                          buildViewTeamMenuItem(t, () => {
                            onSelectTeam(summary.team_id!);
                          }),
                        ]
                      : []),
                  ];

                  if (summary.team_id === managerTeamId) {
                    contextItems.push(buildDividerMenuItem());
                    contextItems.push(
                      buildToggleTransferListMenuItem(t, summary.transfer_listed, async () => {
                        try {
                          const updated = await toggleTransferList(summary.id);
                          handleGameUpdate(updated);
                        } catch {
                          return;
                        }
                      }),
                    );
                    contextItems.push(
                      buildToggleLoanListMenuItem(t, summary.loan_listed, async () => {
                        try {
                          const updated = await toggleLoanList(summary.id);
                          handleGameUpdate(updated);
                        } catch {
                          return;
                        }
                      }),
                    );
                  } else {
                    const playerActions = summary.team_id
                      ? [
                          buildMakeTransferBidMenuItem(t, () => {
                            const full = findFullPlayer(summary.id);
                            if (full) openBidNegotiation(full);
                          }),
                          buildScoutPlayerMenuItem(t, scoutState, () => {
                            void handleScoutPlayer(summary.id);
                          }),
                        ]
                      : summary.retired
                        ? []
                        : [
                            buildOfferFreeAgentContractMenuItem(t, () => {
                              const full = findFullPlayer(summary.id);
                              if (full) openFreeAgentContract(full);
                            }),
                            buildScoutPlayerMenuItem(t, scoutState, () => {
                              void handleScoutPlayer(summary.id);
                            }),
                          ];

                    if (playerActions.length > 0) {
                      contextItems.push(buildDividerMenuItem());
                      contextItems.push(...playerActions);
                    }
                  }

                  const ovr = summary.ovr;
                  const row = (
                    <tr
                      key={summary.id}
                      onClick={() => onSelectPlayer(summary.id)}
                      className="hover:bg-gray-50 dark:hover:bg-navy-700/50 transition-colors cursor-pointer group"
                    >
                      <td className="py-2.5 px-4">
                        <PlayerPositionBadge position={naturalPositionOf(summary)} />
                      </td>
                      <td className="py-2.5 px-4">
                        <div className="flex items-center gap-3">
                          <PlayerAvatar player={summary} />
                          <span className="font-semibold text-sm text-gray-800 dark:text-gray-200 group-hover:text-primary-600 dark:group-hover:text-primary-400 transition-colors">
                            {summary.match_name}
                          </span>
                        </div>
                      </td>
                      <td className="py-2.5 px-4 text-sm text-gray-600 dark:text-gray-400 tabular-nums">
                        {summary.age}
                      </td>
                      <td
                        className="py-2.5 px-4 text-sm text-gray-500 dark:text-gray-400"
                        title={summary.nationality}
                      >
                        <CountryFlag code={summary.nationality} className="text-lg leading-none" />
                      </td>
                      <td className="py-2.5 px-4">
                        {summary.team_id ? (
                          <button
                            type="button"
                            onClick={(e) => {
                              e.stopPropagation();
                              onSelectTeam(summary.team_id!);
                            }}
                            className="text-sm text-gray-600 dark:text-gray-400 hover:text-primary-500 hover:underline transition-colors"
                          >
                            {summary.team_name ?? ""}
                          </button>
                        ) : (
                          <span className="text-sm text-gray-600 dark:text-gray-400">
                            {t("common.freeAgent")}
                          </span>
                        )}
                      </td>
                      <td className="py-2.5 px-4 text-sm text-gray-600 dark:text-gray-400 font-medium">
                        {formatVal(summary.market_value)}
                      </td>
                      <td className="py-2.5 px-4">
                        <span
                          className={`font-heading font-bold text-base tabular-nums ${ratingTierClass(
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
            {total === 0 && (
              <div className="p-8 text-center text-gray-500 dark:text-gray-400 text-sm">
                {t("players.noMatch")}
              </div>
            )}
          </div>
          {totalPages > 1 && (
            <div className="flex items-center justify-between px-4 py-3 border-t border-gray-100 dark:border-navy-600">
              <p className="text-xs text-gray-400 dark:text-gray-500 font-heading">
                {t("players.showingRange", {
                  from: (page - 1) * pageSize + 1,
                  to: Math.min(page * pageSize, total),
                  total,
                })}
              </p>
              <div className="flex items-center gap-1">
                <button
                  type="button"
                  onClick={() => patchQuery({ page: 1 })}
                  disabled={page === 1}
                  className="p-1.5 rounded-lg text-gray-400 hover:text-gray-700 dark:hover:text-white hover:bg-gray-100 dark:hover:bg-navy-700 disabled:opacity-30 disabled:pointer-events-none transition-colors"
                >
                  <ChevronsLeft className="w-4 h-4" />
                </button>
                <button
                  type="button"
                  onClick={() => patchQuery({ page: Math.max(1, page - 1) })}
                  disabled={page === 1}
                  className="p-1.5 rounded-lg text-gray-400 hover:text-gray-700 dark:hover:text-white hover:bg-gray-100 dark:hover:bg-navy-700 disabled:opacity-30 disabled:pointer-events-none transition-colors"
                >
                  <ChevronLeft className="w-4 h-4" />
                </button>
                <span className="px-3 py-1 text-xs font-heading font-bold text-gray-600 dark:text-gray-300">
                  {page} / {totalPages}
                </span>
                <button
                  type="button"
                  onClick={() => patchQuery({ page: Math.min(totalPages, page + 1) })}
                  disabled={page === totalPages}
                  className="p-1.5 rounded-lg text-gray-400 hover:text-gray-700 dark:hover:text-white hover:bg-gray-100 dark:hover:bg-navy-700 disabled:opacity-30 disabled:pointer-events-none transition-colors"
                >
                  <ChevronRight className="w-4 h-4" />
                </button>
                <button
                  type="button"
                  onClick={() => patchQuery({ page: totalPages })}
                  disabled={page === totalPages}
                  className="p-1.5 rounded-lg text-gray-400 hover:text-gray-700 dark:hover:text-white hover:bg-gray-100 dark:hover:bg-navy-700 disabled:opacity-30 disabled:pointer-events-none transition-colors"
                >
                  <ChevronsRight className="w-4 h-4" />
                </button>
              </div>
            </div>
          )}
        </CardBody>
      </Card>
      {bidTarget && (
        <TransferBidModal
          bidTarget={bidTarget}
          teams={gameState.teams}
          bidAmount={bidAmount}
          onBidAmountChange={setBidAmount}
          myTeam={myTeam}
          bidFee={bidFee}
          bidProjection={bidProjection}
          bidFeedback={bidFeedback}
          activeBidOffer={activeBidOffer}
          hasExistingOffer={hasExistingOffer}
          talksResumeOn={talksResumeOn}
          bidResult={bidResult}
          bidLoading={bidLoading}
          bidSubmitDisabled={bidSubmitDisabled}
          onSubmit={handleMakeBid}
          onClose={closeBidNegotiation}
        />
      )}
      {freeAgentTarget && (
        <FreeAgentContractModal
          player={freeAgentTarget}
          teams={gameState.teams}
          wage={contractWage}
          onWageChange={setContractWage}
          contractLength={contractLength}
          onContractLengthChange={setContractLength}
          projection={contractProjection}
          feedback={contractFeedback}
          statusMessage={contractStatusMessage(t)}
          statusClassName={contractStatusClassName}
          submitting={contractSubmitting}
          submitDisabled={contractSubmitDisabled}
          onSubmit={submitFreeAgentContract}
          onClose={closeFreeAgentContract}
        />
      )}
    </div>
  );
}

function SortHeader({
  label,
  sortKey,
  current,
  onClick,
}: {
  label: string;
  sortKey: PlayerSortKey;
  current: PlayerSortKey;
  asc: boolean;
  onClick: (k: PlayerSortKey) => void;
}) {
  const isActive = current === sortKey;
  return (
    <th
      onClick={() => onClick(sortKey)}
      className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 cursor-pointer hover:text-gray-700 dark:hover:text-gray-200 transition-colors select-none"
    >
      <span className="flex items-center gap-1">
        {label}
        <ArrowUpDown
          className={`w-3 h-3 ${isActive ? "text-primary-500" : "text-gray-300 dark:text-navy-600"}`}
        />
      </span>
    </th>
  );
}
