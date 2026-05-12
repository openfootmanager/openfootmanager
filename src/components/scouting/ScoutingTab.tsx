import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { GameStateData } from "../../store/gameStore";
import { getErrorMessage } from "../../utils/errorMessage";
import {
  Card,
  CardBody,
} from "../ui";
import {
  Eye,
  ScanSearch,
} from "lucide-react";
import {
  cancelYouthScouting,
  reassignYouthScouting,
  sendScout,
  startYouthScouting,
} from "../../services/scoutingService";
import {
  calculateAvailableScouts,
  scoutMaxSlots,
} from "./ScoutingTab.helpers";
import {
  buildAlreadyScoutingIds,
  filterScoutablePlayers,
  paginateScoutablePlayers,
} from "./ScoutingTab.model";
import ScoutingAssignmentsList from "./ScoutingAssignmentsList";
import ScoutingOverviewCards from "./ScoutingOverviewCards";
import ScoutingScoutDetailsCard from "./ScoutingScoutDetailsCard";
import ScoutingPlayerSearchCard from "./ScoutingPlayerSearchCard";
import ScoutingYouthRecruitmentCard from "./ScoutingYouthRecruitmentCard";
import TransferBidModal from "../transfers/TransferBidModal";
import { useTransferBidFlow } from "../transfers/useTransferBidFlow";

interface ScoutingTabProps {
  gameState: GameStateData;
  onGameUpdate: (state: GameStateData) => void;
  onSelectPlayer?: (id: string) => void;
  onSelectTeam?: (id: string) => void;
}

const SCOUTING_PAGE_SIZE = 20;

export default function ScoutingTab({
  gameState,
  onGameUpdate,
  onSelectPlayer,
  onSelectTeam,
}: ScoutingTabProps) {
  const { t } = useTranslation();
  const [searchQuery, setSearchQuery] = useState("");
  const [posFilter, setPosFilter] = useState<string>("All");
  const [sending, setSending] = useState<string | null>(null);
  const [playerSearchError, setPlayerSearchError] = useState<string | null>(null);
  const [startingYouthSearch, setStartingYouthSearch] = useState(false);
  const [selectedYouthScoutId, setSelectedYouthScoutId] = useState("");
  const [youthRegion, setYouthRegion] = useState("Domestic");
  const [youthObjective, setYouthObjective] = useState("Balanced");
  const [youthTargetPosition, setYouthTargetPosition] = useState("");
  const [youthSearchError, setYouthSearchError] = useState<string | null>(null);
  const [page, setPage] = useState(0);
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

  const myTeamId = gameState.manager.team_id ?? "";
  const scouts = gameState.staff.filter(
    (s) => s.role === "Scout" && s.team_id === myTeamId,
  );
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

  const allScoutable = filterScoutablePlayers({
    players: gameState.players,
    teams: gameState.teams,
    myTeamId,
    posFilter,
    searchQuery,
  });
  const { totalPages, safePage, players: scoutablePlayers } =
    paginateScoutablePlayers(allScoutable, page, SCOUTING_PAGE_SIZE);

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
      setPlayerSearchError(getErrorMessage(err));
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
      setYouthSearchError(String(err));
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
      setYouthSearchError(String(err));
    }
  };

  const handleReassignYouthScouting = async (
    assignmentId: string,
    scoutId: string,
  ) => {
    setYouthSearchError(null);
    try {
      const updated = await reassignYouthScouting(assignmentId, scoutId);
      onGameUpdate(updated);
    } catch (err) {
      console.error("Failed to reassign youth scouting:", err);
      setYouthSearchError(String(err));
    }
  };

  return (
    <div className="max-w-6xl mx-auto flex flex-col gap-5">
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
          (sum, scout) => sum + scoutMaxSlots(scout.attributes.judging_ability),
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
          posFilter={posFilter}
          searchQuery={searchQuery}
          errorMessage={playerSearchError}
          alreadyScoutingIds={alreadyScoutingIds}
          availableScoutCount={availableScouts.length}
          sendingPlayerId={sending}
          safePage={safePage}
          totalPages={totalPages}
          totalPlayers={allScoutable.length}
          pageSize={SCOUTING_PAGE_SIZE}
          onPositionFilterChange={(position) => {
            setPosFilter(position);
            setPage(0);
          }}
          onSearchQueryChange={(query) => {
            setSearchQuery(query);
            setPage(0);
          }}
          onBidPlayer={openBidNegotiation}
          onSelectPlayer={onSelectPlayer}
          onSelectTeam={onSelectTeam}
          onSendScout={handleSendScout}
          onPreviousPage={() => setPage((currentPage) => Math.max(0, currentPage - 1))}
          onNextPage={() =>
            setPage((currentPage) => Math.min(totalPages - 1, currentPage + 1))
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
    </div>
  );
}
