import type { JSX } from "react";
import type { GameStateData, PlayerSelectionOptions } from "../../store/gameStore";
import { useTranslation } from "react-i18next";
import { setPlayerRole } from "../../services/squadService";

import TacticsPitch from "./TacticsPitch";
import TacticsPlayerTable from "./TacticsPlayerTable";
import TacticsSetPieces from "./TacticsSetPieces";
import TacticsWorkbench from "./TacticsWorkbench";
import TacticsCommandBar from "./TacticsCommandBar";
import TacticsCompareDialog from "./TacticsCompareDialog";
import { useTacticsLibrary } from "./useTacticsLibrary";
import { useTacticsLineup } from "./useTacticsLineup";

/** Each slot's position by its index, whatever order the pitch draws them in. */
function slotPositionsByIndex(slots: readonly { index: number; position: string }[]): string[] {
  const positions: string[] = [];
  for (const slot of slots) positions[slot.index] = slot.position;
  return positions;
}

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
    pitchSlots,
    outOfPositionCount,
    effectiveMatchRoles,
    autoSetPieces,
    selectedPlayer,
    comparePlayer,
    selectedPlayerId,
    comparePlayerId,
    canConfirmSwap,
    handleFormationChange,
    handlePlayStyleChange,
    handleApplyTacticPreset,
    handleAssignBestFit,
    handlePromoteBenchPlayer,
    handleDemoteStarter,
    clearLineupSelection,
    handleLineupPlayerClick,
    handleConfirmSwap,
    handleAssignMatchRole,
    handleTacticsPhaseChange,
    lineupReady,
    lineup,
    familiarity,
  } = useTacticsLineup({ gameState, onGameUpdate });

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
  });

  if (!team) {
    return <p className="text-gray-500 dark:text-gray-400">{t("common.noTeam")}</p>;
  }

  return (
    // The tab fills the workspace (it is one of `tabFillsWorkspace`'s): its regions scroll, the page
    // doesn't.
    <div className="flex min-h-0 w-full flex-1 flex-col gap-4">
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
            startingPlayers={startingXI}
          />
        }
      >
        <div className="grid min-h-0 grid-cols-1 gap-4 xl:h-full xl:grid-cols-[minmax(0,1fr)_22rem] xl:items-start">
          {/* The player table: the eleven by slot, then the substitutes and the reserves. */}
          <div className="min-h-0 overflow-hidden rounded-lg border border-line bg-panel xl:h-full">
            <TacticsPlayerTable
              starters={startingXI}
              others={bench}
              slotPositions={slotPositionsByIndex(pitchSlots)}
              lineup={lineup}
              playerRoles={team?.player_roles}
              matchRoles={effectiveMatchRoles}
              selectedPlayerId={selectedPlayerId}
              comparePlayerId={comparePlayerId}
              onTacticalSelect={(playerId, section) => {
                void handleLineupPlayerClick(playerId, section);
              }}
              onRoleChange={(playerId, role) => {
                void setPlayerRole(playerId, role)
                  .then(onGameUpdate)
                  .catch((error: unknown) => {
                    console.error("Failed to set player role:", error);
                  });
              }}
              onOpenPlayerProfile={onSelectPlayer}
              onAssignMatchRole={(role, playerId) => {
                void handleAssignMatchRole(role, playerId);
              }}
              onAssignBestFit={(playerId) => {
                void handleAssignBestFit(playerId);
              }}
              onPromoteBench={(playerId) => {
                void handlePromoteBenchPlayer(playerId);
              }}
              onDemoteStarter={(playerId) => {
                void handleDemoteStarter(playerId);
              }}
              onClearSelection={clearLineupSelection}
            />
          </div>

          {/* The pitch shows the eleven; every change is made in the table. */}
          <TacticsPitch
            familiarity={familiarity}
            lineupReady={lineupReady}
            formation={formation}
            comparePlayerId={comparePlayerId}
            matchRoles={effectiveMatchRoles}
            tacticsPhase={team?.tactics_phase}
            outOfPositionCount={outOfPositionCount}
            pitchSlots={pitchSlots}
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
