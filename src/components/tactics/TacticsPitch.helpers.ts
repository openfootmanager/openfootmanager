import type { PlayerData } from "../../store/gameStore";
import type { DragState } from "../squad/SquadTab.helpers";

export function getPitchPlayerButtonClassName(options: {
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

export function getBenchPlayerButtonClassName(options: {
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

export function getPitchRatingClassName(
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

export function getEmptySlotClassName(isHovered: boolean): string {
    const baseClassName =
        "w-full min-w-0 rounded-xl border border-dashed px-1.5 py-3.5 text-center sm:px-2 sm:py-4";

    if (isHovered) {
        return `${baseClassName} border-primary-300 bg-primary-500/10`;
    }

    return `${baseClassName} border-white/20 bg-black/10`;
}