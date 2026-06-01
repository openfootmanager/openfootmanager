import type { DragEvent, JSX } from "react";
import { Star } from "lucide-react";
import { useTranslation } from "react-i18next";

import { getPlayerOvr } from "../../lib/helpers";
import type { PlayerData } from "../../store/gameStore";
import { Badge, Card } from "../ui";
import {
  getPitchRowWidth,
  getPitchSlotWidth,
  isPlayerOutOfPosition,
  translatePositionAbbreviation,
  type DragState,
  type PitchSlotRow,
  type SquadSection,
} from "../squad/SquadTab.helpers";

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

function getPitchPlayerButtonClassName(options: {
  dragState: DragState | null;
  comparePlayerId: string | null;
  hoveredSlot: number | null;
  player: PlayerData;
  selectedPlayerId: string | null;
  slotIndex: number;
  wrongPos: boolean;
}): string {
  const {
    dragState,
    comparePlayerId,
    hoveredSlot,
    player,
    selectedPlayerId,
    slotIndex,
    wrongPos,
  } = options;
  const isComparing = player.id === comparePlayerId;
  const isHovered = hoveredSlot === slotIndex;
  const isSelected = player.id === selectedPlayerId;
  let className =
    "w-full min-w-0 max-w-20.5 cursor-grab rounded-xl border px-1.5 py-1.5 shadow-sm transition-all active:cursor-grabbing sm:px-2 sm:py-2";

  if (dragState?.playerId === player.id) {
    className = `${className} opacity-70 ring-2 ring-white/20`;
  } else {
    className = `${className} hover:-translate-y-0.5 hover:shadow-md`;
  }

  if (isSelected) {
    return `${className} border-accent-300 bg-accent-500/15 ring-2 ring-accent-300/40`;
  }

  if (isComparing) {
    return `${className} border-primary-300 bg-primary-500/12 ring-2 ring-primary-300/30`;
  }

  if (isHovered) {
    return `${className} border-primary-300 bg-primary-500/10`;
  }

  if (wrongPos) {
    return `${className} border-red-300/70 bg-red-500/60`;
  }

  return `${className} border-white/10 bg-black/15`;
}

function getBenchPlayerButtonClassName(options: {
  dragState: DragState | null;
  comparePlayerId: string | null;
  player: PlayerData;
  selectedPlayerId: string | null;
}): string {
  const { dragState, comparePlayerId, player, selectedPlayerId } = options;
  const isDragging = dragState?.playerId === player.id;
  const isComparing = comparePlayerId === player.id;
  const isSelected = selectedPlayerId === player.id;
  let className =
    "flex min-h-20 min-w-0 cursor-grab flex-col rounded-xl border px-3 py-2 text-left shadow-sm transition-all active:cursor-grabbing";

  if (isDragging) {
    className = `${className} opacity-70 ring-2 ring-white/20`;
  } else {
    className = `${className} hover:-translate-y-0.5 hover:shadow-md`;
  }

  if (isSelected) {
    return `${className} border-accent-300 bg-accent-600/80 dark:bg-accent-500/15 ring-2 ring-accent-300/40`;
  }

  if (isComparing) {
    return `${className} border-primary-300 bg-primary-500/12 ring-2 ring-primary-300/30`;
  }

  return `${className} border-white/10 bg-gray-500/70 dark:bg-navy-800`;
}

function getPitchRatingClassName(
  player: PlayerData,
  wrongPos: boolean,
): string {
  const baseClassName =
    "mx-auto mb-1.5 flex h-8 w-8 items-center justify-center rounded-full border-2 font-heading text-[11px] font-bold sm:h-9 sm:w-9 sm:text-xs";

  if (wrongPos) {
    return `${baseClassName} border-amber-200 bg-amber-500/85 text-white`;
  }

  if (player.condition >= 50) {
    return `${baseClassName} border-primary-200 bg-primary-500/80 text-white`;
  }

  return `${baseClassName} border-red-200 bg-red-500/80 text-white`;
}

