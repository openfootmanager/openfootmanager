import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { GameStateData, PlayerData } from "../../store/gameStore";
import type { PlayersPageQuery } from "../../services/playersService";
import { usePlayersPage } from "../../hooks/usePlayersPage";
import { getErrorMessage, resolveTranslatedErrorMessage } from "../../utils/errorMessage";
import { Card, CardBody } from "../ui";
import { Eye, ScanSearch } from "lucide-react";
import {
  cancelYouthScouting,
  reassignYouthScouting,
  sendScout,
  startYouthScouting,
} from "../../services/scoutingService";
import { calculateAvailableScouts, scoutMaxSlots } from "./ScoutingTab.helpers";
import { buildAlreadyScoutingIds } from "./ScoutingTab.model";
import ScoutingAssignmentsList from "./ScoutingAssignmentsList";
import ScoutingOverviewCards from "./ScoutingOverviewCards";
import ScoutingScoutDetailsCard from "./ScoutingScoutDetailsCard";
import ScoutingPlayerSearchCard from "./ScoutingPlayerSearchCard";
import ScoutingYouthRecruitmentCard from "./ScoutingYouthRecruitmentCard";
import FreeAgentContractModal from "../transfers/FreeAgentContractModal";
import TransferBidModal from "../transfers/TransferBidModal";
import { useFreeAgentContractFlow } from "../transfers/useFreeAgentContractFlow";
import { useTransferBidFlow } from "../transfers/useTransferBidFlow";

interface ScoutingTabProps {
  gameState: GameStateData;
  onGameUpdate: (state: GameStateData) => void;
  onSelectPlayer?: (id: string) => void;
  onSelectTeam?: (id: string) => void;
}

const SCOUTING_PAGE_SIZE = 20;

// The players a scout could be sent to watch, best first. The backend applies the rules.
const SCOUTABLE_QUERY: PlayersPageQuery = {
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
  scope: "scoutable",
  team_id: null,
  status: "all",
  sort_key: "ovr",
  sort_asc: false,
  page: 1,
  page_size: SCOUTING_PAGE_SIZE,
};

