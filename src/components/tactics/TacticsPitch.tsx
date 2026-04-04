import type { DragEvent, JSX } from "react";
import { Star } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { PlayerData } from "../../store/gameStore";
import { Badge, Card } from "../ui";
import {
  type DragState,
  type PitchSlotRow,
  type SquadSection,
} from "../squad/SquadTab.helpers";
import {
  BenchPlayersSection,
  PitchSurfaceSection,
} from "./TacticsPitch.sections";

interface TacticsPitchProps {
  benchPlayers: PlayerData[];
  dragState: DragState | null;
  formation: string;
  comparePlayerId: string | null;
  hoveredSlot: number | null;
  onClearSelection: () => void;
  onDragStart: (
    event: DragEvent<HTMLElement>,
    playerId: string,
    from: SquadSection,
    slotIndex: number | null,
  ) => void;
  onDragEnd: () => void;
  onLineupPlayerClick: (playerId: string, section: SquadSection) => void;
  onSlotDragOver: (event: DragEvent<HTMLElement>, slotIndex: number) => void;
  onSlotDragLeave: (slotIndex: number) => void;
  onSlotDrop: (event: DragEvent<HTMLElement>, slotIndex: number) => void;
  outOfPositionCount: number;
  pitchSlotRows: PitchSlotRow[];
  selectedPlayer: PlayerData | null;
  selectedPlayerId: string | null;
}

export default function TacticsPitch({
  benchPlayers,
  dragState,
  formation,
  comparePlayerId,
  hoveredSlot,
  onClearSelection,
  onDragEnd,
  onDragStart,
  onLineupPlayerClick,
  onSlotDragLeave,
  onSlotDragOver,
  onSlotDrop,
  outOfPositionCount,
  pitchSlotRows,
  selectedPlayer,
  selectedPlayerId,
}: TacticsPitchProps): JSX.Element {
  const { t } = useTranslation();

  return (
    <Card className="overflow-hidden">
      <div className="flex flex-wrap items-center justify-between gap-3 rounded-t-xl border-b border-gray-100 bg-linear-to-r from-navy-700 to-navy-800 p-4 dark:border-navy-600">
        <div>
          <h3 className="flex items-center gap-2 text-sm font-heading font-bold uppercase tracking-wide text-white">
            <Star className="h-4 w-4 fill-current text-accent-400" />
            {t("preMatch.startingXI", "Starting XI")} — {formation}
          </h3>
          <p className="mt-0.5 text-xs text-gray-400">
            {t(
              "tactics.pitchInteractionHint",
              "Drag players between the pitch and bench, click one player to inspect them, click a second player to compare them, then confirm the swap in the compare panel.",
            )}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Badge
            variant={outOfPositionCount > 0 ? "danger" : "success"}
            size="sm"
          >
            {outOfPositionCount} {t("squad.outOfPosition", "Out of position")}
          </Badge>
          {selectedPlayer ? (
            <button
              type="button"
              onClick={onClearSelection}
              className="text-xs font-heading font-bold uppercase tracking-wider text-accent-400 hover:text-accent-300"
            >
              {t("common.clear", "Clear")}
            </button>
          ) : null}
        </div>
      </div>
      <div className="p-4 sm:p-6">
        <PitchSurfaceSection
          comparePlayerId={comparePlayerId}
          dragState={dragState}
          hoveredSlot={hoveredSlot}
          onDragEnd={onDragEnd}
          onDragStart={onDragStart}
          onLineupPlayerClick={onLineupPlayerClick}
          onSlotDragLeave={onSlotDragLeave}
          onSlotDragOver={onSlotDragOver}
          onSlotDrop={onSlotDrop}
          pitchSlotRows={pitchSlotRows}
          selectedPlayerId={selectedPlayerId}
        />
        <BenchPlayersSection
          benchPlayers={benchPlayers}
          comparePlayerId={comparePlayerId}
          dragState={dragState}
          onDragEnd={onDragEnd}
          onDragStart={onDragStart}
          onLineupPlayerClick={onLineupPlayerClick}
          selectedPlayerId={selectedPlayerId}
        />
      </div>
    </Card>
  );
}
