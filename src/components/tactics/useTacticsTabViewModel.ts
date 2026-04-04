import { useEffect, useMemo, useState } from "react";

import type { GameStateData, PlayerData, TeamData } from "../../store/gameStore";
import {
    buildActivePositionMap,
    buildPitchRows,
    buildPitchSlotRows,
} from "../squad/SquadTab.helpers";
import {
    applyTacticsFormationChange,
    applyTacticsPlayStyleChange,
    persistTacticsStartingXI,
} from "./TacticsTab.controller";
import {
    buildTacticsRoster,
    countOutOfPositionPlayers,
    filterAndSortTacticsPlayers,
    getSelectedAndComparePlayers,
    resolveStartingXiIds,
    type SortKey,
} from "./TacticsTab.helpers";
import { useTacticsLineupInteractions } from "./useTacticsLineupInteractions";

export type TacticsTabMode = "lineup" | "roles";

interface UseTacticsTabViewModelOptions {
    gameState: GameStateData;
    onGameUpdate: (gameState: GameStateData) => void;
}

interface UseTacticsTabViewModelResult {
    activePlayStyle: string;
    activeTab: TacticsTabMode;
    bench: PlayerData[];
    canConfirmSwap: boolean;
    clearLineupSelection: () => void;
    comparePlayer: PlayerData | null;
    comparePlayerId: string | null;
    currentDate: string;
    dragPreviewRef: ReturnType<typeof useTacticsLineupInteractions>["dragPreviewRef"];
    dragState: ReturnType<typeof useTacticsLineupInteractions>["dragState"];
    filteredBench: PlayerData[];
    filteredStartingXI: PlayerData[];
    formation: string;
    handleClearFilters: () => void;
    handleConfirmSwap: () => Promise<void>;
    handleDragStart: ReturnType<typeof useTacticsLineupInteractions>["handleDragStart"];
    handleFormationChange: (formation: string) => Promise<void>;
    handleLineupPlayerClick: ReturnType<typeof useTacticsLineupInteractions>["handleLineupPlayerClick"];
    handlePlayStyleChange: (playStyle: string) => Promise<void>;
    handleSlotDragLeave: ReturnType<typeof useTacticsLineupInteractions>["handleSlotDragLeave"];
    handleSlotDragOver: ReturnType<typeof useTacticsLineupInteractions>["handleSlotDragOver"];
    handleSlotDrop: ReturnType<typeof useTacticsLineupInteractions>["handleSlotDrop"];
    hoveredSlot: number | null;
    myTeam: TeamData | null;
    outOfPositionCount: number;
    pitchSlotRows: ReturnType<typeof buildPitchSlotRows>;
    playerSearch: string;
    positionFilter: string;
    resetDragState: () => void;
    roster: PlayerData[];
    selectedPlayer: PlayerData | null;
    selectedPlayerId: string | null;
    setActiveTab: (tab: TacticsTabMode) => void;
    setPlayerSearch: (value: string) => void;
    setPositionFilter: (value: string) => void;
    sortDir: "asc" | "desc";
    sortKey: SortKey;
    startingXI: PlayerData[];
    toggleSort: (key: SortKey) => void;
    xiActivePosition: Map<string, string>;
}

