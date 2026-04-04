import type { DragEvent, RefObject } from "react";
import { useMemo, useRef, useState } from "react";

import type { DragState, SquadSection } from "../squad/SquadTab.helpers";
import {
    getConfirmedSwapXiIds,
    getDroppedLineupXiIds,
    getEmptyTacticsLineupSelection,
    getNextTacticsLineupSelection,
    type TacticsLineupSelectionState,
} from "./TacticsTab.helpers";

interface UseTacticsLineupInteractionsOptions {
    currentXiIds: string[];
    persistStartingXI: (playerIds: string[]) => Promise<void>;
    xiIds: Set<string>;
}

interface UseTacticsLineupInteractionsResult {
    canConfirmSwap: boolean;
    clearLineupSelection: () => void;
    comparePlayerId: string | null;
    comparePlayerSection: SquadSection | null;
    dragPreviewRef: RefObject<HTMLDivElement | null>;
    dragState: DragState | null;
    handleConfirmSwap: () => Promise<void>;
    handleDragStart: (
        event: DragEvent<HTMLElement>,
        playerId: string,
        from: SquadSection,
        slotIndex?: number | null,
    ) => void;
    handleLineupPlayerClick: (playerId: string, section: SquadSection) => void;
    handleSlotDragLeave: (slotIndex: number) => void;
    handleSlotDragOver: (
        event: DragEvent<HTMLElement>,
        slotIndex: number,
    ) => void;
    handleSlotDrop: (
        event: DragEvent<HTMLElement>,
        slotIndex: number,
    ) => Promise<void>;
    hoveredSlot: number | null;
    resetDragState: () => void;
    selectedPlayerId: string | null;
    selectedPlayerSection: SquadSection | null;
}

export function useTacticsLineupInteractions({
    currentXiIds,
    persistStartingXI,
    xiIds,
}: UseTacticsLineupInteractionsOptions): UseTacticsLineupInteractionsResult {
    const [dragState, setDragState] = useState<DragState | null>(null);
    const [hoveredSlot, setHoveredSlot] = useState<number | null>(null);
    const [selectedPlayerId, setSelectedPlayerId] = useState<string | null>(null);
    const [selectedPlayerSection, setSelectedPlayerSection] =
        useState<SquadSection | null>(null);
    const [comparePlayerId, setComparePlayerId] = useState<string | null>(null);
    const [comparePlayerSection, setComparePlayerSection] =
        useState<SquadSection | null>(null);
    const dragStateRef = useRef<DragState | null>(null);
    const hoveredSlotRef = useRef<number | null>(null);
    const dragPreviewRef = useRef<HTMLDivElement | null>(null);

    function applyLineupSelectionState(
        nextState: TacticsLineupSelectionState,
    ): void {
        setSelectedPlayerId(nextState.selectedPlayerId);
        setSelectedPlayerSection(nextState.selectedPlayerSection);
        setComparePlayerId(nextState.comparePlayerId);
        setComparePlayerSection(nextState.comparePlayerSection);
    }

    const canConfirmSwap = useMemo(() => {
        return !!getConfirmedSwapXiIds({
            currentXiIds,
            selectionState: {
                selectedPlayerId,
                selectedPlayerSection,
                comparePlayerId,
                comparePlayerSection,
            },
        });
    }, [
        comparePlayerId,
        comparePlayerSection,
        currentXiIds,
        selectedPlayerId,
        selectedPlayerSection,
    ]);

    function clearLineupSelection(): void {
        applyLineupSelectionState(getEmptyTacticsLineupSelection());
    }

    function setHoveredSlotValue(slotIndex: number | null): void {
        if (hoveredSlotRef.current === slotIndex) {
            return;
        }

        hoveredSlotRef.current = slotIndex;
        setHoveredSlot(slotIndex);
    }

    function resetDragState(): void {
        dragStateRef.current = null;
        setDragState(null);
        setHoveredSlotValue(null);
    }

    function applyLightweightDragPreview(event: DragEvent<HTMLElement>): void {
        if (!dragPreviewRef.current) {
            return;
        }

        if (typeof event.dataTransfer.setDragImage !== "function") {
            return;
        }

        event.dataTransfer.setDragImage(dragPreviewRef.current, 16, 16);
    }

    function handleDragStart(
        event: DragEvent<HTMLElement>,
        playerId: string,
        from: SquadSection,
        slotIndex: number | null = null,
    ): void {
        event.dataTransfer.effectAllowed = "move";
        event.dataTransfer.setData("text/plain", playerId);
        applyLightweightDragPreview(event);
        const nextDragState = { playerId, from, slotIndex };
        dragStateRef.current = nextDragState;
        setDragState(nextDragState);
    }

    function handleSlotDragOver(
        event: DragEvent<HTMLElement>,
        slotIndex: number,
    ): void {
        event.preventDefault();
        event.dataTransfer.dropEffect = "move";
        setHoveredSlotValue(slotIndex);
    }

    function handleSlotDragLeave(slotIndex: number): void {
        if (hoveredSlotRef.current !== slotIndex) {
            return;
        }

        setHoveredSlotValue(null);
    }

    async function handleSlotDrop(
        event: DragEvent<HTMLElement>,
        slotIndex: number,
    ): Promise<void> {
        event.preventDefault();
        const draggedPlayerId = event.dataTransfer.getData("text/plain");
        const nextXiIds = getDroppedLineupXiIds({
            currentXiIds,
            draggedPlayerId,
            dragState: dragStateRef.current ?? dragState,
            slotIndex,
            xiIds,
        });

        if (!nextXiIds) {
            resetDragState();
            return;
        }

        await persistStartingXI(nextXiIds);
        clearLineupSelection();
        resetDragState();
    }

    function handleLineupPlayerClick(
        playerId: string,
        section: SquadSection,
    ): void {
        applyLineupSelectionState(
            getNextTacticsLineupSelection(
                {
                    selectedPlayerId,
                    selectedPlayerSection,
                    comparePlayerId,
                    comparePlayerSection,
                },
                playerId,
                section,
            ),
        );
    }

    async function handleConfirmSwap(): Promise<void> {
        const nextXiIds = getConfirmedSwapXiIds({
            currentXiIds,
            selectionState: {
                selectedPlayerId,
                selectedPlayerSection,
                comparePlayerId,
                comparePlayerSection,
            },
        });

        if (!nextXiIds) {
            return;
        }

        await persistStartingXI(nextXiIds);
        clearLineupSelection();
    }

    return {
        canConfirmSwap,
        clearLineupSelection,
        comparePlayerId,
        comparePlayerSection,
        dragPreviewRef,
        dragState,
        handleConfirmSwap,
        handleDragStart,
        handleLineupPlayerClick,
        handleSlotDragLeave,
        handleSlotDragOver,
        handleSlotDrop,
        hoveredSlot,
        resetDragState,
        selectedPlayerId,
        selectedPlayerSection,
    };
}