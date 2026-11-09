import type { JSX } from "react";
import { useCallback, useState } from "react";
import type { GameStateData, PlayerSelectionOptions } from "../../store/gameStore";
import { useTranslation } from "react-i18next";
import { setPlayerRole } from "../../services/squadService";
import { resolveTranslatedErrorMessage } from "../../utils/errorMessage";

import TacticsPitch from "./TacticsPitch";
import TacticsPlayerList from "./TacticsPlayerList";
import TacticsSetPieces from "./TacticsSetPieces";
import TacticsWorkbench from "./TacticsWorkbench";
import TacticsCommandBar from "./TacticsCommandBar";
import TacticsCompareDialog from "./TacticsCompareDialog";
import { useTacticsLibrary } from "./useTacticsLibrary";
import { useTacticsFilters } from "./useTacticsFilters";
import { useTacticsLineup } from "./useTacticsLineup";

interface TacticsTabProps {
  gameState: GameStateData | null;
  onSelectPlayer: (id: string, options?: PlayerSelectionOptions) => void;
  onGameUpdate: (g: GameStateData) => void;
}

export default function TacticsTab({
  gameState,
  onSelectPlayer,
  onGameUpdate: onGameUpdateProp,
}: TacticsTabProps): JSX.Element {
  const { t } = useTranslation();
  // Why the last tactic write did not save, in the player's language. A write that succeeds
  // clears it: the screen then shows what the backend holds.
  const [writeError, setWriteError] = useState<string | null>(null);
  const onWriteError = useCallback((error: unknown) => {
    setWriteError(resolveTranslatedErrorMessage(error));
  }, []);
  const onGameUpdate = useCallback(
    (game: GameStateData) => {
      setWriteError(null);
      onGameUpdateProp(game);
    },
    [onGameUpdateProp],
  );

  const {
    team,
    formation,
    activePlayStyle,
    initialPreset,
    roster,
    startingXI,
    bench,
    xiActivePosition,
    pitchSlots,
    outOfPositionCount,
    effectiveMatchRoles,
    autoSetPieces,
    selectedPlayer,
    comparePlayer,
    selectedPlayerId,
    comparePlayerId,
    canConfirmSwap,
    dragState,
    hoveredSlot,
    dragPreviewRef,
    handleFormationChange,
    handlePlayStyleChange,
    handleApplyTacticPreset,
    handleAssignBestFit,
    handlePromoteBenchPlayer,
    handleDemoteStarter,
    clearLineupSelection,
    handleDragStart,
    handleSlotDragOver,
    handleSlotDragLeave,
    handleSlotDrop,
    handleLineupPlayerClick,
    handleConfirmSwap,
    resetDragState,
    handleAssignMatchRole,
    handleTacticsPhaseChange,
    lineupReady,
    familiarity,
  } = useTacticsLineup({ gameState, onGameUpdate, onWriteError });

  const {
    playerSearch,
    setPlayerSearch,
    positionFilter,
    setPositionFilter,
    filteredStartingXI,
    filteredBench,
    handleClearFilters,
  } = useTacticsFilters({ startingXI, bench, xiActivePosition });

  const {
    activeTactic,
    tacticLibrary,
    isCommandBarDirty,
    canResetTactic,
    handleResetTactic,
    applyTacticSelection,
    handleCreateCustomTactic,
    handleDuplicateTactic,
    handleSaveTactic,
    canDeleteTactic,
    handleDeleteTactic,
  } = useTacticsLibrary({
    gameState,
    formation,
    activePlayStyle,
    initialPreset,
    onApplyPreset: handleApplyTacticPreset,
    onGameUpdate,
    onWriteError,
    onWriteSaved: () => setWriteError(null),
  });

  if (!team) {
    return <p className="text-gray-500 dark:text-gray-400">{t("common.noTeam")}</p>;
  }

  return (
    // The tab fills the workspace (it is one of `tabFillsWorkspace`'s): its regions scroll, the page
    // doesn't.
    <div className="flex min-h-0 w-full flex-1 flex-col gap-4">
      <div
        ref={dragPreviewRef}
        aria-hidden="true"
        className="pointer-events-none fixed -left-20 top-0 h-8 w-8 rounded-full border border-white/15 bg-navy-900/90 shadow-lg"
      />
      {writeError ? (
        <p
          role="alert"
          className="rounded-lg border border-line bg-danger-tint px-4 py-2 text-body font-medium text-danger"
        >
          {writeError}
        </p>
      ) : null}
      <TacticsCommandBar
        activeTactic={activeTactic}
        activePlayStyle={activePlayStyle}
        formation={formation}
        isDirty={isCommandBarDirty}
        canReset={canResetTactic}
        onReset={() => {
          void handleResetTactic();
        }}
        canDelete={canDeleteTactic}
        onCreateNew={handleCreateCustomTactic}
        onDelete={handleDeleteTactic}
        onDuplicate={handleDuplicateTactic}
        onFormationChange={(nextFormation) => {
          void handleFormationChange(nextFormation);
        }}
        onPlayStyleChange={(playStyle) => {
          void handlePlayStyleChange(playStyle);
        }}
        onSave={handleSaveTactic}
        onSelectTactic={(id) => {
          const nextTactic = tacticLibrary.find((entry) => entry.id === id);
          if (!nextTactic) {
            return;
          }

          void applyTacticSelection(nextTactic);
        }}
        tacticLibrary={tacticLibrary}
      />

      {lineupReady ? null : (
        <p role="status" className="text-caption text-fg-muted">
          {t("common.lineupLoading")}
        </p>
      )}

      <TacticsWorkbench
        tacticsPhase={team?.tactics_phase}
        onTacticsPhaseChange={(patch) => {
          void handleTacticsPhaseChange(patch);
        }}
        setPieces={
          <TacticsSetPieces
            allSquad={roster}
            autoSetPieces={autoSetPieces}
            matchRoles={team.match_roles}
            onGameUpdate={onGameUpdate}
            onWriteError={onWriteError}
            startingPlayers={startingXI}
          />
        }
      >
        <div className="grid grid-cols-1 gap-5 xl:grid-cols-[260px_1fr] xl:items-start">
          {/* Left: player list */}
          <TacticsPlayerList
            bench={filteredBench}
            comparePlayerId={comparePlayerId}
            dragState={dragState}
            matchRoles={effectiveMatchRoles}
            onAssignMatchRole={(role, playerId) => {
              void handleAssignMatchRole(role, playerId);
            }}
            onClearFilters={handleClearFilters}
            onDemoteStarter={(playerId) => {
              void handleDemoteStarter(playerId);
            }}
            onDragEnd={resetDragState}
            onDragStart={handleDragStart}
            onOpenPlayerProfile={onSelectPlayer}
            onPlayerSearchChange={setPlayerSearch}
            onPositionFilterChange={setPositionFilter}
            onPromoteBench={(playerId) => {
              void handlePromoteBenchPlayer(playerId);
            }}
            onTacticalSelect={(playerId, section) => {
              void handleLineupPlayerClick(playerId, section);
            }}
            playerSearch={playerSearch}
            positionFilter={positionFilter}
            selectedPlayerId={selectedPlayerId}
            starters={filteredStartingXI}
            xiActivePosition={xiActivePosition}
          />

          {/* Center: pitch */}
          <TacticsPitch
            familiarity={familiarity}
            lineupReady={lineupReady}
            dragState={dragState}
            formation={formation}
            comparePlayerId={comparePlayerId}
            hoveredSlot={hoveredSlot}
            matchRoles={effectiveMatchRoles}
            onRoleChange={(playerId, role) => {
              void setPlayerRole(playerId, role)
                .then(onGameUpdate)
                .catch((error: unknown) => {
                  console.error("Failed to set player role:", error);
                  onWriteError(error);
                });
            }}
            playerRoles={team?.player_roles}
            tacticsPhase={team?.tactics_phase}
            onAssignBestFit={(playerId) => {
              void handleAssignBestFit(playerId);
            }}
            onAssignMatchRole={(role, playerId) => {
              void handleAssignMatchRole(role, playerId);
            }}
            onClearSelection={clearLineupSelection}
            onDemoteStarter={(playerId) => {
              void handleDemoteStarter(playerId);
            }}
            onDragEnd={resetDragState}
            onDragStart={handleDragStart}
            onLineupPlayerClick={(playerId, section) => {
              void handleLineupPlayerClick(playerId, section);
            }}
            onOpenPlayerProfile={(playerId) => {
              onSelectPlayer(playerId);
            }}
            onPromoteBench={(playerId) => {
              void handlePromoteBenchPlayer(playerId);
            }}
            onSlotDragLeave={handleSlotDragLeave}
            onSlotDragOver={handleSlotDragOver}
            onSlotDrop={(event, slotIndex) => {
              void handleSlotDrop(event, slotIndex);
            }}
            outOfPositionCount={outOfPositionCount}
            pitchSlots={pitchSlots}
            selectedPlayer={selectedPlayer}
            selectedPlayerId={selectedPlayerId}
          />
        </div>
      </TacticsWorkbench>

      {/* Inspector modal — only when both players are selected for comparison */}
      {selectedPlayer && comparePlayer && (
        <TacticsCompareDialog
          canConfirmSwap={canConfirmSwap}
          comparePlayer={comparePlayer}
          onClose={clearLineupSelection}
          onConfirmSwap={() => {
            void handleConfirmSwap();
          }}
          selectedPlayer={selectedPlayer}
        />
      )}
    </div>
  );
}