function getEmptySlotClassName(isHovered: boolean): string {
  const baseClassName =
    "w-full min-w-0 rounded-xl border border-dashed px-1.5 py-3.5 text-center sm:px-2 sm:py-4";

  if (isHovered) {
    return `${baseClassName} border-primary-300 bg-primary-500/10`;
  }

  return `${baseClassName} border-white/20 bg-black/10`;
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
            {t("preMatch.startingXI")} — {formation}
          </h3>
          <p className="mt-0.5 text-xs text-gray-400">
            {t("tactics.pitchInteractionHint")}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Badge
            variant={outOfPositionCount > 0 ? "danger" : "success"}
            size="sm"
          >
            {outOfPositionCount} {t("squad.outOfPosition")}
          </Badge>
          {selectedPlayer ? (
            <button
              type="button"
              onClick={onClearSelection}
              className="text-xs font-heading font-bold uppercase tracking-wider text-accent-400 hover:text-accent-300"
            >
              {t("common.clear")}
            </button>
          ) : null}
        </div>
      </div>
      <div className="p-4 sm:p-6">
        <div className="relative min-h-115 overflow-visible rounded-xl border border-primary-500/20 bg-linear-to-b from-primary-500 to-primary-600 p-4 dark:from-primary-700 dark:to-primary-800 sm:min-h-130 sm:p-5">
          <div className="absolute inset-x-6 top-1/2 border-t border-white/50" />
          <div className="absolute left-1/2 top-1/2 h-20 w-20 -translate-x-1/2 -translate-y-1/2 rounded-full border border-white/50" />
          <div className="absolute inset-x-[18%] bottom-4 h-[18%] rounded-t-4xl border border-white/50 border-b-0" />
          <div className="absolute inset-x-[32%] bottom-4 h-[8%] rounded-t-2xl border border-white/50 border-b-0" />
          {pitchSlotRows.map((row) => (
            <div
              key={row.label}
              className="absolute left-1/2 grid items-start"
              style={{
                top: row.y,
                width:
                  row.slots.length === 1
                    ? `${getPitchSlotWidth(row.slots.length)}px`
                    : getPitchRowWidth(row.slots.length),
                transform: "translate(-50%, -50%)",
                gridTemplateColumns: `repeat(${row.slots.length}, minmax(0, ${getPitchSlotWidth(row.slots.length)}px))`,
                justifyContent:
                  row.slots.length === 1 ? "center" : "space-between",
              }}
            >
              {row.slots.map((slot) => {
                const isHovered = hoveredSlot === slot.index;
                const player = slot.player;
                const wrongPos = player
                  ? isPlayerOutOfPosition(player, slot.position)
                  : false;
                const slotRating = player ? getPlayerOvr(player) : null;

                return (
                  <div
                    key={`${row.label}-${slot.index}`}
                    data-testid={`pitch-slot-${slot.index}`}
                    className="flex w-full justify-center"
                    onDragOver={(event) => onSlotDragOver(event, slot.index)}
                    onDragLeave={() => onSlotDragLeave(slot.index)}
                    onDrop={(event) => onSlotDrop(event, slot.index)}
                  >
                    {player ? (
                      <button
                        type="button"
                        draggable
                        data-testid={`pitch-player-${player.id}`}
                        onClick={() => onLineupPlayerClick(player.id, "xi")}
                        onDragStart={(event) =>
                          onDragStart(event, player.id, "xi", slot.index)
                        }
                        onDragEnd={onDragEnd}
                        className={getPitchPlayerButtonClassName({
                          dragState,
                          comparePlayerId,
                          hoveredSlot,
                          player,
                          selectedPlayerId,
                          slotIndex: slot.index,
                          wrongPos,
                        })}
                      >
                        <div
                          className={getPitchRatingClassName(player, wrongPos)}
                        >
                          {slotRating}
                        </div>
                        <div className="text-[9px] font-heading font-bold uppercase tracking-wider leading-none text-white/70">
                          {translatePositionAbbreviation(t, slot.position)}
                        </div>
                        <div className="mt-1 truncate text-[10px] font-semibold leading-tight text-white sm:text-[11px]">
                          {player.match_name}
                        </div>
                        <div className="mt-0.5 truncate text-[9px] leading-none text-white/60">
                          {player.condition}%
                        </div>
                      </button>
                    ) : (
                      <div className={getEmptySlotClassName(isHovered)}>
                        <div className="text-[9px] font-heading font-bold uppercase tracking-wider leading-none text-white/70">
                          {translatePositionAbbreviation(t, slot.position)}
                        </div>
                        <div className="mt-1 text-[9px] leading-tight text-white/50">
                          {t("squad.dropPlayerHere")}
                        </div>
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          ))}
        </div>
        <div className="mt-4 border-t border-white/10 pt-4">
          <div className="mb-3 flex items-center justify-between gap-3">
            <div>
              <h4 className="text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-white/80">
                {t("preMatch.substitutes")}
              </h4>
              <p className="mt-1 text-sm text-black dark:text-white/50">
                {benchPlayers.length} {t("squad.playersLabel", "players")}
              </p>
            </div>
          </div>
          {benchPlayers.length > 0 ? (
            <div className="grid grid-cols-1 gap-2 sm:grid-cols-2 xl:grid-cols-3">
              {benchPlayers.map((player) => {
                const benchRating = getPlayerOvr(player);

                return (
                  <button
                    key={player.id}
                    type="button"
                    draggable={!player.injury}
                    data-testid={`pitch-bench-player-${player.id}`}
                    onClick={() => onLineupPlayerClick(player.id, "bench")}
                    onDragStart={(event) => {
                      if (!player.injury) {
                        onDragStart(event, player.id, "bench", null);
                      }
                    }}
                    onDragEnd={onDragEnd}
                    className={getBenchPlayerButtonClassName({
                      dragState,
                      comparePlayerId,
                      player,
                      selectedPlayerId,
                    })}
                  >
                    <div className="flex items-start justify-between gap-3">
                      <div className="min-w-0">
                        <div className="truncate text-sm font-heading font-bold text-white">
                          {player.match_name}
                        </div>
                        <div className="mt-1 text-sm uppercase tracking-wider text-white/60">
                          {translatePositionAbbreviation(
                            t,
                            player.natural_position || player.position,
                          )}
                        </div>
                      </div>
                      <div className="shrink-0 rounded-full border border-primary-200 bg-primary-500/80 px-2 py-1 text-xs font-heading font-bold text-white">
                        {benchRating}
                      </div>
                    </div>
                    <div className="mt-2 flex items-center justify-between gap-2 text-sm text-white/60">
                      <span>{player.condition}%</span>
                      <span>{player.morale}</span>
                    </div>
                  </button>
                );
              })}
            </div>
          ) : (
            <div className="rounded-xl border border-dashed border-white/15 bg-black/10 px-3 py-4 text-sm text-white/50">
              {t("preMatch.noBench")}
            </div>
          )}
        </div>
      </div>
    </Card>
  );
}
