import type { DragEvent } from "react";
import { useTranslation } from "react-i18next";

import { calcOvr } from "../../lib/helpers";
import type { PlayerData } from "../../store/gameStore";
import {
    getPitchRowWidth,
    getPitchSlotWidth,
    isPlayerOutOfPosition,
    translatePositionAbbreviation,
    type DragState,
    type PitchSlotRow,
    type SquadSection,
} from "../squad/SquadTab.helpers";
import {
    getBenchPlayerButtonClassName,
    getEmptySlotClassName,
    getPitchPlayerButtonClassName,
    getPitchRatingClassName,
} from "./TacticsPitch.helpers";

function PitchSlotCell({
    comparePlayerId,
    dragState,
    hoveredSlot,
    onDragEnd,
    onDragStart,
    onLineupPlayerClick,
    onSlotDragLeave,
    onSlotDragOver,
    onSlotDrop,
    selectedPlayerId,
    slot,
}: {
    comparePlayerId: string | null;
    dragState: DragState | null;
    hoveredSlot: number | null;
    onDragEnd: () => void;
    onDragStart: (
        event: DragEvent<HTMLElement>,
        playerId: string,
        from: SquadSection,
        slotIndex: number | null,
    ) => void;
    onLineupPlayerClick: (playerId: string, section: SquadSection) => void;
    onSlotDragLeave: (slotIndex: number) => void;
    onSlotDragOver: (event: DragEvent<HTMLElement>, slotIndex: number) => void;
    onSlotDrop: (event: DragEvent<HTMLElement>, slotIndex: number) => void;
    selectedPlayerId: string | null;
    slot: PitchSlotRow["slots"][number];
}) {
    const { t } = useTranslation();
    const isHovered = hoveredSlot === slot.index;
    const player = slot.player;
    const wrongPos = player
        ? isPlayerOutOfPosition(player, slot.position)
        : false;
    const slotRating = player ? calcOvr(player, slot.position) : null;

    return (
        <div
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
                    <div className={getPitchRatingClassName(player, wrongPos)}>
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
                        {t("squad.dropPlayerHere", "Drop player here")}
                    </div>
                </div>
            )}
        </div>
    );
}

export function PitchSurfaceSection({
    comparePlayerId,
    dragState,
    hoveredSlot,
    onDragEnd,
    onDragStart,
    onLineupPlayerClick,
    onSlotDragLeave,
    onSlotDragOver,
    onSlotDrop,
    pitchSlotRows,
    selectedPlayerId,
}: {
    comparePlayerId: string | null;
    dragState: DragState | null;
    hoveredSlot: number | null;
    onDragEnd: () => void;
    onDragStart: (
        event: DragEvent<HTMLElement>,
        playerId: string,
        from: SquadSection,
        slotIndex: number | null,
    ) => void;
    onLineupPlayerClick: (playerId: string, section: SquadSection) => void;
    onSlotDragLeave: (slotIndex: number) => void;
    onSlotDragOver: (event: DragEvent<HTMLElement>, slotIndex: number) => void;
    onSlotDrop: (event: DragEvent<HTMLElement>, slotIndex: number) => void;
    pitchSlotRows: PitchSlotRow[];
    selectedPlayerId: string | null;
}) {
    return (
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
                        justifyContent: row.slots.length === 1 ? "center" : "space-between",
                    }}
                >
                    {row.slots.map((slot) => (
                        <PitchSlotCell
                            key={`${row.label}-${slot.index}`}
                            comparePlayerId={comparePlayerId}
                            dragState={dragState}
                            hoveredSlot={hoveredSlot}
                            onDragEnd={onDragEnd}
                            onDragStart={onDragStart}
                            onLineupPlayerClick={onLineupPlayerClick}
                            onSlotDragLeave={onSlotDragLeave}
                            onSlotDragOver={onSlotDragOver}
                            onSlotDrop={onSlotDrop}
                            selectedPlayerId={selectedPlayerId}
                            slot={slot}
                        />
                    ))}
                </div>
            ))}
        </div>
    );
}

function BenchPlayerCard({
    comparePlayerId,
    dragState,
    onDragEnd,
    onDragStart,
    onLineupPlayerClick,
    player,
    selectedPlayerId,
}: {
    comparePlayerId: string | null;
    dragState: DragState | null;
    onDragEnd: () => void;
    onDragStart: (
        event: DragEvent<HTMLElement>,
        playerId: string,
        from: SquadSection,
        slotIndex: number | null,
    ) => void;
    onLineupPlayerClick: (playerId: string, section: SquadSection) => void;
    player: PlayerData;
    selectedPlayerId: string | null;
}) {
    const { t } = useTranslation();
    const benchRating = calcOvr(
        player,
        player.natural_position || player.position,
    );

    return (
        <button
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
}

export function BenchPlayersSection({
    benchPlayers,
    comparePlayerId,
    dragState,
    onDragEnd,
    onDragStart,
    onLineupPlayerClick,
    selectedPlayerId,
}: {
    benchPlayers: PlayerData[];
    comparePlayerId: string | null;
    dragState: DragState | null;
    onDragEnd: () => void;
    onDragStart: (
        event: DragEvent<HTMLElement>,
        playerId: string,
        from: SquadSection,
        slotIndex: number | null,
    ) => void;
    onLineupPlayerClick: (playerId: string, section: SquadSection) => void;
    selectedPlayerId: string | null;
}) {
    const { t } = useTranslation();

    return (
        <div className="mt-4 border-t border-white/10 pt-4">
            <div className="mb-3 flex items-center justify-between gap-3">
                <div>
                    <h4 className="text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-white/80">
                        {t("preMatch.substitutes", "Substitutes")}
                    </h4>
                    <p className="mt-1 text-sm text-black dark:text-white/50">
                        {benchPlayers.length} {t("squad.playersLabel", "players")}
                    </p>
                </div>
            </div>
            {benchPlayers.length > 0 ? (
                <div className="grid grid-cols-1 gap-2 sm:grid-cols-2 xl:grid-cols-3">
                    {benchPlayers.map((player) => (
                        <BenchPlayerCard
                            key={player.id}
                            comparePlayerId={comparePlayerId}
                            dragState={dragState}
                            onDragEnd={onDragEnd}
                            onDragStart={onDragStart}
                            onLineupPlayerClick={onLineupPlayerClick}
                            player={player}
                            selectedPlayerId={selectedPlayerId}
                        />
                    ))}
                </div>
            ) : (
                <div className="rounded-xl border border-dashed border-white/15 bg-black/10 px-3 py-4 text-sm text-white/50">
                    {t("preMatch.noBench", "No bench players available.")}
                </div>
            )}
        </div>
    );
}