import type { JSX } from "react";
import type { GameStateData, PlayerSelectionOptions } from "../../store/gameStore";
import { useTranslation } from "react-i18next";
import TacticsRolesPanel from "./TacticsRolesPanel";
import {
  TacticsLineupContent,
  TacticsTabModeTabs,
} from "./TacticsTab.sections";
import { useTacticsTabViewModel } from "./useTacticsTabViewModel";

interface TacticsTabProps {
  gameState: GameStateData;
  onSelectPlayer: (id: string, options?: PlayerSelectionOptions) => void;
  onGameUpdate: (g: GameStateData) => void;
}

export default function TacticsTab({
  gameState,
  onSelectPlayer,
  onGameUpdate,
}: TacticsTabProps): JSX.Element {
  const { t } = useTranslation();
  const model = useTacticsTabViewModel({ gameState, onGameUpdate });

  if (!model.myTeam) {
    return (
      <p className="text-gray-500 dark:text-gray-400">{t("common.noTeam")}</p>
    );
  }

  return (
    <div className="max-w-6xl mx-auto flex flex-col gap-4">
      <div
        ref={model.dragPreviewRef}
        aria-hidden="true"
        className="pointer-events-none fixed -left-20 top-0 h-8 w-8 rounded-full border border-white/15 bg-navy-900/90 shadow-lg"
      />
      <TacticsTabModeTabs
        activeTab={model.activeTab}
        onSelectTab={model.setActiveTab}
      />

      {model.activeTab === "lineup" ? (
        <TacticsLineupContent
          activePlayStyle={model.activePlayStyle}
          bench={model.bench}
          canConfirmSwap={model.canConfirmSwap}
          comparePlayer={model.comparePlayer}
          comparePlayerId={model.comparePlayerId}
          currentDate={model.currentDate}
          dragState={model.dragState}
          filteredBench={model.filteredBench}
          filteredStartingXI={model.filteredStartingXI}
          formation={model.formation}
          handleClearFilters={model.handleClearFilters}
          handleConfirmSwap={model.handleConfirmSwap}
          handleDragStart={model.handleDragStart}
          handleFormationChange={model.handleFormationChange}
          handleLineupPlayerClick={model.handleLineupPlayerClick}
          handlePlayStyleChange={model.handlePlayStyleChange}
          handleSlotDragLeave={model.handleSlotDragLeave}
          handleSlotDragOver={model.handleSlotDragOver}
          handleSlotDrop={model.handleSlotDrop}
          hoveredSlot={model.hoveredSlot}
          onClearSelection={model.clearLineupSelection}
          onPlayerSearchChange={model.setPlayerSearch}
          onPositionFilterChange={model.setPositionFilter}
          onSelectPlayer={onSelectPlayer}
          outOfPositionCount={model.outOfPositionCount}
          pitchSlotRows={model.pitchSlotRows}
          playerSearch={model.playerSearch}
          positionFilter={model.positionFilter}
          resetDragState={model.resetDragState}
          selectedPlayer={model.selectedPlayer}
          selectedPlayerId={model.selectedPlayerId}
          sortDir={model.sortDir}
          sortKey={model.sortKey}
          startingXiCount={model.startingXI.length}
          toggleSort={model.toggleSort}
          xiActivePosition={model.xiActivePosition}
        />
      ) : (
        <TacticsRolesPanel
          allSquad={model.roster}
          matchRoles={model.myTeam.match_roles}
          onGameUpdate={onGameUpdate}
          startingPlayers={model.startingXI}
        />
      )}
    </div>
  );
}