export default function ScoutingTab({
  gameState,
  onGameUpdate,
  onSelectPlayer,
  onSelectTeam,
}: ScoutingTabProps) {
  const { t } = useTranslation();
  const [sending, setSending] = useState<string | null>(null);
  const [playerSearchError, setPlayerSearchError] = useState<string | null>(null);
  const [startingYouthSearch, setStartingYouthSearch] = useState(false);
  const [selectedYouthScoutId, setSelectedYouthScoutId] = useState("");
  const [youthRegion, setYouthRegion] = useState("Domestic");
  const [youthObjective, setYouthObjective] = useState("Balanced");
  const [youthTargetPosition, setYouthTargetPosition] = useState("");
  const [youthSearchError, setYouthSearchError] = useState<string | null>(null);
  const {
    query: scoutableQuery,
    patchQuery: patchScoutableQuery,
    slice: scoutableSlice,
    refetch: refetchScoutable,
  } = usePlayersPage(SCOUTABLE_QUERY, t);
  // Sending a scout, a hire or a day passing changes who can be scouted: ask again whenever the
  // game the screen shows is a new one (not on first render, which has just asked).
  const shownGame = useRef(gameState);
  useEffect(() => {
    if (shownGame.current === gameState) return;
    shownGame.current = gameState;
    refetchScoutable();
  }, [gameState, refetchScoutable]);
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
    bidSubmitDisabled,
    openBidNegotiation,
    closeBidNegotiation,
    handleMakeBid,
  } = useTransferBidFlow({
    gameState,
    onGameUpdate,
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
    onGameUpdate,
  });

  const myTeamId = gameState.manager.team_id ?? "";
  const scouts = gameState.staff.filter((s) => s.role === "Scout" && s.team_id === myTeamId);
  const assignments = gameState.scouting_assignments || [];
  const youthAssignments = gameState.youth_scouting_assignments || [];
  const allAssignments = [...assignments, ...youthAssignments];
  const availableScouts = calculateAvailableScouts(scouts, allAssignments);

  useEffect(() => {
    if (
      selectedYouthScoutId &&
      availableScouts.some((scout) => scout.id === selectedYouthScoutId)
    ) {
      return;
    }

    setSelectedYouthScoutId(availableScouts[0]?.id ?? "");
  }, [availableScouts, selectedYouthScoutId]);

  // Rows show contracts and offers, which live on the full player; the page says who is on it.
  const playersById = useMemo(
    () => new Map(gameState.players.map((player) => [player.id, player])),
    [gameState.players],
  );
  const scoutablePlayers = (scoutableSlice?.items ?? [])
    .map((summary) => playersById.get(summary.id))
    .filter((player): player is PlayerData => player !== undefined);
  const totalScoutable = scoutableSlice?.total ?? 0;
  const totalPages = Math.max(1, Math.ceil(totalScoutable / SCOUTING_PAGE_SIZE));
  const safePage = Math.min(scoutableQuery.page, totalPages) - 1;

  const alreadyScoutingIds = buildAlreadyScoutingIds(assignments);

  const handleSendScout = async (playerId: string) => {
    if (availableScouts.length === 0) {
      setPlayerSearchError(null);
      return;
    }
    const scout = availableScouts[0];
    setPlayerSearchError(null);
    setSending(playerId);
    try {
      const updated = await sendScout(scout.id, playerId);
      setPlayerSearchError(null);
      onGameUpdate(updated);
    } catch (err) {
      console.error("Failed to send scout:", err);
      setPlayerSearchError(resolveTranslatedErrorMessage(getErrorMessage(err), t));
    } finally {
      setSending(null);
    }
  };

  const handleStartYouthScouting = async () => {
    if (!selectedYouthScoutId) return;
    setStartingYouthSearch(true);
    setYouthSearchError(null);
    try {
      const updated = await startYouthScouting({
        scoutId: selectedYouthScoutId,
        region: youthRegion,
        objective: youthObjective,
        targetPosition: youthTargetPosition || null,
      });
      onGameUpdate(updated);
      setSelectedYouthScoutId("");
    } catch (err) {
      console.error("Failed to start youth scouting:", err);
      setYouthSearchError(resolveTranslatedErrorMessage(err, t));
    } finally {
      setStartingYouthSearch(false);
    }
  };

  const handleCancelYouthScouting = async (assignmentId: string) => {
    setYouthSearchError(null);
    try {
      const updated = await cancelYouthScouting(assignmentId);
      onGameUpdate(updated);
    } catch (err) {
      console.error("Failed to cancel youth scouting:", err);
      setYouthSearchError(resolveTranslatedErrorMessage(err, t));
    }
  };

  const handleReassignYouthScouting = async (assignmentId: string, scoutId: string) => {
    setYouthSearchError(null);
    try {
      const updated = await reassignYouthScouting(assignmentId, scoutId);
      onGameUpdate(updated);
    } catch (err) {
      console.error("Failed to reassign youth scouting:", err);
      setYouthSearchError(resolveTranslatedErrorMessage(err, t));
    }
  };

  return (
    <div className="flex flex-col gap-5">
      {/* Header */}
      <div className="flex items-center gap-3">
        <ScanSearch className="w-5 h-5 text-primary-500" />
        <h2 className="text-lg font-heading font-bold uppercase tracking-wider text-gray-800 dark:text-gray-100">
          {t("scouting.title")}
        </h2>
      </div>

      <ScoutingOverviewCards
        scouts={scouts}
        assignmentCount={allAssignments.length}
        availableScoutCount={availableScouts.length}
        totalCapacity={scouts.reduce(
          (sum, scout) => sum + scoutMaxSlots(scout.attributes.judgingAbility),
          0,
        )}
        labels={{
          scouts: t("scouting.scouts"),
          activeAssignments: t("scouting.activeAssignments"),
          freeSlots: t("scouting.freeSlots"),
        }}
      />

      <ScoutingAssignmentsList
        assignments={assignments}
        scouts={scouts}
        players={gameState.players}
        teams={gameState.teams}
        onSelectPlayer={onSelectPlayer}
        onSelectTeam={onSelectTeam}
      />

      {scouts.length > 0 && (
        <ScoutingYouthRecruitmentCard
          youthAssignments={youthAssignments}
          scouts={scouts}
          availableScouts={availableScouts}
          isStarting={startingYouthSearch}
          selectedScoutId={selectedYouthScoutId}
          region={youthRegion}
          objective={youthObjective}
          targetPosition={youthTargetPosition}
          errorMessage={youthSearchError}
          onScoutChange={setSelectedYouthScoutId}
          onRegionChange={setYouthRegion}
          onObjectiveChange={setYouthObjective}
          onTargetPositionChange={setYouthTargetPosition}
          onStartSearch={() => {
            void handleStartYouthScouting();
          }}
          onCancelSearch={(assignmentId) => {
            void handleCancelYouthScouting(assignmentId);
          }}
          onReassignSearch={(assignmentId, scoutId) => {
            void handleReassignYouthScouting(assignmentId, scoutId);
          }}
        />
      )}

      <ScoutingScoutDetailsCard
        scouts={scouts}
        assignments={assignments}
        players={gameState.players}
      />

      {scouts.length === 0 && (
        <Card>
          <CardBody>
            <div className="flex flex-col items-center gap-3 py-8">
              <Eye className="w-10 h-10 text-gray-300 dark:text-navy-600" />
              <p className="text-sm text-gray-500 dark:text-gray-400 text-center">
                {t("scouting.noScouts")}
                <br />
                <span className="text-xs">{t("scouting.noScoutsHint")}</span>
              </p>
            </div>
          </CardBody>
        </Card>
      )}

      {scouts.length > 0 && (
        <ScoutingPlayerSearchCard
          players={scoutablePlayers}
          teams={gameState.teams}
          query={scoutableQuery}
          errorMessage={playerSearchError}
          alreadyScoutingIds={alreadyScoutingIds}
          availableScoutCount={availableScouts.length}
          sendingPlayerId={sending}
          safePage={safePage}
          totalPages={totalPages}
          totalPlayers={totalScoutable}
          pageSize={SCOUTING_PAGE_SIZE}
          onFilterChange={patchScoutableQuery}
          onBidPlayer={openBidNegotiation}
          onOfferFreeAgent={openFreeAgentContract}
          onSelectPlayer={onSelectPlayer}
          onSelectTeam={onSelectTeam}
          onSendScout={handleSendScout}
          onPreviousPage={() => patchScoutableQuery({ page: Math.max(1, scoutableQuery.page - 1) })}
          onNextPage={() =>
            patchScoutableQuery({ page: Math.min(totalPages, scoutableQuery.page + 1) })
          }
        />
      )}
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
