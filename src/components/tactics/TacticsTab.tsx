import type { JSX } from "react";
import type {
  GameStateData,
  PlayerSelectionOptions,
} from "../../store/gameStore";
import { useTranslation } from "react-i18next";
import { setPlayerRole } from "../../services/squadService";

import TacticsPitch from "./TacticsPitch";
import TacticsPlayerList from "./TacticsPlayerList";
import TacticsRightPanel from "./TacticsRightPanel";
import TacticsCommandBar from "./TacticsCommandBar";
import TacticsPlayerFocusPanel from "./TacticsPlayerFocusPanel";
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
  onGameUpdate,
}: TacticsTabProps): JSX.Element {
  const { t } = useTranslation();

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
  } = useTacticsLineup({ gameState, onGameUpdate });

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
    applyTacticSelection,
    handleCreateCustomTactic,
    handleDuplicateTactic,
    handleSaveTactic,
  } = useTacticsLibrary({
    gameState,
    formation,
    activePlayStyle,
    initialPreset,
    onFormationChange: handleFormationChange,
    onPlayStyleChange: handlePlayStyleChange,
  });

  if (!team) {
    return (
      <p className="text-gray-500 dark:text-gray-400">{t("common.noTeam")}</p>
    );
  }

  return (
    <div className="flex w-full flex-col gap-5">
      <div
        ref={dragPreviewRef}
        aria-hidden="true"
        className="pointer-events-none fixed -left-20 top-0 h-8 w-8 rounded-full border border-white/15 bg-navy-900/90 shadow-lg"
      />
      <TacticsCommandBar
        activeTactic={activeTactic}
        activePlayStyle={activePlayStyle}
        formation={formation}
        isDirty={isCommandBarDirty}
        onCreateNew={handleCreateCustomTactic}
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

      <div className="grid grid-cols-1 gap-5 xl:grid-cols-[260px_1fr_270px] xl:items-start">
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
              });
          }}
          playerRoles={team?.player_roles}
          tacticsPhase={team?.tactics_phase}
          teamKitPattern={team?.kit_pattern}
          teamPrimaryColor={team?.colors?.primary}
          teamSecondaryColor={team?.colors?.secondary}
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

        {/* Right: roles + phase blueprint */}
        <TacticsRightPanel
          allSquad={roster}
          matchRoles={team.match_roles}
          onGameUpdate={onGameUpdate}
          onTacticsPhaseChange={(patch) => {
            void handleTacticsPhaseChange(patch);
          }}
          startingPlayers={startingXI}
          tacticsPhase={team?.tactics_phase}
        />
      </div>

      {/* Inspector modal — only when both players are selected for comparison */}
      {selectedPlayer && comparePlayer && (
        <>
          <div
            className="fixed inset-0 z-40 bg-black/50 backdrop-blur-sm"
            onClick={clearLineupSelection}
          />
          <div className="pointer-events-none fixed inset-0 z-50 flex items-center justify-center p-4">
            <div className="pointer-events-auto w-full max-w-lg max-h-[85vh] overflow-y-auto">
              <TacticsPlayerFocusPanel
                canConfirmSwap={canConfirmSwap}
                comparePlayer={comparePlayer}
                onClose={clearLineupSelection}
                onConfirmSwap={() => { void handleConfirmSwap(); }}
                selectedPlayer={selectedPlayer}
              />
            </div>
          </div>
        </>
      )}
    </div>
  );
}
