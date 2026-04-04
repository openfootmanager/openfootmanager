import { useTranslation } from "react-i18next";

import type { PlayerData, PlayerSelectionOptions } from "../../store/gameStore";
import type { PitchSlotRow, SquadSection } from "../squad/SquadTab.helpers";
import TacticsFilters from "./TacticsFilters";
import TacticsPitch from "./TacticsPitch";
import TacticsPlayerFocusPanel from "./TacticsPlayerFocusPanel";
import TacticsPlayerTable from "./TacticsPlayerTable";
import TacticsSetupPanel from "./TacticsSetupPanel";
import type { SortKey } from "./TacticsTab.helpers";

export function TacticsTabModeTabs({
    activeTab,
    onSelectTab,
}: {
    activeTab: "lineup" | "roles";
    onSelectTab: (tab: "lineup" | "roles") => void;
}) {
    const { t } = useTranslation();

    return (
        <div className="flex gap-1 self-start rounded-lg bg-gray-100 p-1 dark:bg-navy-800">
            <button
                type="button"
                onClick={() => onSelectTab("lineup")}
                className={`rounded-md px-4 py-2 text-xs font-heading font-bold uppercase tracking-wider transition-colors ${activeTab === "lineup"
                    ? "bg-white text-gray-900 shadow-sm dark:bg-navy-700 dark:text-white"
                    : "text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200"
                    }`}
            >
                {t("tactics.lineupTab", "Lineup")}
            </button>
            <button
                type="button"
                onClick={() => onSelectTab("roles")}
                className={`rounded-md px-4 py-2 text-xs font-heading font-bold uppercase tracking-wider transition-colors ${activeTab === "roles"
                    ? "bg-white text-gray-900 shadow-sm dark:bg-navy-700 dark:text-white"
                    : "text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200"
                    }`}
            >
                {t("tactics.rolesTab", "Set pieces & roles")}
            </button>
        </div>
    );
}

export function TacticsLineupContent({
    activePlayStyle,
    bench,
    canConfirmSwap,
    comparePlayer,
    comparePlayerId,
    currentDate,
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
    onSelectPlayer,
    onClearSelection,
    onPlayerSearchChange,
    onPositionFilterChange,
    outOfPositionCount,
    pitchSlotRows,
    playerSearch,
    positionFilter,
    resetDragState,
    selectedPlayer,
    selectedPlayerId,
    sortDir,
    sortKey,
    startingXiCount,
    toggleSort,
    hoveredSlot,
    xiActivePosition,
}: {
    activePlayStyle: string;
    bench: PlayerData[];
    canConfirmSwap: boolean;
    comparePlayer: PlayerData | null;
    comparePlayerId: string | null;
    currentDate: string;
    dragState: Parameters<typeof TacticsPitch>[0]["dragState"];
    filteredBench: PlayerData[];
    filteredStartingXI: PlayerData[];
    formation: string;
    handleClearFilters: () => void;
    handleConfirmSwap: () => Promise<void>;
    handleDragStart: Parameters<typeof TacticsPitch>[0]["onDragStart"];
    handleFormationChange: (formation: string) => Promise<void>;
    handleLineupPlayerClick: (playerId: string, section: SquadSection) => void;
    handlePlayStyleChange: (playStyle: string) => Promise<void>;
    handleSlotDragLeave: (slotIndex: number) => void;
    handleSlotDragOver: Parameters<typeof TacticsPitch>[0]["onSlotDragOver"];
    handleSlotDrop: Parameters<typeof TacticsPitch>[0]["onSlotDrop"];
    onSelectPlayer: (id: string, options?: PlayerSelectionOptions) => void;
    onClearSelection: () => void;
    onPlayerSearchChange: (value: string) => void;
    onPositionFilterChange: (value: string) => void;
    outOfPositionCount: number;
    pitchSlotRows: PitchSlotRow[];
    playerSearch: string;
    positionFilter: string;
    resetDragState: () => void;
    selectedPlayer: PlayerData | null;
    selectedPlayerId: string | null;
    sortDir: "asc" | "desc";
    sortKey: SortKey;
    startingXiCount: number;
    toggleSort: (key: SortKey) => void;
    hoveredSlot: number | null;
    xiActivePosition: Map<string, string>;
}) {
    const { t } = useTranslation();

    return (
        <>
            <div className="grid grid-cols-1 items-start gap-4 xl:grid-cols-[minmax(0,1.65fr)_minmax(320px,0.95fr)]">
                <TacticsPitch
                    benchPlayers={bench}
                    dragState={dragState}
                    formation={formation}
                    comparePlayerId={comparePlayerId}
                    hoveredSlot={hoveredSlot}
                    onClearSelection={onClearSelection}
                    onDragEnd={resetDragState}
                    onDragStart={handleDragStart}
                    onLineupPlayerClick={handleLineupPlayerClick}
                    onSlotDragLeave={handleSlotDragLeave}
                    onSlotDragOver={handleSlotDragOver}
                    onSlotDrop={handleSlotDrop}
                    outOfPositionCount={outOfPositionCount}
                    pitchSlotRows={pitchSlotRows}
                    selectedPlayer={selectedPlayer}
                    selectedPlayerId={selectedPlayerId}
                />

                <div className="flex flex-col gap-4">
                    <TacticsPlayerFocusPanel
                        canConfirmSwap={canConfirmSwap}
                        currentDate={currentDate}
                        onConfirmSwap={() => {
                            void handleConfirmSwap();
                        }}
                        selectedPlayer={selectedPlayer}
                        comparePlayer={comparePlayer}
                    />
                    <TacticsSetupPanel
                        activePlayStyle={activePlayStyle}
                        formation={formation}
                        onFormationChange={(nextFormation) => {
                            void handleFormationChange(nextFormation);
                        }}
                        onPlayStyleChange={(playStyle) => {
                            void handlePlayStyleChange(playStyle);
                        }}
                    />
                </div>
            </div>

            <TacticsFilters
                onClear={handleClearFilters}
                onPlayerSearchChange={onPlayerSearchChange}
                onPositionFilterChange={onPositionFilterChange}
                playerSearch={playerSearch}
                positionFilter={positionFilter}
            />

            <TacticsPlayerTable
                currentDate={currentDate}
                emptyMessage={t(
                    "squad.noLineupMatches",
                    "No starters match the current filters.",
                )}
                highlightedPlayerId={selectedPlayerId}
                onSelectPlayer={onSelectPlayer}
                players={filteredStartingXI}
                section="xi"
                sortDir={sortDir}
                sortKey={sortKey}
                title={t("preMatch.startingXI", "Starting XI")}
                toggleSort={toggleSort}
                totalCount={startingXiCount}
                xiActivePosition={xiActivePosition}
            />

            <TacticsPlayerTable
                currentDate={currentDate}
                emptyMessage={t(
                    "squad.noBenchMatches",
                    "No bench players match the current filters.",
                )}
                highlightedPlayerId={selectedPlayerId}
                onSelectPlayer={onSelectPlayer}
                players={filteredBench}
                section="bench"
                sortDir={sortDir}
                sortKey={sortKey}
                title={t("preMatch.substitutes", "Substitutes")}
                toggleSort={toggleSort}
                totalCount={bench.length}
                xiActivePosition={xiActivePosition}
            />
        </>
    );
}