export function useTacticsTabViewModel({
    gameState,
    onGameUpdate,
}: UseTacticsTabViewModelOptions): UseTacticsTabViewModelResult {
    const currentDate = gameState.clock.current_date;
    const myTeam = useMemo(
        () =>
            gameState.teams.find((team) => team.id === gameState.manager.team_id) ??
            null,
        [gameState.manager.team_id, gameState.teams],
    );
    const [playerSearch, setPlayerSearch] = useState("");
    const [positionFilter, setPositionFilter] = useState("All");
    const [sortKey, setSortKey] = useState<SortKey>("pos");
    const [sortDir, setSortDir] = useState<"asc" | "desc">("asc");
    const [pendingStartingXiIds, setPendingStartingXiIds] = useState<
        string[] | null
    >(null);
    const [activeTab, setActiveTab] = useState<TacticsTabMode>("lineup");

    const roster = useMemo(
        () => (myTeam ? buildTacticsRoster(gameState.players, myTeam.id) : []),
        [gameState.players, myTeam],
    );

    const formation = myTeam?.formation || "4-4-2";
    const activePlayStyle = myTeam?.play_style || "Balanced";
    const savedStartingXiIds = myTeam?.starting_xi_ids || [];
    const savedStartingXiKey = savedStartingXiIds.join(",");
    const playersById = useMemo(
        () => new Map(roster.map((player) => [player.id, player])),
        [roster],
    );
    const available = useMemo(
        () => roster.filter((player) => !player.injury),
        [roster],
    );
    const pitchRows = useMemo(() => buildPitchRows(formation), [formation]);

    const startingXiIds = useMemo(
        () =>
            myTeam
                ? resolveStartingXiIds({
                    availablePlayers: available,
                    formation,
                    pendingStartingXiIds,
                    playersById,
                    savedStartingXiIds,
                })
                : [],
        [
            available,
            formation,
            myTeam,
            pendingStartingXiIds,
            playersById,
            savedStartingXiIds,
        ],
    );

    const startingXI = useMemo(
        () =>
            startingXiIds
                .map((id) => playersById.get(id))
                .filter((player): player is PlayerData => player != null),
        [playersById, startingXiIds],
    );

    useEffect(() => {
        if (!pendingStartingXiIds) {
            return;
        }

        if (savedStartingXiKey === pendingStartingXiIds.join(",")) {
            setPendingStartingXiIds(null);
        }
    }, [pendingStartingXiIds, savedStartingXiKey]);

    const pitchSlotRows = useMemo(
        () => buildPitchSlotRows(pitchRows, startingXiIds, playersById),
        [pitchRows, playersById, startingXiIds],
    );
    const xiIds = useMemo(() => new Set(startingXiIds), [startingXiIds]);
    const bench = useMemo(
        () => roster.filter((player) => !xiIds.has(player.id)),
        [roster, xiIds],
    );
    const xiActivePosition = useMemo(
        () => buildActivePositionMap(pitchSlotRows),
        [pitchSlotRows],
    );

    function toggleSort(key: SortKey): void {
        if (sortKey === key) {
            setSortDir((current) => (current === "asc" ? "desc" : "asc"));
            return;
        }

        setSortKey(key);
        setSortDir(key === "ovr" ? "desc" : "asc");
    }

    const filteredStartingXI = useMemo(
        () =>
            filterAndSortTacticsPlayers(
                startingXI,
                {
                    playerSearch,
                    positionFilter,
                    section: "xi",
                    xiActivePosition,
                },
                {
                    currentDate,
                    section: "xi",
                    sortDir,
                    sortKey,
                    xiActivePosition,
                },
            ),
        [
            currentDate,
            playerSearch,
            positionFilter,
            sortDir,
            sortKey,
            startingXI,
            xiActivePosition,
        ],
    );
    const filteredBench = useMemo(
        () =>
            filterAndSortTacticsPlayers(
                bench,
                {
                    playerSearch,
                    positionFilter,
                    section: "bench",
                    xiActivePosition,
                },
                {
                    currentDate,
                    section: "bench",
                    sortDir,
                    sortKey,
                    xiActivePosition,
                },
            ),
        [
            bench,
            currentDate,
            playerSearch,
            positionFilter,
            sortDir,
            sortKey,
            xiActivePosition,
        ],
    );

    const outOfPositionCount = useMemo(
        () => countOutOfPositionPlayers(startingXI, xiActivePosition),
        [startingXI, xiActivePosition],
    );

    async function persistStartingXI(playerIds: string[]): Promise<void> {
        await persistTacticsStartingXI(
            playerIds,
            onGameUpdate,
            setPendingStartingXiIds,
        );
    }

    async function handleFormationChange(nextFormation: string): Promise<void> {
        await applyTacticsFormationChange(nextFormation, onGameUpdate);
    }

    async function handlePlayStyleChange(playStyle: string): Promise<void> {
        await applyTacticsPlayStyleChange(playStyle, onGameUpdate);
    }

    const {
        canConfirmSwap,
        clearLineupSelection,
        comparePlayerId,
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
    } = useTacticsLineupInteractions({
        currentXiIds: startingXiIds,
        persistStartingXI,
        xiIds,
    });

    const { comparePlayer, selectedPlayer } = useMemo(
        () =>
            getSelectedAndComparePlayers(
                comparePlayerId,
                playersById,
                selectedPlayerId,
            ),
        [comparePlayerId, playersById, selectedPlayerId],
    );

    function handleClearFilters(): void {
        setPlayerSearch("");
        setPositionFilter("All");
    }

    return {
        activePlayStyle,
        activeTab,
        bench,
        canConfirmSwap,
        clearLineupSelection,
        comparePlayer,
        comparePlayerId,
        currentDate,
        dragPreviewRef,
        dragState,
        filteredBench,
        filteredStartingXI,
        formation,
        handleClearFilters,
        handleConfirmSwap,
        handleDragStart,
        handleFormationChange,
        handleLineupPlayerClick,
        handlePlayStyleChange,
        handleSlotDragLeave,
        handleSlotDragOver,
        handleSlotDrop,
        hoveredSlot,
        myTeam,
        outOfPositionCount,
        pitchSlotRows,
        playerSearch,
        positionFilter,
        resetDragState,
        roster,
        selectedPlayer,
        selectedPlayerId,
        setActiveTab,
        setPlayerSearch,
        setPositionFilter,
        sortDir,
        sortKey,
        startingXI,
        toggleSort,
        xiActivePosition,
    };
}