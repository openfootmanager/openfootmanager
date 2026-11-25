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
  | "selectedPlayerId"
  | "comparePlayerId"
  | "effectiveMatchRoles"
  | "outOfPositionCount"
  | "clearLineupSelection"
  | "handleAssignBestFit"
  | "handleAssignMatchRole"
  | "handleDemoteStarter"
  | "handleLineupPlayerClick"
  | "handlePromoteBenchPlayer"
>;

interface TacticsLineupSectionProps {
  state: TacticsLineupState;
  /** The roles shown in the table: the match's own, not a stale stored one. */
  playerRoles: Record<string, PlayerRole> | undefined;
  /** A role chosen in the table: one write, whose refusal the tab shows. */
  onRoleChange: (playerId: string, role: PlayerRole) => void;
  onSelectPlayer: (id: string, options?: PlayerSelectionOptions) => void;
}

/**
 * The lineup region of the Tactics tab: the player table, where every change is made, and the pitch
 * that only shows the eleven, inside their frame, with the table's callbacks that turn their events into the lineup state's operations (each is a promise the
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
    selectedPlayerId,
    comparePlayerId,
    effectiveMatchRoles,
    outOfPositionCount,
    clearLineupSelection,
    handleAssignBestFit,
    handleAssignMatchRole,
    handleDemoteStarter,
    handleLineupPlayerClick,
    handlePromoteBenchPlayer,
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
          onAssignBestFit={(playerId) => {
            void handleAssignBestFit(playerId);
          }}
          onClearSelection={clearLineupSelection}
        />
      }
      pitch={
        // The pitch shows the eleven; every change is made in the table.
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
      }
    />
  );
}
