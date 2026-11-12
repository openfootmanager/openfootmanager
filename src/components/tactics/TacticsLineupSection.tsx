import type { JSX } from "react";

import type { PlayerSelectionOptions } from "../../store/gameStore";
import type { PlayerRole } from "../../store/types";
import TacticsLineupFrame from "./TacticsLineupFrame";
import TacticsPitch from "./TacticsPitch";
import TacticsPlayerTable from "./TacticsPlayerTable";
import type { useTacticsLineup } from "./useTacticsLineup";

/** Each slot's position by its index, whatever order the pitch draws them in. */
function slotPositionsByIndex(slots: readonly { index: number; position: string }[]): string[] {
  const positions: string[] = [];
  for (const slot of slots) positions[slot.index] = slot.position;
  return positions;
}

/** What the lineup region reads and drives of the Tactics tab's lineup state. */
export type TacticsLineupState = Pick<
  ReturnType<typeof useTacticsLineup>,
  | "startingXI"
  | "bench"
  | "pitchSlots"
  | "lineup"
  | "team"
  | "formation"
  | "familiarity"
  | "lineupReady"
  | "dragState"
  | "hoveredSlot"
  | "selectedPlayer"
  | "selectedPlayerId"
  | "comparePlayerId"
  | "effectiveMatchRoles"
  | "outOfPositionCount"
  | "clearLineupSelection"
  | "handleAssignBestFit"
  | "handleAssignMatchRole"
  | "handleDemoteStarter"
  | "handleDragStart"
  | "handleLineupPlayerClick"
  | "handlePromoteBenchPlayer"
  | "handleSlotDragLeave"
  | "handleSlotDragOver"
  | "handleSlotDrop"
  | "resetDragState"
>;

interface TacticsLineupSectionProps {
  state: TacticsLineupState;
  /** The roles shown in the table and on the pitch: the match's own, not a stale stored one. */
  playerRoles: Record<string, PlayerRole>;
  /** A role chosen in the table or on the pitch: one write, whose refusal the tab shows. */
  onRoleChange: (playerId: string, role: PlayerRole) => void;
  onSelectPlayer: (id: string, options?: PlayerSelectionOptions) => void;
}

/**
 * The lineup region of the Tactics tab: the player table and the pitch inside their frame, with the
 * callbacks that turn their events into the lineup state's operations (each is a promise the
 * screen does not wait on).
 */
export default function TacticsLineupSection({
  state,
  playerRoles,
  onRoleChange,
  onSelectPlayer,
}: TacticsLineupSectionProps): JSX.Element {
  const {
    startingXI,
    bench,
    pitchSlots,
    lineup,
    team,
    formation,
    familiarity,
    lineupReady,
    dragState,
    hoveredSlot,
    selectedPlayer,
    selectedPlayerId,
    comparePlayerId,
    effectiveMatchRoles,
    outOfPositionCount,
    clearLineupSelection,
    handleAssignBestFit,
    handleAssignMatchRole,
    handleDemoteStarter,
    handleDragStart,
    handleLineupPlayerClick,
    handlePromoteBenchPlayer,
    handleSlotDragLeave,
    handleSlotDragOver,
    handleSlotDrop,
    resetDragState,
  } = state;

  return (
    <TacticsLineupFrame
      table={
        <TacticsPlayerTable
          starters={startingXI}
          others={bench}
          slotPositions={slotPositionsByIndex(pitchSlots)}
          lineup={lineup}
          playerRoles={playerRoles}
          matchRoles={effectiveMatchRoles}
          selectedPlayerId={selectedPlayerId}
          comparePlayerId={comparePlayerId}
          onTacticalSelect={(playerId, section) => {
            void handleLineupPlayerClick(playerId, section);
          }}
          onRoleChange={onRoleChange}
          onOpenPlayerProfile={onSelectPlayer}
          onAssignMatchRole={(role, playerId) => {
            void handleAssignMatchRole(role, playerId);
          }}
          onPromoteBench={(playerId) => {
            void handlePromoteBenchPlayer(playerId);
          }}
          onDemoteStarter={(playerId) => {
            void handleDemoteStarter(playerId);
          }}
          onClearSelection={clearLineupSelection}
          onDragStart={handleDragStart}
          onDragEnd={resetDragState}
        />
      }
      pitch={
        <TacticsPitch
          familiarity={familiarity}
          lineupReady={lineupReady}
          dragState={dragState}
          formation={formation}
          comparePlayerId={comparePlayerId}
          hoveredSlot={hoveredSlot}
          matchRoles={effectiveMatchRoles}
          onRoleChange={onRoleChange}
          playerRoles={playerRoles}
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
      }
    />
  );
}
